//! Операции формы (спека `2026-10-12-delivery-variants-design.md`, § 6): `POST /api/<команда>` один к одному
//! с командами Tauri, от имени вошедшего пользователя. `host` — только домен из конфига (SSRF): исходящий
//! адрес из ввода не строится.

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::future::ready;

use axum::Json;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, header};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::PrivateCookieJar;
use futures::StreamExt;
use longpole_core::browse::{Page, Pipeline, Project, Workflow};
use longpole_core::error::{CmdError, Error, ErrorCode, Field};
use longpole_core::history::{HistoryLabel, NewEntry};
use longpole_core::projects::SavedProject;
use longpole_core::report::{BuildEnv, build_report};
use longpole_core::request::{Form, FormMode, Request, parse_form, parse_remote};
use longpole_core::schema::{Locale, Report};
use longpole_core::source::{AnySource, Provider, Source};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::OffsetDateTime;
use tokio::sync::mpsc;

use crate::auth::{auth_required, forget};
use crate::{AppState, ndjson};

type Cmd<T> = Result<T, CmdError>;

fn unknown_host(domain: &str) -> Error {
    Error::new(ErrorCode::UnknownHost).with("host", domain)
}

/// 401 провайдера при живом access: токен отозван у провайдера.
fn revoked<T>(result: &Cmd<T>) -> bool {
    matches!(result, Err(CmdError::Message(e))
        if e.code == ErrorCode::Unauthorized && e.params.get("status").map(String::as_str) == Some("401"))
}

/// Строка ответа: `{"ok":…}` или `{"err":…}`.
fn line<T: Serialize>(result: Cmd<T>) -> Value {
    match result {
        Ok(value) => json!({ "ok": value }),
        Err(e) => json!({ "err": e }),
    }
}

impl AppState {
    /// Клиент хоста с access пользователя: `unknownHost` — домен не из конфига, `authRequired` — нет входа.
    async fn source(
        &self,
        jar: PrivateCookieJar,
        domain: &str,
    ) -> (PrivateCookieJar, Result<AnySource, Error>) {
        let Some(host) = self.host(domain) else {
            return (jar, Err(unknown_host(domain)));
        };
        let (jar, access) = self.access(jar, domain).await;
        let source =
            access.and_then(|token| AnySource::at(host.host.provider, domain, &host.base, &token));
        (jar, source)
    }

    /// Операция над клиентом хоста; отозванный вход уходит из cookie и становится `authRequired`.
    async fn run<T: Serialize>(
        &self,
        jar: PrivateCookieJar,
        domain: &str,
        op: impl AsyncFnOnce(AnySource) -> Cmd<T>,
    ) -> Response {
        let (jar, source) = self.source(jar, domain).await;
        let result = match source {
            Ok(source) => op(source).await,
            Err(e) => Err(e.into()),
        };
        if revoked(&result) {
            let jar = forget(jar, domain);
            return (jar, ndjson(&line::<()>(Err(auth_required(domain).into())))).into_response();
        }
        (jar, ndjson(&line(result))).into_response()
    }
}

#[derive(Deserialize)]
pub(crate) struct HostArgs {
    host: String,
}

/// Провайдер из конфига, без пробы и без входа.
pub(crate) async fn host_provider(
    State(state): State<AppState>,
    Json(args): Json<HostArgs>,
) -> Response {
    let provider = state
        .host(&args.host)
        .map(|h| h.host.provider)
        .ok_or_else(|| unknown_host(&args.host).into());
    ndjson(&line::<Provider>(provider))
}

#[derive(Deserialize)]
pub(crate) struct ProjectsArgs {
    host: String,
    search: String,
    after: Option<String>,
}

pub(crate) async fn projects(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Json(args): Json<ProjectsArgs>,
) -> Response {
    state
        .run(jar, &args.host, async |source| -> Cmd<Page<Project>> {
            Ok(source
                .list_projects(&args.search, args.after.as_deref())
                .await?)
        })
        .await
}

#[derive(Deserialize)]
pub(crate) struct BranchesArgs {
    host: String,
    project: String,
    search: String,
}

pub(crate) async fn branches(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Json(args): Json<BranchesArgs>,
) -> Response {
    state
        .run(jar, &args.host, async |source| -> Cmd<Vec<String>> {
            Ok(source.list_branches(&args.project, &args.search).await?)
        })
        .await
}

#[derive(Deserialize)]
pub(crate) struct PipelinesArgs {
    host: String,
    project: String,
    r#ref: Option<String>,
    workflow: Option<String>,
    after: Option<String>,
}

pub(crate) async fn pipelines(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Json(args): Json<PipelinesArgs>,
) -> Response {
    state
        .run(jar, &args.host, async |source| -> Cmd<Page<Pipeline>> {
            Ok(source
                .recent_pipelines(
                    &args.project,
                    args.r#ref.as_deref(),
                    args.workflow.as_deref(),
                    args.after.as_deref(),
                )
                .await?)
        })
        .await
}

#[derive(Deserialize)]
pub(crate) struct WorkflowsArgs {
    host: String,
    project: String,
}

pub(crate) async fn workflows(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Json(args): Json<WorkflowsArgs>,
) -> Response {
    state
        .run(jar, &args.host, async |source| -> Cmd<Vec<Workflow>> {
            Ok(source.list_workflows(&args.project).await?)
        })
        .await
}

#[derive(Deserialize)]
pub(crate) struct AddProjectArgs {
    input: String,
}

/// Разбор ссылки (папок у сервера нет) и проверка доступа; список хранит браузер.
pub(crate) async fn add_project(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Json(args): Json<AddProjectArgs>,
) -> Response {
    let project = match parse_remote(args.input.trim()) {
        Ok(project) => project,
        Err(e) => return ndjson(&line::<()>(Err(e.into()))),
    };
    let domain = project.host.clone();
    state
        .run(jar, &domain, async |source| -> Cmd<SavedProject> {
            let found = source.fetch_project(&project.path).await?;
            Ok(SavedProject::new(
                project,
                &found.name,
                OffsetDateTime::now_utc(),
            ))
        })
        .await
}

#[derive(Deserialize)]
pub(crate) struct BuildArgs {
    form: Form,
}

/// Отчёт и запись истории для «Недавних» в браузере. Ответ — поток NDJSON: строки `progress`, последняя —
/// `ok` или `err`. Закрытая вкладка обрывает поток, а с ним и сборку.
pub(crate) async fn build(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: PrivateCookieJar,
    Json(BuildArgs { form }): Json<BuildArgs>,
) -> Response {
    let parsed = match parse_form(&form) {
        Ok(parsed) => parsed,
        Err(errors) => return ndjson(&line::<()>(Err(errors.into()))),
    };
    let host = parsed.host;
    if state.host(&host).is_none() {
        return ndjson(&line::<()>(Err(unknown_host(&host).into())));
    }
    // ссылка ведёт на сам хост конфига: `https://<домен>/…`, без другой схемы и порта
    if form.mode == FormMode::Link && !form.url.trim().starts_with(&format!("https://{host}/")) {
        return ndjson(&line::<()>(Err(field(Field::Url, ErrorCode::InvalidLink))));
    }
    let (jar, source) = state.source(jar, &host).await;
    let source = match source {
        Ok(source) => source,
        Err(e) => return (jar, ndjson(&line::<()>(Err(e.into())))).into_response(),
    };
    if source.provider() == Provider::Github
        && matches!(parsed.request, Request::Aggregate { workflow: None, .. })
    {
        let e = field(Field::Workflow, ErrorCode::WorkflowRequired);
        return (jar, ndjson(&line::<()>(Err(e)))).into_response();
    }
    let locale = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .map_or(Locale::En, Locale::from_tag);
    let request = parsed.request;
    let (tx, rx) = mpsc::unbounded_channel::<Value>();
    let work = async move {
        let progress = tx.clone();
        let now = OffsetDateTime::now_utc();
        let env = BuildEnv { now, locale };
        let built = build_report(&source, &request, env, move |p| {
            // поток уже закрыт — прогресс некому показывать
            let _ = progress.send(json!({ "progress": p }));
        })
        .await;
        let result: Cmd<Value> = built.map_err(Into::into).map(|built| {
            let (Report::Single { meta, .. } | Report::Aggregate { meta, .. }) = &built.report;
            let label = HistoryLabel {
                project: meta.project.clone(),
                label: meta.label.clone(),
            };
            let entry = NewEntry {
                host: host.clone(),
                form,
                request,
                label,
            }
            .at(now);
            json!({ "report": built.report, "entry": entry })
        });
        // ответ уже начат, cookie не поменять: хост сотрёт следующий запрос или новый вход
        let result = if revoked(&result) {
            Err(auth_required(&host).into())
        } else {
            result
        };
        let _ = tx.send(line(result));
    };
    // `work` отдаёт строки в канал и заканчивается последней; канал закрывается, когда `work` завершён
    let lines = futures::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|line| (line, rx))
    })
    .map(|line| Ok::<_, Infallible>(format!("{line}\n")));
    let driver =
        futures::stream::once(work).filter_map(|()| ready(None::<Result<String, Infallible>>));
    let body = Body::from_stream(futures::stream::select(lines, driver));
    (jar, [(header::CONTENT_TYPE, "application/x-ndjson")], body).into_response()
}

fn field(field: Field, code: ErrorCode) -> CmdError {
    BTreeMap::from([(field, Error::new(code))]).into()
}

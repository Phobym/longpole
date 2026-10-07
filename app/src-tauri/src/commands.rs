//! Команды формы. Токен не возвращает ни одна: форма видит только список хостов.
//! Все `async`: синхронная команда Tauri исполняется на главном потоке, а тут файлы и связка ключей.

use std::collections::BTreeMap;

use pipeline_trace_core::browse::{Page, Pipeline, Project, Workflow};
use pipeline_trace_core::error::{CmdError, Error, ErrorCode, Field};
use pipeline_trace_core::history::{HistoryEntry, HistoryLabel, NewEntry};
use pipeline_trace_core::hosts::normalize_host;
use pipeline_trace_core::projects::SavedProject;
use pipeline_trace_core::render::render;
use pipeline_trace_core::report::{BuildEnv, Progress, build_report};
use pipeline_trace_core::request::{
    Form, FormMode, ProjectRef, Request, link_provider, parse_form, parse_project_input,
};
use pipeline_trace_core::schema::{Meta, Report};
use pipeline_trace_core::settings::{AppSettings, SettingsPatch};
use pipeline_trace_core::source::{AnySource, Provider, Source, resolve_provider};
use pipeline_trace_core::tokens::{
    Gh, Glab, HostInfo, find_github_token, find_token, github_hosts, list_hosts,
};
use pipeline_trace_core::{github, gitlab};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State, Webview};
use time::OffsetDateTime;

use crate::menu;
use crate::reports::TEMPLATE;
use crate::state::{AppState, ReportEntry};
use crate::windows::FORM;

type Cmd<T> = Result<T, CmdError>;

/// Сбой окна или меню: сами данные при этом уже сохранены.
fn window_error(e: tauri::Error) -> Error {
    Error::new(ErrorCode::Window).with("detail", e)
}

/// Вызов должен прийти из окна формы (ACL уже не пускает остальные, это вторая линия).
fn ensure_form(webview: &Webview) -> Cmd<()> {
    if webview.label() == FORM {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::Forbidden).into())
    }
}

/// Клиент хоста: тип (кэш или проба), токен по типу — GitLab: хранилище → `glab` → `GITLAB_TOKEN`,
/// GitHub: хранилище → `gh` → `GH_TOKEN`. Связка ключей может ждать ответа, CLI — запускаться,
/// поэтому поиск токена идёт в пуле блокирующих задач.
// ponytail: у `glab` и `gh` нет таймаута, зависший процесс займёт поток пула; добавить, если такое случится
async fn source_for(app: &AppHandle, host: &str) -> Result<AnySource, Error> {
    let host = normalize_host(host)?;
    let provider = resolve_provider(&host, &app.state::<AppState>().settings).await?;
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let tokens = &app.state::<AppState>().tokens;
        let env = |name: &str| std::env::var(name).ok();
        match provider {
            Provider::Gitlab => {
                let glab = Glab::find();
                let token = find_token(&host, tokens, &env, &|args| {
                    glab.as_ref().and_then(|glab| glab.value(args))
                })?;
                Ok(AnySource::Gitlab(gitlab::Client::new(&host, &token)?))
            }
            Provider::Github => {
                let gh = Gh::find();
                let token = find_github_token(&host, tokens, &env, &|args| {
                    gh.as_ref().and_then(|gh| gh.value(args))
                })?;
                Ok(AnySource::Github(github::Client::new(&host, &token)?))
            }
        }
    })
    .await
    .map_err(|e| Error::new(ErrorCode::Storage).with("detail", e))?
}

/// Хосты с источником токена и типом; CLI запускаются — поэтому в пуле блокирующих задач.
#[tauri::command]
pub async fn hosts(app: AppHandle, webview: Webview) -> Cmd<Vec<HostInfo>> {
    ensure_form(&webview)?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let env = |name: &str| std::env::var(name).ok();
        let glab = Glab::find();
        let gh = Gh::find();
        let mut hosts = list_hosts(&state.tokens, &env, &|args| {
            glab.as_ref().and_then(|glab| glab.value(args))
        })?;
        for found in github_hosts(&env, &|args| gh.as_ref().and_then(|gh| gh.value(args))) {
            if !hosts.iter().any(|h| h.host == found.host) {
                hosts.push(found);
            }
        }
        for host in hosts.iter_mut().filter(|h| h.provider.is_none()) {
            host.provider = state.settings.host_kind(&host.host)?;
        }
        Ok::<_, Error>(hosts)
    })
    .await
    .map_err(|e| Error::new(ErrorCode::Storage).with("detail", e))?
    .map_err(Into::<CmdError>::into)
}

#[tauri::command]
pub async fn set_token(
    webview: Webview,
    state: State<'_, AppState>,
    host: String,
    token: String,
) -> Cmd<()> {
    ensure_form(&webview)?;
    Ok(state.tokens.set(&host, &token)?)
}

#[tauri::command]
pub async fn remove_token(webview: Webview, state: State<'_, AppState>, host: String) -> Cmd<()> {
    ensure_form(&webview)?;
    Ok(state.tokens.remove(&host)?)
}

#[tauri::command]
pub async fn history(webview: Webview, state: State<'_, AppState>) -> Cmd<Vec<HistoryEntry>> {
    ensure_form(&webview)?;
    Ok(state.history.list()?)
}

#[tauri::command]
pub async fn remove_history(
    webview: Webview,
    state: State<'_, AppState>,
    at: String,
) -> Cmd<Vec<HistoryEntry>> {
    ensure_form(&webview)?;
    Ok(state.history.remove(&at)?)
}

#[tauri::command]
pub async fn clear_history(webview: Webview, state: State<'_, AppState>) -> Cmd<()> {
    ensure_form(&webview)?;
    Ok(state.history.clear()?)
}

#[tauri::command]
pub async fn projects(
    app: AppHandle,
    webview: Webview,
    host: String,
    search: String,
    after: Option<String>,
) -> Cmd<Page<Project>> {
    ensure_form(&webview)?;
    let source = source_for(&app, &host).await?;
    Ok(source.list_projects(&search, after.as_deref()).await?)
}

#[tauri::command]
pub async fn branches(
    app: AppHandle,
    webview: Webview,
    host: String,
    project: String,
    search: String,
) -> Cmd<Vec<String>> {
    ensure_form(&webview)?;
    let source = source_for(&app, &host).await?;
    Ok(source.list_branches(&project, &search).await?)
}

#[tauri::command]
pub async fn pipelines(
    app: AppHandle,
    webview: Webview,
    host: String,
    project: String,
    r#ref: Option<String>,
    workflow: Option<String>,
    after: Option<String>,
) -> Cmd<Page<Pipeline>> {
    ensure_form(&webview)?;
    let source = source_for(&app, &host).await?;
    Ok(source
        .recent_pipelines(
            &project,
            r#ref.as_deref(),
            workflow.as_deref(),
            after.as_deref(),
        )
        .await?)
}

/// Workflow проекта GitHub; у GitLab — пусто, экран проекта тогда не показывает выбор.
#[tauri::command]
pub async fn workflows(
    app: AppHandle,
    webview: Webview,
    host: String,
    project: String,
) -> Cmd<Vec<Workflow>> {
    ensure_form(&webview)?;
    let source = source_for(&app, &host).await?;
    Ok(source.list_workflows(&project).await?)
}

/// Тип хоста для подсказки о токене: из кэша или пробой.
#[tauri::command]
pub async fn host_provider(app: AppHandle, webview: Webview, host: String) -> Cmd<Provider> {
    ensure_form(&webview)?;
    let host = normalize_host(&host)?;
    Ok(resolve_provider(&host, &app.state::<AppState>().settings).await?)
}

fn history_label(meta: &Meta) -> HistoryLabel {
    HistoryLabel {
        project: meta.project.clone(),
        label: meta.label.clone(),
    }
}

/// Собирает отчёт, пишет историю и кладёт отчёт в память; возвращает его id. Прогресс идёт в `on_progress`.
#[tauri::command]
pub async fn build(
    app: AppHandle,
    webview: Webview,
    state: State<'_, AppState>,
    form: Form,
    on_progress: Channel<Progress>,
) -> Cmd<u32> {
    ensure_form(&webview)?;
    let parsed = parse_form(&form)?;
    // ссылка сама говорит, чей хост: проба не нужна
    if form.mode == FormMode::Link
        && let Some(provider) = link_provider(&form.url)
    {
        state.settings.set_host_kind(&parsed.host, provider)?;
    }
    let source = source_for(&app, &parsed.host).await?;
    if source.provider() == Provider::Github
        && matches!(parsed.request, Request::Aggregate { workflow: None, .. })
    {
        return Err(
            BTreeMap::from([(Field::Workflow, Error::new(ErrorCode::WorkflowRequired))]).into(),
        );
    }
    let now = OffsetDateTime::now_utc();
    let locale = state.locale();
    let built = build_report(
        &source,
        &parsed.request,
        BuildEnv { now, locale },
        |progress| {
            // форму могли закрыть, пока шла сборка: прогресс уже некому показывать
            let _ = on_progress.send(progress);
        },
    )
    .await?;
    let (Report::Single { meta, .. } | Report::Aggregate { meta, .. }) = &built.report;
    let label = history_label(meta);
    let html = render(&built.report, TEMPLATE);
    let json = serde_json::to_string(&built.report)
        .map_err(|e| Error::new(ErrorCode::Storage).with("detail", e))?;
    state.history.add(
        NewEntry {
            host: parsed.host.clone(),
            form,
            request: parsed.request.clone(),
            label,
        },
        now,
    )?;
    Ok(state.push_report(ReportEntry {
        host: parsed.host,
        request: parsed.request,
        json,
        html,
        file_name: built.file_name,
    }))
}

/// JSON отчёта по id; вытесненный из памяти — `report_expired`.
#[tauri::command]
pub async fn report(webview: Webview, state: State<'_, AppState>, id: u32) -> Cmd<String> {
    ensure_form(&webview)?;
    state
        .report_json(id)
        .ok_or_else(|| Error::new(ErrorCode::ReportExpired).with("id", id).into())
}

/// Повтор из «Недавних»: готовый отчёт с тем же хостом и запросом, если он ещё в памяти.
#[tauri::command]
pub async fn find_report(
    webview: Webview,
    state: State<'_, AppState>,
    host: String,
    request: Request,
) -> Cmd<Option<u32>> {
    ensure_form(&webview)?;
    Ok(state.find_report(&host, &request))
}

/// Экран отчёта сообщает, какой отчёт на экране (`None` — ушли с него): ему адресовано ⌘S.
#[tauri::command]
pub async fn set_current_report(
    webview: Webview,
    state: State<'_, AppState>,
    id: Option<u32>,
) -> Cmd<()> {
    ensure_form(&webview)?;
    state.set_current(id);
    Ok(())
}

#[tauri::command]
pub async fn saved_projects(
    webview: Webview,
    state: State<'_, AppState>,
) -> Cmd<Vec<SavedProject>> {
    ensure_form(&webview)?;
    Ok(state.projects.list()?)
}

/// Разбор ввода (может читать `.git/config`), проверка доступа через GraphQL, запись в `projects.json`.
#[tauri::command]
pub async fn add_project(
    app: AppHandle,
    webview: Webview,
    state: State<'_, AppState>,
    input: String,
) -> Cmd<SavedProject> {
    ensure_form(&webview)?;
    let project = tauri::async_runtime::spawn_blocking(move || parse_project_input(&input))
        .await
        .map_err(|e| Error::new(ErrorCode::Storage).with("detail", e))??;
    let source = source_for(&app, &project.host).await?;
    let found = source.fetch_project(&project.path).await?;
    let list = state
        .projects
        .add(project.clone(), &found.name, OffsetDateTime::now_utc())?;
    let saved = list
        .into_iter()
        .find(|p| p.host == project.host && p.path == project.path)
        .ok_or_else(|| Error::new(ErrorCode::Storage).with("detail", "проект не записался"))?;
    Ok(saved)
}

#[tauri::command]
pub async fn remove_project(
    webview: Webview,
    state: State<'_, AppState>,
    project: ProjectRef,
) -> Cmd<Vec<SavedProject>> {
    ensure_form(&webview)?;
    state.settings.forget_project(&project)?;
    Ok(state.projects.remove(&project)?)
}

#[tauri::command]
pub async fn get_settings(webview: Webview, state: State<'_, AppState>) -> Cmd<AppSettings> {
    ensure_form(&webview)?;
    Ok(state.settings.get()?)
}

/// Частичное обновление; смена языка пересобирает меню.
#[tauri::command]
pub async fn set_settings(
    app: AppHandle,
    webview: Webview,
    state: State<'_, AppState>,
    patch: SettingsPatch,
) -> Cmd<AppSettings> {
    ensure_form(&webview)?;
    let locale = patch.locale;
    let settings = state.settings.update(patch)?;
    if let Some(locale) = locale {
        menu::build(&app, locale)
            .and_then(|menu| app.set_menu(menu).map(drop))
            .map_err(window_error)?;
    }
    Ok(settings)
}

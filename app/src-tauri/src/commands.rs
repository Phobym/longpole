//! Команды формы. Токен не возвращает ни одна: форма видит только список хостов.
//! Все `async`: синхронная команда Tauri исполняется на главном потоке, а тут файлы и связка ключей.

use pipeline_trace_core::browse::{
    Page, Pipeline, Project, list_branches, list_projects, recent_pipelines,
};
use pipeline_trace_core::error::{CmdError, Error, ErrorCode};
use pipeline_trace_core::gitlab::Client;
use pipeline_trace_core::history::{HistoryEntry, HistoryLabel, NewEntry};
use pipeline_trace_core::hosts::normalize_host;
use pipeline_trace_core::render::render;
use pipeline_trace_core::report::{BuildEnv, Progress, build_report};
use pipeline_trace_core::request::{Form, parse_form};
use pipeline_trace_core::schema::{Locale, Meta, Report};
use pipeline_trace_core::tokens::{Glab, find_token};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State, Webview};
use time::OffsetDateTime;

use crate::menu;
use crate::reports::{TEMPLATE, open_report};
use crate::state::{AppState, ReportEntry};
use crate::strings::strings;
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

/// Клиент хоста с токеном: хранилище → `glab` → `GITLAB_TOKEN`.
/// Связка ключей может ждать ответа пользователя, `glab` — запускаться, поэтому поиск идёт в пуле блокирующих задач.
// ponytail: у `glab` нет таймаута, зависший процесс займёт поток пула; добавить, если такое случится
async fn client_for(app: &AppHandle, host: &str) -> Result<Client, Error> {
    let host = normalize_host(host)?;
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let glab = Glab::find();
        let token = find_token(
            &host,
            &app.state::<AppState>().tokens,
            &|name| std::env::var(name).ok(),
            &|args| glab.as_ref().and_then(|glab| glab.value(args)),
        )?;
        Client::new(&host, &token)
    })
    .await
    .map_err(|e| Error::new(ErrorCode::Storage).with("detail", e))?
}

#[tauri::command]
pub async fn hosts(webview: Webview, state: State<'_, AppState>) -> Cmd<Vec<String>> {
    ensure_form(&webview)?;
    Ok(state.tokens.hosts()?)
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
    let gql = client_for(&app, &host).await?;
    Ok(list_projects(&gql, &search, after.as_deref()).await?)
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
    let gql = client_for(&app, &host).await?;
    Ok(list_branches(&gql, &project, &search).await?)
}

#[tauri::command]
pub async fn pipelines(
    app: AppHandle,
    webview: Webview,
    host: String,
    project: String,
    r#ref: Option<String>,
    after: Option<String>,
) -> Cmd<Page<Pipeline>> {
    ensure_form(&webview)?;
    let gql = client_for(&app, &host).await?;
    Ok(recent_pipelines(&gql, &project, r#ref.as_deref(), after.as_deref()).await?)
}

fn history_label(meta: &Meta) -> HistoryLabel {
    HistoryLabel {
        project: meta.project.clone(),
        label: meta.label.clone(),
    }
}

/// Заголовок окна отчёта: фиксируется при создании, «все пайплайны» — на языке момента сборки.
fn report_title(label: &HistoryLabel, locale: Locale) -> String {
    let all = strings(locale).all_pipelines;
    format!(
        "{} · {}",
        label.project,
        label.label.as_deref().unwrap_or(all)
    )
}

/// Собирает отчёт, пишет историю и открывает окно отчёта; прогресс идёт в `on_progress`.
#[tauri::command]
pub async fn build(
    app: AppHandle,
    webview: Webview,
    state: State<'_, AppState>,
    form: Form,
    on_progress: Channel<Progress>,
) -> Cmd<()> {
    ensure_form(&webview)?;
    let parsed = parse_form(&form)?;
    let gql = client_for(&app, &parsed.host).await?;
    let now = OffsetDateTime::now_utc();
    let locale = state.locale();
    let built = build_report(
        &gql,
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
    let title = report_title(&label, locale);
    let html = render(&built.report, TEMPLATE);
    state.history.add(
        NewEntry {
            host: parsed.host,
            form,
            request: parsed.request,
            label,
        },
        now,
    )?;
    let entry = ReportEntry {
        html,
        file_name: built.file_name,
    };
    // окно не открылось — отчёт уже в истории, а ошибка окна — сбой самого приложения
    open_report(&app, entry, &title)
        .map_err(|e| Error::new(ErrorCode::Storage).with("detail", e))?;
    Ok(())
}

#[tauri::command]
pub async fn get_locale(webview: Webview, state: State<'_, AppState>) -> Cmd<Locale> {
    ensure_form(&webview)?;
    Ok(state.locale())
}

/// Сохраняет язык и пересобирает меню; открытые отчёты не меняются.
#[tauri::command]
pub async fn set_locale(
    app: AppHandle,
    webview: Webview,
    state: State<'_, AppState>,
    locale: Locale,
) -> Cmd<()> {
    ensure_form(&webview)?;
    state.settings.set_locale(locale)?;
    let rebuilt = menu::build(&app, locale).and_then(|menu| app.set_menu(menu).map(drop));
    rebuilt.map_err(window_error)?;
    Ok(())
}

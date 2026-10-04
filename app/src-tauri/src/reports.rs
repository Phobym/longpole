//! Отчёты в памяти: схема `report://`, открытие окна, «Сохранить».

use std::borrow::Cow;
use std::sync::atomic::Ordering;

use tauri::UriSchemeContext;
use tauri::http::header::CONTENT_TYPE;
use tauri::http::{Request, Response, StatusCode};
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::state::{AppState, ReportEntry, lock};
use crate::strings::strings;
use crate::windows::{create_report, focused};

/// Собранный отчёт-шаблон; `build.rs` проверяет, что он есть, и пересобирает при изменении.
pub const TEMPLATE: &str = include_str!("../../dist-report/report.html");

/// Обработчик `report://`: окно получает отчёт, привязанный к его label, больше ничего.
pub fn serve(
    ctx: UriSchemeContext<'_, Wry>,
    request: Request<Vec<u8>>,
) -> Response<Cow<'static, [u8]>> {
    let state = ctx.app_handle().state::<AppState>();
    let html = if matches!(request.uri().path(), "" | "/") {
        lock(&state.reports)
            .get(ctx.webview_label())
            .map(|entry| entry.html.clone())
    } else {
        None
    };
    let Some(html) = html else {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Cow::Borrowed(&[][..]))
            .expect("верный ответ");
    };
    Response::builder()
        .header(CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Cow::Owned(html.into_bytes()))
        .expect("верный ответ")
}

/// Новое окно отчёта; запись в памяти уходит вместе с окном (`on_window_event`).
pub fn open_report(app: &AppHandle, entry: ReportEntry, title: &str) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    let label = format!(
        "report-{}",
        state.next_report.fetch_add(1, Ordering::SeqCst) + 1
    );
    lock(&state.reports).insert(label.clone(), entry);
    create_report(app, &label, title).inspect_err(|_| {
        lock(&state.reports).remove(&label);
    })
}

/// ⌘S: сохраняется последнее сфокусированное окно, если это отчёт; форма и закрытое окно — ничего.
pub fn save_last_focused(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Some(window) = focused(app) else {
        return;
    };
    let Some(entry) = lock(&state.reports).get(window.label()).cloned() else {
        return;
    };
    let error_title = strings(state.locale()).save_error;
    let failed = app.clone();
    // неблокирующий: сохранение не держит главный поток
    app.dialog()
        .file()
        .set_parent(&window)
        .set_file_name(&entry.file_name)
        .add_filter("HTML", &["html"])
        .save_file(move |chosen| {
            let Some(chosen) = chosen else { return };
            let written = chosen
                .into_path()
                .map_err(|e| e.to_string())
                .and_then(|path| std::fs::write(path, entry.html).map_err(|e| e.to_string()));
            if let Err(detail) = written {
                failed
                    .dialog()
                    .message(detail)
                    .title(error_title)
                    .kind(MessageDialogKind::Error)
                    .show(|_| {});
            }
        });
}

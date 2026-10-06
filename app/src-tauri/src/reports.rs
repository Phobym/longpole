//! Отчёты в памяти: шаблон и «Сохранить отчёт…».

use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::state::AppState;
use crate::strings::strings;
use crate::windows::form;

/// Собранный отчёт-шаблон; `build.rs` проверяет, что он есть, и пересобирает при изменении.
pub const TEMPLATE: &str = include_str!("../../dist-report/report.html");

/// ⌘S: сохраняется отчёт на экране формы; нет отчёта или окна — ничего.
pub fn save_current(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Some((html, file_name)) = state.current_report() else {
        return;
    };
    let Some(window) = form(app) else {
        return;
    };
    let error_title = strings(state.locale()).save_error;
    let failed = app.clone();
    // неблокирующий: сохранение не держит главный поток
    app.dialog()
        .file()
        .set_parent(&window)
        .set_file_name(&file_name)
        .add_filter("HTML", &["html"])
        .save_file(move |chosen| {
            let Some(chosen) = chosen else { return };
            let written = chosen
                .into_path()
                .map_err(|e| e.to_string())
                .and_then(|path| std::fs::write(path, html).map_err(|e| e.to_string()));
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

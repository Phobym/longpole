#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod menu;
mod reports;
mod state;
mod strings;
mod windows;

use tauri::{Manager, RunEvent, WindowEvent};

use crate::state::{AppState, lock};

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // скрипт плагина открывает `target=_blank` через IPC, а окнам отчёта IPC запрещён:
        // ссылки открывает Rust из `on_navigation`/`on_new_window`
        .plugin(
            tauri_plugin_opener::Builder::new()
                .open_js_links_on_click(false)
                .build(),
        )
        .register_uri_scheme_protocol("report", reports::serve)
        .invoke_handler(tauri::generate_handler![
            commands::hosts,
            commands::set_token,
            commands::remove_token,
            commands::history,
            commands::remove_history,
            commands::clear_history,
            commands::projects,
            commands::branches,
            commands::pipelines,
            commands::build,
            commands::get_locale,
            commands::set_locale,
        ])
        .setup(|app| {
            app.manage(AppState::new(&app.path().app_data_dir()?));
            let locale = app.state::<AppState>().locale();
            app.set_menu(menu::build(app.handle(), locale)?)?;
            windows::create_form(app.handle())?;
            Ok(())
        })
        .on_menu_event(menu::on_event)
        .on_window_event(|window, event| {
            let state = window.state::<AppState>();
            match event {
                WindowEvent::Focused(true) => {
                    *lock(&state.last_focused) = Some(window.label().into());
                }
                WindowEvent::Destroyed => {
                    lock(&state.reports).remove(window.label());
                    lock(&state.zoom).remove(window.label());
                }
                _ => {}
            }
        })
        .build(tauri::generate_context!())
        .expect("не удалось собрать приложение");

    app.run(|app, event| match event {
        // macOS живёт в Dock без окон; выход — только явный (⌘Q, `app.exit`)
        #[cfg(target_os = "macos")]
        RunEvent::ExitRequested {
            code: None, api, ..
        } => api.prevent_exit(),
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => {
            if app.get_webview_window(windows::FORM).is_none()
                && let Err(e) = windows::create_form(app)
            {
                eprintln!("форма: {e}");
            }
        }
        _ => {
            let _ = app;
        }
    });
}

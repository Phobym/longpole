#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod menu;
mod reports;
mod state;
mod strings;
mod windows;

use tauri::{AppHandle, Manager, RunEvent, WindowEvent};

use crate::state::AppState;

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
                WindowEvent::Focused(true) => state.focus(window.label()),
                WindowEvent::Destroyed => state.forget(window.label()),
                _ => {}
            }
        })
        .build(tauri::generate_context!())
        .expect("не удалось собрать приложение");

    app.run(on_run_event);
}

/// macOS живёт в Dock без окон: выход только явный (⌘Q, `app.exit`), а клик по иконке возвращает форму.
/// Windows и Linux выходят после закрытия последнего окна, как в Tauri по умолчанию.
#[cfg(target_os = "macos")]
fn on_run_event(app: &AppHandle, event: RunEvent) {
    match event {
        RunEvent::ExitRequested {
            code: None, api, ..
        } => api.prevent_exit(),
        RunEvent::Reopen { .. } => {
            if app.get_webview_window(windows::FORM).is_none()
                && let Err(e) = windows::create_form(app)
            {
                eprintln!("форма: {e}");
            }
        }
        _ => {}
    }
}

#[cfg(not(target_os = "macos"))]
fn on_run_event(_: &AppHandle, _: RunEvent) {}

/// Три списка команд — `COMMANDS` в `build.rs`, `generate_handler!` и `capabilities/form.json` — ведутся
/// вручную: команда, пропущенная в одном, либо открыта окнам отчётов, либо не вызывается из формы.
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    fn quoted(text: &str) -> BTreeSet<String> {
        text.split('"')
            .skip(1)
            .step_by(2)
            .map(String::from)
            .collect()
    }

    #[test]
    fn списки_команд_совпадают() {
        let build = include_str!("../build.rs");
        let in_build = quoted(
            build
                .split("const COMMANDS: &[&str] = &[")
                .nth(1)
                .and_then(|rest| rest.split("];").next())
                .expect("COMMANDS в build.rs"),
        );
        let in_handler: BTreeSet<String> = include_str!("main.rs")
            .split("generate_handler![")
            .nth(1)
            .and_then(|rest| rest.split(']').next())
            .expect("generate_handler! в main.rs")
            .lines()
            .filter_map(|line| line.trim().strip_prefix("commands::"))
            .map(|name| name.trim_end_matches(',').to_string())
            .collect();
        let in_capability: BTreeSet<String> = quoted(include_str!("../capabilities/form.json"))
            .iter()
            .filter_map(|permission| permission.strip_prefix("allow-"))
            .map(|name| name.replace('-', "_"))
            .collect();
        assert_eq!(in_build.len(), 12);
        assert_eq!(in_build, in_handler);
        assert_eq!(in_build, in_capability);
    }
}

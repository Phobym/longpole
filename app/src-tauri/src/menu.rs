//! Меню приложения по таблице спеки (§ 5): строки из словаря Rust, пересборка при смене языка.

use pipeline_trace_core::schema::Locale;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu, SubmenuBuilder};
use tauri::{AppHandle, Manager, Wry};

use crate::reports::save_last_focused;
use crate::state::AppState;
use crate::strings::{Strings, strings};
use crate::windows::{focused, show_form};

const ZOOM_STEP: f64 = 1.2;

const MAC: bool = cfg!(target_os = "macos");
const WINDOWS: bool = cfg!(windows);

type Make = fn(&AppHandle, Option<&str>) -> tauri::Result<PredefinedMenuItem<Wry>>;

fn item(
    app: &AppHandle,
    id: &str,
    text: &str,
    accelerator: Option<&str>,
) -> tauri::Result<MenuItem<Wry>> {
    MenuItem::with_id(app, id, text, true, accelerator)
}

/// Готовый пункт с подписью из словаря.
fn ready(app: &AppHandle, make: Make, text: &str) -> tauri::Result<PredefinedMenuItem<Wry>> {
    make(app, Some(text))
}

pub fn build(app: &AppHandle, locale: Locale) -> tauri::Result<Menu<Wry>> {
    let s = strings(locale);
    let menu = Menu::new(app)?;
    if MAC {
        menu.append(&app_menu(app, s)?)?;
    }
    menu.append(&file_menu(app, s)?)?;
    menu.append(&edit_menu(app, s)?)?;
    menu.append(&view_menu(app, s)?)?;
    menu.append(&window_menu(app, s)?)?;
    Ok(menu)
}

/// Только macOS: меню приложения из готовых пунктов, как `Menu::default`.
fn app_menu(app: &AppHandle, s: &Strings) -> tauri::Result<Submenu<Wry>> {
    SubmenuBuilder::new(app, "pipeline-trace")
        .item(&PredefinedMenuItem::about(app, Some(s.about), None)?)
        .separator()
        .item(&ready(app, PredefinedMenuItem::services, s.services)?)
        .separator()
        .item(&ready(app, PredefinedMenuItem::hide, s.hide)?)
        .item(&ready(app, PredefinedMenuItem::hide_others, s.hide_others)?)
        .item(&ready(app, PredefinedMenuItem::show_all, s.show_all)?)
        .separator()
        .item(&ready(app, PredefinedMenuItem::quit, s.quit)?)
        .build()
}

/// Последний пункт: «Закрыть окно» на macOS, «Выход» готовый на Windows и свой на Linux (`app.exit(0)`).
fn file_menu(app: &AppHandle, s: &Strings) -> tauri::Result<Submenu<Wry>> {
    let menu = SubmenuBuilder::new(app, s.file)
        .item(&item(app, "new", s.new_report, Some("CmdOrCtrl+N"))?)
        .item(&item(app, "save", s.save_report, Some("CmdOrCtrl+S"))?)
        .separator();
    if MAC {
        menu.item(&ready(
            app,
            PredefinedMenuItem::close_window,
            s.close_window,
        )?)
    } else if WINDOWS {
        menu.item(&ready(app, PredefinedMenuItem::quit, s.exit)?)
    } else {
        menu.item(&item(app, "quit", s.exit, None)?)
    }
    .build()
}

/// Отменить/Повторить — только на macOS.
fn edit_menu(app: &AppHandle, s: &Strings) -> tauri::Result<Submenu<Wry>> {
    let mut menu = SubmenuBuilder::new(app, s.edit);
    if MAC {
        menu = menu
            .item(&ready(app, PredefinedMenuItem::undo, s.undo)?)
            .item(&ready(app, PredefinedMenuItem::redo, s.redo)?)
            .separator();
    }
    menu.item(&ready(app, PredefinedMenuItem::cut, s.cut)?)
        .item(&ready(app, PredefinedMenuItem::copy, s.copy)?)
        .item(&ready(app, PredefinedMenuItem::paste, s.paste)?)
        .item(&ready(app, PredefinedMenuItem::select_all, s.select_all)?)
        .build()
}

/// Масштаб — свои пункты везде; полный экран — готовый на macOS и свой на Windows и Linux.
fn view_menu(app: &AppHandle, s: &Strings) -> tauri::Result<Submenu<Wry>> {
    let menu = SubmenuBuilder::new(app, s.view)
        .item(&item(app, "zoom_reset", s.zoom_reset, Some("CmdOrCtrl+0"))?)
        .item(&item(app, "zoom_in", s.zoom_in, Some("CmdOrCtrl+="))?)
        .item(&item(app, "zoom_out", s.zoom_out, Some("CmdOrCtrl+-"))?)
        .separator();
    if MAC {
        menu.item(&ready(app, PredefinedMenuItem::fullscreen, s.fullscreen)?)
    } else {
        menu.item(&item(app, "fullscreen", s.fullscreen, Some("F11"))?)
    }
    .build()
}

/// Готовые пункты на macOS и Windows; на Linux почти пустое — допустимое отличие.
fn window_menu(app: &AppHandle, s: &Strings) -> tauri::Result<Submenu<Wry>> {
    let menu = SubmenuBuilder::new(app, s.window);
    if MAC || WINDOWS {
        let menu = menu
            .item(&ready(app, PredefinedMenuItem::minimize, s.minimize)?)
            .item(&ready(app, PredefinedMenuItem::maximize, s.maximize)?);
        if MAC {
            return menu
                .separator()
                .item(&ready(
                    app,
                    PredefinedMenuItem::close_window,
                    s.close_window,
                )?)
                .build();
        }
        return menu.build();
    }
    menu.item(&item(app, "minimize", s.minimize, None)?).build()
}

fn zoom(app: &AppHandle, change: impl Fn(f64) -> f64) -> tauri::Result<()> {
    let Some(window) = focused(app) else {
        return Ok(());
    };
    // замок масштаба `AppState` не держится во время `set_zoom`: тот ходит в главный поток
    window.set_zoom(app.state::<AppState>().zoom_by(window.label(), change))
}

pub fn on_event(app: &AppHandle, event: MenuEvent) {
    let done = match event.id().as_ref() {
        "new" => show_form(app),
        "save" => {
            save_last_focused(app);
            Ok(())
        }
        "quit" => {
            app.exit(0);
            Ok(())
        }
        "zoom_reset" => zoom(app, |_| 1.0),
        "zoom_in" => zoom(app, |z| z * ZOOM_STEP),
        "zoom_out" => zoom(app, |z| z / ZOOM_STEP),
        "fullscreen" => focused(app).map_or(Ok(()), |w| {
            w.is_fullscreen().and_then(|on| w.set_fullscreen(!on))
        }),
        "minimize" => focused(app).map_or(Ok(()), |w| w.minimize()),
        _ => Ok(()),
    };
    if let Err(e) = done {
        eprintln!("меню «{}»: {e}", event.id().as_ref());
    }
}

//! Меню приложения по таблице спеки (§ 5): строки из словаря Rust, пересборка при смене языка.

use pipeline_trace_core::schema::Locale;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, SubmenuBuilder};
use tauri::{AppHandle, Manager, Wry};

use crate::reports::save_last_focused;
use crate::state::{AppState, lock};
use crate::strings::strings;
use crate::windows::{focused, show_form};

const ZOOM_STEP: f64 = 1.2;
const ZOOM_MIN: f64 = 0.25;
const ZOOM_MAX: f64 = 5.0;

pub fn build(app: &AppHandle, locale: Locale) -> tauri::Result<Menu<Wry>> {
    let s = strings(locale);
    let mac = cfg!(target_os = "macos");
    let windows = cfg!(windows);
    let item = |id, text, accelerator| MenuItem::with_id(app, id, text, true, accelerator);
    let predefined =
        |make: fn(&AppHandle, Option<&str>) -> tauri::Result<PredefinedMenuItem<Wry>>, text| {
            make(app, Some(text))
        };
    let menu = Menu::new(app)?;

    if mac {
        let about = PredefinedMenuItem::about(app, Some(s.about), None)?;
        let services = PredefinedMenuItem::services(app, Some(s.services))?;
        let hide = predefined(PredefinedMenuItem::hide, s.hide)?;
        let hide_others = predefined(PredefinedMenuItem::hide_others, s.hide_others)?;
        let show_all = predefined(PredefinedMenuItem::show_all, s.show_all)?;
        let quit = predefined(PredefinedMenuItem::quit, s.quit)?;
        menu.append(
            &SubmenuBuilder::new(app, "pipeline-trace")
                .item(&about)
                .separator()
                .item(&services)
                .separator()
                .item(&hide)
                .item(&hide_others)
                .item(&show_all)
                .separator()
                .item(&quit)
                .build()?,
        )?;
    }

    // «Закрыть окно» на macOS; «Выход» — готовый на Windows и свой на Linux (`app.exit(0)`)
    let new = item("new", s.new_report, Some("CmdOrCtrl+N"))?;
    let save = item("save", s.save_report, Some("CmdOrCtrl+S"))?;
    let file = SubmenuBuilder::new(app, s.file)
        .item(&new)
        .item(&save)
        .separator();
    let file = if mac {
        file.item(&predefined(
            PredefinedMenuItem::close_window,
            s.close_window,
        )?)
    } else if windows {
        file.item(&predefined(PredefinedMenuItem::quit, s.exit)?)
    } else {
        file.item(&item("quit", s.exit, None)?)
    };
    menu.append(&file.build()?)?;

    // Отменить/Повторить — только на macOS
    let edit = SubmenuBuilder::new(app, s.edit);
    let edit = if mac {
        edit.item(&predefined(PredefinedMenuItem::undo, s.undo)?)
            .item(&predefined(PredefinedMenuItem::redo, s.redo)?)
            .separator()
    } else {
        edit
    };
    menu.append(
        &edit
            .item(&predefined(PredefinedMenuItem::cut, s.cut)?)
            .item(&predefined(PredefinedMenuItem::copy, s.copy)?)
            .item(&predefined(PredefinedMenuItem::paste, s.paste)?)
            .item(&predefined(PredefinedMenuItem::select_all, s.select_all)?)
            .build()?,
    )?;

    let view = SubmenuBuilder::new(app, s.view)
        .item(&item("zoom_reset", s.zoom_reset, Some("CmdOrCtrl+0"))?)
        .item(&item("zoom_in", s.zoom_in, Some("CmdOrCtrl+="))?)
        .item(&item("zoom_out", s.zoom_out, Some("CmdOrCtrl+-"))?)
        .separator();
    let view = if mac {
        view.item(&predefined(PredefinedMenuItem::fullscreen, s.fullscreen)?)
    } else {
        view.item(&item("fullscreen", s.fullscreen, Some("F11"))?)
    };
    menu.append(&view.build()?)?;

    let window = SubmenuBuilder::new(app, s.window);
    let window = if mac {
        window
            .item(&predefined(PredefinedMenuItem::minimize, s.minimize)?)
            .item(&predefined(PredefinedMenuItem::maximize, s.maximize)?)
            .separator()
            .item(&predefined(
                PredefinedMenuItem::close_window,
                s.close_window,
            )?)
    } else if windows {
        window
            .item(&predefined(PredefinedMenuItem::minimize, s.minimize)?)
            .item(&predefined(PredefinedMenuItem::maximize, s.maximize)?)
    } else {
        // почти пустое — допустимое отличие Linux
        window.item(&item("minimize", s.minimize, None)?)
    };
    menu.append(&window.build()?)?;
    Ok(menu)
}

fn zoom(app: &AppHandle, change: impl Fn(f64) -> f64) -> tauri::Result<()> {
    let Some(window) = focused(app) else {
        return Ok(());
    };
    // замок не держим во время `set_zoom`: тот ходит в главный поток, а главный берёт этот же замок
    let factor = {
        let state = app.state::<AppState>();
        let mut zooms = lock(&state.zoom);
        let factor = zooms.entry(window.label().into()).or_insert(1.0);
        *factor = change(*factor).clamp(ZOOM_MIN, ZOOM_MAX);
        *factor
    };
    window.set_zoom(factor)
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

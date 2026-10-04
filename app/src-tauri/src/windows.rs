//! Окна: форма и отчёты, запрет навигации и новых окон.

use tauri::webview::NewWindowResponse;
use tauri::{
    AppHandle, LogicalSize, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use tauri_plugin_opener::OpenerExt;

use crate::state::AppState;

pub const FORM: &str = "form";
const REPORT_SCHEME: &str = "report";
const APP_SCHEME: &str = "tauri";

/// Origin своей схемы: `<схема>://localhost` (macOS, Linux) или `http://<схема>.localhost` (Windows), без порта.
fn is_origin(url: &Url, scheme: &str) -> bool {
    if url.port().is_some() {
        return false;
    }
    match url.scheme() {
        s if s == scheme => url.host_str() == Some("localhost"),
        "http" => url
            .host_str()
            .is_some_and(|host| host.strip_suffix(".localhost") == Some(scheme)),
        _ => false,
    }
}

/// Адрес окна отчёта: ядро по нему зовёт обработчик `report://` с label окна.
fn report_url() -> Url {
    let url = if cfg!(windows) {
        "http://report.localhost/"
    } else {
        "report://localhost/"
    };
    url.parse().expect("верный адрес")
}

/// `https` — в системный браузер; остальное (в том числе `file://`) не открываем.
fn open_external(app: &AppHandle, url: &Url) {
    if url.scheme() == "https"
        && let Err(e) = app.opener().open_url(url.as_str(), None::<&str>)
    {
        eprintln!("не открылась ссылка {url}: {e}");
    }
}

struct Spec<'a> {
    label: &'a str,
    url: WebviewUrl,
    title: &'a str,
    size: LogicalSize<f64>,
}

/// Окно, из которого нельзя уйти со своей страницы. Чужой адрес не загружается, а `https` уходит
/// в системный браузер: на macOS WebKit спрашивает `on_navigation` и для `target=_blank` раньше, чем
/// `on_new_window`, поэтому внешние ссылки открываются здесь, а `on_new_window` — для остальных платформ.
fn open(
    app: &AppHandle,
    spec: Spec,
    own: impl Fn(&Url) -> bool + Send + 'static,
) -> tauri::Result<WebviewWindow> {
    let (navigating, opening) = (app.clone(), app.clone());
    let window = WebviewWindowBuilder::new(app, spec.label, spec.url)
        .title(spec.title)
        .inner_size(spec.size.width, spec.size.height)
        .on_navigation(move |url| {
            if own(url) {
                return true;
            }
            open_external(&navigating, url);
            false
        })
        .on_new_window(move |url, _| {
            open_external(&opening, &url);
            NewWindowResponse::Deny
        })
        .build()?;
    app.state::<AppState>().focus(spec.label);
    Ok(window)
}

pub fn create_form(app: &AppHandle) -> tauri::Result<()> {
    // в dev форму отдаёт vite: его origin тоже свой
    let dev = app
        .config()
        .build
        .dev_url
        .clone()
        .filter(|_| tauri::is_dev());
    let window = open(
        app,
        Spec {
            label: FORM,
            url: WebviewUrl::App("index.html".into()),
            title: "pipeline-trace",
            size: LogicalSize::new(1280.0, 860.0),
        },
        move |url| {
            is_origin(url, APP_SCHEME)
                || dev.as_ref().is_some_and(|dev| url.origin() == dev.origin())
        },
    )?;
    window.set_min_size(Some(LogicalSize::new(900.0, 640.0)))
}

/// Окно, которому адресованы ⌘S и пункты меню: получившее фокус последним, если оно ещё есть.
pub fn focused(app: &AppHandle) -> Option<WebviewWindow> {
    let label = app.state::<AppState>().focused_label()?;
    app.get_webview_window(&label)
}

/// «Новый отчёт»: фокус на форму или создать заново.
pub fn show_form(app: &AppHandle) -> tauri::Result<()> {
    let Some(form) = app.get_webview_window(FORM) else {
        return create_form(app);
    };
    form.unminimize()?;
    form.set_focus()
}

/// Окно отчёта; `title` фиксируется при создании.
pub fn create_report(app: &AppHandle, label: &str, title: &str) -> tauri::Result<()> {
    let spec = Spec {
        label,
        url: WebviewUrl::CustomProtocol(report_url()),
        title,
        size: LogicalSize::new(1440.0, 900.0),
    };
    open(app, spec, |url| is_origin(url, REPORT_SCHEME)).map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn own(scheme: &str, url: &str) -> bool {
        is_origin(&url.parse().unwrap(), scheme)
    }

    #[test]
    fn own_origin_on_every_platform() {
        assert!(own("tauri", "tauri://localhost/index.html"));
        assert!(own("tauri", "http://tauri.localhost/"));
        assert!(own("report", "report://localhost/"));
        assert!(own("report", "http://report.localhost/"));
        assert!(own("report", "http://report.localhost/#x"));
    }

    #[test]
    fn foreign_origin_is_refused() {
        assert!(!own("report", "https://gitlab.example.com/"));
        assert!(!own("report", "tauri://localhost/"));
        assert!(!own("tauri", "report://localhost/"));
        assert!(!own("report", "file:///etc/passwd"));
        // поддомен чужого хоста, похожий на свой
        assert!(!own("report", "http://report.localhost.evil.com/"));
        assert!(!own("report", "report://evil.com/"));
        // схемы и порты, которых в спеке нет
        assert!(!own("report", "https://report.localhost/"));
        assert!(!own("report", "http://report.localhost:8443/"));
        assert!(!own("report", "report://localhost:8443/"));
    }
}

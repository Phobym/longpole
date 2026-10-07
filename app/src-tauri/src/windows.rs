//! Окно формы: запрет навигации и новых окон.

use tauri::webview::NewWindowResponse;
use tauri::{
    AppHandle, LogicalSize, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use tauri_plugin_opener::OpenerExt;

pub const FORM: &str = "form";
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
            title: "Longpole",
            size: LogicalSize::new(1280.0, 860.0),
        },
        move |url| {
            is_origin(url, APP_SCHEME)
                || dev.as_ref().is_some_and(|dev| url.origin() == dev.origin())
        },
    )?;
    window.set_min_size(Some(LogicalSize::new(900.0, 640.0)))
}

/// Окно формы — единственное окно; ему адресованы ⌘S и пункты «Вид».
pub fn form(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(FORM)
}

/// «Новый отчёт»: фокус на форму или создать заново.
pub fn show_form(app: &AppHandle) -> tauri::Result<()> {
    let Some(form) = app.get_webview_window(FORM) else {
        return create_form(app);
    };
    form.unminimize()?;
    form.set_focus()
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
    }

    #[test]
    fn foreign_origin_is_refused() {
        assert!(!own("tauri", "https://gitlab.example.com/"));
        assert!(!own("tauri", "file:///etc/passwd"));
        // поддомен чужого хоста, похожий на свой
        assert!(!own("tauri", "http://tauri.localhost.evil.com/"));
        assert!(!own("tauri", "tauri://evil.com/"));
        // схемы и порты, которых в спеке нет
        assert!(!own("tauri", "https://tauri.localhost/"));
        assert!(!own("tauri", "http://tauri.localhost:8443/"));
        assert!(!own("tauri", "tauri://localhost:8443/"));
    }
}

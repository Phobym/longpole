//! Оболочка Selfhosted (ADR 0001, 0002): конфигурация инстанса и HTTP-сервер поверх ядра.

mod api;
mod auth;
mod cache;
pub mod config;

use std::sync::Arc;
use std::time::Instant;

use axum::Router;
use axum::extract::{FromRef, Request, State};
use axum::http::{HeaderValue, StatusCode, Uri, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum_extra::extract::PrivateCookieJar;
use axum_extra::extract::cookie::Key;
use longpole_core::error::ErrorCode;
use longpole_core::hosts::{HostInfo, TokenSource};
use longpole_core::web_oauth::App;
use rust_embed::{EmbeddedFile, RustEmbed};

use crate::auth::Refreshes;
use crate::cache::ReportCache;
use crate::config::{Config, Host};

/// Сборка фронта с `VITE_DELIVERY=selfhosted`; в release-бинарник встраивается целиком.
#[derive(RustEmbed)]
#[folder = "../dist"]
struct Frontend;

const CSP: &str = "default-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; \
                   object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'";

/// Состояние процесса: хосты конфига, ключ cookie и single-flight refresh.
#[derive(Clone)]
pub struct AppState(Arc<Server>);

struct Server {
    hosts: Vec<HostApp>,
    key: Key,
    /// `Origin` инстанса для сверки (CSRF)
    origin: String,
    hsts: bool,
    refreshes: Refreshes,
    reports: ReportCache,
}

/// Хост конфига с адресом провайдера и redirect URI его приложения.
struct HostApp {
    host: Host,
    base: String,
    redirect_uri: String,
}

impl HostApp {
    fn app(&self) -> App<'_> {
        App {
            provider: self.host.provider,
            base: &self.base,
            client_id: &self.host.client_id,
            client_secret: self.host.client_secret.expose(),
            redirect_uri: &self.redirect_uri,
        }
    }
}

impl AppState {
    /// `base` — адрес провайдера по домену: `https://<домен>`, тесты подставляют wiremock.
    pub fn new(config: Config, base: impl Fn(&str) -> String) -> Self {
        let hosts = config
            .hosts
            .into_iter()
            .map(|host| HostApp {
                base: base(&host.domain),
                // `url` — корень домена, со слэшем на конце
                redirect_uri: format!("{}auth/callback/{}", config.url, host.domain),
                host,
            })
            .collect();
        AppState(Arc::new(Server {
            hosts,
            key: Key::derive_from(config.cookie_key.expose()),
            origin: config.url.origin().ascii_serialization(),
            hsts: config.url.scheme() == "https",
            refreshes: Refreshes::default(),
            reports: ReportCache::new(
                usize::try_from(config.cache_max_mb.saturating_mul(1 << 20)).unwrap_or(usize::MAX),
            ),
        }))
    }

    fn host(&self, domain: &str) -> Option<&HostApp> {
        self.0.hosts.iter().find(|h| h.host.domain == domain)
    }
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Key {
        state.0.key.clone()
    }
}

pub fn app(config: Config) -> Router {
    router(AppState::new(config, |domain| format!("https://{domain}")))
}

/// `/auth/*`, `/api/*`, `/healthz`, статика фронта с fallback на `index.html`, заголовки безопасности (§ 11)
/// и лог запросов (§ 13).
pub fn router(state: AppState) -> Router {
    let hsts = state.0.hsts;
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/auth/login", get(auth::login))
        .route("/auth/callback/{domain}", get(auth::callback))
        .route("/auth/logout", post(auth::logout))
        .route("/api/hosts", post(hosts))
        .route("/api/host_provider", post(api::host_provider))
        .route("/api/projects", post(api::projects))
        .route("/api/branches", post(api::branches))
        .route("/api/pipelines", post(api::pipelines))
        .route("/api/workflows", post(api::workflows))
        .route("/api/add_project", post(api::add_project))
        .route("/api/build", post(api::build))
        .fallback(get(frontend))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::same_origin,
        ))
        .layer(middleware::map_response(move |response| {
            security_headers(response, hsts)
        }))
        .layer(middleware::from_fn(log_request))
        .with_state(state)
}

/// Хосты конфига: у хоста со входом — `Oauth` и логин. Попутно access обновляется лениво, отозванный вход
/// уходит и из cookie.
async fn hosts(State(state): State<AppState>, mut jar: PrivateCookieJar) -> Response {
    let mut list = Vec::new();
    for app in &state.0.hosts {
        let stored = auth::stored_login(&jar, &app.host.domain);
        let (next, session) = state.session(jar, &app.host.domain).await;
        jar = next;
        let login = match session {
            Ok(session) => Some(session.login),
            Err(e) if e.code == ErrorCode::AuthRequired => None,
            // сбой refresh — не выход: следующий запрос попробует снова
            Err(_) => stored,
        };
        list.push(HostInfo {
            host: app.host.domain.clone(),
            source: login.as_ref().map(|_| TokenSource::Oauth),
            provider: Some(app.host.provider),
            login,
        });
    }
    (jar, ndjson(&serde_json::json!({ "ok": list }))).into_response()
}

/// Ответ операции — NDJSON; у всех, кроме `build`, одна строка `{"ok":…}` или `{"err":…}` (§ 6).
fn ndjson(line: &serde_json::Value) -> Response {
    (
        [(header::CONTENT_TYPE, "application/x-ndjson")],
        format!("{line}\n"),
    )
        .into_response()
}

/// Файл сборки, иначе `index.html` — маршруты SPA (§ 8). Пространства сервера и статики не перехватываются.
async fn frontend(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    if let Some(file) = Frontend::get(path) {
        return file_response(file);
    }
    let first = path.split('/').next().unwrap_or_default();
    if matches!(first, "api" | "auth" | "healthz" | "assets") {
        return StatusCode::NOT_FOUND.into_response();
    }
    Frontend::get("index.html").map_or_else(|| StatusCode::NOT_FOUND.into_response(), file_response)
}

fn file_response(file: EmbeddedFile) -> Response {
    (
        [(header::CONTENT_TYPE, file.metadata.mimetype())],
        file.data,
    )
        .into_response()
}

async fn security_headers(mut response: Response, hsts: bool) -> Response {
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CSP),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("same-origin"),
    );
    if hsts {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000"),
        );
    }
    response
}

/// Путь без query: в query `/auth/callback` лежат OAuth `code` и `state`.
async fn log_request(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let started = Instant::now();
    let response = next.run(request).await;
    tracing::info!(
        %method,
        path = %path,
        status = response.status().as_u16(),
        ms = started.elapsed().as_millis() as u64,
        "request"
    );
    response
}

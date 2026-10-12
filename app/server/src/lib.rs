//! Оболочка Selfhosted (ADR 0001, 0002): конфигурация инстанса и HTTP-сервер поверх ядра.

pub mod config;

use std::time::Instant;

use axum::Router;
use axum::extract::Request;
use axum::http::{HeaderValue, StatusCode, Uri, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use rust_embed::{EmbeddedFile, RustEmbed};

use crate::config::Config;

/// Сборка фронта с `VITE_DELIVERY=selfhosted`; в release-бинарник встраивается целиком.
#[derive(RustEmbed)]
#[folder = "../dist"]
struct Frontend;

const CSP: &str = "default-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; \
                   object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'";

/// `/healthz`, статика фронта с fallback на `index.html`, заголовки безопасности (§ 11) и лог запросов (§ 13).
pub fn app(config: &Config) -> Router {
    let hsts = config.url.scheme() == "https";
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .fallback(get(frontend))
        .layer(middleware::map_response(move |response| {
            security_headers(response, hsts)
        }))
        .layer(middleware::from_fn(log_request))
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

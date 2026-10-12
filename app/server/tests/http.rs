use axum::body::{Body, to_bytes};
use axum::http::{Request, Response, StatusCode, header};
use longpole_server::app;
use longpole_server::config::Config;
use tower::ServiceExt;

/// Ждёт собранный фронт в `dist/` — как `generate_context!` у Desktop.
const INDEX: &[u8] = include_bytes!("../../dist/index.html");

fn config(url: &str) -> Config {
    let yaml = format!(
        "url: {url}\ncookie_key: MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=\nhosts:\n  - {{domain: gitlab.example.com, provider: gitlab, client_id: id, client_secret: s}}\n"
    );
    Config::parse(&yaml, |_| None).unwrap()
}

async fn get_from(url: &str, path: &str) -> Response<Body> {
    let request = Request::get(path).body(Body::empty()).unwrap();
    app(config(url)).oneshot(request).await.unwrap()
}

async fn get(path: &str) -> Response<Body> {
    get_from("https://longpole.example.com", path).await
}

async fn body(response: Response<Body>) -> Vec<u8> {
    to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec()
}

#[tokio::test]
async fn healthz_answers_ok() {
    let response = get("/healthz").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await, b"ok");
}

#[tokio::test]
async fn unknown_get_serves_index_html() {
    for path in [
        "/",
        "/gitlab.example.com/group/app/-/pipelines/42",
        "/project/gitlab.example.com/a?ref=main",
    ] {
        let response = get(path).await;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/html",
            "{path}"
        );
        assert_eq!(body(response).await, INDEX, "{path}");
    }
}

#[tokio::test]
async fn fallback_skips_api_auth_healthz_and_assets() {
    for path in [
        "/api",
        "/api/unknown",
        "/auth/unknown",
        "/healthz/x",
        "/assets/missing.js",
    ] {
        assert_eq!(get(path).await.status(), StatusCode::NOT_FOUND, "{path}");
    }
}

#[tokio::test]
async fn serves_built_assets() {
    let html = String::from_utf8(INDEX.to_vec()).unwrap();
    let script = html
        .split("src=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let response = get(script).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "text/javascript");
}

#[tokio::test]
async fn every_response_has_security_headers() {
    for path in ["/healthz", "/", "/api/hosts"] {
        let response = get(path).await;
        let headers = response.headers();
        assert_eq!(
            headers[header::CONTENT_SECURITY_POLICY],
            "default-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; \
             object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'",
            "{path}"
        );
        assert_eq!(headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff", "{path}");
        assert_eq!(headers[header::REFERRER_POLICY], "same-origin", "{path}");
        assert_eq!(
            headers[header::STRICT_TRANSPORT_SECURITY],
            "max-age=31536000",
            "{path}"
        );
    }
}

#[tokio::test]
async fn no_hsts_over_http() {
    let response = get_from("http://localhost:8080", "/healthz").await;
    assert!(
        response
            .headers()
            .get(header::STRICT_TRANSPORT_SECURITY)
            .is_none()
    );
    assert_eq!(
        response.headers()[header::X_CONTENT_TYPE_OPTIONS],
        "nosniff"
    );
}

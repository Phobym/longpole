//! Проба типа хоста и поведение клиента (редиректы) на wiremock; токен и заголовки проверяет
//! `tests/github.rs` через `mount_run`.
mod fixtures;

use pipeline_trace_core::error::ErrorCode;
use pipeline_trace_core::github::{Client, probe_at};
use pipeline_trace_core::source::Provider;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn проба_ghes_по_installed_version() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/meta"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "installed_version": "3.14.0" })),
        )
        .mount(&server)
        .await;
    assert_eq!(
        probe_at("ghe.example", &server.uri()).await.unwrap(),
        Provider::Github
    );
}

#[tokio::test]
async fn проба_gitlab_по_404_и_по_чужому_json() {
    let server = MockServer::start().await;
    assert_eq!(
        probe_at("gl.example", &server.uri()).await.unwrap(),
        Provider::Gitlab
    );
    Mock::given(method("GET"))
        .and(path("/api/v3/meta"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "message": "hi" })))
        .mount(&server)
        .await;
    assert_eq!(
        probe_at("gl.example", &server.uri()).await.unwrap(),
        Provider::Gitlab
    );
}

#[tokio::test]
async fn проба_без_соединения_это_network() {
    // порт 9 (discard) на localhost закрыт
    let result = probe_at("down.example", "http://127.0.0.1:9").await;
    assert_eq!(result.unwrap_err().code, ErrorCode::Network);
}

#[tokio::test]
async fn проба_ghes_приватного_режима_по_заголовку_при_401() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/meta"))
        .respond_with(
            ResponseTemplate::new(401).insert_header("x-github-enterprise-version", "3.14.0"),
        )
        .mount(&server)
        .await;
    assert_eq!(
        probe_at("ghe.example", &server.uri()).await.unwrap(),
        Provider::Github
    );
}

#[tokio::test]
async fn редирект_не_выполняется_и_это_http_status() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/runs/7"))
        .respond_with(
            ResponseTemplate::new(301).insert_header("location", "https://elsewhere.example/x"),
        )
        .mount(&server)
        .await;
    let client = Client::with_api_url("github.com", &server.uri(), "t").unwrap();
    fixtures::assert_error(
        client.fetch_run("o/r", "7").await,
        ErrorCode::HttpStatus,
        &[("host", "github.com"), ("status", "301")],
    );
}

#[tokio::test]
async fn проба_5xx_429_407_без_заголовка_это_ошибка_а_не_gitlab() {
    for status in [503, 429, 407] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/meta"))
            .respond_with(ResponseTemplate::new(status))
            .mount(&server)
            .await;
        let err = probe_at("ghe.example", &server.uri()).await.unwrap_err();
        assert_eq!(err.code, ErrorCode::HttpStatus, "{status}");
    }
}

#[tokio::test]
async fn проба_ghes_с_заголовком_при_503_всё_равно_github() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/meta"))
        .respond_with(
            ResponseTemplate::new(503).insert_header("x-github-enterprise-version", "3.14.0"),
        )
        .mount(&server)
        .await;
    assert_eq!(
        probe_at("ghe.example", &server.uri()).await.unwrap(),
        Provider::Github
    );
}

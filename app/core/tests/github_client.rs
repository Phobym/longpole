//! Проба типа хоста на wiremock. Запросы клиента с токеном проверяются в `tests/github.rs`
//! (Task 4–5) через `Source`: у клиента нет публичных методов запроса.
mod fixtures;

use pipeline_trace_core::error::ErrorCode;
use pipeline_trace_core::github::probe_at;
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

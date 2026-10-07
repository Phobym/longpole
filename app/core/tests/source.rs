//! Шов `Source`: GitLab через любой `Gql`.
mod fixtures;

use std::sync::Mutex;

use fixtures::fake_gql;
use pipeline_trace_core::settings::Settings;
use pipeline_trace_core::source::{Provider, Source, resolve_provider_at};
use serde_json::json;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn gitlab_номер_из_ссылки_становится_gid_а_gid_не_меняется() {
    let seen = Mutex::new(Vec::new());
    let gql = fake_gql(|v| {
        seen.lock().unwrap().push(v["id"].clone());
        json!({ "project": { "pipeline": null } })
    });
    let _ = gql.fetch_pipeline("g/p", "5").await;
    let _ = gql
        .fetch_pipeline("g/p", "gid://gitlab/Ci::Pipeline/6")
        .await;
    assert_eq!(
        *seen.lock().unwrap(),
        vec![
            json!("gid://gitlab/Ci::Pipeline/5"),
            json!("gid://gitlab/Ci::Pipeline/6")
        ]
    );
}

#[tokio::test]
async fn gitlab_провайдер_и_пустой_список_workflow() {
    let gql = fake_gql(|_| json!({}));
    assert_eq!(gql.provider(), Provider::Gitlab);
    assert_eq!(gql.list_workflows("g/p").await.unwrap(), vec![]);
}

#[tokio::test]
async fn тип_хоста_проба_один_раз_дальше_кэш() {
    let dir = TempDir::new().unwrap();
    let settings = Settings::new(dir.path().join("settings.json"));
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/meta"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "installed_version": "3.14.0" })),
        )
        .expect(1)
        .mount(&server)
        .await;
    for _ in 0..2 {
        assert_eq!(
            resolve_provider_at("ghe.example", &settings, &server.uri())
                .await
                .unwrap(),
            Provider::Github
        );
    }
}

#[tokio::test]
async fn github_com_без_пробы() {
    let dir = TempDir::new().unwrap();
    let settings = Settings::new(dir.path().join("settings.json"));
    // адрес, по которому никто не слушает: проба упала бы с `network`
    assert_eq!(
        resolve_provider_at("github.com", &settings, "http://127.0.0.1:9")
            .await
            .unwrap(),
        Provider::Github
    );
}

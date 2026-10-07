//! Шов `Source`: GitLab через любой `Gql`.
mod fixtures;

use std::sync::Mutex;

use fixtures::fake_gql;
use pipeline_trace_core::source::{Provider, Source};
use serde_json::json;

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

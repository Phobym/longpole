//! Перенос `test/browse.test.mjs`.
mod fixtures;

use std::sync::Mutex;

use fixtures::{assert_error, fake_gql};
use pipeline_trace_core::browse::{
    Commit, Page, Pipeline, Project, fetch_project, list_branches, list_projects, recent_pipelines,
};
use pipeline_trace_core::error::ErrorCode;
use serde_json::json;

#[tokio::test]
async fn list_projects_поля_курсор_пустой_поиск_передаётся_как_null() {
    let seen = Mutex::new(Vec::new());
    let gql = fake_gql(|v| {
        seen.lock().unwrap().push(v.clone());
        json!({ "projects": {
            "pageInfo": { "hasNextPage": true, "endCursor": "c1" },
            "nodes": [
                { "fullPath": "g/p", "nameWithNamespace": "G / P", "lastActivityAt": "2026-09-24T10:00:00Z",
                  "repository": { "rootRef": "main" } },
                { "fullPath": "g/empty", "nameWithNamespace": "G / Empty", "lastActivityAt": "2026-09-23T10:00:00Z",
                  "repository": null },
            ],
        } })
    });

    let page = list_projects(&gql, "  ", None).await.unwrap();
    assert_eq!(
        page,
        Page {
            items: vec![
                Project {
                    full_path: "g/p".into(),
                    name: "G / P".into(),
                    last_activity_at: Some("2026-09-24T10:00:00Z".into()),
                    default_branch: Some("main".into()),
                },
                Project {
                    full_path: "g/empty".into(),
                    name: "G / Empty".into(),
                    last_activity_at: Some("2026-09-23T10:00:00Z".into()),
                    default_branch: None,
                },
            ],
            next: Some("c1".into()),
        }
    );

    list_projects(&gql, "mono", Some("c1")).await.unwrap();
    let seen = seen.lock().unwrap();
    assert_eq!(seen[0], json!({ "search": null, "after": null }));
    assert_eq!(seen[1], json!({ "search": "mono", "after": "c1" }));
}

#[tokio::test]
async fn list_branches_шаблон_поиска_и_основная_ветка_первой() {
    let seen = Mutex::new(Vec::new());
    let gql = fake_gql(|v| {
        seen.lock().unwrap().push(v.clone());
        json!({ "project": { "repository": {
            "rootRef": "master", "branchNames": ["feature/a", "master", "fix/b"],
        } } })
    });
    assert_eq!(
        list_branches(&gql, "g/p", "a").await.unwrap(),
        ["master", "feature/a", "fix/b"]
    );
    list_branches(&gql, "g/p", "").await.unwrap();
    let seen = seen.lock().unwrap();
    assert_eq!(seen[0], json!({ "project": "g/p", "pattern": "*a*" }));
    assert_eq!(seen[1]["pattern"], "*");
}

#[tokio::test]
async fn list_branches_пустой_репозиторий_без_веток() {
    let gql = fake_gql(|_| json!({ "project": { "repository": null } }));
    assert!(list_branches(&gql, "g/p", "").await.unwrap().is_empty());
}

#[tokio::test]
async fn recent_pipelines_преобразование_полей_и_курсор() {
    let gql = fake_gql(|_| {
        json!({ "project": { "pipelines": {
            "pageInfo": { "hasNextPage": false, "endCursor": null },
            "nodes": [
                { "id": "gid://gitlab/Ci::Pipeline/1026324", "iid": "51256", "status": "MANUAL", "source": "push",
                  "createdAt": "2026-09-24T19:02:41+03:00", "duration": 3952,
                  "path": "/g/p/-/pipelines/1026324",
                  "commit": { "shortId": "718e7e48", "title": "Publish" }, "user": { "username": "u" } },
                { "id": "gid://gitlab/Ci::Pipeline/7", "iid": "1", "status": "RUNNING", "source": "web",
                  "createdAt": "2026-09-24T19:00:00+03:00", "duration": null,
                  "path": null, "commit": null, "user": null },
            ],
        } } })
    });
    assert_eq!(
        recent_pipelines(&gql, "g/p", Some("master"), None)
            .await
            .unwrap(),
        Page {
            items: vec![
                Pipeline {
                    id: "1026324".into(),
                    iid: "51256".into(),
                    status: "manual".into(),
                    source: Some("push".into()),
                    created_at: "2026-09-24T19:02:41+03:00".into(),
                    duration: Some(3_952_000),
                    commit: Some(Commit {
                        sha: "718e7e48".into(),
                        title: "Publish".into()
                    }),
                    author: Some("u".into()),
                    url: "https://h.example/g/p/-/pipelines/1026324".into(),
                },
                Pipeline {
                    id: "7".into(),
                    iid: "1".into(),
                    status: "running".into(),
                    source: Some("web".into()),
                    created_at: "2026-09-24T19:00:00+03:00".into(),
                    duration: None,
                    commit: None,
                    author: None,
                    url: "https://h.example/g/p/-/pipelines/7".into(),
                },
            ],
            next: None,
        }
    );
}

#[tokio::test]
async fn recent_pipelines_переменные_запроса() {
    let seen = Mutex::new(Vec::new());
    let gql = fake_gql(|v| {
        seen.lock().unwrap().push(v.clone());
        json!({ "project": { "pipelines": { "pageInfo": { "hasNextPage": false, "endCursor": null }, "nodes": [] } } })
    });
    recent_pipelines(&gql, "g/p", None, Some("c1"))
        .await
        .unwrap();
    assert_eq!(
        seen.lock().unwrap()[0],
        json!({ "project": "g/p", "ref": null, "after": "c1" })
    );
}

#[tokio::test]
async fn нет_проекта_это_ошибка_с_кодом() {
    let gql = fake_gql(|_| json!({ "project": null }));
    let params = [("project", "no/such"), ("host", "h.example")];
    assert_error(
        list_branches(&gql, "no/such", "").await,
        ErrorCode::ProjectNotFound,
        &params,
    );
    assert_error(
        recent_pipelines(&gql, "no/such", None, None).await,
        ErrorCode::ProjectNotFound,
        &params,
    );
}

#[tokio::test]
async fn fetch_project_поля_и_project_not_found_при_null() {
    let gql = fake_gql(|v| {
        if v["project"] == "g/p" {
            json!({ "project": { "fullPath": "g/p", "nameWithNamespace": "G / P", "lastActivityAt": null,
                                 "repository": { "rootRef": "main" } } })
        } else {
            json!({ "project": null })
        }
    });
    assert_eq!(
        fetch_project(&gql, "g/p").await.unwrap(),
        Project {
            full_path: "g/p".into(),
            name: "G / P".into(),
            last_activity_at: None,
            default_branch: Some("main".into()),
        }
    );
    assert_error(
        fetch_project(&gql, "g/none").await,
        ErrorCode::ProjectNotFound,
        &[("host", "h.example"), ("project", "g/none")],
    );
}

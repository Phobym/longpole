//! Run 7 на GHES для тестов кэша отчётов: сборку считаем по запросам джоб, проверку прав — по запросам run.
#![allow(dead_code)]

use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, header};
use longpole_server::router;
use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer};

use crate::fixtures::*;

pub const RUN: &str = "/api/v3/repos/o/r/actions/runs/7";
pub const JOBS: &str = "/api/v3/repos/o/r/actions/runs/7/jobs";

/// Run 7 в `o/r` (репозиторий 42) с одним джобом; `status` — `completed` или `in_progress`.
pub async fn mount_run(mock: &MockServer, status: &str, updated_at: &str) {
    // окно, в которое второй одновременный запрос застаёт сборку
    mount_run_after(mock, status, updated_at, Duration::from_millis(200)).await;
}

/// Как [`mount_run`], но джобы отвечают через `delay`.
pub async fn mount_run_after(mock: &MockServer, status: &str, updated_at: &str, delay: Duration) {
    Mock::given(method("GET"))
        .and(path(RUN))
        .respond_with(reply(
            200,
            json!({
                "id": 7, "name": "ci", "run_number": 12, "run_attempt": 1,
                "status": status, "conclusion": if status == "completed" { json!("success") } else { Value::Null },
                "created_at": "2026-10-07T10:00:00Z", "updated_at": updated_at,
                "run_started_at": "2026-10-07T10:00:00Z",
                "head_branch": "main", "head_sha": "abc123", "path": ".github/workflows/ci.yml",
                "html_url": format!("https://{GHES}/o/r/actions/runs/7"), "event": "push",
                "head_commit": {"message": "fix"}, "actor": {"login": "bob"},
                "repository": {"id": 42},
            }),
        ))
        .mount(mock)
        .await;
    Mock::given(method("GET"))
        .and(path(JOBS))
        .respond_with(
            reply(
                200,
                json!({"total_count": 1, "jobs": [{
                    "id": 1, "run_attempt": 1, "name": "build",
                    "status": "completed", "conclusion": "success",
                    "created_at": "2026-10-07T10:00:01Z", "started_at": "2026-10-07T10:00:05Z",
                    "completed_at": "2026-10-07T10:04:00Z",
                    "html_url": format!("https://{GHES}/o/r/actions/runs/7/job/1"),
                }]}),
            )
            .set_delay(delay),
        )
        .mount(mock)
        .await;
}

/// Сколько раз провайдер получил `GET` по пути.
pub async fn hits(mock: &MockServer, target: &str) -> usize {
    mock.received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|r| r.url.path() == target)
        .count()
}

pub fn build(cookie: &str, url: &str, locale: &str, fresh: bool) -> Request<Body> {
    let mut request = api(
        "build",
        cookie,
        json!({"form": {"mode": "link", "url": url}, "fresh": fresh}),
    );
    request
        .headers_mut()
        .insert(header::ACCEPT_LANGUAGE, locale.parse().unwrap());
    request
}

/// Последняя строка потока `build`.
pub async fn last(app: &Router, request: Request<Body>) -> Value {
    let body = to_bytes(send(app, request).await.into_body(), usize::MAX)
        .await
        .unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    serde_json::from_str(text.lines().last().unwrap()).unwrap()
}

pub async fn signed_in(mock: &MockServer) -> (Router, String) {
    let app = router(state(mock));
    let cookie = sign_in(&app, GHES, "").await;
    (app, cookie)
}

pub fn run_url() -> String {
    format!("https://{GHES}/o/r/actions/runs/7")
}

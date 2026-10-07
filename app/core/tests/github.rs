//! GitHub через wiremock: run → `RawPipeline` (спека, § 2), списки формы (§ 6).
mod fixtures;

use fixtures::assert_error;
use pipeline_trace_core::error::ErrorCode;
use pipeline_trace_core::github::Client;
use pipeline_trace_core::model::RawPipeline;
use serde_json::{Value, json};
use time::macros::datetime;
use wiremock::matchers::{header, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

const YAML: &str = "
jobs:
  build: {}
  lint:
    continue-on-error: true
  test:
    needs: build
    strategy:
      matrix:
        os: [linux, mac]
";

fn client(server: &MockServer) -> Client {
    Client::with_api_url("github.com", &server.uri(), "t").unwrap()
}

fn run_json() -> Value {
    json!({
        "id": 7, "name": "ci", "run_number": 12, "run_attempt": 2,
        "status": "completed", "conclusion": "success",
        "created_at": "2026-10-07T10:00:00Z", "updated_at": "2026-10-07T10:40:00Z",
        "run_started_at": "2026-10-07T10:30:00Z",
        "head_branch": "main", "head_sha": "abc123", "path": ".github/workflows/ci.yml",
        "html_url": "https://github.com/o/r/actions/runs/7", "event": "push",
        "head_commit": { "message": "fix: x\n\nтело" }, "actor": { "login": "u" },
    })
}

/// Время — минуты:секунды после 10:00.
fn job(
    id: u64,
    attempt: u32,
    name: &str,
    conclusion: &str,
    created: &str,
    started: &str,
    completed: &str,
) -> Value {
    let at = |ms: &str| format!("2026-10-07T10:{ms}Z");
    json!({
        "id": id, "run_attempt": attempt, "name": name,
        "status": "completed", "conclusion": conclusion,
        "created_at": at(created), "started_at": at(started), "completed_at": at(completed),
        "html_url": format!("https://github.com/o/r/actions/runs/7/job/{id}"),
    })
}

async fn mount_run(server: &MockServer, workflow: Option<&str>) {
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/runs/7"))
        .and(header("authorization", "Bearer t"))
        .and(header("x-github-api-version", "2022-11-28"))
        .respond_with(ResponseTemplate::new(200).set_body_json(run_json()))
        .mount(server)
        .await;
    let next = format!(
        "<{}/repos/o/r/actions/runs/7/jobs?filter=all&per_page=100&page=2>; rel=\"next\"",
        server.uri()
    );
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/runs/7/jobs"))
        .and(query_param("filter", "all"))
        .and(query_param_is_missing("page"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("link", next.as_str())
                .set_body_json(json!({
                    "total_count": 7,
                    "jobs": [
                        job(1, 1, "build", "failure", "00:01", "00:05", "01:00"),
                        job(2, 1, "lint", "success", "00:01", "00:05", "00:30"),
                        // пропущенная: GitHub отдаёт старт позже конца
                        job(3, 1, "test (linux)", "skipped", "01:00", "01:01", "01:00"),
                    ],
                })),
        )
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/runs/7/jobs"))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "total_count": 7,
            "jobs": [
                job(11, 2, "build", "success", "30:01", "30:05", "31:00"),
                // копия из первой попытки: те же времена
                job(12, 2, "lint", "success", "30:01", "00:05", "00:30"),
                job(13, 2, "test (linux)", "success", "31:01", "31:05", "33:00"),
                job(14, 2, "test (mac)", "success", "31:01", "31:05", "34:00"),
            ],
        })))
        .mount(server)
        .await;
    let contents = Mock::given(method("GET"))
        .and(path("/repos/o/r/contents/.github/workflows/ci.yml"))
        .and(query_param("ref", "abc123"))
        .and(header("accept", "application/vnd.github.raw"));
    match workflow {
        Some(text) => contents.respond_with(ResponseTemplate::new(200).set_body_string(text)),
        None => contents.respond_with(ResponseTemplate::new(404)),
    }
    .expect(1)
    .mount(server)
    .await;
}

/// (id, имя, стейдж, ретрай, needs, allow_failure) в порядке API.
fn jobs_view(raw: &RawPipeline) -> Vec<(&str, &str, &str, bool, Vec<&str>, bool)> {
    raw.jobs
        .iter()
        .map(|j| {
            (
                j.id.as_str(),
                j.name.as_str(),
                j.stage.as_str(),
                j.retried,
                j.needs.iter().map(String::as_str).collect(),
                j.allow_failure,
            )
        })
        .collect()
}

#[tokio::test]
async fn run_с_перезапуском_копиями_и_matrix() {
    let server = MockServer::start().await;
    mount_run(&server, Some(YAML)).await;
    let raw = client(&server).fetch_run("o/r", "7").await.unwrap();

    let p = &raw.pipeline;
    assert_eq!(
        (
            p.id.as_str(),
            p.iid.as_str(),
            p.status.as_str(),
            p.r#ref.as_str(),
            p.path.as_str()
        ),
        ("7", "12", "SUCCESS", "main", "/o/r/actions/runs/7")
    );
    assert_eq!(p.created_at, datetime!(2026-10-07 10:00:00 UTC));
    assert_eq!(p.finished_at, Some(datetime!(2026-10-07 10:40:00 UTC)));
    assert_eq!(p.stages, vec!["build · lint", "test"]);
    assert!(!raw.needs_missing);
    assert!(raw.downstream.is_empty());
    assert_eq!(
        jobs_view(&raw),
        vec![
            ("13", "test: [linux]", "test", false, vec!["build"], false),
            ("14", "test: [mac]", "test", false, vec!["build"], false),
            ("11", "build", "build · lint", false, vec![], false),
            ("12", "lint", "build · lint", false, vec![], true),
            ("1", "build", "build · lint", true, vec![], false),
        ]
    );
    let build = &raw.jobs[2];
    assert_eq!(build.queued_duration, Some(4.0));
    assert_eq!(build.web_path, "/o/r/actions/runs/7/job/11");
    assert_eq!(build.status, "SUCCESS");
    assert_eq!(raw.jobs[4].status, "FAILED");
}

#[tokio::test]
async fn без_файла_workflow_один_стейдж_и_признак() {
    let server = MockServer::start().await;
    mount_run(&server, None).await;
    let raw = client(&server).fetch_run("o/r", "7").await.unwrap();
    assert!(raw.needs_missing);
    assert_eq!(raw.pipeline.stages, vec!["ci"]);
    assert!(
        raw.jobs
            .iter()
            .all(|j| j.stage == "ci" && j.needs.is_empty())
    );
    // без YAML matrix не распознать: имя как в API
    assert!(raw.jobs.iter().any(|j| j.name == "test (linux)"));
}

#[tokio::test]
async fn файл_workflow_грузится_один_раз_на_клиент() {
    let server = MockServer::start().await;
    mount_run(&server, Some(YAML)).await;
    let client = client(&server);
    client.fetch_run("o/r", "7").await.unwrap();
    client.fetch_run("o/r", "7").await.unwrap();
    // `.expect(1)` на contents проверяется при остановке сервера
}

#[tokio::test]
async fn нет_run_это_pipeline_not_found() {
    let server = MockServer::start().await;
    assert_error(
        client(&server).fetch_run("o/r", "7").await,
        ErrorCode::PipelineNotFound,
        &[
            ("host", "github.com"),
            ("pipeline", "7"),
            ("project", "o/r"),
        ],
    );
}

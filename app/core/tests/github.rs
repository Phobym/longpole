//! GitHub через wiremock: run → `RawPipeline` (спека, § 2), списки формы (§ 6).
mod fixtures;

use fixtures::assert_error;
use pipeline_trace_core::browse::{Commit, Page, Pipeline, Project, Workflow};
use pipeline_trace_core::error::ErrorCode;
use pipeline_trace_core::github::Client;
use pipeline_trace_core::gitlab::PipelineFilter;
use pipeline_trace_core::model::RawPipeline;
use pipeline_trace_core::source::{Provider, Source};
use serde_json::{Value, json};
use std::collections::BTreeMap;
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

/// Run и его джобы, без файла workflow.
async fn mount_run_and_jobs(server: &MockServer) {
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
}

async fn mount_run(server: &MockServer, workflow: Option<&str>) {
    mount_run_and_jobs(server).await;
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
type JobView<'a> = (&'a str, &'a str, &'a str, bool, Vec<&'a str>, bool);

fn jobs_view(raw: &RawPipeline) -> Vec<JobView<'_>> {
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
    // копия из первой попытки: создана позже старта, очереди не было
    assert_eq!(raw.jobs[3].queued_duration, None);
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

#[tokio::test]
async fn временная_ошибка_файла_workflow_не_кэшируется() {
    let server = MockServer::start().await;
    mount_run_and_jobs(&server).await;
    // файл: сначала 500, потом YAML
    let contents =
        || Mock::given(method("GET")).and(path("/repos/o/r/contents/.github/workflows/ci.yml"));
    contents()
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    contents()
        .respond_with(ResponseTemplate::new(200).set_body_string(YAML))
        .mount(&server)
        .await;
    let client = client(&server);
    assert!(client.fetch_run("o/r", "7").await.unwrap().needs_missing);
    assert!(!client.fetch_run("o/r", "7").await.unwrap().needs_missing);
}

/// Run в списке: `minutes` — длительность последней попытки.
fn listed_run(id: u64, conclusion: &str, minutes: u32) -> Value {
    let mut run = run_json();
    run["id"] = json!(id);
    run["conclusion"] = json!(conclusion);
    run["run_started_at"] = json!("2026-10-07T10:00:00Z");
    run["updated_at"] = json!(format!("2026-10-07T10:{minutes:02}:00Z"));
    run
}

fn filter(statuses: Option<&[&str]>, last: usize) -> PipelineFilter {
    PipelineFilter {
        r#ref: Some("main".into()),
        source: Some("push".into()),
        statuses: statuses.map(|s| s.iter().map(|&x| x.into()).collect()),
        last,
        workflow: Some("ci.yml".into()),
    }
}

#[tokio::test]
async fn провайдер_github() {
    let server = MockServer::start().await;
    assert_eq!(client(&server).provider(), Provider::Github);
}

#[tokio::test]
async fn агрегат_один_статус_уходит_параметром_и_обрезается_по_last() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/workflows/ci.yml/runs"))
        .and(query_param("branch", "main"))
        .and(query_param("event", "push"))
        .and(query_param("status", "success"))
        .and(query_param("per_page", "100"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "workflow_runs": [
            listed_run(1, "success", 5), listed_run(2, "success", 6), listed_run(3, "success", 7),
        ] })),
        )
        .mount(&server)
        .await;
    let listed = client(&server)
        .list_pipelines("o/r", &filter(Some(&["SUCCESS"]), 2))
        .await
        .unwrap();
    assert_eq!(listed.ids, vec!["1", "2"]);
    assert_eq!(listed.counts, BTreeMap::from([("SUCCESS".to_string(), 2)]));
}

#[tokio::test]
async fn агрегат_несколько_статусов_фильтруются_на_клиенте() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/workflows/ci.yml/runs"))
        .and(query_param_is_missing("status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "workflow_runs": [
            listed_run(1, "success", 5), listed_run(2, "cancelled", 6), listed_run(3, "failure", 7),
        ] })),
        )
        .mount(&server)
        .await;
    let listed = client(&server)
        .list_pipelines("o/r", &filter(Some(&["SUCCESS", "FAILED"]), 50))
        .await
        .unwrap();
    assert_eq!(listed.ids, vec!["1", "3"]);
    assert_eq!(
        listed.counts,
        BTreeMap::from([("FAILED".to_string(), 1), ("SUCCESS".to_string(), 1)])
    );
}

#[tokio::test]
async fn pr_строит_самый_долгий_run_head_коммита() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/pulls/42"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "head": { "sha": "abc" } })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/runs"))
        .and(query_param("head_sha", "abc"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "workflow_runs": [
            listed_run(1, "success", 5), listed_run(2, "success", 20), listed_run(3, "success", 1),
        ] })),
        )
        .mount(&server)
        .await;
    assert_eq!(
        client(&server).head_pipeline("o/r", "42").await.unwrap(),
        "2"
    );
}

#[tokio::test]
async fn pr_без_pr_или_без_запусков_это_mr_has_no_pipeline() {
    let server = MockServer::start().await;
    assert_error(
        client(&server).head_pipeline("o/r", "42").await,
        ErrorCode::MrHasNoPipeline,
        &[("iid", "42"), ("project", "o/r")],
    );
    Mock::given(method("GET"))
        .and(path("/repos/o/r/pulls/42"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "head": { "sha": "abc" } })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/runs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "workflow_runs": [] })))
        .mount(&server)
        .await;
    assert_error(
        client(&server).head_pipeline("o/r", "42").await,
        ErrorCode::MrHasNoPipeline,
        &[("iid", "42"), ("project", "o/r")],
    );
}

fn repo_json(full_name: &str) -> Value {
    json!({ "full_name": full_name, "pushed_at": "2026-10-07T09:00:00Z", "default_branch": "main" })
}

fn project(full_name: &str) -> Project {
    Project {
        full_path: full_name.into(),
        name: full_name.into(),
        last_activity_at: Some("2026-10-07T09:00:00Z".into()),
        default_branch: Some("main".into()),
    }
}

#[tokio::test]
async fn проект_и_его_отсутствие() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r"))
        .respond_with(ResponseTemplate::new(200).set_body_json(repo_json("o/r")))
        .mount(&server)
        .await;
    assert_eq!(
        client(&server).fetch_project("o/r").await.unwrap(),
        project("o/r")
    );
    assert_error(
        client(&server).fetch_project("o/none").await,
        ErrorCode::ProjectNotFound,
        &[("host", "github.com"), ("project", "o/none")],
    );
}

#[tokio::test]
async fn свои_репозитории_поиск_по_странице_и_курсор_номером() {
    let server = MockServer::start().await;
    let next = format!(
        "<{}/user/repos?sort=pushed&per_page=100&page=2>; rel=\"next\"",
        server.uri()
    );
    Mock::given(method("GET"))
        .and(path("/user/repos"))
        .and(query_param("sort", "pushed"))
        .and(query_param("page", "1"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("link", next.as_str())
                .set_body_json(json!([repo_json("o/web"), repo_json("o/api")])),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/user/repos"))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([repo_json("o/web-2")])))
        .mount(&server)
        .await;
    let client = client(&server);
    assert_eq!(
        client.list_projects(" WEB ", None).await.unwrap(),
        Page {
            items: vec![project("o/web")],
            next: Some("2".into())
        }
    );
    assert_eq!(
        client.list_projects("", Some("2")).await.unwrap(),
        Page {
            items: vec![project("o/web-2")],
            next: None
        }
    );
}

#[tokio::test]
async fn ветки_основная_первой_и_фильтр() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r"))
        .respond_with(ResponseTemplate::new(200).set_body_json(repo_json("o/r")))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/branches"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            { "name": "dev" }, { "name": "feature/x" }, { "name": "main" },
        ])))
        .mount(&server)
        .await;
    let client = client(&server);
    assert_eq!(
        client.list_branches("o/r", "").await.unwrap(),
        vec!["main", "dev", "feature/x"]
    );
    assert_eq!(
        client.list_branches("o/r", "FEAT").await.unwrap(),
        vec!["feature/x"]
    );
}

#[tokio::test]
async fn последние_запуски_без_workflow_и_с_ним() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/runs"))
        .and(query_param("branch", "main"))
        .and(query_param("per_page", "20"))
        .and(query_param("page", "1"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "workflow_runs": [run_json()] })),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/workflows/ci.yml/runs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "workflow_runs": [] })))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server);
    assert_eq!(
        client
            .recent_pipelines("o/r", Some("main"), None, None)
            .await
            .unwrap(),
        Page {
            items: vec![Pipeline {
                id: "7".into(),
                iid: "12".into(),
                status: "success".into(),
                source: Some("push".into()),
                created_at: "2026-10-07T10:00:00.000Z".into(),
                duration: Some(600_000),
                commit: Some(Commit {
                    sha: "abc123".into(),
                    title: "fix: x".into()
                }),
                author: Some("u".into()),
                url: "https://github.com/o/r/actions/runs/7".into(),
            }],
            next: None,
        }
    );
    client
        .recent_pipelines("o/r", None, Some("ci.yml"), None)
        .await
        .unwrap();
}

#[tokio::test]
async fn workflow_только_активные_имя_файла_без_пути() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/workflows"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "workflows": [
            { "name": "CI", "path": ".github/workflows/ci.yml", "state": "active" },
            { "name": "Old", "path": ".github/workflows/old.yml", "state": "disabled_manually" },
        ] })),
        )
        .mount(&server)
        .await;
    assert_eq!(
        client(&server).list_workflows("o/r").await.unwrap(),
        vec![Workflow {
            file: "ci.yml".into(),
            name: "CI".into()
        }]
    );
}

#[tokio::test]
async fn агрегат_failed_без_параметра_status_и_timed_out_считается() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/workflows/ci.yml/runs"))
        .and(query_param_is_missing("status"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "workflow_runs": [
            listed_run(1, "timed_out", 5), listed_run(2, "success", 6), listed_run(3, "failure", 7),
        ] })),
        )
        .mount(&server)
        .await;
    let listed = client(&server)
        .list_pipelines("o/r", &filter(Some(&["FAILED"]), 50))
        .await
        .unwrap();
    assert_eq!(listed.ids, vec!["1", "3"]);
    assert_eq!(listed.counts, BTreeMap::from([("FAILED".to_string(), 2)]));
}

/// Две страницы агрегата; `Link` на вторую строится от адреса сервера.
async fn mount_two_pages(server: &MockServer, second_expected: u64) {
    let next = format!(
        "<{}/repos/o/r/actions/workflows/ci.yml/runs?page=2>; rel=\"next\"",
        server.uri()
    );
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/workflows/ci.yml/runs"))
        .and(query_param_is_missing("page"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("link", next.as_str())
                .set_body_json(json!({ "workflow_runs": [
                    listed_run(1, "success", 5), listed_run(2, "success", 6),
                ] })),
        )
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/workflows/ci.yml/runs"))
        .and(query_param("page", "2"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "workflow_runs": [
            listed_run(3, "success", 7), listed_run(4, "success", 8),
        ] })),
        )
        .expect(second_expected)
        .mount(server)
        .await;
}

#[tokio::test]
async fn агрегат_идёт_по_страницам_до_last() {
    let server = MockServer::start().await;
    mount_two_pages(&server, 1).await;
    let listed = client(&server)
        .list_pipelines("o/r", &filter(None, 3))
        .await
        .unwrap();
    assert_eq!(listed.ids, vec!["1", "2", "3"]);
}

#[tokio::test]
async fn агрегат_не_просит_лишнюю_страницу() {
    let server = MockServer::start().await;
    mount_two_pages(&server, 0).await;
    let listed = client(&server)
        .list_pipelines("o/r", &filter(None, 2))
        .await
        .unwrap();
    assert_eq!(listed.ids, vec!["1", "2"]);
}

#[tokio::test]
async fn ветка_и_имя_файла_workflow_кодируются() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/workflows/my%20ci.yml/runs"))
        .and(query_param("branch", "feature/a+b"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({ "workflow_runs": [
            listed_run(1, "success", 5),
        ] })),
        )
        .expect(1)
        .mount(&server)
        .await;
    let mut f = filter(None, 5);
    f.r#ref = Some("feature/a+b".into());
    f.workflow = Some("my ci.yml".into());
    let listed = client(&server).list_pipelines("o/r", &f).await.unwrap();
    assert_eq!(listed.ids, vec!["1"]);
}

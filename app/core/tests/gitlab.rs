//! Перенос `test/gitlab.test.mjs` (без `resolveHost`/`resolveToken` — они в S4).
mod fixtures;

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use fixtures::{assert_error, at, fake_gql};
use pipeline_trace_core::error::ErrorCode;
use pipeline_trace_core::gitlab::{
    Client, Gql, PipelineFilter, fetch_pipeline, list_pipelines, mr_head_pipeline,
};
use pipeline_trace_core::model::{RawJob, RawPipelineInfo};
use serde_json::{Value, json};
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn no_more() -> Value {
    json!({ "hasNextPage": false, "endCursor": null })
}

fn job_json(id: &str) -> Value {
    json!({
        "id": id, "name": id, "kind": "BUILD", "status": "SUCCESS",
        "startedAt": null, "finishedAt": null, "queuedDuration": null,
        "retried": false, "allowFailure": false, "webPath": "/g/p/-/jobs/1",
        "stage": { "name": "build" },
        "previousStageJobsOrNeeds": { "nodes": [] },
        "downstreamPipeline": null,
    })
}

fn bridge_json(id: &str, downstream_id: &str, downstream_project: &str) -> Value {
    let mut bridge = job_json(id);
    bridge["kind"] = json!("BRIDGE");
    bridge["downstreamPipeline"] =
        json!({ "id": downstream_id, "project": { "fullPath": downstream_project } });
    bridge
}

fn pipeline_data(id: &str, jobs: Vec<Value>, page_info: Value) -> Value {
    json!({ "project": { "pipeline": {
        "id": id, "iid": "1", "status": "SUCCESS", "createdAt": "2026-09-24T10:00:00+03:00",
        "finishedAt": null, "ref": "master", "path": "/p", "stages": { "nodes": [] },
        "jobs": { "nodes": jobs, "pageInfo": page_info },
    } } })
}

fn job_ids(jobs: &[RawJob]) -> Vec<&str> {
    jobs.iter().map(|j| j.id.as_str()).collect()
}

#[tokio::test]
async fn fetch_pipeline_склеивает_страницы_джоб() {
    let gql = fake_gql(|v| {
        if v["after"] == "c1" {
            pipeline_data(v["id"].as_str().unwrap(), vec![job_json("j2")], no_more())
        } else {
            let more = json!({ "hasNextPage": true, "endCursor": "c1" });
            pipeline_data(v["id"].as_str().unwrap(), vec![job_json("j1")], more)
        }
    });
    let raw = fetch_pipeline(&gql, "g/p", "P1").await.unwrap();
    assert_eq!(raw.project, "g/p");
    assert_eq!(raw.pipeline.id, "P1");
    assert_eq!(job_ids(&raw.jobs), ["j1", "j2"]);
    assert!(raw.downstream.is_empty());
}

#[tokio::test]
async fn fetch_pipeline_переносит_поля_пайплайна_и_джобы() {
    let mut job = job_json("j");
    job["name"] = json!("rspec 1/2");
    job["kind"] = json!("BRIDGE");
    job["status"] = json!("FAILED");
    job["startedAt"] = json!("2026-09-24T10:00:05+03:00");
    job["finishedAt"] = json!("2026-09-24T07:00:15Z");
    job["queuedDuration"] = json!(1.5);
    job["retried"] = json!(true);
    job["allowFailure"] = json!(true);
    job["webPath"] = json!("/g/p/-/jobs/7");
    job["stage"] = json!({ "name": "test" });
    job["previousStageJobsOrNeeds"] = json!({ "nodes": [{ "name": "build" }, { "name": "lint" }] });
    let mut data = pipeline_data("P1", vec![job], no_more());
    data["project"]["pipeline"]["finishedAt"] = json!("2026-09-24T10:30:00+03:00");
    data["project"]["pipeline"]["stages"] =
        json!({ "nodes": [{ "name": "build" }, { "name": "test" }] });

    let raw = fetch_pipeline(&fake_gql(|_| data.clone()), "g/p", "P1")
        .await
        .unwrap();

    assert_eq!(
        raw.pipeline,
        RawPipelineInfo {
            id: "P1".into(),
            iid: "1".into(),
            status: "SUCCESS".into(),
            created_at: at(0),
            finished_at: Some(at(1800)),
            r#ref: "master".into(),
            path: "/p".into(),
            stages: vec!["build".into(), "test".into()],
        }
    );
    assert_eq!(
        raw.jobs,
        [RawJob {
            id: "j".into(),
            name: "rspec 1/2".into(),
            bridge: true,
            status: "FAILED".into(),
            started_at: Some(at(5)),
            finished_at: Some(at(15)),
            queued_duration: Some(1.5),
            retried: true,
            allow_failure: true,
            web_path: "/g/p/-/jobs/7".into(),
            stage: "test".into(),
            needs: vec!["build".into(), "lint".into()],
        }]
    );
}

#[tokio::test]
async fn fetch_pipeline_загружает_downstream_из_его_проекта() {
    let seen = Mutex::new(Vec::new());
    let gql = fake_gql(|v| {
        let (project, id) = (v["project"].as_str().unwrap(), v["id"].as_str().unwrap());
        seen.lock()
            .unwrap()
            .push((project.to_string(), id.to_string()));
        match id {
            "P1" => pipeline_data("P1", vec![bridge_json("br", "P2", "other/proj")], no_more()),
            _ => pipeline_data("P2", vec![], no_more()),
        }
    });
    let raw = fetch_pipeline(&gql, "g/p", "P1").await.unwrap();
    assert_eq!(
        *seen.lock().unwrap(),
        [
            ("g/p".into(), "P1".into()),
            ("other/proj".into(), "P2".into())
        ]
    );
    assert_eq!(raw.downstream["br"].project, "other/proj");
}

#[tokio::test]
async fn fetch_pipeline_нет_проекта_или_пайплайна() {
    let no_project = fake_gql(|_| json!({ "project": null }));
    assert_error(
        fetch_pipeline(&no_project, "g/p", "P1").await,
        ErrorCode::ProjectNotFound,
        &[("project", "g/p"), ("host", "h.example")],
    );
    let no_pipeline = fake_gql(|_| json!({ "project": { "pipeline": null } }));
    assert_error(
        fetch_pipeline(&no_pipeline, "g/p", "P1").await,
        ErrorCode::PipelineNotFound,
        &[
            ("pipeline", "P1"),
            ("project", "g/p"),
            ("host", "h.example"),
        ],
    );
}

#[tokio::test]
async fn fetch_pipeline_неожиданная_форма_ответа_это_graphql_с_detail() {
    let gql = fake_gql(|_| json!({ "project": { "pipeline": { "id": 5 } } }));
    let err = fetch_pipeline(&gql, "g/p", "P1").await.unwrap_err();
    assert_eq!(err.code, ErrorCode::Graphql);
    assert_eq!(err.params["host"], "h.example");
    assert!(!err.params["detail"].is_empty());
}

#[tokio::test]
async fn fetch_pipeline_лимит_на_количество_страниц_джоб() {
    let pages = Mutex::new(0);
    let gql = fake_gql(|v| {
        let mut n = pages.lock().unwrap();
        *n += 1;
        let more = json!({ "hasNextPage": true, "endCursor": format!("c{n}") });
        pipeline_data(
            v["id"].as_str().unwrap(),
            vec![job_json(&format!("j{n}"))],
            more,
        )
    });
    assert_error(
        fetch_pipeline(&gql, "g/p", "P1").await,
        ErrorCode::TooManyJobs,
        &[("pipeline", "P1"), ("project", "g/p"), ("limit", "5000")],
    );
    assert_eq!(*pages.lock().unwrap(), 50);
}

#[tokio::test]
async fn fetch_pipeline_рекурсия_downstream_ограничена_глубиной_3() {
    let requests = Mutex::new(0);
    let gql = fake_gql(|v| {
        *requests.lock().unwrap() += 1;
        let id = v["id"].as_str().unwrap();
        let n: u32 = id[1..].parse().unwrap();
        let jobs = if n < 4 {
            vec![bridge_json(
                &format!("br{id}"),
                &format!("P{}", n + 1),
                "g/p",
            )]
        } else {
            vec![]
        };
        pipeline_data(id, jobs, no_more())
    });
    let raw = fetch_pipeline(&gql, "g/p", "P0").await.unwrap();
    assert_eq!(*requests.lock().unwrap(), 4);
    let p3 = &raw.downstream["brP0"].downstream["brP1"].downstream["brP2"];
    assert_eq!(p3.pipeline.id, "P3");
    assert!(p3.downstream.is_empty());
}

#[tokio::test]
async fn list_pipelines_фильтрует_статусы_считает_их_и_останавливается_на_last() {
    let pages = HashMap::from([
        (
            Value::Null,
            json!({
                "nodes": [{ "id": "a", "status": "SUCCESS" }, { "id": "b", "status": "FAILED" }],
                "pageInfo": { "hasNextPage": true, "endCursor": "c1" },
            }),
        ),
        (
            json!("c1"),
            json!({
                "nodes": [{ "id": "c", "status": "MANUAL" }, { "id": "d", "status": "SUCCESS" }],
                "pageInfo": { "hasNextPage": true, "endCursor": "c2" },
            }),
        ),
    ]);
    let vars = Mutex::new(Vec::new());
    let gql = fake_gql(|v| {
        vars.lock().unwrap().push(v.clone());
        json!({ "project": { "pipelines": pages[&v["after"]] } })
    });
    let filter = PipelineFilter {
        r#ref: Some("master".into()),
        source: None,
        statuses: Some(vec!["SUCCESS".into(), "MANUAL".into()]),
        last: 2,
    };
    let listed = list_pipelines(&gql, "g/p", &filter).await.unwrap();
    assert_eq!(listed.ids, ["a", "c"]);
    assert_eq!(
        listed.counts,
        BTreeMap::from([("SUCCESS".into(), 1), ("MANUAL".into(), 1)])
    );
    let vars = vars.lock().unwrap();
    assert_eq!(vars.len(), 2);
    assert_eq!(
        vars[0],
        json!({ "project": "g/p", "ref": "master", "source": null, "after": null })
    );
}

#[tokio::test]
async fn list_pipelines_нет_проекта() {
    let gql = fake_gql(|_| json!({ "project": null }));
    let filter = PipelineFilter {
        r#ref: None,
        source: None,
        statuses: None,
        last: 5,
    };
    assert_error(
        list_pipelines(&gql, "g/p", &filter).await,
        ErrorCode::ProjectNotFound,
        &[("project", "g/p"), ("host", "h.example")],
    );
}

#[tokio::test]
async fn mr_head_pipeline_возвращает_gid_и_падает_если_пайплайна_нет() {
    let with =
        fake_gql(|_| json!({ "project": { "mergeRequest": { "headPipeline": { "id": "P9" } } } }));
    assert_eq!(mr_head_pipeline(&with, "g/p", "12").await.unwrap(), "P9");

    let no_head = fake_gql(|_| json!({ "project": { "mergeRequest": { "headPipeline": null } } }));
    let no_mr = fake_gql(|_| json!({ "project": { "mergeRequest": null } }));
    for gql_result in [
        mr_head_pipeline(&no_head, "g/p", "12").await,
        mr_head_pipeline(&no_mr, "g/p", "12").await,
    ] {
        assert_error(
            gql_result,
            ErrorCode::MrHasNoPipeline,
            &[("iid", "12"), ("project", "g/p")],
        );
    }
}

fn client(server: &MockServer) -> Client {
    Client::with_base_url("h.example", &server.uri(), "tok").unwrap()
}

async fn query_answering(
    response: ResponseTemplate,
) -> Result<Value, pipeline_trace_core::error::Error> {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(response)
        .mount(&server)
        .await;
    client(&server).query("{ x }", json!({})).await
}

#[tokio::test]
async fn client_шлёт_post_с_bearer_токеном_и_возвращает_data() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/graphql"))
        .and(header("authorization", "Bearer tok"))
        .and(body_json(
            json!({ "query": "{ x }", "variables": { "a": 1 } }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "x": 1 } })))
        .expect(1)
        .mount(&server)
        .await;
    let gql = client(&server);
    assert_eq!(gql.host(), "h.example");
    assert_eq!(
        gql.query("{ x }", json!({ "a": 1 })).await.unwrap(),
        json!({ "x": 1 })
    );
}

#[tokio::test]
async fn client_ошибки_http_и_graphql_это_коды_с_параметрами() {
    for status in [401, 403] {
        assert_error(
            query_answering(ResponseTemplate::new(status)).await,
            ErrorCode::Unauthorized,
            &[("host", "h.example"), ("status", &status.to_string())],
        );
    }
    assert_error(
        query_answering(ResponseTemplate::new(502)).await,
        ErrorCode::HttpStatus,
        &[("host", "h.example"), ("status", "502")],
    );
    let errors = json!({ "errors": [{ "message": "Field x missing" }, { "message": "second" }] });
    assert_error(
        query_answering(ResponseTemplate::new(200).set_body_json(errors)).await,
        ErrorCode::Graphql,
        &[("host", "h.example"), ("detail", "Field x missing; second")],
    );
}

#[tokio::test]
async fn client_не_json_и_недоступный_хост_это_network_с_detail() {
    let not_json = ResponseTemplate::new(200).set_body_string("<html>login</html>");
    // на порту 1 никто не слушает: соединение отклоняется сразу
    let unreachable = Client::with_base_url("h.example", "http://127.0.0.1:1", "tok").unwrap();

    for result in [
        query_answering(not_json).await,
        unreachable.query("{ x }", json!({})).await,
    ] {
        let err = result.unwrap_err();
        assert_eq!(err.code, ErrorCode::Network);
        assert_eq!(err.params["host"], "h.example");
        assert!(!err.params["detail"].is_empty());
    }
}

#[tokio::test]
async fn client_держит_не_больше_4_запросов_одновременно() {
    let server = MockServer::start().await;
    let delay = Duration::from_millis(150);
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(delay)
                .set_body_json(json!({ "data": {} })),
        )
        .mount(&server)
        .await;
    let gql = client(&server);
    let started = Instant::now();
    futures::future::try_join_all((0..10).map(|_| gql.query("{ x }", json!({}))))
        .await
        .unwrap();
    // 10 запросов по 4 — минимум три «волны»; без лимита хватило бы одной
    assert!(
        started.elapsed() >= delay * 3,
        "прошло {:?}",
        started.elapsed()
    );
}

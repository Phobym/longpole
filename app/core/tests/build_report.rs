//! Сборка отчёта по запросу: `test/report.test.mjs` на фейковом `Gql`.
mod fixtures;

use std::sync::Mutex;

use fixtures::{assert_error, fake_gql};
use pipeline_trace_core::error::ErrorCode;
use pipeline_trace_core::report::{BuildEnv, Built, Progress, build_report, default_file_name};
use pipeline_trace_core::request::{Request, Status};
use pipeline_trace_core::schema::{Locale, Report};
use serde_json::{Value, json};
use time::macros::datetime;

const ENV: BuildEnv = BuildEnv {
    now: datetime!(2026-09-24 10:00:00 +03:00),
    locale: Locale::Ru,
};

fn no_more() -> Value {
    json!({ "hasNextPage": false, "endCursor": null })
}

fn pipeline_data(id: &str) -> Value {
    let iid = id.rsplit('/').next().unwrap();
    json!({ "project": { "pipeline": {
        "id": id, "iid": iid, "status": "SUCCESS", "createdAt": "2026-09-24T10:00:00+03:00",
        "finishedAt": null, "ref": "master", "path": "/p", "stages": { "nodes": [] },
        "jobs": { "nodes": [{
            "id": "gid://gitlab/Ci::Build/1", "name": "build", "kind": "BUILD", "status": "SUCCESS",
            "startedAt": "2026-09-24T10:00:00+03:00", "finishedAt": "2026-09-24T10:01:00+03:00",
            "queuedDuration": 1.0, "retried": false, "allowFailure": false, "webPath": "/g/p/-/jobs/1",
            "stage": { "name": "build" }, "previousStageJobsOrNeeds": { "nodes": [] },
            "downstreamPipeline": null,
        }], "pageInfo": no_more() },
    } } })
}

/// Отвечает на три запроса ядра; различает их по переменным: `iid` — MR, `id` — пайплайн, иначе список.
fn gitlab(listed: Vec<Value>, head: Option<&'static str>) -> impl Fn(&Value) -> Value + Sync {
    move |v| {
        if v.get("iid").is_some() {
            let head = head.map(|id| json!({ "id": id }));
            json!({ "project": { "mergeRequest": { "headPipeline": head } } })
        } else if let Some(id) = v.get("id") {
            pipeline_data(id.as_str().unwrap())
        } else {
            json!({ "project": { "pipelines": { "nodes": listed, "pageInfo": no_more() } } })
        }
    }
}

fn listed(statuses: &[&str]) -> Vec<Value> {
    statuses
        .iter()
        .enumerate()
        .map(|(i, s)| json!({ "id": format!("gid://gitlab/Ci::Pipeline/{}", i + 1), "status": s }))
        .collect()
}

fn aggregate(r#ref: Option<&str>, source: Option<&str>, last: u32) -> Request {
    Request::Aggregate {
        project: "g/p".into(),
        r#ref: r#ref.map(Into::into),
        source: source.map(Into::into),
        last,
        statuses: Some(vec![Status::Success]),
    }
}

async fn build(request: &Request, handler: impl Fn(&Value) -> Value + Sync) -> Built {
    build_report(&fake_gql(handler), request, ENV, |_| {})
        .await
        .expect("отчёт собран")
}

#[tokio::test]
async fn pipeline_одно_дерево_метка_по_имени_прогресс_0_и_1() {
    let progress = Mutex::new(Vec::new());
    let request = Request::Pipeline {
        project: "g/p".into(),
        pipeline_id: "5".into(),
    };
    let built = build_report(&fake_gql(gitlab(vec![], None)), &request, ENV, |p| {
        progress.lock().unwrap().push(p)
    })
    .await
    .unwrap();

    assert!(matches!(built.report, Report::Single { .. }));
    let meta = built.report.meta();
    assert_eq!(meta.host, "h.example");
    assert_eq!(meta.project, "g/p");
    assert_eq!(meta.label.as_deref(), Some("#5"));
    assert_eq!(meta.locale, Locale::Ru);
    assert_eq!(meta.status_counts, None);
    assert_eq!(meta.generated_at, "2026-09-24T07:00:00.000Z");
    assert_eq!(built.file_name, "pipeline-trace-g-p-5.html");
    assert_eq!(
        *progress.lock().unwrap(),
        [
            Progress {
                loaded: 0,
                total: 1
            },
            Progress {
                loaded: 1,
                total: 1
            }
        ]
    );
}

#[tokio::test]
async fn mr_пайплайн_берётся_из_head_pipeline_суффикс_mr_iid() {
    let request = Request::Mr {
        project: "g/p".into(),
        mr_iid: "7".into(),
    };
    let handler = gitlab(vec![], Some("gid://gitlab/Ci::Pipeline/9"));
    let built = build(&request, handler).await;
    assert_eq!(built.report.meta().label.as_deref(), Some("#9"));
    assert_eq!(built.file_name, "pipeline-trace-g-p-mr7.html");
}

#[tokio::test]
async fn aggregate_агрегат_метка_и_счётчики_статусов() {
    let handler = gitlab(listed(&["SUCCESS", "SUCCESS"]), None);
    let built = build(&aggregate(Some("master"), None, 2), handler).await;

    let Report::Aggregate { trees, .. } = &built.report else {
        panic!("режим агрегата")
    };
    assert_eq!(trees.len(), 2);
    let meta = built.report.meta();
    assert_eq!(meta.label.as_deref(), Some("master"));
    assert_eq!(
        meta.status_counts,
        Some([("SUCCESS".to_string(), 2)].into())
    );
    assert_eq!(built.file_name, "pipeline-trace-g-p-master.html");
}

#[tokio::test]
async fn aggregate_метка_из_ref_и_source_или_none_для_всех_пайплайнов() {
    let handler = || gitlab(listed(&["SUCCESS"]), None);
    let both = build(&aggregate(Some("master"), Some("push"), 1), handler()).await;
    assert_eq!(both.report.meta().label.as_deref(), Some("master · push"));

    let source = build(&aggregate(None, Some("push"), 1), handler()).await;
    assert_eq!(source.report.meta().label.as_deref(), Some("push"));
    assert_eq!(source.file_name, "pipeline-trace-g-p-push.html");

    let all = build(&aggregate(None, None, 1), handler()).await;
    assert_eq!(all.report.meta().label, None);
    assert_eq!(all.file_name, "pipeline-trace-g-p-all.html");
}

#[tokio::test]
async fn aggregate_прогресс_доходит_до_total_каждое_значение_один_раз() {
    let progress = Mutex::new(Vec::new());
    let handler = gitlab(listed(&["SUCCESS", "SUCCESS", "SUCCESS"]), None);
    build_report(&fake_gql(handler), &aggregate(None, None, 3), ENV, |p| {
        progress.lock().unwrap().push(p)
    })
    .await
    .unwrap();

    let progress = progress.into_inner().unwrap();
    assert_eq!(
        progress[0],
        Progress {
            loaded: 0,
            total: 3
        }
    );
    let mut loaded: Vec<usize> = progress.iter().map(|p| p.loaded).collect();
    loaded.sort();
    assert_eq!(loaded, [0, 1, 2, 3]);
    assert!(progress.iter().all(|p| p.total == 3));
}

#[tokio::test]
async fn aggregate_без_подходящих_пайплайнов_это_no_pipelines() {
    let result = build_report(
        &fake_gql(gitlab(vec![], None)),
        &aggregate(Some("x"), None, 5),
        ENV,
        |_| {},
    )
    .await;
    assert_error(result.map(|b| b.file_name), ErrorCode::NoPipelines, &[]);
}

#[tokio::test]
async fn ошибки_gitlab_идут_наружу_кодом() {
    let request = Request::Pipeline {
        project: "g/p".into(),
        pipeline_id: "5".into(),
    };
    let gql = fake_gql(|_| json!({ "project": { "pipeline": null } }));
    let result = build_report(&gql, &request, ENV, |_| {}).await;
    assert_error(
        result.map(|b| b.file_name),
        ErrorCode::PipelineNotFound,
        &[
            ("host", "h.example"),
            ("pipeline", "gid://gitlab/Ci::Pipeline/5"),
            ("project", "g/p"),
        ],
    );
}

#[test]
fn default_file_name_слаг_из_проекта_и_суффикса() {
    assert_eq!(
        default_file_name("group/proj", "master"),
        "pipeline-trace-group-proj-master.html"
    );
    assert_eq!(
        default_file_name("g/p", "feature/x y"),
        "pipeline-trace-g-p-feature-x-y.html"
    );
    assert_eq!(
        default_file_name("g.x/p_1", "a.b-c"),
        "pipeline-trace-g.x-p_1-a.b-c.html"
    );
}

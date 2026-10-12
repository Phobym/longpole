//! Кэш отчётов (спека `2026-10-12-delivery-variants-design.md`, § 7) через `POST /api/build`.

mod fixtures;
mod ghes_run;

use fixtures::*;
use ghes_run::*;
use serde_json::json;
use wiremock::Mock;
use wiremock::matchers::{method, path};

#[tokio::test]
async fn промах_строит_а_попадание_выдаётся_после_проверки_прав_на_языке_запроса() {
    let mock = providers(28800).await;
    mount_run(&mock, "completed", "2026-10-07T10:05:00Z").await;
    let (app, cookie) = signed_in(&mock).await;

    let first = last(&app, build(&cookie, &run_url(), "en", false)).await;
    assert_eq!(first["ok"]["report"]["meta"]["locale"], "en", "{first}");
    let runs_after_miss = hits(&mock, RUN).await;
    // по ссылке с id репозитория — тот же ключ
    Mock::given(method("GET"))
        .and(path("/api/v3/repositories/42"))
        .respond_with(reply(200, json!({"full_name": "o/r"})))
        .mount(&mock)
        .await;
    let url = format!("https://{GHES}/42/actions/runs/7");
    let second = last(&app, build(&cookie, &url, "ru-RU", false)).await;

    assert_eq!(hits(&mock, JOBS).await, 1, "отчёт построен один раз");
    assert_eq!(
        hits(&mock, RUN).await,
        runs_after_miss + 1,
        "попадание — после проверки прав"
    );
    let (mut a, b) = (first["ok"]["report"].clone(), &second["ok"]["report"]);
    assert_eq!(b["meta"]["locale"], "ru");
    assert_eq!(b["meta"]["generatedAt"], a["meta"]["generatedAt"]);
    a["meta"]["locale"] = json!("ru");
    assert_eq!(&a, b);
    assert_eq!(
        second["ok"]["entry"]["request"],
        json!({"mode": "pipeline", "project": "42", "pipelineId": "7"}),
        "запись истории — по запросу пользователя"
    );
}

#[tokio::test]
async fn без_прав_у_провайдера_отчёт_из_кэша_не_выдаётся() {
    for status in [403, 404] {
        let mock = providers(28800).await;
        mount_run(&mock, "completed", "2026-10-07T10:05:00Z").await;
        let (app, cookie) = signed_in(&mock).await;
        assert!(
            last(&app, build(&cookie, &run_url(), "en", false))
                .await
                .get("ok")
                .is_some()
        );
        mock.reset().await;
        Mock::given(method("GET"))
            .and(path(RUN))
            .respond_with(reply(status, json!({"message": "no"})))
            .mount(&mock)
            .await;

        let denied = last(&app, build(&cookie, &run_url(), "en", false)).await;
        assert_eq!(
            denied["err"]["code"], "pipelineNotFound",
            "{status}: {denied}"
        );
        assert_eq!(hits(&mock, JOBS).await, 0);
    }
}

#[tokio::test]
async fn смена_updated_at_перестраивает_отчёт() {
    let mock = providers(28800).await;
    mount_run(&mock, "completed", "2026-10-07T10:05:00Z").await;
    let (app, cookie) = signed_in(&mock).await;
    last(&app, build(&cookie, &run_url(), "en", false)).await;
    mock.reset().await;
    mount_run(&mock, "completed", "2026-10-07T11:00:00Z").await;

    last(&app, build(&cookie, &run_url(), "en", false)).await;
    last(&app, build(&cookie, &run_url(), "en", false)).await;

    assert_eq!(hits(&mock, JOBS).await, 1, "перестроен и снова в кэше");
}

#[tokio::test]
async fn незавершённый_пайплайн_не_кэшируется() {
    let mock = providers(28800).await;
    mount_run(&mock, "in_progress", "2026-10-07T10:05:00Z").await;
    let (app, cookie) = signed_in(&mock).await;
    for _ in 0..2 {
        let out = last(&app, build(&cookie, &run_url(), "en", false)).await;
        assert!(out.get("ok").is_some(), "{out}");
    }
    assert_eq!(hits(&mock, JOBS).await, 2);
}

#[tokio::test]
async fn перестроить_обходит_кэш() {
    let mock = providers(28800).await;
    mount_run(&mock, "completed", "2026-10-07T10:05:00Z").await;
    let (app, cookie) = signed_in(&mock).await;
    last(&app, build(&cookie, &run_url(), "en", false)).await;
    let fresh = last(&app, build(&cookie, &run_url(), "en", true)).await;
    assert!(fresh.get("ok").is_some(), "{fresh}");
    assert_eq!(hits(&mock, JOBS).await, 2);
}

#[tokio::test]
async fn одновременные_запросы_одного_ключа_строят_отчёт_один_раз() {
    let mock = providers(28800).await;
    mount_run(&mock, "completed", "2026-10-07T10:05:00Z").await;
    let (app, cookie) = signed_in(&mock).await;
    let (a, b) = tokio::join!(
        last(&app, build(&cookie, &run_url(), "en", false)),
        last(&app, build(&cookie, &run_url(), "en", false)),
    );
    assert!(a.get("ok").is_some() && b.get("ok").is_some(), "{a} {b}");
    assert_eq!(hits(&mock, JOBS).await, 1);
}

#[tokio::test]
async fn имя_файла_как_в_desktop_и_у_отчёта_из_кэша() {
    let mock = providers(28800).await;
    mount_run(&mock, "completed", "2026-10-07T10:05:00Z").await;
    Mock::given(method("GET"))
        .and(path("/api/v3/repos/o/r/pulls/5"))
        .respond_with(reply(200, json!({"head": {"sha": "abc123"}})))
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v3/repos/o/r/actions/runs"))
        .respond_with(reply(
            200,
            json!({"total_count": 1, "workflow_runs": [{
                "id": 7, "name": "ci", "run_number": 12, "run_attempt": 1,
                "status": "completed", "conclusion": "success",
                "created_at": "2026-10-07T10:00:00Z", "updated_at": "2026-10-07T10:05:00Z",
                "run_started_at": "2026-10-07T10:00:00Z",
                "head_branch": "main", "head_sha": "abc123", "path": ".github/workflows/ci.yml",
                "html_url": format!("https://{GHES}/o/r/actions/runs/7"), "event": "pull_request",
                "head_commit": {"message": "fix"}, "actor": {"login": "bob"},
            }]}),
        ))
        .mount(&mock)
        .await;
    let (app, cookie) = signed_in(&mock).await;

    // Desktop: `longpole-<путь>-<id пайплайна>.html`, у MR и PR — `-mr<номер>`
    let run = last(&app, build(&cookie, &run_url(), "en", false)).await;
    assert_eq!(run["ok"]["fileName"], "longpole-o-r-7.html", "{run}");
    let hit = last(&app, build(&cookie, &run_url(), "en", false)).await;
    assert_eq!(hit["ok"]["fileName"], "longpole-o-r-7.html");
    let pull = format!("https://{GHES}/o/r/pull/5");
    let pr = last(&app, build(&cookie, &pull, "en", false)).await;
    assert_eq!(pr["ok"]["fileName"], "longpole-o-r-mr5.html", "{pr}");
}

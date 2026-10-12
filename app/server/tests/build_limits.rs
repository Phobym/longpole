//! Лимиты построений (спека `2026-10-12-delivery-variants-design.md`, § 11) через `POST /api/build`.

mod fixtures;
mod ghes_run;

use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::Request;
use fixtures::*;
use ghes_run::*;
use longpole_server::config::Config;
use longpole_server::router;
use serde_json::Value;
use wiremock::MockServer;

async fn signed_in_with(mock: &MockServer, config: Config) -> (Router, String) {
    let uri = mock.uri();
    let app = router(longpole_server::AppState::new(config, move |_| uri.clone()));
    let cookie = sign_in(&app, GHES, "").await;
    (app, cookie)
}

/// Все строки потока `build`.
async fn lines(app: &Router, request: Request<Body>) -> Vec<Value> {
    let body = to_bytes(send(app, request).await.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(body.to_vec())
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[tokio::test]
async fn третье_одновременное_построение_пользователя_получает_too_many_builds() {
    let mock = providers(28800).await;
    mount_run(&mock, "completed", "2026-10-07T10:05:00Z").await;
    let (app, cookie) = signed_in(&mock).await;

    let (a, b, c) = tokio::join!(
        last(&app, build(&cookie, &run_url(), "en", true)),
        last(&app, build(&cookie, &run_url(), "en", true)),
        last(&app, build(&cookie, &run_url(), "en", true)),
    );

    let all = [&a, &b, &c];
    let refused = all.iter().filter(|r| r["err"]["code"] == "tooManyBuilds");
    assert_eq!(refused.count(), 1, "{a} {b} {c}");
    assert_eq!(all.iter().filter(|r| r.get("ok").is_some()).count(), 2);
    // построения закончились — лимит снова свободен
    let after = last(&app, build(&cookie, &run_url(), "en", false)).await;
    assert!(after.get("ok").is_some(), "{after}");
}

#[tokio::test]
async fn сверх_max_concurrent_builds_запрос_ждёт_в_очереди() {
    let mock = providers(28800).await;
    mount_run(&mock, "completed", "2026-10-07T10:05:00Z").await;
    let mut config = config();
    config.max_concurrent_builds = 1.try_into().unwrap();
    let (app, cookie) = signed_in_with(&mock, config).await;

    let (a, b) = tokio::join!(
        lines(&app, build(&cookie, &run_url(), "en", true)),
        lines(&app, build(&cookie, &run_url(), "en", true)),
    );

    let queued = |l: &Vec<Value>| l.iter().any(|l| l["queued"] == true);
    assert_eq!(
        [&a, &b].iter().filter(|l| queued(l)).count(),
        1,
        "{a:?} {b:?}"
    );
    for l in [&a, &b] {
        assert!(l.last().unwrap().get("ok").is_some(), "{l:?}");
    }
}

#[tokio::test]
async fn таймаут_даёт_build_timeout_и_не_оставляет_отчёт_в_кэше() {
    let mock = providers(28800).await;
    let slow = Duration::from_millis(500);
    mount_run_after(&mock, "completed", "2026-10-07T10:05:00Z", slow).await;
    let mut config = config();
    config.build_timeout = Duration::from_millis(100);
    let (app, cookie) = signed_in_with(&mock, config).await;

    let timed_out = last(&app, build(&cookie, &run_url(), "en", false)).await;
    assert_eq!(timed_out["err"]["code"], "buildTimeout", "{timed_out}");

    // и после того, как провайдер ответил бы, отменённая сборка ничего не кладёт в кэш
    tokio::time::sleep(slow).await;
    mock.reset().await;
    mount_run_after(&mock, "completed", "2026-10-07T10:05:00Z", Duration::ZERO).await;
    let next = last(&app, build(&cookie, &run_url(), "en", false)).await;
    assert!(next.get("ok").is_some(), "{next}");
    assert_eq!(
        hits(&mock, JOBS).await,
        1,
        "в кэше пусто — отчёт построен заново"
    );
}

//! Операции формы (спека `2026-10-12-delivery-variants-design.md`, § 6): `POST /api/<команда>` от имени
//! вошедшего пользователя, ответ — NDJSON, `host` — только домен из конфига.

mod fixtures;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Response, StatusCode};
use fixtures::*;
use longpole_server::router;
use serde_json::{Value, json};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer};

/// Строки NDJSON ответа.
async fn lines(response: Response<Body>) -> Vec<Value> {
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "application/x-ndjson");
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(body.to_vec())
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

async fn call(app: &Router, cmd: &str, cookie: &str, body: Value) -> Vec<Value> {
    lines(send(app, api(cmd, cookie, body)).await).await
}

#[tokio::test]
async fn домен_не_из_конфига_даёт_unknown_host() {
    let mock = providers(7200).await;
    let app = router(state(&mock));
    let cookie = sign_in(&app, GITLAB, "").await;
    let unknown =
        json!({"kind": "message", "code": "unknownHost", "params": {"host": "evil.example"}});
    for (cmd, body) in [
        ("host_provider", json!({"host": "evil.example"})),
        (
            "projects",
            json!({"host": "evil.example", "search": "", "after": null}),
        ),
        (
            "branches",
            json!({"host": "evil.example", "project": "g/p", "search": ""}),
        ),
        (
            "pipelines",
            json!({"host": "evil.example", "project": "g/p", "ref": null, "workflow": null, "after": null}),
        ),
        (
            "workflows",
            json!({"host": "evil.example", "project": "g/p"}),
        ),
        (
            "build",
            json!({"form": {"mode": "aggregate", "host": "evil.example", "project": "g/p", "last": "10", "statuses": ["ANY"]}}),
        ),
        (
            "build",
            json!({"form": {"mode": "link", "url": "https://evil.example/g/p/-/pipelines/1"}}),
        ),
    ] {
        assert_eq!(
            call(&app, cmd, &cookie, body).await,
            [json!({"err": unknown})],
            "{cmd}"
        );
    }
    assert!(
        mock.received_requests()
            .await
            .unwrap()
            .iter()
            .all(|r| !r.url.path().starts_with("/api/graphql")),
        "к провайдеру не ходили"
    );
}

/// Run 7 в `o/r` на GHES с одним джобом; файла workflow нет (404 wiremock по умолчанию).
async fn mount_run(mock: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/api/v3/repos/o/r/actions/runs/7"))
        .and(header("authorization", "Bearer ghu-a1"))
        .respond_with(reply(
            200,
            json!({
                "id": 7, "name": "ci", "run_number": 12, "run_attempt": 1,
                "status": "completed", "conclusion": "success",
                "created_at": "2026-10-07T10:00:00Z", "updated_at": "2026-10-07T10:05:00Z",
                "run_started_at": "2026-10-07T10:00:00Z",
                "head_branch": "main", "head_sha": "abc123", "path": ".github/workflows/ci.yml",
                "html_url": format!("https://{GHES}/o/r/actions/runs/7"), "event": "push",
                "head_commit": {"message": "fix"}, "actor": {"login": "bob"}, "repository": {"id": 42},
            }),
        ))
        .mount(mock)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v3/repos/o/r/actions/runs/7/jobs"))
        .respond_with(reply(
            200,
            json!({"total_count": 1, "jobs": [{
                "id": 1, "run_attempt": 1, "name": "build",
                "status": "completed", "conclusion": "success",
                "created_at": "2026-10-07T10:00:01Z", "started_at": "2026-10-07T10:00:05Z",
                "completed_at": "2026-10-07T10:04:00Z",
                "html_url": format!("https://{GHES}/o/r/actions/runs/7/job/1"),
            }]}),
        ))
        .mount(mock)
        .await;
}

#[tokio::test]
async fn поток_build_это_прогресс_и_последняя_строка_ok() {
    let mock = providers(28800).await;
    mount_run(&mock).await;
    let app = router(state(&mock));
    let cookie = sign_in(&app, GHES, "").await;
    let url = format!("https://{GHES}/o/r/actions/runs/7?token=secret");
    let out = call(
        &app,
        "build",
        &cookie,
        json!({"form": {"mode": "link", "url": url}}),
    )
    .await;
    let (last, progress) = out.split_last().unwrap();
    assert_eq!(
        progress,
        [
            json!({"progress": {"loaded": 0, "total": 1}}),
            json!({"progress": {"loaded": 1, "total": 1}}),
        ]
    );
    let ok = &last["ok"];
    assert_eq!(ok["report"]["mode"], "single", "{last}");
    assert_eq!(ok["report"]["meta"]["host"], GHES);
    assert_eq!(ok["entry"]["host"], GHES);
    assert_eq!(
        ok["entry"]["request"],
        json!({"mode": "pipeline", "project": "o/r", "pipelineId": "7"})
    );
    assert_eq!(
        ok["entry"]["form"]["url"],
        format!("https://{GHES}/o/r/actions/runs/7"),
        "ссылка в истории без query"
    );
}

#[tokio::test]
async fn ссылка_с_id_репозитория_строит_отчёт_по_его_пути() {
    let mock = providers(28800).await;
    mount_run(&mock).await;
    Mock::given(method("GET"))
        .and(path("/api/v3/repositories/42"))
        .respond_with(reply(200, json!({"full_name": "o/r"})))
        .mount(&mock)
        .await;
    let app = router(state(&mock));
    let cookie = sign_in(&app, GHES, "").await;
    let url = format!("https://{GHES}/42/actions/runs/7");
    let out = call(
        &app,
        "build",
        &cookie,
        json!({"form": {"mode": "link", "url": url}}),
    )
    .await;
    let ok = &out.last().unwrap()["ok"];
    assert_eq!(ok["report"]["meta"]["project"], "o/r", "{out:?}");
    assert_eq!(ok["entry"]["label"]["project"], "o/r");
    // запрос в истории остаётся по id: он переживёт переименование
    assert_eq!(
        ok["entry"]["request"],
        json!({"mode": "pipeline", "project": "42", "pipelineId": "7"})
    );
}

#[tokio::test]
async fn поток_build_с_ошибкой_провайдера_кончается_err() {
    let mock = providers(28800).await;
    let app = router(state(&mock));
    let cookie = sign_in(&app, GHES, "").await;
    let url = format!("https://{GHES}/o/r/actions/runs/404");
    let out = call(
        &app,
        "build",
        &cookie,
        json!({"form": {"mode": "link", "url": url}}),
    )
    .await;
    let (last, progress) = out.split_last().unwrap();
    assert_eq!(last["err"]["kind"], "message", "{out:?}");
    assert!(progress.iter().all(|l| l.get("progress").is_some()));
}

#[tokio::test]
async fn ссылка_с_чужим_origin_отклоняется() {
    let mock = providers(28800).await;
    let app = router(state(&mock));
    let cookie = sign_in(&app, GHES, "").await;
    let invalid = json!({"err": {"kind": "fields", "errors": {"url": {"code": "invalidLink", "params": {}}}}});
    for url in [
        format!("http://{GHES}/o/r/actions/runs/7"),
        format!("https://evil.example@{GHES}/o/r/actions/runs/7"),
    ] {
        assert_eq!(
            call(
                &app,
                "build",
                &cookie,
                json!({"form": {"mode": "link", "url": url}})
            )
            .await,
            std::slice::from_ref(&invalid),
            "{url}"
        );
    }
    assert!(
        mock.received_requests()
            .await
            .unwrap()
            .iter()
            .all(|r| !r.url.path().contains("/repos/")),
        "к провайдеру не ходили"
    );
}

#[tokio::test]
async fn ответ_401_при_живом_access_даёт_auth_required_и_стирает_вход() {
    let mock = providers(7200).await;
    Mock::given(method("POST"))
        .and(path("/api/graphql"))
        .respond_with(reply(401, json!({"message": "401 Unauthorized"})))
        .mount(&mock)
        .await;
    let app = router(state(&mock));
    let cookie = sign_in(&app, GITLAB, "").await;
    let response = send(
        &app,
        api(
            "projects",
            &cookie,
            json!({"host": GITLAB, "search": "", "after": null}),
        ),
    )
    .await;
    let (session, _) = set_cookie(response.headers(), "longpole_session").unwrap();
    assert_eq!(
        lines(response).await,
        [json!({"err": {"kind": "message", "code": "authRequired", "params": {"host": GITLAB}}})]
    );
    let hosts = call(
        &app,
        "hosts",
        &format!("longpole_session={session}"),
        json!({}),
    )
    .await;
    assert_eq!(hosts[0]["ok"][0]["login"], Value::Null, "вход стёрт");
}

#[tokio::test]
async fn ответ_401_во_время_build_кончает_поток_auth_required() {
    let mock = providers(28800).await;
    Mock::given(method("GET"))
        .and(path("/api/v3/repos/o/r/actions/runs/7"))
        .respond_with(reply(401, json!({"message": "Bad credentials"})))
        .mount(&mock)
        .await;
    let app = router(state(&mock));
    let cookie = sign_in(&app, GHES, "").await;
    let url = format!("https://{GHES}/o/r/actions/runs/7");
    let out = call(
        &app,
        "build",
        &cookie,
        json!({"form": {"mode": "link", "url": url}}),
    )
    .await;
    assert_eq!(
        out.last().unwrap(),
        &json!({"err": {"kind": "message", "code": "authRequired", "params": {"host": GHES}}})
    );
}

#[tokio::test]
async fn операции_формы_идут_к_провайдеру_с_токеном_пользователя() {
    let mock = providers(7200).await;
    Mock::given(method("GET"))
        .and(path("/api/v3/repos/o/r/actions/workflows"))
        .and(header("authorization", "Bearer ghu-a1"))
        .respond_with(reply(
            200,
            json!({"total_count": 1, "workflows": [
                {"id": 1, "name": "CI", "path": ".github/workflows/ci.yml", "state": "active"}
            ]}),
        ))
        .mount(&mock)
        .await;
    let app = router(state(&mock));
    let cookie = sign_in(&app, GHES, "").await;
    assert_eq!(
        call(&app, "host_provider", "", json!({"host": GHES})).await,
        [json!({"ok": "github"})]
    );
    let workflows = call(
        &app,
        "workflows",
        &cookie,
        json!({"host": GHES, "project": "o/r"}),
    )
    .await;
    assert_eq!(workflows[0]["ok"][0]["file"], "ci.yml", "{workflows:?}");
    // экран проекта по ссылке с id репозитория вместо пути
    Mock::given(method("GET"))
        .and(path("/api/v3/repositories/42"))
        .respond_with(reply(200, json!({"full_name": "o/r"})))
        .mount(&mock)
        .await;
    let by_id = call(
        &app,
        "workflows",
        &cookie,
        json!({"host": GHES, "project": "42"}),
    )
    .await;
    assert_eq!(by_id, workflows);
    let no_session = call(
        &app,
        "workflows",
        "",
        json!({"host": GHES, "project": "o/r"}),
    )
    .await;
    assert_eq!(no_session[0]["err"]["code"], "authRequired");
}

#[tokio::test]
async fn add_project_не_читает_папки_сервера() {
    let mock = providers(7200).await;
    let app = router(state(&mock));
    let cookie = sign_in(&app, GITLAB, "").await;
    assert_eq!(
        call(&app, "add_project", &cookie, json!({"input": "/app"})).await,
        [json!({"err": {"kind": "message", "code": "projectInputInvalid", "params": {}}})]
    );
}

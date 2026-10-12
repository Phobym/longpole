//! Вход в хосты инстанса и сессия (спека `2026-10-12-delivery-variants-design.md`, § 5): OAuth web flow
//! против wiremock, зашифрованная cookie, ленивый refresh с single-flight, выход, CSRF.

use std::time::Duration;

mod fixtures;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode, header};
use axum_extra::extract::cookie::{Key, PrivateCookieJar};
use fixtures::*;
use longpole_core::error::ErrorCode;
use longpole_server::router;
use serde_json::{Value, json};
use url::Url;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer};

/// `POST /api/hosts`: список и обновлённая cookie сессии, если сервер её прислал.
async fn hosts(app: &Router, cookie: &str) -> (Value, Option<String>) {
    let response = send(app, post("/api/hosts", cookie, URL)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = set_cookie(response.headers(), "longpole_session")
        .map(|(v, _)| format!("longpole_session={v}"));
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let line: Value = serde_json::from_slice(&body).unwrap();
    (line["ok"].clone(), cookie)
}

fn signed_in(hosts: &Value) -> Vec<(String, Value)> {
    hosts
        .as_array()
        .unwrap()
        .iter()
        .map(|h| (h["host"].as_str().unwrap().to_owned(), h["login"].clone()))
        .collect()
}

#[tokio::test]
async fn hosts_без_входа_отдаёт_хосты_конфига() {
    let mock = MockServer::start().await;
    let app = router(state(&mock));
    let (list, _) = hosts(&app, "").await;
    assert_eq!(
        list,
        json!([
            {"host": GITLAB, "source": null, "provider": "gitlab", "login": null},
            {"host": GHES, "source": null, "provider": "github", "login": null},
        ])
    );
}

#[tokio::test]
async fn вход_пишет_зашифрованную_сессию_и_hosts_отдаёт_логин() {
    let mock = providers(7200).await;
    let app = router(state(&mock));
    let login = send(&app, get(&format!("/auth/login?host={GITLAB}"), "")).await;
    assert!(location(&login).starts_with(&format!("{}/oauth/authorize?", mock.uri())));

    let cookie = sign_in(&app, GITLAB, "").await;
    for secret in ["gl-a1", "gl-r1", "ann"] {
        assert!(!cookie.contains(secret), "cookie не шифрована: {secret}");
    }
    let (list, _) = hosts(&app, &cookie).await;
    assert_eq!(
        list[0],
        json!({"host": GITLAB, "source": "oauth", "provider": "gitlab", "login": "ann"})
    );
    assert_eq!(list[1]["source"], Value::Null);

    // подделанная cookie не расшифровывается — входа нет
    let tampered = cookie.replace(&cookie[cookie.len() - 6..], "AAAAAA");
    assert_eq!(
        signed_in(&hosts(&app, &tampered).await.0),
        [(GITLAB.into(), Value::Null), (GHES.into(), Value::Null)]
    );
}

#[tokio::test]
async fn атрибуты_cookie_сессии() {
    let mock = providers(7200).await;
    let app = router(state(&mock));
    let login = send(&app, get(&format!("/auth/login?host={GHES}"), "")).await;
    let state_param = Url::parse(&location(&login))
        .unwrap()
        .query_pairs()
        .find(|(k, _)| k == "state")
        .unwrap()
        .1
        .into_owned();
    let (pending, _) = set_cookie(login.headers(), "longpole_oauth").unwrap();
    let callback = send(
        &app,
        get(
            &format!("/auth/callback/{GHES}?code=c1&state={state_param}"),
            &format!("longpole_oauth={pending}"),
        ),
    )
    .await;
    assert_eq!(location(&callback), "/", "return_to по умолчанию");
    let (_, attrs) = set_cookie(callback.headers(), "longpole_session").unwrap();
    for attr in [
        "HttpOnly",
        "Secure",
        "SameSite=Lax",
        "Path=/",
        "Max-Age=2592000",
    ] {
        assert!(attrs.contains(attr), "{attr}: {attrs}");
    }
    let (_, removed) = set_cookie(callback.headers(), "longpole_oauth").unwrap();
    assert!(removed.contains("Max-Age=0"), "{removed}");
}

#[tokio::test]
async fn return_to_только_относительный_путь() {
    let mock = MockServer::start().await;
    let app = router(state(&mock));
    for return_to in [
        "//evil.example/x",
        "https://evil.example/x",
        "/%5Cevil.example",
        // браузер выкидывает табуляцию и перевод строки из `Location`: вышло бы `//evil.example`
        "/%09/evil.example",
        "/%0A/evil.example",
        "x",
    ] {
        let response = send(
            &app,
            get(
                &format!("/auth/login?host={GITLAB}&return_to={return_to}"),
                "",
            ),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{return_to}");
    }
    let unknown = send(&app, get("/auth/login?host=evil.example", "")).await;
    assert_eq!(unknown.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn callback_с_чужим_state_отклоняется() {
    let mock = providers(7200).await;
    let app = router(state(&mock));
    let login = send(&app, get(&format!("/auth/login?host={GITLAB}"), "")).await;
    let (pending, _) = set_cookie(login.headers(), "longpole_oauth").unwrap();
    let callback = send(
        &app,
        get(
            &format!("/auth/callback/{GITLAB}?code=c1&state=forged"),
            &format!("longpole_oauth={pending}"),
        ),
    )
    .await;
    assert_eq!(callback.status(), StatusCode::BAD_REQUEST);
    assert_eq!(set_cookie(callback.headers(), "longpole_session"), None);
}

#[tokio::test]
async fn два_параллельных_запроса_с_истекающим_access_делают_один_refresh() {
    let mock = providers(60).await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains(
            "grant_type=refresh_token&refresh_token=gl-r1",
        ))
        .respond_with(
            reply(
                200,
                json!({"access_token": "gl-a2", "refresh_token": "gl-r2", "expires_in": 7200}),
            )
            .set_delay(Duration::from_millis(200)),
        )
        .expect(1)
        .mount(&mock)
        .await;
    let app = router(state(&mock));
    let cookie = sign_in(&app, GITLAB, "").await;
    let ((first, renewed), (second, _)) = tokio::join!(hosts(&app, &cookie), hosts(&app, &cookie));
    assert_eq!(first[0]["login"], "ann");
    assert_eq!(second[0]["login"], "ann");
    // новая cookie уходит в том же ответе; с ней refresh больше не нужен
    let renewed = renewed.expect("новая cookie");
    assert_eq!(hosts(&app, &renewed).await.1, None);
}

#[tokio::test]
async fn invalid_grant_даёт_auth_required_и_убирает_хост_из_cookie() {
    let mock = providers(60).await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .respond_with(reply(400, json!({"error": "invalid_grant"})))
        .mount(&mock)
        .await;
    let state = state(&mock);
    let app = router(state.clone());
    let cookie = sign_in(&app, GITLAB, "").await;
    let cookie = sign_in(&app, GHES, &cookie).await;

    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    let jar = PrivateCookieJar::from_headers(&headers, Key::derive_from(&base64(KEY)));
    let (jar, access) = state.access(jar, GITLAB).await;
    let e = access.unwrap_err();
    assert_eq!(e.code, ErrorCode::AuthRequired);
    assert_eq!(e.params["host"], GITLAB);
    let session = jar.get("longpole_session").unwrap();
    assert!(!session.value().contains(GITLAB), "{}", session.value());
    assert!(session.value().contains(GHES), "второй хост остался");

    let (list, renewed) = hosts(&app, &cookie).await;
    assert_eq!(
        signed_in(&list),
        [(GITLAB.into(), Value::Null), (GHES.into(), json!("bob"))]
    );
    assert!(renewed.is_some(), "хост убран и из cookie браузера");
}

#[tokio::test]
async fn сбой_refresh_не_выход() {
    let mock = providers(60).await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .respond_with(reply(502, json!({})))
        .mount(&mock)
        .await;
    let app = router(state(&mock));
    let cookie = sign_in(&app, GITLAB, "").await;
    let (list, renewed) = hosts(&app, &cookie).await;
    assert_eq!(
        list[0]["login"], "ann",
        "вход остаётся до следующей попытки"
    );
    assert_eq!(renewed, None);
}

#[tokio::test]
async fn access_без_входа_auth_required() {
    let mock = MockServer::start().await;
    let state = state(&mock);
    let jar = PrivateCookieJar::new(Key::derive_from(&base64(KEY)));
    let (_, access) = state.access(jar, GITLAB).await;
    assert_eq!(access.unwrap_err().code, ErrorCode::AuthRequired);
}

#[tokio::test]
async fn origin_чужого_сайта_отклоняется() {
    let mock = providers(7200).await;
    let app = router(state(&mock));
    let cookie = sign_in(&app, GITLAB, "").await;
    for origin in [
        "https://evil.example",
        "null",
        "http://longpole.example.com",
    ] {
        let api = send(&app, post("/api/hosts", &cookie, origin)).await;
        assert_eq!(api.status(), StatusCode::FORBIDDEN, "{origin}");
        let logout = send(
            &app,
            post(&format!("/auth/logout?host={GITLAB}"), &cookie, origin),
        )
        .await;
        assert_eq!(logout.status(), StatusCode::FORBIDDEN, "{origin}");
    }
    let no_origin = Request::post("/api/hosts")
        .header(header::COOKIE, &cookie)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{}"))
        .unwrap();
    assert_eq!(send(&app, no_origin).await.status(), StatusCode::FORBIDDEN);
    let form = Request::post("/api/hosts")
        .header(header::ORIGIN, URL)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from("{}"))
        .unwrap();
    assert_eq!(send(&app, form).await.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        hosts(&app, &cookie).await.0[0]["login"],
        "ann",
        "вход не тронут"
    );
}

#[tokio::test]
async fn выход_стирает_вход_только_в_один_хост_и_отзывает_токен_gitlab() {
    let mock = providers(7200).await;
    Mock::given(method("POST"))
        .and(path("/oauth/revoke"))
        .and(body_string_contains("token=gl-a1"))
        .respond_with(reply(200, json!({})))
        .expect(1)
        .mount(&mock)
        .await;
    let app = router(state(&mock));
    let cookie = sign_in(&app, GITLAB, "").await;
    let cookie = sign_in(&app, GHES, &cookie).await;

    let response = send(
        &app,
        post(&format!("/auth/logout?host={GITLAB}"), &cookie, URL),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let (session, _) = set_cookie(response.headers(), "longpole_session").unwrap();
    let (list, _) = hosts(&app, &format!("longpole_session={session}")).await;
    assert_eq!(
        signed_in(&list),
        [(GITLAB.into(), Value::Null), (GHES.into(), json!("bob"))]
    );
    // отзыв — best effort в фоне
    for _ in 0..50 {
        let revoked = mock
            .received_requests()
            .await
            .unwrap()
            .iter()
            .any(|r| r.url.path() == "/oauth/revoke");
        if revoked {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("токен GitLab не отозван");
}

//! OAuth web flow с секретом на wiremock (спека `2026-10-12-delivery-variants-design.md`, § 3, § 5):
//! GitLab — с PKCE, GHES — без.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use longpole_core::error::ErrorCode;
use longpole_core::source::Provider;
use longpole_core::web_oauth::{App, RefreshError, User};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use url::Url;
use wiremock::matchers::{body_string, body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn reply(status: u16, body: Value) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_json(body)
}

fn app(provider: Provider, base: &str) -> App<'_> {
    App {
        provider,
        base,
        client_id: "cid",
        client_secret: "secret",
        redirect_uri: "https://longpole.example/auth/callback/h.example",
    }
}

fn query(url: &str) -> Vec<(String, String)> {
    Url::parse(url)
        .unwrap()
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect()
}

#[test]
fn url_авторизации_gitlab_с_pkce() {
    let auth = app(Provider::Gitlab, "https://h.example")
        .authorize()
        .unwrap();
    assert!(auth.url.starts_with("https://h.example/oauth/authorize?"));
    let verifier = auth.verifier.expect("PKCE");
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    assert_eq!(
        query(&auth.url),
        [
            ("client_id", "cid"),
            (
                "redirect_uri",
                "https://longpole.example/auth/callback/h.example"
            ),
            ("response_type", "code"),
            ("state", auth.state.as_str()),
            ("scope", "read_api"),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ]
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
    );
    let again = app(Provider::Gitlab, "https://h.example")
        .authorize()
        .unwrap();
    assert_ne!(again.state, auth.state, "state случайный");
}

#[test]
fn url_авторизации_ghes_без_pkce() {
    let auth = app(Provider::Github, "https://h.example")
        .authorize()
        .unwrap();
    assert!(
        auth.url
            .starts_with("https://h.example/login/oauth/authorize?")
    );
    assert_eq!(auth.verifier, None);
    assert_eq!(
        query(&auth.url),
        [
            ("client_id", "cid"),
            (
                "redirect_uri",
                "https://longpole.example/auth/callback/h.example"
            ),
            ("state", auth.state.as_str()),
        ]
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
    );
}

#[tokio::test]
async fn обмен_кода_gitlab_с_секретом_и_verifier() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string(
            "client_id=cid&client_secret=secret&code=c1&grant_type=authorization_code\
             &redirect_uri=https%3A%2F%2Flongpole.example%2Fauth%2Fcallback%2Fh.example&code_verifier=v1",
        ))
        .respond_with(reply(
            200,
            json!({"access_token": "a1", "refresh_token": "r1", "expires_in": 7200}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    let tokens = app(Provider::Gitlab, &server.uri())
        .exchange("c1", Some("v1"))
        .await
        .unwrap();
    assert_eq!(
        (tokens.access.as_str(), tokens.refresh.as_str()),
        ("a1", "r1")
    );
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    assert!((tokens.expires_at - now - 7200).abs() <= 2);
    assert!(
        !format!("{tokens:?}").contains("a1"),
        "Debug не печатает токен"
    );
}

#[tokio::test]
async fn обмен_кода_ghes_без_verifier() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .and(header("accept", "application/json"))
        .and(body_string(
            "client_id=cid&client_secret=secret&code=c1\
             &redirect_uri=https%3A%2F%2Flongpole.example%2Fauth%2Fcallback%2Fh.example",
        ))
        .respond_with(reply(
            200,
            json!({"access_token": "ghu_1", "refresh_token": "ghr_1", "expires_in": 28800}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    let tokens = app(Provider::Github, &server.uri())
        .exchange("c1", None)
        .await
        .unwrap();
    assert_eq!(tokens.access, "ghu_1");
    assert_eq!(tokens.refresh, "ghr_1");
}

#[tokio::test]
async fn ошибка_обмена_кода_oauth_failed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(reply(
            200,
            json!({"error": "bad_verification_code", "error_description": "The code is wrong"}),
        ))
        .mount(&server)
        .await;
    let e = app(Provider::Github, &server.uri())
        .exchange("c1", None)
        .await
        .unwrap_err();
    assert_eq!(e.code, ErrorCode::OauthFailed);
    assert_eq!(e.params["detail"], "The code is wrong");
}

#[tokio::test]
async fn refresh_новая_пара_токенов() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains(
            "grant_type=refresh_token&refresh_token=r1",
        ))
        .and(body_string_contains("client_secret=secret"))
        .respond_with(reply(
            200,
            json!({"access_token": "a2", "refresh_token": "r2", "expires_in": 7200}),
        ))
        .mount(&server)
        .await;
    let tokens = app(Provider::Gitlab, &server.uri())
        .refresh("r1")
        .await
        .unwrap();
    assert_eq!(
        (tokens.access.as_str(), tokens.refresh.as_str()),
        ("a2", "r2")
    );
}

#[tokio::test]
async fn refresh_отозван_при_invalid_grant_bad_refresh_token_и_401() {
    for (provider, status, body) in [
        (Provider::Gitlab, 400, json!({"error": "invalid_grant"})),
        (Provider::Github, 200, json!({"error": "bad_refresh_token"})),
        (Provider::Gitlab, 401, json!({})),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(reply(status, body.clone()))
            .mount(&server)
            .await;
        let e = app(provider, &server.uri())
            .refresh("r1")
            .await
            .unwrap_err();
        assert_eq!(e, RefreshError::Revoked, "{body}");
    }
}

#[tokio::test]
async fn refresh_сбой_сервера_не_отзыв() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(reply(502, json!({})))
        .mount(&server)
        .await;
    let e = app(Provider::Gitlab, &server.uri())
        .refresh("r1")
        .await
        .unwrap_err();
    assert!(matches!(e, RefreshError::Failed(e) if e.code == ErrorCode::HttpStatus));
}

#[tokio::test]
async fn пользователь_по_токену() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/user"))
        .and(header("authorization", "Bearer a1"))
        .respond_with(reply(200, json!({"id": 7, "username": "ann"})))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v3/user"))
        .and(header("authorization", "Bearer ghu_1"))
        .respond_with(reply(200, json!({"id": 9, "login": "bob"})))
        .mount(&server)
        .await;
    assert_eq!(
        app(Provider::Gitlab, &server.uri())
            .user("a1")
            .await
            .unwrap(),
        User {
            id: 7,
            login: "ann".into()
        }
    );
    assert_eq!(
        app(Provider::Github, &server.uri())
            .user("ghu_1")
            .await
            .unwrap(),
        User {
            id: 9,
            login: "bob".into()
        }
    );
}

#[tokio::test]
async fn отзыв_токена_gitlab() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/revoke"))
        .and(body_string("client_id=cid&client_secret=secret&token=a1"))
        .respond_with(reply(200, json!({})))
        .expect(1)
        .mount(&server)
        .await;
    app(Provider::Gitlab, &server.uri()).revoke("a1").await;
}

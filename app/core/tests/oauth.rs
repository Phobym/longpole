//! Вход через OAuth на wiremock (спека `2026-10-09-oauth-design.md`): запись, client_id, refresh.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use longpole_core::error::{Error, ErrorCode};
use longpole_core::oauth::{Credential, RefreshError, UserCode, client_id, login, now, refresh};
use longpole_core::source::Provider;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use url::Url;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn reply(status: u16, body: Value) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_json(body)
}

fn stale(server: &MockServer, redirect_uri: Option<&str>) -> Credential {
    Credential {
        access: "old".into(),
        refresh: Some("r1".into()),
        expires_at: Some(0),
        client_id: "app".into(),
        token_url: format!("{}/oauth/token", server.uri()),
        redirect_uri: redirect_uri.map(String::from),
    }
}

#[test]
fn запись_oauth_отличается_от_pat_и_знает_срок() {
    let credential = Credential {
        access: "a".into(),
        refresh: None,
        expires_at: Some(1000),
        client_id: "app".into(),
        token_url: "https://h/oauth/token".into(),
        redirect_uri: None,
    };
    let stored = serde_json::to_string(&credential).unwrap();
    assert!(stored.contains(r#""kind":"oauth""#));
    assert_eq!(Credential::parse(&stored), Some(credential.clone()));
    assert_eq!(Credential::parse("glpat-xxxx"), None);
    assert!(credential.is_fresh(600));
    assert!(!credential.is_fresh(800));
    let endless = Credential {
        expires_at: None,
        ..credential
    };
    assert!(endless.is_fresh(i64::MAX));
    assert!(
        !format!("{endless:?}").contains("\"a\""),
        "Debug не печатает токен"
    );
}

#[test]
fn client_id_введённый_обрезается_пустой_и_с_пробелами_отклоняются() {
    assert_eq!(
        client_id("gitlab.example.com", Some(" abc ")).unwrap(),
        "abc"
    );
    for given in [None, Some(""), Some("  "), Some("a b")] {
        assert_eq!(
            client_id("gitlab.example.com", given).unwrap_err().code,
            ErrorCode::OauthClientId,
            "{given:?}"
        );
    }
}

#[tokio::test]
async fn refresh_gitlab_передаёт_redirect_uri_и_возвращает_новую_пару() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .and(body_string_contains("refresh_token=r1"))
        .and(body_string_contains("client_id=app"))
        .and(body_string_contains(
            "redirect_uri=http%3A%2F%2F127.0.0.1%3A5000%2Fcallback",
        ))
        .respond_with(reply(
            200,
            json!({"access_token": "new", "refresh_token": "r2", "expires_in": 7200}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    let old = stale(&server, Some("http://127.0.0.1:5000/callback"));
    let renewed = refresh(&old).await.unwrap();
    assert_eq!(renewed.access, "new");
    assert_eq!(renewed.refresh.as_deref(), Some("r2"));
    assert_eq!(renewed.client_id, "app");
    assert_eq!(renewed.token_url, old.token_url);
    assert_eq!(renewed.redirect_uri, old.redirect_uri);
    assert!(renewed.expires_at.unwrap() > now() + 7000);
}

#[tokio::test]
async fn refresh_отозванный_и_сбой_сервера_различаются() {
    let cases = [
        (400, json!({"error": "invalid_grant"}), true),
        // GitHub отвечает ошибкой OAuth со статусом 200
        (200, json!({"error": "bad_refresh_token"}), true),
        (401, json!({}), true),
        (503, json!({}), false),
    ];
    for (status, body, revoked) in cases {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(reply(status, body.clone()))
            .mount(&server)
            .await;
        match refresh(&stale(&server, None)).await {
            Err(RefreshError::Revoked) => assert!(revoked, "{status} {body}"),
            Err(RefreshError::Failed(e)) => {
                assert!(!revoked, "{status} {body}");
                assert_eq!(e.code, ErrorCode::HttpStatus);
            }
            Ok(_) => panic!("{status} {body}: обновился"),
        }
    }
}

#[tokio::test]
async fn refresh_без_refresh_token_считается_отозванным() {
    let server = MockServer::start().await;
    let credential = Credential {
        refresh: None,
        ..stale(&server, None)
    };
    assert!(matches!(
        refresh(&credential).await,
        Err(RefreshError::Revoked)
    ));
}

const GITHUB_TOKEN: &str = "/login/oauth/access_token";

async fn device_code(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/login/device/code"))
        .and(body_string_contains("client_id=app"))
        .respond_with(reply(
            200,
            json!({
                "device_code": "dev", "user_code": "ABCD-1234",
                "verification_uri": "https://github.com/login/device",
                "interval": 0, "expires_in": 900
            }),
        ))
        .mount(server)
        .await;
}

async fn github_login(server: &MockServer) -> (Result<Credential, Error>, Option<UserCode>) {
    let shown = Arc::new(Mutex::new(None));
    let sink = shown.clone();
    let result = login(
        Provider::Github,
        &server.uri(),
        "app",
        |_: &str| Ok(()),
        move |code| *sink.lock().unwrap() = Some(code),
    )
    .await;
    let code = shown.lock().unwrap().take();
    (result, code)
}

#[tokio::test]
async fn github_ждёт_pending_и_slow_down_затем_выдаёт_токен() {
    let server = MockServer::start().await;
    device_code(&server).await;
    // wiremock берёт первый подходящий мок; `up_to_n_times` выключает его после ответа
    Mock::given(method("POST"))
        .and(path(GITHUB_TOKEN))
        .respond_with(reply(200, json!({"error": "authorization_pending"})))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(GITHUB_TOKEN))
        .respond_with(reply(200, json!({"error": "slow_down", "interval": 0})))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(GITHUB_TOKEN))
        .and(body_string_contains("device_code=dev"))
        .and(body_string_contains(
            "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code",
        ))
        .respond_with(reply(
            200,
            json!({"access_token": "ghu_a", "refresh_token": "ghr_r", "expires_in": 28800}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    let (result, code) = github_login(&server).await;
    let credential = result.unwrap();
    assert_eq!(credential.access, "ghu_a");
    assert_eq!(credential.refresh.as_deref(), Some("ghr_r"));
    assert!(credential.expires_at.unwrap() > now() + 28000);
    assert_eq!(
        credential.token_url,
        format!("{}{GITHUB_TOKEN}", server.uri())
    );
    assert_eq!(credential.redirect_uri, None);
    assert_eq!(
        code,
        Some(UserCode {
            user_code: "ABCD-1234".into(),
            verification_uri: "https://github.com/login/device".into(),
        })
    );
}

#[tokio::test]
async fn github_отказ_истёкший_код_и_выключенный_device_flow() {
    let cases = [
        ("access_denied", ErrorCode::OauthDenied),
        ("expired_token", ErrorCode::OauthExpired),
        ("device_flow_disabled", ErrorCode::OauthFailed),
    ];
    for (error, expected) in cases {
        let server = MockServer::start().await;
        device_code(&server).await;
        Mock::given(method("POST"))
            .and(path(GITHUB_TOKEN))
            .respond_with(reply(200, json!({"error": error})))
            .mount(&server)
            .await;
        let (result, _) = github_login(&server).await;
        assert_eq!(result.unwrap_err().code, expected, "{error}");
    }
}

fn param(url: &Url, name: &str) -> String {
    url.query_pairs()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
        .unwrap_or_else(|| panic!("нет {name} в {url}"))
}

/// Браузер пользователя: сначала посторонний запрос с чужим `state`, затем редирект с `query`.
fn browser(query: &'static str) -> impl FnOnce(&str) -> Result<(), Error> {
    move |authorize: &str| {
        let url = Url::parse(authorize).unwrap();
        let (redirect, state) = (param(&url, "redirect_uri"), param(&url, "state"));
        tokio::spawn(async move {
            let stranger = reqwest::get(format!("{redirect}?code=x&state=wrong"))
                .await
                .unwrap();
            assert_eq!(stranger.status(), 400);
            let done = reqwest::get(format!("{redirect}?{query}&state={state}"))
                .await
                .unwrap();
            assert_eq!(done.status(), 200);
        });
        Ok(())
    }
}

async fn gitlab_login(
    server: &MockServer,
    open: impl FnOnce(&str) -> Result<(), Error>,
) -> Result<Credential, Error> {
    let base = server.uri();
    let flow = login(Provider::Gitlab, &base, "app", open, |_| {
        unreachable!("у GitLab нет кода для ввода")
    });
    tokio::time::timeout(Duration::from_secs(5), flow)
        .await
        .expect("вход не завершился")
}

#[tokio::test]
async fn gitlab_pkce_чужой_state_получает_400_а_обмен_идёт_с_верным_verifier() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=authorization_code"))
        .respond_with(reply(
            200,
            json!({"access_token": "gl_a", "refresh_token": "gl_r", "expires_in": 7200}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    let authorize = Arc::new(Mutex::new(None));
    let seen = authorize.clone();
    let user = browser("code=c");
    let credential = gitlab_login(&server, move |url: &str| {
        *seen.lock().unwrap() = Some(Url::parse(url).unwrap());
        user(url)
    })
    .await
    .unwrap();

    let authorize = authorize.lock().unwrap().clone().unwrap();
    assert_eq!(authorize.path(), "/oauth/authorize");
    assert_eq!(param(&authorize, "client_id"), "app");
    assert_eq!(param(&authorize, "response_type"), "code");
    assert_eq!(param(&authorize, "scope"), "read_api");
    assert_eq!(param(&authorize, "code_challenge_method"), "S256");
    let redirect = param(&authorize, "redirect_uri");
    assert!(redirect.starts_with("http://127.0.0.1:") && redirect.ends_with("/callback"));

    assert_eq!(credential.access, "gl_a");
    assert_eq!(credential.refresh.as_deref(), Some("gl_r"));
    assert_eq!(
        credential.token_url,
        format!("{}/oauth/token", server.uri())
    );
    assert_eq!(credential.redirect_uri.as_deref(), Some(redirect.as_str()));

    let requests = server.received_requests().await.unwrap();
    let form: HashMap<String, String> = url::form_urlencoded::parse(&requests[0].body)
        .into_owned()
        .collect();
    assert_eq!(form["code"], "c");
    assert_eq!(form["client_id"], "app");
    assert_eq!(form["redirect_uri"], redirect);
    assert!(!form.contains_key("client_secret"));
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(form["code_verifier"].as_bytes()));
    assert_eq!(param(&authorize, "code_challenge"), challenge);
}

#[tokio::test]
async fn gitlab_отказ_в_браузере() {
    let server = MockServer::start().await;
    let error = gitlab_login(&server, browser("error=access_denied"))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::OauthDenied);
}

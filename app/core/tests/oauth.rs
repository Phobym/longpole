//! Вход через OAuth на wiremock (спека `2026-10-09-oauth-design.md`): запись, client_id, refresh.

use longpole_core::error::ErrorCode;
use longpole_core::oauth::{Credential, RefreshError, client_id, now, refresh};
use serde_json::{Value, json};
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

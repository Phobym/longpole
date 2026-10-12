//! Общее для тестов сервера: конфиг с хостами GitLab и GHES на wiremock, запросы, вход.
#![allow(dead_code)]

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Request, Response, StatusCode, header};
use longpole_server::AppState;
use longpole_server::config::Config;
use serde_json::{Value, json};
use tower::ServiceExt;
use url::Url;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub const URL: &str = "https://longpole.example.com";
pub const KEY: &str = "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=";
pub const GITLAB: &str = "gitlab.example.com";
pub const GHES: &str = "ghe.example.com";

pub fn config() -> Config {
    let yaml = format!(
        "url: {URL}\ncookie_key: {KEY}\nhosts:\n\
         \x20 - {{domain: {GITLAB}, provider: gitlab, client_id: gl-id, client_secret: gl-secret}}\n\
         \x20 - {{domain: {GHES}, provider: github, client_id: gh-id, client_secret: gh-secret}}\n"
    );
    Config::parse(&yaml, |_| None).unwrap()
}

/// Оба хоста ходят на один wiremock: пути GitLab и GHES не пересекаются.
pub fn state(mock: &MockServer) -> AppState {
    let uri = mock.uri();
    AppState::new(config(), move |_| uri.clone())
}

pub fn reply(status: u16, body: Value) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_json(body)
}

/// Провайдеры: код меняется на токены с `expires_in`, `/user` знает пользователя.
pub async fn providers(expires_in: i64) -> MockServer {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(body_string_contains("grant_type=authorization_code"))
        .respond_with(reply(
            200,
            json!({"access_token": "gl-a1", "refresh_token": "gl-r1", "expires_in": expires_in}),
        ))
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/user"))
        .respond_with(reply(200, json!({"id": 7, "username": "ann"})))
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(reply(
            200,
            json!({"access_token": "ghu-a1", "refresh_token": "ghr-r1", "expires_in": 28800}),
        ))
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v3/user"))
        .respond_with(reply(200, json!({"id": 9, "login": "bob"})))
        .mount(&mock)
        .await;
    mock
}

pub async fn send(app: &Router, request: Request<Body>) -> Response<Body> {
    app.clone().oneshot(request).await.unwrap()
}

pub fn get(uri: &str, cookie: &str) -> Request<Body> {
    Request::get(uri)
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap()
}

pub fn post(uri: &str, cookie: &str, origin: &str) -> Request<Body> {
    Request::post(uri)
        .header(header::COOKIE, cookie)
        .header(header::ORIGIN, origin)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{}"))
        .unwrap()
}

/// `Set-Cookie` с этим именем: `(значение, атрибуты)`.
pub fn set_cookie(headers: &HeaderMap, name: &str) -> Option<(String, String)> {
    headers
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|v| {
            let (pair, attrs) = v.split_once(';').unwrap_or((v, ""));
            let value = pair.strip_prefix(&format!("{name}="))?;
            Some((value.to_owned(), attrs.to_owned()))
        })
}

pub fn location(response: &Response<Body>) -> String {
    response.headers()[header::LOCATION]
        .to_str()
        .unwrap()
        .to_owned()
}

/// Вход через `/auth/login` и `/auth/callback`; к `cookie` добавляется сессия.
pub async fn sign_in(app: &Router, host: &str, cookie: &str) -> String {
    let login = send(
        app,
        get(&format!("/auth/login?host={host}&return_to=/x"), cookie),
    )
    .await;
    assert_eq!(login.status(), StatusCode::FOUND);
    let state = Url::parse(&location(&login))
        .unwrap()
        .query_pairs()
        .find(|(k, _)| k == "state")
        .unwrap()
        .1
        .into_owned();
    let (pending, attrs) = set_cookie(login.headers(), "longpole_oauth").unwrap();
    assert!(attrs.contains("Path=/auth"), "{attrs}");
    let callback = send(
        app,
        get(
            &format!("/auth/callback/{host}?code=c1&state={state}"),
            &format!("{cookie}; longpole_oauth={pending}"),
        ),
    )
    .await;
    assert_eq!(callback.status(), StatusCode::FOUND);
    assert_eq!(location(&callback), "/x");
    let (session, _) = set_cookie(callback.headers(), "longpole_session").unwrap();
    format!("longpole_session={session}")
}

/// `POST /api/<cmd>` с JSON-телом от своего `Origin`.
pub fn api(cmd: &str, cookie: &str, body: Value) -> Request<Body> {
    Request::post(format!("/api/{cmd}"))
        .header(header::COOKIE, cookie)
        .header(header::ORIGIN, URL)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

pub fn base64(s: &str) -> Vec<u8> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(s).unwrap()
}

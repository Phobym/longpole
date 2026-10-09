//! Вход через OAuth без `client_secret` (спека `2026-10-09-oauth-design.md`): запись в связке ключей,
//! client_id и обновление токена. Сами flow — `login`.

use std::fmt;
use std::time::Duration;

use reqwest::header::{ACCEPT, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use url::Url;
use url::form_urlencoded::Serializer;

use crate::error::{Error, ErrorCode, network_error};
use crate::source::GITHUB_COM;

/// Общий предел ожидания входа: столько живёт device-код GitHub.
pub const LOGIN_TIMEOUT: Duration = Duration::from_secs(600);
/// Токен обновляется заранее, чтобы не истечь посреди сборки отчёта.
const REFRESH_MARGIN: i64 = 300;
const TIMEOUT: Duration = Duration::from_secs(30);
const GITLAB_COM: &str = "gitlab.com";

/// client_id приложений Longpole на github.com и gitlab.com; не секрет. Пустая строка — приложение
/// не зарегистрировано, и вход на облако требует client_id, как self-hosted.
pub const GITHUB_CLIENT_ID: &str = "";
pub const GITLAB_CLIENT_ID: &str = "";

/// Запись OAuth в связке ключей; PAT лежит там же простой строкой.
/// `refresh` и `expires_at` — `None`, если сервер выдал бессрочный токен.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename = "oauth")]
pub struct Credential {
    pub access: String,
    pub refresh: Option<String>,
    /// unix-секунды
    pub expires_at: Option<i64>,
    pub client_id: String,
    /// куда идёт refresh: `…/login/oauth/access_token` у GitHub, `…/oauth/token` у GitLab
    pub token_url: String,
    /// GitLab сверяет его и при refresh; у GitHub `None`
    pub redirect_uri: Option<String>,
}

impl Credential {
    /// Запись OAuth — JSON-объект; PAT им не бывает.
    pub fn parse(stored: &str) -> Option<Self> {
        serde_json::from_str(stored).ok()
    }

    pub fn is_fresh(&self, now: i64) -> bool {
        self.expires_at.is_none_or(|at| at - now > REFRESH_MARGIN)
    }
}

/// Без токенов: `Debug` может попасть в лог или panic.
impl fmt::Debug for Credential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credential")
            .field("expires_at", &self.expires_at)
            .field("client_id", &self.client_id)
            .field("token_url", &self.token_url)
            .field("redirect_uri", &self.redirect_uri)
            .finish_non_exhaustive()
    }
}

pub fn now() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

/// client_id для входа: введённый пользователем или наш для github.com и gitlab.com.
pub fn client_id(host: &str, given: Option<&str>) -> Result<String, Error> {
    let missing = || Error::new(ErrorCode::OauthClientId).with("host", host);
    match given.map(str::trim).filter(|id| !id.is_empty()) {
        Some(id) if id.chars().any(char::is_whitespace) => Err(missing()),
        Some(id) => Ok(id.to_string()),
        None => match host {
            GITHUB_COM => Some(GITHUB_CLIENT_ID),
            GITLAB_COM => Some(GITLAB_CLIENT_ID),
            _ => None,
        }
        .filter(|id| !id.is_empty())
        .map(String::from)
        .ok_or_else(missing),
    }
}

/// Почему не обновился токен: `Revoked` — запись больше не годится, `Failed` — временный сбой.
#[derive(Debug, PartialEq, Eq)]
pub enum RefreshError {
    Revoked,
    Failed(Error),
}

/// Новая пара токенов по refresh token; секрет не нужен ни GitHub (Device Flow), ни GitLab (non-confidential).
pub async fn refresh(credential: &Credential) -> Result<Credential, RefreshError> {
    let Some(refresh_token) = &credential.refresh else {
        return Err(RefreshError::Revoked);
    };
    let host = host_of(&credential.token_url);
    let mut params = vec![
        ("client_id", credential.client_id.as_str()),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token.as_str()),
    ];
    if let Some(uri) = &credential.redirect_uri {
        params.push(("redirect_uri", uri.as_str()));
    }
    match request_token(&host, &credential.token_url, &params).await {
        Ok(Ok(tokens)) => Ok(tokens.into_credential(
            &credential.client_id,
            &credential.token_url,
            credential.redirect_uri.clone(),
        )),
        Ok(Err(e)) if matches!(e.code.as_str(), "invalid_grant" | "bad_refresh_token") => {
            Err(RefreshError::Revoked)
        }
        Ok(Err(e)) => Err(RefreshError::Failed(oauth_error(&host, e))),
        Err(e)
            if e.code == ErrorCode::HttpStatus
                && e.params.get("status").is_some_and(|s| s == "401") =>
        {
            Err(RefreshError::Revoked)
        }
        Err(e) => Err(RefreshError::Failed(e)),
    }
}

/// Токены из ответа сервера.
struct Tokens {
    access: String,
    refresh: Option<String>,
    expires_in: Option<i64>,
}

impl Tokens {
    fn into_credential(
        self,
        client_id: &str,
        token_url: &str,
        redirect_uri: Option<String>,
    ) -> Credential {
        Credential {
            access: self.access,
            refresh: self.refresh,
            expires_at: self.expires_in.map(|seconds| now() + seconds),
            client_id: client_id.into(),
            token_url: token_url.into(),
            redirect_uri,
        }
    }
}

/// Ошибка по протоколу OAuth: поле `error` ответа (`authorization_pending`, `invalid_grant`, …).
struct OauthError {
    code: String,
    detail: String,
    /// новый интервал опроса при `slow_down`
    interval: Option<u64>,
}

impl OauthError {
    fn new(code: String, description: Option<String>) -> Self {
        Self {
            detail: description.unwrap_or_else(|| code.clone()),
            code,
            interval: None,
        }
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    error: Option<String>,
    error_description: Option<String>,
    interval: Option<u64>,
}

/// Код ошибки OAuth → код ошибки ядра.
fn oauth_error(host: &str, e: OauthError) -> Error {
    let code = match e.code.as_str() {
        "access_denied" => ErrorCode::OauthDenied,
        "expired_token" => ErrorCode::OauthExpired,
        _ => return failed(host, e.detail),
    };
    Error::new(code).with("host", host)
}

pub fn failed(host: &str, detail: impl ToString) -> Error {
    Error::new(ErrorCode::OauthFailed)
        .with("host", host)
        .with("detail", detail)
}

pub fn host_of(url: &str) -> String {
    Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(String::from))
        .unwrap_or_default()
}

/// POST формы с `Accept: application/json` (без него GitHub отвечает формой); статус и тело.
async fn post_form(
    host: &str,
    url: &str,
    params: &[(&str, &str)],
) -> Result<(u16, Vec<u8>), Error> {
    let http = reqwest::Client::builder()
        .timeout(TIMEOUT)
        // эндпоинты токенов не редиректят; 307/308 унёс бы refresh token на чужой хост
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| network_error(host, e))?;
    let body = Serializer::new(String::new()).extend_pairs(params).finish();
    let response = http
        .post(url)
        .header(ACCEPT, "application/json")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| network_error(host, e))?;
    let status = response.status().as_u16();
    let bytes = response.bytes().await.map_err(|e| network_error(host, e))?;
    Ok((status, bytes.to_vec()))
}

/// Ответ эндпоинта токена: токены или ошибка OAuth; без того и другого — ошибка HTTP.
fn parse_reply(host: &str, status: u16, body: &[u8]) -> Result<Result<Tokens, OauthError>, Error> {
    let ok = (200..300).contains(&status);
    match serde_json::from_slice::<TokenResponse>(body).ok() {
        Some(TokenResponse {
            error: Some(code),
            error_description,
            interval,
            ..
        }) => Ok(Err(OauthError {
            interval,
            ..OauthError::new(code, error_description)
        })),
        Some(TokenResponse {
            access_token: Some(access),
            refresh_token,
            expires_in,
            ..
        }) if ok => Ok(Ok(Tokens {
            access,
            refresh: refresh_token,
            expires_in,
        })),
        _ if !ok => Err(Error::new(ErrorCode::HttpStatus)
            .with("host", host)
            .with("status", status)),
        _ => Err(failed(host, "no access_token in response")),
    }
}

async fn request_token(
    host: &str,
    url: &str,
    params: &[(&str, &str)],
) -> Result<Result<Tokens, OauthError>, Error> {
    let (status, body) = post_form(host, url, params).await?;
    parse_reply(host, status, &body)
}

//! OAuth web flow confidential-приложения для Selfhosted (спека `2026-10-12-delivery-variants-design.md`, § 3, § 5):
//! URL авторизации (GitLab — с PKCE, GHES — без), обмен кода, refresh, пользователь и отзыв токена.
//! Без состояния: `state`, verifier и токены хранит оболочка. С loopback-веткой `oauth` кода не делит.

use std::fmt;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use url::Url;
use url::form_urlencoded::Serializer;

use crate::error::{Error, ErrorCode, network_error};
use crate::source::Provider;

const TIMEOUT: Duration = Duration::from_secs(30);

/// Приложение хоста из конфигурации инстанса. `base` — `https://<домен>`, тесты подставляют wiremock.
pub struct App<'a> {
    pub provider: Provider,
    pub base: &'a str,
    pub client_id: &'a str,
    pub client_secret: &'a str,
    pub redirect_uri: &'a str,
}

/// Куда отправить браузер и что запомнить до callback.
pub struct Authorize {
    pub url: String,
    pub state: String,
    /// PKCE verifier; только GitLab
    pub verifier: Option<String>,
}

/// Токены входа; `Debug` их не печатает.
#[derive(Clone, PartialEq, Eq)]
pub struct Tokens {
    pub access: String,
    pub refresh: String,
    /// unix-секунды
    pub expires_at: i64,
}

impl fmt::Debug for Tokens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tokens")
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

/// Пользователь у провайдера.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: u64,
    pub login: String,
}

/// `Revoked` — вход больше не годится (`invalid_grant`, 401), `Failed` — временный сбой.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshError {
    Revoked,
    Failed(Error),
}

impl App<'_> {
    pub fn authorize(&self) -> Result<Authorize, Error> {
        let state = self.random(16)?;
        let mut params = vec![
            ("client_id", self.client_id.to_owned()),
            ("redirect_uri", self.redirect_uri.to_owned()),
        ];
        let (path, verifier) = match self.provider {
            Provider::Github => {
                params.push(("state", state.clone()));
                ("login/oauth/authorize", None)
            }
            Provider::Gitlab => {
                let verifier = self.random(32)?;
                let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
                params.extend([
                    ("response_type", "code".to_owned()),
                    ("state", state.clone()),
                    ("scope", "read_api".to_owned()),
                    ("code_challenge", challenge),
                    ("code_challenge_method", "S256".to_owned()),
                ]);
                ("oauth/authorize", Some(verifier))
            }
        };
        let url = Url::parse_with_params(&self.url(path), &params).map_err(|e| self.failed(e))?;
        Ok(Authorize {
            url: url.into(),
            state,
            verifier,
        })
    }

    /// Код из callback → токены; `verifier` — из `authorize` (у GitLab).
    pub async fn exchange(&self, code: &str, verifier: Option<&str>) -> Result<Tokens, Error> {
        let mut params = vec![("code", code)];
        if self.provider == Provider::Gitlab {
            params.push(("grant_type", "authorization_code"));
        }
        params.push(("redirect_uri", self.redirect_uri));
        if let Some(verifier) = verifier {
            params.push(("code_verifier", verifier));
        }
        match self.token(&params).await? {
            Ok(tokens) => Ok(tokens),
            Err(e) => Err(self.failed(e.detail)),
        }
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<Tokens, RefreshError> {
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ];
        match self.token(&params).await {
            Ok(Ok(tokens)) => Ok(tokens),
            Ok(Err(e)) if matches!(e.code.as_str(), "invalid_grant" | "bad_refresh_token") => {
                Err(RefreshError::Revoked)
            }
            Ok(Err(e)) => Err(RefreshError::Failed(self.failed(e.detail))),
            Err(e) if e.params.get("status").is_some_and(|s| s == "401") => {
                Err(RefreshError::Revoked)
            }
            Err(e) => Err(RefreshError::Failed(e)),
        }
    }

    /// Кто вошёл: id и логин у провайдера.
    pub async fn user(&self, access: &str) -> Result<User, Error> {
        #[derive(Deserialize)]
        struct Wire {
            id: u64,
            #[serde(alias = "username")]
            login: String,
        }
        let api = match self.provider {
            Provider::Gitlab => "api/v4",
            Provider::Github => "api/v3",
        };
        let response = self
            .http()?
            .get(self.url(&format!("{api}/user")))
            .header(AUTHORIZATION, format!("Bearer {access}"))
            .header(ACCEPT, "application/json")
            .send()
            .await
            .map_err(|e| network_error(&self.host(), e))?;
        let status = response.status();
        if !status.is_success() {
            let code = match status.as_u16() {
                401 => ErrorCode::Unauthorized,
                _ => ErrorCode::HttpStatus,
            };
            return Err(Error::new(code)
                .with("host", self.host())
                .with("status", status.as_u16()));
        }
        let wire: Wire = response
            .json()
            .await
            .map_err(|e| network_error(&self.host(), e))?;
        Ok(User {
            id: wire.id,
            login: wire.login,
        })
    }

    /// Отзыв access-токена у GitLab — best effort: ошибки не важны, вход уже стёрт у нас. У GHES ничего не делает.
    pub async fn revoke(&self, access: &str) {
        if self.provider == Provider::Gitlab {
            let _ = self
                .post(&self.url("oauth/revoke"), &[("token", access)])
                .await;
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{path}", self.base.trim_end_matches('/'))
    }

    fn host(&self) -> String {
        Url::parse(self.base)
            .ok()
            .and_then(|u| u.host_str().map(String::from))
            .unwrap_or_default()
    }

    fn failed(&self, detail: impl ToString) -> Error {
        Error::new(ErrorCode::OauthFailed)
            .with("host", self.host())
            .with("detail", detail)
    }

    fn random(&self, bytes: usize) -> Result<String, Error> {
        let mut buf = vec![0; bytes];
        getrandom::fill(&mut buf).map_err(|e| self.failed(e))?;
        Ok(URL_SAFE_NO_PAD.encode(buf))
    }

    fn http(&self) -> Result<reqwest::Client, Error> {
        reqwest::Client::builder()
            .timeout(TIMEOUT)
            // редирект унёс бы код, секрет или токен на чужой хост
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| network_error(&self.host(), e))
    }

    /// POST формы с `client_id` и `client_secret`; `Accept: application/json` — без него GitHub отвечает формой.
    async fn post(&self, url: &str, params: &[(&str, &str)]) -> Result<(u16, Vec<u8>), Error> {
        let body = Serializer::new(String::new())
            .extend_pairs([
                ("client_id", self.client_id),
                ("client_secret", self.client_secret),
            ])
            .extend_pairs(params)
            .finish();
        let response = self
            .http()?
            .post(url)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header(USER_AGENT, "longpole")
            .body(body)
            .send()
            .await
            .map_err(|e| network_error(&self.host(), e))?;
        let status = response.status().as_u16();
        let bytes = response
            .bytes()
            .await
            .map_err(|e| network_error(&self.host(), e))?;
        Ok((status, bytes.to_vec()))
    }

    /// Ответ эндпоинта токена: токены или ошибка OAuth; без того и другого — ошибка HTTP.
    async fn token(&self, params: &[(&str, &str)]) -> Result<Result<Tokens, OauthError>, Error> {
        #[derive(Deserialize)]
        struct Reply {
            access_token: Option<String>,
            refresh_token: Option<String>,
            expires_in: Option<i64>,
            error: Option<String>,
            error_description: Option<String>,
        }
        let path = match self.provider {
            Provider::Gitlab => "oauth/token",
            Provider::Github => "login/oauth/access_token",
        };
        let (status, body) = self.post(&self.url(path), params).await?;
        let ok = (200..300).contains(&status);
        match serde_json::from_slice::<Reply>(&body).ok() {
            Some(Reply {
                error: Some(code),
                error_description,
                ..
            }) => Ok(Err(OauthError {
                detail: error_description.unwrap_or_else(|| code.clone()),
                code,
            })),
            Some(Reply {
                access_token: Some(access),
                refresh_token: Some(refresh),
                expires_in: Some(expires_in),
                ..
            }) if ok => Ok(Ok(Tokens {
                access,
                refresh,
                expires_at: OffsetDateTime::now_utc().unix_timestamp() + expires_in,
            })),
            _ if !ok => Err(Error::new(ErrorCode::HttpStatus)
                .with("host", self.host())
                .with("status", status)),
            // приложение без истекающих токенов: refresh нужен сессии (спека, § 5)
            _ => Err(self.failed("no access_token, refresh_token or expires_in in response")),
        }
    }
}

/// Поле `error` ответа OAuth (`invalid_grant`, `bad_refresh_token`, …).
struct OauthError {
    code: String,
    detail: String,
}

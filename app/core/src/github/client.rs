//! HTTP-клиент GitHub REST: заголовки, пагинация по `Link`, лимиты запросов.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::header::{ACCEPT, HeaderMap, LINK};
use serde::de::DeserializeOwned;
use time::OffsetDateTime;
use tokio::sync::Semaphore;

use super::workflow::Workflow;
use crate::error::{Error, ErrorCode, network_error};
use crate::iso::iso;
use crate::source::Provider;

const MAX_PARALLEL: usize = 4;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const READ_TIMEOUT: Duration = Duration::from_secs(60);
const USER_AGENT: &str = "pipeline-trace";
const JSON: &str = "application/vnd.github+json";
const RAW: &str = "application/vnd.github.raw";
const API_VERSION: &str = "2022-11-28";

/// github.com живёт на отдельном хосте API, GHES — под `/api/v3`.
pub(crate) fn api_url(host: &str) -> String {
    if host == crate::source::GITHUB_COM {
        "https://api.github.com".into()
    } else {
        format!("https://{host}/api/v3")
    }
}

fn http(host: &str) -> Result<reqwest::Client, Error> {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        // редирект унёс бы токен на чужой хост
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| network_error(host, e))
}

pub struct Client {
    http: reqwest::Client,
    host: String,
    api: String,
    token: String,
    permits: Semaphore,
    /// (путь, sha) → разобранный файл workflow; `None` — не загрузился
    pub(crate) workflows: Mutex<HashMap<(String, String), Option<Arc<Workflow>>>>,
}

/// Тело ответа и адрес следующей страницы.
struct Body {
    next: Option<String>,
    bytes: Vec<u8>,
}

impl Client {
    pub fn new(host: &str, token: &str) -> Result<Self, Error> {
        Self::with_api_url(host, &api_url(host), token)
    }

    /// `api` — корень REST без завершающего слэша; тесты подставляют адрес wiremock.
    pub fn with_api_url(host: &str, api: &str, token: &str) -> Result<Self, Error> {
        Ok(Self {
            http: http(host)?,
            host: host.into(),
            api: api.trim_end_matches('/').into(),
            token: token.into(),
            permits: Semaphore::new(MAX_PARALLEL),
            workflows: Mutex::default(),
        })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    async fn get(&self, target: &str, accept: &str) -> Result<Option<Body>, Error> {
        let url = if target.starts_with("http://") || target.starts_with("https://") {
            // токен уходит только на свой API, даже если адрес пришёл не из `next_link`
            if !own_url(&self.api, target) {
                return Err(Error::new(ErrorCode::Network)
                    .with("host", &self.host)
                    .with("detail", "адрес вне API хоста"));
            }
            target.to_string()
        } else {
            format!("{}{target}", self.api)
        };
        let _permit = self.permits.acquire().await.expect("семафор не закрывают");
        let response = self
            .http
            .get(&url)
            .bearer_auth(&self.token)
            .header(ACCEPT, accept)
            .header("X-GitHub-Api-Version", API_VERSION)
            .send()
            .await
            .map_err(|e| network_error(&self.host, e))?;
        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(status_error(
                &self.host,
                status,
                response.headers(),
                OffsetDateTime::now_utc(),
            ));
        }
        let next = next_link(response.headers(), &self.api);
        let bytes = response
            .bytes()
            .await
            .map_err(|e| network_error(&self.host, e))?
            .to_vec();
        Ok(Some(Body { next, bytes }))
    }

    /// Неожиданная форма JSON — `graphql` с текстом serde: перевод общий «неожиданный ответ».
    fn decode<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<T, Error> {
        serde_json::from_slice(bytes).map_err(|e| {
            Error::new(ErrorCode::Graphql)
                .with("host", &self.host)
                .with("detail", e)
        })
    }

    pub(crate) async fn json<T: DeserializeOwned>(&self, target: &str) -> Result<Option<T>, Error> {
        match self.get(target, JSON).await? {
            Some(body) => self.decode(&body.bytes).map(Some),
            None => Ok(None),
        }
    }

    pub(crate) async fn page<T: DeserializeOwned>(
        &self,
        target: &str,
    ) -> Result<Option<(T, Option<String>)>, Error> {
        match self.get(target, JSON).await? {
            Some(body) => Ok(Some((self.decode(&body.bytes)?, body.next))),
            None => Ok(None),
        }
    }

    /// Содержимое файла как есть (`contents` с `Accept: raw`).
    pub(crate) async fn raw(&self, target: &str) -> Result<Option<String>, Error> {
        Ok(self
            .get(target, RAW)
            .await?
            .map(|body| String::from_utf8_lossy(&body.bytes).into_owned()))
    }
}

/// Не-2xx в код ошибки: лимит — по `retry-after` или `x-ratelimit-remaining: 0`, 401/403 — нет доступа.
pub(crate) fn status_error(
    host: &str,
    status: StatusCode,
    headers: &HeaderMap,
    now: OffsetDateTime,
) -> Error {
    let header = |name: &str| headers.get(name).and_then(|v| v.to_str().ok());
    if matches!(status.as_u16(), 403 | 429) {
        let limited = |reset: OffsetDateTime| {
            Error::new(ErrorCode::RateLimited)
                .with("host", host)
                .with("reset", iso(reset))
        };
        if let Some(seconds) = header("retry-after").and_then(|v| v.parse::<i64>().ok()) {
            let wait = time::Duration::seconds(seconds.max(0));
            return limited(now.checked_add(wait).unwrap_or(now));
        }
        if header("x-ratelimit-remaining") == Some("0") {
            let reset = header("x-ratelimit-reset")
                .and_then(|v| v.parse::<i64>().ok())
                .and_then(|t| OffsetDateTime::from_unix_timestamp(t).ok())
                .unwrap_or(now);
            return limited(reset);
        }
    }
    let code = if matches!(status.as_u16(), 401 | 403) {
        ErrorCode::Unauthorized
    } else {
        ErrorCode::HttpStatus
    };
    Error::new(code)
        .with("host", host)
        .with("status", status.as_u16())
}

/// `rel="next"` из `Link`; адрес не своего API игнорируется — токен уйдёт только на свой хост.
pub(crate) fn next_link(headers: &HeaderMap, api: &str) -> Option<String> {
    let value = headers.get(LINK)?.to_str().ok()?;
    value
        .split(',')
        .find_map(|part| {
            let (url, rel) = part.split_once(';')?;
            (rel.trim() == r#"rel="next""#).then(|| {
                url.trim()
                    .trim_start_matches('<')
                    .trim_end_matches('>')
                    .to_string()
            })
        })
        .filter(|url| own_url(api, url))
}

/// Адрес на этом API: сам корень или путь под ним, а не просто общий префикс строки
/// (`https://api.github.com.evil/…` под префикс подходит, но чужой).
fn own_url(api: &str, url: &str) -> bool {
    url == api || url.starts_with(&format!("{api}/"))
}

/// Тип хоста по `GET /api/v3/meta` без токена: GHES ставит `X-GitHub-Enterprise-Version` на каждый
/// ответ API, в том числе 401 приватного режима; иначе GHES — 200 с `installed_version`, GitLab — нет.
pub async fn probe_at(host: &str, base_url: &str) -> Result<Provider, Error> {
    let response = http(host)?
        .get(format!("{}/api/v3/meta", base_url.trim_end_matches('/')))
        .send()
        .await
        .map_err(|e| network_error(host, e))?;
    if response
        .headers()
        .contains_key("x-github-enterprise-version")
    {
        return Ok(Provider::Github);
    }
    if !response.status().is_success() {
        return Ok(Provider::Gitlab);
    }
    let body: serde_json::Value = response.json().await.unwrap_or_default();
    Ok(if body.get("installed_version").is_some() {
        Provider::Github
    } else {
        Provider::Gitlab
    })
}

pub async fn probe(host: &str) -> Result<Provider, Error> {
    probe_at(host, &format!("https://{host}")).await
}

#[cfg(test)]
mod tests {
    use reqwest::StatusCode;
    use reqwest::header::{HeaderMap, HeaderValue};
    use time::macros::datetime;

    use super::*;

    const NOW: OffsetDateTime = datetime!(2026-10-07 10:00:00 UTC);

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        pairs
            .iter()
            .map(|&(k, v)| (k.parse().unwrap(), HeaderValue::from_str(v).unwrap()))
            .collect()
    }

    #[test]
    fn лимит_по_remaining_ноль_с_временем_сброса() {
        let e = status_error(
            "h",
            StatusCode::FORBIDDEN,
            &headers(&[
                ("x-ratelimit-remaining", "0"),
                ("x-ratelimit-reset", "1791367200"),
            ]),
            NOW,
        );
        assert_eq!(e.code, ErrorCode::RateLimited);
        assert_eq!(e.params["reset"], "2026-10-07T10:00:00.000Z");
    }

    #[test]
    fn вторичный_лимит_по_retry_after() {
        let e = status_error(
            "h",
            StatusCode::TOO_MANY_REQUESTS,
            &headers(&[("retry-after", "60")]),
            NOW,
        );
        assert_eq!(e.code, ErrorCode::RateLimited);
        assert_eq!(e.params["reset"], "2026-10-07T10:01:00.000Z");
    }

    #[test]
    fn без_лимита_403_это_нет_доступа_а_500_это_статус() {
        assert_eq!(
            status_error("h", StatusCode::FORBIDDEN, &HeaderMap::new(), NOW).code,
            ErrorCode::Unauthorized
        );
        assert_eq!(
            status_error("h", StatusCode::UNAUTHORIZED, &HeaderMap::new(), NOW).code,
            ErrorCode::Unauthorized
        );
        let e = status_error(
            "h",
            StatusCode::INTERNAL_SERVER_ERROR,
            &HeaderMap::new(),
            NOW,
        );
        assert_eq!(
            (e.code, e.params["status"].as_str()),
            (ErrorCode::HttpStatus, "500")
        );
    }

    #[test]
    fn следующая_страница_только_на_своём_api() {
        let link = r#"<https://api.github.com/repos/o/r/actions/runs?page=2>; rel="next", <https://api.github.com/repos/o/r/actions/runs?page=9>; rel="last""#;
        assert_eq!(
            next_link(&headers(&[("link", link)]), "https://api.github.com").as_deref(),
            Some("https://api.github.com/repos/o/r/actions/runs?page=2")
        );
        assert_eq!(
            next_link(&headers(&[("link", link)]), "https://ghe.example/api/v3"),
            None
        );
        assert_eq!(next_link(&HeaderMap::new(), "https://api.github.com"), None);
        let own = r#"<https://api.github.com/x>; rel="next""#;
        assert_eq!(
            next_link(&headers(&[("link", own)]), "https://api.github.com").as_deref(),
            Some("https://api.github.com/x")
        );
        for foreign in [
            "https://api.github.com.evil/x",
            "https://api.github.com:8443/x",
        ] {
            let link = format!(r#"<{foreign}>; rel="next""#);
            assert_eq!(
                next_link(&headers(&[("link", &link)]), "https://api.github.com"),
                None
            );
        }
    }

    #[test]
    fn огромный_retry_after_не_паникует() {
        let e = status_error(
            "h",
            StatusCode::TOO_MANY_REQUESTS,
            &headers(&[("retry-after", "9223372036854775807")]),
            NOW,
        );
        assert_eq!(e.code, ErrorCode::RateLimited);
        assert_eq!(e.params["reset"], "2026-10-07T10:00:00.000Z");
    }

    #[test]
    fn адрес_api_для_github_com_и_ghes() {
        assert_eq!(api_url("github.com"), "https://api.github.com");
        assert_eq!(api_url("ghe.example"), "https://ghe.example/api/v3");
    }

    #[tokio::test]
    async fn чужой_абсолютный_адрес_отказ_до_запроса() {
        let client = Client::with_api_url("h", "https://api.github.com", "t").unwrap();
        let err = client
            .json::<serde_json::Value>("https://api.github.com.evil/x")
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Network);
    }
}

use std::fmt::Write;
use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::Semaphore;

use super::Gql;
use crate::error::{Error, ErrorCode};

const MAX_PARALLEL: usize = 4;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const READ_TIMEOUT: Duration = Duration::from_secs(60);

/// HTTP-клиент GitLab GraphQL: не больше `MAX_PARALLEL` запросов одновременно.
pub struct Client {
    http: reqwest::Client,
    host: String,
    endpoint: String,
    token: String,
    permits: Semaphore,
}

#[derive(Deserialize)]
struct Response {
    data: Option<Value>,
    errors: Option<Vec<GraphqlError>>,
}

#[derive(Deserialize)]
struct GraphqlError {
    message: String,
}

impl Client {
    pub fn new(host: &str, token: &str) -> Result<Self, Error> {
        Self::with_base_url(host, &format!("https://{host}"), token)
    }

    /// `base_url` без `/api/graphql`; тесты HTTP-слоя подставляют адрес wiremock.
    pub fn with_base_url(host: &str, base_url: &str, token: &str) -> Result<Self, Error> {
        let http = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            // GraphQL GitLab не редиректит; 307/308 унёс бы POST с телом на чужой хост
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| network_error(host, e))?;
        Ok(Self {
            http,
            host: host.into(),
            endpoint: format!("{}/api/graphql", base_url.trim_end_matches('/')),
            token: token.into(),
            permits: Semaphore::new(MAX_PARALLEL),
        })
    }
}

/// Текст ошибки reqwest со всей цепочкой причин: сам по себе он ничего не говорит.
fn network_error(host: &str, e: reqwest::Error) -> Error {
    let e = e.without_url();
    let mut detail = e.to_string();
    let mut source = std::error::Error::source(&e);
    while let Some(cause) = source {
        write!(detail, ": {cause}").expect("запись в String не падает");
        source = cause.source();
    }
    Error::new(ErrorCode::Network)
        .with("host", host)
        .with("detail", detail)
}

impl Gql for Client {
    fn host(&self) -> &str {
        &self.host
    }

    async fn query(&self, query: &str, variables: Value) -> Result<Value, Error> {
        let _permit = self.permits.acquire().await.expect("семафор не закрывают");
        let response = self
            .http
            .post(&self.endpoint)
            .bearer_auth(&self.token)
            .json(&json!({ "query": query, "variables": variables }))
            .send()
            .await
            .map_err(|e| network_error(&self.host, e))?;

        let status = response.status();
        if matches!(status.as_u16(), 401 | 403) {
            return Err(self
                .error(ErrorCode::Unauthorized)
                .with("status", status.as_u16()));
        }
        if !status.is_success() {
            return Err(self
                .error(ErrorCode::HttpStatus)
                .with("status", status.as_u16()));
        }

        let body: Response = response
            .json()
            .await
            .map_err(|e| network_error(&self.host, e))?;
        if let Some(errors) = body.errors.filter(|e| !e.is_empty()) {
            let messages: Vec<_> = errors.into_iter().map(|e| e.message).collect();
            return Err(self
                .error(ErrorCode::Graphql)
                .with("detail", messages.join("; ")));
        }
        Ok(body.data.unwrap_or(Value::Null))
    }
}

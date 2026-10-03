use std::collections::BTreeMap;

use serde::Serialize;
use ts_rs::TS;

/// Код ошибки: перевод и подстановка `params` — на фронтенде.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ErrorCode {
    /// 401/403: `host`, `status`
    Unauthorized,
    /// прочий не-2xx: `host`, `status`
    HttpStatus,
    /// `errors[]` GraphQL или неожиданная форма ответа: `host`, `detail` — текст GitLab или serde
    Graphql,
    /// нет соединения, TLS, не JSON: `host`, `detail` — текст ошибки транспорта
    Network,
    /// `project`, `host`
    ProjectNotFound,
    /// `pipeline`, `project`, `host`
    PipelineNotFound,
    /// `pipeline`, `project`, `limit`
    TooManyJobs,
    /// `iid`, `project`
    MrHasNoPipeline,
}

/// Ошибка ядра: код и параметры, без готового текста.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code:?} {params:?}")]
pub struct Error {
    pub code: ErrorCode,
    pub params: BTreeMap<&'static str, String>,
}

impl Error {
    pub fn new(code: ErrorCode) -> Self {
        Self {
            code,
            params: BTreeMap::new(),
        }
    }

    pub fn with(mut self, key: &'static str, value: impl ToString) -> Self {
        self.params.insert(key, value.to_string());
        self
    }
}

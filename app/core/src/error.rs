use std::collections::BTreeMap;
use std::fmt::Write;

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
    /// фильтры агрегата не нашли ни одного пайплайна
    NoPipelines,
    /// нет токена ни в хранилище, ни в `glab`, ни в окружении: `host`
    NoToken,
    EmptyToken,
    /// хранение токенов выключено или запись отклонена платформой
    KeychainUnavailable,
    /// файл данных или хранилище отдали неожиданный ответ: `detail` — текст ошибки
    Storage,
    /// поле `url` формы: ссылка не на пайплайн и не на MR
    InvalidLink,
    InvalidHost,
    InvalidProject,
    /// `max`
    InvalidLast,
    InvalidStatuses,
    /// команду вызвало не окно формы
    Forbidden,
    /// окно или меню не создались: `detail` — текст ошибки Tauri
    Window,
    /// поле ввода проекта: не ссылка на проект GitLab, не ssh-URL и не путь к папке
    ProjectInputInvalid,
    /// папка без `.git` или без `remote "origin"`
    ProjectDirNoRemote,
    /// отчёт вытеснен из памяти: `id`
    ReportExpired,
    /// лимит запросов GitHub: `host`, `reset` — ISO 8601, когда можно повторить
    RateLimited,
    /// агрегат GitHub без workflow
    WorkflowRequired,
}

/// Ошибка ядра: код и параметры, без готового текста.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, TS)]
#[error("{code:?} {params:?}")]
// `Error` затенил бы глобальный в сгенерированном TS
#[ts(export, rename = "ErrorBody")]
pub struct Error {
    pub code: ErrorCode,
    pub params: BTreeMap<&'static str, String>,
}

impl Error {
    /// Ошибка без параметров; добавляйте их через [`Error::with`].
    pub fn new(code: ErrorCode) -> Self {
        Self {
            code,
            params: BTreeMap::new(),
        }
    }

    /// Параметр для подстановки в перевод; повторный ключ перезаписывается.
    pub fn with(mut self, key: &'static str, value: impl ToString) -> Self {
        self.params.insert(key, value.to_string());
        self
    }
}

/// Поле формы сборки, к которому относится ошибка.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Field {
    Url,
    Host,
    Project,
    Last,
    Statuses,
}

/// Что получает фронтенд от команды: общая ошибка или ошибки по полям формы (их возвращает только `build`).
#[derive(Debug, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum CmdError {
    Message(Error),
    Fields { errors: BTreeMap<Field, Error> },
}

impl From<Error> for CmdError {
    fn from(e: Error) -> Self {
        Self::Message(e)
    }
}

impl From<BTreeMap<Field, Error>> for CmdError {
    fn from(errors: BTreeMap<Field, Error>) -> Self {
        Self::Fields { errors }
    }
}

/// Текст ошибки reqwest со всей цепочкой причин: сам по себе он ничего не говорит.
pub(crate) fn network_error(host: &str, e: reqwest::Error) -> Error {
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

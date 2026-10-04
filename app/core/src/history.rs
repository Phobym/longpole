//! История запросов: `history.json`, не больше `LIMIT` записей, новые сверху.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;

use crate::error::Error;
use crate::iso::iso;
use crate::json_file;
use crate::request::{Form, FormMode, Request};

const LIMIT: usize = 20;

/// Подпись записи: форма рендерит её на текущем языке.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HistoryLabel {
    pub project: String,
    /// как `meta.label`: `None` — «все пайплайны»
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HistoryEntry {
    /// ISO 8601; он же идентификатор записи
    pub at: String,
    pub host: String,
    pub form: Form,
    pub request: Request,
    pub label: HistoryLabel,
}

/// Запись без времени: его проставляет `History::add`.
#[derive(Debug, Clone)]
pub struct NewEntry {
    pub host: String,
    pub form: Form,
    pub request: Request,
    pub label: HistoryLabel,
}

/// Ссылка на пайплайн может нести `?private_token=…`: в историю идёт только путь.
/// В режиме агрегата поле `url` не участвует в запросе, а в нём могла остаться ссылка со своим секретом.
fn stored_url(mode: FormMode, mut url: String) -> String {
    if mode == FormMode::Aggregate {
        return String::new();
    }
    if let Some(at) = url.find(['?', '#']) {
        url.drain(at..);
    }
    url
}

pub struct History {
    file: PathBuf,
    /// чтение-изменение-запись не должны перемешиваться у одновременных сборок
    lock: Mutex<()>,
}

impl History {
    pub fn new(file: impl Into<PathBuf>) -> Self {
        Self {
            file: file.into(),
            lock: Mutex::new(()),
        }
    }

    pub fn list(&self) -> Result<Vec<HistoryEntry>, Error> {
        json_file::read(&self.file)
    }

    /// Запись наверх; прежняя с тем же хостом и запросом заменяется.
    pub fn add(&self, new: NewEntry, now: OffsetDateTime) -> Result<Vec<HistoryEntry>, Error> {
        let _guard = json_file::lock(&self.lock);
        let entry = HistoryEntry {
            at: iso(now),
            host: new.host,
            form: Form {
                url: stored_url(new.form.mode, new.form.url),
                ..new.form
            },
            request: new.request,
            label: new.label,
        };
        let rest: Vec<_> = self
            .list()?
            .into_iter()
            .filter(|e| (&e.host, &e.request) != (&entry.host, &entry.request))
            .collect();
        let mut next = vec![entry];
        next.extend(rest);
        next.truncate(LIMIT);
        self.save(next)
    }

    pub fn remove(&self, at: &str) -> Result<Vec<HistoryEntry>, Error> {
        let _guard = json_file::lock(&self.lock);
        self.save(self.list()?.into_iter().filter(|e| e.at != at).collect())
    }

    pub fn clear(&self) -> Result<(), Error> {
        let _guard = json_file::lock(&self.lock);
        self.save(Vec::new()).map(drop)
    }

    fn save(&self, list: Vec<HistoryEntry>) -> Result<Vec<HistoryEntry>, Error> {
        json_file::write(&self.file, &list)?;
        Ok(list)
    }
}

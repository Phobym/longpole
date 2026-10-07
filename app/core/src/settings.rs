//! Настройки приложения: `settings.json` — язык, тема, последний открытый проект, типы хостов (GitLab или GitHub).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::Error;
use crate::json_file;
use crate::request::ProjectRef;
use crate::schema::Locale;
use crate::source::{GITHUB_COM, Provider};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

/// Что лежит в файле: всё необязательное, старый файл только с `locale` читается.
#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stored {
    locale: Option<Locale>,
    theme: Option<Theme>,
    last_project: Option<ProjectRef>,
    /// хост → тип; `github.com` не пишется — он известен
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    host_kinds: BTreeMap<String, Provider>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppSettings {
    pub locale: Locale,
    pub theme: Theme,
    pub last_project: Option<ProjectRef>,
}

/// Частичное обновление: `None` — поле не трогать.
#[derive(Debug, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct SettingsPatch {
    #[ts(optional)]
    pub locale: Option<Locale>,
    #[ts(optional)]
    pub theme: Option<Theme>,
    #[ts(optional)]
    pub last_project: Option<ProjectRef>,
}

pub struct Settings {
    file: PathBuf,
    lock: Mutex<()>,
}

impl Settings {
    pub fn new(file: impl Into<PathBuf>) -> Self {
        Self {
            file: file.into(),
            lock: Mutex::new(()),
        }
    }

    pub fn get(&self) -> Result<AppSettings, Error> {
        let stored: Stored = json_file::read(&self.file)?;
        Ok(AppSettings {
            locale: stored.locale.unwrap_or_else(Locale::system),
            theme: stored.theme.unwrap_or_default(),
            last_project: stored.last_project,
        })
    }

    /// Выбранный язык, а пока выбора не было — системный.
    pub fn locale(&self) -> Result<Locale, Error> {
        self.get().map(|s| s.locale)
    }

    pub fn update(&self, patch: SettingsPatch) -> Result<AppSettings, Error> {
        let _guard = json_file::lock(&self.lock);
        let mut stored: Stored = json_file::read(&self.file)?;
        if patch.locale.is_some() {
            stored.locale = patch.locale;
        }
        if patch.theme.is_some() {
            stored.theme = patch.theme;
        }
        if patch.last_project.is_some() {
            stored.last_project = patch.last_project;
        }
        json_file::write(&self.file, &stored)?;
        drop(_guard);
        self.get()
    }

    /// Тип хоста без сети: `github.com` — GitHub, остальные — из кэша пробы.
    pub fn host_kind(&self, host: &str) -> Result<Option<Provider>, Error> {
        if host == GITHUB_COM {
            return Ok(Some(Provider::Github));
        }
        let stored: Stored = json_file::read(&self.file)?;
        Ok(stored.host_kinds.get(host).copied())
    }

    pub fn set_host_kind(&self, host: &str, provider: Provider) -> Result<(), Error> {
        if host == GITHUB_COM {
            return Ok(());
        }
        let _guard = json_file::lock(&self.lock);
        let mut stored: Stored = json_file::read(&self.file)?;
        if stored.host_kinds.get(host) != Some(&provider) {
            stored.host_kinds.insert(host.into(), provider);
            json_file::write(&self.file, &stored)?;
        }
        Ok(())
    }

    /// Проект убран из списка: он больше не «последний».
    pub fn forget_project(&self, project: &ProjectRef) -> Result<(), Error> {
        let _guard = json_file::lock(&self.lock);
        let mut stored: Stored = json_file::read(&self.file)?;
        if stored.last_project.as_ref() == Some(project) {
            stored.last_project = None;
            json_file::write(&self.file, &stored)?;
        }
        Ok(())
    }
}

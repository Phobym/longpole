//! Хосты: проверка имени; в Desktop — ещё список хостов с сохранёнными токенами.

#[cfg(feature = "desktop")]
use std::path::PathBuf;
use std::sync::LazyLock;
#[cfg(feature = "desktop")]
use std::sync::Mutex;

use regex::Regex;

use crate::error::{Error, ErrorCode};
#[cfg(feature = "desktop")]
use crate::json_file;

static HOST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Za-z0-9]([A-Za-z0-9.-]*[A-Za-z0-9])?(:[0-9]+)?$").expect("верный шаблон")
});

/// Имя хоста с необязательным портом, без схемы и пути.
pub fn is_valid_host(host: &str) -> bool {
    HOST.is_match(host)
}

/// Хост без схемы и завершающего слэша: так его пишут в `GITLAB_HOST` и в адресной строке.
pub(crate) fn bare(host: &str) -> &str {
    let host = host.trim();
    let host = host
        .strip_prefix("https://")
        .or_else(|| host.strip_prefix("http://"))
        .unwrap_or(host);
    host.strip_suffix('/').unwrap_or(host)
}

/// Хост из поля ввода: `https://gitlab.example.com/` → `gitlab.example.com`.
pub fn normalize_host(raw: &str) -> Result<String, Error> {
    let host = bare(raw);
    if is_valid_host(host) {
        Ok(host.into())
    } else {
        Err(Error::new(ErrorCode::InvalidHost))
    }
}

/// Хосты, для которых сохранён токен: `hosts.json`, по алфавиту, без секретов.
#[cfg(feature = "desktop")]
pub(crate) struct HostList {
    file: PathBuf,
    lock: Mutex<()>,
}

#[cfg(feature = "desktop")]
impl HostList {
    pub(crate) fn new(file: PathBuf) -> Self {
        Self {
            file,
            lock: Mutex::new(()),
        }
    }

    pub(crate) fn list(&self) -> Result<Vec<String>, Error> {
        json_file::read(&self.file)
    }

    pub(crate) fn add(&self, host: &str) -> Result<(), Error> {
        self.update(|hosts| hosts.push(host.into()))
    }

    pub(crate) fn remove(&self, host: &str) -> Result<(), Error> {
        self.update(|hosts| hosts.retain(|h| h != host))
    }

    fn update(&self, change: impl FnOnce(&mut Vec<String>)) -> Result<(), Error> {
        let _guard = json_file::lock(&self.lock);
        let mut hosts = self.list()?;
        change(&mut hosts);
        hosts.sort();
        hosts.dedup();
        json_file::write(&self.file, &hosts)
    }
}

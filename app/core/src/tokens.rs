//! Токены GitLab: системная связка ключей через `keyring-core`, `glab` и поиск токена для хоста.

use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use keyring_core::{CredentialStore, Entry, Error as KeyringError};

use crate::error::{Error, ErrorCode};
use crate::hosts::{HostList, bare, normalize_host};

/// Service записи в связке ключей; account — хост.
pub const SERVICE: &str = "dev.pipeline-trace.desktop";

/// Платформенное хранилище; `None` — системного нет (Linux без Secret Service), хранение токенов выключено.
pub fn platform_store() -> Option<Arc<CredentialStore>> {
    #[cfg(target_os = "macos")]
    let store =
        apple_native_keyring_store::keychain::Store::new().map(|s| s as Arc<CredentialStore>);
    #[cfg(target_os = "windows")]
    let store = windows_native_keyring_store::Store::new().map(|s| s as Arc<CredentialStore>);
    #[cfg(target_os = "linux")]
    let store = dbus_secret_service_keyring_store::Store::new().map(|s| s as Arc<CredentialStore>);
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    let store: Result<Arc<CredentialStore>, KeyringError> = Err(KeyringError::NoDefaultStore);
    store.ok()
}

/// Платформа не даёт хранить токены: нет Secret Service, связка ключей заблокирована или отказала.
fn is_unavailable(e: &KeyringError) -> bool {
    matches!(
        e,
        KeyringError::PlatformFailure(_) | KeyringError::NoStorageAccess(_)
    )
}

/// Отказ платформы — хранение выключено, остальное — сбой хранилища с текстом ошибки.
fn key_error(e: KeyringError) -> Error {
    if is_unavailable(&e) {
        Error::new(ErrorCode::KeychainUnavailable)
    } else {
        Error::new(ErrorCode::Storage).with("detail", e)
    }
}

/// Для чтения недоступное хранилище — «токена нет»: остаются `glab` и env; прочие сбои идут наружу.
fn not_stored(e: Error) -> Result<Option<String>, Error> {
    if e.code == ErrorCode::KeychainUnavailable {
        Ok(None)
    } else {
        Err(e)
    }
}

/// Токены в связке ключей и список хостов, для которых они сохранены.
pub struct TokenStore {
    store: Option<Arc<CredentialStore>>,
    hosts: HostList,
}

impl TokenStore {
    pub fn new(hosts_file: impl Into<PathBuf>, store: Option<Arc<CredentialStore>>) -> Self {
        Self {
            store,
            hosts: HostList::new(hosts_file.into()),
        }
    }

    /// Хосты с сохранёнными токенами, по алфавиту.
    pub fn hosts(&self) -> Result<Vec<String>, Error> {
        self.hosts.list()
    }

    /// Токен хоста; недоступное хранилище и битая запись — как «токена нет», чтобы работали `glab` и env.
    pub fn get(&self, host: &str) -> Result<Option<String>, Error> {
        let entry = match self.entry(host) {
            Ok(Some(entry)) => entry,
            Ok(None) => return Ok(None),
            Err(e) => return not_stored(e),
        };
        match entry.get_password() {
            Ok(token) => Ok(Some(token)),
            Err(
                KeyringError::NoEntry
                | KeyringError::BadEncoding(_)
                | KeyringError::BadDataFormat(..),
            ) => Ok(None),
            Err(e) => not_stored(key_error(e)),
        }
    }

    pub fn set(&self, host: &str, token: &str) -> Result<(), Error> {
        let host = normalize_host(host)?;
        let Some(entry) = self.entry(&host)? else {
            return Err(Error::new(ErrorCode::KeychainUnavailable));
        };
        let token = token.trim();
        if token.is_empty() {
            return Err(Error::new(ErrorCode::EmptyToken));
        }
        entry.set_password(token).map_err(key_error)?;
        self.hosts.add(&host)
    }

    pub fn remove(&self, host: &str) -> Result<(), Error> {
        if let Some(entry) = self.entry(host)? {
            match entry.delete_credential() {
                Ok(()) | Err(KeyringError::NoEntry) => {}
                Err(e) => return Err(key_error(e)),
            }
        }
        self.hosts.remove(host)
    }

    fn entry(&self, host: &str) -> Result<Option<Entry>, Error> {
        let Some(store) = &self.store else {
            return Ok(None);
        };
        store
            .build(SERVICE, host, None)
            .map(Some)
            .map_err(key_error)
    }
}

/// Токен для хоста: хранилище → `glab` → `GITLAB_TOKEN` (если `GITLAB_HOST` или хост `glab` по умолчанию совпадает).
/// `env` и `glab` — аргументы ради тестов.
pub fn find_token(
    host: &str,
    store: &TokenStore,
    env: &dyn Fn(&str) -> Option<String>,
    glab: &dyn Fn(&[&str]) -> Option<String>,
) -> Result<String, Error> {
    if let Some(token) = store.get(host)? {
        return Ok(token);
    }
    if let Some(token) = glab(&["config", "get", "token", "--host", host]) {
        return Ok(token);
    }
    if let Some(token) = env("GITLAB_TOKEN").filter(|t| !t.is_empty()) {
        let is_host = |other: &str| bare(other) == bare(host);
        if env("GITLAB_HOST").is_some_and(|h| is_host(&h))
            || glab(&["config", "get", "host"]).is_some_and(|h| is_host(&h))
        {
            return Ok(token);
        }
    }
    Err(Error::new(ErrorCode::NoToken).with("host", host))
}

/// Приложение из GUI не наследует `PATH` терминала, поэтому после него смотрим типовые каталоги.
const FIXED_DIRS: [&str; 3] = [
    "/opt/homebrew/bin",
    "/usr/local/bin",
    "/home/linuxbrew/.linuxbrew/bin",
];

/// Найденный исполняемый файл `glab`.
pub struct Glab(PathBuf);

impl Glab {
    pub fn find() -> Option<Glab> {
        let dirs = Self::search_dirs(env::var_os("PATH").as_deref(), env::home_dir().as_deref());
        Self::find_in(&dirs)
    }

    /// `PATH` процесса, типовые каталоги, `~/.local/bin`.
    pub fn search_dirs(path_var: Option<&OsStr>, home: Option<&Path>) -> Vec<PathBuf> {
        path_var
            .into_iter()
            .flat_map(env::split_paths)
            // пустой и относительный элемент PATH — это текущий каталог, исполнять оттуда нельзя
            .filter(|dir| dir.is_absolute())
            .chain(FIXED_DIRS.map(PathBuf::from))
            .chain(home.map(|h| h.join(".local").join("bin")))
            .collect()
    }

    pub fn find_in(dirs: &[PathBuf]) -> Option<Glab> {
        let name = format!("glab{}", env::consts::EXE_SUFFIX);
        dirs.iter()
            .map(|dir| dir.join(&name))
            .find(|path| path.is_file())
            .map(Glab)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Вывод команды; любой сбой и пустой вывод — `None`: `glab` может быть не настроен, это не ошибка.
    // ponytail: без таймаута, `glab config get` локальный и мгновенный
    pub fn value(&self, args: &[&str]) -> Option<String> {
        let output = Command::new(&self.0)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string()).filter(|s| !s.is_empty())
    }
}

//! Токены GitLab: системная связка ключей через `keyring-core`, `glab` и поиск токена для хоста.

use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use keyring_core::{CredentialStore, Entry, Error as KeyringError};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Error, ErrorCode};
use crate::hosts::{HostList, bare, normalize_host};
use crate::source::{GITHUB_COM, Provider};

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
        let host = normalize_host(host)?;
        if let Some(entry) = self.entry(&host)? {
            match entry.delete_credential() {
                Ok(()) | Err(KeyringError::NoEntry) => {}
                Err(e) => return Err(key_error(e)),
            }
        }
        self.hosts.remove(&host)
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

/// Откуда у хоста токен; «Удалить» в настройках есть только у `Keychain`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum TokenSource {
    Keychain,
    Glab,
    Env,
    Gh,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct HostInfo {
    pub host: String,
    pub source: TokenSource,
    /// `None` — хост из связки ключей, тип ещё не определён
    pub provider: Option<Provider>,
}

/// Хосты, для которых токен найдётся: связка ключей (по алфавиту), хост `glab` по умолчанию, `GITLAB_HOST`.
pub fn list_hosts(
    store: &TokenStore,
    env: &dyn Fn(&str) -> Option<String>,
    glab: &dyn Fn(&[&str]) -> Option<String>,
) -> Result<Vec<HostInfo>, Error> {
    let mut hosts: Vec<HostInfo> = store
        .hosts()?
        .into_iter()
        .map(|host| HostInfo {
            host,
            source: TokenSource::Keychain,
            provider: None,
        })
        .collect();
    let mut push = |host: String, source: TokenSource| {
        if !hosts.iter().any(|h| h.host == host) {
            hosts.push(HostInfo {
                host,
                source,
                provider: Some(Provider::Gitlab),
            });
        }
    };
    if let Some(host) = glab(&["config", "get", "host"]).map(|h| bare(&h).to_string())
        && glab(&["config", "get", "token", "--host", &host]).is_some()
    {
        push(host, TokenSource::Glab);
    }
    if env("GITLAB_TOKEN").is_some_and(|t| !t.is_empty())
        && let Some(host) = env("GITLAB_HOST")
            .map(|h| bare(&h).to_string())
            .filter(|h| !h.is_empty())
    {
        push(host, TokenSource::Env);
    }
    Ok(hosts)
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
        find_tool("glab", dirs).map(Glab)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn value(&self, args: &[&str]) -> Option<String> {
        tool_value(&self.0, args)
    }
}

/// Найденный исполняемый файл `gh`.
pub struct Gh(PathBuf);

impl Gh {
    pub fn find() -> Option<Gh> {
        let dirs = Glab::search_dirs(env::var_os("PATH").as_deref(), env::home_dir().as_deref());
        find_tool("gh", &dirs).map(Gh)
    }

    pub fn value(&self, args: &[&str]) -> Option<String> {
        tool_value(&self.0, args)
    }
}

fn find_tool(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    let name = format!("{name}{}", env::consts::EXE_SUFFIX);
    dirs.iter()
        .map(|dir| dir.join(&name))
        .find(|path| path.is_file())
}

/// Вывод команды; любой сбой и пустой вывод — `None`: CLI может быть не настроен, это не ошибка.
// ponytail: без таймаута, `config get` и `auth token` локальные и мгновенные
fn tool_value(path: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new(path)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string()).filter(|s| !s.is_empty())
}

/// Токен GitHub (спека, § 4): хранилище → `gh auth token` → `GH_TOKEN`/`GITHUB_TOKEN` для github.com,
/// `GH_ENTERPRISE_TOKEN`/`GITHUB_ENTERPRISE_TOKEN` для GHES из `GH_HOST`.
pub fn find_github_token(
    host: &str,
    store: &TokenStore,
    env: &dyn Fn(&str) -> Option<String>,
    gh: &dyn Fn(&[&str]) -> Option<String>,
) -> Result<String, Error> {
    if let Some(token) = store.get(host)? {
        return Ok(token);
    }
    if let Some(token) = gh(&["auth", "token", "--hostname", host]) {
        return Ok(token);
    }
    let named = |name: &str| env(name).filter(|t| !t.is_empty());
    let token = if host == GITHUB_COM {
        named("GH_TOKEN").or_else(|| named("GITHUB_TOKEN"))
    } else if env("GH_HOST").is_some_and(|h| bare(&h) == host) {
        named("GH_ENTERPRISE_TOKEN").or_else(|| named("GITHUB_ENTERPRISE_TOKEN"))
    } else {
        None
    };
    token.ok_or_else(|| Error::new(ErrorCode::NoToken).with("host", host))
}

#[derive(Deserialize)]
struct GhStatus {
    hosts: BTreeMap<String, Vec<GhAccount>>,
}

#[derive(Deserialize)]
struct GhAccount {
    state: String,
}

/// Хосты GitHub с токеном у `gh` (по алфавиту) и github.com из окружения.
pub fn github_hosts(
    env: &dyn Fn(&str) -> Option<String>,
    gh: &dyn Fn(&[&str]) -> Option<String>,
) -> Vec<HostInfo> {
    let info = |host: String, source| HostInfo {
        host,
        source,
        provider: Some(Provider::Github),
    };
    let mut hosts: Vec<HostInfo> = match gh(&["auth", "status", "--json", "hosts"])
        .and_then(|text| serde_json::from_str::<GhStatus>(&text).ok())
    {
        Some(status) => status
            .hosts
            .into_iter()
            .filter(|(_, accounts)| accounts.iter().any(|a| a.state == "success"))
            .map(|(host, _)| info(host, TokenSource::Gh))
            .collect(),
        // старый `gh` без `--json`
        None => gh(&["auth", "token", "--hostname", GITHUB_COM])
            .map(|_| info(GITHUB_COM.into(), TokenSource::Gh))
            .into_iter()
            .collect(),
    };
    let env_token = ["GH_TOKEN", "GITHUB_TOKEN"]
        .iter()
        .any(|name| env(name).is_some_and(|t| !t.is_empty()));
    if env_token && !hosts.iter().any(|h| h.host == GITHUB_COM) {
        hosts.push(info(GITHUB_COM.into(), TokenSource::Env));
    }
    hosts
}

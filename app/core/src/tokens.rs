//! Токены GitLab и GitHub: системная связка ключей через `keyring-core`, `glab` и поиск токена для хоста.

use std::collections::{BTreeMap, HashMap};
use std::env;
use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::thread;
use std::time::Duration;

use keyring_core::{CredentialStore, Entry, Error as KeyringError};
use serde::Deserialize;
use serde::de::IgnoredAny;

use crate::error::{Error, ErrorCode};
use crate::hosts::{HostInfo, HostList, TokenSource, bare, normalize_host};
use crate::oauth::{self, Credential, RefreshError};
use crate::source::{GITHUB_COM, Provider};

/// Service записи в связке ключей; account — хост.
pub const SERVICE: &str = "dev.longpole.desktop";

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
    /// хост → замок обновления: GitLab при refresh отзывает старую пару, обновлять её дважды нельзя
    refreshing: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

/// Что лежит в связке ключей: готовый токен или запись OAuth, которую пора обновить.
enum Stored {
    Ready(Option<String>),
    Stale(Credential),
}

impl TokenStore {
    pub fn new(hosts_file: impl Into<PathBuf>, store: Option<Arc<CredentialStore>>) -> Self {
        Self {
            store,
            hosts: HostList::new(hosts_file.into()),
            refreshing: Mutex::default(),
        }
    }

    /// Хосты с сохранёнными токенами, по алфавиту.
    pub fn hosts(&self) -> Result<Vec<String>, Error> {
        self.hosts.list()
    }

    /// Токен хоста; недоступное хранилище и битая запись — как «токена нет», чтобы работали `glab` и env.
    /// Запись OAuth за 5 минут до срока обновляется через `block_on`: вызывать из пула блокирующих задач tokio.
    pub fn get(&self, host: &str) -> Result<Option<String>, Error> {
        // ключ замка — нормализованный хост, как у писателей
        let host = normalize_host(host)?;
        if let Stored::Ready(token) = self.stored(&host)? {
            return Ok(token);
        }
        let lock = self.refresh_lock(&host);
        let _guard = lock.lock().unwrap_or_else(PoisonError::into_inner);
        // пока ждали замок, запись мог обновить другой поток
        match self.stored(&host)? {
            Stored::Ready(token) => Ok(token),
            Stored::Stale(credential) => self.refresh(&host, &credential),
        }
    }

    /// Запись хоста — OAuth: в настройках у неё своя подпись.
    pub fn is_oauth(&self, host: &str) -> bool {
        matches!(self.read(host), Ok(Some(value)) if Credential::parse(&value).is_some())
    }

    pub fn set(&self, host: &str, token: &str) -> Result<(), Error> {
        self.write(host, token.trim())
    }

    pub fn set_oauth(&self, host: &str, credential: &Credential) -> Result<(), Error> {
        self.write(host, &Self::to_json(credential)?)
    }

    /// Ждёт замок хоста: идущее обновление не должно потом вернуть удалённую запись.
    pub fn remove(&self, host: &str) -> Result<(), Error> {
        let host = normalize_host(host)?;
        let lock = self.refresh_lock(&host);
        let _guard = lock.lock().unwrap_or_else(PoisonError::into_inner);
        self.delete(&host)
    }

    fn to_json(credential: &Credential) -> Result<String, Error> {
        serde_json::to_string(credential)
            .map_err(|e| Error::new(ErrorCode::Storage).with("detail", e))
    }

    /// Запись под замком хоста, чтобы обновление не затёрло новый вход или токен.
    /// Порядок проверок прежний: хост, хранилище, пустое значение.
    fn write(&self, host: &str, value: &str) -> Result<(), Error> {
        let host = normalize_host(host)?;
        let lock = self.refresh_lock(&host);
        let _guard = lock.lock().unwrap_or_else(PoisonError::into_inner);
        self.put(&host, value)
    }

    /// Запись без замка — для `refresh`, который уже держит его внутри `get`.
    fn put(&self, host: &str, value: &str) -> Result<(), Error> {
        let Some(entry) = self.entry(host)? else {
            return Err(Error::new(ErrorCode::KeychainUnavailable));
        };
        if value.is_empty() {
            return Err(Error::new(ErrorCode::EmptyToken));
        }
        entry.set_password(value).map_err(key_error)?;
        self.hosts.add(host)
    }

    /// Удаление без замка, см. `put`.
    fn delete(&self, host: &str) -> Result<(), Error> {
        if let Some(entry) = self.entry(host)? {
            match entry.delete_credential() {
                Ok(()) | Err(KeyringError::NoEntry) => {}
                Err(e) => return Err(key_error(e)),
            }
        }
        self.hosts.remove(host)
    }

    /// Значение записи как есть: PAT или JSON OAuth.
    fn read(&self, host: &str) -> Result<Option<String>, Error> {
        let entry = match self.entry(host) {
            Ok(Some(entry)) => entry,
            Ok(None) => return Ok(None),
            Err(e) => return not_stored(e),
        };
        match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(
                KeyringError::NoEntry
                | KeyringError::BadEncoding(_)
                | KeyringError::BadDataFormat(..),
            ) => Ok(None),
            Err(e) => not_stored(key_error(e)),
        }
    }

    fn stored(&self, host: &str) -> Result<Stored, Error> {
        let Some(value) = self.read(host)? else {
            return Ok(Stored::Ready(None));
        };
        Ok(match Credential::parse(&value) {
            None => Stored::Ready(Some(value)),
            Some(credential) if credential.is_fresh(oauth::now()) => {
                Stored::Ready(Some(credential.access))
            }
            Some(credential) => Stored::Stale(credential),
        })
    }

    fn refresh(&self, host: &str, credential: &Credential) -> Result<Option<String>, Error> {
        let runtime = tokio::runtime::Handle::try_current()
            .map_err(|e| Error::new(ErrorCode::Storage).with("detail", e))?;
        match runtime.block_on(oauth::refresh(credential)) {
            Ok(renewed) => {
                self.put(host, &Self::to_json(&renewed)?)?;
                Ok(Some(renewed.access))
            }
            // отозван или истёк: дальше ищем в `glab`/`gh` и env, в конце — блок входа
            Err(RefreshError::Revoked) => {
                self.delete(host)?;
                Ok(None)
            }
            Err(RefreshError::Failed(e)) => Err(e),
        }
    }

    // ponytail: замки не удаляются — их столько, сколько хостов с OAuth за сеанс
    fn refresh_lock(&self, host: &str) -> Arc<Mutex<()>> {
        self.refreshing
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(host.to_string())
            .or_default()
            .clone()
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
            source: Some(if store.is_oauth(&host) {
                TokenSource::Oauth
            } else {
                TokenSource::Keychain
            }),
            host,
            provider: None,
            login: None,
        })
        .collect();
    let mut push = |host: String, source: TokenSource| {
        if !hosts.iter().any(|h| h.host == host) {
            hosts.push(HostInfo {
                host,
                source: Some(source),
                provider: Some(Provider::Gitlab),
                login: None,
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
        tool_value(Command::new(&self.0).args(args), None, false)
    }
}

/// `gh auth status` ходит в сеть проверять токены: офлайн ждать его незачем.
const GH_TIMEOUT: Duration = Duration::from_secs(5);

const GH_ENV_TOKENS: [&str; 4] = [
    "GH_TOKEN",
    "GITHUB_TOKEN",
    "GH_ENTERPRISE_TOKEN",
    "GITHUB_ENTERPRISE_TOKEN",
];

/// Найденный исполняемый файл `gh`.
pub struct Gh(PathBuf);

impl Gh {
    pub fn find() -> Option<Gh> {
        let dirs = Glab::search_dirs(env::var_os("PATH").as_deref(), env::home_dir().as_deref());
        Self::find_in(&dirs)
    }

    pub fn find_in(dirs: &[PathBuf]) -> Option<Gh> {
        find_tool("gh", dirs).map(Gh)
    }

    /// Токены окружения `gh` не отдаём: `gh auth token --hostname H` вернул бы `GH_ENTERPRISE_TOKEN`
    /// любому enterprise-хосту, а их привязку к `GH_HOST` разбирает `find_github_token`.
    ///
    /// `gh auth status` выходит с кодом 1, если хоть одна учётная запись не прошла проверку (офлайн,
    /// отозванный токен), но JSON печатает; поэтому его вывод берём при любом коде, остальное — только при успехе.
    pub fn value(&self, args: &[&str]) -> Option<String> {
        tool_value(
            without_env_tokens(Command::new(&self.0).args(args)),
            Some(GH_TIMEOUT),
            args.starts_with(&["auth", "status"]),
        )
    }
}

fn find_tool(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    let name = format!("{name}{}", env::consts::EXE_SUFFIX);
    dirs.iter()
        .map(|dir| dir.join(&name))
        .find(|path| path.is_file())
}

/// Команда без токенов окружения GitHub (см. `Gh::value`).
fn without_env_tokens(command: &mut Command) -> &mut Command {
    for name in GH_ENV_TOKENS {
        command.env_remove(name);
    }
    command
}

/// Вывод команды; любой сбой, таймаут и пустой вывод — `None`: CLI может быть не настроен, это не ошибка.
/// `any_status` — брать вывод и при ненулевом коде выхода.
// ponytail: у `glab` без таймаута — `config get` локальный и мгновенный; `gh auth status` ходит в сеть,
// его ограничивают 5 с, после чего процесс убивается
fn tool_value(
    command: &mut Command,
    timeout: Option<Duration>,
    any_status: bool,
) -> Option<String> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command.spawn().ok()?;
    let mut stdout = child.stdout.take()?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        let _ = tx.send(buf);
    });
    let output = match timeout {
        Some(limit) => rx.recv_timeout(limit).ok(),
        None => rx.recv().ok(),
    };
    if output.is_none() {
        let _ = child.kill();
    }
    let status = child.wait().ok()?;
    if !any_status && !status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output?).trim().to_string()).filter(|s| !s.is_empty())
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
    hosts: BTreeMap<String, Vec<IgnoredAny>>,
}

/// Хосты GitHub с учётной записью в `gh` (по алфавиту) и github.com из окружения.
pub fn github_hosts(
    env: &dyn Fn(&str) -> Option<String>,
    gh: &dyn Fn(&[&str]) -> Option<String>,
) -> Vec<HostInfo> {
    let info = |host: String, source| HostInfo {
        host,
        source: Some(source),
        provider: Some(Provider::Github),
        login: None,
    };
    let mut hosts: Vec<HostInfo> = match gh(&["auth", "status", "--json", "hosts"])
        .and_then(|text| serde_json::from_str::<GhStatus>(&text).ok())
    {
        Some(status) => status
            .hosts
            .into_iter()
            // токены `gh` проверяет по сети: офлайн ни у одного `state` не success, а негодный токен
            // честнее показать ошибкой `unauthorized` при использовании, чем скрыть хост
            .filter(|(_, accounts)| !accounts.is_empty())
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn токены_окружения_у_gh_удаляются() {
        let mut command = Command::new("sh");
        command
            .args([
                "-c",
                "echo ${GH_ENTERPRISE_TOKEN:-unset}:${GH_TOKEN:-unset}",
            ])
            .env("GH_ENTERPRISE_TOKEN", "метка-ent")
            .env("GH_TOKEN", "метка-gh");
        let output = without_env_tokens(&mut command).output().unwrap();
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "unset:unset"
        );
    }
}

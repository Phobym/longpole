//! Конфигурация инстанса (спека, § 4): YAML по фиксированному пути, `${ИМЯ}` из окружения,
//! статическая проверка со всеми ошибками сразу.

use std::collections::HashSet;
use std::fmt;
use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::sync::LazyLock;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use longpole_core::source::Provider;
use regex::{Captures, Regex};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_yaml_ng::Mapping;
use url::Url;

pub const PATH: &str = "/etc/longpole/config.yaml";

#[derive(Debug)]
pub struct Config {
    /// Публичный URL инстанса, только корень домена.
    pub url: Url,
    pub cookie_key: Secret<[u8; 32]>,
    pub listen: SocketAddr,
    pub cache_max_mb: u64,
    pub max_concurrent_builds: NonZeroUsize,
    pub build_timeout: Duration,
    pub log: Log,
    pub hosts: Vec<Host>,
}

#[derive(Debug)]
pub struct Log {
    pub level: LogLevel,
    pub format: LogFormat,
}

impl Default for Log {
    fn default() -> Self {
        Log {
            level: LogLevel::Info,
            format: LogFormat::Text,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Text,
    Json,
}

#[derive(Debug)]
pub struct Host {
    /// Ключ хоста: домен в нижнем регистре, порт допустим, схема всегда `https`.
    pub domain: String,
    pub provider: Provider,
    pub client_id: String,
    pub client_secret: Secret<String>,
}

/// Секрет: `Debug` и `Display` печатают `[redacted]` (спека, § 13).
pub struct Secret<T>(T);

impl<T> Secret<T> {
    pub fn expose(&self) -> &T {
        &self.0
    }
}

impl<T> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

impl<T> fmt::Display for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

impl Config {
    /// Ошибка — список строк «поле: что не так», по строке на каждую найденную ошибку.
    pub fn parse(text: &str, env: impl Fn(&str) -> Option<String>) -> Result<Config, Vec<String>> {
        let mut errors = Vec::new();
        let text = substitute(text, env, &mut errors);
        let root = match serde_yaml_ng::from_str::<Mapping>(&text) {
            Ok(root) => root,
            Err(e) => {
                errors.push(e.to_string());
                return Err(errors);
            }
        };
        let mut f = Fields::new(root, String::new(), &mut errors);
        let url = f.required("url", check_url);
        let cookie_key = f.required("cookie_key", check_cookie_key);
        let listen = f.field("listen", Ok).unwrap_or(([0, 0, 0, 0], 8080).into());
        let cache_max_mb = f.field("cache_max_mb", Ok).unwrap_or(256);
        let max_concurrent_builds = f
            .field("max_concurrent_builds", Ok)
            .unwrap_or(NonZeroUsize::new(8).expect("не ноль"));
        let build_timeout = f
            .field("build_timeout", check_duration)
            .unwrap_or(Duration::from_secs(300));
        let log = f
            .field("log", Ok)
            .map(|map| parse_log(f.nested(map, "log.")));
        let hosts = f.required("hosts", |hosts: Vec<Mapping>| match hosts.is_empty() {
            true => Err("нужен хотя бы один хост".into()),
            false => Ok(hosts),
        });
        let hosts = hosts.map(|hosts| parse_hosts(&mut f, hosts));
        f.finish();

        match (url, cookie_key, hosts) {
            (Some(url), Some(cookie_key), Some(hosts)) if errors.is_empty() => Ok(Config {
                url,
                cookie_key,
                listen,
                cache_max_mb,
                max_concurrent_builds,
                build_timeout,
                log: log.unwrap_or_default(),
                hosts,
            }),
            _ => Err(errors),
        }
    }
}

static VAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{([^}]*)\}").expect("верный шаблон"));

/// `${ИМЯ}` во всём тексте; незаданная или пустая переменная — ошибка. `$$` и `${X:-y}` не поддерживаются.
// ponytail: подстановка в текст до разбора YAML — значение с `: ` или `#` сломает разметку;
// секреты в base64 и hex этим не грешат. Если начнёт мешать — подставлять в строки уже разобранного YAML.
fn substitute(
    text: &str,
    env: impl Fn(&str) -> Option<String>,
    errors: &mut Vec<String>,
) -> String {
    VAR.replace_all(text, |c: &Captures| {
        env(&c[1]).filter(|v| !v.is_empty()).unwrap_or_else(|| {
            errors.push(format!(
                "{}: переменная окружения не задана или пуста",
                &c[0]
            ));
            // остаётся как есть: пустое значение дало бы вторую, сбивающую с толку ошибку разбора
            c[0].to_owned()
        })
    })
    .into_owned()
}

fn parse_log(mut f: Fields) -> Log {
    let default = Log::default();
    let level = f.field("level", Ok).unwrap_or(default.level);
    let format = f.field("format", Ok).unwrap_or(default.format);
    f.finish();
    Log { level, format }
}

fn parse_hosts(f: &mut Fields, hosts: Vec<Mapping>) -> Vec<Host> {
    let mut seen = HashSet::new();
    let mut parsed = Vec::new();
    for (i, map) in hosts.into_iter().enumerate() {
        let mut h = f.nested(map, &format!("hosts[{i}]."));
        let domain = h.required("domain", check_domain);
        if let Some(domain) = &domain
            && !seen.insert(domain.clone())
        {
            h.error(
                "domain",
                format!("домен {domain} уже указан у другого хоста"),
            );
        }
        let provider = h.required("provider", Ok);
        let client_id = h.required("client_id", Ok);
        let client_secret = h.required("client_secret", |s: String| Ok(Secret(s)));
        h.finish();
        if let (Some(domain), Some(provider), Some(client_id), Some(client_secret)) =
            (domain, provider, client_id, client_secret)
        {
            parsed.push(Host {
                domain,
                provider,
                client_id,
                client_secret,
            });
        }
    }
    parsed
}

fn check_url(s: String) -> Result<Url, String> {
    let url = Url::parse(&s).map_err(|e| format!("{s}: не URL ({e})"))?;
    let localhost = url.scheme() == "http" && url.host_str() == Some("localhost");
    if url.scheme() != "https" && !localhost {
        return Err(format!("{s}: нужен https (http — только для localhost)"));
    }
    if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
        return Err(format!("{s}: только корень домена, без пути и query"));
    }
    Ok(url)
}

fn check_cookie_key(s: String) -> Result<Secret<[u8; 32]>, String> {
    STANDARD
        .decode(s.trim())
        .ok()
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .map(Secret)
        .ok_or_else(|| "нужно 32 байта в base64 (openssl rand -base64 32)".into())
}

/// `30s`, `5m`, `1h`.
fn check_duration(s: String) -> Result<Duration, String> {
    let (n, unit) = s.split_at(s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len()));
    let unit = match unit {
        "s" => 1,
        "m" => 60,
        "h" => 3600,
        _ => 0,
    };
    n.parse::<u64>()
        .ok()
        .and_then(|n| n.checked_mul(unit))
        .filter(|&secs| secs > 0)
        .map(Duration::from_secs)
        .ok_or_else(|| format!("{s}: нужно положительное число с единицей s, m или h"))
}

/// Домен с необязательным портом; возвращается в нижнем регистре.
fn check_domain(s: String) -> Result<String, String> {
    let url = Url::parse(&format!("https://{s}")).ok();
    let normalized = url
        .as_ref()
        .filter(|url| url.path() == "/")
        .and_then(|url| {
            let host = url.host_str()?;
            Some(
                url.port()
                    .map_or(host.to_owned(), |port| format!("{host}:{port}")),
            )
        });
    match normalized {
        Some(domain) if domain == s.to_ascii_lowercase() => Ok(domain),
        _ => Err(format!("{s}: нужен домен без схемы и пути, порт допустим")),
    }
}

/// Поля одного уровня YAML. Каждое разбирается отдельно и копит ошибки; оставшиеся ключи — неизвестные поля.
struct Fields<'a> {
    map: Mapping,
    path: String,
    errors: &'a mut Vec<String>,
}

impl<'a> Fields<'a> {
    fn new(map: Mapping, path: String, errors: &'a mut Vec<String>) -> Self {
        Fields { map, path, errors }
    }

    fn nested(&mut self, map: Mapping, prefix: &str) -> Fields<'_> {
        Fields::new(map, format!("{}{prefix}", self.path), self.errors)
    }

    /// Необязательное поле: `None`, если его нет или в нём ошибка.
    fn field<T: DeserializeOwned, U>(
        &mut self,
        key: &str,
        check: impl FnOnce(T) -> Result<U, String>,
    ) -> Option<U> {
        let value = self.map.remove(key)?;
        match serde_yaml_ng::from_value(value)
            .map_err(|e| e.to_string())
            .and_then(check)
        {
            Ok(value) => Some(value),
            Err(e) => {
                self.error(key, e);
                None
            }
        }
    }

    fn required<T: DeserializeOwned, U>(
        &mut self,
        key: &str,
        check: impl FnOnce(T) -> Result<U, String>,
    ) -> Option<U> {
        if !self.map.contains_key(key) {
            self.error(key, "обязательное поле");
        }
        self.field(key, check)
    }

    fn error(&mut self, key: &str, message: impl fmt::Display) {
        self.errors.push(format!("{}{key}: {message}", self.path));
    }

    fn finish(self) {
        for key in self.map.keys() {
            let key = key
                .as_str()
                .map_or_else(|| format!("{key:?}"), str::to_owned);
            self.errors
                .push(format!("{}{key}: неизвестное поле", self.path));
        }
    }
}

use std::time::Duration;

use longpole_core::source::Provider;
use longpole_server::config::{Config, LogFormat, LogLevel};

const KEY: &str = "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY="; // 32 байта

fn env(name: &str) -> Option<String> {
    match name {
        "COOKIE_KEY" => Some(KEY.into()),
        "SECRET" => Some("s3cret".into()),
        "EMPTY" => Some(String::new()),
        _ => None,
    }
}

fn parse(yaml: &str) -> Result<Config, Vec<String>> {
    Config::parse(yaml, env)
}

const MINIMAL: &str = "
url: https://longpole.example.com
cookie_key: ${COOKIE_KEY}
hosts:
  - domain: gitlab.example.com
    provider: gitlab
    client_id: id
    client_secret: ${SECRET}
";

#[test]
fn substitutes_env_and_fills_defaults() {
    let config = parse(MINIMAL).unwrap();
    assert_eq!(config.url.as_str(), "https://longpole.example.com/");
    assert_eq!(
        config.cookie_key.expose(),
        b"0123456789abcdef0123456789abcdef"
    );
    assert_eq!(config.listen.to_string(), "0.0.0.0:8080");
    assert_eq!(config.cache_max_mb, 256);
    assert_eq!(config.max_concurrent_builds.get(), 8);
    assert_eq!(config.build_timeout, Duration::from_secs(300));
    let host = &config.hosts[0];
    assert_eq!(host.domain, "gitlab.example.com");
    assert_eq!(host.provider, Provider::Gitlab);
    assert_eq!(host.client_secret.expose(), "s3cret");
}

fn errors(yaml: &str) -> Vec<String> {
    parse(yaml).unwrap_err()
}

#[test]
fn unset_or_empty_variable_is_an_error() {
    let errors = errors(&MINIMAL.replace("${SECRET}", "${MISSING}${EMPTY}"));
    assert_eq!(
        errors,
        [
            "${MISSING}: переменная окружения не задана или пуста",
            "${EMPTY}: переменная окружения не задана или пуста",
        ]
    );
}

#[test]
fn reports_all_errors_at_once() {
    let errors = errors(
        "
url: http://longpole.example.com
cookie_key: c2hvcnQ=
build_timeout: 5 minutes
colour: red
log:
  level: loud
hosts:
  - domain: https://gitlab.example.com
    provider: gitlab
    client_id: id
    client_secret: x
    scope: read_api
",
    );
    assert_eq!(
        errors,
        [
            "url: http://longpole.example.com: нужен https (http — только для localhost)",
            "cookie_key: нужно 32 байта в base64 (openssl rand -base64 32)",
            "build_timeout: 5 minutes: нужно положительное число с единицей s, m или h",
            "log.level: unknown variant `loud`, expected one of `trace`, `debug`, `info`, `warn`, `error`",
            "hosts[0].domain: https://gitlab.example.com: нужен домен без схемы и пути, порт допустим",
            "hosts[0].scope: неизвестное поле",
            "colour: неизвестное поле",
        ]
    );
}

#[test]
fn requires_url_cookie_key_and_hosts() {
    assert_eq!(
        errors("listen: 0.0.0.0:9000"),
        [
            "url: обязательное поле",
            "cookie_key: обязательное поле",
            "hosts: обязательное поле"
        ]
    );
    assert_eq!(
        errors(&format!(
            "url: https://l.example.com\ncookie_key: {KEY}\nhosts: []"
        )),
        ["hosts: нужен хотя бы один хост"]
    );
}

#[test]
fn rejects_duplicate_domains_case_insensitively() {
    let second = "
  - domain: GitLab.Example.com
    provider: github
    client_id: id2
    client_secret: x
";
    assert_eq!(
        errors(&format!("{MINIMAL}{second}")),
        ["hosts[1].domain: домен gitlab.example.com уже указан у другого хоста"]
    );
}

#[test]
fn domain_may_have_port() {
    let config = parse(&MINIMAL.replace("gitlab.example.com", "gitlab.example.com:8443")).unwrap();
    assert_eq!(config.hosts[0].domain, "gitlab.example.com:8443");
}

#[test]
fn url_is_https_root_or_http_localhost() {
    let with_url = |url: &str| parse(&MINIMAL.replace("https://longpole.example.com", url));
    assert!(with_url("http://localhost:8080").is_ok());
    assert_eq!(
        with_url("http://127.0.0.1").unwrap_err(),
        ["url: http://127.0.0.1: нужен https (http — только для localhost)"]
    );
    assert_eq!(
        with_url("https://example.com/longpole/").unwrap_err(),
        ["url: https://example.com/longpole/: только корень домена, без пути и query"]
    );
}

#[test]
fn optional_fields_override_defaults() {
    let config = parse(&format!(
        "{MINIMAL}listen: 127.0.0.1:9000\ncache_max_mb: 64\nmax_concurrent_builds: 2\nbuild_timeout: 90s\nlog:\n  level: debug\n  format: json\n"
    ))
    .unwrap();
    assert_eq!(config.listen.to_string(), "127.0.0.1:9000");
    assert_eq!(config.cache_max_mb, 64);
    assert_eq!(config.max_concurrent_builds.get(), 2);
    assert_eq!(config.build_timeout, Duration::from_secs(90));
    assert_eq!(config.log.level, LogLevel::Debug);
    assert_eq!(config.log.format, LogFormat::Json);
}

#[test]
fn secrets_print_as_redacted() {
    let config = parse(MINIMAL).unwrap();
    let debug = format!("{config:?}");
    assert!(debug.contains("cookie_key: [redacted]"), "{debug}");
    assert!(debug.contains("client_secret: [redacted]"), "{debug}");
    assert!(!debug.contains("s3cret"), "{debug}");
    assert_eq!(config.hosts[0].client_secret.to_string(), "[redacted]");
}

#[test]
fn rejects_bad_cookie_key_and_build_timeout() {
    let yaml = |key: &str, timeout: &str| {
        MINIMAL.replace("${COOKIE_KEY}", key) + &format!("build_timeout: {timeout}\n")
    };
    assert_eq!(
        errors(&yaml("not base64!", "99999999999999999999h")),
        [
            "cookie_key: нужно 32 байта в base64 (openssl rand -base64 32)",
            "build_timeout: 99999999999999999999h: нужно положительное число с единицей s, m или h",
        ]
    );
    assert_eq!(
        errors(&yaml(KEY, "5124095576030432h")), // * 3600 переполняет u64
        ["build_timeout: 5124095576030432h: нужно положительное число с единицей s, m или h"]
    );
    assert_eq!(
        errors(&yaml(KEY, "0m")),
        ["build_timeout: 0m: нужно положительное число с единицей s, m или h"]
    );
}

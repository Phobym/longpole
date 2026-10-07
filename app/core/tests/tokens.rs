//! Токены и хосты: `desktop/tokens.test` на мок-сторе `keyring-core` и поиск токена из `gitlab.test`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use keyring_core::api::CredentialStoreApi;
use keyring_core::mock::{Cred, Store};
use keyring_core::{CredentialStore, Entry, Error as KeyringError};
use pipeline_trace_core::error::{Error, ErrorCode};
use pipeline_trace_core::hosts::normalize_host;
use pipeline_trace_core::source::Provider;
use pipeline_trace_core::tokens::{
    Gh, Glab, HostInfo, SERVICE, TokenSource, TokenStore, find_github_token, find_token,
    github_hosts, list_hosts,
};
use tempfile::TempDir;

struct Fixture {
    dir: TempDir,
    mock: Arc<Store>,
    tokens: TokenStore,
}

fn fixture() -> Fixture {
    let dir = TempDir::new().expect("tempdir");
    let mock = Store::new().expect("мок-стор");
    let store: Arc<CredentialStore> = mock.clone();
    let tokens = TokenStore::new(dir.path().join("hosts.json"), Some(store));
    Fixture { dir, mock, tokens }
}

impl Fixture {
    fn entry(&self, host: &str) -> Entry {
        self.mock.build(SERVICE, host, None).expect("запись")
    }

    /// Следующая операция с записью хоста вернёт `error`.
    fn fail_next(&self, host: &str, error: KeyringError) {
        let entry = self.entry(host);
        let cred = entry.as_any().downcast_ref::<Cred>().expect("мок-запись");
        cred.inner.lock().unwrap().borrow_mut().error = Some(error);
    }

    fn hosts_file(&self) -> PathBuf {
        self.dir.path().join("hosts.json")
    }
}

fn platform_error() -> Box<dyn std::error::Error + Send + Sync> {
    "отказ платформы".into()
}

fn code(error: Error) -> ErrorCode {
    error.code
}

#[test]
fn set_get_hosts_remove_а_в_файле_хостов_нет_токена() {
    let f = fixture();
    assert_eq!(f.tokens.hosts().unwrap(), Vec::<String>::new());
    f.tokens.set("h.example", "secret-1").unwrap();
    assert_eq!(
        f.tokens.get("h.example").unwrap().as_deref(),
        Some("secret-1")
    );
    assert_eq!(f.tokens.hosts().unwrap(), ["h.example"]);
    let file = std::fs::read_to_string(f.hosts_file()).unwrap();
    assert!(!file.contains("secret-1"));

    f.tokens.remove("h.example").unwrap();
    assert_eq!(f.tokens.get("h.example").unwrap(), None);
    assert_eq!(f.tokens.hosts().unwrap(), Vec::<String>::new());
}

#[test]
fn запись_лежит_в_связке_ключей_под_service_и_хостом() {
    let f = fixture();
    f.tokens.set("h.example", "secret-1").unwrap();
    assert_eq!(SERVICE, "dev.pipeline-trace.desktop");
    assert_eq!(f.entry("h.example").get_password().unwrap(), "secret-1");
}

#[test]
fn хосты_по_алфавиту_без_повторов() {
    let f = fixture();
    for host in ["b.example", "a.example", "b.example"] {
        f.tokens.set(host, "t").unwrap();
    }
    assert_eq!(f.tokens.hosts().unwrap(), ["a.example", "b.example"]);
}

#[test]
fn хост_нормализуется_при_сохранении() {
    let f = fixture();
    f.tokens.set("https://gitlab.example.com/", "t").unwrap();
    assert_eq!(f.tokens.hosts().unwrap(), ["gitlab.example.com"]);
    assert_eq!(
        f.tokens.get("gitlab.example.com").unwrap().as_deref(),
        Some("t")
    );

    let err = f.tokens.set("bad host!", "t").unwrap_err();
    assert_eq!(code(err), ErrorCode::InvalidHost);
}

#[test]
fn хост_нормализуется_и_при_удалении() {
    let f = fixture();
    f.tokens.set("gitlab.example.com", "t").unwrap();
    f.tokens.remove("https://gitlab.example.com/").unwrap();
    assert_eq!(f.tokens.get("gitlab.example.com").unwrap(), None);
    assert_eq!(f.tokens.hosts().unwrap(), Vec::<String>::new());
}

#[test]
fn normalize_host_схема_слэш_и_порт() {
    assert_eq!(
        normalize_host(" http://h.example:8080/ ").unwrap(),
        "h.example:8080"
    );
    assert_eq!(normalize_host("h.example").unwrap(), "h.example");
    for bad in ["", "--evil", "https://", "h/path"] {
        assert_eq!(
            code(normalize_host(bad).unwrap_err()),
            ErrorCode::InvalidHost,
            "{bad:?}"
        );
    }
}

#[test]
fn пустой_токен_не_сохраняется_а_лишние_пробелы_обрезаются() {
    let f = fixture();
    assert_eq!(
        code(f.tokens.set("h.example", "   ").unwrap_err()),
        ErrorCode::EmptyToken
    );
    assert_eq!(f.tokens.hosts().unwrap(), Vec::<String>::new());

    f.tokens.set("h.example", "  t  ").unwrap();
    assert_eq!(f.tokens.get("h.example").unwrap().as_deref(), Some("t"));
}

#[test]
fn без_хранилища_запись_запрещена_а_чтение_даёт_none() {
    let dir = TempDir::new().unwrap();
    let tokens = TokenStore::new(dir.path().join("hosts.json"), None);
    assert_eq!(
        code(tokens.set("h.example", "x").unwrap_err()),
        ErrorCode::KeychainUnavailable
    );
    assert_eq!(tokens.get("h.example").unwrap(), None);
    assert_eq!(tokens.hosts().unwrap(), Vec::<String>::new());
}

#[test]
fn отказ_платформы_при_записи_это_keychain_unavailable_и_хост_не_добавляется() {
    for error in [
        KeyringError::NoStorageAccess(platform_error()),
        KeyringError::PlatformFailure(platform_error()),
    ] {
        let f = fixture();
        f.fail_next("h.example", error);
        assert_eq!(
            code(f.tokens.set("h.example", "x").unwrap_err()),
            ErrorCode::KeychainUnavailable
        );
        assert_eq!(f.tokens.hosts().unwrap(), Vec::<String>::new());
    }
}

#[test]
fn битый_токен_в_хранилище_считается_отсутствующим() {
    let f = fixture();
    f.entry("h.example").set_secret(&[0xff, 0xfe]).unwrap();
    assert_eq!(f.tokens.get("h.example").unwrap(), None);
}

#[test]
fn недоступное_хранилище_при_чтении_не_мешает_glab_и_env() {
    let f = fixture();
    f.fail_next("h.example", KeyringError::NoStorageAccess(platform_error()));
    assert_eq!(f.tokens.get("h.example").unwrap(), None);
}

#[test]
fn чужая_ошибка_хранилища_при_чтении_это_storage_с_detail() {
    let f = fixture();
    f.fail_next("h.example", KeyringError::TooLong("user".into(), 1));
    let err = f.tokens.get("h.example").unwrap_err();
    assert_eq!(err.code, ErrorCode::Storage);
    assert!(err.params.contains_key("detail"));
}

/// Хранилище, которое не может даже создать запись (нет Secret Service).
struct Broken;

impl std::fmt::Debug for Broken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Broken")
    }
}

impl CredentialStoreApi for Broken {
    fn vendor(&self) -> String {
        "broken".into()
    }

    fn id(&self) -> String {
        "broken".into()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn build(
        &self,
        _service: &str,
        _user: &str,
        _mods: Option<&HashMap<&str, &str>>,
    ) -> keyring_core::Result<Entry> {
        Err(KeyringError::PlatformFailure(platform_error()))
    }
}

#[test]
fn хранилище_без_записей_это_выключенное_хранение_а_не_storage() {
    let dir = TempDir::new().unwrap();
    let store: Arc<CredentialStore> = Arc::new(Broken);
    let tokens = TokenStore::new(dir.path().join("hosts.json"), Some(store));
    assert_eq!(tokens.get("h.example").unwrap(), None);
    assert_eq!(
        code(tokens.set("h.example", "x").unwrap_err()),
        ErrorCode::KeychainUnavailable
    );
    assert_eq!(
        code(tokens.remove("h.example").unwrap_err()),
        ErrorCode::KeychainUnavailable
    );
}

#[test]
fn remove_без_токена_не_ошибка() {
    let f = fixture();
    f.tokens.remove("h.example").unwrap();
}

// --- поиск токена: хранилище → glab → env ---

type Env = HashMap<&'static str, &'static str>;

fn env_of(pairs: &[(&'static str, &'static str)]) -> Env {
    pairs.iter().copied().collect()
}

/// `glab config get host` и `glab config get token --host H`.
fn glab_of(
    host: Option<&'static str>,
    token: Option<&'static str>,
) -> impl Fn(&[&str]) -> Option<String> {
    move |args| match args {
        ["config", "get", "host"] => host.map(Into::into),
        ["config", "get", "token", "--host", _] => token.map(Into::into),
        other => panic!("неожиданные аргументы glab: {other:?}"),
    }
}

fn find(f: &Fixture, env: &Env, glab: impl Fn(&[&str]) -> Option<String>) -> Result<String, Error> {
    find_token(
        "h",
        &f.tokens,
        &|k| env.get(k).map(|v| v.to_string()),
        &glab,
    )
}

#[test]
fn токен_из_хранилища_главнее_glab_и_env() {
    let f = fixture();
    f.tokens.set("h", "from-store").unwrap();
    let env = env_of(&[("GITLAB_TOKEN", "from-env"), ("GITLAB_HOST", "h")]);
    assert_eq!(
        find(&f, &env, glab_of(Some("h"), Some("from-glab"))).unwrap(),
        "from-store"
    );
}

#[test]
fn без_хранилища_токен_из_glab_главнее_env() {
    let f = fixture();
    let env = env_of(&[("GITLAB_TOKEN", "from-env"), ("GITLAB_HOST", "h")]);
    assert_eq!(
        find(&f, &env, glab_of(None, Some("from-glab"))).unwrap(),
        "from-glab"
    );
}

#[test]
fn env_работает_для_gitlab_host_с_любой_записью_хоста() {
    let f = fixture();
    for host in ["h", "https://h/", "http://h"] {
        let env = env_of(&[("GITLAB_TOKEN", "from-env"), ("GITLAB_HOST", host)]);
        assert_eq!(
            find(&f, &env, glab_of(None, None)).unwrap(),
            "from-env",
            "{host}"
        );
    }
}

#[test]
fn env_работает_для_хоста_glab_по_умолчанию_без_gitlab_host() {
    let f = fixture();
    let env = env_of(&[("GITLAB_TOKEN", "from-env")]);
    assert_eq!(
        find(&f, &env, glab_of(Some("h"), None)).unwrap(),
        "from-env"
    );
}

#[test]
fn env_не_подходит_к_чужому_хосту_и_пустому_токену() {
    let f = fixture();
    let other = env_of(&[("GITLAB_TOKEN", "from-env"), ("GITLAB_HOST", "other")]);
    let empty = env_of(&[("GITLAB_TOKEN", ""), ("GITLAB_HOST", "h")]);
    for env in [other, empty] {
        let err = find(&f, &env, glab_of(None, None)).unwrap_err();
        assert_eq!(
            (err.code, err.params),
            (ErrorCode::NoToken, [("host", "h".to_string())].into())
        );
    }
}

#[test]
fn ошибка_хранилища_не_маскируется_отсутствием_токена() {
    let f = fixture();
    f.fail_next("h", KeyringError::TooLong("user".into(), 1));
    let err = find(&f, &Env::new(), glab_of(None, Some("from-glab"))).unwrap_err();
    assert_eq!(err.code, ErrorCode::Storage);
}

#[test]
fn list_hosts_связка_потом_glab_потом_env_без_дублей() {
    let f = fixture();
    f.tokens.set("b.example", "t").unwrap();
    f.tokens.set("a.example", "t").unwrap();
    let env = |name: &str| match name {
        "GITLAB_TOKEN" => Some("env-token".to_string()),
        "GITLAB_HOST" => Some("https://env.example/".to_string()),
        _ => None,
    };
    let glab = |args: &[&str]| match args {
        ["config", "get", "host"] => Some("glab.example".to_string()),
        ["config", "get", "token", "--host", "glab.example"] => Some("glab-token".to_string()),
        _ => None,
    };
    let info = |host: &str, source: TokenSource| HostInfo {
        host: host.into(),
        source,
        provider: (source != TokenSource::Keychain).then_some(Provider::Gitlab),
    };
    assert_eq!(
        list_hosts(&f.tokens, &env, &glab).unwrap(),
        vec![
            info("a.example", TokenSource::Keychain),
            info("b.example", TokenSource::Keychain),
            info("glab.example", TokenSource::Glab),
            info("env.example", TokenSource::Env),
        ]
    );

    // хост glab уже в связке — не дублируется; без GITLAB_TOKEN env-хоста нет
    f.tokens.set("glab.example", "t").unwrap();
    let no_env = |_: &str| None;
    let hosts: Vec<String> = list_hosts(&f.tokens, &no_env, &glab)
        .unwrap()
        .into_iter()
        .map(|h| h.host)
        .collect();
    assert_eq!(hosts, ["a.example", "b.example", "glab.example"]);

    // токен есть, а GITLAB_HOST пуст — пустого хоста в списке нет
    let empty_host = |name: &str| match name {
        "GITLAB_TOKEN" => Some("env-token".to_string()),
        "GITLAB_HOST" => Some(String::new()),
        _ => None,
    };
    let hosts: Vec<String> = list_hosts(&f.tokens, &empty_host, &|_: &[&str]| None)
        .unwrap()
        .into_iter()
        .map(|h| h.host)
        .collect();
    assert_eq!(hosts, ["a.example", "b.example", "glab.example"]);
}

// --- glab ---

#[cfg(unix)]
fn fake_glab(dir: &Path, script: &str) -> Glab {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("glab");
    std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    Glab::find_in(&[dir.into()]).expect("glab")
}

#[test]
fn glab_ищется_в_path_затем_в_фиксированных_каталогах() {
    let path_var = std::env::join_paths(["/a", "/b"]).unwrap();
    let dirs = Glab::search_dirs(Some(&path_var), Some(Path::new("/home/u")));
    let expected: Vec<PathBuf> = [
        "/a",
        "/b",
        "/opt/homebrew/bin",
        "/usr/local/bin",
        "/home/linuxbrew/.linuxbrew/bin",
        "/home/u/.local/bin",
    ]
    .map(PathBuf::from)
    .into();
    assert_eq!(dirs, expected);
    assert_eq!(Glab::search_dirs(None, None).len(), 3);
}

#[test]
fn относительные_каталоги_path_пропускаются() {
    let path_var = std::env::join_paths(["", ".", "bin", "/abs"]).unwrap();
    let dirs = Glab::search_dirs(Some(&path_var), None);
    assert_eq!(dirs[0], PathBuf::from("/abs"));
    assert!(dirs.iter().all(|d| d.is_absolute()));
}

#[test]
fn find_in_берёт_первый_каталог_где_есть_glab() {
    let (first, second) = (TempDir::new().unwrap(), TempDir::new().unwrap());
    let name = format!("glab{}", std::env::consts::EXE_SUFFIX);
    assert!(Glab::find_in(&[first.path().into(), second.path().into()]).is_none());
    std::fs::write(second.path().join(&name), "").unwrap();
    std::fs::create_dir(first.path().join(&name)).unwrap();
    let found = Glab::find_in(&[first.path().into(), second.path().into()]).expect("glab");
    assert_eq!(found.path(), second.path().join(&name));
}

#[cfg(unix)]
#[test]
fn glab_value_это_обрезанный_stdout_а_сбой_и_пустота_это_none() {
    let dir = TempDir::new().unwrap();
    let glab = fake_glab(dir.path(), "echo \"  tok-$2  \"");
    assert_eq!(glab.value(&["config", "get"]).as_deref(), Some("tok-get"));

    let empty = TempDir::new().unwrap();
    assert_eq!(fake_glab(empty.path(), "echo").value(&["x"]), None);

    let failing = TempDir::new().unwrap();
    assert_eq!(
        fake_glab(failing.path(), "echo oops; exit 1").value(&["x"]),
        None
    );
}

fn no_cli(_: &[&str]) -> Option<String> {
    None
}

#[test]
fn github_токен_хранилище_затем_gh_затем_env() {
    let f = fixture();
    let env = |name: &str| match name {
        "GH_TOKEN" => Some("gh-env".to_string()),
        "GITHUB_TOKEN" => Some("github-env".to_string()),
        _ => None,
    };
    let gh = |args: &[&str]| {
        (args == ["auth", "token", "--hostname", "github.com"]).then(|| "gh-cli".to_string())
    };
    assert_eq!(
        find_github_token("github.com", &f.tokens, &env, &gh).unwrap(),
        "gh-cli"
    );
    assert_eq!(
        find_github_token("github.com", &f.tokens, &env, &no_cli).unwrap(),
        "gh-env"
    );
    f.tokens.set("github.com", "stored").unwrap();
    assert_eq!(
        find_github_token("github.com", &f.tokens, &env, &gh).unwrap(),
        "stored"
    );
}

#[test]
fn ghes_токен_из_enterprise_переменных_только_для_gh_host() {
    let f = fixture();
    let env = |name: &str| match name {
        "GH_HOST" => Some("https://ghe.example/".to_string()),
        "GH_ENTERPRISE_TOKEN" => Some("ent".to_string()),
        "GH_TOKEN" => Some("public".to_string()),
        _ => None,
    };
    assert_eq!(
        find_github_token("ghe.example", &f.tokens, &env, &no_cli).unwrap(),
        "ent"
    );
    let err = find_github_token("other.example", &f.tokens, &env, &no_cli).unwrap_err();
    assert_eq!(err.code, ErrorCode::NoToken);
}

#[test]
fn хосты_github_из_gh_и_окружения() {
    let status = r#"{"hosts":{"github.com":[{"state":"success"}],"ghe.example":[{"state":"success"}],"dead.example":[{"state":"error"}]}}"#;
    let gh =
        |args: &[&str]| (args == ["auth", "status", "--json", "hosts"]).then(|| status.to_string());
    let info = |host: &str, source| HostInfo {
        host: host.into(),
        source,
        provider: Some(Provider::Github),
    };
    let no_env = |_: &str| None;
    assert_eq!(
        github_hosts(&no_env, &gh),
        vec![
            info("dead.example", TokenSource::Gh),
            info("ghe.example", TokenSource::Gh),
            info("github.com", TokenSource::Gh)
        ]
    );
    let env = |name: &str| (name == "GITHUB_TOKEN").then(|| "t".to_string());
    assert_eq!(
        github_hosts(&env, &no_cli),
        vec![info("github.com", TokenSource::Env)]
    );
}

#[cfg(unix)]
fn fake_gh(script: &str) -> (TempDir, Gh) {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("gh");
    std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    let gh = Gh::find_in(&[dir.path().into()]).expect("gh");
    (dir, gh)
}

// токены окружения `gh` не получает: их разбирает наш fallback со своей привязкой к хосту
// (слабая проверка: в окружении тестов этих переменных может не быть)
#[cfg(unix)]
#[test]
fn gh_запускается_без_токенов_окружения() {
    let (_dir, gh) = fake_gh("exec env");
    let out = gh.value(&[]).expect("env");
    for name in [
        "GH_TOKEN",
        "GITHUB_TOKEN",
        "GH_ENTERPRISE_TOKEN",
        "GITHUB_ENTERPRISE_TOKEN",
    ] {
        assert!(!out.contains(name), "{name}");
    }
}

#[cfg(unix)]
#[test]
fn зависший_gh_обрывается_по_таймауту() {
    let (_dir, gh) = fake_gh("exec sleep 30");
    let started = std::time::Instant::now();
    assert_eq!(gh.value(&["auth", "status"]), None);
    assert!(started.elapsed() < std::time::Duration::from_secs(15));
}

//! Разбор сырой формы и ссылки в запрос на сборку отчёта.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Error, ErrorCode, Field};
use crate::hosts::is_valid_host;
use crate::source::{GITHUB_COM, Provider};

const MAX_LAST: u32 = 500;

static LINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^https?://([^/]+)/(.+?)/-/(pipelines|merge_requests)/([0-9]+)(?:[/?#]|$)")
        .expect("верный шаблон")
});
/// `https://host/owner/repo/actions/runs/<id>[/job/…|/attempts/…]` или `…/pull/<n>[/…]`.
static GITHUB_LINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^https?://([^/]+)/([^/]+/[^/]+)/(actions/runs|pull)/([0-9]+)(?:[/?#]|$)")
        .expect("верный шаблон")
});
/// Третий сегмент пути на github: страница репозитория, а не часть пути проекта.
const GITHUB_PAGES: [&str; 6] = ["actions", "pull", "tree", "blob", "commit", "issues"];
/// Сегмент из одних точек (`..`) — не путь проекта; `regex` без lookahead, поэтому проверяем отдельно.
static SEGMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9_.-]+$").expect("верный шаблон"));
/// `https://[user[:token]@]host/group/project[.git][/]`; хвост `/-/…` (страницы проекта) отрезает `parse_remote`.
static REPO_HTTP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^https?://(?:[^@/]+@)?([^/]+)/(.+?)(?:\.git)?/?$").expect("верный шаблон")
});
/// `ssh://[user@]host[:port]/group/project[.git]`; порт — ssh, в хост GitLab не входит.
static REPO_SSH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^ssh://(?:[^@/]+@)?([^/:]+)(?::[0-9]+)?/(.+?)(?:\.git)?/?$")
        .expect("верный шаблон")
});
/// `[user@]host:group/project[.git]` — scp-форма git.
static REPO_SCP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:[^@/:]+@)?([A-Za-z0-9.-]+):([^/].*?)(?:\.git)?/?$").expect("верный шаблон")
});

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum FormMode {
    Link,
    #[default]
    Aggregate,
}

/// Форма как её заполнил пользователь: ничего не проверено, числа — текстом из поля.
/// Неизвестные поля (токен и прочее) при разборе отбрасываются.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct Form {
    pub mode: FormMode,
    pub host: String,
    pub url: String,
    pub project: String,
    pub r#ref: String,
    pub source: String,
    pub last: String,
    /// `ANY` — без фильтра
    pub statuses: Vec<String>,
    /// файл workflow GitHub (`ci.yml`); GitLab поле не читает
    pub workflow: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export)]
pub enum Status {
    Success,
    Manual,
    Failed,
    Canceled,
    Running,
}

impl Status {
    const ALL: [Status; 5] = [
        Status::Success,
        Status::Manual,
        Status::Failed,
        Status::Canceled,
        Status::Running,
    ];

    /// Как GitLab пишет статус в GraphQL.
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Success => "SUCCESS",
            Status::Manual => "MANUAL",
            Status::Failed => "FAILED",
            Status::Canceled => "CANCELED",
            Status::Running => "RUNNING",
        }
    }

    fn parse(text: &str) -> Option<Status> {
        Self::ALL.into_iter().find(|s| s.as_str() == text)
    }
}

/// Что строить. Хост — отдельно: он не часть запроса к проекту.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "mode",
    rename_all = "lowercase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum Request {
    Pipeline {
        project: String,
        pipeline_id: String,
    },
    Mr {
        project: String,
        mr_iid: String,
    },
    Aggregate {
        project: String,
        r#ref: Option<String>,
        source: Option<String>,
        last: u32,
        /// `None` — любые статусы
        statuses: Option<Vec<Status>>,
        /// файл workflow GitHub; старые записи истории — `None`
        #[serde(default)]
        workflow: Option<String>,
    },
}

impl Request {
    pub fn project(&self) -> &str {
        match self {
            Request::Pipeline { project, .. }
            | Request::Mr { project, .. }
            | Request::Aggregate { project, .. } => project,
        }
    }
}

/// Проект на хосте: результат разбора ввода «Добавить проект» и ключ `projects.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectRef {
    pub host: String,
    pub path: String,
}

/// Ссылка на репозиторий, пайплайн или MR, ssh-URL или абсолютный путь к папке с клоном → хост и путь.
pub fn parse_project_input(input: &str) -> Result<ProjectRef, Error> {
    let input = input.trim();
    let dir = std::path::Path::new(input);
    if dir.is_absolute() {
        let url = remote_origin(dir)?;
        return parse_remote(url.trim()).map_err(|_| Error::new(ErrorCode::ProjectDirNoRemote));
    }
    parse_remote(input)
}

fn parse_remote(input: &str) -> Result<ProjectRef, Error> {
    let invalid = || Error::new(ErrorCode::ProjectInputInvalid);
    let caps = LINK
        .captures(input)
        .or_else(|| GITHUB_LINK.captures(input))
        .or_else(|| REPO_HTTP.captures(input))
        .or_else(|| REPO_SSH.captures(input))
        .or_else(|| REPO_SCP.captures(input))
        .ok_or_else(invalid)?;
    let host = match &caps[1] {
        // ssh через 443 у github.com живёт на отдельном имени
        "ssh.github.com" => GITHUB_COM.to_string(),
        other => other.to_string(),
    };
    if !is_valid_host(&host) {
        return Err(invalid());
    }
    let tail_cut = caps[2].split("/-/").next().unwrap_or_default();
    let path = project(cut_github_page(tail_cut)).map_err(|_| invalid())?;
    Ok(ProjectRef { host, path })
}

/// `o/r/tree/main/src` → `o/r`.
// ponytail: проект GitLab `group/sub/tree` тоже обрежется до `group/sub`; если встретится — резать только для хостов GitHub из `hostKinds`
fn cut_github_page(path: &str) -> &str {
    let mut parts = path.splitn(4, '/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(owner), Some(repo), Some(page)) if GITHUB_PAGES.contains(&page) => {
            &path[..owner.len() + 1 + repo.len()]
        }
        _ => path,
    }
}

// ponytail: наивный разбор `.git/config` — без `[include]`, `url.<base>.insteadOf` и значений в кавычках; если реальные конфиги ломаются, нужен настоящий парсер git-config.
/// `url` из `[remote "origin"]` в `.git/config`; worktree (`.git` — файл `gitdir:`) ведёт к общему `config`.
fn remote_origin(dir: &std::path::Path) -> Result<String, Error> {
    let no_remote = || Error::new(ErrorCode::ProjectDirNoRemote);
    let git = dir.join(".git");
    let git_dir = if git.is_file() {
        let text = std::fs::read_to_string(&git).map_err(|_| no_remote())?;
        let target = text
            .trim()
            .strip_prefix("gitdir:")
            .ok_or_else(no_remote)?
            .trim();
        let git_dir = dir.join(target);
        match std::fs::read_to_string(git_dir.join("commondir")) {
            Ok(common) => git_dir.join(common.trim()),
            Err(_) => git_dir,
        }
    } else {
        git
    };
    let config = std::fs::read_to_string(git_dir.join("config")).map_err(|_| no_remote())?;
    let mut in_origin = false;
    for line in config.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_origin = line == "[remote \"origin\"]";
            continue;
        }
        if !in_origin {
            continue;
        }
        if let Some(rest) = line.strip_prefix("url")
            && let Some(url) = rest.trim_start().strip_prefix('=')
        {
            return Ok(url.trim().to_string());
        }
    }
    Err(no_remote())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    pub host: String,
    pub request: Request,
}

pub type FieldErrors = BTreeMap<Field, Error>;

pub fn parse_form(form: &Form) -> Result<Parsed, FieldErrors> {
    match form.mode {
        FormMode::Link => parse_link(&form.url),
        FormMode::Aggregate => parse_aggregate(form),
    }
}

/// Тип хоста по форме ссылки на пайплайн, MR, run или PR.
pub fn link_provider(url: &str) -> Option<Provider> {
    let url = url.trim();
    if LINK.is_match(url) {
        Some(Provider::Gitlab)
    } else if GITHUB_LINK.is_match(url) {
        Some(Provider::Github)
    } else {
        None
    }
}

fn parse_link(url: &str) -> Result<Parsed, FieldErrors> {
    let invalid = || BTreeMap::from([(Field::Url, Error::new(ErrorCode::InvalidLink))]);
    let url = url.trim();
    let caps = LINK
        .captures(url)
        .or_else(|| GITHUB_LINK.captures(url))
        .ok_or_else(invalid)?;
    let host = caps[1].to_string();
    if !is_valid_host(&host) {
        return Err(invalid());
    }
    let project = project(&caps[2]).map_err(|_| invalid())?;
    let id = caps[4].to_string();
    let request = if matches!(&caps[3], "pipelines" | "actions/runs") {
        Request::Pipeline {
            project,
            pipeline_id: id,
        }
    } else {
        Request::Mr {
            project,
            mr_iid: id,
        }
    };
    Ok(Parsed { host, request })
}

fn parse_aggregate(form: &Form) -> Result<Parsed, FieldErrors> {
    let mut errors = FieldErrors::new();
    let host = keep(&mut errors, Field::Host, host(&form.host));
    let project = keep(&mut errors, Field::Project, project(&form.project));
    let last = keep(&mut errors, Field::Last, last(&form.last));
    let statuses = keep(&mut errors, Field::Statuses, statuses(&form.statuses));
    let (Some(host), Some(project), Some(last), Some(statuses)) = (host, project, last, statuses)
    else {
        return Err(errors);
    };
    Ok(Parsed {
        host,
        request: Request::Aggregate {
            project,
            r#ref: non_empty(&form.r#ref),
            source: non_empty(&form.source),
            last,
            statuses,
            workflow: non_empty(&form.workflow),
        },
    })
}

/// Значение поля или `None` с записью ошибки.
fn keep<T>(errors: &mut FieldErrors, field: Field, result: Result<T, Error>) -> Option<T> {
    result
        .map_err(|e| {
            errors.insert(field, e);
        })
        .ok()
}

fn non_empty(text: &str) -> Option<String> {
    Some(text.trim()).filter(|t| !t.is_empty()).map(Into::into)
}

fn host(text: &str) -> Result<String, Error> {
    let host = text.trim();
    if is_valid_host(host) {
        Ok(host.into())
    } else {
        Err(Error::new(ErrorCode::InvalidHost))
    }
}

fn project(text: &str) -> Result<String, Error> {
    let project = text.trim();
    let segments: Vec<&str> = project.split('/').collect();
    let valid = segments.len() > 1
        && segments
            .iter()
            .all(|s| SEGMENT.is_match(s) && !s.chars().all(|c| c == '.'));
    if valid {
        Ok(project.into())
    } else {
        Err(Error::new(ErrorCode::InvalidProject))
    }
}

fn last(text: &str) -> Result<u32, Error> {
    text.trim()
        .parse()
        .ok()
        .filter(|n| (1..=MAX_LAST).contains(n))
        .ok_or_else(|| Error::new(ErrorCode::InvalidLast).with("max", MAX_LAST))
}

fn statuses(texts: &[String]) -> Result<Option<Vec<Status>>, Error> {
    if texts.iter().any(|t| t == "ANY") {
        return Ok(None);
    }
    let parsed: Option<Vec<Status>> = texts.iter().map(|t| Status::parse(t)).collect();
    parsed
        .filter(|list| !list.is_empty())
        .map(Some)
        .ok_or_else(|| Error::new(ErrorCode::InvalidStatuses))
}

# Первый запуск: «Мои проекты», отчёт в окне, настройки — план реализации

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Новый пользователь добавляет проект по ссылке или папке, вводит токен по подсказке и видит отчёт в том же окне; тема, язык и токены живут в настройках.

**Architecture:** Ядро (`app/core`) получает разбор ввода проекта, файл `projects.json`, расширенные настройки и список хостов с источником токена. Оболочка Tauri перестаёт открывать окна отчётов: `build` кладёт отчёт в LRU-кеш и возвращает id, фронтенд рендерит `pages/report` на маршруте `/report/$id`. Фронтенд получает сайдбар «Мои проекты», диалог добавления, экран настроек и тему по классу.

**Tech Stack:** Rust 2024 (`pipeline-trace-core`, Tauri 2, `ts-rs` 12, `serde`, `regex`, `time`), React 19, TanStack Router + Query, Zustand, i18next, Tailwind v4, Radix (`radix-ui`), `@tauri-apps/plugin-dialog`.

Спека — `docs/specs/2026-10-06-first-run-ux-design.md` (далее «спека, § N»). Правила репозитория: `docs/agents/code-smells.md` (проход по диффу перед коммитом), комментарии и тексты — по-русски, имена тестов ядра — по-русски через `_`, как в соседних файлах.

## Global Constraints

- Команды Tauri перечисляются в трёх местах, и тест `списки_команд_совпадают` в `app/src-tauri/src/main.rs:98` требует совпадения: `COMMANDS` в `app/src-tauri/build.rs`, `generate_handler!` в `main.rs`, `allow-*` в `app/src-tauri/capabilities/form.json`. Каждая новая команда — во все три; итоговое число команд — 18.
- Токен не возвращает ни одна команда (спека Tauri, § 5).
- Все команды — `async`, первой строкой `ensure_form(&webview)?`.
- Ошибки — коды `ErrorCode` + `params`; текст только во фронтенде. Новый код в `ErrorCode` требует перевода в `ru.json` и `en.json`, иначе `tsc` падает (`app/src/shared/i18n/index.ts:42`).
- Сгенерированные ts-rs типы в `app/src/shared/api/schema/` коммитятся: после изменения типов с `#[ts(export)]` запускать `cargo test -p pipeline-trace-core` из `app/` и коммитить `schema/`.
- Хост хранится без схемы и завершающего `/` (`core::hosts::normalize_host`).
- Фронтенд — FSD, Steiger: `pages` не импортируют `pages`, `features` не импортируют `features` и `widgets`, `entities` не импортируют `features`. Слайс с одним потребителем — в исключения `app/steiger.config.ts`.
- Фронтенд без автотестов; критерий готовности задачи — `npm run typecheck && npm run lint:fsd && npm run check:locales` зелёные и ручная проверка в `npm run tauri dev`.
- Перед коммитом Rust: `cargo fmt --all`. Pre-push гоняет `cargo clippy --workspace --all-targets -- -D warnings`.
- Все команды npm и cargo запускаются из `app/`.
- Коммиты задач 1–6 (ядро и оболочка) делаются с `LEFTHOOK=0`: до задачи 7 `tsc` падает на новых кодах ошибок без перевода. С задачи 7 хуки обязательны.

## Задачи и зависимости

| № | Задача | Зависит от |
|---|---|---|
| 1 | Ядро: коды ошибок и `parse_project_input` | — |
| 2 | Ядро: `core::projects` (`projects.json`) | 1 |
| 3 | Ядро: настройки (тема, последний проект) и `browse::fetch_project` | 1 |
| 4 | Ядро: хосты с источником токена | — |
| 5 | Оболочка: отчёт в памяти по id, удаление окон отчётов | 1 |
| 6 | Оболочка: команды проектов, настроек, хостов; ts-rs типы | 2, 3, 4, 5 |
| 7 | Фронтенд: API, словари, тема по классу, настройки приложения | 6 |
| 8 | Фронтенд: раскладка, сайдбар «Мои проекты», маршруты | 7 |
| 9 | Фронтенд: диалог «Добавить проект» и блок токена | 8 |
| 10 | Фронтенд: отчёт в окне, повтор из «Недавних», ⌘S | 8 |
| 11 | Фронтенд: экран настроек | 8 |
| 12 | Документы, чек-лист приёмки, удаление мёртвого кода | 9, 10, 11 |

Задачи 1–4 независимы между собой (кроме 2 и 3 от типа `ProjectRef` из 1). Задачи 9, 10, 11 независимы.

---

### Task 1: Ядро — коды ошибок и `parse_project_input`

**Files:**
- Modify: `app/core/src/error.rs:10-47` (enum `ErrorCode`)
- Modify: `app/core/src/request.rs` (новые regex, `ProjectRef`, `parse_project_input`, `remote_origin`)
- Create: `app/core/tests/project_input.rs`
- Modify: `docs/specs/2026-10-06-first-run-ux-design.md` § 6 (таблица кодов)

**Interfaces:**
- Produces: `request::ProjectRef { host: String, path: String }` (`Serialize`, `Deserialize`, `TS`), `request::parse_project_input(input: &str) -> Result<ProjectRef, Error>`, `ErrorCode::{ProjectInputInvalid, ProjectDirNoRemote, ReportExpired}`.
- Отступление от спеки § 6: вместо новых `project_not_found`, `token_missing`, `token_rejected` используются существующие `ProjectNotFound` (`project`, `host`), `NoToken` (`host`), `Unauthorized` (`host`, `status`) — у них те же параметры и смысл. В спеке таблица правится на этом шаге.

- [ ] **Step 1: Добавить коды ошибок**

В `app/core/src/error.rs` после варианта `Window` (строка 46) добавить:

```rust
    /// поле ввода проекта: не ссылка на проект GitLab, не ssh-URL и не путь к папке
    ProjectInputInvalid,
    /// папка без `.git` или без `remote "origin"`
    ProjectDirNoRemote,
    /// отчёт вытеснен из памяти: `id`
    ReportExpired,
```

- [ ] **Step 2: Написать падающие тесты**

Создать `app/core/tests/project_input.rs`:

```rust
//! Разбор ввода диалога «Добавить проект»: ссылки, ssh-URL, папка с клоном.

use pipeline_trace_core::error::{Error, ErrorCode};
use pipeline_trace_core::request::{ProjectRef, parse_project_input};
use tempfile::TempDir;

fn r(host: &str, path: &str) -> Result<ProjectRef, Error> {
    Ok(ProjectRef {
        host: host.into(),
        path: path.into(),
    })
}

#[test]
fn https_ссылки_на_репозиторий_с_git_и_слэшем() {
    for input in [
        "https://gitlab.example.com/group/project",
        "https://gitlab.example.com/group/project/",
        "https://gitlab.example.com/group/project.git",
        "  http://gitlab.example.com/group/project.git/ ",
    ] {
        assert_eq!(parse_project_input(input), r("gitlab.example.com", "group/project"), "{input}");
    }
    assert_eq!(parse_project_input("https://h.example/g/sub/p"), r("h.example", "g/sub/p"));
}

#[test]
fn ссылка_на_пайплайн_и_mr_даёт_проект() {
    assert_eq!(parse_project_input("https://h.example/g/p/-/pipelines/123"), r("h.example", "g/p"));
    assert_eq!(
        parse_project_input("https://h.example/g/p/-/merge_requests/7/pipelines"),
        r("h.example", "g/p")
    );
}

#[test]
fn ssh_url_scp_и_с_портом_без_порта_в_хосте() {
    assert_eq!(
        parse_project_input("git@gitlab.example.com:group/project.git"),
        r("gitlab.example.com", "group/project")
    );
    assert_eq!(
        parse_project_input("ssh://git@gitlab.example.com/group/project.git"),
        r("gitlab.example.com", "group/project")
    );
    assert_eq!(
        parse_project_input("ssh://git@gitlab.example.com:2222/group/project"),
        r("gitlab.example.com", "group/project")
    );
}

#[test]
fn мусор_это_project_input_invalid() {
    for input in ["", "foo", "https://h.example/", "https://h.example/single", "https://h.example/g/../p", "ftp://h/g/p"] {
        assert_eq!(parse_project_input(input).unwrap_err().code, ErrorCode::ProjectInputInvalid, "{input:?}");
    }
}

fn repo(config: &str) -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    std::fs::create_dir_all(dir.path().join(".git")).unwrap();
    std::fs::write(dir.path().join(".git").join("config"), config).unwrap();
    dir
}

#[test]
fn папка_с_клоном_remote_origin() {
    let dir = repo("[core]\n\trepositoryformatversion = 0\n[remote \"upstream\"]\n\turl = git@other:x/y.git\n[remote \"origin\"]\n\turl = git@gitlab.example.com:group/project.git\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n");
    assert_eq!(parse_project_input(dir.path().to_str().unwrap()), r("gitlab.example.com", "group/project"));
}

#[test]
fn worktree_с_файлом_git_читает_общий_config() {
    let main = repo("[remote \"origin\"]\n\turl = https://gitlab.example.com/group/project.git\n");
    let wt_git = main.path().join(".git").join("worktrees").join("wt");
    std::fs::create_dir_all(&wt_git).unwrap();
    std::fs::write(wt_git.join("commondir"), "../..\n").unwrap();
    let wt = TempDir::new().unwrap();
    std::fs::write(wt.path().join(".git"), format!("gitdir: {}\n", wt_git.display())).unwrap();
    assert_eq!(parse_project_input(wt.path().to_str().unwrap()), r("gitlab.example.com", "group/project"));
}

#[test]
fn папка_без_git_или_без_origin_это_project_dir_no_remote() {
    let plain = TempDir::new().unwrap();
    assert_eq!(parse_project_input(plain.path().to_str().unwrap()).unwrap_err().code, ErrorCode::ProjectDirNoRemote);
    let no_origin = repo("[remote \"upstream\"]\n\turl = git@h:x/y.git\n");
    assert_eq!(parse_project_input(no_origin.path().to_str().unwrap()).unwrap_err().code, ErrorCode::ProjectDirNoRemote);
}
```

- [ ] **Step 3: Убедиться, что тесты падают**

Run: `cd app && cargo test -p pipeline-trace-core --test project_input`
Expected: ошибка компиляции `cannot find function parse_project_input`.

- [ ] **Step 4: Реализовать разбор**

В `app/core/src/request.rs` после `SEGMENT` (строка 21) добавить:

```rust
/// `https://host/group/project[.git][/]`; хвост `/-/…` раньше отрезает `LINK`.
static REPO_HTTP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^https?://([^/]+)/(.+?)(?:\.git)?/?$").expect("верный шаблон"));
/// `ssh://[user@]host[:port]/group/project[.git]`; порт — ssh, в хост GitLab не входит.
static REPO_SSH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^ssh://(?:[^@/]+@)?([^/:]+)(?::[0-9]+)?/(.+?)(?:\.git)?/?$").expect("верный шаблон")
});
/// `[user@]host:group/project[.git]` — scp-форма git.
static REPO_SCP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:[^@/:]+@)?([A-Za-z0-9.-]+):([^/].*?)(?:\.git)?/?$").expect("верный шаблон")
});
```

После `impl Request` (строка 120) добавить:

```rust
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
        .or_else(|| REPO_HTTP.captures(input))
        .or_else(|| REPO_SSH.captures(input))
        .or_else(|| REPO_SCP.captures(input))
        .ok_or_else(invalid)?;
    let host = caps[1].to_string();
    if !is_valid_host(&host) {
        return Err(invalid());
    }
    let path = project(&caps[2]).map_err(|_| invalid())?;
    Ok(ProjectRef { host, path })
}

/// `url` из `[remote "origin"]` в `.git/config`; worktree (`.git` — файл `gitdir:`) ведёт к общему `config`.
fn remote_origin(dir: &std::path::Path) -> Result<String, Error> {
    let no_remote = || Error::new(ErrorCode::ProjectDirNoRemote);
    let git = dir.join(".git");
    let git_dir = if git.is_file() {
        let text = std::fs::read_to_string(&git).map_err(|_| no_remote())?;
        let target = text.trim().strip_prefix("gitdir:").ok_or_else(no_remote)?.trim();
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
```

`~/…` не раскрывается: `plugin-dialog` отдаёт абсолютный путь, а ручной `~` даст `ProjectInputInvalid` — упомянуть в PR. `project()` уже отвергает `..` и одиночный сегмент.

- [ ] **Step 5: Запустить тесты**

Run: `cd app && cargo test -p pipeline-trace-core --test project_input`
Expected: `test result: ok. 7 passed`.

Run: `cd app && cargo test -p pipeline-trace-core`
Expected: все зелёные; в `app/src/shared/api/schema/` появился `ProjectRef.ts`, обновился `ErrorCode.ts`.

- [ ] **Step 6: Поправить спеку § 6**

В `docs/specs/2026-10-06-first-run-ux-design.md` заменить таблицу § 6 на:

```markdown
| Код | Текст (ru) |
|---|---|
| `projectInputInvalid` | Не похоже на ссылку на проект GitLab или путь к папке |
| `projectDirNoRemote` | В папке нет git-репозитория с remote `origin` |
| `projectNotFound` (существующий) | Проект {project} на {host} не найден или нет доступа |
| `noToken` (существующий, текст меняется) | Нужен токен для {host} |
| `unauthorized` (существующий) | GitLab {host} ответил {status}: токен недействителен или нет доступа |
| `reportExpired` | Отчёт больше не в памяти, построй заново |
```

- [ ] **Step 7: Commit**

```bash
cd app && cargo fmt --all && cd ..
git add app/core/src/error.rs app/core/src/request.rs app/core/tests/project_input.rs app/src/shared/api/schema docs/specs/2026-10-06-first-run-ux-design.md
LEFTHOOK=0 git commit -m "feat(core): разбор ввода проекта — ссылка, ssh-URL, папка с клоном"
```

---

### Task 2: Ядро — `core::projects` (`projects.json`)

**Files:**
- Create: `app/core/src/projects.rs`
- Modify: `app/core/src/lib.rs:3-19` (добавить `pub mod projects;`)
- Create: `app/core/tests/projects.rs`

**Interfaces:**
- Consumes: `request::ProjectRef`, `json_file::{read, write, lock}`, `iso::iso`.
- Produces: `projects::SavedProject { host, path, name, added_at }` (`TS`, camelCase), `projects::Projects::new(file)`, `.list() -> Result<Vec<SavedProject>, Error>`, `.add(project: ProjectRef, name: &str, now: OffsetDateTime) -> Result<Vec<SavedProject>, Error>`, `.remove(project: &ProjectRef) -> Result<Vec<SavedProject>, Error>`.

- [ ] **Step 1: Написать падающие тесты**

Создать `app/core/tests/projects.rs`:

```rust
//! «Мои проекты»: `projects.json` на tempdir.

use pipeline_trace_core::error::ErrorCode;
use pipeline_trace_core::projects::{Projects, SavedProject};
use pipeline_trace_core::request::ProjectRef;
use tempfile::TempDir;
use time::macros::datetime;

fn projects() -> (TempDir, Projects) {
    let dir = TempDir::new().expect("tempdir");
    let projects = Projects::new(dir.path().join("data").join("projects.json"));
    (dir, projects)
}

fn p(path: &str) -> ProjectRef {
    ProjectRef {
        host: "h.example".into(),
        path: path.into(),
    }
}

const T0: time::OffsetDateTime = datetime!(2026-10-06 10:00:00 UTC);

#[test]
fn пустой_файл_это_пустой_список() {
    let (_dir, projects) = projects();
    assert_eq!(projects.list().unwrap(), []);
}

#[test]
fn порядок_добавления_повтор_обновляет_имя_на_месте() {
    let (_dir, projects) = projects();
    projects.add(p("g/a"), "A", T0).unwrap();
    projects.add(p("g/b"), "B", T0).unwrap();
    let list = projects.add(p("g/a"), "A2", T0).unwrap();
    assert_eq!(
        list,
        vec![
            SavedProject {
                host: "h.example".into(),
                path: "g/a".into(),
                name: "A2".into(),
                added_at: "2026-10-06T10:00:00.000Z".into(),
            },
            SavedProject {
                host: "h.example".into(),
                path: "g/b".into(),
                name: "B".into(),
                added_at: "2026-10-06T10:00:00.000Z".into(),
            },
        ]
    );
}

#[test]
fn удаление_и_перезапуск() {
    let (dir, projects) = projects();
    projects.add(p("g/a"), "A", T0).unwrap();
    projects.add(p("g/b"), "B", T0).unwrap();
    assert_eq!(projects.remove(&p("g/a")).unwrap().len(), 1);
    let again = Projects::new(dir.path().join("data").join("projects.json"));
    let list = again.list().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].path, "g/b");
    let file = std::fs::read_to_string(dir.path().join("data").join("projects.json")).unwrap();
    assert!(file.contains("\"addedAt\""));
}

#[test]
fn битый_файл_это_ошибка_storage() {
    let (dir, projects) = projects();
    std::fs::create_dir_all(dir.path().join("data")).unwrap();
    std::fs::write(dir.path().join("data").join("projects.json"), "{").unwrap();
    assert_eq!(projects.list().unwrap_err().code, ErrorCode::Storage);
}
```

- [ ] **Step 2: Убедиться, что тесты падают**

Run: `cd app && cargo test -p pipeline-trace-core --test projects`
Expected: `could not find projects in pipeline_trace_core`.

- [ ] **Step 3: Реализовать модуль**

Создать `app/core/src/projects.rs`:

```rust
//! «Мои проекты»: `projects.json`, в порядке добавления, без секретов.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;

use crate::error::Error;
use crate::iso::iso;
use crate::json_file;
use crate::request::ProjectRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SavedProject {
    pub host: String,
    pub path: String,
    /// `nameWithNamespace` из GitLab
    pub name: String,
    /// ISO 8601
    pub added_at: String,
}

impl SavedProject {
    fn is(&self, project: &ProjectRef) -> bool {
        self.host == project.host && self.path == project.path
    }
}

pub struct Projects {
    file: PathBuf,
    lock: Mutex<()>,
}

impl Projects {
    pub fn new(file: impl Into<PathBuf>) -> Self {
        Self {
            file: file.into(),
            lock: Mutex::new(()),
        }
    }

    pub fn list(&self) -> Result<Vec<SavedProject>, Error> {
        json_file::read(&self.file)
    }

    /// В конец списка; уже добавленный проект остаётся на месте, обновляется только имя.
    pub fn add(
        &self,
        project: ProjectRef,
        name: &str,
        now: OffsetDateTime,
    ) -> Result<Vec<SavedProject>, Error> {
        let _guard = json_file::lock(&self.lock);
        let mut list = self.list()?;
        match list.iter_mut().find(|p| p.is(&project)) {
            Some(found) => found.name = name.into(),
            None => list.push(SavedProject {
                host: project.host,
                path: project.path,
                name: name.into(),
                added_at: iso(now),
            }),
        }
        self.save(list)
    }

    pub fn remove(&self, project: &ProjectRef) -> Result<Vec<SavedProject>, Error> {
        let _guard = json_file::lock(&self.lock);
        self.save(self.list()?.into_iter().filter(|p| !p.is(project)).collect())
    }

    fn save(&self, list: Vec<SavedProject>) -> Result<Vec<SavedProject>, Error> {
        json_file::write(&self.file, &list)?;
        Ok(list)
    }
}
```

В `app/core/src/lib.rs` добавить `pub mod projects;` между `pub mod model;` и `pub mod render;`. В `app/core/src/json_file.rs:1` дополнить doc-комментарий: «`projects.json`».

- [ ] **Step 4: Запустить тесты**

Run: `cd app && cargo test -p pipeline-trace-core --test projects`
Expected: `4 passed`. В `schema/` появился `SavedProject.ts`.

- [ ] **Step 5: Commit**

```bash
cd app && cargo fmt --all && cd ..
git add app/core/src/projects.rs app/core/src/lib.rs app/core/src/json_file.rs app/core/tests/projects.rs app/src/shared/api/schema
LEFTHOOK=0 git commit -m "feat(core): projects.json — список «Мои проекты»"
```

---

### Task 3: Ядро — настройки (тема, последний проект) и `browse::fetch_project`

**Files:**
- Modify: `app/core/src/settings.rs` (полностью)
- Modify: `app/core/tests/settings.rs`
- Modify: `app/core/src/browse.rs:69-109` (вынести `WireProject → Project`, добавить `fetch_project`)
- Modify: `app/core/tests/browse.rs` (новый тест)

**Interfaces:**
- Consumes: `request::ProjectRef`, `schema::Locale`.
- Produces: `settings::Theme { System, Light, Dark }` (`TS`, lowercase), `settings::AppSettings { locale, theme, last_project: Option<ProjectRef> }` (`TS`, camelCase), `settings::SettingsPatch { locale: Option<Locale>, theme: Option<Theme>, last_project: Option<ProjectRef> }` (`Deserialize`, `TS`, camelCase, все поля `#[ts(optional)]`), `Settings::get() -> Result<AppSettings, Error>`, `Settings::update(patch) -> Result<AppSettings, Error>`, `Settings::forget_project(&ProjectRef) -> Result<(), Error>`, `Settings::locale()` остаётся. `browse::fetch_project(gql, path) -> Result<Project, Error>`.

- [ ] **Step 1: Дописать падающие тесты настроек**

В `app/core/tests/settings.rs` заменить импорты и добавить тесты:

```rust
use pipeline_trace_core::error::ErrorCode;
use pipeline_trace_core::request::ProjectRef;
use pipeline_trace_core::schema::Locale;
use pipeline_trace_core::settings::{AppSettings, Settings, SettingsPatch, Theme};
use tempfile::TempDir;
```

```rust
#[test]
fn без_файла_тема_системная_последнего_проекта_нет() {
    let (_dir, settings) = settings();
    let got = settings.get().unwrap();
    assert_eq!(got.theme, Theme::System);
    assert_eq!(got.last_project, None);
    assert_eq!(got.locale, Locale::system());
}

#[test]
fn патч_меняет_только_переданное_и_переживает_перезапуск() {
    let (dir, settings) = settings();
    settings
        .update(SettingsPatch {
            theme: Some(Theme::Dark),
            ..Default::default()
        })
        .unwrap();
    let project = ProjectRef {
        host: "h.example".into(),
        path: "g/p".into(),
    };
    let got = settings
        .update(SettingsPatch {
            last_project: Some(project.clone()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(got.theme, Theme::Dark);
    assert_eq!(got.last_project, Some(project.clone()));

    let again = Settings::new(dir.path().join("data").join("settings.json"));
    assert_eq!(
        again.get().unwrap(),
        AppSettings {
            locale: Locale::system(),
            theme: Theme::Dark,
            last_project: Some(project.clone()),
        }
    );
    again.forget_project(&project).unwrap();
    assert_eq!(again.get().unwrap().last_project, None);
}

#[test]
fn старый_файл_только_с_locale_читается() {
    let (dir, settings) = settings();
    std::fs::create_dir_all(dir.path().join("data")).unwrap();
    std::fs::write(dir.path().join("data").join("settings.json"), "{\"locale\":\"en\"}").unwrap();
    let got = settings.get().unwrap();
    assert_eq!((got.locale, got.theme, got.last_project), (Locale::En, Theme::System, None));
}
```

Существующий тест `выбор_языка_сохраняется_и_переживает_перезапуск` переписать через `update(SettingsPatch { locale: Some(other), ..Default::default() })`, остальные оставить.

- [ ] **Step 2: Убедиться, что тесты падают**

Run: `cd app && cargo test -p pipeline-trace-core --test settings`
Expected: ошибки компиляции `Theme`, `SettingsPatch`.

- [ ] **Step 3: Переписать `settings.rs`**

Заменить содержимое `app/core/src/settings.rs`:

```rust
//! Настройки приложения: `settings.json` — язык, тема, последний открытый проект.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::Error;
use crate::json_file;
use crate::request::ProjectRef;
use crate::schema::Locale;

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
```

- [ ] **Step 4: Запустить тесты настроек**

Run: `cd app && cargo test -p pipeline-trace-core --test settings`
Expected: все зелёные (7 тестов).

- [ ] **Step 5: Тест `fetch_project`**

В `app/core/tests/browse.rs` дописать импорт `fetch_project` в `use pipeline_trace_core::browse::{…}` и тест:

```rust
#[tokio::test]
async fn fetch_project_поля_и_project_not_found_при_null() {
    let gql = fake_gql(|v| {
        if v["project"] == "g/p" {
            json!({ "project": { "fullPath": "g/p", "nameWithNamespace": "G / P", "lastActivityAt": null,
                                 "repository": { "rootRef": "main" } } })
        } else {
            json!({ "project": null })
        }
    });
    assert_eq!(
        fetch_project(&gql, "g/p").await.unwrap(),
        Project {
            full_path: "g/p".into(),
            name: "G / P".into(),
            last_activity_at: None,
            default_branch: Some("main".into()),
        }
    );
    assert_error(
        fetch_project(&gql, "g/none").await,
        ErrorCode::ProjectNotFound,
        &[("host", "h.example"), ("project", "g/none")],
    );
}
```

- [ ] **Step 6: Реализовать `fetch_project`**

В `app/core/src/browse.rs` после `WireProject` (строка 88) добавить `impl From<WireProject> for Project` и использовать его в `list_projects`:

```rust
impl From<WireProject> for Project {
    fn from(p: WireProject) -> Self {
        Project {
            full_path: p.full_path,
            name: p.name_with_namespace,
            last_activity_at: p.last_activity_at,
            default_branch: p.repository.and_then(|r| r.root_ref),
        }
    }
}
```

В `list_projects` заменить замыкание на `Page::from_connection(data.projects, Project::from)`. Ниже добавить:

```rust
const PROJECT_QUERY: &str = "query($project: ID!) {
  project(fullPath: $project) { fullPath nameWithNamespace lastActivityAt repository { rootRef } }
}";

/// Один проект по пути: проверка доступа при добавлении в «Мои проекты».
pub async fn fetch_project(gql: &impl Gql, path: &str) -> Result<Project, Error> {
    let found: WireProject = query_project(gql, PROJECT_QUERY, json!({ "project": path }), path).await?;
    Ok(found.into())
}
```

- [ ] **Step 7: Запустить все тесты ядра и закоммитить**

Run: `cd app && cargo test -p pipeline-trace-core`
Expected: зелёные; в `schema/` появились `Theme.ts`, `AppSettings.ts`, `SettingsPatch.ts`.

```bash
cd app && cargo fmt --all && cd ..
git add app/core/src/settings.rs app/core/src/browse.rs app/core/tests/settings.rs app/core/tests/browse.rs app/src/shared/api/schema
LEFTHOOK=0 git commit -m "feat(core): тема и последний проект в settings.json, fetch_project"
```

---

### Task 4: Ядро — хосты с источником токена

**Files:**
- Modify: `app/core/src/tokens.rs` (после `find_token`)
- Modify: `app/core/tests/tokens.rs` (новый тест)

**Interfaces:**
- Produces: `tokens::TokenSource { Keychain, Glab, Env }` (`TS`, lowercase), `tokens::HostInfo { host: String, source: TokenSource }` (`TS`), `tokens::list_hosts(store: &TokenStore, env: &dyn Fn(&str) -> Option<String>, glab: &dyn Fn(&[&str]) -> Option<String>) -> Result<Vec<HostInfo>, Error>`.

- [ ] **Step 1: Падающий тест**

В `app/core/tests/tokens.rs` добавить импорт `HostInfo, TokenSource, list_hosts` в `use pipeline_trace_core::tokens::{…}` и тест:

```rust
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
}
```

- [ ] **Step 2: Убедиться, что тест падает**

Run: `cd app && cargo test -p pipeline-trace-core --test tokens list_hosts`
Expected: `cannot find … list_hosts`.

- [ ] **Step 3: Реализовать**

В `app/core/src/tokens.rs` после `find_token` (строка 152) добавить:

```rust
/// Откуда у хоста токен; «Удалить» в настройках есть только у `Keychain`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum TokenSource {
    Keychain,
    Glab,
    Env,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct HostInfo {
    pub host: String,
    pub source: TokenSource,
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
        })
        .collect();
    let mut push = |host: String, source: TokenSource| {
        if !hosts.iter().any(|h| h.host == host) {
            hosts.push(HostInfo { host, source });
        }
    };
    if let Some(host) = glab(&["config", "get", "host"]).map(|h| bare(&h).to_string())
        && glab(&["config", "get", "token", "--host", &host]).is_some()
    {
        push(host, TokenSource::Glab);
    }
    if env("GITLAB_TOKEN").is_some_and(|t| !t.is_empty())
        && let Some(host) = env("GITLAB_HOST").map(|h| bare(&h).to_string())
    {
        push(host, TokenSource::Env);
    }
    Ok(hosts)
}
```

Добавить `use serde::Serialize; use ts_rs::TS;` в начало файла.

- [ ] **Step 4: Запустить тесты и закоммитить**

Run: `cd app && cargo test -p pipeline-trace-core --test tokens`
Expected: зелёные; в `schema/` появились `TokenSource.ts`, `HostInfo.ts`.

```bash
cd app && cargo fmt --all && cd ..
git add app/core/src/tokens.rs app/core/tests/tokens.rs app/src/shared/api/schema
LEFTHOOK=0 git commit -m "feat(core): список хостов с источником токена"
```

---

### Task 5: Оболочка — отчёт в памяти по id, удаление окон отчётов

**Files:**
- Modify: `app/src-tauri/Cargo.toml` (добавить `serde_json = "1"`)
- Modify: `app/src-tauri/src/state.rs` (полностью)
- Modify: `app/src-tauri/src/reports.rs` (полностью)
- Modify: `app/src-tauri/src/windows.rs` (удалить отчётные части, `focused` → `form`)
- Modify: `app/src-tauri/src/main.rs` (схема, события окна, список команд, тест)
- Modify: `app/src-tauri/src/menu.rs` (`save`, `zoom`, `fullscreen`, `minimize`)
- Modify: `app/src-tauri/src/commands.rs` (`build`, новые `report`, `find_report`, `set_current_report`)
- Modify: `app/src-tauri/src/strings.rs` (удалить `all_pipelines`)
- Modify: `app/src-tauri/build.rs`, `app/src-tauri/capabilities/form.json`

**Interfaces:**
- Consumes: `request::Request`, `ErrorCode::ReportExpired`.
- Produces: команды `build(form, on_progress) -> u32`, `report(id: u32) -> String` (JSON `Report`), `find_report(host: String, request: Request) -> Option<u32>`, `set_current_report(id: Option<u32>) -> ()`. `AppState::{push_report, report_json, find_report, set_current, current_report}`. `windows::form(app) -> Option<WebviewWindow>`. `reports::save_current(app)`.

- [ ] **Step 1: Переписать `state.rs`**

```rust
//! Общее состояние приложения: хранилища на диске, отчёты в памяти, масштаб окна.

use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use pipeline_trace_core::history::History;
use pipeline_trace_core::projects::Projects;
use pipeline_trace_core::request::Request;
use pipeline_trace_core::schema::Locale;
use pipeline_trace_core::settings::Settings;
use pipeline_trace_core::tokens::{TokenStore, platform_store};

const ZOOM_MIN: f64 = 0.25;
const ZOOM_MAX: f64 = 5.0;
/// Сколько отчётов держится в памяти: повтор из «Недавних» не ходит в GitLab, старые вытесняются.
// ponytail: html и json каждого отчёта лежат целиком; если память станет заметна — хранить только json и рендерить html при ⌘S
const REPORTS_KEPT: usize = 10;

/// Отчёт в памяти: `json` отдаёт команда `report`, `html` пишет «Сохранить отчёт…».
pub struct ReportEntry {
    pub host: String,
    pub request: Request,
    pub json: String,
    pub html: String,
    pub file_name: String,
}

pub struct AppState {
    pub tokens: TokenStore,
    pub history: History,
    pub settings: Settings,
    pub projects: Projects,
    /// от старого к новому; id растут
    reports: Mutex<VecDeque<(u32, ReportEntry)>>,
    /// отчёт на экране формы: ему адресовано ⌘S
    current_report: Mutex<Option<u32>>,
    /// label окна → масштаб: у webview нет геттера
    pub zoom: Mutex<HashMap<String, f64>>,
    next_report: AtomicU32,
}

impl AppState {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            tokens: TokenStore::new(data_dir.join("hosts.json"), platform_store()),
            history: History::new(data_dir.join("history.json")),
            settings: Settings::new(data_dir.join("settings.json")),
            projects: Projects::new(data_dir.join("projects.json")),
            reports: Mutex::default(),
            current_report: Mutex::default(),
            zoom: Mutex::default(),
            next_report: AtomicU32::new(0),
        }
    }

    /// Новый отчёт получает id; самый старый сверх `REPORTS_KEPT` вытесняется.
    pub fn push_report(&self, entry: ReportEntry) -> u32 {
        let id = self.next_report.fetch_add(1, Ordering::SeqCst) + 1;
        let mut reports = lock(&self.reports);
        reports.push_back((id, entry));
        while reports.len() > REPORTS_KEPT {
            reports.pop_front();
        }
        id
    }

    pub fn report_json(&self, id: u32) -> Option<String> {
        lock(&self.reports)
            .iter()
            .find(|(found, _)| *found == id)
            .map(|(_, entry)| entry.json.clone())
    }

    /// Свежайший отчёт с тем же хостом и запросом — ключ записи истории.
    pub fn find_report(&self, host: &str, request: &Request) -> Option<u32> {
        lock(&self.reports)
            .iter()
            .rev()
            .find(|(_, e)| e.host == host && e.request == *request)
            .map(|(id, _)| *id)
    }

    pub fn set_current(&self, id: Option<u32>) {
        *lock(&self.current_report) = id;
    }

    /// `html` и имя файла отчёта на экране.
    pub fn current_report(&self) -> Option<(String, String)> {
        let id = (*lock(&self.current_report))?;
        lock(&self.reports)
            .iter()
            .find(|(found, _)| *found == id)
            .map(|(_, e)| (e.html.clone(), e.file_name.clone()))
    }

    /// Окно закрыто: его масштаб больше не нужен.
    pub fn forget(&self, label: &str) {
        lock(&self.zoom).remove(label);
    }

    /// Новый масштаб окна после `change`, в пределах `ZOOM_MIN..=ZOOM_MAX`.
    pub fn zoom_by(&self, label: &str, change: impl Fn(f64) -> f64) -> f64 {
        let mut zooms = lock(&self.zoom);
        let factor = zooms.entry(label.into()).or_insert(1.0);
        *factor = change(*factor).clamp(ZOOM_MIN, ZOOM_MAX);
        *factor
    }

    /// Язык интерфейса; не читается файл настроек — системный.
    pub fn locale(&self) -> Locale {
        self.settings.locale().unwrap_or_else(|_| Locale::system())
    }
}

/// Замки охраняют только карты в памяти: паника в другом потоке не повод падать дальше.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(n: u32) -> ReportEntry {
        ReportEntry {
            host: "h".into(),
            request: Request::Pipeline {
                project: "g/p".into(),
                pipeline_id: n.to_string(),
            },
            json: format!("{{\"n\":{n}}}"),
            html: String::new(),
            file_name: format!("{n}.html"),
        }
    }

    #[test]
    fn одиннадцатый_отчёт_вытесняет_первый_а_поиск_находит_свежайший() {
        let state = AppState::new(&std::env::temp_dir().join("pipeline-trace-state-test"));
        let ids: Vec<u32> = (1..=11).map(|n| state.push_report(entry(n))).collect();
        assert_eq!(ids[0], 1);
        assert_eq!(state.report_json(1), None);
        assert_eq!(state.report_json(2).as_deref(), Some("{\"n\":2}"));
        let same = state.push_report(entry(2));
        assert_eq!(state.find_report("h", &entry(2).request), Some(same));
        assert_eq!(state.find_report("h", &entry(1).request), None);
        state.set_current(Some(same));
        assert_eq!(state.current_report().map(|(_, f)| f).as_deref(), Some("2.html"));
        state.set_current(None);
        assert_eq!(state.current_report(), None);
    }
}
```

- [ ] **Step 2: Переписать `reports.rs`**

```rust
//! Отчёты в памяти: шаблон и «Сохранить отчёт…».

use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::state::AppState;
use crate::strings::strings;
use crate::windows::form;

/// Собранный отчёт-шаблон; `build.rs` проверяет, что он есть, и пересобирает при изменении.
pub const TEMPLATE: &str = include_str!("../../dist-report/report.html");

/// ⌘S: сохраняется отчёт на экране формы; нет отчёта или окна — ничего.
pub fn save_current(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Some((html, file_name)) = state.current_report() else {
        return;
    };
    let Some(window) = form(app) else {
        return;
    };
    let error_title = strings(state.locale()).save_error;
    let failed = app.clone();
    // неблокирующий: сохранение не держит главный поток
    app.dialog()
        .file()
        .set_parent(&window)
        .set_file_name(&file_name)
        .add_filter("HTML", &["html"])
        .save_file(move |chosen| {
            let Some(chosen) = chosen else { return };
            let written = chosen
                .into_path()
                .map_err(|e| e.to_string())
                .and_then(|path| std::fs::write(path, html).map_err(|e| e.to_string()));
            if let Err(detail) = written {
                failed
                    .dialog()
                    .message(detail)
                    .title(error_title)
                    .kind(MessageDialogKind::Error)
                    .show(|_| {});
            }
        });
}
```

- [ ] **Step 3: Упростить `windows.rs`**

Удалить `REPORT_SCHEME`, `report_url`, `create_report`, строку `app.state::<AppState>().focus(spec.label);` в `open` и импорт `AppState`. Заменить `focused`:

```rust
/// Окно формы — единственное окно; ему адресованы ⌘S и пункты «Вид».
pub fn form(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(FORM)
}
```

Обновить doc-комментарий модуля: «Окно формы: запрет навигации и новых окон». В тестах убрать все `own("report", …)` строки, оставить проверки `tauri` и чужих origin (`https://gitlab.example.com/`, `file:///etc/passwd`, порт).

- [ ] **Step 4: Обновить `menu.rs`**

Импорты: `use crate::reports::save_current; use crate::windows::{form, show_form};`. В `zoom` и `on_event` заменить `focused(app)` на `form(app)`, `"save" => { save_current(app); Ok(()) }`. «Новый отчёт» ⌘N остаётся фокусом на окно формы (`show_form`): переход на `/` из меню потребовал бы события в webview, а окно одно. В спеке § 2 заменить «„Новый отчёт“ ⌘N — переход на `/`» на «„Новый отчёт“ ⌘N — фокус на окно, как сейчас».

- [ ] **Step 5: Обновить `main.rs`**

Удалить `.register_uri_scheme_protocol("report", reports::serve)`. Комментарий к `opener`: «ссылки `https` открывает Rust из `on_navigation`/`on_new_window`, скрипт плагина не нужен». В `on_window_event` оставить только `WindowEvent::Destroyed => state.forget(window.label())`. В `generate_handler!` добавить `commands::report, commands::find_report, commands::set_current_report` после `commands::build`. В тесте `assert_eq!(in_build.len(), 15)`.

В `build.rs` в `COMMANDS` добавить `"report", "find_report", "set_current_report"`; в `capabilities/form.json` — `"allow-report", "allow-find-report", "allow-set-current-report"`; описание capability: «Единственное окно приложения».

- [ ] **Step 6: Обновить `commands.rs`**

Импорты: убрать `Locale` из `schema`, `strings`, `ReportEntry` остаётся из `state`, `open_report` убрать; добавить `use pipeline_trace_core::request::Request;`. Удалить `report_title`. Переписать `build` с момента `let (Report::Single…)`:

```rust
    let (Report::Single { meta, .. } | Report::Aggregate { meta, .. }) = &built.report;
    let label = history_label(meta);
    let html = render(&built.report, TEMPLATE);
    let json = serde_json::to_string(&built.report)
        .map_err(|e| Error::new(ErrorCode::Storage).with("detail", e))?;
    state.history.add(
        NewEntry {
            host: parsed.host.clone(),
            form,
            request: parsed.request.clone(),
            label,
        },
        now,
    )?;
    Ok(state.push_report(ReportEntry {
        host: parsed.host,
        request: parsed.request,
        json,
        html,
        file_name: built.file_name,
    }))
}

/// JSON отчёта по id; вытесненный из памяти — `report_expired`.
#[tauri::command]
pub async fn report(webview: Webview, state: State<'_, AppState>, id: u32) -> Cmd<String> {
    ensure_form(&webview)?;
    state
        .report_json(id)
        .ok_or_else(|| Error::new(ErrorCode::ReportExpired).with("id", id).into())
}

/// Повтор из «Недавних»: готовый отчёт с тем же хостом и запросом, если он ещё в памяти.
#[tauri::command]
pub async fn find_report(
    webview: Webview,
    state: State<'_, AppState>,
    host: String,
    request: Request,
) -> Cmd<Option<u32>> {
    ensure_form(&webview)?;
    Ok(state.find_report(&host, &request))
}

/// Экран отчёта сообщает, какой отчёт на экране (`None` — ушли с него): ему адресовано ⌘S.
#[tauri::command]
pub async fn set_current_report(
    webview: Webview,
    state: State<'_, AppState>,
    id: Option<u32>,
) -> Cmd<()> {
    ensure_form(&webview)?;
    state.set_current(id);
    Ok(())
}
```

Сигнатура `build` становится `-> Cmd<u32>`; комментарий над ней: «Собирает отчёт, пишет историю и кладёт отчёт в память; возвращает его id».

В `strings.rs` удалить поле `all_pipelines` из `Strings`, `RU`, `EN` и слово «заголовок окна отчёта» из doc-комментария.

- [ ] **Step 7: Собрать и проверить**

Run: `cd app && npm run build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p pipeline-trace`
Expected: без предупреждений; тесты `списки_команд_совпадают`, `одиннадцатый_отчёт…`, `own_origin_on_every_platform`, `foreign_origin_is_refused` зелёные.

Run: `cd app && npm run tauri dev` → клик по пайплайну в форме больше не открывает окно (отчёт пока некуда рендерить, это ожидаемо до задачи 10); ошибок в консоли Rust нет.

- [ ] **Step 8: Commit**

```bash
cd app && cargo fmt --all && cd ..
git add app/src-tauri
LEFTHOOK=0 git commit -m "refactor(tauri): отчёт в памяти по id вместо окон report-<n>"
```

---

### Task 6: Оболочка — команды проектов, настроек, хостов

**Files:**
- Modify: `app/src-tauri/src/commands.rs` (`hosts`, новые `add_project`, `remove_project`, `saved_projects`, `get_settings`, `set_settings`; удалить `get_locale`, `set_locale`)
- Modify: `app/src-tauri/src/main.rs`, `app/src-tauri/build.rs`, `app/src-tauri/capabilities/form.json`

**Interfaces:**
- Consumes: `projects::{Projects, SavedProject}`, `settings::{AppSettings, SettingsPatch}`, `tokens::{HostInfo, list_hosts}`, `browse::fetch_project`, `request::{ProjectRef, parse_project_input}`.
- Produces: команды `hosts() -> HostInfo[]`, `add_project(input: String) -> SavedProject`, `remove_project(project: ProjectRef) -> SavedProject[]`, `saved_projects() -> SavedProject[]`, `get_settings() -> AppSettings`, `set_settings(patch: SettingsPatch) -> AppSettings`. Полный список команд (18): `hosts, set_token, remove_token, history, remove_history, clear_history, projects, branches, pipelines, build, report, find_report, set_current_report, add_project, remove_project, saved_projects, get_settings, set_settings`.

- [ ] **Step 1: Переписать `hosts` и удалить команды языка**

```rust
/// Хосты с источником токена; `glab` запускается — поэтому в пуле блокирующих задач.
#[tauri::command]
pub async fn hosts(app: AppHandle, webview: Webview) -> Cmd<Vec<HostInfo>> {
    ensure_form(&webview)?;
    tauri::async_runtime::spawn_blocking(move || {
        let glab = Glab::find();
        list_hosts(
            &app.state::<AppState>().tokens,
            &|name| std::env::var(name).ok(),
            &|args| glab.as_ref().and_then(|glab| glab.value(args)),
        )
    })
    .await
    .map_err(|e| Error::new(ErrorCode::Storage).with("detail", e))?
    .map_err(Into::into)
}
```

Удалить `get_locale` и `set_locale`. Добавить:

```rust
#[tauri::command]
pub async fn saved_projects(webview: Webview, state: State<'_, AppState>) -> Cmd<Vec<SavedProject>> {
    ensure_form(&webview)?;
    Ok(state.projects.list()?)
}

/// Разбор ввода (может читать `.git/config`), проверка доступа через GraphQL, запись в `projects.json`.
#[tauri::command]
pub async fn add_project(
    app: AppHandle,
    webview: Webview,
    state: State<'_, AppState>,
    input: String,
) -> Cmd<SavedProject> {
    ensure_form(&webview)?;
    let project = tauri::async_runtime::spawn_blocking(move || parse_project_input(&input))
        .await
        .map_err(|e| Error::new(ErrorCode::Storage).with("detail", e))??;
    let gql = client_for(&app, &project.host).await?;
    let found = fetch_project(&gql, &project.path).await?;
    let list = state
        .projects
        .add(project.clone(), &found.name, OffsetDateTime::now_utc())?;
    let saved = list
        .into_iter()
        .find(|p| p.host == project.host && p.path == project.path)
        .ok_or_else(|| Error::new(ErrorCode::Storage).with("detail", "проект не записался"))?;
    Ok(saved)
}

#[tauri::command]
pub async fn remove_project(
    webview: Webview,
    state: State<'_, AppState>,
    project: ProjectRef,
) -> Cmd<Vec<SavedProject>> {
    ensure_form(&webview)?;
    state.settings.forget_project(&project)?;
    Ok(state.projects.remove(&project)?)
}

#[tauri::command]
pub async fn get_settings(webview: Webview, state: State<'_, AppState>) -> Cmd<AppSettings> {
    ensure_form(&webview)?;
    Ok(state.settings.get()?)
}

/// Частичное обновление; смена языка пересобирает меню.
#[tauri::command]
pub async fn set_settings(
    app: AppHandle,
    webview: Webview,
    state: State<'_, AppState>,
    patch: SettingsPatch,
) -> Cmd<AppSettings> {
    ensure_form(&webview)?;
    let locale = patch.locale;
    let settings = state.settings.update(patch)?;
    if let Some(locale) = locale {
        menu::build(&app, locale)
            .and_then(|menu| app.set_menu(menu).map(drop))
            .map_err(window_error)?;
    }
    Ok(settings)
}
```

Импорты в `commands.rs`: `browse::{…, fetch_project}`, `projects::SavedProject`, `request::{Form, ProjectRef, Request, parse_form, parse_project_input}`, `settings::{AppSettings, SettingsPatch}`, `tokens::{Glab, HostInfo, find_token, list_hosts}`.

- [ ] **Step 2: Три списка команд**

`build.rs` `COMMANDS`: убрать `get_locale`, `set_locale`; добавить `add_project`, `remove_project`, `saved_projects`, `get_settings`, `set_settings`. `main.rs` `generate_handler!` — то же; тест `assert_eq!(in_build.len(), 18)`. `capabilities/form.json`: убрать `allow-get-locale`, `allow-set-locale`; добавить `allow-add-project`, `allow-remove-project`, `allow-saved-projects`, `allow-get-settings`, `allow-set-settings`, а также `dialog:allow-open` (выбор папки в диалоге «Добавить проект»).

- [ ] **Step 3: Собрать, прогнать тесты, сгенерировать типы**

Run: `cd app && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p pipeline-trace --bin pipeline-trace && cargo test -p pipeline-trace-core`
Expected: зелёные. `git status app/src/shared/api/schema` показывает новые `ProjectRef.ts`, `SavedProject.ts`, `Theme.ts`, `AppSettings.ts`, `SettingsPatch.ts`, `TokenSource.ts`, `HostInfo.ts` и изменённый `ErrorCode.ts`.

- [ ] **Step 4: Commit**

```bash
cd app && cargo fmt --all && cd ..
git add app/src-tauri app/src/shared/api/schema
LEFTHOOK=0 git commit -m "feat(tauri): команды проектов, настроек и хостов с источником токена"
```

---

### Task 7: Фронтенд — API, словари, тема по классу, настройки приложения

**Files:**
- Modify: `app/src/shared/api/api.ts`, `app/src/shared/api/index.ts`
- Modify: `app/src/shared/i18n/locales/ru.json`, `app/src/shared/i18n/locales/en.json`
- Create: `app/src/shared/lib/theme.ts`
- Modify: `app/src/shared/ui/theme.css:64-115`
- Create: `app/src/entities/settings/model/settings.ts`, `app/src/entities/settings/index.ts`
- Modify: `app/src/app/form/main.tsx`, `app/src/app/report/main.tsx`
- Modify: `app/src/widgets/language-switch/ui/LanguageSwitch.tsx`
- Modify: `app/src/entities/host/model/hosts.ts`, `app/src/features/manage-token/ui/HostSelect.tsx`

**Interfaces:**
- Consumes: типы из `shared/api/schema/` (задача 6).
- Produces: `api.{hosts(): HostInfo[], savedProjects, addProject(input), removeProject(project), getSettings, setSettings(patch), build(): number, report(id): Report, findReport(host, request): number | null, setCurrentReport(id | null)}`; `shared/lib/theme.applyTheme(theme: Theme)`; `entities/settings.{settingsQuery, useSettings, useUpdateSettings}`; `entities/host.useHostState().saved: HostInfo[]`.

- [ ] **Step 1: `api.ts`**

Заменить импорты типов и экспорты (с `hosts` до конца файла):

```ts
import type { AppSettings } from './schema/AppSettings'
import type { HostInfo } from './schema/HostInfo'
import type { ProjectRef } from './schema/ProjectRef'
import type { Report } from './schema/Report'
import type { Request } from './schema/Request'
import type { SavedProject } from './schema/SavedProject'
import type { SettingsPatch } from './schema/SettingsPatch'
```

```ts
export const hosts = () => call<HostInfo[]>('hosts')
export const setToken = (host: string, token: string) => call<null>('set_token', { host, token })
export const removeToken = (host: string) => call<null>('remove_token', { host })
export const history = () => call<HistoryEntry[]>('history')
export const removeHistory = (at: string) => call<HistoryEntry[]>('remove_history', { at })
export const clearHistory = () => call<null>('clear_history')
export const projects = (host: string, search: string, after: string | null) =>
  call<Page<Project>>('projects', { host, search, after })
export const branches = (host: string, project: string, search: string) =>
  call<string[]>('branches', { host, project, search })
export const pipelines = (host: string, project: string, ref: string | null, after: string | null) =>
  call<Page<Pipeline>>('pipelines', { host, project, ref, after })
/** Собирает отчёт и возвращает его id для `/report/$id`; `onProgress` получает `{ loaded, total }` только от этой сборки. */
export const build = (form: Form, onProgress: (p: Progress) => void) => {
  const channel = new Channel<Progress>(onProgress)
  return call<number>('build', { form, onProgress: channel })
}
/** Ядро отдаёт JSON текстом: `Report` не клонируется в Rust, а строка уже есть у `render`. */
export const report = async (id: number): Promise<Result<Report>> => {
  const r = await call<string>('report', { id })
  return r.ok ? { ok: true, value: JSON.parse(r.value) as Report } : r
}
export const findReport = (host: string, request: Request) => call<number | null>('find_report', { host, request })
export const setCurrentReport = (id: number | null) => call<null>('set_current_report', { id })
export const savedProjects = () => call<SavedProject[]>('saved_projects')
export const addProject = (input: string) => call<SavedProject>('add_project', { input })
export const removeProject = (project: ProjectRef) => call<SavedProject[]>('remove_project', { project })
export const getSettings = () => call<AppSettings>('get_settings')
export const setSettings = (patch: SettingsPatch) => call<AppSettings>('set_settings', { patch })
```

В `index.ts` экспорт функций: `addProject, branches, build, clearHistory, findReport, getSettings, history, hosts, pipelines, projects, removeHistory, removeProject, removeToken, report, savedProjects, setCurrentReport, setSettings, setToken`; добавить `export type` для `AppSettings`, `HostInfo`, `ProjectRef`, `Request`, `SavedProject`, `SettingsPatch`, `Theme`, `TokenSource`.

- [ ] **Step 2: Словари**

`ru.json`, блок `errors`: изменить `"noToken": "Нужен токен для {{host}}"`; добавить

```json
    "projectInputInvalid": "Не похоже на ссылку на проект GitLab или путь к папке",
    "projectDirNoRemote": "В папке нет git-репозитория с remote origin",
    "reportExpired": "Отчёт больше не в памяти, построй заново"
```

В блок `form` добавить (ключи используются задачами 8–11; `host.hint` и `host.placeholder` остаются для `TokenForm` в настройках):

```json
    "settingsButton": "Настройки",
    "sidebar": {
      "projects": "Мои проекты",
      "add": "+ Добавить",
      "recent": "Недавние",
      "removeProject": "Убрать из списка: {{name}}"
    },
    "empty": {
      "title": "Добавь первый проект",
      "text": "Вставь ссылку на репозиторий GitLab или выбери папку с клоном.",
      "add": "+ Добавить проект"
    },
    "addProject": {
      "title": "Добавить проект",
      "placeholder": "https://gitlab.example.com/group/project или путь к папке",
      "hint": "Подойдёт ссылка на репозиторий, пайплайн или MR.",
      "pickFolder": "Выбрать папку…",
      "findOnHost": "Найти в списке проектов хоста",
      "add": "Добавить"
    },
    "token": {
      "need": "Нужен токен для {{host}}",
      "scope": "Personal Access Token со scope read_api.",
      "create": "Создать на {{host}} ↗",
      "stored": "Хранится в системной связке ключей."
    },
    "settings": {
      "title": "Настройки",
      "back": "← Назад",
      "hosts": "Хосты и токены",
      "noHosts": "Пока нет ни одного хоста",
      "addHost": "Добавить хост",
      "source": { "keychain": "связка ключей", "glab": "glab", "env": "переменные окружения" },
      "theme": "Тема",
      "themes": { "system": "Системная", "light": "Светлая", "dark": "Тёмная" }
    },
    "reportView": {
      "back": "← Назад",
      "loading": "Загрузка отчёта…"
    }
```

Ключи `form.projects.title`, `form.projects.back` убираются в задаче 8 вместе с экраном «Проекты». `en.json` — те же ключи по-английски: `"noToken": "A token for {{host}} is needed"`, `"projectInputInvalid": "Doesn't look like a GitLab project link or a folder path"`, `"projectDirNoRemote": "The folder has no git repository with an origin remote"`, `"reportExpired": "The report is no longer in memory, build it again"`, `sidebar`: `My projects` / `+ Add` / `Recent` / `Remove from the list: {{name}}`, `empty`: `Add your first project` / `Paste a GitLab repository link or pick a folder with a clone.` / `+ Add project`, `addProject`: `Add project` / `https://gitlab.example.com/group/project or a folder path` / `A repository, pipeline or MR link works.` / `Pick folder…` / `Find in the host's project list` / `Add`, `token`: `A token for {{host}} is needed` / `Personal Access Token with the read_api scope.` / `Create on {{host}} ↗` / `Stored in the system keychain.`, `settings`: `Settings` / `← Back` / `Hosts and tokens` / `No hosts yet` / `Add host` / `keychain` / `glab` / `environment variables` / `Theme` / `System` / `Light` / `Dark`, `reportView`: `← Back` / `Loading the report…`, `settingsButton`: `Settings`.

Run: `cd app && npm run check:locales` → без ошибок.

- [ ] **Step 3: Тема по классу**

Создать `app/src/shared/lib/theme.ts`:

```ts
import type { Theme } from '../api/schema/Theme'

const media = window.matchMedia('(prefers-color-scheme: dark)')
let detach: (() => void) | null = null

/** Класс `dark` на `<html>`; `system` следит за системной темой до следующего вызова. */
export function applyTheme(theme: Theme) {
  detach?.()
  detach = null
  const set = (dark: boolean) => document.documentElement.classList.toggle('dark', dark)
  if (theme !== 'system') {
    set(theme === 'dark')
    return
  }
  set(media.matches)
  const onChange = (e: MediaQueryListEvent) => set(e.matches)
  media.addEventListener('change', onChange)
  detach = () => media.removeEventListener('change', onChange)
}
```

В `theme.css`: комментарий строки 4 → «Тёмная тема — класс `dark` на `<html>` (`shared/lib/theme.ts`): системная, светлая или тёмная из настроек; сохранённый отчёт следует системе.» Блок `:root { color-scheme: light dark; … }` → `:root { color-scheme: light; --radius: 0.625rem; }`. Заменить `@media (prefers-color-scheme: dark) { :root { … } }` на `:root.dark { color-scheme: dark; … те же переменные … }` (убрать один уровень вложенности).

В `app/src/app/report/main.tsx` добавить `import { applyTheme } from '../../shared/lib/theme'` и первой строкой `main()` — `applyTheme('system')`.

- [ ] **Step 4: Сущность настроек**

`app/src/entities/settings/model/settings.ts`:

```ts
import { queryOptions, useQuery, useQueryClient } from '@tanstack/react-query'
import { useCallback } from 'react'
import { getSettings, setSettings, unwrap, type SettingsPatch } from '../../../shared/api'
import { showError } from '../../../shared/lib/dialogs'

export const settingsQuery = queryOptions({ queryKey: ['settings'], queryFn: () => unwrap(getSettings()), staleTime: Infinity })

export const useSettings = () => useQuery(settingsQuery)

/** Частичное обновление; ответ ядра — полные настройки, они сразу в кеше. Стабильна: годится в зависимости эффектов. */
export function useUpdateSettings() {
  const queryClient = useQueryClient()
  return useCallback(
    async (patch: SettingsPatch) => {
      const result = await setSettings(patch)
      if (result.ok) queryClient.setQueryData(settingsQuery.queryKey, result.value)
      else await showError(result.error)
      return result.ok
    },
    [queryClient],
  )
}
```

`index.ts`: `export { settingsQuery, useSettings, useUpdateSettings } from './model/settings'`.

- [ ] **Step 5: Старт формы и переключатель языка**

`app/src/app/form/main.tsx`: заменить `getLocale` на `getSettings`, добавить импорты `applyTheme` и `settingsQuery`:

```ts
  const settings = await getSettings()
  const initial = settings.ok ? settings.value.locale : browserLocale()
  applyTheme(settings.ok ? settings.value.theme : 'system')
  if (settings.ok) queryClient.setQueryData(settingsQuery.queryKey, settings.value)
  await initI18n(initial)
```

`LanguageSwitch.tsx` (widgets/language-switch): вместо `setLocale` — `const update = useUpdateSettings()`, в `choose`: `await changeLocale(locale); await update({ locale })` (ошибку показывает `useUpdateSettings`).

`entities/host/model/hosts.ts`: `saved` теперь `HostInfo[]`; `host = picked.kind === 'auto' ? (saved[0]?.host ?? null) : …`, `tokenHost: host !== null && saved.some((h) => h.host === host) ? host : null`. В `HostSelect.tsx` строки 31–35: `saved.map((h) => <SelectItem key={h.host} value={h.host}>{t('form.host.saved', { host: h.host })}</SelectItem>)`.

- [ ] **Step 6: Проверка и коммит**

Run: `cd app && npm run typecheck && npm run lint:fsd && npm run check:locales`
Expected: зелёные.

```bash
git add app/src
git commit -m "feat(web): API новых команд, словари, тема по классу, сущность настроек"
```

---

### Task 8: Фронтенд — раскладка, сайдбар «Мои проекты», маршруты

**Files:**
- Create: `app/src/entities/saved-project/model/queries.ts`, `app/src/entities/saved-project/ui/SavedProjectItem.tsx`, `app/src/entities/saved-project/index.ts`
- Create: `app/src/features/add-project/model/dialog.ts`, `app/src/features/add-project/index.ts`
- Create: `app/src/widgets/projects-sidebar/ui/ProjectsSidebar.tsx`, `app/src/widgets/projects-sidebar/index.ts`
- Create: `app/src/widgets/app-header/ui/AppHeader.tsx`, `app/src/widgets/app-header/index.ts`
- Create: `app/src/pages/home/ui/HomePage.tsx`, `app/src/pages/home/index.ts`
- Remove: `app/src/widgets/history-sidebar/`, `app/src/pages/projects/`, ключи `form.projects.title`, `form.projects.back` в обоих словарях
- Modify: `app/src/app/form/router.tsx` (полностью), `app/src/pages/project/ui/ProjectPage.tsx`, `app/src/shared/ui/button.tsx` (экспорт `buttonVariants`), `app/steiger.config.ts`

**Interfaces:**
- Consumes: `api.{savedProjects, removeProject, findReport}`, `entities/settings`, `entities/history-entry`, `features/build-by-link.fillLink`, `features/build-aggregate.fillAggSettings`, `features/history-actions`.
- Produces: `entities/saved-project.{savedProjectsQuery, useSavedProjects, useRemoveProject, SavedProjectItem}`; `features/add-project.{openAddProject, closeAddProject, useAddProjectOpen}`; маршруты `/`, `/project/$host/$`, `/report/$id` (заглушка до задачи 10), `/settings` (заглушка до задачи 11); `Layout` без сайдбара и шапки на `/report/$id`; клавиши Esc, ⌘⇧N, ⌘,.

- [ ] **Step 1: Сущность «сохранённый проект»**

`entities/saved-project/model/queries.ts`:

```ts
import { queryOptions, useQuery, useQueryClient } from '@tanstack/react-query'
import { apiError, removeProject, savedProjects, unwrap, type ProjectRef } from '../../../shared/api'
import { showError } from '../../../shared/lib/dialogs'

export const savedProjectsQuery = queryOptions({ queryKey: ['savedProjects'], queryFn: () => unwrap(savedProjects()) })

export function useSavedProjects() {
  const { data, isPending, error } = useQuery(savedProjectsQuery)
  return { projects: data ?? [], ready: !isPending, error: apiError(error) }
}

/** Убирает проект из списка; ядро заодно забывает его как «последний». */
export function useRemoveProject() {
  const queryClient = useQueryClient()
  return async (project: ProjectRef) => {
    const result = await removeProject(project)
    if (result.ok) queryClient.setQueryData(savedProjectsQuery.queryKey, result.value)
    else await showError(result.error)
  }
}
```

`entities/saved-project/ui/SavedProjectItem.tsx`:

```tsx
import { XIcon } from 'lucide-react'
import type { SavedProject } from '../../../shared/api'
import { cn } from '../../../shared/lib/cn'
import { useTranslation } from '../../../shared/i18n'

type Props = { project: SavedProject; active: boolean; onOpen: () => void; onRemove: () => void }

// крестик — вне <button>: вложенные кнопки недопустимы
export function SavedProjectItem({ project, active, onOpen, onRemove }: Props) {
  const { t } = useTranslation()
  return (
    <li className="group flex items-center gap-1">
      <button
        type="button"
        aria-current={active ? 'page' : undefined}
        onClick={onOpen}
        className={cn(
          'flex min-w-0 flex-1 flex-col rounded-md px-2 py-1.5 text-left outline-hidden hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring',
          active && 'bg-selected',
        )}
      >
        <span translate="no" className="truncate text-sm font-medium">
          {project.name}
        </span>
        <span translate="no" className="truncate text-xs text-muted-foreground">
          {project.host}
        </span>
      </button>
      <button
        type="button"
        aria-label={t('form.sidebar.removeProject', { name: project.name })}
        onClick={onRemove}
        className="rounded-md p-1 text-muted-foreground opacity-0 outline-hidden group-hover:opacity-100 hover:bg-accent focus-visible:opacity-100 focus-visible:ring-2 focus-visible:ring-ring"
      >
        <XIcon className="size-4" />
      </button>
    </li>
  )
}
```

`index.ts`: экспорт `savedProjectsQuery, useSavedProjects, useRemoveProject, SavedProjectItem`.

- [ ] **Step 2: Стор диалога добавления**

`features/add-project/model/dialog.ts`:

```ts
import { create } from 'zustand'

const useDialogStore = create<{ open: boolean }>(() => ({ open: false }))

export const openAddProject = () => useDialogStore.setState({ open: true })
export const closeAddProject = () => useDialogStore.setState({ open: false })
export const useAddProjectOpen = () => useDialogStore((s) => s.open)
```

`index.ts`: `export { closeAddProject, openAddProject, useAddProjectOpen } from './model/dialog'` (задача 9 добавит `ProjectInput`).

- [ ] **Step 3: Сайдбар**

`widgets/projects-sidebar/ui/ProjectsSidebar.tsx`:

```tsx
import { useNavigate, useParams } from '@tanstack/react-router'
import type { ReactNode } from 'react'
import { findReport, type HistoryEntry } from '../../../shared/api'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { openAddProject } from '../../../features/add-project'
import { fillAggSettings } from '../../../features/build-aggregate'
import { fillLink } from '../../../features/build-by-link'
import { ClearHistoryButton, RemoveEntryButton } from '../../../features/history-actions'
import { resetSearch } from '../../../features/search-projects'
import { pickHost, useHostState } from '../../../entities/host'
import { HistoryEntryItem, useHistory } from '../../../entities/history-entry'
import { useOpenProject } from '../../../entities/project'
import { SavedProjectItem, useRemoveProject, useSavedProjects } from '../../../entities/saved-project'

function Section({ title, action, children }: { title: string; action?: ReactNode; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-semibold">{title}</h2>
        {action}
      </div>
      {children}
    </section>
  )
}

export function ProjectsSidebar() {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const navigate = useNavigate()
  const openProject = useOpenProject()
  const params = useParams({ strict: false })
  const { projects, error: projectsError } = useSavedProjects()
  const removeProject = useRemoveProject()
  const { entries, error: historyError } = useHistory()
  const { host: currentHost } = useHostState()

  // Готовый отчёт ещё в памяти — сразу на него; иначе запись заполняет сторы и ведёт на нужный экран, как раньше.
  const openEntry = async ({ host, form, request }: HistoryEntry) => {
    const cached = await findReport(host, request)
    if (cached.ok && cached.value !== null) {
      void navigate({ to: '/report/$id', params: { id: String(cached.value) } })
      return
    }
    if (form.mode === 'link') {
      fillLink(form.url)
      void navigate({ to: '/' })
      return
    }
    if (host !== currentHost) resetSearch()
    pickHost(host)
    fillAggSettings(form)
    void openProject({ host, project: form.project, branch: form.ref || undefined })
  }

  const error = projectsError ?? historyError
  return (
    <aside className="flex w-72 shrink-0 flex-col gap-5 overflow-y-auto border-r bg-card p-4">
      {error && (
        <p role="alert" className="text-sm text-destructive">
          {errorText(error)}
        </p>
      )}
      <Section
        title={t('form.sidebar.projects')}
        action={
          <Button type="button" variant="ghost" size="sm" onClick={openAddProject}>
            {t('form.sidebar.add')}
          </Button>
        }
      >
        <ul className="flex flex-col gap-0.5">
          {projects.map((project) => (
            <SavedProjectItem
              key={`${project.host}/${project.path}`}
              project={project}
              active={params.host === project.host && params._splat === project.path}
              onOpen={() => void openProject({ host: project.host, project: project.path, branch: undefined, name: project.name })}
              onRemove={() => void removeProject({ host: project.host, path: project.path })}
            />
          ))}
        </ul>
      </Section>
      <Section title={t('form.sidebar.recent')} action={entries.length > 0 && <ClearHistoryButton />}>
        {entries.length === 0 && <p className="text-sm text-muted-foreground">{t('form.history.empty')}</p>}
        <ul className="flex flex-col gap-0.5">
          {entries.map((entry) => (
            <li key={entry.at} className="flex items-center gap-1">
              <HistoryEntryItem entry={entry} onOpen={() => void openEntry(entry)} />
              <RemoveEntryButton entry={entry} />
            </li>
          ))}
        </ul>
      </Section>
    </aside>
  )
}
```

`index.ts`: `export { ProjectsSidebar } from './ui/ProjectsSidebar'`. Папку `widgets/history-sidebar/` убрать. `useParams({ strict: false })` возвращает объединение параметров всех маршрутов: `host` и `_splat` в нём есть.

- [ ] **Step 4: Шапка**

В `shared/ui/button.tsx` сделать `buttonVariants` экспортируемым (`export const buttonVariants = cva(…)`). `widgets/app-header/ui/AppHeader.tsx`:

```tsx
import { Link } from '@tanstack/react-router'
import { SettingsIcon } from 'lucide-react'
import { useTranslation } from '../../../shared/i18n'
import { cn } from '../../../shared/lib/cn'
import { buttonVariants } from '../../../shared/ui/button'
import { BuildByLink } from '../../../features/build-by-link'

export function AppHeader() {
  const { t } = useTranslation()
  return (
    <header className="flex items-start gap-3 border-b bg-card px-6 py-3">
      <div className="min-w-0 flex-1">
        <BuildByLink />
      </div>
      <Link to="/settings" aria-label={t('form.settingsButton')} className={cn(buttonVariants({ variant: 'ghost', size: 'icon' }), 'mt-6')}>
        <SettingsIcon />
      </Link>
    </header>
  )
}
```

`index.ts`: `export { AppHeader } from './ui/AppHeader'`.

- [ ] **Step 5: Домашняя страница**

`pages/home/ui/HomePage.tsx`:

```tsx
import { Navigate } from '@tanstack/react-router'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { Card } from '../../../shared/ui/card'
import { openAddProject } from '../../../features/add-project'
import { useSavedProjects } from '../../../entities/saved-project'
import { useSettings } from '../../../entities/settings'

/** Нет проектов — пустое состояние; есть — последний открытый (или первый). */
export function HomePage() {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const { projects, ready, error } = useSavedProjects()
  const { data: settings } = useSettings()
  if (error) {
    return (
      <p role="alert" className="text-sm text-destructive">
        {errorText(error)}
      </p>
    )
  }
  if (!ready || !settings) return <p className="text-sm text-muted-foreground">{t('form.loading')}</p>
  if (projects.length === 0) {
    return (
      <Card className="items-center py-16 text-center">
        <h1 className="text-xl font-semibold">{t('form.empty.title')}</h1>
        <p className="max-w-md text-sm text-muted-foreground">{t('form.empty.text')}</p>
        <Button type="button" onClick={openAddProject}>
          {t('form.empty.add')}
        </Button>
      </Card>
    )
  }
  const last = settings.lastProject
  const target = projects.find((p) => p.host === last?.host && p.path === last?.path) ?? projects[0]
  return (
    <Navigate to="/project/$host/$" params={{ host: target.host, _splat: target.path }} search={{ ref: undefined }} state={{ name: target.name }} replace />
  )
}
```

`index.ts`: `export { HomePage } from './ui/HomePage'`. Папку `pages/projects/` убрать, ключи `form.projects.title` и `form.projects.back` убрать из обоих словарей.

- [ ] **Step 6: Экран проекта запоминается**

В `pages/project/ui/ProjectPage.tsx` убрать кнопку «← Проекты» (и импорты `Button`, `useNavigate`), заголовок оставить без обёртки; добавить:

```tsx
import { useEffect } from 'react'
import { useUpdateSettings } from '../../../entities/settings'
…
  const update = useUpdateSettings()
  // страница смонтирована с key по проекту: эффект идёт один раз на проект
  useEffect(() => {
    void update({ lastProject: { host: target.host, path: target.project } })
  }, [update, target.host, target.project])
```

- [ ] **Step 7: Роутер и раскладка**

Заменить `app/src/app/form/router.tsx`:

```tsx
import { createHashHistory, createRootRoute, createRoute, createRouter, Outlet, useMatches, useNavigate, useRouter } from '@tanstack/react-router'
import { useEffect } from 'react'
import { openAddProject } from '../../features/add-project'
import { HomePage } from '../../pages/home'
import { ProjectPage } from '../../pages/project'
import { AppHeader } from '../../widgets/app-header'
import { ProjectsSidebar } from '../../widgets/projects-sidebar'

const isMac = navigator.platform.startsWith('Mac')
const mod = (e: KeyboardEvent) => (isMac ? e.metaKey : e.ctrlKey)

function Layout() {
  const router = useRouter()
  const navigate = useNavigate()
  const onReport = useMatches({ select: (matches) => matches.some((m) => m.routeId === '/report/$id') })
  const onSettings = useMatches({ select: (matches) => matches.some((m) => m.routeId === '/settings') })

  // Один обработчик клавиш на приложение. Открытый Popover/Select гасит Esc сам (`defaultPrevented`):
  // первый Esc закрывает его, второй уводит с отчёта или настроек.
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.defaultPrevented) return
      if (e.key === 'Escape' && (onReport || onSettings)) {
        if (router.history.canGoBack()) router.history.back()
        else void navigate({ to: '/' })
      } else if (mod(e) && e.shiftKey && e.key.toLowerCase() === 'n') {
        e.preventDefault()
        openAddProject()
      } else if (mod(e) && e.key === ',') {
        e.preventDefault()
        void navigate({ to: '/settings' })
      }
    }
    document.addEventListener('keydown', onKeyDown)
    return () => document.removeEventListener('keydown', onKeyDown)
  }, [router, navigate, onReport, onSettings])

  // отчёту нужна вся ширина: без сайдбара и шапки
  if (onReport) {
    return (
      <main className="h-screen overflow-y-auto">
        <Outlet />
      </main>
    )
  }
  return (
    <div className="flex h-screen">
      <ProjectsSidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <AppHeader />
        <main className="flex-1 overflow-y-auto">
          <div className="mx-auto flex max-w-4xl flex-col gap-3 p-6">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  )
}

const rootRoute = createRootRoute({ component: Layout })

const homeRoute = createRoute({ getParentRoute: () => rootRoute, path: '/', component: HomePage })

// Хост — параметр, путь проекта (`group/sub/project`) — splat, ветка — `?ref=`.
const projectRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/project/$host/$',
  validateSearch: (search: Record<string, unknown>): { ref?: string } => ({
    ref: typeof search.ref === 'string' && search.ref !== '' ? search.ref : undefined,
  }),
  component: () => {
    const { host, _splat } = projectRoute.useParams()
    const { ref } = projectRoute.useSearch()
    // key: другой проект — другая страница, прогресс, ошибки и подсказки веток прежнего не переносятся
    return <ProjectPage key={`${host}/${_splat}`} host={host} project={_splat ?? ''} branch={ref} />
  },
})

// Заглушки до задач 10 и 11.
export const reportRoute = createRoute({ getParentRoute: () => rootRoute, path: '/report/$id', component: () => null })
const settingsRoute = createRoute({ getParentRoute: () => rootRoute, path: '/settings', component: () => null })

export const router = createRouter({
  routeTree: rootRoute.addChildren([homeRoute, projectRoute, reportRoute, settingsRoute]),
  history: createHashHistory(),
})

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
```

Если `router.history.canGoBack` отсутствует в установленной версии `@tanstack/history`, заменить на `window.history.length > 1`.

- [ ] **Step 8: Steiger**

В `app/steiger.config.ts` в `files` заменить `./src/widgets/{projects-panel,pipelines-list,aggregate-block}/**` на `./src/widgets/{projects-panel,pipelines-list,aggregate-block,projects-sidebar,app-header,add-project-dialog}/**`, `./src/features/{manage-token,select-branch,history-actions,build-aggregate}/**` на `./src/features/{manage-token,select-branch,history-actions,build-aggregate,add-project,choose-theme}/**`, `./src/entities/pipeline/**` на `./src/entities/{pipeline,saved-project,settings}/**`. Комментарий: «страниц стало четыре, многие слайсы нужны только одной».

- [ ] **Step 9: Проверка и коммит**

Run: `cd app && npm run typecheck && npm run lint:fsd && npm run check:locales`
Expected: зелёные.

Run: `cd app && npm run tauri dev` → с чистым каталогом данных приложения (на macOS это `~/Library/Application Support/dev.pipeline-trace.desktop`, переименовать его перед запуском) видно пустое состояние, сайдбар с «Мои проекты» и «Недавние», шапка с полем ссылки и шестерёнкой; ⌘, ведёт на пустой экран настроек, Esc возвращает.

```bash
git add app/src app/steiger.config.ts
git commit -m "feat(web): сайдбар «Мои проекты», шапка, маршруты /report и /settings"
```

---

### Task 9: Фронтенд — диалог «Добавить проект» и блок токена

**Files:**
- Create: `app/src/shared/ui/dialog.tsx`
- Create: `app/src/features/manage-token/ui/TokenBlock.tsx`; Modify: `app/src/features/manage-token/model/useSaveToken.ts`, `app/src/features/manage-token/index.ts`
- Create: `app/src/features/add-project/model/useAddProject.ts`, `app/src/features/add-project/model/pickFolder.ts`; Modify: `app/src/features/add-project/index.ts`
- Create: `app/src/widgets/add-project-dialog/ui/AddProjectDialog.tsx`, `app/src/widgets/add-project-dialog/index.ts`
- Modify: `app/src/widgets/projects-panel/ui/ProjectsPanel.tsx` (проп `onPick`)
- Modify: `app/src/features/build-by-link/ui/BuildByLink.tsx` (проп `tokenBlock`), `app/src/widgets/app-header/ui/AppHeader.tsx`
- Modify: `app/src/app/form/router.tsx` (монтирование диалога)

**Interfaces:**
- Consumes: `api.addProject`, `features/add-project.{useAddProjectOpen, closeAddProject}`, `entities/saved-project.savedProjectsQuery`, `entities/host.{hostsQuery, useHostState}`, `entities/project.useOpenProject`, `@tauri-apps/plugin-dialog.open` (разрешение `dialog:allow-open` из задачи 6).
- Produces: `shared/ui/dialog.{Dialog, DialogContent, DialogTitle, DialogDescription}`; `features/manage-token.TokenBlock({ host, onSaved })`; `useSaveToken().save(host, token): Promise<boolean>`; `features/add-project.{useAddProject, pickFolder}`; `widgets/add-project-dialog.AddProjectDialog`; `ProjectsPanel({ host, onPick })`; `BuildByLink({ tokenBlock? })`.

- [ ] **Step 1: Диалог на Radix**

`shared/ui/dialog.tsx`:

```tsx
import { Dialog as DialogPrimitive } from 'radix-ui'
import type { ComponentProps } from 'react'
import { cn } from '../lib/cn'

export const Dialog = DialogPrimitive.Root
export const DialogTitle = ({ className, ...props }: ComponentProps<typeof DialogPrimitive.Title>) => (
  <DialogPrimitive.Title className={cn('text-lg font-semibold', className)} {...props} />
)
export const DialogDescription = ({ className, ...props }: ComponentProps<typeof DialogPrimitive.Description>) => (
  <DialogPrimitive.Description className={cn('text-sm text-muted-foreground', className)} {...props} />
)

/** Esc и клик по подложке закрывают (Radix зовёт `onOpenChange`); кнопки закрытия — у потребителя. */
export const DialogContent = ({ className, ...props }: ComponentProps<typeof DialogPrimitive.Content>) => (
  <DialogPrimitive.Portal>
    <DialogPrimitive.Overlay className="fixed inset-0 z-50 bg-black/40" />
    <DialogPrimitive.Content
      className={cn(
        'fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-full max-w-lg -translate-x-1/2 -translate-y-1/2 flex-col gap-4 overflow-y-auto rounded-xl border bg-card p-6 text-card-foreground shadow-lg outline-hidden',
        className,
      )}
      {...props}
    />
  </DialogPrimitive.Portal>
)
```

Radix Dialog при Esc вызывает `preventDefault` на событии, поэтому корневой обработчик в `Layout` его пропускает.

- [ ] **Step 2: Блок токена**

`useSaveToken.ts`: `save` возвращает `result.ok` (после `pickHost(...)` добавить `return true`, в ветке ошибки `return false`).

`features/manage-token/ui/TokenBlock.tsx`:

```tsx
import { useId, useState } from 'react'
import { messageError } from '../../../shared/api'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { Input } from '../../../shared/ui/input'
import { useSaveToken } from '../model/useSaveToken'

/** Токен для известного хоста прямо там, где он понадобился; после сохранения `onSaved` повторяет действие. */
export function TokenBlock({ host, onSaved }: { host: string; onSaved: () => void }) {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const id = useId()
  const [token, setToken] = useState('')
  const { save, error } = useSaveToken()
  const failure = messageError(error)
  // страница токенов GitLab с готовым именем и scope; `https` уходит в системный браузер через on_navigation
  const createUrl = `https://${host}/-/user_settings/personal_access_tokens?name=pipeline-trace&scopes=read_api`
  return (
    <form
      className="flex flex-col gap-2 rounded-md border border-warn bg-secondary/40 p-3"
      onSubmit={async (e) => {
        e.preventDefault()
        if (await save(host, token)) onSaved()
      }}
    >
      <p className="text-sm font-medium">{t('form.token.need', { host })}</p>
      <p className="text-xs text-muted-foreground">
        {t('form.token.scope')}{' '}
        <a href={createUrl} target="_blank" rel="noopener" className="text-primary hover:underline">
          {t('form.token.create', { host })}
        </a>
      </p>
      <label htmlFor={id} className="text-sm font-medium">
        {t('form.host.token')}
      </label>
      <Input id={id} type="password" autoComplete="new-password" autoCapitalize="off" autoCorrect="off" spellCheck={false} aria-invalid={failure !== undefined} aria-describedby={failure ? `${id}-error` : undefined} value={token} onChange={(e) => setToken(e.target.value)} />
      <p className="text-xs text-muted-foreground">{t('form.token.stored')}</p>
      {failure && (
        <p id={`${id}-error`} role="alert" className="text-sm text-destructive">
          {errorText(failure)}
        </p>
      )}
      <Button type="submit" className="self-start">
        {t('form.host.saveToken')}
      </Button>
    </form>
  )
}
```

`features/manage-token/index.ts`: добавить `export { TokenBlock } from './ui/TokenBlock'`.

- [ ] **Step 3: Фича добавления**

`features/add-project/model/pickFolder.ts`:

```ts
import { open } from '@tauri-apps/plugin-dialog'

/** Системный выбор папки; отмена и сбой диалога — `null`. */
export async function pickFolder(): Promise<string | null> {
  try {
    const chosen = await open({ directory: true, multiple: false })
    return typeof chosen === 'string' ? chosen : null
  } catch (e) {
    console.error(e)
    return null
  }
}
```

`features/add-project/model/useAddProject.ts`:

```ts
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { addProject, apiError, unwrap } from '../../../shared/api'
import { hostsQuery } from '../../../entities/host'
import { useOpenProject } from '../../../entities/project'
import { savedProjectsQuery } from '../../../entities/saved-project'
import { closeAddProject } from './dialog'

/** Разбор, проверка доступа и запись — в ядре; успех закрывает диалог и открывает проект. */
export function useAddProject() {
  const queryClient = useQueryClient()
  const openProject = useOpenProject()
  const mutation = useMutation({
    mutationFn: (input: string) => unwrap(addProject(input)),
    onSuccess: async (saved) => {
      await queryClient.invalidateQueries(savedProjectsQuery)
      await queryClient.invalidateQueries(hostsQuery) // токен нового хоста мог появиться по пути
      closeAddProject()
      void openProject({ host: saved.host, project: saved.path, branch: undefined, name: saved.name })
    },
  })
  return { add: mutation.mutate, busy: mutation.isPending, error: apiError(mutation.error), reset: mutation.reset }
}
```

`index.ts`: добавить `export { pickFolder } from './model/pickFolder'` и `export { useAddProject } from './model/useAddProject'`.

- [ ] **Step 4: Панель проектов отдаёт выбор наружу**

`widgets/projects-panel/ui/ProjectsPanel.tsx`: сигнатура `ProjectsPanel({ host, onPick }: { host: string; onPick: (project: Project) => void })`, импорт типа `Project` из `shared/api`, в `ProjectRow` — `onOpen={() => onPick(project)}`; `useOpenProject` из импорта убрать.

- [ ] **Step 5: Виджет диалога**

`widgets/add-project-dialog/ui/AddProjectDialog.tsx`:

```tsx
import { useId, useState } from 'react'
import { messageError } from '../../../shared/api'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '../../../shared/ui/dialog'
import { Input } from '../../../shared/ui/input'
import { closeAddProject, pickFolder, useAddProject, useAddProjectOpen } from '../../../features/add-project'
import { HostSelect, TokenBlock } from '../../../features/manage-token'
import { ProjectsPanel, resetSearch } from '../../../features/search-projects'
import { useHostState } from '../../../entities/host'

/** Одно поле на ссылку или путь, «Выбрать папку…», разворачиваемый поиск по хосту; токен запрашивается на месте. */
export function AddProjectDialog() {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const id = useId()
  const open = useAddProjectOpen()
  const [input, setInput] = useState('')
  const [browse, setBrowse] = useState(false)
  const { add, busy, error, reset } = useAddProject()
  const { tokenHost } = useHostState()
  const failure = messageError(error)
  const needsToken = failure?.code === 'noToken' ? failure.params.host : null

  const close = () => {
    closeAddProject()
    setInput('')
    setBrowse(false)
    reset()
  }
  const pick = async () => {
    const dir = await pickFolder()
    if (!dir) return
    setInput(dir)
    add(dir)
  }

  return (
    <Dialog open={open} onOpenChange={(next) => !next && close()}>
      <DialogContent>
        <DialogTitle>{t('form.addProject.title')}</DialogTitle>
        <DialogDescription>{t('form.addProject.hint')}</DialogDescription>
        <form
          className="flex flex-col gap-2"
          onSubmit={(e) => {
            e.preventDefault()
            add(input)
          }}
        >
          <div className="flex gap-2">
            <Input id={id} autoFocus translate="no" placeholder={t('form.addProject.placeholder')} aria-invalid={failure !== undefined && !needsToken} aria-describedby={failure ? `${id}-error` : undefined} value={input} onChange={(e) => setInput(e.target.value)} />
            <Button type="button" variant="outline" className="shrink-0" onClick={() => void pick()}>
              {t('form.addProject.pickFolder')}
            </Button>
          </div>
          {failure && !needsToken && (
            <p id={`${id}-error`} role="alert" className="text-sm text-destructive">
              {errorText(failure)}
            </p>
          )}
          {needsToken && <TokenBlock host={needsToken} onSaved={() => add(input)} />}
          <div className="flex justify-end gap-2">
            <Button type="button" variant="ghost" onClick={close}>
              {t('form.cancel')}
            </Button>
            <Button type="submit" disabled={busy || input.trim() === ''}>
              {busy ? t('form.loading') : t('form.addProject.add')}
            </Button>
          </div>
        </form>
        <button type="button" aria-expanded={browse} className="self-start text-sm text-primary hover:underline" onClick={() => setBrowse((b) => !b)}>
          {t('form.addProject.findOnHost')} {browse ? '▾' : '▸'}
        </button>
        {browse && (
          <div className="flex flex-col gap-3 border-t pt-3">
            <HostSelect onChange={resetSearch} />
            {tokenHost !== null && <ProjectsPanel host={tokenHost} onPick={(project) => add(`https://${tokenHost}/${project.fullPath}`)} />}
          </div>
        )}
      </DialogContent>
    </Dialog>
  )
}
```

`index.ts`: `export { AddProjectDialog } from './ui/AddProjectDialog'`. Виджет не может импортировать другой виджет (`fsd/forbidden-imports`), поэтому в шаге 4 `ProjectsPanel` переезжает из `widgets/projects-panel` в `features/search-projects/ui/ProjectsPanel.tsx` (он состоит из `ProjectSearch` той же фичи, `entities/project` и `shared/ui/paged-list`), экспортируется из `features/search-projects/index.ts`; папка `widgets/projects-panel` убирается, в `steiger.config.ts` `projects-panel` убирается из списка виджетов.

В `router.tsx` в `Layout` добавить `<AddProjectDialog />` рядом с `<Outlet />` в обеих ветках (импорт из `../../widgets/add-project-dialog`).

- [ ] **Step 6: Токен под полем ссылки в шапке**

`BuildByLink.tsx`: проп `tokenBlock?: (host: string, retry: () => void) => ReactNode`; после блока ошибки:

```tsx
      {failure?.code === 'noToken' && tokenBlock?.(failure.params.host, () => start(linkForm(url)))}
```

и ошибку `noToken` не показывать текстом, если есть `tokenBlock`: `{failure && !(failure.code === 'noToken' && tokenBlock) && (<p …>)}`.

`AppHeader.tsx`: `<BuildByLink tokenBlock={(host, retry) => <TokenBlock host={host} onSaved={retry} />} />`, импорт `TokenBlock` из `features/manage-token`.

- [ ] **Step 7: Проверка и коммит**

Run: `cd app && npm run typecheck && npm run lint:fsd && npm run check:locales`
Expected: зелёные.

Run: `cd app && npm run tauri dev` с чистым каталогом данных: «+ Добавить проект» → вставить `https://<хост>/<группа>/<проект>` → под полем блок «Нужен токен для <хост>» со ссылкой; ввести токен → проект в сайдбаре, открыт экран проекта. Повторно: «Выбрать папку…» с клоном → проект добавлен без вопросов. «Найти в списке проектов хоста» → список, клик добавляет. Ссылка на пайплайн незнакомого хоста в шапке → тот же блок токена → после сохранения сборка стартует сама.

```bash
git add app/src app/steiger.config.ts
git commit -m "feat(web): диалог «Добавить проект», блок токена на месте"
```

---

### Task 10: Фронтенд — отчёт в окне, повтор из «Недавних», ⌘S

**Files:**
- Create: `app/src/entities/report/model/queries.ts`; Modify: `app/src/entities/report/model/provider.tsx`, `app/src/entities/report/index.ts`
- Modify: `app/src/widgets/report-header/ui/ReportHeader.tsx`, `app/src/pages/report/ui/ReportPage.tsx`
- Modify: `app/src/entities/history-entry/model/useBuild.ts`
- Create: `app/src/app/form/report-route.tsx`; Modify: `app/src/app/form/router.tsx`

**Interfaces:**
- Consumes: `api.{report, setCurrentReport}`, `pages/report.ReportPage`.
- Produces: `entities/report.useReportById(id)`; `ReportViewProvider({ report, embedded? })` и `useReportView().embedded`; `ReportPage({ report, embedded? })`; маршрут `/report/$id` с панелью «← Назад».

- [ ] **Step 1: Запрос отчёта**

`entities/report/model/queries.ts`:

```ts
import { useQuery } from '@tanstack/react-query'
import { report, unwrap } from '../../../shared/api'

/** Кеш отчётов живёт в Rust: здесь не держим копию дольше, чем экран смонтирован. */
export const useReportById = (id: number) =>
  useQuery({ queryKey: ['report', id], queryFn: () => unwrap(report(id)), staleTime: Infinity, gcTime: 0 })
```

`entities/report/index.ts`: добавить `export { useReportById } from './model/queries'`.

- [ ] **Step 2: Признак «внутри приложения»**

`provider.tsx`: в тип `ReportView` добавить `/** отчёт на экране формы: без своего переключателя языка и заголовка окна */ embedded: boolean`; сигнатура `ReportViewProvider({ report, embedded = false, children }: { report: Report; embedded?: boolean; children: ReactNode })`; в `value` добавить `embedded` и в зависимости `useMemo`.

`ReportHeader.tsx`: `const { report, tree, embedded } = useReportView()`; эффект заголовка — `if (embedded) return` первой строкой и `embedded` в зависимостях; `{!embedded && <LanguageSwitch />}`.

`ReportPage.tsx`: `export function ReportPage({ report, embedded }: { report: Report; embedded?: boolean })` → `<ReportViewProvider report={report} embedded={embedded}>`.

- [ ] **Step 3: Сборка ведёт на отчёт**

`useBuild.ts`: импорт `useNavigate` из `@tanstack/react-router`; `const navigate = useNavigate()`; `onSuccess: (id) => { void queryClient.invalidateQueries(historyQuery); void navigate({ to: '/report/$id', params: { id: String(id) } }) }`. Комментарий: «успех ведёт на экран отчёта: прогресс строки или кнопки гаснет вместе с размонтированием».

- [ ] **Step 4: Маршрут отчёта**

`app/src/app/form/report-route.tsx`:

```tsx
import { useNavigate, useParams, useRouter } from '@tanstack/react-router'
import { useEffect } from 'react'
import { apiError, setCurrentReport } from '../../shared/api'
import { useTranslation } from '../../shared/i18n'
import { showError } from '../../shared/lib/dialogs'
import { Button } from '../../shared/ui/button'
import { useReportById } from '../../entities/report'
import { ReportPage } from '../../pages/report'

/** Отчёт из памяти ядра; ядро узнаёт, что он на экране (⌘S). Вытесненный или битый id — ошибка и домой. */
export function ReportRoute() {
  const { t } = useTranslation()
  const { id } = useParams({ from: '/report/$id' })
  const reportId = Number(id)
  const navigate = useNavigate()
  const router = useRouter()
  const { data, error, isPending } = useReportById(reportId)

  useEffect(() => {
    void setCurrentReport(reportId)
    return () => void setCurrentReport(null)
  }, [reportId])

  const failure = apiError(error)
  useEffect(() => {
    if (!failure) return
    void showError(failure).then(() => navigate({ to: '/', replace: true }))
  }, [failure, navigate])

  const back = () => {
    if (router.history.canGoBack()) router.history.back()
    else void navigate({ to: '/' })
  }

  if (isPending || !data) return <p className="p-6 text-sm text-muted-foreground">{t('form.reportView.loading')}</p>
  const title = `${data.meta.project} · ${data.meta.label ?? t('report.allPipelines')}`
  return (
    <>
      <div className="sticky top-0 z-10 flex items-center gap-3 border-b bg-card px-4 py-2">
        <Button type="button" variant="ghost" size="sm" onClick={back}>
          {t('form.reportView.back')}
        </Button>
        <h1 translate="no" className="truncate text-sm font-semibold">
          {title}
        </h1>
      </div>
      <ReportPage report={data} embedded />
    </>
  )
}
```

В `router.tsx`: `import { ReportRoute } from './report-route'`, у `reportRoute` — `component: ReportRoute`, слово «Заглушки» в комментарии оставить только про `settingsRoute`.

- [ ] **Step 5: Проверка и коммит**

Run: `cd app && npm run typecheck && npm run lint:fsd`
Expected: зелёные (Steiger: `app/` может импортировать всё).

Run: `cd app && npm run tauri dev`: клик по пайплайну → отчёт в этом же окне без сайдбара, сверху «← Назад» и заголовок; водопад, панель деталей, клавиши работают; переключателя языка в шапке отчёта нет; ⌘S → диалог сохранения с именем `pipeline-trace-<проект>-<id>.html`, файл открывается в браузере; «← Назад» и Esc возвращают на экран проекта; клик по той же записи в «Недавних» открывает отчёт мгновенно, без «Загружаю…»; после 11 сборок первая запись в «Недавних» снова строит с нуля. Тёмная тема в настройках системы → отчёт внутри тёмный.

```bash
git add app/src
git commit -m "feat(web): отчёт в главном окне, повтор из «Недавних» из памяти"
```

---

### Task 11: Фронтенд — экран настроек

**Files:**
- Create: `app/src/features/choose-theme/ui/ThemeSwitch.tsx`, `app/src/features/choose-theme/index.ts`
- Modify: `app/src/features/manage-token/ui/TokenForm.tsx` (проп `onSaved`)
- Create: `app/src/pages/settings/ui/SettingsPage.tsx`, `app/src/pages/settings/index.ts`
- Modify: `app/src/app/form/router.tsx`

**Interfaces:**
- Consumes: `entities/settings.{useSettings, useUpdateSettings}`, `shared/lib/theme.applyTheme`, `entities/host.hostsQuery` (`HostInfo[]`), `features/manage-token.{TokenForm, RemoveTokenButton}`, `widgets/language-switch.LanguageSwitch`.
- Produces: `features/choose-theme.ThemeSwitch`; `pages/settings.SettingsPage`; `TokenForm({ defaultHost, onSaved? })`.

- [ ] **Step 1: Переключатель темы**

`features/choose-theme/ui/ThemeSwitch.tsx`:

```tsx
import type { Theme } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { applyTheme } from '../../../shared/lib/theme'
import { Button } from '../../../shared/ui/button'
import { useSettings, useUpdateSettings } from '../../../entities/settings'

const THEMES: Theme[] = ['system', 'light', 'dark']

/** Тема применяется сразу и сохраняется в settings.json. */
export function ThemeSwitch() {
  const { t } = useTranslation()
  const { data } = useSettings()
  const update = useUpdateSettings()
  const current = data?.theme ?? 'system'
  const choose = async (theme: Theme) => {
    applyTheme(theme)
    await update({ theme })
  }
  return (
    <div role="group" aria-label={t('form.settings.theme')} className="flex gap-0.5">
      {THEMES.map((theme) => (
        <Button key={theme} type="button" variant={current === theme ? 'secondary' : 'ghost'} size="sm" aria-pressed={current === theme} onClick={() => void choose(theme)}>
          {t(`form.settings.themes.${theme}`)}
        </Button>
      ))}
    </div>
  )
}
```

`index.ts`: `export { ThemeSwitch } from './ui/ThemeSwitch'`.

- [ ] **Step 2: `TokenForm` сообщает об успехе**

`TokenForm.tsx`: пропы `{ defaultHost, onSaved }: { defaultHost: string; onSaved?: () => void }`; `onSubmit`: `if (await save(host, token)) onSaved?.()`.

- [ ] **Step 3: Страница настроек**

`pages/settings/ui/SettingsPage.tsx`:

```tsx
import { useQuery } from '@tanstack/react-query'
import { useNavigate, useRouter } from '@tanstack/react-router'
import { useState } from 'react'
import { apiError } from '../../../shared/api'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { Card } from '../../../shared/ui/card'
import { ThemeSwitch } from '../../../features/choose-theme'
import { RemoveTokenButton, TokenForm } from '../../../features/manage-token'
import { hostsQuery } from '../../../entities/host'
import { LanguageSwitch } from '../../../widgets/language-switch'

export function SettingsPage() {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const navigate = useNavigate()
  const router = useRouter()
  const { data: hosts = [], error } = useQuery(hostsQuery)
  const [adding, setAdding] = useState(false)
  const failure = apiError(error)
  const back = () => {
    if (router.history.canGoBack()) router.history.back()
    else void navigate({ to: '/' })
  }
  return (
    <Card>
      <div className="flex items-center gap-3">
        <Button type="button" variant="ghost" size="sm" onClick={back}>
          {t('form.settings.back')}
        </Button>
        <h1 className="text-xl font-semibold">{t('form.settings.title')}</h1>
      </div>

      <section className="flex flex-col gap-2">
        <h2 className="text-sm font-semibold">{t('form.settings.hosts')}</h2>
        {failure && (
          <p role="alert" className="text-sm text-destructive">
            {errorText(failure)}
          </p>
        )}
        {hosts.length === 0 && !failure && <p className="text-sm text-muted-foreground">{t('form.settings.noHosts')}</p>}
        <ul className="flex flex-col gap-1.5">
          {hosts.map((h) => (
            <li key={h.host} className="flex items-center gap-3 rounded-md border px-3 py-2 text-sm">
              <span translate="no" className="flex-1 truncate font-medium">
                {h.host}
              </span>
              <span className="text-xs text-muted-foreground">{t(`form.settings.source.${h.source}`)}</span>
              {h.source === 'keychain' && <RemoveTokenButton host={h.host} />}
            </li>
          ))}
        </ul>
        {adding ? (
          <TokenForm defaultHost="" onSaved={() => setAdding(false)} />
        ) : (
          <Button type="button" variant="outline" className="self-start" onClick={() => setAdding(true)}>
            {t('form.settings.addHost')}
          </Button>
        )}
        <p className="text-xs text-muted-foreground">{t('errors.keychainUnavailable')}</p>
      </section>

      <section className="flex flex-col gap-2">
        <h2 className="text-sm font-semibold">{t('form.settings.theme')}</h2>
        <ThemeSwitch />
      </section>

      <section className="flex flex-col gap-2">
        <h2 className="text-sm font-semibold">{t('form.language')}</h2>
        <LanguageSwitch />
      </section>
    </Card>
  )
}
```

Подсказка про Linux без Secret Service показывается только на Linux: обернуть `<p>` в `navigator.platform.startsWith('Linux') && …`.

`index.ts`: `export { SettingsPage } from './ui/SettingsPage'`. В `router.tsx`: `settingsRoute` → `component: SettingsPage`, комментарий про заглушки убрать.

- [ ] **Step 4: Проверка и коммит**

Run: `cd app && npm run typecheck && npm run lint:fsd && npm run check:locales`
Expected: зелёные.

Run: `cd app && npm run tauri dev`: шестерёнка → таблица хостов с источником; «Удалить токен» только у связки, с подтверждением; «Добавить хост» → форма, после сохранения хост в списке; «Тёмная» → приложение темнеет сразу, после перезапуска тема сохранена; «Системная» следует переключению темы ОС; RU/EN меняет язык и меню; Esc возвращает.

```bash
git add app/src
git commit -m "feat(web): экран настроек — хосты, тема, язык"
```

---

### Task 12: Документы, чек-лист приёмки, мёртвый код

**Files:**
- Modify: `README.md` (раздел «Приложение»)
- Modify: `docs/specs/2026-10-03-tauri-rewrite-design.md` § 12
- Modify: `docs/specs/2026-10-03-tauri-rewrite-acceptance.md` (новый раздел)
- Modify: `app/src/shared/i18n/locales/ru.json`, `en.json` (неиспользуемые ключи)
- Modify: `app/steiger.config.ts`

- [ ] **Step 1: README**

В `README.md` раздел «Приложение» заменить список возможностей:

```markdown
Форма строит отчёт по ссылке или по проекту из списка «Мои проекты». Приложение умеет:

- ссылку на пайплайн или MR — вставляешь в поле в шапке и жмёшь «Построить»;
- «Мои проекты» в сайдбаре: добавление по ссылке на репозиторий, пайплайн или MR, по ssh-URL, по папке с клоном (читается `remote.origin.url`) или из списка проектов хоста; при добавлении проверяется доступ;
- токен запрашивается там, где понадобился: под полем ссылки или в диалоге добавления, со ссылкой на создание Personal Access Token (scope `read_api`); токены из `glab` и `GITLAB_TOKEN` приложение видит само;
- экран проекта — ветка (с подсказками из списка веток), последние пайплайны с переходом в отчёт по клику и агрегат по N пайплайнам с фильтром по статусам;
- отчёт открывается в том же окне: «← Назад» или Esc возвращают к проекту, `Cmd/Ctrl+S` сохраняет HTML; последние 10 отчётов держатся в памяти, повтор из «Недавних» открывается без похода в GitLab;
- при запуске открывается последний проект;
- «Недавние» — история последних 20 запросов с повтором по клику, удалением записи или очисткой;
- настройки (шестерёнка, `Cmd/Ctrl+,`): хосты и токены с источником (связка ключей, `glab`, окружение), тема (системная, светлая, тёмная), язык RU/EN;
- агрегат по статусам: пайплайн, остановившийся на ручной джобе, GitLab помечает `manual`; короткие служебные `success`-пайплайны (например, Publish-коммиты) смешиваются с полными — чтобы их отсечь, оставь только MANUAL.
```

Абзац «Токены, история и настройки хранятся в каталоге данных приложения…» дополнить: «…и список проектов (`projects.json`)».

- [ ] **Step 2: Спека Tauri § 12**

В таблицу § 12 `docs/specs/2026-10-03-tauri-rewrite-design.md` добавить строки:

```markdown
| Отчёт в отдельном окне `report-<n>` без IPC, схема `report://` | Отчёт в главном окне по `/report/$id`, последние 10 в памяти — `docs/specs/2026-10-06-first-run-ux-design.md` |
| Поиск по всем проектам хоста на первом экране | «Мои проекты» и диалог добавления, поиск по хосту внутри диалога — та же спека |
| Переключатель языка в шапке, тема только системная | Экран настроек: язык, тема, хосты — та же спека |
```

- [ ] **Step 3: Чек-лист приёмки**

В конец `docs/specs/2026-10-03-tauri-rewrite-acceptance.md` добавить раздел:

```markdown
## 7. Первый запуск: «Мои проекты», отчёт в окне, настройки

Спека — `docs/specs/2026-10-06-first-run-ux-design.md`. Проходится на macOS в dev-сборке с чистым каталогом данных приложения; пункты `(смоук)` — на Windows и Linux.

### 7.1. Пустое состояние и добавление

- [ ] Чистый каталог данных → в центре «Добавь первый проект» с кнопкой, в сайдбаре «Мои проекты» пусто, «Недавние» — «Пока пусто», в шапке поле ссылки и шестерёнка. (смоук)
- [ ] «+ Добавить проект» → диалог с одним полем, «Выбрать папку…», «Найти в списке проектов хоста».
- [ ] Вставить `https://<хост>/<группа>/<проект>` при отсутствии токена → под полем «Нужен токен для <хост>», ссылка «Создать на <хост> ↗» открывает системный браузер на странице токенов с именем `pipeline-trace` и scope `read_api`.
- [ ] Ввести неверный токен → «GitLab <хост> ответил 401…», введённая ссылка на месте.
- [ ] Ввести верный токен → проект в «Мои проекты», открыт экран проекта, диалог закрыт. (смоук)
- [ ] Ссылка на несуществующий проект → «Проект … не найден или нет доступа».
- [ ] «foo» → «Не похоже на ссылку на проект GitLab или путь к папке».
- [ ] «Выбрать папку…» с клоном (https- и ssh-remote) → проект добавлен; папка без git → «В папке нет git-репозитория с remote origin».
- [ ] «Найти в списке проектов хоста» → хост, поиск, «Показать ещё»; клик по строке добавляет проект.
- [ ] Повторное добавление того же проекта → дубля нет, проект на прежнем месте.
- [ ] Крестик у проекта в сайдбаре → проект убран без подтверждения; доступное имя «Убрать из списка: <имя>».
- [ ] ⌘⇧N открывает диалог, Esc закрывает его (второй Esc ничего не делает на экране проекта).
- [ ] Ссылка на пайплайн незнакомого хоста в шапке → блок токена под полем; после сохранения сборка стартует сама.

### 7.2. Отчёт в окне

- [ ] Клик по пайплайну → отчёт в том же окне без сайдбара, сверху «← Назад» и `<проект> · #<iid>`; переключателя RU/EN в шапке отчёта нет. (смоук)
- [ ] Водопад, панель деталей, клавиши `[`, `]`, `0`, ↑/↓, Enter, ←/→, Esc (снятие выделения) — как в разделе 2.
- [ ] «← Назад» и Esc (без выделенной строки) → экран проекта, список пайплайнов на месте.
- [ ] ⌘S на отчёте → диалог сохранения с именем `pipeline-trace-<проект>-<суффикс>.html`; файл открывается в браузере, в нём есть RU/EN. (смоук)
- [ ] ⌘S на экране проекта или настроек → ничего не происходит.
- [ ] Клик по записи в «Недавних» сразу после сборки → отчёт мгновенно, без «Загружаю…»; после 11 новых сборок самая старая запись строит заново с прогрессом.
- [ ] Язык RU→EN в настройках → открытый затем отчёт на английском; тема «Тёмная» → отчёт тёмный; сохранённый файл следует теме системы.
- [ ] Ссылка «GitLab ↗» в панели деталей открывает системный браузер.

### 7.3. Настройки и запуск

- [ ] Шестерёнка и ⌘, → экран настроек; Esc и «← Назад» возвращают. (смоук)
- [ ] Хосты: источник «связка ключей» с кнопкой «Удалить токен» и подтверждением; хост из `glab` — источник «glab» без кнопки; `GITLAB_TOKEN`+`GITLAB_HOST` — «переменные окружения» без кнопки.
- [ ] «Добавить хост» → форма хост+токен; после сохранения хост в таблице.
- [ ] Тема: «Светлая» и «Тёмная» применяются сразу и переживают перезапуск; «Системная» следует смене темы ОС на лету.
- [ ] Перезапуск приложения → открыт последний открытый проект; убрать его из списка и перезапустить → открыт первый из оставшихся; убрать все → пустое состояние. (смоук)
- [ ] `settings.json` содержит `locale`, `theme`, `lastProject`; `projects.json` — `host`, `path`, `name`, `addedAt`; токенов в них нет.
- [ ] Linux без Secret Service: под таблицей хостов подсказка про `GITLAB_TOKEN`/libsecret, добавление по ссылке с вводом токена даёт «Системная связка ключей недоступна…».
```

- [ ] **Step 4: Неиспользуемые ключи словарей**

Найти ключи `form.*`, на которые больше нет ссылок:

```bash
cd app && node -e '
const flat=(o,p="")=>Object.entries(o).flatMap(([k,v])=>v&&typeof v==="object"?flat(v,p+k+"."):[p+k]);
const ru=require("./src/shared/i18n/locales/ru.json");
for (const k of flat(ru)) { const base=k.replace(/_(one|few|many|other)$/,""); process.stdout.write(base+"\n") }' | sort -u | while read k; do grep -rqF "'$k'" src || grep -rqF "\"$k\"" src || grep -rqF "\`$k" src || echo "unused: $k"; done
```

Ожидаемо не используются `form.host.hint` (удалить из обоих словарей). Коды ошибок (`errors.*`) не проверяются этим скриптом — они нужны все. Ключи с шаблоном (`form.settings.themes.*`, `form.settings.source.*`) скрипт пометит как неиспользуемые ложно — оставить.

Run: `cd app && npm run typecheck && npm run check:locales` → зелёные.

- [ ] **Step 5: Steiger и запахи**

`steiger.config.ts`: убедиться, что список исключений содержит только существующие слайсы (`history-sidebar`, `projects-panel` удалены). Пройти по диффу ветки по `docs/agents/code-smells.md`; найденное и оставленное — в описание PR.

Run: `cd app && npm run lint:fsd && npm run build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p pipeline-trace-core`
Expected: зелёные.

- [ ] **Step 6: Commit и PR**

```bash
git add README.md docs app/src app/steiger.config.ts
git commit -m "docs: первый запуск — README, отступления спеки Tauri, чек-лист приёмки"
```

Дальше — по скиллу `superpowers:finishing-a-development-branch`: PR в `main` из ветки с описанием по разделам спеки и списком оставленных запахов.

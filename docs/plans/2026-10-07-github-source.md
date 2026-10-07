# GitHub Actions как второй источник — план реализации

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Отчёт pipeline-trace строится и по GitHub Actions (github.com и GHES): ссылка на run или PR, проекты GitHub в «Моих проектах», агрегат по N запускам одного workflow.

**Architecture:** В ядре появляется трейт `Source` (`core/src/source.rs`). GitLab реализует его для любого `Gql` через blanket-impl — существующий код и тесты GitLab не меняются. GitHub — новый модуль `core/src/github*` с REST-клиентом; run, его джобы и файл workflow превращаются в существующий `RawPipeline`, поэтому `model`, `critical_path`, `aggregate`, `insights`, `render` и фронтенд отчёта не трогаются. Тип хоста (`Provider`) выводится из хоста: `github.com`, кэш `hostKinds` в `settings.json` или проба `/api/v3/meta`.

**Tech Stack:** Rust 2024 (`reqwest` 0.13, `serde`, `time`, `regex`, `ts-rs`, `wiremock` в тестах), новая зависимость `serde_yaml_ng`; Tauri 2; React 19, TanStack Router/Query, i18next.

Спека — `docs/specs/2026-10-07-github-source-design.md` (далее «спека, § N»). Правила репозитория: `docs/agents/code-smells.md` — проход по диффу перед каждым коммитом; комментарии, тексты и имена тестов — по-русски, как в соседнем коде.

## Global Constraints

- Все команды — из `app/`. Ядро: `cargo test -p pipeline-trace-core`. Приложение: `npm run build && cargo test -p pipeline-trace` (`build.rs` требует `dist-report/report.html`). Перед коммитом: `cargo fmt --all --check`; перед пушем: `npm run build && cargo clippy --workspace --all-targets -- -D warnings`.
- Фронтенд: `npm run typecheck`, `npm run lint:fsd`, `npm run check:locales`. Автотестов фронтенда нет: проверка — `npm run dev` на моке (`src/app/dev/mock.ts`), `http://localhost:5173/`.
- Типы для фронтенда генерирует ts-rs при `cargo test -p pipeline-trace-core` в `src/shared/api/schema/`; сгенерированные файлы коммитятся вместе с изменением Rust-типа.
- Существующие тесты GitLab проходят без правок их данных; правятся только литералы структур, получивших новое поле (перечислено в задачах).
- Тип хоста не хранится в `projects.json`, истории и форме (спека, § 3).
- Заголовки GitHub API: `Authorization: Bearer <token>`, `Accept: application/vnd.github+json`, `X-GitHub-Api-Version: 2022-11-28`, `User-Agent: pipeline-trace`; редиректы выключены; не больше 4 параллельных запросов; таймауты соединения 15 с, чтения 60 с.
- Пределы: джобы run — не больше 50 страниц по 100; список запусков агрегата — не больше 10 страниц по 100.
- Коммиты — на ветке `Phobym/github`, сообщения по образцу истории: `feat(core): …`, `feat(tauri): …`, `feat(web): …`, `docs: …`.
- `lefthook` может отсутствовать в PATH: тогда проверки из `lefthook.yml` запускаются вручную перед коммитом.

---

### Task 1: Шов `Source` и `Provider` (только GitLab, поведение не меняется)

**Files:**
- Create: `app/core/src/source.rs`
- Create: `app/core/tests/source.rs`
- Modify: `app/core/src/lib.rs`, `app/core/src/report.rs`, `app/core/src/browse.rs`, `app/core/src/gitlab.rs`, `app/core/src/model.rs`
- Modify (тесты): `app/core/tests/fixtures.rs`, `app/core/tests/browse.rs`
- Modify (мок): `app/src/app/dev/mock.ts`
- Generated: `app/src/shared/api/schema/Provider.ts`, `Workflow.ts`, `Pipeline.ts`

**Interfaces:**
- Produces:
  - `pipeline_trace_core::source::Provider { Gitlab, Github }` — serde/ts `"gitlab" | "github"`, derive `Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS`.
  - `pipeline_trace_core::source::Source` — трейт ниже; `impl<G: Gql> Source for G`.
  - `pipeline_trace_core::browse::Workflow { file: String, name: String }`.
  - `browse::Pipeline` получает поле `url: String` — адрес пайплайна в вебе, по нему фронтенд строит отчёт.
  - `gitlab::PipelineFilter` получает поле `workflow: Option<String>` (GitLab его игнорирует).
  - `model::RawPipeline` получает поле `needs_missing: bool` (GitLab — всегда `false`).

- [ ] **Step 1: Тест шва — падает (нет модуля `source`)**

`app/core/tests/source.rs`:

```rust
//! Шов `Source`: GitLab через любой `Gql`.
mod fixtures;

use std::sync::Mutex;

use fixtures::fake_gql;
use pipeline_trace_core::source::{Provider, Source};
use serde_json::json;

#[tokio::test]
async fn gitlab_номер_из_ссылки_становится_gid_а_gid_не_меняется() {
    let seen = Mutex::new(Vec::new());
    let gql = fake_gql(|v| {
        seen.lock().unwrap().push(v["id"].clone());
        json!({ "project": { "pipeline": null } })
    });
    let _ = gql.fetch_pipeline("g/p", "5").await;
    let _ = gql.fetch_pipeline("g/p", "gid://gitlab/Ci::Pipeline/6").await;
    assert_eq!(
        *seen.lock().unwrap(),
        vec![json!("gid://gitlab/Ci::Pipeline/5"), json!("gid://gitlab/Ci::Pipeline/6")]
    );
}

#[tokio::test]
async fn gitlab_провайдер_и_пустой_список_workflow() {
    let gql = fake_gql(|_| json!({}));
    assert_eq!(gql.provider(), Provider::Gitlab);
    assert_eq!(gql.list_workflows("g/p").await.unwrap(), vec![]);
}
```

В `app/core/tests/browse.rs`, тест `recent_pipelines_преобразование_полей_и_курсор`: в первый узел ответа добавить `"path": "/g/p/-/pipelines/1026324"`, во второй — `"path": null`; в ожидаемые `Pipeline` — `url: "https://h.example/g/p/-/pipelines/1026324".into()` и `url: String::new()`.

- [ ] **Step 2: Убедиться, что тесты падают**

Run: `cargo test -p pipeline-trace-core --test source --test browse`
Expected: ошибка компиляции — `unresolved import pipeline_trace_core::source`, у `Pipeline` нет поля `url`.

- [ ] **Step 3: `source.rs`**

`app/core/src/source.rs`:

```rust
//! Источник пайплайнов — GitLab или GitHub (спека, § 6). Отчёт и команды формы работают через
//! `Source` и о провайдере не знают.

use std::future::Future;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::browse::{self, Page, Pipeline, Project, Workflow};
use crate::error::Error;
use crate::gitlab::{self, Gql, Listed, PipelineFilter};
use crate::model::RawPipeline;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Provider {
    Gitlab,
    Github,
}

/// Операции, которые нужны отчёту и экранам формы.
pub trait Source: Sync {
    fn host(&self) -> &str;

    fn provider(&self) -> Provider;

    /// `id` — то, что отдали `list_pipelines` и `head_pipeline`, или номер из ссылки.
    fn fetch_pipeline(
        &self,
        project: &str,
        id: &str,
    ) -> impl Future<Output = Result<RawPipeline, Error>> + Send;

    fn list_pipelines(
        &self,
        project: &str,
        filter: &PipelineFilter,
    ) -> impl Future<Output = Result<Listed, Error>> + Send;

    /// Пайплайн MR (GitLab) или самый долгий run head-коммита PR (GitHub).
    fn head_pipeline(
        &self,
        project: &str,
        number: &str,
    ) -> impl Future<Output = Result<String, Error>> + Send;

    fn fetch_project(&self, path: &str) -> impl Future<Output = Result<Project, Error>> + Send;

    fn list_projects(
        &self,
        search: &str,
        after: Option<&str>,
    ) -> impl Future<Output = Result<Page<Project>, Error>> + Send;

    fn list_branches(
        &self,
        project: &str,
        search: &str,
    ) -> impl Future<Output = Result<Vec<String>, Error>> + Send;

    fn recent_pipelines(
        &self,
        project: &str,
        r#ref: Option<&str>,
        workflow: Option<&str>,
        after: Option<&str>,
    ) -> impl Future<Output = Result<Page<Pipeline>, Error>> + Send;

    /// У GitLab workflow нет — пустой список.
    fn list_workflows(
        &self,
        project: &str,
    ) -> impl Future<Output = Result<Vec<Workflow>, Error>> + Send;
}

/// GitLab — любой `Gql`: и HTTP-клиент, и фейк тестов.
impl<G: Gql> Source for G {
    fn host(&self) -> &str {
        Gql::host(self)
    }

    fn provider(&self) -> Provider {
        Provider::Gitlab
    }

    async fn fetch_pipeline(&self, project: &str, id: &str) -> Result<RawPipeline, Error> {
        // номер из ссылки — в gid; список и MR уже отдают gid
        let gid = if id.starts_with("gid://") {
            id.to_string()
        } else {
            format!("gid://gitlab/Ci::Pipeline/{id}")
        };
        gitlab::fetch_pipeline(self, project, &gid).await
    }

    async fn list_pipelines(&self, project: &str, filter: &PipelineFilter) -> Result<Listed, Error> {
        gitlab::list_pipelines(self, project, filter).await
    }

    async fn head_pipeline(&self, project: &str, number: &str) -> Result<String, Error> {
        gitlab::mr_head_pipeline(self, project, number).await
    }

    async fn fetch_project(&self, path: &str) -> Result<Project, Error> {
        browse::fetch_project(self, path).await
    }

    async fn list_projects(&self, search: &str, after: Option<&str>) -> Result<Page<Project>, Error> {
        browse::list_projects(self, search, after).await
    }

    async fn list_branches(&self, project: &str, search: &str) -> Result<Vec<String>, Error> {
        browse::list_branches(self, project, search).await
    }

    async fn recent_pipelines(
        &self,
        project: &str,
        r#ref: Option<&str>,
        _workflow: Option<&str>,
        after: Option<&str>,
    ) -> Result<Page<Pipeline>, Error> {
        browse::recent_pipelines(self, project, r#ref, after).await
    }

    async fn list_workflows(&self, _project: &str) -> Result<Vec<Workflow>, Error> {
        Ok(Vec::new())
    }
}
```

`app/core/src/lib.rs`: добавить `pub mod source;` по алфавиту (после `pub mod settings;`).

- [ ] **Step 4: `Workflow`, `Pipeline.url` в `browse.rs`**

После `pub struct Pipeline` добавить в него последнее поле и новый тип:

```rust
    pub author: Option<String>,
    /// страница пайплайна в вебе: по ней строится отчёт
    pub url: String,
}

/// Workflow GitHub Actions; `file` — имя файла (`ci.yml`), им фильтруют запуски.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Workflow {
    pub file: String,
    pub name: String,
}
```

В `PIPELINES_QUERY` в `nodes { … }` добавить `path`:

```rust
      nodes { id iid status source createdAt duration path commit { shortId title } user { username } }
```

В `WireRecentPipeline` добавить поле `path: Option<String>,` (после `duration`). В `recent_pipelines` взять хост до замыкания и заполнить `url`:

```rust
    let host = gql.host().to_string();
    Ok(Page::from_connection(found.pipelines, |p| Pipeline {
        // … поля как раньше …
        author: p.user.map(|u| u.username),
        url: p.path.map(|path| format!("https://{host}{path}")).unwrap_or_default(),
    }))
```

- [ ] **Step 5: `PipelineFilter.workflow`, `RawPipeline.needs_missing`**

`app/core/src/gitlab.rs`, `PipelineFilter` — последнее поле:

```rust
    pub last: usize,
    /// файл workflow GitHub; GitLab его не читает
    pub workflow: Option<String>,
}
```

`app/core/src/model.rs`, `RawPipeline` — последнее поле:

```rust
    pub downstream: HashMap<String, RawPipeline>,
    /// файл workflow GitHub не загрузился: стейджи и `needs` неизвестны
    pub needs_missing: bool,
}
```

Литералы `RawPipeline { … }` дополнить `needs_missing: false`: `app/core/src/gitlab.rs` (в `fetch_at_depth`), `app/core/tests/fixtures.rs` (в `raw_pipeline`). Литералы `PipelineFilter { … }` дополнить `workflow: None`: `app/core/src/report.rs` и все в `app/core/tests/gitlab.rs` (найти: `grep -n "PipelineFilter {" app/core/tests/gitlab.rs`).

- [ ] **Step 6: `report.rs` через `Source`**

Заменить импорт `use crate::gitlab::{Gql, PipelineFilter, fetch_pipeline, list_pipelines, mr_head_pipeline};` на:

```rust
use crate::gitlab::PipelineFilter;
use crate::source::Source;
```

`resolve`:

```rust
async fn resolve(source: &impl Source, request: &Request) -> Result<Target, Error> {
    match request {
        Request::Pipeline { pipeline_id, .. } => Ok(Target {
            ids: vec![pipeline_id.clone()],
            suffix: pipeline_id.clone(),
            status_counts: None,
        }),
        Request::Mr { project, mr_iid } => Ok(Target {
            ids: vec![source.head_pipeline(project, mr_iid).await?],
            suffix: format!("mr{mr_iid}"),
            status_counts: None,
        }),
        Request::Aggregate {
            project,
            r#ref,
            source: event,
            last,
            statuses,
        } => {
            let filter = PipelineFilter {
                r#ref: r#ref.clone(),
                source: event.clone(),
                statuses: statuses
                    .as_ref()
                    .map(|list| list.iter().map(|s| s.as_str().to_string()).collect()),
                last: *last as usize,
                workflow: None,
            };
            let listed = source.list_pipelines(project, &filter).await?;
            if listed.ids.is_empty() {
                return Err(Error::new(ErrorCode::NoPipelines));
            }
            Ok(Target {
                ids: listed.ids,
                suffix: r#ref
                    .as_deref()
                    .or(event.as_deref())
                    .unwrap_or("all")
                    .to_string(),
                status_counts: Some(listed.counts),
            })
        }
    }
}
```

`load_trees` и `build_report`: параметр `gql: &impl Gql` → `source: &impl Source`; `fetch_pipeline(gql, project, id)` → `source.fetch_pipeline(project, id)`; `gql.host()` → `source.host()`; `resolve(gql, request)` → `resolve(source, request)`; `load_trees(gql, …)` → `load_trees(source, …)`. Doc-комментарий `build_report`: «Хост берётся у `source`: он один и в запросах, и в ссылках отчёта.»

- [ ] **Step 7: Мок фронтенда**

`app/src/app/dev/mock.ts`, в `.map(...)` пайплайнов добавить поле:

```ts
  url: `https://${HOST}/g/p/-/pipelines/${iid}`,
```

- [ ] **Step 8: Тесты зелёные, типы сгенерированы**

Run: `cd app && cargo test -p pipeline-trace-core`
Expected: PASS, все тесты; в `src/shared/api/schema/` появились `Provider.ts`, `Workflow.ts`, в `Pipeline.ts` — поле `url`.

Run: `cd app && npm run typecheck`
Expected: без ошибок.

- [ ] **Step 9: Commit**

```bash
git add app/core app/src/shared/api/schema app/src/app/dev/mock.ts
git commit -m "feat(core): шов Source и Provider, url пайплайна в списке"
```

### Task 2: Разбор файла workflow — уровни `needs` и сопоставление имён

**Files:**
- Create: `app/core/src/github.rs` (пока только объявление модуля)
- Create: `app/core/src/github/workflow.rs` (с юнит-тестами внутри: типы `pub(crate)`)
- Modify: `app/core/src/lib.rs`, `app/core/Cargo.toml`, `docs/specs/2026-10-07-github-source-design.md`

**Interfaces:**
- Produces (`crate::github::workflow`, всё `pub(crate)`):
  - `fn parse(text: &str) -> Option<Workflow>` — `None`: не YAML или нет `jobs`.
  - `struct Workflow` с методами `fn find(&self, api_name: &str) -> Option<Matched<'_>>` и `fn stage_names(&self) -> Vec<String>` (индекс — уровень).
  - `struct Job { key: String, needs: Vec<String>, allow_failure: bool, level: usize, .. }` — поля `key`, `needs`, `allow_failure`, `level` — `pub(crate)`.
  - `struct Matched<'a> { job: &'a Job, name: String }` — `name` — имя джобы для отчёта (шарды matrix — в форме `X: [..]`).

- [ ] **Step 1: Зависимость и пустые модули**

Run: `cd app && cargo add serde_yaml_ng -p pipeline-trace-core`

`app/core/src/github.rs`:

```rust
//! GitHub Actions: workflow run как пайплайн (спека, § 2), списки для формы (§ 6).

pub(crate) mod workflow;
```

`app/core/src/lib.rs`: `pub mod github;` после `pub mod gitlab;`.

- [ ] **Step 2: Тесты — падают**

`app/core/src/github/workflow.rs` — сначала только тесты и сигнатуры с `todo!()`:

```rust
//! Файл workflow GitHub Actions: `needs`, уровни джоб и сопоставление имён джоб API с джобами YAML
//! (спека, § 2.4–2.5).

#[cfg(test)]
mod tests {
    use super::*;

    const YAML: &str = r#"
on: push
jobs:
  build:
    runs-on: ubuntu-latest
  lint:
    name: Lint code
    continue-on-error: true
  test:
    needs: build
    strategy:
      matrix:
        os: [linux, mac]
  e2e:
    name: 'e2e ${{ matrix.shard }} on linux'
    needs: [build]
    strategy:
      matrix:
        shard: [1, 2]
  call:
    needs: [test, e2e, lint]
    uses: ./.github/workflows/deploy.yml
  dynamic:
    name: '${{ inputs.title }}'
    continue-on-error: ${{ inputs.soft }}
"#;

    fn key_and_name(w: &Workflow, api: &str) -> Option<(String, String)> {
        w.find(api).map(|m| (m.job.key.clone(), m.name))
    }

    #[test]
    fn уровни_по_needs_строкой_и_списком() {
        let w = parse(YAML).unwrap();
        let level = |key: &str| w.jobs.iter().find(|j| j.key == key).unwrap().level;
        assert_eq!(
            [level("build"), level("lint"), level("dynamic"), level("test"), level("e2e"), level("call")],
            [0, 0, 0, 1, 1, 2]
        );
    }

    #[test]
    fn имена_стейджей_из_джоб_уровня_по_ключу() {
        let w = parse(YAML).unwrap();
        assert_eq!(
            w.stage_names(),
            vec!["build · dynamic · +1".to_string(), "e2e · test".into(), "call".into()]
        );
    }

    #[test]
    fn сопоставление_имён_api() {
        let w = parse(YAML).unwrap();
        assert_eq!(key_and_name(&w, "build"), Some(("build".into(), "build".into())));
        assert_eq!(key_and_name(&w, "Lint code"), Some(("lint".into(), "Lint code".into())));
        assert_eq!(key_and_name(&w, "test (linux)"), Some(("test".into(), "test: [linux]".into())));
        assert_eq!(
            key_and_name(&w, "e2e 1 on linux"),
            Some(("e2e".into(), "e2e: [e2e 1 on linux]".into()))
        );
        assert_eq!(
            key_and_name(&w, "call / deploy prod"),
            Some(("call".into(), "call / deploy prod".into()))
        );
        // шаблон из одних выражений ничего не ловит, иначе забрал бы все имена
        assert_eq!(key_and_name(&w, "что-то своё"), None);
    }

    #[test]
    fn continue_on_error_только_буквальное_true() {
        let w = parse(YAML).unwrap();
        let soft = |key: &str| w.jobs.iter().find(|j| j.key == key).unwrap().allow_failure;
        assert!(soft("lint"));
        assert!(!soft("dynamic"));
        assert!(!soft("build"));
    }

    #[test]
    fn цикл_и_несуществующая_джоба_не_роняют_разбор() {
        let w = parse("jobs:\n  a:\n    needs: [b, ghost]\n  b:\n    needs: a\n").unwrap();
        let level = |key: &str| w.jobs.iter().find(|j| j.key == key).unwrap().level;
        assert_eq!((level("a"), level("b")), (1, 0));
    }

    #[test]
    fn не_yaml_и_файл_без_jobs_дают_none() {
        assert!(parse("jobs: [не закрыто").is_none());
        assert!(parse("on: push\n").is_none());
    }
}
```

Run: `cd app && cargo test -p pipeline-trace-core --lib github::workflow`
Expected: ошибка компиляции — нет `parse`, `Workflow`.

- [ ] **Step 3: Реализация**

Над `#[cfg(test)]` в `app/core/src/github/workflow.rs`:

```rust
use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;
use serde_yaml_ng::Value;

static EXPR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{\{.*?\}\}").expect("верный шаблон"));

#[derive(Deserialize)]
struct File {
    jobs: BTreeMap<String, YamlJob>,
}

/// Поля джобы, которые нужны отчёту; остальные игнорируются. `Value` — чтобы чужая форма поля не роняла разбор.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct YamlJob {
    name: Option<String>,
    needs: Option<Value>,
    strategy: Option<Value>,
    continue_on_error: Option<Value>,
}

/// Джоба YAML с посчитанным уровнем.
#[derive(Debug)]
pub(crate) struct Job {
    pub(crate) key: String,
    /// `name:` или ключ — так GitHub называет джобу без matrix
    title: String,
    /// `name:` с `${{ }}` и литеральным текстом: шаблон имени из API
    pattern: Option<Regex>,
    pub(crate) needs: Vec<String>,
    matrix: bool,
    pub(crate) allow_failure: bool,
    /// длина самого длинного пути по `needs`
    pub(crate) level: usize,
}

impl Job {
    /// Имя в заголовке стейджа: шаблон показать нельзя — тогда ключ.
    fn display(&self) -> &str {
        if self.pattern.is_some() || EXPR.is_match(&self.title) {
            &self.key
        } else {
            &self.title
        }
    }
}

#[derive(Debug)]
pub(crate) struct Workflow {
    /// по ключу
    jobs: Vec<Job>,
}

/// Джоба API, сопоставленная с джобой YAML.
#[derive(Debug)]
pub(crate) struct Matched<'a> {
    pub(crate) job: &'a Job,
    /// имя для отчёта: шарды matrix — `X: [..]`, их сворачивает `model::shard_group_name`
    pub(crate) name: String,
}

pub(crate) fn parse(text: &str) -> Option<Workflow> {
    let file: File = serde_yaml_ng::from_str(text).ok()?;
    let mut jobs: Vec<Job> = file
        .jobs
        .into_iter()
        .map(|(key, job)| {
            let title = job.name.unwrap_or_else(|| key.clone());
            Job {
                pattern: pattern(&title),
                title,
                needs: needs_list(job.needs.as_ref()),
                matrix: job.strategy.as_ref().is_some_and(|s| s.get("matrix").is_some()),
                allow_failure: job.continue_on_error == Some(Value::Bool(true)),
                level: 0,
                key,
            }
        })
        .collect();
    for (job, level) in jobs.iter_mut().zip(levels(&jobs)) {
        job.level = level;
    }
    Some(Workflow { jobs })
}

fn needs_list(needs: Option<&Value>) -> Vec<String> {
    match needs {
        Some(Value::String(one)) => vec![one.clone()],
        Some(Value::Sequence(many)) => many
            .iter()
            .filter_map(|n| n.as_str().map(String::from))
            .collect(),
        _ => Vec::new(),
    }
}

/// `name:` с выражениями → регулярное выражение: литералы экранированы, выражение — `.*?`.
/// Имя из одних выражений шаблоном не становится: оно совпало бы с любой джобой.
fn pattern(name: &str) -> Option<Regex> {
    if !EXPR.is_match(name) || EXPR.replace_all(name, "").trim().is_empty() {
        return None;
    }
    let mut re = String::from("^");
    let mut last = 0;
    for m in EXPR.find_iter(name) {
        re.push_str(&regex::escape(&name[last..m.start()]));
        re.push_str(".*?");
        last = m.end();
    }
    re.push_str(&regex::escape(&name[last..]));
    re.push('$');
    Regex::new(&re).ok()
}

/// Уровень каждой джобы; рёбра в цикл и к несуществующим джобам не считаются.
fn levels(jobs: &[Job]) -> Vec<usize> {
    let index: HashMap<&str, usize> = jobs
        .iter()
        .enumerate()
        .map(|(i, j)| (j.key.as_str(), i))
        .collect();
    let mut memo = vec![None; jobs.len()];
    let mut visiting = vec![false; jobs.len()];
    (0..jobs.len())
        .map(|i| visit(i, jobs, &index, &mut memo, &mut visiting))
        .collect()
}

fn visit(
    i: usize,
    jobs: &[Job],
    index: &HashMap<&str, usize>,
    memo: &mut [Option<usize>],
    visiting: &mut [bool],
) -> usize {
    if let Some(level) = memo[i] {
        return level;
    }
    visiting[i] = true;
    let mut level = 0;
    for need in &jobs[i].needs {
        if let Some(&n) = index.get(need.as_str())
            && !visiting[n]
        {
            level = level.max(visit(n, jobs, index, memo, visiting) + 1);
        }
    }
    visiting[i] = false;
    memo[i] = Some(level);
    level
}

/// `X (a, b)` → (`X`, `a, b`): так GitHub называет matrix-джобу без выражений в имени.
fn matrix_suffix(name: &str) -> Option<(&str, &str)> {
    name.strip_suffix(')')?.rsplit_once(" (")
}

impl Workflow {
    /// Джоба YAML для имени из API (спека, § 2.5); `caller / inner` — джоба reusable workflow.
    pub(crate) fn find(&self, api_name: &str) -> Option<Matched<'_>> {
        self.find_direct(api_name).or_else(|| {
            let (caller, _) = api_name.split_once(" / ")?;
            let found = self.find_direct(caller)?;
            Some(Matched {
                job: found.job,
                name: api_name.into(),
            })
        })
    }

    fn find_direct(&self, api_name: &str) -> Option<Matched<'_>> {
        let plain = |j: &&Job| j.pattern.is_none();
        if let Some(job) = self.jobs.iter().filter(plain).find(|j| j.title == api_name) {
            return Some(Matched {
                job,
                name: api_name.into(),
            });
        }
        if let Some((base, values)) = matrix_suffix(api_name)
            && let Some(job) = self
                .jobs
                .iter()
                .filter(plain)
                .find(|j| j.matrix && j.title == base)
        {
            return Some(Matched {
                job,
                name: format!("{base}: [{values}]"),
            });
        }
        let job = self
            .jobs
            .iter()
            .find(|j| j.pattern.as_ref().is_some_and(|p| p.is_match(api_name)))?;
        let name = if job.matrix {
            format!("{}: [{api_name}]", job.key)
        } else {
            api_name.into()
        };
        Some(Matched { job, name })
    }

    /// Имена уровней `0..=max` (спека, § 2.4): `a`, `a · b`, `a · b · +N`.
    pub(crate) fn stage_names(&self) -> Vec<String> {
        let max = self.jobs.iter().map(|j| j.level).max().unwrap_or(0);
        (0..=max)
            .map(|level| {
                let names: Vec<&str> = self
                    .jobs
                    .iter()
                    .filter(|j| j.level == level)
                    .map(Job::display)
                    .collect();
                match names.as_slice() {
                    [] => String::new(),
                    [one] => (*one).to_string(),
                    [a, b] => format!("{a} · {b}"),
                    [a, b, rest @ ..] => format!("{a} · {b} · +{}", rest.len()),
                }
            })
            .collect()
    }
}
```

Проверка ожиданий теста `имена_стейджей_из_джоб_уровня_по_ключу`: уровень 0 по ключу — `build`, `dynamic` (шаблон без литералов → `pattern == None`, но `title` с выражением → `display` = ключ), `lint` (`Lint code`) → `build · dynamic · +1`; уровень 1 — `e2e` (шаблон → ключ), `test` → `e2e · test`; уровень 2 — `call`.

- [ ] **Step 4: Тесты зелёные**

Run: `cd app && cargo test -p pipeline-trace-core --lib github::workflow`
Expected: PASS, 6 тестов.

- [ ] **Step 5: Спека — правило шаблона из одних выражений**

В `docs/specs/2026-10-07-github-source-design.md`, § 2.5, пункт 2 дополнить предложением: «Имя из одних выражений (`${{ inputs.title }}`) шаблоном не становится — оно совпало бы с любой джобой; такие джобы API остаются несопоставленными.»

- [ ] **Step 6: Commit**

```bash
git add app/core/Cargo.toml app/Cargo.lock app/core/src/lib.rs app/core/src/github.rs app/core/src/github/workflow.rs docs/specs/2026-10-07-github-source-design.md
git commit -m "feat(core): разбор workflow GitHub — уровни needs и сопоставление имён"
```

### Task 3: HTTP-клиент GitHub — пагинация, ошибки, лимиты, проба хоста

**Files:**
- Create: `app/core/src/github/client.rs`
- Create: `app/core/tests/github_client.rs`
- Modify: `app/core/src/github.rs`, `app/core/src/error.rs`, `app/core/src/gitlab/client.rs`
- Modify: `app/src/shared/i18n/locales/ru.json`, `app/src/shared/i18n/locales/en.json`
- Generated: `app/src/shared/api/schema/ErrorCode.ts`

**Interfaces:**
- Consumes: `source::Provider` (Task 1), `github::workflow::Workflow` (Task 2).
- Produces:
  - `ErrorCode::RateLimited` — параметры `host`, `reset` (ISO 8601 UTC с миллисекундами, как `iso`).
  - `pub(crate) fn error::network_error(host: &str, e: reqwest::Error) -> Error` (перенос из `gitlab/client.rs`).
  - `pub struct github::Client` с `pub fn new(host: &str, token: &str) -> Result<Self, Error>`, `pub fn with_api_url(host: &str, api: &str, token: &str) -> Result<Self, Error>`, `pub fn host(&self) -> &str`.
  - `pub(crate)` методы клиента: `async fn json<T: DeserializeOwned>(&self, target: &str) -> Result<Option<T>, Error>`, `async fn page<T: DeserializeOwned>(&self, target: &str) -> Result<Option<(T, Option<String>)>, Error>` (второе — адрес следующей страницы), `async fn raw(&self, target: &str) -> Result<Option<String>, Error>`. `target` — путь API (`/repos/o/r/…`) или полный адрес из `Link`. `None` — 404.
  - `pub(crate) workflows: Mutex<HashMap<(String, String), Option<Arc<Workflow>>>>` — кэш файлов workflow по (путь, sha) на время жизни клиента.
  - `pub async fn github::probe_at(host: &str, base_url: &str) -> Result<Provider, Error>` и `pub async fn github::probe(host: &str) -> Result<Provider, Error>` (= `probe_at(host, "https://<host>")`).

- [ ] **Step 1: Тесты — падают**

`app/core/tests/github_client.rs`:

```rust
//! Проба типа хоста на wiremock. Запросы клиента с токеном проверяются в `tests/github.rs`
//! (Task 4–5) через `Source`: у клиента нет публичных методов запроса.
mod fixtures;

use fixtures::assert_error;
use pipeline_trace_core::error::ErrorCode;
use pipeline_trace_core::github::probe_at;
use pipeline_trace_core::source::Provider;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn проба_ghes_по_installed_version() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/meta"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "installed_version": "3.14.0" })))
        .mount(&server)
        .await;
    assert_eq!(probe_at("ghe.example", &server.uri()).await.unwrap(), Provider::Github);
}

#[tokio::test]
async fn проба_gitlab_по_404_и_по_чужому_json() {
    let server = MockServer::start().await;
    assert_eq!(probe_at("gl.example", &server.uri()).await.unwrap(), Provider::Gitlab);
    Mock::given(method("GET"))
        .and(path("/api/v3/meta"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "message": "hi" })))
        .mount(&server)
        .await;
    assert_eq!(probe_at("gl.example", &server.uri()).await.unwrap(), Provider::Gitlab);
}

#[tokio::test]
async fn проба_без_соединения_это_network() {
    // порт 9 (discard) на localhost закрыт
    let result = probe_at("down.example", "http://127.0.0.1:9").await;
    assert_eq!(result.unwrap_err().code, ErrorCode::Network);
}
```

Юнит-тесты `status_error` и `next_link` — в конце `app/core/src/github/client.rs`:

```rust
#[cfg(test)]
mod tests {
    use reqwest::StatusCode;
    use reqwest::header::{HeaderMap, HeaderValue};
    use time::macros::datetime;

    use super::*;

    const NOW: OffsetDateTime = datetime!(2026-10-07 10:00:00 UTC);

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        pairs
            .iter()
            .map(|&(k, v)| (k.parse().unwrap(), HeaderValue::from_str(v).unwrap()))
            .collect()
    }

    #[test]
    fn лимит_по_remaining_ноль_с_временем_сброса() {
        let e = status_error(
            "h",
            StatusCode::FORBIDDEN,
            &headers(&[("x-ratelimit-remaining", "0"), ("x-ratelimit-reset", "1791367200")]),
            NOW,
        );
        assert_eq!(e.code, ErrorCode::RateLimited);
        assert_eq!(e.params["reset"], "2026-10-07T10:00:00.000Z");
    }

    #[test]
    fn вторичный_лимит_по_retry_after() {
        let e = status_error("h", StatusCode::TOO_MANY_REQUESTS, &headers(&[("retry-after", "60")]), NOW);
        assert_eq!(e.code, ErrorCode::RateLimited);
        assert_eq!(e.params["reset"], "2026-10-07T10:01:00.000Z");
    }

    #[test]
    fn 403_без_лимита_это_нет_доступа_а_500_это_статус() {
        assert_eq!(status_error("h", StatusCode::FORBIDDEN, &HeaderMap::new(), NOW).code, ErrorCode::Unauthorized);
        assert_eq!(status_error("h", StatusCode::UNAUTHORIZED, &HeaderMap::new(), NOW).code, ErrorCode::Unauthorized);
        let e = status_error("h", StatusCode::INTERNAL_SERVER_ERROR, &HeaderMap::new(), NOW);
        assert_eq!((e.code, e.params["status"].as_str()), (ErrorCode::HttpStatus, "500"));
    }

    #[test]
    fn следующая_страница_только_на_своём_api() {
        let link = r#"<https://api.github.com/repos/o/r/actions/runs?page=2>; rel="next", <https://api.github.com/repos/o/r/actions/runs?page=9>; rel="last""#;
        assert_eq!(
            next_link(&headers(&[("link", link)]), "https://api.github.com").as_deref(),
            Some("https://api.github.com/repos/o/r/actions/runs?page=2")
        );
        assert_eq!(next_link(&headers(&[("link", link)]), "https://ghe.example/api/v3"), None);
        assert_eq!(next_link(&HeaderMap::new(), "https://api.github.com"), None);
    }

    #[test]
    fn адрес_api_для_github_com_и_ghes() {
        assert_eq!(api_url("github.com"), "https://api.github.com");
        assert_eq!(api_url("ghe.example"), "https://ghe.example/api/v3");
    }
}
```

`1791367200` — unix-время `2026-10-07T10:00:00Z`; проверить: `date -u -r 1791367200` (macOS) или `date -u -d @1791367200`.

Run: `cd app && cargo test -p pipeline-trace-core --test github_client`
Expected: ошибка компиляции — нет `github::probe_at`.

- [ ] **Step 2: `network_error` в `error.rs`, `RateLimited`**

Перенести `fn network_error` из `app/core/src/gitlab/client.rs` в конец `app/core/src/error.rs` как `pub(crate) fn network_error(host: &str, e: reqwest::Error) -> Error` (тело без изменений, вместе с `use std::fmt::Write;`). В `gitlab/client.rs` удалить функцию и `use std::fmt::Write;`, добавить `use crate::error::network_error;`.

В `ErrorCode` после `ReportExpired`:

```rust
    /// лимит запросов GitHub: `host`, `reset` — ISO 8601, когда можно повторить
    RateLimited,
    /// агрегат GitHub без workflow
    WorkflowRequired,
```

`WorkflowRequired` добавляется здесь же, чтобы локали правились один раз; используется в Task 6 и Task 9.

Локали — в `errors` после `reportExpired`:

`ru.json`:
```json
    "rateLimited": "{{host}}: лимит запросов исчерпан, повтори после {{reset}}",
    "workflowRequired": "Выбери workflow: агрегат GitHub строится по одному workflow",
```

`en.json`:
```json
    "rateLimited": "{{host}}: rate limit exceeded, retry after {{reset}}",
    "workflowRequired": "Choose a workflow: a GitHub aggregate covers one workflow",
```

- [ ] **Step 3: Клиент**

`app/core/src/github/client.rs` (над `#[cfg(test)]`):

```rust
//! HTTP-клиент GitHub REST: заголовки, пагинация по `Link`, лимиты запросов.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::header::{ACCEPT, HeaderMap, LINK};
use serde::de::DeserializeOwned;
use time::OffsetDateTime;
use tokio::sync::Semaphore;

use super::workflow::Workflow;
use crate::error::{Error, ErrorCode, network_error};
use crate::iso::iso;
use crate::source::Provider;

const MAX_PARALLEL: usize = 4;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const READ_TIMEOUT: Duration = Duration::from_secs(60);
const USER_AGENT: &str = "pipeline-trace";
const JSON: &str = "application/vnd.github+json";
const RAW: &str = "application/vnd.github.raw";
const API_VERSION: &str = "2022-11-28";

/// github.com живёт на отдельном хосте API, GHES — под `/api/v3`.
pub(crate) fn api_url(host: &str) -> String {
    if host == crate::source::GITHUB_COM {
        "https://api.github.com".into()
    } else {
        format!("https://{host}/api/v3")
    }
}

fn http(host: &str) -> Result<reqwest::Client, Error> {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        // редирект унёс бы токен на чужой хост
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| network_error(host, e))
}

pub struct Client {
    http: reqwest::Client,
    host: String,
    api: String,
    token: String,
    permits: Semaphore,
    /// (путь, sha) → разобранный файл workflow; `None` — не загрузился
    pub(crate) workflows: Mutex<HashMap<(String, String), Option<Arc<Workflow>>>>,
}

/// Тело ответа и адрес следующей страницы.
struct Body {
    next: Option<String>,
    bytes: Vec<u8>,
}

impl Client {
    pub fn new(host: &str, token: &str) -> Result<Self, Error> {
        Self::with_api_url(host, &api_url(host), token)
    }

    /// `api` — корень REST без завершающего слэша; тесты подставляют адрес wiremock.
    pub fn with_api_url(host: &str, api: &str, token: &str) -> Result<Self, Error> {
        Ok(Self {
            http: http(host)?,
            host: host.into(),
            api: api.trim_end_matches('/').into(),
            token: token.into(),
            permits: Semaphore::new(MAX_PARALLEL),
            workflows: Mutex::default(),
        })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    async fn get(&self, target: &str, accept: &str) -> Result<Option<Body>, Error> {
        let url = if target.starts_with("http://") || target.starts_with("https://") {
            target.to_string()
        } else {
            format!("{}{target}", self.api)
        };
        let _permit = self.permits.acquire().await.expect("семафор не закрывают");
        let response = self
            .http
            .get(&url)
            .bearer_auth(&self.token)
            .header(ACCEPT, accept)
            .header("X-GitHub-Api-Version", API_VERSION)
            .send()
            .await
            .map_err(|e| network_error(&self.host, e))?;
        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !status.is_success() {
            return Err(status_error(
                &self.host,
                status,
                response.headers(),
                OffsetDateTime::now_utc(),
            ));
        }
        let next = next_link(response.headers(), &self.api);
        let bytes = response
            .bytes()
            .await
            .map_err(|e| network_error(&self.host, e))?
            .to_vec();
        Ok(Some(Body { next, bytes }))
    }

    /// Неожиданная форма JSON — `graphql` с текстом serde: перевод общий «неожиданный ответ».
    fn decode<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<T, Error> {
        serde_json::from_slice(bytes).map_err(|e| {
            Error::new(ErrorCode::Graphql)
                .with("host", &self.host)
                .with("detail", e)
        })
    }

    pub(crate) async fn json<T: DeserializeOwned>(&self, target: &str) -> Result<Option<T>, Error> {
        match self.get(target, JSON).await? {
            Some(body) => self.decode(&body.bytes).map(Some),
            None => Ok(None),
        }
    }

    pub(crate) async fn page<T: DeserializeOwned>(
        &self,
        target: &str,
    ) -> Result<Option<(T, Option<String>)>, Error> {
        match self.get(target, JSON).await? {
            Some(body) => Ok(Some((self.decode(&body.bytes)?, body.next))),
            None => Ok(None),
        }
    }

    /// Содержимое файла как есть (`contents` с `Accept: raw`).
    pub(crate) async fn raw(&self, target: &str) -> Result<Option<String>, Error> {
        Ok(self
            .get(target, RAW)
            .await?
            .map(|body| String::from_utf8_lossy(&body.bytes).into_owned()))
    }
}

/// Не-2xx в код ошибки: лимит — по `retry-after` или `x-ratelimit-remaining: 0`, 401/403 — нет доступа.
pub(crate) fn status_error(
    host: &str,
    status: StatusCode,
    headers: &HeaderMap,
    now: OffsetDateTime,
) -> Error {
    let header = |name: &str| headers.get(name).and_then(|v| v.to_str().ok());
    if matches!(status.as_u16(), 403 | 429) {
        let limited = |reset: OffsetDateTime| {
            Error::new(ErrorCode::RateLimited)
                .with("host", host)
                .with("reset", iso(reset))
        };
        if let Some(seconds) = header("retry-after").and_then(|v| v.parse::<i64>().ok()) {
            return limited(now + time::Duration::seconds(seconds));
        }
        if header("x-ratelimit-remaining") == Some("0") {
            let reset = header("x-ratelimit-reset")
                .and_then(|v| v.parse::<i64>().ok())
                .and_then(|t| OffsetDateTime::from_unix_timestamp(t).ok())
                .unwrap_or(now);
            return limited(reset);
        }
    }
    let code = if matches!(status.as_u16(), 401 | 403) {
        ErrorCode::Unauthorized
    } else {
        ErrorCode::HttpStatus
    };
    Error::new(code)
        .with("host", host)
        .with("status", status.as_u16())
}

/// `rel="next"` из `Link`; адрес не своего API игнорируется — токен уйдёт только на свой хост.
pub(crate) fn next_link(headers: &HeaderMap, api: &str) -> Option<String> {
    let value = headers.get(LINK)?.to_str().ok()?;
    value
        .split(',')
        .find_map(|part| {
            let (url, rel) = part.split_once(';')?;
            (rel.trim() == r#"rel="next""#).then(|| {
                url.trim()
                    .trim_start_matches('<')
                    .trim_end_matches('>')
                    .to_string()
            })
        })
        .filter(|url| url.starts_with(api))
}

/// Тип хоста по `GET /api/v3/meta` без токена: GHES отвечает 200 с `installed_version`, GitLab — нет.
pub async fn probe_at(host: &str, base_url: &str) -> Result<Provider, Error> {
    let response = http(host)?
        .get(format!("{}/api/v3/meta", base_url.trim_end_matches('/')))
        .send()
        .await
        .map_err(|e| network_error(host, e))?;
    if !response.status().is_success() {
        return Ok(Provider::Gitlab);
    }
    let body: serde_json::Value = response.json().await.unwrap_or_default();
    Ok(if body.get("installed_version").is_some() {
        Provider::Github
    } else {
        Provider::Gitlab
    })
}

pub async fn probe(host: &str) -> Result<Provider, Error> {
    probe_at(host, &format!("https://{host}")).await
}
```

`app/core/src/source.rs` — после `use`:

```rust
/// Хост GitHub, тип которого известен без пробы и кэша.
pub const GITHUB_COM: &str = "github.com";
```

`app/core/src/github.rs`:

```rust
//! GitHub Actions: workflow run как пайплайн (спека, § 2), списки для формы (§ 6).

mod client;
pub(crate) mod workflow;

pub use client::{Client, probe, probe_at};
```

- [ ] **Step 4: Тесты зелёные, локали совпадают**

Run: `cd app && cargo test -p pipeline-trace-core`
Expected: PASS (новые: 3 в `github_client`, 5 в `github::client::tests`).

Run: `cd app && npm run check:locales && npm run typecheck`
Expected: без ошибок.

Предупреждения `dead_code` о `json`, `page`, `raw`, `workflows` до Task 4–5 ожидаемы: clippy с `-D warnings` запускается только перед пушем, пушить до Task 5 не нужно.

- [ ] **Step 5: Commit**

```bash
git add app/core app/src/shared/api/schema app/src/shared/i18n/locales
git commit -m "feat(core): HTTP-клиент GitHub — Link, лимиты, проба хоста"
```

### Task 4: Run GitHub → `RawPipeline`

**Files:**
- Modify: `app/core/src/github.rs`
- Create: `app/core/tests/github.rs`
- Modify: `docs/specs/2026-10-07-github-source-design.md` (§ 2.6, одно предложение)

**Interfaces:**
- Consumes: `github::Client` и его `json`/`page`/`raw`/`workflows` (Task 3), `workflow::{parse, Workflow, Matched}` (Task 2), `RawPipeline.needs_missing` (Task 1).
- Produces:
  - `pub async fn Client::fetch_run(&self, project: &str, id: &str) -> Result<RawPipeline, Error>` — run со всеми попытками; 404 на run или джобы — `PipelineNotFound` (`host`, `pipeline`, `project`); больше 50 страниц джоб — `TooManyJobs`.
  - `pub(crate) struct WireRun` (поля ниже) и `impl WireRun { pub(crate) fn duration(&self, now: OffsetDateTime) -> time::Duration }` — нужны Task 5.
  - `pub(crate) fn status(status: &str, conclusion: Option<&str>) -> &'static str` — таблица спеки, § 2.3.
  - `pub(crate) fn path_of(url: &str) -> String` — путь из абсолютного адреса.

- [ ] **Step 1: Тесты — падают**

`app/core/tests/github.rs`:

```rust
//! GitHub через wiremock: run → `RawPipeline` (спека, § 2), списки формы (§ 6).
mod fixtures;

use fixtures::assert_error;
use pipeline_trace_core::error::ErrorCode;
use pipeline_trace_core::github::Client;
use pipeline_trace_core::model::RawPipeline;
use serde_json::{Value, json};
use time::macros::datetime;
use wiremock::matchers::{header, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

const YAML: &str = "
jobs:
  build: {}
  lint:
    continue-on-error: true
  test:
    needs: build
    strategy:
      matrix:
        os: [linux, mac]
";

fn client(server: &MockServer) -> Client {
    Client::with_api_url("github.com", &server.uri(), "t").unwrap()
}

fn run_json() -> Value {
    json!({
        "id": 7, "name": "ci", "run_number": 12, "run_attempt": 2,
        "status": "completed", "conclusion": "success",
        "created_at": "2026-10-07T10:00:00Z", "updated_at": "2026-10-07T10:40:00Z",
        "run_started_at": "2026-10-07T10:30:00Z",
        "head_branch": "main", "head_sha": "abc123", "path": ".github/workflows/ci.yml",
        "html_url": "https://github.com/o/r/actions/runs/7", "event": "push",
        "head_commit": { "message": "fix: x\n\nтело" }, "actor": { "login": "u" },
    })
}

/// Время — минуты:секунды после 10:00.
fn job(id: u64, attempt: u32, name: &str, conclusion: &str, created: &str, started: &str, completed: &str) -> Value {
    let at = |ms: &str| format!("2026-10-07T10:{ms}Z");
    json!({
        "id": id, "run_attempt": attempt, "name": name,
        "status": "completed", "conclusion": conclusion,
        "created_at": at(created), "started_at": at(started), "completed_at": at(completed),
        "html_url": format!("https://github.com/o/r/actions/runs/7/job/{id}"),
    })
}

async fn mount_run(server: &MockServer, workflow: Option<&str>) {
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/runs/7"))
        .and(header("authorization", "Bearer t"))
        .and(header("x-github-api-version", "2022-11-28"))
        .respond_with(ResponseTemplate::new(200).set_body_json(run_json()))
        .mount(server)
        .await;
    let next = format!(
        "<{}/repos/o/r/actions/runs/7/jobs?filter=all&per_page=100&page=2>; rel=\"next\"",
        server.uri()
    );
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/runs/7/jobs"))
        .and(query_param("filter", "all"))
        .and(query_param_is_missing("page"))
        .respond_with(ResponseTemplate::new(200).insert_header("link", next.as_str()).set_body_json(json!({
            "total_count": 7,
            "jobs": [
                job(1, 1, "build", "failure", "00:01", "00:05", "01:00"),
                job(2, 1, "lint", "success", "00:01", "00:05", "00:30"),
                // пропущенная: GitHub отдаёт старт позже конца
                job(3, 1, "test (linux)", "skipped", "01:00", "01:01", "01:00"),
            ],
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/runs/7/jobs"))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "total_count": 7,
            "jobs": [
                job(11, 2, "build", "success", "30:01", "30:05", "31:00"),
                // копия из первой попытки: те же времена
                job(12, 2, "lint", "success", "30:01", "00:05", "00:30"),
                job(13, 2, "test (linux)", "success", "31:01", "31:05", "33:00"),
                job(14, 2, "test (mac)", "success", "31:01", "31:05", "34:00"),
            ],
        })))
        .mount(server)
        .await;
    let contents = Mock::given(method("GET"))
        .and(path("/repos/o/r/contents/.github/workflows/ci.yml"))
        .and(query_param("ref", "abc123"))
        .and(header("accept", "application/vnd.github.raw"));
    match workflow {
        Some(text) => contents.respond_with(ResponseTemplate::new(200).set_body_string(text)),
        None => contents.respond_with(ResponseTemplate::new(404)),
    }
    .expect(1)
    .mount(server)
    .await;
}

/// (id, имя, стейдж, ретрай, needs, allow_failure) в порядке API.
fn jobs_view(raw: &RawPipeline) -> Vec<(&str, &str, &str, bool, Vec<&str>, bool)> {
    raw.jobs
        .iter()
        .map(|j| {
            (
                j.id.as_str(),
                j.name.as_str(),
                j.stage.as_str(),
                j.retried,
                j.needs.iter().map(String::as_str).collect(),
                j.allow_failure,
            )
        })
        .collect()
}

#[tokio::test]
async fn run_с_перезапуском_копиями_и_matrix() {
    let server = MockServer::start().await;
    mount_run(&server, Some(YAML)).await;
    let raw = client(&server).fetch_run("o/r", "7").await.unwrap();

    let p = &raw.pipeline;
    assert_eq!(
        (p.id.as_str(), p.iid.as_str(), p.status.as_str(), p.r#ref.as_str(), p.path.as_str()),
        ("7", "12", "SUCCESS", "main", "/o/r/actions/runs/7")
    );
    assert_eq!(p.created_at, datetime!(2026-10-07 10:00:00 UTC));
    assert_eq!(p.finished_at, Some(datetime!(2026-10-07 10:40:00 UTC)));
    assert_eq!(p.stages, vec!["build · lint", "test"]);
    assert!(!raw.needs_missing);
    assert!(raw.downstream.is_empty());
    assert_eq!(
        jobs_view(&raw),
        vec![
            ("13", "test: [linux]", "test", false, vec!["build"], false),
            ("14", "test: [mac]", "test", false, vec!["build"], false),
            ("11", "build", "build · lint", false, vec![], false),
            ("12", "lint", "build · lint", false, vec![], true),
            ("1", "build", "build · lint", true, vec![], false),
        ]
    );
    let build = &raw.jobs[2];
    assert_eq!(build.queued_duration, Some(4.0));
    assert_eq!(build.web_path, "/o/r/actions/runs/7/job/11");
    assert_eq!(build.status, "SUCCESS");
    assert_eq!(raw.jobs[4].status, "FAILED");
}

#[tokio::test]
async fn без_файла_workflow_один_стейдж_и_признак() {
    let server = MockServer::start().await;
    mount_run(&server, None).await;
    let raw = client(&server).fetch_run("o/r", "7").await.unwrap();
    assert!(raw.needs_missing);
    assert_eq!(raw.pipeline.stages, vec!["ci"]);
    assert!(raw.jobs.iter().all(|j| j.stage == "ci" && j.needs.is_empty()));
    // без YAML matrix не распознать: имя как в API
    assert!(raw.jobs.iter().any(|j| j.name == "test (linux)"));
}

#[tokio::test]
async fn файл_workflow_грузится_один_раз_на_клиент() {
    let server = MockServer::start().await;
    mount_run(&server, Some(YAML)).await;
    let client = client(&server);
    client.fetch_run("o/r", "7").await.unwrap();
    client.fetch_run("o/r", "7").await.unwrap();
    // `.expect(1)` на contents проверяется при остановке сервера
}

#[tokio::test]
async fn нет_run_это_pipeline_not_found() {
    let server = MockServer::start().await;
    assert_error(
        client(&server).fetch_run("o/r", "7").await,
        ErrorCode::PipelineNotFound,
        &[("host", "github.com"), ("pipeline", "7"), ("project", "o/r")],
    );
}
```

Юнит-тест таблицы статусов — в конце `app/core/src/github.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn статусы_github_в_словаре_gitlab() {
        let cases = [
            ("completed", Some("success"), "SUCCESS"),
            ("completed", Some("failure"), "FAILED"),
            ("completed", Some("timed_out"), "FAILED"),
            ("completed", Some("startup_failure"), "FAILED"),
            ("completed", Some("cancelled"), "CANCELED"),
            ("completed", Some("skipped"), "SKIPPED"),
            ("completed", Some("neutral"), "SKIPPED"),
            ("completed", Some("action_required"), "MANUAL"),
            ("waiting", None, "MANUAL"),
            ("pending", None, "MANUAL"),
            ("requested", None, "MANUAL"),
            ("queued", None, "PENDING"),
            ("in_progress", None, "RUNNING"),
        ];
        for (s, c, expected) in cases {
            assert_eq!(status(s, c), expected, "{s} {c:?}");
        }
    }

    #[test]
    fn путь_из_адреса() {
        assert_eq!(path_of("https://github.com/o/r/actions/runs/7"), "/o/r/actions/runs/7");
        assert_eq!(path_of("не адрес"), "");
    }
}
```

Run: `cd app && cargo test -p pipeline-trace-core --test github`
Expected: ошибка компиляции — нет метода `fetch_run`.

- [ ] **Step 2: Реализация**

`app/core/src/github.rs` целиком (над `#[cfg(test)]`):

```rust
//! GitHub Actions: workflow run как пайплайн (спека, § 2), списки для формы (§ 6).

mod client;
pub(crate) mod workflow;

use std::cmp::Reverse;
use std::collections::HashMap;
use std::sync::{Arc, PoisonError};

use serde::Deserialize;
use time::OffsetDateTime;

pub use client::{Client, probe, probe_at};

use crate::error::{Error, ErrorCode};
use crate::model::{RawJob, RawPipeline, RawPipelineInfo};
use workflow::{Matched, Workflow};

const MAX_JOB_PAGES: usize = 50;
const JOBS_PER_PAGE: usize = 100;

#[derive(Deserialize)]
pub(crate) struct WireRun {
    pub(crate) id: u64,
    name: Option<String>,
    pub(crate) run_number: u64,
    pub(crate) status: String,
    pub(crate) conclusion: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub(crate) created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub(crate) updated_at: OffsetDateTime,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub(crate) run_started_at: Option<OffsetDateTime>,
    head_branch: Option<String>,
    pub(crate) head_sha: String,
    path: String,
    pub(crate) html_url: String,
    pub(crate) event: String,
    pub(crate) head_commit: Option<HeadCommit>,
    pub(crate) actor: Option<Actor>,
}

#[derive(Deserialize)]
pub(crate) struct HeadCommit {
    pub(crate) message: String,
}

#[derive(Deserialize)]
pub(crate) struct Actor {
    pub(crate) login: String,
}

impl WireRun {
    pub(crate) fn completed(&self) -> bool {
        self.status == "completed"
    }

    /// От старта последней попытки до конца; идущий — до `now`.
    pub(crate) fn duration(&self, now: OffsetDateTime) -> time::Duration {
        let end = if self.completed() { self.updated_at } else { now };
        end - self.run_started_at.unwrap_or(self.created_at)
    }
}

#[derive(Deserialize)]
struct WireJob {
    id: u64,
    run_attempt: u32,
    name: String,
    status: String,
    conclusion: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(default, with = "time::serde::rfc3339::option")]
    started_at: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    completed_at: Option<OffsetDateTime>,
    html_url: Option<String>,
}

#[derive(Deserialize)]
struct Jobs {
    jobs: Vec<WireJob>,
}

/// Статус GitHub в словаре GitLab (спека, § 2.3); неизвестный итог завершённой — `FAILED`.
pub(crate) fn status(status: &str, conclusion: Option<&str>) -> &'static str {
    match (status, conclusion) {
        ("completed", Some("success")) => "SUCCESS",
        ("completed", Some("cancelled")) => "CANCELED",
        ("completed", Some("skipped" | "neutral")) => "SKIPPED",
        ("completed", Some("action_required")) => "MANUAL",
        ("completed", _) => "FAILED",
        ("waiting" | "pending" | "requested", _) => "MANUAL",
        ("queued", _) => "PENDING",
        _ => "RUNNING",
    }
}

/// `https://host/o/r/…` → `/o/r/…`; не адрес — пусто.
pub(crate) fn path_of(url: &str) -> String {
    url.split_once("://")
        .and_then(|(_, rest)| rest.find('/').map(|i| rest[i..].to_string()))
        .unwrap_or_default()
}

/// Пропущенной джобе GitHub ставит старт позже конца: времён у неё нет.
fn without_skipped_times(mut job: WireJob) -> WireJob {
    if job.conclusion.as_deref() == Some("skipped") {
        job.started_at = None;
        job.completed_at = None;
    }
    job
}

/// Джобы всех попыток (спека, § 2.6): последняя попытка — основная; ранняя с теми же временами —
/// копия, не запускавшаяся — не ретрай; остальные ранние — `retried`.
fn attempts(mut jobs: Vec<WireJob>) -> Vec<(WireJob, bool)> {
    jobs.sort_by_key(|j| Reverse(j.run_attempt));
    let mut seen: HashMap<String, Vec<(Option<OffsetDateTime>, Option<OffsetDateTime>)>> =
        HashMap::new();
    let mut kept = Vec::new();
    for job in jobs {
        let times = (job.started_at, job.completed_at);
        let earlier = seen.entry(job.name.clone()).or_default();
        let retried = !earlier.is_empty();
        if earlier.contains(&times) || (retried && times.0.is_none()) {
            continue;
        }
        earlier.push(times);
        kept.push((job, retried));
    }
    kept
}

/// Run и его джобы → `RawPipeline`; `workflow == None` — файл не загрузился.
fn to_raw(project: &str, run: &WireRun, jobs: Vec<WireJob>, workflow: Option<&Workflow>) -> RawPipeline {
    let mut found: Vec<(WireJob, bool, Option<Matched<'_>>)> =
        attempts(jobs.into_iter().map(without_skipped_times).collect())
            .into_iter()
            .map(|(job, retried)| {
                let matched = workflow.and_then(|w| w.find(&job.name));
                (job, retried, matched)
            })
            .collect();
    // как у GitLab: от новых к старым
    found.sort_by_key(|(job, ..)| Reverse(job.created_at));

    // ключ YAML → имена джоб API последней попытки: из них `needs`
    let mut names_by_key: HashMap<String, Vec<String>> = HashMap::new();
    for (_, retried, matched) in &found {
        if let Some(m) = matched
            && !retried
        {
            names_by_key.entry(m.job.key.clone()).or_default().push(m.name.clone());
        }
    }
    let stages = match workflow {
        Some(w) => w.stage_names(),
        None => vec![run.name.clone().unwrap_or_default()],
    };

    let raw_jobs = found
        .iter()
        .map(|(job, retried, matched)| {
            let needs = matched
                .as_ref()
                .map(|m| {
                    m.job
                        .needs
                        .iter()
                        .flat_map(|key| names_by_key.get(key).into_iter().flatten().cloned())
                        .collect()
                })
                .unwrap_or_default();
            RawJob {
                id: job.id.to_string(),
                name: matched.as_ref().map_or_else(|| job.name.clone(), |m| m.name.clone()),
                bridge: false,
                status: status(&job.status, job.conclusion.as_deref()).into(),
                started_at: job.started_at,
                finished_at: job.completed_at,
                queued_duration: job.started_at.map(|s| (s - job.created_at).as_seconds_f64()),
                retried: *retried,
                allow_failure: matched.as_ref().is_some_and(|m| m.job.allow_failure),
                web_path: job.html_url.as_deref().map(path_of).unwrap_or_default(),
                stage: stages[matched.as_ref().map_or(0, |m| m.job.level)].clone(),
                needs,
            }
        })
        .collect();

    RawPipeline {
        project: project.into(),
        pipeline: RawPipelineInfo {
            id: run.id.to_string(),
            iid: run.run_number.to_string(),
            status: status(&run.status, run.conclusion.as_deref()).into(),
            created_at: run.created_at,
            finished_at: run.completed().then_some(run.updated_at),
            r#ref: run.head_branch.clone().unwrap_or_default(),
            path: path_of(&run.html_url),
            stages,
        },
        jobs: raw_jobs,
        downstream: HashMap::new(),
        needs_missing: workflow.is_none(),
    }
}

impl Client {
    /// Workflow run со всеми попытками (спека, § 2).
    pub async fn fetch_run(&self, project: &str, id: &str) -> Result<RawPipeline, Error> {
        let not_found = || {
            Error::new(ErrorCode::PipelineNotFound)
                .with("host", self.host())
                .with("pipeline", id)
                .with("project", project)
        };
        let run: WireRun = self
            .json(&format!("/repos/{project}/actions/runs/{id}"))
            .await?
            .ok_or_else(not_found)?;
        let mut jobs = Vec::new();
        let mut next = Some(format!(
            "/repos/{project}/actions/runs/{id}/jobs?filter=all&per_page={JOBS_PER_PAGE}"
        ));
        for _ in 0..MAX_JOB_PAGES {
            let Some(target) = next.take() else { break };
            let (page, link): (Jobs, _) = self.page(&target).await?.ok_or_else(not_found)?;
            jobs.extend(page.jobs);
            next = link;
        }
        if next.is_some() {
            return Err(Error::new(ErrorCode::TooManyJobs)
                .with("pipeline", id)
                .with("project", project)
                .with("limit", MAX_JOB_PAGES * JOBS_PER_PAGE));
        }
        let workflow = self.workflow(project, &run).await;
        Ok(to_raw(project, &run, jobs, workflow.as_deref()))
    }

    /// Файл workflow на коммите запуска; любой сбой — `None`: отчёт строится без `needs` (спека, § 2.4).
    // ponytail: параллельные запуски одного агрегата могут скачать файл одновременно — лишний запрос, не ошибка
    async fn workflow(&self, project: &str, run: &WireRun) -> Option<Arc<Workflow>> {
        let path = run.path.split('@').next().unwrap_or(&run.path);
        let key = (path.to_string(), run.head_sha.clone());
        let cached = self
            .workflows
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&key)
            .cloned();
        if let Some(cached) = cached {
            return cached;
        }
        let text = self
            .raw(&format!("/repos/{project}/contents/{path}?ref={}", run.head_sha))
            .await
            .ok()
            .flatten();
        let parsed = text.as_deref().and_then(workflow::parse).map(Arc::new);
        self.workflows
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key, parsed.clone());
        parsed
    }
}
```

- [ ] **Step 3: Тесты зелёные**

Run: `cd app && cargo test -p pipeline-trace-core --test github --lib github::tests`
Expected: PASS (4 + 2).

- [ ] **Step 4: Спека — непускавшиеся ранние попытки**

`docs/specs/2026-10-07-github-source-design.md`, § 2.6, после «…отбрасывается.» добавить: «Ранняя попытка, которая не запускалась (пропущенная, без времён), тоже отбрасывается: ретраем она не была.»

- [ ] **Step 5: Commit**

```bash
git add app/core/src/github.rs app/core/tests/github.rs docs/specs/2026-10-07-github-source-design.md
git commit -m "feat(core): run GitHub в RawPipeline — попытки, копии, needs из workflow"
```

### Task 5: Списки GitHub и `impl Source for github::Client`

**Files:**
- Create: `app/core/src/github/lists.rs`
- Modify: `app/core/src/github.rs`
- Modify (тесты): `app/core/tests/github.rs`

**Interfaces:**
- Consumes: `WireRun`, `WireRun::duration`, `WireRun::completed`, `status` (Task 4); `Client::json`/`page` (Task 3); `Source`, `Provider`, `Workflow`, `Pipeline.url`, `PipelineFilter.workflow` (Task 1).
- Produces: `impl Source for github::Client` — все операции из спеки, § 6 и § 7.

Когерентность проверена 2026-10-07 на мини-крейте: `impl<G: Gql> Source for G` уживается с `impl Source for github::Client` и с enum-диспетчером (Task 9), если `Source::host` в реализациях вызывается полным путём (`Gql::host(self)`, `Client::host(self)`, `Source::host(c)`) — иначе неоднозначность имени `host`.

- [ ] **Step 1: Тесты — падают**

Дописать в `app/core/tests/github.rs` (в `use` добавить `pipeline_trace_core::browse::{Commit, Page, Pipeline, Project, Workflow}`, `pipeline_trace_core::gitlab::PipelineFilter`, `pipeline_trace_core::source::{Provider, Source}`, `std::collections::BTreeMap`):

```rust
/// Run в списке: `minutes` — длительность последней попытки.
fn listed_run(id: u64, conclusion: &str, minutes: u32) -> Value {
    let mut run = run_json();
    run["id"] = json!(id);
    run["conclusion"] = json!(conclusion);
    run["run_started_at"] = json!("2026-10-07T10:00:00Z");
    run["updated_at"] = json!(format!("2026-10-07T10:{minutes:02}:00Z"));
    run
}

fn filter(statuses: Option<&[&str]>, last: usize) -> PipelineFilter {
    PipelineFilter {
        r#ref: Some("main".into()),
        source: Some("push".into()),
        statuses: statuses.map(|s| s.iter().map(|&x| x.into()).collect()),
        last,
        workflow: Some("ci.yml".into()),
    }
}

#[tokio::test]
async fn провайдер_github() {
    let server = MockServer::start().await;
    assert_eq!(client(&server).provider(), Provider::Github);
}

#[tokio::test]
async fn агрегат_один_статус_уходит_параметром_и_обрезается_по_last() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/workflows/ci.yml/runs"))
        .and(query_param("branch", "main"))
        .and(query_param("event", "push"))
        .and(query_param("status", "success"))
        .and(query_param("per_page", "100"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "workflow_runs": [
            listed_run(1, "success", 5), listed_run(2, "success", 6), listed_run(3, "success", 7),
        ] })))
        .mount(&server)
        .await;
    let listed = client(&server)
        .list_pipelines("o/r", &filter(Some(&["SUCCESS"]), 2))
        .await
        .unwrap();
    assert_eq!(listed.ids, vec!["1", "2"]);
    assert_eq!(listed.counts, BTreeMap::from([("SUCCESS".to_string(), 2)]));
}

#[tokio::test]
async fn агрегат_несколько_статусов_фильтруются_на_клиенте() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/workflows/ci.yml/runs"))
        .and(query_param_is_missing("status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "workflow_runs": [
            listed_run(1, "success", 5), listed_run(2, "cancelled", 6), listed_run(3, "failure", 7),
        ] })))
        .mount(&server)
        .await;
    let listed = client(&server)
        .list_pipelines("o/r", &filter(Some(&["SUCCESS", "FAILED"]), 50))
        .await
        .unwrap();
    assert_eq!(listed.ids, vec!["1", "3"]);
    assert_eq!(
        listed.counts,
        BTreeMap::from([("FAILED".to_string(), 1), ("SUCCESS".to_string(), 1)])
    );
}

#[tokio::test]
async fn pr_строит_самый_долгий_run_head_коммита() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/pulls/42"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "head": { "sha": "abc" } })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/runs"))
        .and(query_param("head_sha", "abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "workflow_runs": [
            listed_run(1, "success", 5), listed_run(2, "success", 20), listed_run(3, "success", 1),
        ] })))
        .mount(&server)
        .await;
    assert_eq!(client(&server).head_pipeline("o/r", "42").await.unwrap(), "2");
}

#[tokio::test]
async fn pr_без_pr_или_без_запусков_это_mr_has_no_pipeline() {
    let server = MockServer::start().await;
    assert_error(
        client(&server).head_pipeline("o/r", "42").await,
        ErrorCode::MrHasNoPipeline,
        &[("iid", "42"), ("project", "o/r")],
    );
    Mock::given(method("GET"))
        .and(path("/repos/o/r/pulls/42"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "head": { "sha": "abc" } })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/runs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "workflow_runs": [] })))
        .mount(&server)
        .await;
    assert_error(
        client(&server).head_pipeline("o/r", "42").await,
        ErrorCode::MrHasNoPipeline,
        &[("iid", "42"), ("project", "o/r")],
    );
}

fn repo_json(full_name: &str) -> Value {
    json!({ "full_name": full_name, "pushed_at": "2026-10-07T09:00:00Z", "default_branch": "main" })
}

fn project(full_name: &str) -> Project {
    Project {
        full_path: full_name.into(),
        name: full_name.into(),
        last_activity_at: Some("2026-10-07T09:00:00Z".into()),
        default_branch: Some("main".into()),
    }
}

#[tokio::test]
async fn проект_и_его_отсутствие() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r"))
        .respond_with(ResponseTemplate::new(200).set_body_json(repo_json("o/r")))
        .mount(&server)
        .await;
    assert_eq!(client(&server).fetch_project("o/r").await.unwrap(), project("o/r"));
    assert_error(
        client(&server).fetch_project("o/none").await,
        ErrorCode::ProjectNotFound,
        &[("host", "github.com"), ("project", "o/none")],
    );
}

#[tokio::test]
async fn свои_репозитории_поиск_по_странице_и_курсор_номером() {
    let server = MockServer::start().await;
    let next = format!("<{}/user/repos?sort=pushed&per_page=100&page=2>; rel=\"next\"", server.uri());
    Mock::given(method("GET"))
        .and(path("/user/repos"))
        .and(query_param("sort", "pushed"))
        .and(query_param("page", "1"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("link", next.as_str())
                .set_body_json(json!([repo_json("o/web"), repo_json("o/api")])),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/user/repos"))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([repo_json("o/web-2")])))
        .mount(&server)
        .await;
    let client = client(&server);
    assert_eq!(
        client.list_projects(" WEB ", None).await.unwrap(),
        Page { items: vec![project("o/web")], next: Some("2".into()) }
    );
    assert_eq!(
        client.list_projects("", Some("2")).await.unwrap(),
        Page { items: vec![project("o/web-2")], next: None }
    );
}

#[tokio::test]
async fn ветки_основная_первой_и_фильтр() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r"))
        .respond_with(ResponseTemplate::new(200).set_body_json(repo_json("o/r")))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/branches"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            { "name": "dev" }, { "name": "feature/x" }, { "name": "main" },
        ])))
        .mount(&server)
        .await;
    let client = client(&server);
    assert_eq!(client.list_branches("o/r", "").await.unwrap(), vec!["main", "dev", "feature/x"]);
    assert_eq!(client.list_branches("o/r", "FEAT").await.unwrap(), vec!["feature/x"]);
}

#[tokio::test]
async fn последние_запуски_без_workflow_и_с_ним() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/runs"))
        .and(query_param("branch", "main"))
        .and(query_param("per_page", "20"))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "workflow_runs": [run_json()] })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/workflows/ci.yml/runs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "workflow_runs": [] })))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server);
    assert_eq!(
        client.recent_pipelines("o/r", Some("main"), None, None).await.unwrap(),
        Page {
            items: vec![Pipeline {
                id: "7".into(),
                iid: "12".into(),
                status: "success".into(),
                source: Some("push".into()),
                created_at: "2026-10-07T10:00:00.000Z".into(),
                duration: Some(600_000),
                commit: Some(Commit { sha: "abc123".into(), title: "fix: x".into() }),
                author: Some("u".into()),
                url: "https://github.com/o/r/actions/runs/7".into(),
            }],
            next: None,
        }
    );
    client.recent_pipelines("o/r", None, Some("ci.yml"), None).await.unwrap();
}

#[tokio::test]
async fn workflow_только_активные_имя_файла_без_пути() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/workflows"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "workflows": [
            { "name": "CI", "path": ".github/workflows/ci.yml", "state": "active" },
            { "name": "Old", "path": ".github/workflows/old.yml", "state": "disabled_manually" },
        ] })))
        .mount(&server)
        .await;
    assert_eq!(
        client(&server).list_workflows("o/r").await.unwrap(),
        vec![Workflow { file: "ci.yml".into(), name: "CI".into() }]
    );
}
```

Run: `cd app && cargo test -p pipeline-trace-core --test github`
Expected: ошибка компиляции — `Client` не реализует `Source`.

- [ ] **Step 2: `lists.rs`**

`app/core/src/github/lists.rs`:

```rust
//! Списки GitHub для формы и агрегата (спека, § 6–7).

use std::collections::BTreeMap;

use serde::Deserialize;
use time::OffsetDateTime;

use super::{Client, WireRun, status};
use crate::browse::{Commit, Page, Pipeline, Project, Workflow};
use crate::error::{Error, ErrorCode};
use crate::gitlab::{Listed, PipelineFilter};
use crate::iso::iso;

const MAX_LIST_PAGES: usize = 10;
const RECENT_PER_PAGE: usize = 20;
const MAX_BRANCHES: usize = 20;

#[derive(Deserialize)]
struct Runs {
    workflow_runs: Vec<WireRun>,
}

#[derive(Deserialize)]
struct Repo {
    full_name: String,
    pushed_at: Option<String>,
    default_branch: Option<String>,
}

impl From<Repo> for Project {
    fn from(r: Repo) -> Self {
        Project {
            full_path: r.full_name.clone(),
            name: r.full_name,
            last_activity_at: r.pushed_at,
            default_branch: r.default_branch,
        }
    }
}

#[derive(Deserialize)]
struct Branch {
    name: String,
}

#[derive(Deserialize)]
struct Pull {
    head: Head,
}

#[derive(Deserialize)]
struct Head {
    sha: String,
}

#[derive(Deserialize)]
struct Workflows {
    workflows: Vec<WireWorkflow>,
}

#[derive(Deserialize)]
struct WireWorkflow {
    name: String,
    path: String,
    state: String,
}

/// Как `encodeURIComponent`: ветка `feature/a+b` не должна сломать запрос.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// `?a=1&b=2` без пустых параметров.
fn query(params: &[(&str, Option<String>)]) -> String {
    let parts: Vec<String> = params
        .iter()
        .filter_map(|(key, value)| value.as_deref().map(|v| format!("{key}={}", encode(v))))
        .collect();
    if parts.is_empty() {
        String::new()
    } else {
        format!("?{}", parts.join("&"))
    }
}

fn runs_path(project: &str, workflow: Option<&str>) -> String {
    match workflow {
        Some(file) => format!("/repos/{project}/actions/workflows/{file}/runs"),
        None => format!("/repos/{project}/actions/runs"),
    }
}

/// Статус формы в параметр `status` списка запусков (спека, § 7).
fn status_param(status: &str) -> Option<&'static str> {
    match status {
        "SUCCESS" => Some("success"),
        "FAILED" => Some("failure"),
        "CANCELED" => Some("cancelled"),
        "RUNNING" => Some("in_progress"),
        "MANUAL" => Some("waiting"),
        _ => None,
    }
}

/// Курсор страниц GitHub — номер страницы текстом.
fn page_number(after: Option<&str>) -> usize {
    after.and_then(|a| a.parse().ok()).unwrap_or(1)
}

fn to_pipeline(run: WireRun) -> Pipeline {
    Pipeline {
        id: run.id.to_string(),
        iid: run.run_number.to_string(),
        status: status(&run.status, run.conclusion.as_deref()).to_lowercase(),
        source: Some(run.event.clone()),
        created_at: iso(run.created_at),
        duration: run
            .completed()
            .then(|| run.duration(run.updated_at).whole_milliseconds() as i64),
        commit: run.head_commit.as_ref().map(|c| Commit {
            sha: run.head_sha.chars().take(8).collect(),
            title: c.message.lines().next().unwrap_or_default().into(),
        }),
        author: run.actor.as_ref().map(|a| a.login.clone()),
        url: run.html_url.clone(),
    }
}

impl Client {
    fn project_not_found(&self, project: &str) -> Error {
        Error::new(ErrorCode::ProjectNotFound)
            .with("host", self.host())
            .with("project", project)
    }

    pub(crate) async fn repo(&self, project: &str) -> Result<Project, Error> {
        let repo: Repo = self
            .json(&format!("/repos/{project}"))
            .await?
            .ok_or_else(|| self.project_not_found(project))?;
        Ok(repo.into())
    }

    /// Свои репозитории от недавно изменённых; поиск — по загруженной странице (у REST его нет).
    pub(crate) async fn repos(&self, search: &str, after: Option<&str>) -> Result<Page<Project>, Error> {
        let page = page_number(after);
        let (repos, next): (Vec<Repo>, _) = self
            .page(&format!("/user/repos?sort=pushed&per_page=100&page={page}"))
            .await?
            .unwrap_or_default();
        let needle = search.trim().to_lowercase();
        Ok(Page {
            items: repos
                .into_iter()
                .filter(|r| r.full_name.to_lowercase().contains(&needle))
                .map(Project::from)
                .collect(),
            next: next.map(|_| (page + 1).to_string()),
        })
    }

    /// Ветки первой страницы по подстроке; основная — первой.
    pub(crate) async fn branches(&self, project: &str, search: &str) -> Result<Vec<String>, Error> {
        let root = self.repo(project).await?.default_branch;
        let found: Vec<Branch> = self
            .json(&format!("/repos/{project}/branches?per_page=100"))
            .await?
            .unwrap_or_default();
        let needle = search.trim().to_lowercase();
        let mut names: Vec<String> = found
            .into_iter()
            .map(|b| b.name)
            .filter(|n| n.to_lowercase().contains(&needle))
            .collect();
        if let Some(i) = root.and_then(|root| names.iter().position(|n| *n == root)) {
            names[..=i].rotate_right(1);
        }
        names.truncate(MAX_BRANCHES);
        Ok(names)
    }

    pub(crate) async fn recent(
        &self,
        project: &str,
        r#ref: Option<&str>,
        workflow: Option<&str>,
        after: Option<&str>,
    ) -> Result<Page<Pipeline>, Error> {
        let page = page_number(after);
        let target = format!(
            "{}{}",
            runs_path(project, workflow),
            query(&[
                ("branch", r#ref.map(String::from)),
                ("per_page", Some(RECENT_PER_PAGE.to_string())),
                ("page", Some(page.to_string())),
            ])
        );
        let (runs, next): (Runs, _) = self
            .page(&target)
            .await?
            .ok_or_else(|| self.project_not_found(project))?;
        Ok(Page {
            items: runs.workflow_runs.into_iter().map(to_pipeline).collect(),
            next: next.map(|_| (page + 1).to_string()),
        })
    }

    pub(crate) async fn workflow_list(&self, project: &str) -> Result<Vec<Workflow>, Error> {
        let found: Workflows = self
            .json(&format!("/repos/{project}/actions/workflows?per_page=100"))
            .await?
            .ok_or_else(|| self.project_not_found(project))?;
        Ok(found
            .workflows
            .into_iter()
            .filter(|w| w.state == "active")
            .map(|w| Workflow {
                file: w.path.rsplit('/').next().unwrap_or(&w.path).to_string(),
                name: w.name,
            })
            .collect())
    }

    /// Запуски по фильтру агрегата, не больше `filter.last`; несколько статусов — фильтр на клиенте.
    pub(crate) async fn list_runs(&self, project: &str, filter: &PipelineFilter) -> Result<Listed, Error> {
        let single = filter
            .statuses
            .as_ref()
            .filter(|s| s.len() == 1)
            .and_then(|s| status_param(&s[0]));
        let mut next = Some(format!(
            "{}{}",
            runs_path(project, filter.workflow.as_deref()),
            query(&[
                ("branch", filter.r#ref.clone()),
                ("event", filter.source.clone()),
                ("status", single.map(String::from)),
                ("per_page", Some("100".into())),
            ])
        ));
        let mut listed = Listed {
            ids: Vec::new(),
            counts: BTreeMap::new(),
        };
        for _ in 0..MAX_LIST_PAGES {
            if listed.ids.len() >= filter.last {
                break;
            }
            let Some(target) = next.take() else { break };
            let (runs, link): (Runs, _) = self
                .page(&target)
                .await?
                .ok_or_else(|| self.project_not_found(project))?;
            for run in runs.workflow_runs {
                if listed.ids.len() >= filter.last {
                    break;
                }
                let s = status(&run.status, run.conclusion.as_deref());
                if filter.statuses.as_ref().is_some_and(|w| !w.iter().any(|w| w == s)) {
                    continue;
                }
                listed.ids.push(run.id.to_string());
                *listed.counts.entry(s.to_string()).or_default() += 1;
            }
            next = link;
        }
        Ok(listed)
    }

    /// Самый долгий run head-коммита PR (спека, § 6).
    pub(crate) async fn head_run(&self, project: &str, number: &str) -> Result<String, Error> {
        let none = || {
            Error::new(ErrorCode::MrHasNoPipeline)
                .with("iid", number)
                .with("project", project)
        };
        let pull: Pull = self
            .json(&format!("/repos/{project}/pulls/{number}"))
            .await?
            .ok_or_else(none)?;
        let runs: Runs = self
            .json(&format!(
                "/repos/{project}/actions/runs?head_sha={}&per_page=100",
                pull.head.sha
            ))
            .await?
            .ok_or_else(none)?;
        let now = OffsetDateTime::now_utc();
        runs.workflow_runs
            .iter()
            .max_by_key(|r| r.duration(now))
            .map(|r| r.id.to_string())
            .ok_or_else(none)
    }
}
```

- [ ] **Step 3: `impl Source` в `github.rs`**

В `app/core/src/github.rs`: `mod lists;` после `mod client;`; в `use` — `use crate::browse::{Page, Pipeline, Project, Workflow as Flow};`, `use crate::gitlab::{Listed, PipelineFilter};`, `use crate::source::{Provider, Source};` (`Workflow` из `browse` переименован в `Flow`: имя `Workflow` в модуле занято файлом workflow). В конец, перед `#[cfg(test)]`:

```rust
impl Source for Client {
    fn host(&self) -> &str {
        Client::host(self)
    }

    fn provider(&self) -> Provider {
        Provider::Github
    }

    async fn fetch_pipeline(&self, project: &str, id: &str) -> Result<RawPipeline, Error> {
        self.fetch_run(project, id).await
    }

    async fn list_pipelines(&self, project: &str, filter: &PipelineFilter) -> Result<Listed, Error> {
        self.list_runs(project, filter).await
    }

    async fn head_pipeline(&self, project: &str, number: &str) -> Result<String, Error> {
        self.head_run(project, number).await
    }

    async fn fetch_project(&self, path: &str) -> Result<Project, Error> {
        self.repo(path).await
    }

    async fn list_projects(&self, search: &str, after: Option<&str>) -> Result<Page<Project>, Error> {
        self.repos(search, after).await
    }

    async fn list_branches(&self, project: &str, search: &str) -> Result<Vec<String>, Error> {
        self.branches(project, search).await
    }

    async fn recent_pipelines(
        &self,
        project: &str,
        r#ref: Option<&str>,
        workflow: Option<&str>,
        after: Option<&str>,
    ) -> Result<Page<Pipeline>, Error> {
        self.recent(project, r#ref, workflow, after).await
    }

    async fn list_workflows(&self, project: &str) -> Result<Vec<Flow>, Error> {
        self.workflow_list(project).await
    }
}
```

- [ ] **Step 4: Тесты зелёные**

Run: `cd app && cargo test -p pipeline-trace-core`
Expected: PASS, все; в `tests/github.rs` — 4 из Task 4 и 10 новых.

Run: `cd app && cargo clippy -p pipeline-trace-core --all-targets -- -D warnings`
Expected: без предупреждений (`dead_code` из Task 3–4 ушли).

- [ ] **Step 5: Commit**

```bash
git add app/core/src/github.rs app/core/src/github/lists.rs app/core/tests/github.rs
git commit -m "feat(core): списки GitHub и Source для клиента GitHub"
```

### Task 6: Ссылки GitHub, ввод проекта, поле `workflow`

**Files:**
- Modify: `app/core/src/request.rs`, `app/core/src/error.rs`, `app/core/src/report.rs`
- Modify (тесты): `app/core/tests/request.rs`, `app/core/tests/project_input.rs`, `app/core/tests/build_report.rs`, `app/core/tests/history.rs`
- Modify (фронтенд, чтобы типы сошлись): `app/src/entities/history-entry/model/forms.ts`, `app/src/app/dev/mock.ts`
- Generated: `app/src/shared/api/schema/Form.ts`, `Request.ts`, `Field.ts`

**Interfaces:**
- Consumes: `Provider`, `GITHUB_COM` (Task 1, 3); `PipelineFilter.workflow` (Task 1); `ErrorCode::WorkflowRequired` (Task 3).
- Produces:
  - `Form.workflow: String` (`#[serde(default)]` уже на структуре — старые записи истории читаются).
  - `Request::Aggregate { …, workflow: Option<String> }` с `#[serde(default)]` на поле.
  - `Field::Workflow`.
  - `pub fn request::link_provider(url: &str) -> Option<Provider>` — тип хоста по форме ссылки; `None` — не ссылка на пайплайн/MR/run/PR.
  - `parse_form` распознаёт ссылки GitHub: run → `Request::Pipeline`, PR → `Request::Mr`.
  - `parse_project_input` понимает страницы GitHub и `ssh.github.com`.

- [ ] **Step 1: Тесты — падают**

Дописать в `app/core/tests/request.rs` (в `use` добавить `link_provider` из `request` и `pipeline_trace_core::source::Provider`):

```rust
#[test]
fn ссылки_github_на_run_job_attempt_и_pr() {
    let run = |url: &str| parse_form(&link(url)).map(|p| (p.host, p.request));
    let pipeline = |id: &str| Request::Pipeline {
        project: "o/r".into(),
        pipeline_id: id.into(),
    };
    for url in [
        "https://github.com/o/r/actions/runs/7",
        "https://github.com/o/r/actions/runs/7/job/99",
        "https://github.com/o/r/actions/runs/7/attempts/2",
        " https://github.com/o/r/actions/runs/7?pr=3 ",
    ] {
        assert_eq!(run(url), Ok(("github.com".into(), pipeline("7"))), "{url}");
    }
    assert_eq!(
        run("https://ghe.example/o/r/pull/42/files"),
        Ok((
            "ghe.example".into(),
            Request::Mr {
                project: "o/r".into(),
                mr_iid: "42".into(),
            }
        ))
    );
}

#[test]
fn тип_хоста_по_форме_ссылки() {
    assert_eq!(link_provider("https://ghe.example/o/r/actions/runs/7"), Some(Provider::Github));
    assert_eq!(link_provider("https://h.example/g/p/-/pipelines/1"), Some(Provider::Gitlab));
    assert_eq!(link_provider("https://h.example/g/p"), None);
}

#[test]
fn агрегат_берёт_workflow_из_формы() {
    let parsed = parse_form(&aggregate(|f| f.workflow = " ci.yml ".into())).unwrap();
    let Request::Aggregate { workflow, .. } = parsed.request else {
        panic!("ждали агрегат")
    };
    assert_eq!(workflow.as_deref(), Some("ci.yml"));
}

#[test]
fn старая_запись_истории_без_workflow_читается() {
    let request: Request = serde_json::from_value(serde_json::json!({
        "mode": "aggregate", "project": "g/p", "ref": null, "source": null, "last": 5, "statuses": null,
    }))
    .unwrap();
    assert!(matches!(request, Request::Aggregate { workflow: None, .. }));
}
```

В существующих литералах `Request::Aggregate { … }` добавить `workflow: None`: `app/core/tests/request.rs` (строки ~125, ~148, ~164), `app/core/tests/build_report.rs` (~61). В литерале `Form { … }` в `app/core/tests/history.rs` (~21) добавить `workflow: String::new(),`.

Дописать в `app/core/tests/project_input.rs`:

```rust
#[test]
fn страницы_github_дают_репозиторий() {
    for input in [
        "https://github.com/o/r",
        "https://github.com/o/r.git",
        "https://github.com/o/r/actions/runs/7",
        "https://github.com/o/r/pull/42",
        "https://github.com/o/r/tree/main/src",
        "https://github.com/o/r/blob/main/README.md",
        "git@github.com:o/r.git",
        "ssh://git@ssh.github.com:443/o/r.git",
    ] {
        assert_eq!(parse_project_input(input), r("github.com", "o/r"), "{input}");
    }
}
```

Run: `cd app && cargo test -p pipeline-trace-core --test request --test project_input`
Expected: ошибка компиляции — нет `link_provider`, у `Form` нет `workflow`.

- [ ] **Step 2: `Field::Workflow`**

`app/core/src/error.rs`, enum `Field` — после `Statuses`: `Workflow,`.

- [ ] **Step 3: `request.rs`**

Импорт: `use crate::source::{GITHUB_COM, Provider};`.

Шаблон ссылки GitHub рядом с `LINK`:

```rust
/// `https://host/owner/repo/actions/runs/<id>[/job/…|/attempts/…]` или `…/pull/<n>[/…]`.
static GITHUB_LINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^https?://([^/]+)/([^/]+/[^/]+)/(actions/runs|pull)/([0-9]+)(?:[/?#]|$)")
        .expect("верный шаблон")
});
/// Третий сегмент пути на github: страница репозитория, а не часть пути проекта.
const GITHUB_PAGES: [&str; 6] = ["actions", "pull", "tree", "blob", "commit", "issues"];
```

`Form` — после `statuses`:

```rust
    /// файл workflow GitHub (`ci.yml`); GitLab поле не читает
    pub workflow: String,
```

`Request::Aggregate` — после `statuses`:

```rust
        /// файл workflow GitHub; старые записи истории — `None`
        #[serde(default)]
        workflow: Option<String>,
```

Новая функция и правки разбора:

```rust
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
```

В `parse_remote`: цепочку `LINK.captures(input)` продолжить `.or_else(|| GITHUB_LINK.captures(input))` перед `REPO_HTTP`; хост и хвост:

```rust
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
```

```rust
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
```

`parse_aggregate`, в литерал `Request::Aggregate`: `workflow: non_empty(&form.workflow),`.

- [ ] **Step 4: `report.rs` — workflow в фильтр, подпись и имя файла**

В `resolve`, ветка `Request::Aggregate`: в деструктуризацию добавить `workflow`, в `PipelineFilter` — `workflow: workflow.clone()` вместо `None`, суффикс:

```rust
            let filter_part = r#ref.as_deref().or(event.as_deref()).unwrap_or("all");
            Ok(Target {
                ids: listed.ids,
                suffix: match workflow {
                    Some(file) => format!("{}-{filter_part}", workflow_stem(file)),
                    None => filter_part.to_string(),
                },
                status_counts: Some(listed.counts),
            })
```

```rust
/// `ci.yml` → `ci`.
fn workflow_stem(file: &str) -> &str {
    file.strip_suffix(".yml")
        .or_else(|| file.strip_suffix(".yaml"))
        .unwrap_or(file)
}
```

В `build_report` подпись агрегата — workflow первым:

```rust
        Request::Aggregate {
            r#ref,
            source,
            workflow,
            ..
        } => {
            let parts: Vec<&str> = [workflow, r#ref, source]
                .into_iter()
                .filter_map(|p| p.as_deref())
                .collect();
            (Some(parts.join(" · ")).filter(|l| !l.is_empty()), true)
        }
```

Тест на имя файла и подпись — в Task 8 (сквозной GitHub).

- [ ] **Step 5: Фронтенд — поле формы**

`app/src/entities/history-entry/model/forms.ts`: в `blank` добавить `workflow: ''`; тип `Aggregate` получает `workflow: string`.

```ts
const blank: Form = { mode: 'link', host: '', url: '', project: '', ref: '', source: '', last: '', statuses: [], workflow: '' }

type Aggregate = { host: string; project: string; ref: string; workflow: string; last: string; statuses: readonly Status[] | null }
```

`app/src/features/build-aggregate/model/useBuildAggregate.ts` пока передаёт `workflow: ''` (настоящий выбор — Task 10):

```ts
  return { ...build, start: () => start(aggregateForm({ host, project, ref: branch ?? '', workflow: '', ...readAggSettings() })) }
```

`app/src/app/dev/mock.ts`: в записях истории, где есть `form: { … }`, добавить `workflow: ''`.

- [ ] **Step 6: Тесты и типы зелёные**

Run: `cd app && cargo test -p pipeline-trace-core`
Expected: PASS.

Run: `cd app && npm run typecheck`
Expected: без ошибок.

- [ ] **Step 7: Commit**

```bash
git add app/core app/src/shared/api/schema app/src/entities/history-entry app/src/features/build-aggregate app/src/app/dev/mock.ts
git commit -m "feat(core): ссылки и проекты GitHub, workflow в форме агрегата"
```

### Task 7: Тип хоста и токены GitHub

**Files:**
- Modify: `app/core/src/settings.rs`, `app/core/src/source.rs`, `app/core/src/tokens.rs`
- Modify (тесты): `app/core/tests/settings.rs`, `app/core/tests/source.rs`, `app/core/tests/tokens.rs`
- Modify (мок): `app/src/app/dev/mock.ts`
- Generated: `app/src/shared/api/schema/TokenSource.ts`, `HostInfo.ts`

**Interfaces:**
- Consumes: `Provider`, `GITHUB_COM` (Task 1, 3), `github::probe_at` (Task 3).
- Produces:
  - `Settings::host_kind(&self, host: &str) -> Result<Option<Provider>, Error>` — `github.com` всегда `Some(Github)`; иначе из `hostKinds`.
  - `Settings::set_host_kind(&self, host: &str, provider: Provider) -> Result<(), Error>` — `github.com` не пишется.
  - `pub async fn source::resolve_provider(host: &str, settings: &Settings) -> Result<Provider, Error>` и тестовый вход `resolve_provider_at(host, settings, base_url)`: кэш → проба → запись в кэш.
  - `TokenSource::Gh` (`"gh"`).
  - `HostInfo.provider: Option<Provider>` — `glab` и `GITLAB_TOKEN` дают `Some(Gitlab)`, `gh` и `GH_TOKEN` — `Some(Github)`, связка ключей — `None` (заполняют команды из `hostKinds`, Task 9).
  - `pub fn tokens::find_github_token(host, store, env, gh) -> Result<String, Error>`, `pub fn tokens::github_hosts(env, gh) -> Vec<HostInfo>`, `pub struct tokens::Gh` с `find()` и `value(args)`.

- [ ] **Step 1: Тесты — падают**

Дописать в `app/core/tests/settings.rs` (в `use` — `pipeline_trace_core::source::Provider`):

```rust
#[test]
fn тип_хоста_github_com_без_кэша_остальные_из_кэша() {
    let (_dir, settings) = settings();
    assert_eq!(settings.host_kind("github.com").unwrap(), Some(Provider::Github));
    assert_eq!(settings.host_kind("ghe.example").unwrap(), None);
    settings.set_host_kind("ghe.example", Provider::Github).unwrap();
    settings.set_host_kind("gl.example", Provider::Gitlab).unwrap();
    assert_eq!(settings.host_kind("ghe.example").unwrap(), Some(Provider::Github));
    assert_eq!(settings.host_kind("gl.example").unwrap(), Some(Provider::Gitlab));
    // кэш не мешает остальным настройкам
    settings.update(SettingsPatch { theme: Some(Theme::Dark), ..Default::default() }).unwrap();
    assert_eq!(settings.host_kind("ghe.example").unwrap(), Some(Provider::Github));
}
```

Дописать в `app/core/tests/source.rs` (в `use` — `pipeline_trace_core::settings::Settings`, `pipeline_trace_core::source::resolve_provider_at`, `tempfile::TempDir`, `wiremock::matchers::{method, path}`, `wiremock::{Mock, MockServer, ResponseTemplate}`):

```rust
#[tokio::test]
async fn тип_хоста_проба_один_раз_дальше_кэш() {
    let dir = TempDir::new().unwrap();
    let settings = Settings::new(dir.path().join("settings.json"));
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/meta"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "installed_version": "3.14.0" })))
        .expect(1)
        .mount(&server)
        .await;
    for _ in 0..2 {
        assert_eq!(
            resolve_provider_at("ghe.example", &settings, &server.uri()).await.unwrap(),
            Provider::Github
        );
    }
}

#[tokio::test]
async fn github_com_без_пробы() {
    let dir = TempDir::new().unwrap();
    let settings = Settings::new(dir.path().join("settings.json"));
    // адрес, по которому никто не слушает: проба упала бы с `network`
    assert_eq!(
        resolve_provider_at("github.com", &settings, "http://127.0.0.1:9").await.unwrap(),
        Provider::Github
    );
}
```

Дописать в `app/core/tests/tokens.rs` (в `use` из `tokens` — `find_github_token`, `github_hosts`; плюс `pipeline_trace_core::source::Provider`):

```rust
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
    let gh = |args: &[&str]| (args == ["auth", "token", "--hostname", "github.com"]).then(|| "gh-cli".to_string());
    assert_eq!(find_github_token("github.com", &f.tokens, &env, &gh).unwrap(), "gh-cli");
    assert_eq!(find_github_token("github.com", &f.tokens, &env, &no_cli).unwrap(), "gh-env");
    f.tokens.set("github.com", "stored").unwrap();
    assert_eq!(find_github_token("github.com", &f.tokens, &env, &gh).unwrap(), "stored");
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
    assert_eq!(find_github_token("ghe.example", &f.tokens, &env, &no_cli).unwrap(), "ent");
    let err = find_github_token("other.example", &f.tokens, &env, &no_cli).unwrap_err();
    assert_eq!(err.code, ErrorCode::NoToken);
}

#[test]
fn хосты_github_из_gh_и_окружения() {
    let status = r#"{"hosts":{"github.com":[{"state":"success"}],"ghe.example":[{"state":"success"}],"dead.example":[{"state":"error"}]}}"#;
    let gh = |args: &[&str]| (args == ["auth", "status", "--json", "hosts"]).then(|| status.to_string());
    let info = |host: &str, source| HostInfo {
        host: host.into(),
        source,
        provider: Some(Provider::Github),
    };
    let no_env = |_: &str| None;
    assert_eq!(
        github_hosts(&no_env, &gh),
        vec![info("ghe.example", TokenSource::Gh), info("github.com", TokenSource::Gh)]
    );
    let env = |name: &str| (name == "GITHUB_TOKEN").then(|| "t".to_string());
    assert_eq!(github_hosts(&env, &no_cli), vec![info("github.com", TokenSource::Env)]);
}
```

В существующем тесте со `list_hosts` (строка ~358) хелпер `info` получает провайдера по источнику:

```rust
    let info = |host: &str, source: TokenSource| HostInfo {
        host: host.into(),
        source,
        provider: (source != TokenSource::Keychain).then_some(Provider::Gitlab),
    };
```

Run: `cd app && cargo test -p pipeline-trace-core --test settings --test source --test tokens`
Expected: ошибка компиляции — нет `host_kind`, `resolve_provider_at`, `find_github_token`.

- [ ] **Step 2: `settings.rs` — кэш типа хоста**

Импорт `use std::collections::BTreeMap;` и `use crate::source::{GITHUB_COM, Provider};`. В `Stored` — поле:

```rust
    /// хост → тип; `github.com` не пишется — он известен
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    host_kinds: BTreeMap<String, Provider>,
```

Методы `Settings`:

```rust
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
```

- [ ] **Step 3: `source.rs` — определение провайдера**

```rust
/// Тип хоста (спека, § 3): кэш, иначе проба `/api/v3/meta` с записью в кэш. Сбой сети пробы — ошибка, кэш не пишется.
pub async fn resolve_provider(host: &str, settings: &Settings) -> Result<Provider, Error> {
    resolve_provider_at(host, settings, &format!("https://{host}")).await
}

/// `base_url` — адрес хоста; тесты подставляют wiremock.
pub async fn resolve_provider_at(host: &str, settings: &Settings, base_url: &str) -> Result<Provider, Error> {
    if let Some(known) = settings.host_kind(host)? {
        return Ok(known);
    }
    let probed = github::probe_at(host, base_url).await?;
    settings.set_host_kind(host, probed)?;
    Ok(probed)
}
```

Импорты: `use crate::github;`, `use crate::settings::Settings;`.

- [ ] **Step 4: `tokens.rs` — `gh`, токен и хосты GitHub**

Импорты: `use std::collections::BTreeMap;`, `use serde::Deserialize;`, `use crate::source::{GITHUB_COM, Provider};`.

`TokenSource` — после `Env`: `Gh,` (doc-комментарий enum: «Откуда у хоста токен; «Удалить» в настройках есть только у `Keychain`.» — без изменений).

`HostInfo`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct HostInfo {
    pub host: String,
    pub source: TokenSource,
    /// `None` — хост из связки ключей, тип ещё не определён
    pub provider: Option<Provider>,
}
```

В `list_hosts`: связка ключей — `provider: None`; замыкание `push` получает провайдера `Some(Provider::Gitlab)`:

```rust
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
```

Поиск исполняемого файла — общий для `glab` и `gh`. Заменить тела `Glab::find_in` и `Glab::value` вызовами общих функций и добавить `Gh`:

```rust
fn find_tool(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    let name = format!("{name}{}", env::consts::EXE_SUFFIX);
    dirs.iter().map(|dir| dir.join(&name)).find(|path| path.is_file())
}

/// Вывод команды; любой сбой и пустой вывод — `None`: CLI может быть не настроен, это не ошибка.
// ponytail: без таймаута, `config get` и `auth token` локальные и мгновенные
fn tool_value(path: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new(path).args(args).stdin(Stdio::null()).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string()).filter(|s| !s.is_empty())
}
```

```rust
    pub fn find_in(dirs: &[PathBuf]) -> Option<Glab> {
        find_tool("glab", dirs).map(Glab)
    }

    pub fn value(&self, args: &[&str]) -> Option<String> {
        tool_value(&self.0, args)
    }
```

```rust
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
```

Токен и хосты GitHub:

```rust
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
```

- [ ] **Step 5: Мок фронтенда и подпись источника `gh`**

`app/src/app/dev/mock.ts`, `hosts`:

```ts
const hosts: HostInfo[] = [
  { host: HOST, source: 'keychain', provider: 'gitlab' },
  { host: 'gitlab.com', source: 'glab', provider: 'gitlab' },
  { host: 'github.com', source: 'gh', provider: 'github' },
]
```

Настройки подписывают источник ключом `form.settings.source.<источник>`. Добавить `gh` в обе локали:

`ru.json`: `"source": { "keychain": "связка ключей", "glab": "glab", "gh": "gh", "env": "переменные окружения" },`

`en.json`: `"source": { "keychain": "keychain", "glab": "glab", "gh": "gh", "env": "environment variables" },`

- [ ] **Step 6: Тесты и типы зелёные**

Run: `cd app && cargo test -p pipeline-trace-core`
Expected: PASS.

Run: `cd app && npm run typecheck && npm run check:locales`
Expected: без ошибок.

- [ ] **Step 7: Commit**

```bash
git add app/core app/src/shared/api/schema app/src/app/dev/mock.ts app/src/shared/i18n/locales
git commit -m "feat(core): тип хоста с пробой и кэшем, токены GitHub из gh и окружения"
```

### Task 8: `Meta.provider`, `Meta.needsMissing`, сквозная сборка отчёта по GitHub

**Files:**
- Modify: `app/core/src/schema.rs`, `app/core/src/report.rs`
- Modify (тесты): `app/core/tests/github.rs`, плюс литералы `Meta { … }` в `app/core/tests/render.rs` и `app/core/tests/report.rs`
- Modify (фикстуры фронтенда): `app/src/shared/api/fixtures/single.json`, `app/src/shared/api/fixtures/aggregate.json`
- Generated: `app/src/shared/api/schema/Meta.ts`

**Interfaces:**
- Consumes: `Source::provider` (Task 1), `RawPipeline.needs_missing` (Task 1), `impl Source for github::Client` (Task 5), workflow в агрегате (Task 6).
- Produces: `Meta { …, provider: Provider, needs_missing: bool }` → в JSON `provider: "gitlab" | "github"`, `needsMissing: boolean`.

- [ ] **Step 1: Тесты — падают**

Дописать в `app/core/tests/github.rs` (в `use` — `pipeline_trace_core::report::{BuildEnv, build_report}`, `pipeline_trace_core::request::Request`, `pipeline_trace_core::schema::Locale`):

```rust
const ENV: BuildEnv = BuildEnv {
    now: datetime!(2026-10-07 11:00:00 UTC),
    locale: Locale::Ru,
};

/// Узел отчёта по имени и виду.
fn node<'a>(report: &'a Value, kind: &str, name: &str) -> &'a Value {
    report["tree"]["nodes"]
        .as_object()
        .unwrap()
        .values()
        .find(|n| n["kind"] == kind && n["name"] == name)
        .unwrap_or_else(|| panic!("нет узла {kind} {name}"))
}

#[tokio::test]
async fn отчёт_по_run_зависимости_из_needs_и_провайдер() {
    let server = MockServer::start().await;
    mount_run(&server, Some(YAML)).await;
    let built = build_report(
        &client(&server),
        &Request::Pipeline { project: "o/r".into(), pipeline_id: "7".into() },
        ENV,
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(built.file_name, "pipeline-trace-o-r-7.html");
    let report = serde_json::to_value(&built.report).unwrap();
    assert_eq!(report["meta"]["provider"], "github");
    assert_eq!(report["meta"]["needsMissing"], false);
    assert_eq!(report["meta"]["label"], "#12");
    let build = node(&report, "job", "build");
    let shard = node(&report, "job", "test: [linux]");
    assert_eq!(shard["deps"], json!([build["id"]]));
}

#[tokio::test]
async fn агрегат_по_workflow_подпись_и_имя_файла() {
    let server = MockServer::start().await;
    mount_run(&server, Some(YAML)).await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/actions/workflows/ci.yml/runs"))
        .and(query_param("branch", "main"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "workflow_runs": [listed_run(7, "success", 40)] })))
        .mount(&server)
        .await;
    let built = build_report(
        &client(&server),
        &Request::Aggregate {
            project: "o/r".into(),
            r#ref: Some("main".into()),
            source: None,
            last: 10,
            statuses: None,
            workflow: Some("ci.yml".into()),
        },
        ENV,
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(built.file_name, "pipeline-trace-o-r-ci-main.html");
    let report = serde_json::to_value(&built.report).unwrap();
    assert_eq!(report["meta"]["label"], "ci.yml · main");
    assert_eq!(report["meta"]["statusCounts"], json!({ "SUCCESS": 1 }));
}

#[tokio::test]
async fn без_файла_workflow_отчёт_помечен() {
    let server = MockServer::start().await;
    mount_run(&server, None).await;
    let built = build_report(
        &client(&server),
        &Request::Pipeline { project: "o/r".into(), pipeline_id: "7".into() },
        ENV,
        |_| {},
    )
    .await
    .unwrap();
    let report = serde_json::to_value(&built.report).unwrap();
    assert_eq!(report["meta"]["needsMissing"], true);
}
```

Дописать в `app/core/tests/build_report.rs` проверку провайдера GitLab — в первый тест, где строится одиночный пайплайн, после сборки:

```rust
    let json = serde_json::to_value(&built.report).unwrap();
    assert_eq!(json["meta"]["provider"], "gitlab");
    assert_eq!(json["meta"]["needsMissing"], false);
```

(имя переменной результата сборки — как в этом тесте; если тест не связывает результат с именем, связать: `let built = build_report(…).await.unwrap();`).

Run: `cd app && cargo test -p pipeline-trace-core --test github --test build_report`
Expected: FAIL — в `meta` нет `provider` (`Null != "github"`).

- [ ] **Step 2: `Meta`**

`app/core/src/schema.rs`, `Meta` — после `generated_at`:

```rust
    pub provider: Provider,
    /// GitHub: файл workflow не загрузился — стейджи и связи джоб неизвестны
    pub needs_missing: bool,
```

Импорт `use crate::source::Provider;`. Литералы `Meta { … }` в `app/core/tests/render.rs` и `app/core/tests/report.rs` дополнить `provider: Provider::Gitlab, needs_missing: false,` (импорт `pipeline_trace_core::source::Provider`).

- [ ] **Step 3: `report.rs`**

`load_trees` возвращает и признак:

```rust
async fn load_trees(
    source: &impl Source,
    project: &str,
    ids: &[String],
    env: BuildEnv,
    on_progress: impl Fn(Progress) + Send + Sync,
) -> Result<(Vec<Span>, bool), Error> {
    // … загрузка как раньше …
    let needs_missing = raws.iter().any(|raw| raw.needs_missing);
    let base_url = format!("https://{}", source.host());
    let trees = raws
        .iter()
        .map(|raw| build_tree(raw, &base_url, env.now))
        .collect();
    Ok((trees, needs_missing))
}
```

В `build_report`: `let (trees, needs_missing) = load_trees(…).await?;` и в `Meta` — `provider: source.provider(), needs_missing,`.

- [ ] **Step 4: Фикстуры фронтенда**

В `"meta": { … }` файлов `app/src/shared/api/fixtures/single.json` и `aggregate.json` добавить `"provider": "gitlab", "needsMissing": false`.

- [ ] **Step 5: Тесты и типы зелёные**

Run: `cd app && cargo test -p pipeline-trace-core`
Expected: PASS.

Run: `cd app && npm run typecheck`
Expected: без ошибок.

- [ ] **Step 6: Commit**

```bash
git add app/core app/src/shared/api/schema app/src/shared/api/fixtures
git commit -m "feat(core): провайдер и признак needsMissing в отчёте, сквозная сборка по GitHub"
```

### Task 9: Команды Tauri — источник по типу хоста, `workflows`, `host_provider`

**Files:**
- Modify: `app/core/src/source.rs` (диспетчер `AnySource`)
- Modify: `app/core/tests/source.rs`
- Modify: `app/src-tauri/src/commands.rs`, `app/src-tauri/src/main.rs`, `app/src-tauri/build.rs`, `app/src-tauri/capabilities/form.json`
- Modify (фронтенд, сигнатуры): `app/src/shared/api/api.ts`, `app/src/shared/api/index.ts`, `app/src/app/dev/mock.ts`

**Interfaces:**
- Consumes: всё из Task 1–8.
- Produces:
  - `pub enum source::AnySource { Gitlab(gitlab::Client), Github(github::Client) }` с `impl Source`.
  - Команды: `workflows(host, project) -> Vec<Workflow>`, `host_provider(host) -> Provider`; `pipelines(host, project, ref, workflow, after)` — новый аргумент `workflow: Option<String>`.
  - `build`: ссылка пишет тип хоста в кэш (`link_provider`); агрегат GitHub без workflow — `CmdError::Fields { Workflow: workflowRequired }`.
  - `hosts`: хосты `gh`/`GH_TOKEN` добавлены, `provider` хостов связки ключей заполнен из кэша.
  - Фронтенд: `api.workflows(host, project)`, `api.hostProvider(host)`, `api.pipelines(host, project, ref, workflow, after)`.

- [ ] **Step 1: Тест диспетчера — падает**

Дописать в `app/core/tests/source.rs` (в `use` — `pipeline_trace_core::source::AnySource`, `pipeline_trace_core::github`, `pipeline_trace_core::gitlab`):

```rust
#[tokio::test]
async fn диспетчер_отдаёт_провайдера_и_хост_варианта() {
    let gl = AnySource::Gitlab(gitlab::Client::new("gl.example", "t").unwrap());
    let gh = AnySource::Github(github::Client::new("github.com", "t").unwrap());
    assert_eq!((gl.provider(), gl.host()), (Provider::Gitlab, "gl.example"));
    assert_eq!((gh.provider(), gh.host()), (Provider::Github, "github.com"));
}
```

Run: `cd app && cargo test -p pipeline-trace-core --test source`
Expected: ошибка компиляции — нет `AnySource`.

- [ ] **Step 2: `AnySource`**

В конец `app/core/src/source.rs`:

```rust
/// Клиент хоста, выбранный по типу: команды формы работают с ним через `Source`.
pub enum AnySource {
    Gitlab(gitlab::Client),
    Github(github::Client),
}

impl Source for AnySource {
    fn host(&self) -> &str {
        match self {
            Self::Gitlab(c) => Source::host(c),
            Self::Github(c) => Source::host(c),
        }
    }

    fn provider(&self) -> Provider {
        match self {
            Self::Gitlab(c) => c.provider(),
            Self::Github(c) => c.provider(),
        }
    }

    async fn fetch_pipeline(&self, project: &str, id: &str) -> Result<RawPipeline, Error> {
        match self {
            Self::Gitlab(c) => c.fetch_pipeline(project, id).await,
            Self::Github(c) => c.fetch_pipeline(project, id).await,
        }
    }

    async fn list_pipelines(&self, project: &str, filter: &PipelineFilter) -> Result<Listed, Error> {
        match self {
            Self::Gitlab(c) => c.list_pipelines(project, filter).await,
            Self::Github(c) => c.list_pipelines(project, filter).await,
        }
    }

    async fn head_pipeline(&self, project: &str, number: &str) -> Result<String, Error> {
        match self {
            Self::Gitlab(c) => c.head_pipeline(project, number).await,
            Self::Github(c) => c.head_pipeline(project, number).await,
        }
    }

    async fn fetch_project(&self, path: &str) -> Result<Project, Error> {
        match self {
            Self::Gitlab(c) => c.fetch_project(path).await,
            Self::Github(c) => c.fetch_project(path).await,
        }
    }

    async fn list_projects(&self, search: &str, after: Option<&str>) -> Result<Page<Project>, Error> {
        match self {
            Self::Gitlab(c) => c.list_projects(search, after).await,
            Self::Github(c) => c.list_projects(search, after).await,
        }
    }

    async fn list_branches(&self, project: &str, search: &str) -> Result<Vec<String>, Error> {
        match self {
            Self::Gitlab(c) => c.list_branches(project, search).await,
            Self::Github(c) => c.list_branches(project, search).await,
        }
    }

    async fn recent_pipelines(
        &self,
        project: &str,
        r#ref: Option<&str>,
        workflow: Option<&str>,
        after: Option<&str>,
    ) -> Result<Page<Pipeline>, Error> {
        match self {
            Self::Gitlab(c) => c.recent_pipelines(project, r#ref, workflow, after).await,
            Self::Github(c) => c.recent_pipelines(project, r#ref, workflow, after).await,
        }
    }

    async fn list_workflows(&self, project: &str) -> Result<Vec<Workflow>, Error> {
        match self {
            Self::Gitlab(c) => c.list_workflows(project).await,
            Self::Github(c) => c.list_workflows(project).await,
        }
    }
}
```

Run: `cd app && cargo test -p pipeline-trace-core --test source`
Expected: PASS.

- [ ] **Step 3: `commands.rs`**

Импорты ядра заменить на:

```rust
use std::collections::BTreeMap;

use pipeline_trace_core::browse::{Page, Pipeline, Project, Workflow};
use pipeline_trace_core::error::{CmdError, Error, ErrorCode, Field};
use pipeline_trace_core::history::{HistoryEntry, HistoryLabel, NewEntry};
use pipeline_trace_core::hosts::normalize_host;
use pipeline_trace_core::projects::SavedProject;
use pipeline_trace_core::render::render;
use pipeline_trace_core::report::{BuildEnv, Progress, build_report};
use pipeline_trace_core::request::{
    Form, FormMode, ProjectRef, Request, link_provider, parse_form, parse_project_input,
};
use pipeline_trace_core::schema::{Meta, Report};
use pipeline_trace_core::settings::{AppSettings, SettingsPatch};
use pipeline_trace_core::source::{AnySource, Provider, Source, resolve_provider};
use pipeline_trace_core::tokens::{
    Gh, Glab, HostInfo, find_github_token, find_token, github_hosts, list_hosts,
};
use pipeline_trace_core::{github, gitlab};
```

`client_for` заменить на `source_for`:

```rust
/// Клиент хоста: тип (кэш или проба), токен по типу — GitLab: хранилище → `glab` → `GITLAB_TOKEN`,
/// GitHub: хранилище → `gh` → `GH_TOKEN`. Связка ключей может ждать ответа, CLI — запускаться,
/// поэтому поиск токена идёт в пуле блокирующих задач.
// ponytail: у `glab` и `gh` нет таймаута, зависший процесс займёт поток пула; добавить, если такое случится
async fn source_for(app: &AppHandle, host: &str) -> Result<AnySource, Error> {
    let host = normalize_host(host)?;
    let provider = resolve_provider(&host, &app.state::<AppState>().settings).await?;
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let tokens = &app.state::<AppState>().tokens;
        let env = |name: &str| std::env::var(name).ok();
        match provider {
            Provider::Gitlab => {
                let glab = Glab::find();
                let token = find_token(&host, tokens, &env, &|args| {
                    glab.as_ref().and_then(|glab| glab.value(args))
                })?;
                Ok(AnySource::Gitlab(gitlab::Client::new(&host, &token)?))
            }
            Provider::Github => {
                let gh = Gh::find();
                let token = find_github_token(&host, tokens, &env, &|args| {
                    gh.as_ref().and_then(|gh| gh.value(args))
                })?;
                Ok(AnySource::Github(github::Client::new(&host, &token)?))
            }
        }
    })
    .await
    .map_err(|e| Error::new(ErrorCode::Storage).with("detail", e))?
}
```

`hosts`:

```rust
/// Хосты с источником токена и типом; CLI запускаются — поэтому в пуле блокирующих задач.
#[tauri::command]
pub async fn hosts(app: AppHandle, webview: Webview) -> Cmd<Vec<HostInfo>> {
    ensure_form(&webview)?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let env = |name: &str| std::env::var(name).ok();
        let glab = Glab::find();
        let gh = Gh::find();
        let mut hosts = list_hosts(&state.tokens, &env, &|args| {
            glab.as_ref().and_then(|glab| glab.value(args))
        })?;
        for found in github_hosts(&env, &|args| gh.as_ref().and_then(|gh| gh.value(args))) {
            if !hosts.iter().any(|h| h.host == found.host) {
                hosts.push(found);
            }
        }
        for host in hosts.iter_mut().filter(|h| h.provider.is_none()) {
            host.provider = state.settings.host_kind(&host.host)?;
        }
        Ok(hosts)
    })
    .await
    .map_err(|e| Error::new(ErrorCode::Storage).with("detail", e))?
    .map_err(Into::<CmdError>::into)
}
```

(замыкание возвращает `Result<Vec<HostInfo>, Error>`; если вывод типа не справится — `Ok::<_, Error>(hosts)`).

Команды списков — через `Source`:

```rust
#[tauri::command]
pub async fn projects(app: AppHandle, webview: Webview, host: String, search: String, after: Option<String>) -> Cmd<Page<Project>> {
    ensure_form(&webview)?;
    let source = source_for(&app, &host).await?;
    Ok(source.list_projects(&search, after.as_deref()).await?)
}

#[tauri::command]
pub async fn branches(app: AppHandle, webview: Webview, host: String, project: String, search: String) -> Cmd<Vec<String>> {
    ensure_form(&webview)?;
    let source = source_for(&app, &host).await?;
    Ok(source.list_branches(&project, &search).await?)
}

#[tauri::command]
pub async fn pipelines(
    app: AppHandle,
    webview: Webview,
    host: String,
    project: String,
    r#ref: Option<String>,
    workflow: Option<String>,
    after: Option<String>,
) -> Cmd<Page<Pipeline>> {
    ensure_form(&webview)?;
    let source = source_for(&app, &host).await?;
    Ok(source
        .recent_pipelines(&project, r#ref.as_deref(), workflow.as_deref(), after.as_deref())
        .await?)
}

/// Workflow проекта GitHub; у GitLab — пусто, экран проекта тогда не показывает выбор.
#[tauri::command]
pub async fn workflows(app: AppHandle, webview: Webview, host: String, project: String) -> Cmd<Vec<Workflow>> {
    ensure_form(&webview)?;
    let source = source_for(&app, &host).await?;
    Ok(source.list_workflows(&project).await?)
}

/// Тип хоста для подсказки о токене: из кэша или пробой.
#[tauri::command]
pub async fn host_provider(app: AppHandle, webview: Webview, host: String) -> Cmd<Provider> {
    ensure_form(&webview)?;
    let host = normalize_host(&host)?;
    Ok(resolve_provider(&host, &app.state::<AppState>().settings).await?)
}
```

(`cargo fmt` разнесёт длинные сигнатуры.)

`build` — начало:

```rust
    ensure_form(&webview)?;
    let parsed = parse_form(&form)?;
    // ссылка сама говорит, чей хост: проба не нужна
    if form.mode == FormMode::Link
        && let Some(provider) = link_provider(&form.url)
    {
        state.settings.set_host_kind(&parsed.host, provider)?;
    }
    let source = source_for(&app, &parsed.host).await?;
    if source.provider() == Provider::Github
        && matches!(parsed.request, Request::Aggregate { workflow: None, .. })
    {
        return Err(BTreeMap::from([(Field::Workflow, Error::new(ErrorCode::WorkflowRequired))]).into());
    }
```

дальше `build_report(&source, …)` вместо `build_report(&gql, …)`.

`add_project`: `let gql = client_for(…)` → `let source = source_for(&app, &project.host).await?;`, `fetch_project(&gql, &project.path)` → `source.fetch_project(&project.path)`.

`FormMode` должен сравниваться: у него уже `PartialEq` (derive в `request.rs`).

- [ ] **Step 4: Регистрация команд**

`app/src-tauri/build.rs`, `COMMANDS` — после `"pipelines",`: `"workflows",` и `"host_provider",`.

`app/src-tauri/src/main.rs`, `generate_handler!` — после `commands::pipelines,`: `commands::workflows,` и `commands::host_provider,`. В тесте `списки_команд_совпадают`: `assert_eq!(in_build.len(), 18);` → `20`.

`app/src-tauri/capabilities/form.json` — после `"allow-pipelines",`: `"allow-workflows",` и `"allow-host-provider",`.

- [ ] **Step 5: Фронтенд — обёртки команд и мок**

`app/src/shared/api/api.ts`: импорты `Provider`, `Workflow` из `./schema/…`; заменить `pipelines` и добавить две обёртки:

```ts
export const pipelines = (host: string, project: string, ref: string | null, workflow: string | null, after: string | null) =>
  call<Page<Pipeline>>('pipelines', { host, project, ref, workflow, after })
export const workflows = (host: string, project: string) => call<Workflow[]>('workflows', { host, project })
export const hostProvider = (host: string) => call<Provider>('host_provider', { host })
```

`app/src/shared/api/index.ts`: реэкспортировать `workflows`, `hostProvider` и типы `Provider`, `Workflow` тем же способом, что соседние.

`app/src/entities/pipeline/model/queries.ts` — временно передать `null` вторым с конца аргументом (настоящий workflow — Task 10):

```ts
    queryFn: ({ pageParam }) => unwrap(pipelines(host, project, ref ?? null, null, pageParam)),
```

`app/src/app/dev/mock.ts`, в `commands`:

```ts
  workflows: ({ host }) =>
    host === 'github.com'
      ? [
          { file: 'ci.yml', name: 'CI' },
          { file: 'release.yml', name: 'Release' },
        ]
      : [],
  host_provider: ({ host }) => (host === 'github.com' ? 'github' : 'gitlab'),
```

- [ ] **Step 6: Сборка и проверки**

Run: `cd app && npm run build && cargo test -p pipeline-trace-core && cargo test -p pipeline-trace`
Expected: PASS, включая `списки_команд_совпадают`.

Run: `cd app && npm run typecheck && npm run lint:fsd`
Expected: без ошибок.

Run: `cd app && cargo clippy --workspace --all-targets -- -D warnings`
Expected: без предупреждений.

- [ ] **Step 7: Commit**

```bash
git add app/core app/src-tauri app/src/shared/api app/src/entities/pipeline app/src/app/dev/mock.ts
git commit -m "feat(tauri): источник по типу хоста, команды workflows и host_provider"
```

### Task 10: Форма — выбор workflow, ссылки из ядра, токен и тексты для GitHub

**Files:**
- Create: `app/src/features/select-workflow/index.ts`, `app/src/features/select-workflow/ui/WorkflowSelect.tsx`
- Create: `app/src/features/manage-token/model/useHostProvider.ts`
- Modify: `app/src/entities/project/model/useOpenProject.ts`, `app/src/entities/project/model/queries.ts`, `app/src/entities/project/index.ts`
- Modify: `app/src/entities/pipeline/model/queries.ts`, `app/src/entities/pipeline/index.ts`
- Modify: `app/src/app/form/router.tsx`, `app/src/pages/project/ui/ProjectPage.tsx`, `app/src/widgets/pipelines-list/ui/PipelinesList.tsx`
- Modify: `app/src/features/build-aggregate/model/useBuildAggregate.ts`, `app/src/widgets/aggregate-block/ui/AggregateBlock.tsx` (имя файла — как в каталоге `ui/`)
- Modify: `app/src/widgets/projects-sidebar/ui/ProjectsSidebar.tsx`, `app/src/features/manage-token/ui/TokenBlock.tsx`, `app/src/pages/settings/ui/SettingsPage.tsx`
- Modify: `app/src/shared/i18n/locales/ru.json`, `app/src/shared/i18n/locales/en.json`
- Modify: `docs/specs/2026-10-07-github-source-design.md` (§ 10, одно предложение)

**Interfaces:**
- Consumes: `api.workflows`, `api.hostProvider`, `api.pipelines(…, workflow, after)`, типы `Workflow`, `Provider`, `Pipeline.url`, `HostInfo.provider`, `Form.workflow`, поле ошибки `workflow` (Task 1–9).
- Produces: `ProjectRef.workflow?: string`, поиск маршрута `?workflow=`, `useWorkflows(host, project)`, `useHostProvider(host)`, `WorkflowSelect`.

Автотестов фронтенда нет; проверка — `typecheck`, `lint:fsd`, `check:locales` и мок в браузере (Step 9).

- [ ] **Step 1: `ProjectRef.workflow` и маршрут**

`app/src/entities/project/model/useOpenProject.ts`:

```ts
/** Проект на хосте, ветка (`undefined` — все ветки) и workflow GitHub (`undefined` — первый из списка): словарь экрана «Проект». */
export type ProjectRef = { host: string; project: string; branch: string | undefined; workflow?: string }

export type ProjectTarget = ProjectRef & { name?: string; replace?: boolean }

/** Открывает экран «Проект»; ветка — в `?ref=`, workflow — в `?workflow=`, имя — в `state` навигации. */
export function useOpenProject() {
  const navigate = useNavigate()
  return ({ host, project, branch, workflow, name, replace }: ProjectTarget) =>
    navigate({ to: '/project/$host/$', params: { host, _splat: project }, search: { ref: branch, workflow }, state: { name }, replace })
}
```

`app/src/app/form/router.tsx`, `projectRoute`:

```tsx
// Хост — параметр, путь проекта (`group/sub/project`) — splat, ветка — `?ref=`, workflow GitHub — `?workflow=`.
const projectRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/project/$host/$',
  validateSearch: (search: Record<string, unknown>): { ref?: string; workflow?: string } => ({
    ref: typeof search.ref === 'string' && search.ref !== '' ? search.ref : undefined,
    workflow: typeof search.workflow === 'string' && search.workflow !== '' ? search.workflow : undefined,
  }),
  component: () => {
    const { host, _splat } = projectRoute.useParams()
    const { ref, workflow } = projectRoute.useSearch()
    // key: другой проект — другая страница, прогресс, ошибки и подсказки веток прежнего не переносятся
    return <ProjectPage key={`${host}/${_splat}`} host={host} project={_splat ?? ''} branch={ref} workflow={workflow} />
  },
})
```

- [ ] **Step 2: Запросы workflow и пайплайнов**

`app/src/entities/project/model/queries.ts` — импорт `workflows` из `shared/api`, новый хук:

```ts
/** Workflow проекта: у GitLab пусто. Меняются редко — без повторной загрузки за сессию. */
export const useWorkflows = (host: string, project: string) =>
  useQuery({
    queryKey: ['workflows', host, project],
    queryFn: () => unwrap(workflows(host, project)),
    staleTime: Infinity,
  })
```

`app/src/entities/project/index.ts`: `export { useBranches, useProjects, useWorkflows } from './model/queries'`.

`app/src/entities/pipeline/model/queries.ts` — целиком:

```ts
import { useInfiniteQuery } from '@tanstack/react-query'
import { pipelines, unwrap } from '../../../shared/api'

export const usePipelines = (host: string, project: string, ref: string | undefined, workflow: string | undefined) =>
  useInfiniteQuery({
    queryKey: ['pipelines', host, project, ref ?? null, workflow ?? null],
    queryFn: ({ pageParam }) => unwrap(pipelines(host, project, ref ?? null, workflow ?? null, pageParam)),
    initialPageParam: null as string | null,
    staleTime: 0, // список последних пайплайнов устаревает сразу: при каждом открытии проекта берём свежий
    getNextPageParam: (last) => last.next,
  })
```

`app/src/entities/pipeline/index.ts`: `export { usePipelines } from './model/queries'` (ссылку на пайплайн теперь отдаёт ядро — `pipeline.url`).

- [ ] **Step 3: Список пайплайнов по ссылке из ядра**

`app/src/widgets/pipelines-list/ui/PipelinesList.tsx` — целиком:

```tsx
import { type Pipeline } from '../../../shared/api'
import { useErrorText } from '../../../shared/i18n'
import { PagedList } from '../../../shared/ui/paged-list'
import { linkForm, useBuild } from '../../../entities/history-entry'
import { PipelineRow, usePipelines } from '../../../entities/pipeline'
import type { ProjectRef } from '../../../entities/project'

// Своя сборка на строку: у каждой свой Channel, прогресс и ошибка остаются в её строке.
function BuildRow({ pipeline }: { pipeline: Pipeline }) {
  const errorText = useErrorText()
  const { start, busy, progressLabel, error } = useBuild()
  return (
    <PipelineRow
      pipeline={pipeline}
      busy={busy}
      progress={progressLabel}
      error={error && errorText(error)}
      onOpen={() => start(linkForm(pipeline.url))}
    />
  )
}

export function PipelinesList({ host, project, branch, workflow }: ProjectRef) {
  const query = usePipelines(host, project, branch, workflow)
  return <PagedList query={query}>{(pipeline) => <BuildRow key={pipeline.id} pipeline={pipeline} />}</PagedList>
}
```

- [ ] **Step 4: Выбор workflow и экран проекта**

`app/src/features/select-workflow/ui/WorkflowSelect.tsx`:

```tsx
import { useId } from 'react'
import type { Workflow } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { labelClasses } from '../../../shared/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../../../shared/ui/select'

type Props = { workflows: readonly Workflow[]; value: string; onChange: (file: string) => void }

/** Workflow GitHub: последние запуски и агрегат строятся по одному workflow. */
export function WorkflowSelect({ workflows, value, onChange }: Props) {
  const { t } = useTranslation()
  const id = useId()
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={id} className={labelClasses}>
        {t('form.workflow.label')}
      </label>
      <Select value={value} onValueChange={onChange}>
        <SelectTrigger id={id} translate="no">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {workflows.map((w) => (
            <SelectItem key={w.file} value={w.file}>
              {w.name} · {w.file}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  )
}
```

`app/src/features/select-workflow/index.ts`: `export { WorkflowSelect } from './ui/WorkflowSelect'`.

`app/src/pages/project/ui/ProjectPage.tsx` — целиком:

```tsx
import { useLocation } from '@tanstack/react-router'
import { useEffect } from 'react'
import { Card } from '../../../shared/ui/card'
import { BranchSelect } from '../../../features/select-branch'
import { WorkflowSelect } from '../../../features/select-workflow'
import { useOpenProject, useWorkflows, type ProjectRef } from '../../../entities/project'
import { useUpdateSettings } from '../../../entities/settings'
import { AggregateBlock } from '../../../widgets/aggregate-block'
import { PipelinesList } from '../../../widgets/pipelines-list'

export function ProjectPage(target: ProjectRef) {
  const openProject = useOpenProject()
  const update = useUpdateSettings()
  // имя приходит в state навигации; из записи истории его нет — тогда заголовок это путь
  const name = useLocation({ select: (location) => location.state.name })
  // страница смонтирована с key по проекту: эффект идёт один раз на проект
  useEffect(() => {
    void update({ lastProject: { host: target.host, path: target.project } })
  }, [update, target.host, target.project])
  // GitHub без выбора — первый workflow; у GitLab список пуст и workflow нет
  const { data: workflows = [] } = useWorkflows(target.host, target.project)
  const current: ProjectRef = { ...target, workflow: target.workflow ?? workflows[0]?.file }
  return (
    <Card>
      <h1 translate="no" className="truncate text-[17px] font-semibold">
        {name ?? target.project}
      </h1>
      <BranchSelect {...current} onCommit={(branch) => void openProject({ ...current, branch: branch || undefined, name, replace: true })} />
      {workflows.length > 0 && current.workflow && (
        <WorkflowSelect workflows={workflows} value={current.workflow} onChange={(workflow) => void openProject({ ...current, workflow, name, replace: true })} />
      )}
      <PipelinesList {...current} />
      <AggregateBlock {...current} />
    </Card>
  )
}
```

- [ ] **Step 5: Агрегат и повтор из истории с workflow**

`app/src/features/build-aggregate/model/useBuildAggregate.ts`:

```ts
export function useBuildAggregate({ host, project, branch, workflow }: ProjectRef) {
  const { start, ...build } = useBuild()
  // настройки читаем в момент клика: подписка на стор не нужна
  return { ...build, start: () => start(aggregateForm({ host, project, ref: branch ?? '', workflow: workflow ?? '', ...readAggSettings() })) }
}
```

Блок агрегата (`app/src/widgets/aggregate-block/ui/`) — ошибка поля `workflow` над кнопкой:

```tsx
  const workflowError = fieldError(error, 'workflow')
  // …
      <LastField error={fieldError(error, 'last')} />
      {workflowError && (
        <p role="alert" className="text-sm text-destructive">
          {errorText(workflowError)}
        </p>
      )}
      <BuildAggregateButton busy={busy} progressLabel={progressLabel} onClick={start} />
```

`app/src/widgets/projects-sidebar/ui/ProjectsSidebar.tsx`, в `openEntry`:

```ts
    void openProject({ host, project: form.project, branch: form.ref || undefined, workflow: form.workflow || undefined })
```

- [ ] **Step 6: Подсказка о токене по типу хоста**

`app/src/features/manage-token/model/useHostProvider.ts`:

```ts
import { useQuery } from '@tanstack/react-query'
import { hostProvider, unwrap, type Provider } from '../../../shared/api'

const GITHUB_COM = 'github.com'

/** Тип хоста для подсказки о токене; github.com известен сразу, остальные — из кэша ядра или пробы. Пока неизвестен — `undefined`. */
export function useHostProvider(host: string): Provider | undefined {
  const { data } = useQuery({
    queryKey: ['hostProvider', host],
    queryFn: () => unwrap(hostProvider(host)),
    enabled: host !== GITHUB_COM,
    staleTime: Infinity,
  })
  return host === GITHUB_COM ? 'github' : data
}
```

`app/src/features/manage-token/ui/TokenBlock.tsx` — вместо константы `createUrl` и текста scope:

```tsx
  const provider = useHostProvider(host)
  // страница создания токена с готовым именем и правами; `https` уходит в системный браузер через on_navigation
  const createUrl =
    provider === 'github'
      ? `https://${host}/settings/personal-access-tokens/new`
      : `https://${host}/-/user_settings/personal_access_tokens?name=pipeline-trace&scopes=read_api`
```

```tsx
        {t(provider === 'github' ? 'form.token.scopeGithub' : 'form.token.scope')}{' '}
```

Импорт: `import { useHostProvider } from '../model/useHostProvider'`.

- [ ] **Step 7: Тип хоста в настройках**

`app/src/pages/settings/ui/SettingsPage.tsx`, в строке хоста перед подписью источника:

```tsx
              {h.provider && (
                <span translate="no" className="text-[11px] text-muted-foreground">
                  {h.provider === 'github' ? 'GitHub' : 'GitLab'}
                </span>
              )}
```

- [ ] **Step 8: Тексты**

В обеих локалях заменить значения (ключ → ru / en):

| Ключ | ru | en |
|---|---|---|
| `errors.unauthorized` | `{{host}} ответил {{status}}: токен недействителен или нет доступа` | `{{host}} responded {{status}}: the token is invalid or access is denied` |
| `errors.httpStatus` | `{{host}} ответил {{status}}` | `{{host}} responded {{status}}` |
| `errors.graphql` | `Неожиданный ответ {{host}}: {{detail}}` | `Unexpected response from {{host}}: {{detail}}` |
| `errors.network` | `Не удалось связаться с {{host}}: {{detail}}` | `Could not reach {{host}}: {{detail}}` |
| `errors.mrHasNoPipeline` | `У MR или PR {{iid}} в проекте {{project}} нет пайплайна` | `MR or PR {{iid}} in project {{project}} has no pipeline` |
| `errors.keychainUnavailable` | `Системная связка ключей недоступна: задай GITLAB_TOKEN и GITLAB_HOST (для GitHub — GH_TOKEN) или установи libsecret` | `The system keychain is unavailable: set GITLAB_TOKEN and GITLAB_HOST (GH_TOKEN for GitHub) or install libsecret` |
| `errors.invalidLink` | `Нужна ссылка на пайплайн или MR GitLab (…/-/pipelines/<id>, …/-/merge_requests/<iid>) либо на run или PR GitHub (…/actions/runs/<id>, …/pull/<n>)` | `A GitLab pipeline or MR link (…/-/pipelines/<id>, …/-/merge_requests/<iid>) or a GitHub run or PR link (…/actions/runs/<id>, …/pull/<n>) is required` |
| `errors.invalidHost` | `Укажи хост, например gitlab.example.com` | `Enter a host, for example gitlab.example.com` |
| `errors.projectInputInvalid` | `Не похоже на ссылку на репозиторий GitLab или GitHub или путь к папке` | `Doesn't look like a GitLab or GitHub repository link or a folder path` |
| `form.link.label` | `Вставь ссылку на пайплайн, MR, run или PR` | `Paste a pipeline, MR, run or PR link` |
| `form.empty.text` | `Вставь ссылку на репозиторий GitLab или GitHub или выбери папку с клоном.` | `Paste a GitLab or GitHub repository link or pick a folder with a clone.` |
| `form.addProject.placeholder` | `https://gitlab.example.com/group/project, https://github.com/owner/repo или путь к папке` | `https://gitlab.example.com/group/project, https://github.com/owner/repo or a folder path` |
| `form.addProject.hint` | `Подойдёт ссылка на репозиторий, пайплайн, MR, run или PR.` | `A repository, pipeline, MR, run or PR link works.` |

Новые ключи:

- `form.token.scopeGithub` — ru `Fine-grained token с правами Actions: read и Contents: read.`, en `Fine-grained token with Actions: read and Contents: read permissions.`
- `form.workflow.label` (новый объект `workflow` рядом с `branch`) — ru `Workflow`, en `Workflow`.

- [ ] **Step 9: Проверки и мок в браузере**

Run: `cd app && npm run typecheck && npm run lint:fsd && npm run check:locales`
Expected: без ошибок.

Мок: в `app/src/app/dev/mock.ts` добавить в `saved` проект GitHub — `{ host: 'github.com', path: 'o/r', name: 'o / r', addedAt: iso(10) }`.

Run: `cd app && npm run dev`, открыть `http://localhost:5173/`.
Expected:
- проект `o / r` показывает выбор workflow «CI · ci.yml», смена на «Release» меняет `?workflow=release.yml` в адресе;
- проект GitLab выбора workflow не показывает;
- в настройках у `github.com` подпись «GitHub» и источник «gh».

- [ ] **Step 10: Спека — поле «Событие»**

`docs/specs/2026-10-07-github-source-design.md`, § 10: удалить предложение «Поле `source` подписано «Событие» (`push`, `pull_request`, `schedule`…).» и дописать: «Поля `source` в форме нет — фильтр по событию остаётся в ядре для записей истории.»

- [ ] **Step 11: Commit**

```bash
git add app/src docs/specs/2026-10-07-github-source-design.md
git commit -m "feat(web): выбор workflow, ссылки пайплайнов из ядра, токен и тексты для GitHub"
```

### Task 11: Отчёт — ссылки по провайдеру и предупреждение `needsMissing`

**Files:**
- Rename: `app/src/widgets/detail-panel/ui/GitlabLink.tsx` → `app/src/widgets/detail-panel/ui/SourceLink.tsx`
- Modify: `app/src/widgets/detail-panel/ui/DetailPanel.tsx`, `app/src/widgets/detail-panel/ui/TopPipelines.tsx`
- Modify: `app/src/widgets/report-header/ui/ReportHeader.tsx`
- Modify: `app/src/shared/i18n/locales/ru.json`, `app/src/shared/i18n/locales/en.json`

**Interfaces:**
- Consumes: `report.meta.provider`, `report.meta.needsMissing` (Task 8), `useReportView()` (`entities/report`).
- Produces: `SourceLink` (бывший `GitlabLink`), ключи `report.panel.openIn`, `report.panel.hostLink`, `report.needsMissing`.

- [ ] **Step 1: Ссылка в источник**

`git mv app/src/widgets/detail-panel/ui/GitlabLink.tsx app/src/widgets/detail-panel/ui/SourceLink.tsx`, содержимое:

```tsx
import type { ReactNode } from 'react'
import type { Provider } from '../../../shared/api'
import { cn } from '../../../shared/lib/cn'

/** Имя источника в подписи ссылки: бренд, не переводится. */
export const sourceName = (provider: Provider) => (provider === 'github' ? 'GitHub' : 'GitLab')

/** Ссылка в GitLab или GitHub в новой вкладке; без адреса — прочерк. */
export const SourceLink = ({ url, className, children }: { url: string | null; className?: string; children: ReactNode }) =>
  url ? (
    <a href={url} target="_blank" rel="noopener" className={cn('text-link', className)}>
      {children}
    </a>
  ) : (
    '—'
  )
```

`DetailPanel.tsx`: импорт `SourceLink, sourceName` из `./SourceLink`; `const { tree, state, dispatch, report } = useReportView()`; ссылка:

```tsx
          <SourceLink url={node.url}>{t('report.panel.openIn', { name: sourceName(report.meta.provider) })}</SourceLink>
```

`TopPipelines.tsx`: импорт `SourceLink, sourceName`; ссылка (переменная `report` в компоненте уже есть — её использует `pipelineOf(report, x.tree)`):

```tsx
            <SourceLink url={pipeline.url} className="text-[11px]">
              {t('report.panel.hostLink', { name: sourceName(report.meta.provider) })}
            </SourceLink>
```

Локали, `report.panel`: ключ `openGitlab` заменить на `openIn`, `gitlab` — на `hostLink`:

- ru: `"openIn": "Открыть в {{name}} ↗"`, `"hostLink": "{{name}} ↗"`
- en: `"openIn": "Open in {{name}} ↗"`, `"hostLink": "{{name}} ↗"`

Проверить, что старых ключей больше никто не читает: `grep -rn "openGitlab\|panel.gitlab\|GitlabLink" app/src` — пусто.

- [ ] **Step 2: Предупреждение без файла workflow**

`ReportHeader.tsx`, после `<BackToAggregate />`, перед `<div className="ml-auto …">` (`basis-full` переносит строку под чипы):

```tsx
      {meta.needsMissing && (
        <p role="note" className="basis-full text-[12.5px] text-muted-foreground">
          {t('report.needsMissing')}
        </p>
      )}
```

Локали, корень `report`:

- ru: `"needsMissing": "Зависимости джоб недоступны: нет доступа к файлу workflow. Стейдж один, критический путь — по времени."`
- en: `"needsMissing": "Job dependencies are unavailable: no access to the workflow file. One stage, the critical path follows timing only."`

- [ ] **Step 3: Проверки и мок**

Run: `cd app && npm run typecheck && npm run lint:fsd && npm run check:locales`
Expected: без ошибок.

В `app/src/shared/api/fixtures/single.json` временно поставить `"provider": "github", "needsMissing": true`, открыть отчёт в `npm run dev` (`http://localhost:5173/`, отчёт из «Недавних»): в шапке строка-предупреждение на всю ширину, в панели деталей «Открыть в GitHub ↗». Вернуть `"gitlab"` и `false`.

- [ ] **Step 4: Commit**

```bash
git add app/src
git commit -m "feat(web): отчёт — ссылки в GitLab или GitHub, предупреждение без файла workflow"
```

### Task 12: README, сверка спеки, проверка на живом GitHub

**Files:**
- Modify: `README.md`
- Modify: `docs/specs/2026-10-07-github-source-design.md` (только если проверка нашла расхождение)

**Interfaces:**
- Consumes: всё из Task 1–11.

- [ ] **Step 1: README**

В `README.md`:

- первый абзац: «Показывает, на что уходит время пайплайна GitLab или GitHub Actions: …»;
- в разделе «Приложение», пункт про ссылку: «ссылку на пайплайн или MR GitLab, на run или PR GitHub — вставляешь в поле в шапке и жмёшь «Построить»; у PR строится самый долгий run head-коммита»;
- пункт про токен — добавить: «для GitHub — fine-grained token с правами Actions: read и Contents: read; токены `gh` и `GH_TOKEN`/`GITHUB_TOKEN` приложение видит само»;
- пункт про экран проекта — добавить: «у проекта GitHub — выбор workflow: последние запуски и агрегат строятся по одному workflow»;
- новый абзац после списка: «GitHub: пайплайн — один workflow run. Стейджи и критический путь строятся по `needs` из файла workflow на коммите запуска; без доступа к файлу отчёт строится одним стейджем. Перезапуск run рисуется как ретрай перезапущенных джоб. GitHub Enterprise Server определяется сам по `/api/v3/meta`.»
- абзац про поиск токена: «Токен GitHub ищется в связке ключей, затем в `gh`, затем в `GH_TOKEN`/`GITHUB_TOKEN` (github.com) или `GH_ENTERPRISE_TOKEN` при `GH_HOST` (GHES).»

- [ ] **Step 2: Полная проверка**

Run: `cd app && npm run build && cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p pipeline-trace-core && cargo test -p pipeline-trace && npm run typecheck && npm run lint:fsd && npm run check:locales`
Expected: всё без ошибок.

- [ ] **Step 3: Живой GitHub**

Run: `cd app && npm run tauri dev`. Токен — из `gh` (`gh auth status` должен показывать вход) или ввести в блоке токена.

Проверить по шагам и записать результат в описание PR:

1. Ссылка `https://github.com/denoland/deno/actions/runs/37489604184` строит отчёт: несколько стейджей, matrix свёрнут в группы, у перезапущенной `test node_compat (1/3) debug macos-x86_64` — ретрай с ожиданием в несколько часов.
2. Ссылка на открытый PR любого публичного репозитория с Actions строит отчёт.
3. «Добавить проект» → `https://github.com/denoland/deno` → экран проекта показывает выбор workflow, последние запуски открываются в отчёт.
4. Агрегат по 5 запускам `ci.generated.yml` на `main` строится; подпись — `ci.generated.yml · main`.
5. Проект GitLab открывается и строится как раньше.

Если поведение API расходится со спекой (имена джоб, попытки), поправить код и тест, затем спеку — в том же коммите.

- [ ] **Step 4: Commit**

```bash
git add README.md docs/specs/2026-10-07-github-source-design.md
git commit -m "docs: GitHub Actions в README"
```

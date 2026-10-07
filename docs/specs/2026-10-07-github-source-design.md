# GitHub Actions как второй источник — спека

Дата: 2026-10-07. Добавляет GitHub Actions (github.com и GitHub Enterprise Server) как второй источник пайплайнов рядом с GitLab. Базовые спеки: `docs/specs/2026-10-03-tauri-rewrite-design.md`, `docs/specs/2026-10-06-first-run-ux-design.md`. Здесь только отличия. Базовая спека `2026-09-24-pipeline-trace-design.md` откладывала GitHub до появления потребности — она появилась.

## 1. Цель и рамки

Пользователь вставляет ссылку на workflow run или PR GitHub, добавляет репозиторий GitHub в «Мои проекты» и получает тот же отчёт: водопад, критический путь, агрегат по N запусков.

Делается:

- Один workflow run — один пайплайн. Агрегат — по N запускам одного workflow.
- Стейджи и критический путь строятся по `needs` из YAML workflow.
- Хосты: github.com и GHES. Тип хоста определяется автоматически.
- Ссылка на PR строит самый долгий run head-коммита.
- Токены: связка ключей → `gh` → переменные окружения.

Не делается: агрегат по нескольким workflow, связь запусков через `workflow_run`, GitHub App и OAuth device flow, логи джоб.

Принцип: GitHub-данные превращаются в существующий `RawPipeline`. Всё ниже — `model`, `critical_path`, `aggregate`, `insights`, `render`, фронтенд отчёта — не меняется, кроме подписей, завязанных на провайдера.

## 2. Run → `RawPipeline`

### 2.1. Пайплайн

REST: `GET /repos/{owner}/{repo}/actions/runs/{id}`.

| Поле `RawPipelineInfo` | Источник |
|---|---|
| `id` | `run.id` |
| `iid` | `run.run_number` |
| `created_at` | `run.created_at` |
| `finished_at` | `run.updated_at`, если `status == completed`, иначе `None` |
| `status` | `status` + `conclusion` → таблица 2.3 |
| `ref` | `run.head_branch` |
| `path` | путь из `run.html_url` (без схемы и хоста) |
| `stages` | имена уровней графа `needs` по порядку (2.4) |

`project` — `owner/repo`. `downstream` — всегда пустая карта.

### 2.2. Джобы

REST: `GET /repos/{owner}/{repo}/actions/runs/{id}/jobs?filter=all&per_page=100`, пагинация по заголовку `Link`, не больше 50 страниц (иначе `tooManyJobs`, как у GitLab).

| Поле `RawJob` | Источник |
|---|---|
| `id` | `job.id` |
| `name` | `job.name`; matrix-суффикс переписан (2.5) |
| `bridge` | `false` |
| `status` | `job.status` + `job.conclusion` → таблица 2.3 |
| `started_at` / `finished_at` | `job.started_at` / `job.completed_at`; у `conclusion == skipped` — оба `None` (GitHub отдаёт им `started_at` позже `completed_at`) |
| `queued_duration` | `started_at − created_at` в секундах; `None`, если не стартовала |
| `retried` | 2.6 |
| `allow_failure` | `continue-on-error: true` у джобы в YAML; иначе `false` |
| `web_path` | путь из `job.html_url` |
| `stage` | имя уровня джобы (2.4) |
| `needs` | имена джоб API, соответствующих `needs` из YAML (2.5) |

Порядок: от новых к старым, как у GitLab (модель разворачивает список).

`build_tree` получает `base_url = https://<host>` (для github.com — `https://github.com`), ссылки собираются без изменений модели.

### 2.3. Статусы

| GitHub | `RawPipeline` / `Status` |
|---|---|
| `completed` + `success` | `SUCCESS` |
| `completed` + `failure`, `timed_out`, `startup_failure` | `FAILED` |
| `completed` + `cancelled` | `CANCELED` |
| `completed` + `skipped`, `neutral` | `SKIPPED` |
| `completed` + `action_required` | `MANUAL` |
| `waiting`, `pending`, `requested` | `MANUAL` |
| `queued` | `PENDING` |
| `in_progress` | `RUNNING` |

### 2.4. Стейджи

Файл workflow загружается `GET /repos/{owner}/{repo}/contents/{run.path}?ref={run.head_sha}` (`Accept: application/vnd.github.raw`). Хвост `@ref` в `run.path` отбрасывается. Один файл на пару (sha, путь) за сборку — кэш в памяти на время `build_report`.

Уровень джобы YAML — длина самого длинного пути по `needs` до неё; джоба без `needs` — уровень 0. Цикл или ссылка на несуществующую джобу — такие рёбра отбрасываются, уровень считается без них; паники нет.

Имя стейджа уровня: имя джобы, если на уровне одна джоба YAML; иначе «этап N» / «stage N» по `BuildEnv.locale`, N с 1.

Файл не загрузился (404, нет прав Contents, не YAML) — все джобы на уровне 0, без `needs`, отчёт строится. В `Meta` — признак `needsMissing: true`, отчёт показывает строку «Зависимости джоб недоступны: нет доступа к файлу workflow».

### 2.5. Сопоставление имён API и YAML

Для каждой джобы API по порядку:

1. **Reusable workflow**: имя `caller / inner` — берётся часть до первого ` / `, дальше по правилам ниже; джоба наследует `needs` и уровень вызывающей.
2. **Шаблон**: `name:` с `${{ … }}` превращается в регулярное выражение — литералы экранируются, каждое выражение становится `.*?`, шаблон привязан к началу и концу. Так GitHub называет matrix-джобы с выражениями в имени (`test ${{ matrix.crate }} debug macos` → `test integration (1/2) debug macos`).
3. **Matrix без выражений в имени**: GitHub дописывает ` (a, b)` к `name:` или ключу — `X (a, b)` сопоставляется с `X`.
4. **Точное совпадение** с `name:` джобы YAML или её ключом, если `name:` нет.
5. Не сопоставилась — уровень 0, без `needs`.

Порядок проверки: точное совпадение, matrix-суффикс, шаблон. Первое совпадение выигрывает.

Джоба API, сопоставленная с джобой YAML со `strategy.matrix`, переименовывается для группировки шардов: `X (a, b)` → `X: [a, b]`; по шаблону — `<ключ YAML>: [<имя из API>]`. Существующая группировка (`model::shard_group_name`) сворачивает их без изменений модели.

`needs` джобы API — имена всех джоб API, сопоставленных с джобами YAML из её `needs` (для matrix — все шарды).

### 2.6. Ретраи

Перезапуск в GitHub создаёт новую попытку run (`run_attempt`). `filter=all` отдаёт джобы всех попыток. Проверено 2026-10-07 на `denoland/deno` run 37489604184 (2 попытки, 134 джобы в каждой):

- джобы, не перезапущенные во второй попытке, повторяются в ней с новым `id`, но с теми же `started_at` и `completed_at`;
- перезапущенная джоба во второй попытке имеет другие времена;
- `run.created_at` остаётся временем первой попытки, `run.run_started_at` — время последней.

Правило: среди джоб с одинаковым итоговым именем последняя попытка — основная. Более ранняя попытка с теми же `started_at` и `completed_at`, что у более поздней, — дубль и отбрасывается. Остальные ранние попытки — `retried: true`. Ожидание между попытками может длиться часы — так и рисуется.

## 3. Хосты и тип хоста

Тип хоста — `enum Provider { Gitlab, Github }`, в `projects.json`, истории и форме не хранится.

Определение:

1. `github.com` — всегда `Github`, без запросов.
2. Иначе — карта `hostKinds` (хост → `gitlab` | `github`) в `settings.json`.
3. Хоста нет в карте — проба `GET https://<host>/api/v3/meta` без токена, таймаут соединения 15 с: 200 с полем `installed_version` — `Github`, любой другой ответ — `Gitlab`. Результат пишется в карту.
4. Сетевая ошибка пробы — ошибка `network`, в карту ничего не пишется.

Ссылка, разобранная как GitHub (раздел 5), пишет `github` в карту без пробы; ссылка вида `/-/pipelines/` — `gitlab`.

Существующие пользователи: карта пуста, первый запрос к хосту делает пробу и записывает `gitlab`. Миграции нет.

API: github.com — `https://api.github.com`, GHES — `https://<host>/api/v3`. Редиректы выключены. Заголовки: `Authorization: Bearer`, `Accept: application/vnd.github+json`, `X-GitHub-Api-Version: 2022-11-28`, `User-Agent: pipeline-trace`. Не больше 4 параллельных запросов, таймауты как у GitLab-клиента.

## 4. Токены

Связка ключей не меняется: service `dev.pipeline-trace.desktop`, account — хост.

`find_token` для `Github`:

1. Связка ключей.
2. `gh auth token --hostname <host>`. `Gh` ищется так же, как `Glab` (PATH, типовые каталоги, `~/.local/bin`).
3. github.com: `GH_TOKEN`, затем `GITHUB_TOKEN`. GHES: `GH_ENTERPRISE_TOKEN`, затем `GITHUB_ENTERPRISE_TOKEN`, если `GH_HOST` совпадает с хостом.
4. Иначе — `noToken`.

Для `Gitlab` — как сейчас.

`TokenSource` получает вариант `Gh`. `list_hosts` добавляет хосты из `gh auth status --json hosts` (если версия `gh` не умеет `--json` — только github.com при успешном `gh auth token`) с источником `gh` и github.com из `GH_TOKEN`/`GITHUB_TOKEN` с источником `env`.

Подсказка о токене для GitHub: ссылка `https://<host>/settings/personal-access-tokens/new`, права — Actions: read, Contents: read (Metadata: read добавляется сам). Для classic PAT — scope `repo`.

## 5. Ссылки и ввод проекта

`parse_link` распознаёт, кроме GitLab:

| Ссылка | Запрос |
|---|---|
| `https://<host>/<owner>/<repo>/actions/runs/<id>` с хвостом `/job/<n>`, `/attempts/<n>` или без | `Pipeline { project, pipeline_id: id }` |
| `https://<host>/<owner>/<repo>/pull/<n>` с любым хвостом | `Mr { project, mr_iid: n }` |

`project` для GitHub — ровно два сегмента `owner/repo`.

`parse_project_input` уже понимает `https://<host>/<owner>/<repo>[.git]`, scp-форму и `ssh://`. Добавляется отрезание хвостов страниц GitHub (`/actions/…`, `/pull/…`, `/tree/…`, `/blob/…`) рядом с текущим отрезанием `/-/…`. Хост `ssh.github.com` (ssh через 443) приводится к `github.com`.

## 6. Ядро: трейт `Source`

`report.rs` и `browse.rs` переходят с `&impl Gql` на `&impl Source`:

```rust
pub trait Source: Sync {
    fn host(&self) -> &str;
    fn provider(&self) -> Provider;
    fn fetch_pipeline(&self, project: &str, id: &str) -> impl Future<Output = Result<RawPipeline, Error>> + Send;
    fn list_pipelines(&self, project: &str, filter: &PipelineFilter) -> impl Future<Output = Result<Listed, Error>> + Send;
    /// head-пайплайн MR (GitLab) или самый долгий run head sha PR (GitHub)
    fn head_pipeline(&self, project: &str, number: &str) -> impl Future<Output = Result<String, Error>> + Send;
    fn fetch_project(&self, path: &str) -> impl Future<Output = Result<Project, Error>> + Send;
    fn list_projects(&self, search: &str, after: Option<&str>) -> impl Future<Output = Result<Page<Project>, Error>> + Send;
    fn list_branches(&self, project: &str, search: &str) -> impl Future<Output = Result<Vec<String>, Error>> + Send;
    fn recent_pipelines(&self, project: &str, r#ref: Option<&str>, workflow: Option<&str>, after: Option<&str>) -> impl Future<Output = Result<Page<Pipeline>, Error>> + Send;
    /// у GitLab — пустой список
    fn list_workflows(&self, project: &str) -> impl Future<Output = Result<Vec<Workflow>, Error>> + Send;
}
```

- `gitlab::Client` реализует `Source`, делегируя в существующие функции. `Gql` остаётся внутренним швом тестов GitLab.
- Новые модули: `github.rs` (реализация `Source`, преобразование в `RawPipeline`), `github/client.rs` (REST, `Link`-пагинация, rate limit), `github/workflow.rs` (разбор YAML, уровни, сопоставление имён — чистые функции).
- `enum AnySource { Gitlab(gitlab::Client), Github(github::Client) }` реализует `Source` диспетчеризацией; `commands::client_for` возвращает его по `Provider` хоста.
- `report::resolve` для `Pipeline` больше не собирает gid сам: идентификатор передаётся в `fetch_pipeline` как есть, GitLab-реализация оборачивает его в `gid://gitlab/Ci::Pipeline/…`.

GitHub-реализация операций:

| Операция | REST |
|---|---|
| `fetch_project` | `GET /repos/{o}/{r}` → `full_name`, `pushed_at`, `default_branch` |
| `list_projects` | `GET /user/repos?sort=pushed&per_page=100`; `search` фильтрует страницу по подстроке `full_name` без учёта регистра; курсор — номер страницы |
| `list_branches` | `GET /repos/{o}/{r}/branches?per_page=100`, первая страница, фильтр по подстроке, не больше 20; ветка по умолчанию — первой |
| `recent_pipelines` | `GET /repos/{o}/{r}/actions/workflows/{workflow}/runs?branch=&per_page=20`; без `workflow` — `/actions/runs` |
| `list_workflows` | `GET /repos/{o}/{r}/actions/workflows`, только `state == active` |
| `list_pipelines` | `GET …/workflows/{workflow}/runs?branch=&event=&status=&per_page=100`, не больше 10 страниц |
| `head_pipeline` | `GET /repos/{o}/{r}/pulls/{n}` → `head.sha`; `GET /actions/runs?head_sha=` → run с максимальной длительностью (идущий — до `now`); нет запусков — `mrHasNoPipeline` |

`browse::Pipeline` для run: `id` — `run.id`, `iid` — `run_number`, `status` — строчными по таблице 2.3, `source` — `event`, `commit` — `head_sha` и первая строка `head_commit.message`, `author` — `actor.login`, `duration` — `updated_at − run_started_at` у завершённого.

Новый тип `Workflow { file: String, name: String }`, `file` — базовое имя `path` (`ci.yml`).

## 7. Агрегат

`Form` и `Request::Aggregate` получают `workflow: Option<String>` (`#[serde(default)]` — старые записи истории читаются). GitLab поле игнорирует.

Для GitHub пустой `workflow` — ошибка поля `Field::Workflow` с кодом `workflowRequired`. Проверка — в `commands::build` после определения провайдера: `parse_form` о провайдере не знает.

Отображение фильтров:

| Форма | Параметр GitHub |
|---|---|
| `ref` | `branch` |
| `source` | `event` |
| `SUCCESS` | `status=success` |
| `FAILED` | `status=failure` |
| `CANCELED` | `status=cancelled` |
| `RUNNING` | `status=in_progress` |
| `MANUAL` | `status=waiting` |

Несколько статусов — запрос без `status`, фильтр на клиенте по таблице 2.3, как у GitLab. Счётчики статусов — по таблице 2.3.

Суффикс имени файла агрегата: `<workflow без расширения>-<ref или source или all>`.

## 8. Ошибки

Новые коды `ErrorCode`:

- `rateLimited` — `host`, `reset` (ISO 8601): 403 или 429 с `x-ratelimit-remaining: 0`, либо с `retry-after` (тогда `reset = now + retry-after`).
- `workflowRequired` — агрегат GitHub без workflow.

Остальное — существующими кодами: 401 и 403 без признаков лимита — `unauthorized`; 404 на репо — `projectNotFound`; 404 на run — `pipelineNotFound`; иной не-2xx — `httpStatus`; неожиданная форма JSON — `graphql` с `detail` (код не переименовывается, перевод становится общим «неожиданный ответ сервера»).

## 9. Отчёт

`Meta` получает `provider: "gitlab" | "github"` и `needsMissing: bool`.

- `GitlabLink` и тексты «в GitLab» в панели деталей, `TopPipelines`, `StabilitySection` — по `meta.provider`.
- `needsMissing` — строка-предупреждение под шапкой отчёта.

## 10. Интерфейс формы

- Поле ссылки: плейсхолдер «Ссылка на пайплайн, MR, run или PR».
- Диалог добавления проекта: принимает ссылки и ssh-URL GitHub; список проектов хоста работает и для GitHub.
- Экран проекта GitHub: селект workflow рядом с веткой (из `list_workflows`). Выбор живёт в состоянии экрана, как ветка, и не сохраняется; по умолчанию — первый из `list_workflows`. Последние запуски и агрегат фильтруются по нему. Поле `source` подписано «Событие» (`push`, `pull_request`, `schedule`…).
- Блок токена: ссылка и права по провайдеру (раздел 4).
- Настройки → хосты: значок провайдера у хоста; источник `gh` отображается как `glab`.
- i18n: новые ключи в `ru.json` и `en.json`.

## 11. Зависимости

Ядро получает парсер YAML: `serde_yaml_ng` (форк `serde_yaml`, тот же API serde). Других новых зависимостей нет.

## 12. Тесты

- `github/workflow.rs`: `needs` строкой и списком, `name:`, matrix, reusable workflow, несопоставленная джоба, цикл в `needs`, ссылка на несуществующую джобу, `continue-on-error`.
- Преобразование run → `RawPipeline`: ответы API в `json!` по форме, снятой с `denoland/deno` (раздел 2.6), включая перезапуск, копии джоб и пропущенную джобу.
- HTTP (wiremock): `Link`-пагинация, 401, 404, 403/429 с лимитом, базовый адрес GHES, проба `/api/v3/meta` для GHES и для GitLab.
- `request.rs`: ссылки run, job, attempt, PR; ввод проекта с хвостами страниц GitHub и `ssh.github.com`.
- `tokens.rs`: порядок поиска для github.com и GHES с подставными `env` и `gh`; `list_hosts` с `gh`.
- Сквозной: `build_report` на мок-`Source` с GitHub-фикстурой — критический путь проходит по `needs`.
- Существующие тесты GitLab проходят без правок фикстур.

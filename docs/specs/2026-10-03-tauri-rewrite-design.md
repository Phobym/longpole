# pipeline-trace на Tauri 2 + React — спека

Дата: 2026-10-03. Итог карты [«Переписать pipeline-trace на Rust + Tauri + React/TypeScript»](https://github.com/Phobym/pipeline-trace/issues/1). Каждое положение ссылается на тикет-решение; поправки из комментариев к закрытым тикетам уже учтены. План реализации — `docs/plans/2026-10-03-tauri-rewrite.md`, чек-лист приёмки — `docs/specs/2026-10-03-tauri-rewrite-acceptance.md`.

## 1. Цель и рамки

Переписать десктопное приложение с Electron на Tauri 2: Rust-ядро, React/TypeScript-интерфейс.

- **Мотив**: размер и память (Electron → Tauri), поддерживаемость (типы, компоненты), производительность агрегата.
- **Паритет поведения**: те же экраны, поведение, клавиши, тексты. Внешний вид может меняться ([«UI-фреймворк для формы и отчёта»](https://github.com/Phobym/pipeline-trace/issues/13)). Осознанные отступления перечислены в § 12.
- **Языки интерфейса**: русский и английский. Русский — эталон паритета (тексты дословно из старого кода), английский — перевод ([«Локализация интерфейса»](https://github.com/Phobym/pipeline-trace/issues/18)).
- **Удаляется**: CLI `pipeline-trace`, каталоги `src/`, `bin/`, `desktop/`, workflow `desktop.yml` — одним коммитом после приёмки.
- **Не делается**: миграция токенов и истории из Electron; подпись, нотаризация, автообновление; E2E-тесты UI; новые экраны и функции.
- **Выпуск**: `v0.2.0`, `productName: pipeline-trace`, identifier `dev.pipeline-trace.desktop`, ставится поверх.
- **Паритет доказывается** переносом всех `node:test` в `cargo test` и ручным чек-листом приёмки.

## 2. Раскладка `app/`

[«Границы Rust-ядра и раскладка проекта `app/`»](https://github.com/Phobym/pipeline-trace/issues/6), FSD — [«Архитектура React-отчёта»](https://github.com/Phobym/pipeline-trace/issues/9).

```
app/
  Cargo.toml              # [workspace] members = ["core", "src-tauri"]
  rust-toolchain.toml
  core/                   # crate pipeline-trace-core — без зависимостей от Tauri
  src-tauri/              # тонкий слой: команды, окна, меню; tauri.conf.json, capabilities/, icons/
  src/                    # фронтенд, Feature-Sliced Design, одно дерево
    app/form/  app/report/          # две точки входа
    pages/ widgets/ features/ entities/
    shared/
      ui/                 # shadcn/ui
      lib/format.ts
      i18n/locales/{ru,en}.json
      api/                # api.ts поверх invoke
      api/schema/         # ts-rs типы — генерируются, коммитятся
      api/fixtures/       # эталонные Report JSON из тестов core — коммитятся
  package.json            # единственный источник версии
  vite.config.ts          # форма → dist/
  vite.report.config.ts   # отчёт → dist-report/report.html (single-file)
```

`cargo test -p pipeline-trace-core` не требует webview. Границы FSD проверяет Steiger в CI.

## 3. Rust-ядро (`app/core`)

### Модули
`gitlab`, `model`, `aggregate`, `critical_path`, `insights`, `browse`, `report`, `render`, `request`, `history`, `tokens`/`hosts`, `settings`.

### Сеть
[«Rust-стек для GitLab GraphQL-клиента»](https://github.com/Phobym/pipeline-trace/issues/3), расхождения с Node — #6.

- `reqwest` 0.13 (`default-features = false`, `rustls`, `json`), `serde`/`serde_json`, `tokio::sync::Semaphore(4)`, `futures` (`try_join_all`, `BoxFuture` для рекурсии downstream), `thiserror` 2.
- Запросы — строки + serde, без кодогенерации (`graphql_client`/`cynic` не берём).
- Трейт `Gql` — шов для тестов (повторяет `fakeGql`); клиент принимает базовый URL — тесты HTTP-слоя на `wiremock` 0.6.
- TLS — `rustls-platform-verifier`: системное хранилище CA на всех ОС (отступление: корпоративный CA работает без настройки). `NODE_EXTRA_CA_CERTS` не поддерживается.
- Прокси из `HTTP(S)_PROXY` — принимается (дефолт reqwest).
- `connect_timeout(15s)` + `read_timeout(60s)`, общего таймаута нет.

### `build_report`
```rust
pub async fn build_report(gql: &impl Gql, host: &str, request: &Request, now: OffsetDateTime,
    on_progress: impl Fn(Progress) + Send + Sync) -> Result<Built, Error> // Built { report, file_name }
```
- `Request` — `Pipeline | Mr | Aggregate`; прогресс `{ loaded, total }` — callback, Tauri оборачивает в `ipc::Channel`.
- `default_file_name` внутри: `pipeline-trace-<project>-<suffix>.html`.
- `render(report, template: &str) -> String` — подставляет JSON в собранный шаблон отчёта; экранируется только `<` (→ `<`). Шаблон передаёт Tauri-crate через `include_str!`.

### Разбор формы (`core::request`)
Команда `build` принимает сырую форму. Регулярки хоста и проекта, лимит `last` 1–500, набор статусов (`SUCCESS`, `MANUAL`, `FAILED`, `CANCELED`, `RUNNING`, `ANY`) и разбор ссылки `…/-/pipelines/<id>` / `…/-/merge_requests/<iid>` переезжают из `desktop/app/request.mjs` и `src/cli-args.mjs`. Ошибки — по полям (`fields`), см. § 5.

### Токены и хосты
Notes карты, уточнение — [«Сборка итоговой спеки и плана»](https://github.com/Phobym/pipeline-trace/issues/17).

- Порядок поиска токена: хранилище → `glab config get token --host` → `GITLAB_TOKEN` + `GITLAB_HOST`, как сейчас.
- Хранилище — `keyring-core` + `apple-native-keyring-store`, `windows-native-keyring-store`, `dbus-secret-service-keyring-store`; `set_default_store` при старте. Если стор не создался (Linux без Secret Service → `PlatformFailure`), хранение токенов выключено с прежним сообщением, env и `glab` работают. `NoStorageAccess` при записи — то же сообщение.
- Запись в связке ключей: service `dev.pipeline-trace.desktop`, account — хост.
- Список хостов — отдельный JSON без секретов в `app_data_dir`.
- `glab` ищется в `PATH` процесса, затем `/opt/homebrew/bin`, `/usr/local/bin`, `/home/linuxbrew/.linuxbrew/bin`, `~/.local/bin` ([«Окна, меню и жизненный цикл приложения»](https://github.com/Phobym/pipeline-trace/issues/10)). `fix-path-env` не берём.

### История и настройки
- `history.json` в `app_data_dir`: только поля запроса (`mode`, `host`, `url`, `project`, `ref`, `source`, `last`, `statuses`), лимит 20, дедупликация по `[host, request]` — как `desktop/app/history.mjs`. Подпись записи хранится структурой (проект + метка, как `meta.label`) и рендерится в форме на текущем языке.
- `settings.json` в `app_data_dir`: `locale` (§ 10).

## 4. Схема отчёта

[«Новая схема JSON отчёта»](https://github.com/Phobym/pipeline-trace/issues/5), поправка — `meta.locale` (#18).

Всё, что старый шаблон досчитывал сам (`criticalPath`, `stabilityKey`, `critShare`, `barStart`/`barEnd`/`aggEnd`, инверсия `deps`, «держит стейдж»), считает Rust. В отчёте остаются только `visibleId` (свёртка групп — состояние UI) и форматирование.

```ts
type Report =
  | { mode: 'single';    meta: Meta; tree: Tree<SingleNode> }
  | { mode: 'aggregate'; meta: Meta; agg: Tree<AggNode>; trees: Tree<SingleNode>[] }

type Meta = { host: string; project: string; label: string | null /* null — «все пайплайны» */; locale: 'ru' | 'en'
              statusCounts: Record<string, number> | null; generatedAt: string /* ISO */ }

type Ms = number
type Id = string

type Tree<N> = {
  root: Id
  nodes: Record<Id, N>
  critical: Record<Id, { ids: Id[]; gaps: { from: Id; to: Id; ms: Ms }[] }>
  hotspots: Hotspot[]
  totalRetryLoss: Ms
  saving: Ms
}

type Kind = 'pipeline' | 'stage' | 'group' | 'job' | 'bridge'

type BaseNode = {
  id: Id; kind: Kind; name: string
  parent: Id | null; children: Id[]
  deps: Id[]; after: Id | null
  bar: { start: Ms; end: Ms } | null
  dependents?: Id[]                                // linkable: job/bridge/group
  critShare?: number                               // linkable
  excess?: { id: Id; peersEnd: Ms; excess: Ms }    // стейдж
  holds?: { stage: Id; excess: Ms }                // узел, держащий стейдж
  retryLoss?: Ms                                   // лист, > 0
  stability?: { retried: number; present: number } // лист
  saving?: Ms                                      // сумма его hotspots
}

type SingleNode = BaseNode & {
  start: Ms | null; end: Ms | null; queued: Ms | null
  status: string | null; allowFailure: boolean; url: string | null
  attempts: { start: Ms | null; end: Ms | null; status: string; url: string }[]
  project?: string; ref?: string                   // pipeline
}

type P = { p50: Ms | null; p90: Ms | null }
type AggNode = BaseNode & {
  position?: number                                // стейдж
  stats: { present: number; total: number; start: P; end: P; duration: P; queued: P
           retries: number; retried: number; retryLoss: Ms; critical: number
           samples: { tree: number; start: Ms; end: Ms; retries: number }[] }
}

type Hotspot = { id: Id; name: string; saving: Ms } & (
  | { kind: 'retry'; retries: number }
  | { kind: 'retry'; retried: number; present: number }
  | { kind: 'excess'; stage: string; excess: Ms })
```

- **Критические пути** по scope в `Tree.critical`. Single: пайплайн, каждый стейдж, downstream-пайплайны и их стейджи; bridge scope не бывает. Aggregate: один scope, в `ids` узлы с `stats.critical ≥ 0.5` (порог применяет Rust), `gaps: []`. В обоих режимах Rust добавляет в `ids` группу, если на пути хотя бы один её шард (паритет с `onCritical` в `template.html`).
- **`critShare`** у linkable-узлов: single — 0 или 1; aggregate — сумма по шардам, не больше 1. Подсветка водопада — только из `critical[scope].ids`.
- **`dependents`** — инверсия `deps` в Rust; у группы — объединение по шардам без своих узлов.
- Стабильность одиночных деревьев в агрегате Rust сопоставляет по имени-пути.
- **Единицы**: целые мс от создания корневого пайплайна, `null` — не запускалось. `samples[].tree` — индекс в `trees`. Поля `schemaVersion` нет: шаблон и данные собираются вместе.
- Паритетный тест на причуду: в агрегате у стейджа и группы собственный `stats.critical` всегда 0; группа подсвечена, только если в `ids` есть её шард, — при шардах < 0,5 по отдельности и сумме ≥ 0,5 строка группы не подсвечена, хотя в панели `critShare` показывает путь.
- Типы генерирует `ts-rs` 12 с `TS_RS_LARGE_INT=number` в `app/src/shared/api/schema/`.

## 5. Оболочка Tauri (`app/src-tauri`)

### Команды и IPC
[«Модель безопасности и IPC-команды в Tauri»](https://github.com/Phobym/pipeline-trace/issues/7), поправки — #8, #18.

| Команда | Вход | Выход |
|---|---|---|
| `hosts` | — | `string[]` |
| `set_token` | `host, token` | `()` — только запись |
| `remove_token` | `host` | `()` |
| `history` | — | `HistoryEntry[]` |
| `remove_history` | `at` | `HistoryEntry[]` |
| `clear_history` | — | `()` |
| `projects` | `{host, search, after}` | `Page<Project>` |
| `branches` | `{host, project, search}` | `string[]` |
| `pipelines` | `{host, project, ref, after}` | `Page<Pipeline>` |
| `build` | `form, onProgress: Channel<Progress>` | `()` — открывает окно отчёта |
| `get_locale` | — | `'ru' \| 'en'` |
| `set_locale` | `locale` | `()` — пересобирает меню |

- Токен не возвращает ни одна команда: форма видит только список хостов.
- Прогресс — `Channel`, не событие (события не подчиняются capabilities); ошибка `send` при закрытой форме игнорируется.
- **Ошибки**: `Result<T, CmdError>`,
  `CmdError = { kind: 'message', code, params } | { kind: 'fields', errors: Record<поле, { code, params }> }`.
  `code` — enum, экспорт ts-rs; перевод на фронтенде (§ 10). Чужие тексты — коды `graphql` / `network` с исходным текстом в `params.detail`; `network` → «Не удалось связаться с GitLab {host}: {detail}». `fields` использует только `build`.
- `api.ts` пишется руками (12 функций поверх `invoke`), типы — из ts-rs; отказ `invoke` превращается в `{ ok: true, value } | { ok: false, error: CmdError }`. `tauri-specta` не берём.

### Безопасность
- `AppManifest::commands` в `build.rs` — без него свои команды доступны любому локальному окну, включая `report://`.
- Одна capability `form.json` с `"windows": ["form"]`: наши `allow-*`, `dialog:allow-ask` и `dialog:allow-message` (подтверждения, #8). Окна `report-<n>` не входят ни в одну capability. `core:*` для `Channel` не нужен (S5, проверено на macOS): прогресс — короткие сообщения, они идут через `eval`, а `plugin:__TAURI_CHANNEL__|fetch` для больших выведен из ACL самим Tauri.
- `ensure_form(&webview)?` в каждой команде — сверка label (аналог `fromForm`).
- **CSP формы**: `app.security.csp` — `default-src 'self'; style-src 'self' 'unsafe-inline'; connect-src ipc: http://ipc.localhost`.
- **CSP отчёта**: только `<meta>` из сборки — `default-src 'none'; script-src 'sha256-…'; style-src 'sha256-…'; img-src data:; connect-src 'none'`.
- **Навигация**: `on_navigation` разрешает только собственный origin окна (`tauri://localhost` / `http://tauri.localhost`, `report://localhost` / `http://report.localhost`); `on_new_window` всегда `Deny`. `https` Rust открывает через `opener` из обоих обработчиков: на macOS WebKit для `target=_blank` зовёт только `on_navigation`. JS-перехват плагина (`open_js_links_on_click`) выключен: он открывает ссылку через IPC, а окнам отчёта IPC запрещён.
- **Отчёт отдаётся из памяти**: `HashMap<label, { html, file_name }>`, своя схема `report://` отдаёт по `webview_label`; «Сохранить» пишет этот буфер; запись удаляется при закрытии окна. Временных файлов нет.
- **Окно отчёта без IPC**: получает тот же самодостаточный HTML, что сохраняется.

### Окна, меню, жизненный цикл
[«Окна, меню и жизненный цикл приложения»](https://github.com/Phobym/pipeline-trace/issues/10), поправка — #18.

- Окна: `form` 1280×860, min 900×640, заголовок `pipeline-trace`; `report-<n>` 1440×900, заголовок фиксирован при создании (словарь Rust, язык на момент построения).
- Меню (строки — словарь в Rust на ru/en, пересборка при смене языка):

  | Меню | macOS | Windows | Linux |
  |---|---|---|---|
  | **pipeline-trace** | меню приложения из готовых пунктов (О программе, Скрыть, Завершить ⌘Q), как `Menu::default` | — | — |
  | **Файл** | Новый отчёт ⌘N, Сохранить отчёт… ⌘S, Закрыть окно (готовый) | Новый отчёт, Сохранить отчёт…, «Выход» (готовый) вместо «Закрыть окно» | то же, «Выход» — свой пункт, `app.exit(0)` |
  | **Правка** | все готовые, включая Отменить/Повторить | Вырезать/Копировать/Вставить/Выделить всё; Отменить/Повторить в меню нет | как Windows |
  | **Вид** | масштаб (сброс/+/−) — свои пункты, `set_zoom`; полный экран — готовый | масштаб — свои; полный экран — свой, `set_fullscreen` | как Windows |
  | **Окно** | готовые | Свернуть/Развернуть — готовые | почти пустое — допустимое отличие |

  «Перезагрузить» не переносим. DevTools — только в dev-сборке.
- «Новый отчёт»: фокус на `form` или создать заново.
- «Сохранить отчёт…»: запоминается label последнего окна с `WindowEvent::Focused(true)`; по ⌘S сохраняется оно, если это окно отчёта и оно существует, иначе ничего. Неблокирующий `save_file` с родителем, имя по умолчанию, фильтр HTML; ошибка — `message` с `MessageDialogKind::Error`.
- Жизненный цикл: macOS — `RunEvent::ExitRequested` при `code == None` → `prevent_exit()`, `RunEvent::Reopen` → форма, если её нет; Windows/Linux — выход после закрытия последнего окна.
- Данные — `app_data_dir()`.

## 6. Фронтенд: общее

- **FSD**, одно дерево, входы `app/form/` (роутер, QueryClient, провайдеры, i18n) и `app/report/` (чтение JSON из `<script type="application/json">`, монтирование). Отчёт импортирует только `pages/report`; в бандле отчёта нет `@tauri-apps/api`. Steiger в CI (#9).
- **UI**: shadcn/ui на Radix + Tailwind v4, CSS собирается статически. База — тема shadcn; поверх — семантические токены домена: статусы (`ok`/`fail`/`warn`/`run`/`idle`/`queued`), `crit*`, `retry*`/`over*`, `dep-up`/`dep-down`, пилюли статусов. Тёмная тема по `prefers-color-scheme`, без переключателя (#13).
- **Форматирование** — `shared/lib/format.ts`, поведение каждой функции сохраняется точно: `durationShort` (форма, `12s`), `duration` (отчёт, `12.3s`/`850ms`/`0s`), `relativeTime`, `dateTime`, `number`; локаль — текущая (#8, #18).

## 7. Форма

[«Архитектура React-формы приложения»](https://github.com/Phobym/pipeline-trace/issues/8), [«Роутер и библиотека состояния для React-формы»](https://github.com/Phobym/pipeline-trace/issues/12), FSD-раскладка — поправки в #8 и #12.

| Слой | Slice |
|---|---|
| `pages` | `projects`, `project` |
| `widgets` | `history-sidebar` (в т.ч. открытие записи истории), `projects-panel`, `pipelines-list`, `aggregate-block`, переключатель языка |
| `features` | `build-by-link`, `build-aggregate` (`useBuild` + `Channel`), `manage-token` (+ `ask`), `search-projects`, `select-branch`, `history-actions` (удалить/очистить) |
| `entities` | `host`, `project`, `pipeline`, `history-entry` — query-ключи, хуки TanStack Query, строки |
| `shared` | `api`, `ui`, `lib/format`, `i18n`, `api/schema` |

- **Роутер**: TanStack Router, маршруты в коде, hash history. `/` — «Проекты»; `/project/$host/$` — «Проект» (хост параметром, `fullPath` в splat, ветка — `?ref=` со схемой). Имя проекта — через `state` навигации; из записи истории его нет → заголовок `fullPath`. «← Проекты» и Esc → `navigate({ to: '/' })`; Esc — один `keydown` в корневом маршруте. Экраны монтируются условно; при возврате поиск и список могут перезагрузиться.
- **Состояние**: маленькие Zustand-сторы по slice — `host` → `entities/host/model`; `aggSettings: { last, statuses }` → `features/build-aggregate/model` (живёт между проектами, меняется только записью истории); `linkUrl` + `focusLinkToken` → `features/build-by-link/model`. Персистентности нет: `last` при запуске = 50. Запись истории заполняет сторы из `widgets/history-sidebar` через публичные API slice и вызывает `navigate`.
- **Данные**: TanStack Query. Гонки ответов закрываются ключами запросов; «Показать ещё» — infinite query; debounce поиска 300 мс; ошибки веток не показываются (как сейчас). Сборка — мутация на вызов `build` со своим `Channel` на каждую кнопку/строку, «Загружаю N из M…» остаётся на своём месте.
- **Хост**: выпадающий список всегда показывает текущий хост; для хоста без токена — временный пункт.
- **Поиск проектов**: текст поиска — в сторе `features/search-projects`; после «← Проекты» текст и список (из кеша Query) на месте.
- **Ветки**: Combobox принимает ветку не из подсказок; список пайплайнов перезагружается при выборе подсказки, по Enter или при уходе фокуса, если значение изменилось (паритет с `change`).
- **Esc**: корневой обработчик пропускает событие, если оно уже обработано (`defaultPrevented`) или открыт оверлей — первый Esc закрывает Popover/Select, второй возвращает к «Проектам».
- **Кнопка агрегата**: «Агрегат по N пайплайнам» со склонением («по 1 пайплайну»).
- **Контролы**: Select (хост), Combobox — Command + Popover (ветки вместо `<datalist>`), Input (N).
- **Подтверждения**: `await ask(…)` / `message(…)` из `@tauri-apps/plugin-dialog` («Удалить токен для …?», «Очистить историю запросов?», ошибка удаления токена). Нативные `confirm`/`alert` в Tauri 2 на macOS не работают, а plugin-dialog подменяет их асинхронными.
- Бандл формы — без лимита и без код-сплиттинга.

## 8. Отчёт

[«Архитектура React-отчёта»](https://github.com/Phobym/pipeline-trace/issues/9), сборка — [«Сборка React-отчёта в один автономный HTML»](https://github.com/Phobym/pipeline-trace/issues/4), проверка стека — [«Прототип отчёта: проверка стека»](https://github.com/Phobym/pipeline-trace/issues/14).

| Слой | Slice | Что внутри |
|---|---|---|
| `pages` | `report` | композиция, режим single/aggregate, `ReportViewProvider` |
| `widgets` | `report-header` | заголовок, чипы, «← к агрегату», переключатель языка |
| | `hotspots` | «Куда направить силы» |
| | `waterfall` | Axis, StageCard, Row, LinksOverlay |
| | `detail-panel` | шапка, вердикт, KPI, полоса длительностей, стабильность, связи, «Самые долгие» / «Открыть в GitLab» |
| `features` | `timeline-navigation`, `node-selection`, `group-toggle`, `tree-switch`, `keys-help` | |
| `entities` | `report` | редьюсер состояния просмотра, селекторы `visibleId`, `cards()`, связи выбранного |
| | `node` | бар, попытки, пилюли статуса и стабильности, dep-тег |

- **Сборка**: второй Vite-конфиг с `vite-plugin-singlefile`; свой плагин после инлайна считает sha256 инлайн-`<script>`/`<style>` и пишет `<meta>` CSP (§ 5). Данные — `<script type="application/json">`. Rust встраивает шаблон через `include_str!`; `build.rs` проверяет, что артефакт есть, и ставит `rerun-if-changed`. React (Preact снят). Без `unsafe-inline`.
- **Состояние**: один `useReducer` в `entities/report/model`, контекст `{ state, dispatch }`. Состояние `{ tree: 'agg' | индекс, collapsed, selected, scope, showAll, view }`; действия `open`, `select`, `toggle`, `reveal`, `setView`, `close`, `toggleShowAll`. Правила паритета — в чистом редьюсере: `open` сбрасывает всё и сворачивает группы; выбор стейджа/пайплайна в single меняет `scope`; выбор другого узла сбрасывает `showAll`. Производные — `useMemo`.
- **Взаимодействие**:
  - `useViewDrag`: `mousedown` на дорожке, `mousemove`/`mouseup` на `document`, порог 4 px, `dragEndedAt` гасит `click` 50 мс;
  - `useWheelZoom`: нативный `wheel` с `passive: false`; ⌘/ctrl (pinch) — зум от курсора, shift/горизонталь — сдвиг; минимум 1 с, максимум — полный view;
  - `useReportKeys`: `[`, `]`, `0`, Esc на `document`, с модификаторами — игнор; стрелки/Enter/←→ — `onKeyDown` у `Row`;
  - строки с ключом `id` не пересоздаются; фокус при клике ставится вручную; `setView` — не чаще раза в кадр.
- **Линии связей**: `LinksOverlay` — `useLayoutEffect`, карта ref'ов `id → element`, ширина дорожки; пересчёт на `view`/`collapsed`/`selected` и по `ResizeObserver` (исправляет баг с ресайзом).
- **Производительность**: без виртуализации; `Row` в `React.memo`, `view` через rAF. Прототип: 400 строк, p95 кадра 16,7 мс без потерь при CPU×4. Запасной вариант — бары через CSS-переменные `--view-a`/`--view-w`.
- **shadcn**: Badge, Button, Popover (`?`), Card (стейджи). Панель — `<aside>`, сужает водопад. Подсказки имён — нативный `title`.
- **Размер**: прототип — 110 КБ gzip, жёсткого лимита нет.
- **Язык**: оба словаря в файле, переключатель в шапке. В окне приложения (`location.protocol === 'report:'` или хост `report.localhost`) по умолчанию `meta.locale`, иначе (файл в браузере) — язык браузера. `meta.label === null` выводится как «все пайплайны» на текущем языке.
- Окно отчёта в приложении показывает тот же HTML, что сохраняется.

## 9. Тесты

#6, поправка — #18.

| Было | Стало |
|---|---|
| `model.test` | `core::model` |
| `aggregate.test` + `order.test` | `core::aggregate` |
| `critical-path.test` | `core::critical_path` |
| `insights.test` | `core::insights` |
| `gitlab.test` | `core::gitlab` (фейковый `Gql` + wiremock) |
| `browse.test` | `core::browse` |
| `report.test` | `core::report` (фейковый `Gql`) |
| `render.test` | `core::render`: экранирование `</script>`, round-trip JSON; проверка подстановки критического пути удаляется |
| `cli-args.test` | ссылки pipeline/MR → `core::request`; флаги, `--help`, неизвестный флаг — удаляются |
| `desktop/request.test` | `core::request` |
| `desktop/history.test` | `core::history` (tempdir) |
| `desktop/tokens.test` | переписывается под мок-стор `keyring-core` + список хостов |
| новый | причуда `stats.critical = 0` у стейджа и группы в агрегате |

- Фикстуры — Rust-билдеры (`job(name, stage, JobOpts { .., ..Default::default() })`, `raw_pipeline`), не JSON-файлы.
- Тесты сверяют `code` + `params` ошибок, не строки.
- Тесты агрегата выгружают эталонные `Report` JSON (single и aggregate) в `app/src/shared/api/fixtures/` — на них разрабатывается отчёт.
- Фронтенд — без автотестов, проверка чек-листом.

## 10. Локализация

[«Локализация интерфейса: русский и английский»](https://github.com/Phobym/pipeline-trace/issues/18).

- Выбор языка: системная локаль (`sys-locale`: `ru*` → русский, иначе английский) + переключатель RU/EN в форме, хранится в `settings.json`. Смена применяется сразу: форма перерисовывается, меню пересобирается; открытые отчёты не меняются.
- i18next + react-i18next; словари `shared/i18n/locales/{ru,en}.json`, типизированные ключи, склонения через `Intl.PluralRules`.
- Ошибки — коды + параметры из Rust, перевод на фронтенде; словарь Rust — только меню, заголовок окна отчёта и ошибка сохранения отчёта.
- `Intl.*` с текущей локалью; длительности одинаковы для обоих языков.
- Русские тексты — дословно из старого кода; английские пишет агент, пользователь вычитывает при приёмке.
- CI: совпадение наборов ключей `ru.json`/`en.json`; покрытие кодов ошибок — `tsc` через ts-rs enum.

## 11. CI и релиз

[«Сборка и релиз Tauri-приложения в CI»](https://github.com/Phobym/pipeline-trace/issues/11), поправка — #15.

- **`ci.yml`** (PR и push в `main`, ubuntu): `cargo fmt --check`, `cargo clippy`, `cargo test -p pipeline-trace-core`, `git diff` сгенерированных `shared/api/schema/` и `shared/api/fixtures/`, `tsc --noEmit`, Steiger, проверка ключей словарей, сборка обоих Vite-конфигов.
- **`release.yml`** — тег `v*` и `workflow_dispatch` (артефакты без релиза, для приёмки); `tauri-action`, `releaseDraft: true`:

  | Runner | Цель | Артефакты |
  |---|---|---|
  | `macos-14` | `aarch64-apple-darwin` | dmg arm64 |
  | `macos-14` | `x86_64-apple-darwin` (кросс) | dmg x64 |
  | `windows-latest` | x64 | nsis |
  | `ubuntu-22.04` | x64 | AppImage, deb |

  Падает, если тег ≠ версии. Кеш — `Swatinem/rust-cache`.
- Подпись: macOS ad-hoc (`signingIdentity: "-"`), Windows — без подписи; README «Установка без подписи» сохраняется.
- WebView2 — `downloadBootstrapper`. Linux — AppImage (70+ МБ) и deb.
- Иконка — новый PNG 1024×1024, размеры — `tauri icon`. Метаданные: категория `DeveloperTool`, описание «Водопад длительности GitLab-пайплайнов», maintainer deb как сейчас. Версия — из `app/package.json` (`"version": "../package.json"`).
- `aws-lc-rs`: нужен ли NASM на Windows — покажет первый прогон; обходы — `prebuilt-nasm` или провайдер `ring`.

## 12. Отступления от паритета

| Было | Стало |
|---|---|
| Внешний вид `form.css`/`template.html` | shadcn/ui + Tailwind, токены домена |
| Нативные `<select>` хоста и `<datalist>` веток | Select и Combobox |
| Только русский интерфейс | русский и английский, переключатели в форме и отчёте |
| Хост из истории не синхронизировал список хостов | список показывает текущий хост |
| Линии связей не перерисовывались при ресайзе | перерисовываются |
| Только встроенные корни Mozilla, без прокси, без таймаутов | системный CA, `HTTP(S)_PROXY`, таймауты 15/60 с |
| `glab` из GUI-запуска не находился | `PATH` + фиксированный список путей |
| `confirm()`/`alert()` | диалоги plugin-dialog |
| «Под фильтры не попал ни один пайплайн: … (--ref, --source, --status)» | «… проверь ветку и статусы» |
| «Агрегат по 1 пайплайнам» | склонение: «по 1 пайплайну» |
| Прогресс одной сборки получали все ожидающие кнопки | у каждой сборки свой `Channel` |
| Меню Electron с ролями | без «Перезагрузить»; без Отменить/Повторить на Win/Linux; тонкое «Окно» на Linux |
| У окна отчёта нет IPC вовсе | мост внедрён, но запрещён ACL и CSP |
| Отчёт без CSP | CSP на хешах |
| zip-артефакты, релиз сразу | без zip, релиз черновиком |
| Токены и история Electron | не переносятся |
| CLI | удалён |

## 13. Ручные проверки на реализации

- ~~Минимальный `core:*` для `Channel`~~ — не нужен, форма получает прогресс с одними `allow-*` (S5).
- `on_navigation` не мешает загрузке первого URL; Ctrl+Z/Ctrl+Y в полях на Windows/Linux; Linux без Secret Service → `PlatformFailure`; фокус окна при ⌘S на Windows/Linux — раздел «Проверки платформы» чек-листа, ставятся на шаг S5.

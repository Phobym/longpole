# pipeline-trace на Tauri 2 + React — план реализации

Дата: 2026-10-03. Спека — `docs/specs/2026-10-03-tauri-rewrite-design.md` (далее «спека, § N»), чек-лист — `docs/specs/2026-10-03-tauri-rewrite-acceptance.md`. Нарезка — [«Порядок реализации и нарезка плана»](https://github.com/Phobym/pipeline-trace/issues/16) с поправкой из [«Локализация интерфейса»](https://github.com/Phobym/pipeline-trace/issues/18).

## Правила для каждого шага

- Один шаг — один PR, одна сессия агента. Новое приложение пишется в `app/`; старые `src/`, `bin/`, `desktop/` не трогаются до S10.
- **Общий критерий готовности** — зелёный `ci.yml` (после S0 он есть всегда): `cargo fmt --check`, `cargo clippy`, `cargo test -p pipeline-trace-core`, `git diff` сгенерированных `shared/api/schema/` и `shared/api/fixtures/`, `tsc --noEmit`, Steiger, проверка ключей словарей, сборка обоих Vite-конфигов.
- **Ядро (S1–S4)** — TDD (навык `tdd`): тест из `node:test` переносится первым и падает, затем реализация. Таблица переноса — спека, § 9. Выброшенный тест — только с записью в описании PR, почему.
- **Фронтенд** — без автотестов; готовность — пройденный свой раздел чек-листа на macOS в dev-сборке.
- Эталон поведения — старый код (`src/*.mjs`, `src/template.html`, `desktop/app/*`); русские тексты переносятся дословно в `ru.json`, английские пишет агент.

## Зависимости

```
S0 ─┬─ S1 ── S2 ─┬──────── S4 ── S5 ─┬─ S7 ─┐
    ├─ S3 ───────┘                   │      ├─ S9 ── S10
    └─ S6 ───────────┬───────────────┘      │
                 S2 ─┴─ S8a ── S8b ─────────┘
```

| № | Шаг | Зависит от |
|---|---|---|
| S0 | Каркас `app/` | — |
| S1 | Модель | S0 |
| S2 | Агрегат, критический путь, insights, дерево схемы | S1 |
| S3 | GitLab-клиент и browse | S0 |
| S4 | Запрос, история, токены, настройки, `build_report`, `render`, коды ошибок | S2, S3 |
| S5 | Оболочка Tauri | S4 |
| S6 | Общий слой фронтенда | S0 |
| S7 | Форма | S5, S6 |
| S8a | Отчёт: водопад и навигация | S2, S6 |
| S8b | Отчёт: панель, шапка, single-file | S8a |
| S9 | Релиз | S5, S8b |
| S10 | Приёмка и удаление старого кода | S7, S9 |

После S0 три дорожки идут параллельно: ядро (S1 → S2 → S4, параллельно S3), фронтенд (S6, затем S8a на эталонных JSON из S2), оболочка и форма (S5 → S7).

---

## S0. Каркас `app/`

**Цель**: пустое, но собираемое приложение и CI, на которые ложатся остальные шаги.

**Файлы**:
- `app/Cargo.toml` (workspace `core`, `src-tauri`), `app/rust-toolchain.toml`;
- `app/core/` — пустой crate `pipeline-trace-core` с `ts-rs` 12 (`TS_RS_LARGE_INT=number`), экспорт в `app/src/shared/api/schema/`;
- `app/src-tauri/` — `tauri.conf.json` (`productName`, identifier, `"version": "../package.json"`), окно `form`, пустой `build.rs` с `AppManifest`;
- `app/package.json`, `tsconfig`, `vite.config.ts`, `vite.report.config.ts` (заглушка отчёта со `vite-plugin-singlefile` и плагином CSP-хешей — по образцу ветки `prototype/report-stack`);
- скелет FSD (`app/src/{app/form,app/report,pages,widgets,features,entities,shared}`), Steiger;
- `.github/workflows/ci.yml` — все проверки общего критерия, включая скрипт сверки ключей `ru.json`/`en.json` и `git diff` сгенерированного.

**Готово, когда**: `ci.yml` зелёный на пустом приложении; `tauri dev` открывает окно формы с заглушкой; сборка отчёта даёт `dist-report/report.html` с `<meta>` CSP без `unsafe-inline`.

## S1. Модель

**Цель**: `core::model` — `buildTree` из `src/model.mjs`.

**Файлы**: `app/core/src/model.rs`, `app/core/tests/fixtures.rs` (билдеры `job(name, stage, JobOpts { .., ..Default::default() })`, `raw_pipeline(...)` по `test/fixtures.mjs`).

**Тесты**: `model.test`.

**Готово, когда**: все тесты `model.test` перенесены и зелёные.

## S2. Агрегат, критический путь, insights, дерево схемы

**Цель**: всё, что считает отчёт, — в Rust, в форме новой схемы (спека, § 4).

**Файлы**: `app/core/src/{aggregate,critical_path,insights}.rs`, типы схемы `Report`/`Tree`/`SingleNode`/`AggNode`/`Hotspot`/`Meta` с `#[derive(TS)]`.

**Тесты**: `aggregate.test` + `order.test`, `critical-path.test`, `insights.test`; новый — причуда `stats.critical = 0` у стейджа и группы в агрегате. Тест-генератор выгружает эталонные `Report` JSON (single и aggregate, `meta.locale = 'ru'`) в `app/src/shared/api/fixtures/`.

**Готово, когда**: тесты зелёные; `critical` по всем scope (группа в `ids`, если на пути её шард), `critShare`, `dependents`, `bar`, insights внутри узлов, `meta.label: null` для «все пайплайны»; фикстуры и TS-типы сгенерированы и закоммичены.

## S3. GitLab-клиент и browse

**Цель**: `core::gitlab` и `core::browse` (спека, § 3 «Сеть»).

**Файлы**: `app/core/src/{gitlab,browse}.rs`, трейт `Gql`, клиент на reqwest с `rustls-platform-verifier`, `Semaphore(4)`, таймаутами 15/60 с.

**Тесты**: `gitlab.test` (фейковый `Gql` для логики, wiremock для HTTP-слоя), `browse.test`.

**Готово, когда**: тесты зелёные; ошибки — коды с параметрами (`graphql`/`network` с `params.detail` для чужих текстов), не строки.

## S4. Запрос, история, токены, настройки, `build_report`

**Цель**: замкнуть ядро: от сырой формы до готового HTML отчёта.

**Файлы**:
- `core::request` — разбор формы и ссылки, ошибки по полям (`fields` с кодами);
- `core::history` — `history.json`, лимит 20, дедупликация, подпись записи структурой (проект + метка);
- `core::tokens`/`hosts` — `keyring-core` + платформенные сторы (service `dev.pipeline-trace.desktop`, account — хост), `set_default_store`, `PlatformFailure` → хранение выключено; список хостов; поиск токена «хранилище → `glab` → env»; поиск `glab` по `PATH` и фиксированному списку;
- `core::settings` — `settings.json` с `locale`;
- `core::report::build_report` (callback прогресса, инъекция `now`, `file_name`), `core::render` (шаблон аргументом);
- enum кода ошибки `ErrorCode` + `CmdError` с экспортом ts-rs.

**Тесты**: `report.test`, `render.test` (без проверки подстановки критического пути), ссылки из `cli-args.test`, `desktop/request.test`, `desktop/history.test` (tempdir), `desktop/tokens.test` (мок-стор `keyring-core`).

**Готово, когда**: тесты зелёные; `CmdError`/`ErrorCode` экспортированы в `shared/api/schema/`.

## S5. Оболочка Tauri

**Цель**: `src-tauri` по спеке, § 5.

**Файлы**: `app/src-tauri/src/*.rs`, `build.rs` (`AppManifest::commands`, проверка артефакта отчёта, `rerun-if-changed`), `capabilities/form.json`.

**Содержание**: 12 команд с `ensure_form`; `build` с `Channel<Progress>`; схема `report://` из памяти по `webview_label`; `on_navigation`/`on_new_window` + `opener`; CSP формы; окна; меню по таблице на ru/en (словарь Rust; на macOS — меню приложения, на Win/Linux в «Файл» «Выход» вместо «Закрыть окно»), пересборка по `set_locale`; ⌘N, ⌘S (последнее сфокусированное окно отчёта, неблокирующий `save_file`, ошибка через `message`); жизненный цикл macOS/Windows/Linux.

**Готово, когда**: на macOS пройдены пункты раздела «Проверки платформы» чек-листа; найден минимальный `core:*` для `Channel` — временная страница формы получает прогресс `build` и открывает окно отчёта.

## S6. Общий слой фронтенда

**Цель**: всё, на что опираются форма и отчёт.

**Файлы**: `shared/ui` (shadcn/ui на Radix + Tailwind v4, тема + семантические токены домена, тёмная тема по системе); `shared/lib/format.ts` (`durationShort`, `duration`, `relativeTime`, `dateTime`, `number` — поведение 1:1 со старым кодом, локаль параметром); `shared/i18n` (i18next + react-i18next, `locales/{ru,en}.json`, типизированные ключи, перевод `CmdError` по `code`); `shared/api/api.ts` (12 функций поверх `invoke`, `{ ok, value | error }`).

**Готово, когда**: общий критерий; словарь покрывает все `ErrorCode` (иначе `tsc` падает); ключи `ru`/`en` совпадают.

## S7. Форма

**Цель**: форма по спеке, § 7.

**Файлы**: `app/form/` (роутер TanStack Router с hash history, QueryClient, i18n с `get_locale`), `pages/{projects,project}`, `widgets/{history-sidebar,projects-panel,pipelines-list,aggregate-block}` + переключатель языка, `features/{build-by-link,build-aggregate,manage-token,search-projects,select-branch,history-actions}`, `entities/{host,project,pipeline,history-entry}`, Zustand-сторы по slice, подтверждения через `ask`/`message`.

**Готово, когда**: раздел «Форма» чек-листа пройден на macOS в dev-сборке (на русском полностью, на английском — смоук).

## S8a. Отчёт: водопад и навигация

**Цель**: основная часть отчёта на эталонных JSON из S2, без Tauri.

**Файлы**: `app/report/` (чтение JSON, i18n), `pages/report`, `entities/report` (редьюсер, селекторы `visibleId`, `cards()`, связи выбранного), `entities/node`, `widgets/waterfall` (Axis, StageCard, Row, LinksOverlay), `features/{timeline-navigation,node-selection,group-toggle}`.

**Содержание**: `useViewDrag`, `useWheelZoom`, `useReportKeys`; `Row` в `React.memo`, `view` через rAF; `LinksOverlay` с `useLayoutEffect` и `ResizeObserver`.

**Готово, когда**: подразделы «Водопад», «Связи и линии», «Клавиши и жесты» чек-листа пройдены на фикстурах в dev-сервере.

## S8b. Отчёт: панель, шапка, single-file

**Цель**: отчёт целиком, в одном файле, встроенный в Rust.

**Файлы**: `widgets/{report-header,hotspots,detail-panel}`, `features/{tree-switch,keys-help}`, переключатель языка в шапке (по умолчанию `meta.locale`, в браузере — язык браузера), финальный `vite.report.config.ts`.

**Готово, когда**: раздел «Отчёт» чек-листа пройден на macOS — в окне приложения и в сохранённом файле, открытом по `file://` в браузере; CSP без ошибок в консоли.

## S9. Релиз

**Цель**: артефакты и выпуск по спеке, § 11.

**Файлы**: `.github/workflows/release.yml` (тег `v*` + `workflow_dispatch`, матрица, `releaseDraft: true`, проверка тег = версия), иконка (PNG 1024×1024 → `tauri icon`), метаданные бандлов, README (установка без подписи, новые имена файлов, `NODE_EXTRA_CA_CERTS` не поддерживается, языки).

**Готово, когда**: `workflow_dispatch` собирает dmg arm64, dmg x64, nsis, AppImage, deb; вопрос NASM для `aws-lc-rs` на Windows закрыт.

## S10. Приёмка и удаление старого кода (`ready-for-human`)

**Цель**: доказать паритет и выпустить `v0.2.0`.

**Порядок**:
1. Артефакты `release.yml` через `workflow_dispatch`.
2. Пользователь проходит чек-лист: разделы «Форма» и «Отчёт» полностью на macOS (русский) + смоук на английском; смоук на Windows и Linux; «Проверки платформы» на трёх ОС; сверка с 0.1.0 на трёх реальных запросах; замеры.
3. Непройденный пункт → issue `bug`, блокирует удаление старого кода, если пользователь не перенёс его в «Отступления».
4. Один коммит: удалить `src/`, `bin/`, `desktop/`, `test/`, `.github/workflows/desktop.yml`, корневой `package.json` CLI; обновить README.
5. Тег `v0.2.0` → черновик релиза → запуск артефактов → публикация; цифры замеров — в описание релиза.

**Готово, когда**: чек-лист пройден, `bug`-issues закрыты или перенесены, `v0.2.0` опубликован.

# Что в longpole-core и во фронте завязано на десктоп

Тикет: [#73](https://github.com/Phobym/longpole/issues/73), карта
[#71](https://github.com/Phobym/longpole/issues/71). Источник — код репо на коммите
`d794343` (`origin/main`, 2026-10-12): `app/core`, `app/src-tauri`, `app/src`.
Ссылки `файл:строка` даны относительно `app/`.

Контекст: к Tauri-оболочке добавляется серверная — многопользовательский stateless
веб-сервис (сессия в зашифрованной cookie, без БД, у каждого пользователя свой
OAuth-токен, хосты и OAuth-приложение задаёт администратор) поверх того же
`longpole-core` и того же фронта с подменой `invoke` ↔ `fetch`.

## Вывод

- **Сборка отчёта переносится как есть.** `gitlab`, `github`, `browse`, `model`,
  `aggregate`, `critical_path`, `insights`, `report::build_report`, `render`,
  `request::parse_form`/`parse_remote` не трогают ни диск, ни окружение, ни Tauri:
  им нужен только `Source` (клиент с токеном) и `BuildEnv { now, locale }`.
  Клиенты создаются на каждый вызов (`src-tauri/src/commands.rs:54`), общего
  состояния не держат.
- **Не переносится слой токенов и хранилищ ядра**: `tokens.rs` (связка ключей +
  `hosts.json` + `glab`/`gh`/env), `oauth::login` (loopback `127.0.0.1` и Device Flow
  без секрета), `History`/`Projects`/`Settings`/`HostList` (JSON-файлы в каталоге
  данных). Серверу нужен свой `source_for`: токен из cookie-сессии → `Client::new`.
- **Главный риск утечки — не Tauri, а «удобства десктопа»**: `find_token` и
  `list_hosts` берут токен из `GITLAB_TOKEN`/`GH_TOKEN` и `glab`/`gh` процесса;
  на сервере это отдаст токен сервиса любому пользователю. Плюс `AppState`
  (отчёты с общими id, входы по хосту) в одном процессе на всех.
- **Фронт завязан на десктоп в 4 местах**: `shared/api/api.ts` (`invoke`,
  `Channel`), `shared/lib/dialogs.ts` (`ask`/`message`), `features/add-project/model/pickFolder.ts`
  (выбор папки), «Сохранить отчёт» только через меню ⌘S. Остальное — обычный
  React в браузере (hash-роутер, `matchMedia`, `navigator.language`).

## 1. Tauri-команды

Все 22 команды — `async`, первым делом зовут `ensure_form(&webview)`
(`src-tauri/src/commands.rs:42`) — проверку, что вызов пришёл из окна `form`.
Список ведётся в трёх местах: `src-tauri/build.rs:5` (`COMMANDS`),
`src-tauri/src/main.rs:23` (`generate_handler!`), `src-tauri/capabilities/form.json`;
тест `src-tauri/src/main.rs:103` сверяет их. `Cmd<T> = Result<T, CmdError>`.
Параметры `app: AppHandle`, `webview: Webview`, `state: State<AppState>` вводит
Tauri; во фронт-API (`src/shared/api/api.ts:39-81`) они не видны.

| Команда | Сигнатура (без `AppHandle`/`Webview`/`State`) | Строка | Что читает/пишет |
|---|---|---|---|
| `hosts` | `() -> Vec<HostInfo>` | commands.rs:84 | связка ключей, `hosts.json`, `glab`, `gh`, env, `settings.json` |
| `set_token` | `(host: String, token: String) -> ()` | :112 | связка ключей, `hosts.json` |
| `remove_token` | `(host: String) -> ()` | :121 | связка ключей, `hosts.json` |
| `oauth_login` | `(host: String, client_id: Option<String>, on_code: Channel<UserCode>) -> bool` | :132 | opener (системный браузер), loopback-порт, связка ключей, окно |
| `oauth_cancel` | `(host: String) -> ()` | :183 | `AppState.logins` |
| `history` | `() -> Vec<HistoryEntry>` | :190 | `history.json` |
| `remove_history` | `(at: String) -> Vec<HistoryEntry>` | :196 | `history.json` |
| `clear_history` | `() -> ()` | :206 | `history.json` |
| `projects` | `(host: String, search: String, after: Option<String>) -> Page<Project>` | :212 | токен + API хоста |
| `branches` | `(host: String, project: String, search: String) -> Vec<String>` | :225 | токен + API |
| `pipelines` | `(host: String, project: String, ref: Option<String>, workflow: Option<String>, after: Option<String>) -> Page<Pipeline>` | :238 | токен + API |
| `workflows` | `(host: String, project: String) -> Vec<Workflow>` | :261 | токен + API |
| `host_provider` | `(host: String) -> Provider` | :274 | `settings.json` (кэш), проба `https://{host}/api/v3/meta` |
| `build` | `(form: Form, on_progress: Channel<Progress>) -> u32` | :289 | токен + API, `settings.json`, `history.json`, `AppState.reports` |
| `report` | `(id: u32) -> String` (JSON) | :349 | `AppState.reports` |
| `find_report` | `(host: String, request: Request) -> Option<u32>` | :358 | `AppState.reports` |
| `set_current_report` | `(id: Option<u32>) -> ()` | :370 | `AppState.current_report` (цель ⌘S) |
| `saved_projects` | `() -> Vec<SavedProject>` | :381 | `projects.json` |
| `add_project` | `(input: String) -> SavedProject` | :391 | локальный `.git/config`, токен + API, `projects.json` |
| `remove_project` | `(project: ProjectRef) -> Vec<SavedProject>` | :414 | `settings.json`, `projects.json` |
| `get_settings` | `() -> AppSettings` | :425 | `settings.json` |
| `set_settings` | `(patch: SettingsPatch) -> AppSettings` | :432 | `settings.json`, пересборка меню |

Вне команд: меню (`src-tauri/src/menu.rs:142`) — «Новый», «Сохранить» (⌘S →
`reports::save_current`, `src-tauri/src/reports.rs:14`), масштаб, полный экран;
жизненный цикл macOS (`src-tauri/src/main.rs:69`).

## 2. Места, привязанные к десктопу

Легенда: **как есть** — код годится серверу без правок; **с изменениями** — логика
нужна, но источник данных/транспорт другой; **не переносится** — на сервере не
нужно или опасно.

### `app/core` (`longpole-core`)

| Файл:строка | Что делает | Сервер | Почему |
|---|---|---|---|
| `Cargo.toml:12`, `:31-38` | `keyring-core` + платформенные хранилища (Keychain, Windows, dbus Secret Service с vendored-крипто) | не переносится | в контейнере нет связки ключей; Linux-сборка тянет dbus. Нужна фича-флаг или вынос `tokens` из ядра |
| `src/tokens.rs:24` | `SERVICE = "dev.longpole.desktop"` | не переносится | имя записи в связке ключей |
| `src/tokens.rs:27` | `platform_store()` — выбор хранилища по ОС | не переносится | см. выше |
| `src/tokens.rs:67-236` | `TokenStore`: токен хоста в связке ключей, `hosts.json`, обновление OAuth через `block_on` под замком хоста | не переносится | один набор токенов на машину; на сервере токен — в cookie сессии. Логику «свежий/просроченный → refresh» можно взять (`Credential::is_fresh`, `oauth::refresh`), но хранить обновлённую пару — в новую cookie |
| `src/tokens.rs:218` | `refresh_lock` — `HashMap<host, Mutex>` | не переносится | ключ — хост, а не пользователь (см. § 3) |
| `src/tokens.rs:240` | `find_token`: связка ключей → `glab config get token` → `GITLAB_TOKEN` при `GITLAB_HOST` | не переносится | **на сервере отдаст токен процесса/`glab` сервиса любому пользователю** |
| `src/tokens.rs:285` | `list_hosts`: хосты из связки, `glab`, `GITLAB_HOST` | не переносится | хосты задаёт администратор в конфиге |
| `src/tokens.rs:328-453` | `Glab`/`Gh::find`, `FIXED_DIRS` (`/opt/homebrew/bin`…), `tool_value` — запуск CLI | не переносится | CLI на сервере не нужен; запуск процессов на запрос — лишняя поверхность |
| `src/tokens.rs:457` | `find_github_token`: связка → `gh auth token` → `GH_TOKEN`/`GITHUB_TOKEN`/`GH_ENTERPRISE_TOKEN` | не переносится | то же, что `find_token` |
| `src/tokens.rs:486` | `github_hosts`: `gh auth status`, env | не переносится | то же |
| `src/oauth.rs:32-34`, `:80` | `GITHUB_CLIENT_ID`/`GITLAB_CLIENT_ID` и `client_id(host, given)` — свой или введённый пользователем client_id | с изменениями | на сервере client_id (и secret) из конфига администратора, пользователь его не вводит |
| `src/oauth.rs:38-61` | `Credential` (access/refresh/expires_at/token_url/redirect_uri) | как есть | годится как содержимое cookie; нет поля `client_secret` — его подставлять из конфига при refresh |
| `src/oauth.rs:104` | `refresh` без `client_secret` | с изменениями | confidential-приложению нужен `client_secret` в теле запроса |
| `src/oauth.rs:289` | `login` — GitHub Device Flow / GitLab PKCE с `open` (браузер) | не переносится | сервису нужен authorization code flow с redirect на свой URL и callback-эндпоинтом; Device Flow на вебе не нужен |
| `src/oauth.rs:314` | `github` — Device Flow, опрос в цикле до 600 с (`LOGIN_TIMEOUT`, `:23`) | не переносится | держит соединение/задачу 10 минут на пользователя |
| `src/oauth.rs:364-406` | `gitlab` — PKCE, `TcpListener::bind("127.0.0.1:0")`, redirect `http://127.0.0.1:{port}/callback` | не переносится | loopback на сервере недостижим из браузера пользователя. PKCE-параметры (`random_token`, `:409`; challenge, `:372`) можно переиспользовать |
| `src/oauth.rs:415-483`, `src/oauth_page.html` | ответ браузеру на loopback-порту, страница «вход выполнен» | не переносится | callback делает HTTP-роутер сервера |
| `src/oauth.rs:214` | `post_form` — POST к эндпоинту токена без редиректов | как есть | |
| `src/json_file.rs:18-36` | чтение/атомарная запись JSON-файла | не переносится (для данных пользователя) | сервер stateless, базы нет |
| `src/hosts.rs:41-74` | `HostList` — `hosts.json` | не переносится | список хостов — из конфига |
| `src/hosts.rs:31` | `normalize_host` | как есть | |
| `src/history.rs:58-114` | `History` — `history.json`, 20 записей | не переносится | по карте «Недавние» — в браузере; `stored_url` (`:48`, вырезает `?private_token=`) стоит перенести во фронт |
| `src/projects.rs:33-85` | `Projects` — `projects.json` | не переносится | «Мои проекты» — в браузере |
| `src/settings.rs:71-155` | `Settings` — `settings.json`: язык, тема, `last_project`, подсказки, `host_kinds` | не переносится (пользовательская часть) / с изменениями (`host_kinds`) | язык, тема, подсказки — в браузере; тип хоста на сервере известен из конфига |
| `src/settings.rs:87`, `src/schema.rs:64` | `Locale::system()` через `sys_locale` | с изменениями | на сервере это локаль контейнера; язык брать из запроса/браузера |
| `src/source.rs:28-45` | `resolve_provider`: кэш в `settings.json`, иначе проба `https://{host}/api/v3/meta` без токена | с изменениями | на сервере тип из конфига; проба произвольного хоста по вводу пользователя — SSRF |
| `src/gitlab/client.rs:35`, `src/github/client.rs:27` | клиенты ходят на `https://{host}` с токеном, без редиректов | как есть | но хост должен проверяться против списка из конфига до создания клиента (SSRF) |
| `src/request.rs:157-163`, `:213` | `parse_project_input`: абсолютный путь → читает `.git/config` (и `gitdir:`/`commondir`) с диска | с изменениями | на сервере ветку с путём надо отключить: иначе пользователь читает файлы сервера по своему пути |
| `src/report.rs:143`, `src/render.rs:17` | сборка отчёта и вставка JSON в шаблон | как есть | шаблон передаётся строкой; сейчас его вшивает оболочка (`src-tauri/src/reports.rs:11`) |
| `src/lib.rs:1` | «Ядро без зависимостей от Tauri» | — | подтверждается: `tauri` в `core/Cargo.toml` нет |

### `app/src-tauri`

| Файл:строка | Что делает | Сервер | Почему |
|---|---|---|---|
| `src/main.rs:15-61` | Builder, плагины `dialog`/`opener`, `generate_handler!`, `app.manage(AppState)`, меню, окно | не переносится | у сервера свой роутер; реестр команд стоит описать один раз для обеих оболочек |
| `src/main.rs:48` | `AppState::new(app_data_dir)` — каталог данных ОС | не переносится | |
| `src/main.rs:69-83` | жизнь в Dock на macOS | не переносится | |
| `src/state.rs:31-131` | `AppState`: хранилища + отчёты/входы/масштаб в памяти | с изменениями | см. § 3 |
| `src/commands.rs:42` | `ensure_form` — вызов только из окна `form` | не переносится | на сервере вместо этого — сессия, CSRF |
| `src/commands.rs:54-80` | `source_for`: тип хоста → `find_token`/`find_github_token` в `spawn_blocking` → клиент | с изменениями | серверный аналог: хост из конфига, токен из сессии; логика выбора `Client` по `Provider` та же |
| `src/commands.rs:60`, `:88` | `std::env::var` как источник токенов | не переносится | см. `find_token` |
| `src/commands.rs:148-153` | `opener().open_url` — авторизация GitLab в системном браузере | не переносится | на вебе — редирект в той же вкладке |
| `src/commands.rs:137`, `:294` | `Channel<UserCode>`, `Channel<Progress>` — поток событий во фронт | с изменениями | SSE или стриминг ответа `fetch` |
| `src/commands.rs:178` | `show_form` после входа — вернуть окно | не переносится | |
| `src/commands.rs:289-345` | `build`: собрать, отрендерить HTML, записать историю, положить в `AppState.reports`, вернуть числовой id | с изменениями | ядро вызывается так же; история — в браузере; хранение отчёта — кэш с ключом не по счётчику (§ 3) |
| `src/commands.rs:313` | `state.locale()` — язык отчёта из настроек/системы | с изменениями | язык из запроса |
| `src/commands.rs:398` | `parse_project_input` в `spawn_blocking` (может читать диск) | с изменениями | только ссылки, без путей |
| `src/commands.rs:442` | смена языка пересобирает нативное меню | не переносится | |
| `src/reports.rs:11` | `TEMPLATE = include_str!("../../dist-report/report.html")` | как есть (переместить) | серверу тот же шаблон; вшивать в обеих оболочках или в ядре |
| `src/reports.rs:14-45` | ⌘S: нативный диалог сохранения, `std::fs::write` | не переносится | на вебе — скачивание (`Content-Disposition` или `Blob`) |
| `src/menu.rs`, `src/strings.rs` | нативное меню и его строки на 8 языках | не переносится | |
| `src/windows.rs:13-105` | окно формы, запрет навигации, внешние `https` — в системный браузер | не переносится | в браузере это обычные ссылки |
| `tauri.conf.json:15-16` | CSP (`connect-src ipc: http://ipc.localhost`) | с изменениями | серверу свои заголовки (CSP с `connect-src 'self'`) |
| `capabilities/form.json` | ACL команд для окна `form` | не переносится | |

### `app/src` (фронт)

| Файл:строка | Что делает | Сервер | Почему |
|---|---|---|---|
| `shared/api/api.ts:1`, `:31-37` | `invoke` из `@tauri-apps/api/core` в обёртке `call` | с изменениями | единственная точка транспорта: подменить `call` на `fetch` — остальные 20 функций не меняются. Ошибку `ipc` (`:21`) переименовать в транспортную |
| `shared/api/api.ts:43-46`, `:60-63` | `new Channel` для кода входа и прогресса сборки | с изменениями | SSE/`ReadableStream`; вход на вебе — редирект, `onCode` не нужен |
| `shared/api/api.ts:70-76` | `setCurrentReport` — сообщает ядру, какой отчёт на экране (для ⌘S) | не переносится | на вебе «Сохранить» — кнопка со скачиванием |
| `shared/lib/dialogs.ts:1-22` | `ask`/`message` из `@tauri-apps/plugin-dialog` | с изменениями | `window.confirm`/свой модальный диалог; обёртка уже одна |
| `features/add-project/model/pickFolder.ts:1-12` | системный выбор папки | не переносится | папки с клоном на сервере нет; кнопку (`widgets/add-project-dialog/ui/AddProjectDialog.tsx:44`, `:65`) скрыть |
| `app/form/report-route.tsx:25-27` | вызывает `setCurrentReport` | с изменениями | см. выше |
| `app/form/report-route.tsx:16` + `router.tsx:91` | отчёт по числовому id из памяти процесса | с изменениями | для «отчёта по ссылке» id должен быть адресом запроса, а не счётчиком |
| `features/manage-token/ui/OauthLogin.tsx:18-48` | поле client_id для self-managed хоста | не переносится | client_id задаёт администратор |
| `features/manage-token/ui/OauthWaiting.tsx:22` | код Device Flow GitHub со ссылкой | не переносится | на вебе — web flow |
| `pages/settings/ui/SettingsPage.tsx:50-56`, `widgets/projects-sidebar/ui/ProjectsSidebar.tsx:116` | источник токена (`keychain`/`oauth`/`glab`/`gh`/`env`) и «Удалить токен» | с изменениями | на вебе источник один (OAuth-сессия); «Удалить» → «Выйти» |
| `widgets/projects-sidebar/ui/ProjectsSidebar.tsx:46` | `findReport` — повтор из «Недавних» без сети | с изменениями | на сервере — общий кэш отчётов с проверкой прав |
| `app/form/main.tsx:20` | dev-мок `window.__TAURI_INTERNALS__` (`app/dev/mock.ts`) | с изменениями | при `fetch`-транспорте мок станет моком `fetch`/MSW или не нужен |
| `app/form/main.tsx:23` | язык и тема из `get_settings` до рендера | с изменениями | в веб-версии — из `localStorage` |
| `app/form/router.tsx:96` | `createHashHistory()` | как есть | работает в браузере; для отчёта по ссылке удобнее browser history |
| `shared/lib/theme.ts:3`, `shared/i18n/index.ts:42` | `matchMedia`, `navigator.language` | как есть | веб-API |
| `package.json` | `@tauri-apps/api`, `@tauri-apps/plugin-dialog` в зависимостях | с изменениями | импортируются только в `api.ts`, `dialogs.ts`, `pickFolder.ts` — их и подменять по флагу поставки |

`localStorage`/`sessionStorage` во фронте сейчас не используются: всё состояние
пользователя живёт в файлах ядра.

## 3. Общее состояние: риск утечки между пользователями

Всё ниже — поля одного `AppState` или статические данные процесса. В десктопе
пользователь один; в сервере один процесс обслуживает всех, и состояние станет
общим.

| Где | Состояние | Чем грозит в сервере |
|---|---|---|
| `src-tauri/src/state.rs:39`, `:44`, `:63-71` | `reports: VecDeque<(u32, ReportEntry)>` на 10 отчётов, id — глобальный `AtomicU32` с 1 | **утечка**: id предсказуемы, `report(id)` отдаст чужой отчёт (данные приватного проекта); чужие сборки вытесняют твои |
| `src-tauri/src/state.rs:81-87` | `find_report(host, request)` — свежайший отчёт по хосту и запросу | **утечка**: отдаёт отчёт, собранный другим пользователем, без проверки, есть ли у текущего доступ к проекту |
| `src-tauri/src/state.rs:41`, `:89-100` | `current_report: Option<u32>` — один «отчёт на экране» | ⌘S одного пользователя сохранил бы отчёт другого; на сервере не нужен |
| `src-tauri/src/state.rs:37`, `:121-130` | `logins: HashMap<host, oneshot::Sender>` | вход одного пользователя на хост отменяет начатый вход другого на тот же хост; `oauth_cancel` чужой |
| `src-tauri/src/state.rs:43` | `zoom: HashMap<label, f64>` | безвредно, на сервере не нужно |
| `src-tauri/src/state.rs:50`, `core/src/tokens.rs:67-72` | `TokenStore`: один токен на хост, `hosts.json`, замки обновления по хосту | **утечка**: токен, сохранённый одним пользователем, получит любой, кто обратится к тому же хосту |
| `core/src/tokens.rs:240`, `:457`; `src-tauri/src/commands.rs:60`, `:88` | `GITLAB_TOKEN`/`GH_TOKEN`/`GITHUB_TOKEN`/`GH_ENTERPRISE_TOKEN` и `glab`/`gh` процесса | **утечка**: служебный токен контейнера станет токеном каждого пользователя без своего (карта: «общего сервисного токена нет») |
| `src-tauri/src/state.rs:51`, `core/src/history.rs:58` | `history.json` — одна история | все видят «Недавние» всех (проекты, ветки, MR) |
| `src-tauri/src/state.rs:53`, `core/src/projects.rs:33` | `projects.json` — один список | «Мои проекты» общие |
| `src-tauri/src/state.rs:52`, `core/src/settings.rs:71` | `settings.json` — язык, тема, `last_project`, подсказки | один пользователь меняет язык/последний проект всем; `host_kinds` — кэш типа хоста, общий, но не секретный (и на сервере не нужен) |
| `core/src/settings.rs:132`, `src-tauri/src/commands.rs:299-303` | `build` в режиме ссылки переписывает `host_kinds` | пользователь может «перекрасить» тип хоста для всех (порча кэша); на сервере тип — из конфига |
| `core/src/hosts.rs:11`, `core/src/model.rs:9`, `core/src/request.rs:16-41`, `core/src/report.rs:22`, `core/src/github/workflow.rs:11` | `static LazyLock<Regex>` | безопасно: неизменяемые |

Клиенты GitLab/GitHub (`Semaphore` на `MAX_PARALLEL`, `core/src/gitlab/client.rs:53`)
создаются на каждый вызов и состояния между пользователями не делят.

## Что из этого следует для спеки (не решение, а входные данные)

1. Серверной оболочке нужен свой `source_for` (хост из конфига → токен из сессии)
   и свой `oauth` web flow с `client_secret`; из ядра используются `Credential`,
   `refresh` (с секретом), `post_form`, PKCE-утилиты.
2. `tokens.rs` и платформенные keyring-крейты стоит убрать из сборки сервера
   (фича Cargo или вынос в `src-tauri`), иначе Linux-образ тянет dbus и в коде
   остаётся путь к env/CLI-токенам.
3. `AppState.reports` не годится как кэш «отчёта по ссылке»: нужен ключ
   `(host, request)` и проверка доступа пользователя при каждой выдаче.
4. `parse_project_input` на сервере — только ветка без файловой системы.
5. Фронту хватит подмены трёх модулей (`api.ts`, `dialogs.ts`, `pickFolder.ts`)
   плюс скрытия UI токенов/папки и замены ⌘S на кнопку скачивания.

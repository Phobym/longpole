# Tauri 2 против `desktop/app/main.mjs`: чем закрыть каждое поведение

Дата обращения ко всем источникам: 2026-09-25.

Эталон поведения: `desktop/app/main.mjs`, `desktop/app/preload.cjs`,
`desktop/app/tokens.mjs`, `docs/specs/2026-09-24-desktop-design.md`.

Версии (crates.io, [S46]): `tauri` 2.11.6 — последняя стабильная (в ветке
уже вышла `3.0.0-alpha.2`, её не рассматриваем), `tauri-plugin-dialog` 2.7.3,
`tauri-plugin-opener` 2.5.5, `keyring` 4.2.0, `keyring-core` 1.0.0.
Ссылки на исходники закреплены на теге `tauri-v2.11.6`.

## 1. Вывод

Почти всё поведение `main.mjs` переносится штатными API Tauri 2 и двумя
плагинами (`dialog`, `opener`). Жёстких блокеров нет. Есть четыре места, где
паритет требует явного обхода, и одно, где он недостижим буквально:

1. **IPC в окне отчёта нельзя убрать физически.** Tauri внедряет мост IPC
   в каждый webview. Отказ даёт только ACL: без `AppManifest::commands` в
   `build.rs` свои команды доступны *любому* локальному окну
   ([S20], [S21]). Нужны манифест команд, capability только на label
   `form` и проверка label внутри команды (аналог `fromForm`).
2. **Выход после закрытия последнего окна на macOS.** Electron-код не
   выходит (`window-all-closed`), Tauri по умолчанию выходит. Нужен
   `RunEvent::ExitRequested` → `api.prevent_exit()` на macOS и
   `RunEvent::Reopen` вместо `activate` ([S16], [S17]).
3. **Меню на Linux/Windows.** Нет предопределённых пунктов `undo`/`redo`/
   `fullscreen` (Win/Linux) и `quit`/`close_window`/`minimize` (Linux), нет
   аналогов ролей `viewMenu` (reload, zoom, devtools). Их придётся собрать из
   своих пунктов ([S10], [S15]).
4. **Пути данных.** `app_data_dir` не совпадает с Electron `userData`
   ни по имени каталога (bundle identifier против имени приложения), ни на
   Linux (`~/.local/share` против `~/.config`) ([S36], [S37]).
5. **Недостижимо буквально: перенос сохранённых токенов.** `tokens.json`
   зашифрован ключом Chromium `safeStorage`. Rust-код его не расшифрует, и
   при переходе на `keyring` токены придётся ввести заново. Это вывод из
   устройства хранилища, а не из документации Tauri.

Прочее: отчёт отдаётся через свою URI-схему (`data:` не поддерживается),
навигация режется через `on_navigation`/`on_new_window`, прогресс идёт через
`Channel`, `glab` ищется через `fix-path-env` или фиксированный список путей.
По размеру бандла выигрыш на порядки (единицы МБ против ~120–150 МБ). По
памяти первоисточников с цифрами нет, её нужно мерить.

## 2. Таблица: поведение Electron → механизм Tauri → оговорки

| # | Поведение в `main.mjs` | Механизм Tauri 2 | Оговорки |
|---|---|---|---|
| 1a | Окно формы 1280×860, min 900×640, `title: 'pipeline-trace'` | `WebviewWindowBuilder::new(app, "form", WebviewUrl::App("index.html"))` + `.inner_size()`, `.min_inner_size()`, `.title()` [S1], [S2] | Label `form` — граница безопасности: capabilities привязываются к label, а не к заголовку [S20] |
| 1b | Окно на каждый отчёт, `loadFile(file)` из `mkdtemp` | Своя схема `register_asynchronous_uri_scheme_protocol("report", …)` [S3]. Обработчик берёт `ctx.webview_label()` [S4] и отдаёт байты файла (или буфера в памяти) из `HashMap<label, …>`. Окно: `WebviewUrl::CustomProtocol("report://localhost/")` [S2] | Origin зависит от ОС: macOS/Linux `report://localhost/…`, Windows `http://report.localhost/…` [S3]. Из памяти можно отдавать без временного файла, но «Сохранить» тогда пишет буфер, а не `copyFile` |
| 1c | Альтернатива: asset protocol | `app.security.assetProtocol.enable` + `scope` (glob) [S5] | Предназначен для подгрузки файлов из страницы, требует scope на временный каталог. Своя схема проще и не открывает файловую систему. Рекомендация: своя схема |
| 1d | Альтернатива: `data:` | Не поддерживается: `WebviewUrl::External` допускает только `http`/`https` [S2] | Была бы ещё и non-local origin [S21]. Отказаться |
| 1e | Заголовок отчёта фиксирован (`page-title-updated` → `preventDefault`) | `.title(label)` при создании [S1]. Синхронизации `document.title` → заголовок окна по умолчанию нет: обработчик смены заголовка подключается только при явном `on_document_title_changed` [S1], [S48] | Совпадает с Electron без дополнительного кода |
| 2a | `will-navigate` → запрет любой навигации | `.on_navigation(\|url\| …)`, `false` отменяет [S1]. Tauri оборачивает пользовательский обработчик, и отказ из него окончательный [S7] | Разрешать нужно собственный origin (`report://localhost` / `http://report.localhost`), иначе может не загрузиться первая страница. Срабатывает ли обработчик на первый URL, документация wry не говорит [S47]: проверить руками на трёх ОС |
| 2b | `setWindowOpenHandler`: `https:` → `shell.openExternal`, всегда `deny` | `.on_new_window(\|url, _\| { if url.scheme()=="https" { app.opener().open_url(url, None::<&str>) } NewWindowResponse::Deny })` [S1], [S6], [S8] | В шаблоне отчёта есть `target="_blank"` (строки 489, 563 `src/template.html`), так что путь реальный. Scope плагина opener (`mailto:`, `tel:`, `http(s)`) действует только на вызовы из JS [S8]: проверку `https:` делаем сами в Rust, как сейчас |
| 2c | Клик по обычной ссылке без `_blank` | Попадает в `on_navigation` → `false`. Во внешний браузер не открываем, как в Electron (там `will-navigate` только отменяет) | — |
| 3a | Меню «Файл → Новый отчёт» `CmdOrCtrl+N`, «Сохранить отчёт…» `CmdOrCtrl+S` | `MenuBuilder`/`SubmenuBuilder`, `MenuItem` с accelerator, `Builder::menu()` + `on_menu_event()` [S3], [S9]. Строка `CmdOrCtrl` разбирается muda: META на macOS, CONTROL на остальных [S13] | Сфокусированное окно: `Manager::get_focused_window` только за фичей `unstable` [S14]. Без неё перебираем `webview_windows()` и берём то, у которого `is_focused()` [S49] |
| 3b | «Новый отчёт»: `formWindow.focus()` или создать | `get_webview_window("form")` → `set_focus()`, иначе построить заново [S49] | — |
| 3c | macOS: `appMenu` первым, `role: 'close'` | На macOS первый submenu становится меню приложения, верхний уровень — только submenu [S9], [S11]. Пункты `about`, `services`, `hide`, `hide_others`, `quit`, `close_window` есть как `PredefinedMenuItem` [S10]; `Menu::default` собирает ровно такое меню [S12] | Паритет полный |
| 3d | Win/Linux: `role: 'quit'` «Выход» | `PredefinedMenuItem::quit` на Linux «Unsupported» [S10] | На Linux — свой пункт, который вызывает `app.exit(0)` |
| 3e | `editMenu` | `undo`, `redo` на Win/Linux «Unsupported»; `cut`, `copy`, `paste`, `select_all` поддержаны везде [S10] | На Win/Linux пункты «Отменить/Повторить» из меню не собрать предопределёнными. Работают ли Ctrl+Z/Ctrl+Y в полях формы силами самого webview, первоисточник не говорит: проверить |
| 3f | `viewMenu` (reload, zoom, devtools, fullscreen) | Предопределённых пунктов нет, кроме `fullscreen` (только macOS) [S10]. Свои пункты: `Webview::reload()`, `set_zoom()`, `open_devtools()` [S15] | Devtools в релизе — только с фичей `devtools`, на macOS это приватные API [S15], [S47]. Решить, нужны ли они в релизе вообще |
| 3g | `windowMenu` | `minimize`, `maximize` (Linux нет), `close_window` (Linux нет), `bring_all_to_front` (только macOS) [S10], [S12] | На Linux подменю «Окно» почти пустое |
| 3h | Меню на Win/Linux | Меню привязано к окну, а не глобально [S9], [S11]. `set_menu` у app назначает меню окнам без собственного [S14] | Все окна отчёта получат полосу меню, как в Electron с `setApplicationMenu` |
| 4 | `showSaveDialog(win, { defaultPath: name, filters: html })`, ошибка → `showErrorBox` | `app.dialog().file().set_parent(&win).set_file_name(name).add_filter("HTML", &["html"]).save_file(cb)` [S18], [S19]. Ошибка: `.message(..).kind(MessageDialogKind::Error)` [S18] | Из обработчика меню (главный поток) только неблокирующий `save_file`; `blocking_save_file` на главном потоке запрещён [S19]. Разрешение `dialog:allow-save` нужно только для вызова из JS [S18], а у нас вызов из Rust |
| 5a | `fromForm(event)`: каналы отвечают только окну формы | (1) `tauri_build::AppManifest::new().commands(&[...])` в `build.rs` генерирует `allow-*`/`deny-*` для своих команд [S22]. (2) Capability с `"windows": ["form"]` и этими разрешениями [S20]. (3) Внутри команды параметр `WebviewWindow`, проверка `label() == "form"` [S26] | Без шага (1) все свои команды доступны всем локальным окнам [S20], и `report://` считается локальным, потому что это зарегистрированная пользователем схема [S21]. С манифестом ACL проверяется для каждой команды [S21] |
| 5b | У окна отчёта IPC нет совсем (нет preload) | Нет. `__TAURI_INTERNALS__` и invoke-ключ внедряются в каждый webview [S21]; отказ — только на уровне ACL. Окно без capability не получит ни свои команды (при манифесте), ни `core:*`/плагины: плагинные команды проверяются всегда [S21] | **Буквальный паритет недостижим**, функциональный — да. Дополнительно CSP отчёта `connect-src 'none'` закрывает транспорт `ipc:`/`http://ipc.localhost` [S23] |
| 5c | Форма: CSP `default-src 'self'; style-src 'self' 'unsafe-inline'` | `app.security.csp` в `tauri.conf.json`; Tauri дописывает хэши и nonce для бандла [S5], [S23]. Для IPC нужен `connect-src ipc: http://ipc.localhost` [S23] | CSP-заголовок ставится только на ассеты протокола `tauri://` [S24]. Meta-тег в `index.html` можно оставить |
| 5d | Отчёт с инлайн-`<script>` (строка 208 шаблона) | На свою схему глобальный CSP не распространяется [S24]: заголовок `Content-Security-Policy` ставим сами в ответе обработчика схемы [S3]. Например `default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data:; connect-src 'none'` | Сейчас у отчёта в Electron CSP нет вовсе, так что это усиление, а не паритет. Вместо `'unsafe-inline'` можно считать `sha256-` хэш скрипта при `render()` |
| 6 | `event.sender.send('progress', p)` в окно, вызвавшее `build` | `tauri::ipc::Channel<Progress>` параметром команды `build`, `channel.send(p)` [S25], [S26], [S27]. Альтернатива — `emit_to("form", …)` [S25] | Документация рекомендует каналы для прогресса: они быстрые и упорядоченные [S25]. События не подчиняются capabilities [S25], поэтому для данных только формы канал точнее. `send` возвращает `Result` [S27]: закрытое окно формы — ошибка, игнорируем (аналог `isDestroyed`) |
| 6b | Ошибка возвращается значением `{ ok, error }`, без префикса Electron | `Result<T, E: Serialize>`: промис в JS отклоняется сериализованным `E` как есть [S26] | Префикса нет, можно отказаться от обёртки `{ ok }` или сохранить её ради UI |
| 7a | `safeStorage` → `tokens.json` | `keyring` 4.x — теперь «зонтик» для CLI; приложениям рекомендуют `keyring-core` + сторы по платформам: `apple-native-keyring-store`, `windows-native-keyring-store`, `dbus-secret-service-keyring-store` / `zbus-secret-service-keyring-store` [S28], [S29]. Нужен `set_default_store`, иначе `NoDefaultStore` [S29] | Вариант попроще — `keyring` 3.x с фичами `apple-native`, `windows-native`, `sync-secret-service`. Без фич 3.x **молча** использует mock-хранилище в памяти [S30]: фичи обязательны. Список хостов (`hosts()`) придётся хранить отдельно, например в JSON без секретов |
| 7b | Linux без keyring: отказ «Системная связка ключей недоступна…» | `dbus-secret-service-keyring-store`: `Store::new()` сразу вызывает `SecretService::connect` и при неудаче возвращает `PlatformFailure` [S32]. Заблокированная коллекция, отказ в prompt, отсутствие результата → `NoStorageAccess` [S33] | **Обнаружить можно**: создать стор при старте; ошибка → хранение токенов выключено с тем же сообщением, env и `glab` работают. Но подключение к D-Bus ещё не гарантирует доступ, поэтому `NoStorageAccess` при `set_password` тоже показываем как это сообщение. Под WSL нет default-коллекции [S31]; известны проблемы в headless-окружениях [S31]; обращения из нескольких потоков не рекомендуются [S30] |
| 8 | `execFile('glab', …)` из `.app` | `std::process::Command::new("glab")` ищет по `PATH` процесса. GUI-приложения на macOS/Linux не наследуют `PATH` из dotfiles [S34]. `fix_path_env::fix()` запускает `$SHELL -ilc` (по умолчанию `/bin/zsh` на macOS) и переносит `PATH` в процесс [S35] | В Electron та же проблема: `execFile` тоже видит PATH launchd, так что это не регрессия. `fix-path-env` не опубликован на crates.io, это git-зависимость [S46]. Запуск login-shell стоит времени и зависит от dotfiles; альтернатива — попробовать `PATH`, затем `/opt/homebrew/bin/glab` и `/usr/local/bin/glab` (эти пути — вывод, не из первоисточника Tauri) |
| 9 | `app.getPath('userData')` → `tokens.json`, `history.json` | `app.path().app_data_dir()` = `data_dir/${bundle_identifier}`: macOS `~/Library/Application Support/<id>`, Windows `{FOLDERID_RoamingAppData}\<id>`, Linux `$XDG_DATA_HOME` или `~/.local/share/<id>` [S36] | Electron `userData` = `appData` + имя приложения; на Linux `appData` — `~/.config` [S37]. Ближе к Electron `app_config_dir` (на Linux `~/.config`) [S36], но имя каталога всё равно identifier. История без миграции потеряется: либо один раз копировать из старого пути, либо принять |
| 9b | `mkdtemp(tmpdir())` | `app.path().temp_dir()` = `std::env::temp_dir` [S36] + свой уникальный подкаталог | При схеме «из памяти» временный каталог не нужен |
| 9c | `window-all-closed` → `quit` кроме macOS; `activate` → окно формы | Выход после последнего окна — поведение Tauri по умолчанию; на macOS гасить через `RunEvent::ExitRequested { api, .. }` → `api.prevent_exit()` [S17] (при `code == None`, чтобы «Выход» из меню работал). `activate` → `RunEvent::Reopen { has_visible_windows }` [S16] | `Reopen` приходит из `applicationShouldHandleReopen` [S16]. Условие Electron «нет формы» проверяем сами через `get_webview_window("form")` |
| 10 | Размер и память | См. раздел 3 | — |

## 3. Размер бандла и память

**Размер.** Tauri не везёт браузерный движок и использует системный webview
[S38]: WebView2 на Windows, WKWebView на macOS, webkit2gtk на Linux [S44].
«A minimal Tauri app can be less than 600KB in size» [S38]. Профиль
`release` с `lto`, `opt-level = "s"`, `strip` и отсечение неиспользуемых
команд по ACL (Tauri ≥ 2.4) уменьшают его дальше [S39]. Исключения:

- AppImage везёт зависимости, включая WebKit: «the file size grows from the
  2-6 MB range to 70+ MB» [S40]. `.deb` зависит от системного
  `libwebkit2gtk-4.1` [S40].
- Windows: по умолчанию WebView2 скачивается bootstrapper'ом (+0 МБ).
  `embedBootstrapper` добавляет ~1.8 МБ, `offlineInstaller` ~127 МБ,
  `fixedVersion` ~180 МБ [S41]. В Windows 10 (с апреля 2018) и 11 рантайм
  уже есть [S41].

Electron везёт Chromium целиком. Сам рантайм Electron v44.4.5 в zip, ещё без
кода приложения: darwin-arm64 — 130 418 529 байт (~124 МиБ), linux-x64 —
122 995 981 (~117 МиБ), win32-x64 — 158 184 819 (~151 МиБ) [S42].

Итог по порядку величин: единицы МБ (macOS, `.deb`, NSIS) против ~120–150 МБ.
Для AppImage разрыв сокращается до ~70 против ~120 МБ.

**Память.** Ни Tauri, ни Electron не публикуют измеренных цифр памяти: на
странице Tauri о памяти ничего нет [S38]. Из устройства движков:
Electron — многопроцессная модель Chromium, у каждого `BrowserWindow` свой
renderer [S43]. WebView2 — та же модель Edge: браузерный процесс, renderer'ы
и GPU-процесс на каждую user data folder, renderer может обслуживать
несколько webview [S45]. Поэтому на Windows разница с Electron в памяти —
не порядок величин. Её нужно мерить на целевом сценарии (форма плюс
N окон отчёта). Любые цифры «Tauri в N раз легче по памяти» из вторичных
источников сюда не включены.

## 4. Что проверить руками до реализации

1. `on_navigation` на первом переходе на `report://` на трёх ОС (строка 2a).
2. Ctrl+Z/Ctrl+Y в полях формы на Windows и Linux без пунктов меню (3e).
3. Linux без запущенного Secret Service: `Store::new()` действительно падает
   с `PlatformFailure`, а не поднимает `gnome-keyring` через D-Bus activation (7b).
4. Время `fix_path_env::fix()` при тяжёлом `.zshrc` (8).

## 5. Источники

- [S1] WebviewWindowBuilder, tauri 2.11.6 — https://docs.rs/tauri/2.11.6/tauri/webview/struct.WebviewWindowBuilder.html
- [S2] WebviewUrl — https://docs.rs/tauri/2.11.6/tauri/enum.WebviewUrl.html
- [S3] Builder (`register_*_uri_scheme_protocol`, `menu`, `on_menu_event`) — https://docs.rs/tauri/2.11.6/tauri/struct.Builder.html
- [S4] UriSchemeContext — https://docs.rs/tauri/2.11.6/tauri/struct.UriSchemeContext.html
- [S5] Configuration reference (security.csp, assetProtocol, capabilities) — https://v2.tauri.app/reference/config/
- [S6] NewWindowResponse — https://docs.rs/tauri/2.11.6/tauri/webview/enum.NewWindowResponse.html
- [S7] Обёртка navigation handler — https://github.com/tauri-apps/tauri/blob/tauri-v2.11.6/crates/tauri/src/manager/webview.rs#L577-L605
- [S8] Opener plugin — https://v2.tauri.app/plugin/opener/
- [S9] Window Menu — https://v2.tauri.app/learn/window-menu/
- [S10] PredefinedMenuItem — https://docs.rs/tauri/2.11.6/tauri/menu/struct.PredefinedMenuItem.html
- [S11] Menu — https://docs.rs/tauri/2.11.6/tauri/menu/struct.Menu.html
- [S12] `Menu::default` — https://github.com/tauri-apps/tauri/blob/tauri-v2.11.6/crates/tauri/src/menu/menu.rs#L142-L233
- [S13] muda, разбор `CmdOrCtrl` — https://github.com/tauri-apps/muda/blob/dev/src/accelerator/mod.rs#L251-L256
- [S14] App (`set_menu`, `get_focused_window` за `unstable`) — https://docs.rs/tauri/2.11.6/tauri/struct.App.html
- [S15] Webview (`reload`, `set_zoom`, `open_devtools`) — https://docs.rs/tauri/2.11.6/tauri/webview/struct.Webview.html
- [S16] `RunEvent::Reopen` — https://github.com/tauri-apps/tauri/blob/tauri-v2.11.6/crates/tauri/src/app.rs#L275-L281
- [S17] ExitRequestApi — https://docs.rs/tauri/2.11.6/tauri/struct.ExitRequestApi.html
- [S18] Dialog plugin — https://v2.tauri.app/plugin/dialog/
- [S19] FileDialogBuilder, tauri-plugin-dialog 2.7.3 — https://docs.rs/tauri-plugin-dialog/2.7.3/tauri_plugin_dialog/struct.FileDialogBuilder.html
- [S20] Capabilities — https://v2.tauri.app/security/capabilities/
- [S21] `is_local_url` и проверка ACL в `on_message` — https://github.com/tauri-apps/tauri/blob/tauri-v2.11.6/crates/tauri/src/webview/mod.rs#L1698-L1847
- [S22] AppManifest, tauri-build 2.6.3 — https://docs.rs/tauri-build/2.6.3/tauri_build/struct.AppManifest.html
- [S23] Content Security Policy — https://v2.tauri.app/security/csp/
- [S24] CSP-заголовок протокола `tauri://` — https://github.com/tauri-apps/tauri/blob/tauri-v2.11.6/crates/tauri/src/protocol/tauri.rs
- [S25] Calling the Frontend from Rust — https://v2.tauri.app/develop/calling-frontend/
- [S26] Calling Rust from the Frontend — https://v2.tauri.app/develop/calling-rust/
- [S27] ipc::Channel — https://docs.rs/tauri/2.11.6/tauri/ipc/struct.Channel.html
- [S28] keyring 4.2.0 — https://docs.rs/keyring/4.2.0/keyring/
- [S29] keyring-core 1.0.0 — https://docs.rs/keyring-core/1.0.0/keyring_core/
- [S30] keyring 3.6.3 (фичи, mock-хранилище по умолчанию, потоки) — https://docs.rs/keyring/3.6.3/keyring/
- [S31] dbus-secret-service-keyring-store 1.0.1 — https://docs.rs/dbus-secret-service-keyring-store/1.0.1/dbus_secret_service_keyring_store/
- [S32] там же, `Service::new` — https://docs.rs/crate/dbus-secret-service-keyring-store/1.0.1/source/src/service.rs
- [S33] там же, соответствие ошибок — https://docs.rs/crate/dbus-secret-service-keyring-store/1.0.1/source/src/errors.rs
- [S34] fix-path-env-rs, README — https://github.com/tauri-apps/fix-path-env-rs
- [S35] fix-path-env-rs, `fix()` — https://github.com/tauri-apps/fix-path-env-rs/blob/dev/src/lib.rs
- [S36] PathResolver — https://docs.rs/tauri/2.11.6/tauri/path/struct.PathResolver.html
- [S37] Electron `app.getPath` — https://www.electronjs.org/docs/latest/api/app#appgetpathname
- [S38] What is Tauri? — https://v2.tauri.app/start/
- [S39] App Size — https://v2.tauri.app/concept/size/
- [S40] AppImage — https://v2.tauri.app/distribute/appimage/
- [S41] Windows Installer (WebView2 install modes) — https://v2.tauri.app/distribute/windows-installer/
- [S42] Electron v44.4.5, ассеты релиза — https://github.com/electron/electron/releases/tag/v44.4.5
- [S43] Electron Process Model — https://www.electronjs.org/docs/latest/tutorial/process-model
- [S44] Webview Versions — https://v2.tauri.app/reference/webview-versions/
- [S45] WebView2 process model — https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/process-model
- [S46] crates.io: https://crates.io/crates/tauri, https://crates.io/crates/tauri-plugin-dialog, https://crates.io/crates/tauri-plugin-opener, https://crates.io/crates/keyring, https://crates.io/crates/keyring-core
- [S47] wry 0.57.0 WebViewBuilder — https://docs.rs/wry/0.57.0/wry/struct.WebViewBuilder.html
- [S48] Подключение document-title handler в runtime — https://github.com/tauri-apps/tauri/blob/tauri-v2.11.6/crates/tauri-runtime-wry/src/lib.rs#L4965-L4967
- [S49] WebviewWindow (`is_focused`, `set_focus`) и Manager (`webview_windows`, `get_webview_window`) — https://docs.rs/tauri/2.11.6/tauri/webview/struct.WebviewWindow.html, https://docs.rs/tauri/2.11.6/tauri/trait.Manager.html

Кроме источников из задания (v2.tauri.app, tauri на GitHub, docs.rs) для
сравнения использованы первичные страницы Electron [S37], [S42], [S43],
Microsoft [S45] и crates.io [S46].

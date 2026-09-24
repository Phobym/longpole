# pipeline-trace Desktop: приложение для macOS, Linux и Windows

Дата: 2026-09-24. Статус: дизайн утверждён, реализации нет.

## Что это

Десктопное приложение на Electron. Оно делает то же, что CLI, но без установленных Node и `glab`: форма запуска, хранение токенов в системной связке ключей, история запросов и отчёт в окне приложения. Логика загрузки и расчётов — тот же код из `src/`, что у CLI. Интерфейс отчёта — тот же `src/template.html`.

## Границы

Входит:

- форма запуска: хост, токен, ссылка на пайплайн или MR либо агрегат (проект, ref или source, N, статусы);
- хранение токенов по хостам, зашифрованное через `safeStorage`;
- история последних 20 запросов;
- окно отчёта и сохранение HTML;
- сборки для macOS (arm64, x64), Windows (x64), Linux (x64) без подписи;
- GitHub Actions: сборка трёх ОС по тегу `v*` и выкладка в релиз.

Не входит:

- подпись и нотаризация, автообновление;
- мониторинг по расписанию и уведомления;
- хранение отчётов и истории метрик между запусками (кроме истории запросов).

## Структура

```
src/report.mjs            buildReport — общая оркестрация CLI и приложения (новый)
bin/pipeline-trace.mjs    CLI: разбор аргументов → buildReport → файл
desktop/
  package.json            devDependencies: electron, electron-builder; "type": "module"
  scripts/sync-core.mjs   копирует ../src в app/core перед запуском и сборкой
  app/main.mjs            главный процесс
  app/preload.cjs         мост contextBridge: build, save, hosts, setToken, removeToken, history, removeHistory, clearHistory
  app/tokens.mjs          хранилище токенов (шифрование внедряется)
  app/history.mjs         история запросов
  app/request.mjs         разбор и проверка формы → запрос для buildReport
  app/ui/index.html       форма (стиль C, светлая и тёмная тема)
  app/ui/form.js          логика формы
  app/core/               копия src/ (в .gitignore)
  test/*.test.mjs         node:test
.github/workflows/desktop.yml
```

Корневой `package.json` не получает зависимостей: CLI остаётся без них.

## `src/report.mjs`

```js
buildReport(request, { gql, host, now?, onProgress? }) → Promise<{ report, suffix }>
// request: { mode: 'pipeline' | 'mr' | 'aggregate', project, pipelineId?, mrIid?, ref?, source?, statuses?, last? }
// onProgress({ loaded, total })
```

Функция повторяет нынешнюю оркестрацию `bin/pipeline-trace.mjs`: получает id пайплайнов, загружает их, строит деревья, агрегат и `insights`, собирает `report` для `render()`. `suffix` — часть имени файла по умолчанию. CLI вызывает её и пишет файл; поведение CLI не меняется.

## Главный процесс

- Окно формы 1280×860, минимальный размер 900×640.
- `build(request)` по IPC:
  1. `request.mjs` проверяет поля и возвращает запрос или список ошибок по полям на русском;
  2. хост берётся из формы, токен — из хранилища; если его нет — из `glab config get token --host`; если нет и его — ошибка «Нет токена для <host>»;
  3. `buildReport` с `onProgress` → событие в форму «Загружаю <loaded> из <total>»;
  4. `render(report)` → файл во временной папке приложения → окно отчёта (одно на запрос, заголовок — проект и метка).
- Кнопка «Сохранить HTML» в окне отчёта (меню и Cmd/Ctrl+S) — системный диалог сохранения, имя по умолчанию как у CLI.
- Ошибки GitLab (401/403, 404, сеть) приходят в форму текстом из `gitlab.mjs`.

## Токены

- Файл `userData/tokens.json`: `{ [host]: base64(safeStorage.encryptString(token)) }`.
- В окно формы токены не передаются: форма получает только список хостов и признак «токен сохранён».
- Добавление, замена и удаление токена — из формы; поле токена `type="password"`, `autocomplete="off"`.
- Если `safeStorage.isEncryptionAvailable()` ложно (Linux без keyring), сохранение токена запрещено с сообщением «Системная связка ключей недоступна: задайте GITLAB_TOKEN и GITLAB_HOST или установите libsecret». Переменные окружения работают по тем же правилам, что в CLI.

## История

- Файл `userData/history.json`, последние 20 запросов, новые сверху, без токенов: `{ at: ISO-8601, host, form, request, label }`, где `form` содержит только mode, host, url, project, ref, source, last, statuses — токены никогда не сохраняются.
- Клик по записи заполняет форму; повторный одинаковый запрос поднимается наверх.
- У каждой записи — кнопка «×» (`removeHistory`), удаляет только её. Кнопка «Очистить» у заголовка (`clearHistory`) — с подтверждением через `confirm`, скрыта при пустой истории.

## Безопасность

- Все окна: `contextIsolation: true`, `sandbox: true`, `nodeIntegration: false`.
- Форма: CSP `default-src 'self'; style-src 'self' 'unsafe-inline'`.
- Окно отчёта: навигация внутри окна запрещена (`will-navigate` → отмена); `window.open` и внешние ссылки открываются через `shell.openExternal`, только `https:`.
- Мост `preload.cjs` отдаёт только функции `build`, `hosts`, `setToken`, `removeToken`, `history`, `removeHistory`, `clearHistory`, `onProgress`; других каналов нет. Сохранение отчёта — не через мост, а через пункт меню «Файл → Сохранить отчёт…» (Cmd/Ctrl+S).

## Форма (стиль C)

- Слева — история; справа — карточка формы.
- Хост: выпадающий список сохранённых хостов, «+ Добавить хост» раскрывает поля хоста и токена.
- Переключатель режима: «Ссылка» (одно поле для ссылки на пайплайн или MR) и «Агрегат» (проект, ref, source, N, статусы).
- Кнопка «Построить отчёт»; во время загрузки — «Загружаю 4 из 10…» и кнопка неактивна.
- Ошибки — под соответствующим полем; ошибки GitLab — над кнопкой.
- Светлая и тёмная тема по системе, токены цвета те же, что в `template.html`.

## Сборка

- `npm run dev` (в `desktop/`) — `sync-core` + `electron .`.
- `npm run dist` — `sync-core` + `electron-builder` для текущей ОС.
- Цели: macOS `dmg`, `zip` (arm64, x64); Windows `nsis`, `zip` (x64); Linux `AppImage`, `deb` (x64). `CSC_IDENTITY_AUTO_DISCOVERY=false`.
- macOS: подпись ad-hoc, не полноценная. `build.mac.identity: "-"` — electron-builder (26.x) поддерживает это значение напрямую и подписывает сборку ключом `-` через `codesign`. Нотаризации нет: без неё Gatekeeper всё равно требует «Открыть» через контекстное меню при первом запуске.
- `.github/workflows/desktop.yml`: матрица `macos-14`, `windows-latest`, `ubuntu-latest`; `npm test` в корне и в `desktop/`; `npm run dist`; по тегу `v*` артефакты прикладываются к GitHub Release.

## Установка без подписи (README)

- macOS: перетащить в «Программы», первый запуск — «Открыть» из контекстного меню или `xattr -dr com.apple.quarantine "/Applications/pipeline-trace.app"`.
- Windows: SmartScreen → «Подробнее» → «Выполнить в любом случае».
- Linux: `chmod +x pipeline-trace-*.AppImage` или `sudo apt install ./pipeline-trace_*.deb`.

## Тесты

- `test/report.test.mjs` (корень) — `buildReport` на подменённом `gql`: режимы pipeline, mr, aggregate; `onProgress`; ошибка «ни один пайплайн не подошёл».
- `desktop/test/tokens.test.mjs` — хранение, чтение, удаление, отказ без шифрования (шифрование подменяется).
- `desktop/test/history.test.mjs` — лимит 20, порядок, подъём повторного запроса, удаление одной записи и очистка всей истории.
- `desktop/test/request.test.mjs` — разбор формы: ссылки на пайплайн и MR, агрегат, ошибки по полям.
- Главный процесс и окна автотестами не покрываются: ручная проверка сборки на macOS; сборки Windows и Linux — через CI.

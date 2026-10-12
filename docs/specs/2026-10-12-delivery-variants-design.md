# Три варианта поставки Longpole — спека

Дата: 2026-10-12. Добавляет к **Desktop** вариант поставки **Selfhosted** — веб-сервис в контуре организации — и **отчёт по ссылке** в нём. Решения приняты на карте «[Карта: три варианта поставки Longpole](https://github.com/Phobym/longpole/issues/71)»; у каждого раздела указан тикет, где лежат обоснования и отвергнутые варианты. Архитектурная рамка — ADR [0001](../adr/0001-core-and-two-shells.md) и [0002](../adr/0002-stateless-selfhosted.md). Словарь — `CONTEXT.md`.

Базовые спеки, от которых здесь описаны только отличия: `2026-10-07-github-source-design.md` (хосты, токены, ссылки), `2026-10-08-onboarding-design.md` и `2026-10-11-oauth-welcome-design.md` (приветствие, аккаунты), `2026-10-09-oauth-design.md` (вход в Desktop).

Исследования (ветки `research/*`, не мержатся):

- [Привязка к десктопу](https://github.com/Phobym/longpole/blob/research/desktop-coupling/docs/research/2026-10-12-desktop-coupling.md) — 22 команды Tauri и места, которые не переносятся.
- [OAuth на сервере](https://github.com/Phobym/longpole/blob/research/server-oauth/docs/research/2026-10-12-server-oauth.md) — GitLab self-managed и GHES.
- [Ссылка в пайплайне](https://github.com/Phobym/longpole/blob/research/link-in-pipeline/docs/research/2026-10-12-link-in-pipeline.md) — механизмы GitLab и GitHub.

## 1. Цель и рамки

Команда, у которой GitLab self-managed (пилот — GitLab Литрес) или GHES, разворачивает один инстанс Longpole. Пользователи входят через OAuth своего хоста и смотрят отчёты в браузере, ничего не устанавливая. Из пайплайна ведёт ссылка «Посмотреть в Longpole», которая сразу открывает отчёт.

Делается:

- **Selfhosted** — веб-сервис из того же репозитория и той же версии, что Desktop. Лицензия MIT, платной редакции нет.
- Хосты и OAuth-приложения задаёт **администратор** в **конфигурации инстанса**. Пользователь их не меняет.
- Каждый пользователь входит через OAuth web flow (confidential-приложение), сервер ходит в API от его имени. Общего сервисного токена нет.
- Сервер без состояния: сессия в зашифрованной cookie, кэш отчётов в памяти, базы нет. «Мои проекты», «Недавние» и настройки хранятся в браузере.
- Объём веб-версии: форма, экран проекта, агрегат, отчёт по ссылке. Онбординг адаптирован под хосты инстанса.
- Код: одно ядро `longpole-core`, две оболочки (`src-tauri` и `server`), один фронт с подменяемым транспортом и флагом поставки.
- Desktop по функциям не меняется.

Не делается (раздел «Out of scope» карты):

- Helm-чарт и поставка без контейнера; публикация образа (GHCR) и проверка Dockerfile в CI.
- Несколько реплик. Single-flight обновления токена и кэш держатся в памяти процесса.
- Деплой под подпутём (`https://example.com/longpole/`).
- Серверная БД: пользователи, общие проекты, история.
- Ссылка без правки CI через вебхук и commit status (нужен сервисный токен).
- Предрасчёт отчётов в CI, deep links в Desktop, Desktop с вшитым корпоративным хостом.
- Метрики Prometheus.

## 2. Раскладка кода

ADR 0001. Тикет: «[Что в longpole-core и во фронте завязано на десктоп](https://github.com/Phobym/longpole/issues/73)».

```
app/
├── core/        longpole-core: сборка отчёта; десктопные модули за фичей `desktop`
├── src-tauri/   оболочка Desktop (как сейчас)
├── server/      оболочка Selfhosted, новый крейт `longpole-server` (axum)
└── src/         один фронт; транспорт и возможности выбираются флагом DELIVERY
```

`server` добавляется в `[workspace].members`. Оболочки только переводят свой транспорт в вызовы ядра и держат состояние процесса. Логики отчёта в них нет.

## 3. Ядро

Тикет: «[Что в longpole-core и во фронте завязано на десктоп](https://github.com/Phobym/longpole/issues/73)».

**Переносится как есть:** клиенты GitLab/GitHub (`gitlab`, `github`, `source`), `request`, `report`, `aggregate`, `insights`, `critical_path`, `model`, `schema`, `render`, `browse`, `error`.

**Только Desktop** — за cargo-фичей `desktop` (включена по умолчанию, `src-tauri` её использует, `server` собирает ядро с `default-features = false`):

- `tokens` — связка ключей, `glab`/`gh`, env-токены (`GITLAB_TOKEN`, `GH_TOKEN`);
- loopback и Device Flow в `oauth`, вместе с `oauth_page.html`;
- файловые хранилища `history`, `projects`, `settings`, `hosts` и `json_file`;
- зависимости `keyring-core` и `*-keyring-store`, включая `dbus-secret-service-keyring-store` на Linux. В серверном бинарнике их быть не должно.

**Нужно ядру для сервера:**

- OAuth web flow с секретом: построение URL авторизации (GitLab — с PKCE, GHES — без), обмен кода и refresh. Функции без состояния, транспорт — `reqwest` ядра. Это отдельный модуль, у которого нет общего кода с loopback-веткой `oauth`.
- Клиент хоста из пары (хост, токен) без поиска токена. `source_for` с цепочкой «хранилище → CLI → env» остаётся в `src-tauri`.
- Пагинация (`Link`/`next`) идёт только в пределах того же origin. Закрепляется тестом ядра (§6, защита от SSRF).

Поведение Desktop на этом этапе не меняется.

## 4. Конфигурация инстанса

Тикеты: «[Конфигурация инстанса при деплое](https://github.com/Phobym/longpole/issues/79)», лимиты — «[Защита сервера: заголовки и лимиты построения отчётов](https://github.com/Phobym/longpole/issues/81)», `listen` и `log` — «[Docker-образ в релизе, healthcheck, логи и метрики](https://github.com/Phobym/longpole/issues/82)».

Один YAML-файл по фиксированному пути `/etc/longpole/config.yaml` (парсер `serde_yaml_ng`). Переопределений через env и своего пути нет. Во всём файле работает подстановка `${ИМЯ}` из окружения. Незаданная или пустая переменная — ошибка конфигурации. Синтаксиса `$$` и `${X:-y}` нет.

```yaml
url: https://longpole.litres.ru
cookie_key: ${LONGPOLE_COOKIE_KEY}
listen: 0.0.0.0:8080          # необязательно
cache_max_mb: 256             # необязательно
max_concurrent_builds: 8      # необязательно
build_timeout: 5m             # необязательно
log:
  level: info                 # необязательно
  format: text                # text | json, необязательно
hosts:
  - domain: gitlab.litres.ru
    provider: gitlab
    client_id: 3f9a…
    client_secret: ${GITLAB_CLIENT_SECRET}
```

| Поле | Правило |
| --- | --- |
| `url` | Обязательно. Публичный URL инстанса, он же `LONGPOLE_URL` в CI. Только `https`, исключение — `http://localhost`. Только корень домена. |
| `cookie_key` | Обязательно. 32 байта в base64 (`openssl rand -base64 32`). Ротации нет: смена ключа означает тихий повторный вход для всех. |
| `listen` | По умолчанию `0.0.0.0:8080`. TLS завершает reverse proxy или ingress. |
| `cache_max_mb` | Лимит кэша отчётов, по умолчанию 256. |
| `max_concurrent_builds` | Построений на реплику, по умолчанию 8. |
| `build_timeout` | По умолчанию 5 минут, время в очереди не входит. |
| `log.level`, `log.format` | `info` и `text` по умолчанию. `RUST_LOG` не читается. |
| `hosts[].domain` | Без схемы и пути, порт допустим, схема всегда `https`. Уникален: **ключ хоста = домен**. |
| `hosts[].provider` | `gitlab` или `github`, задаётся явно. |
| `hosts[].client_id`, `client_secret` | GitLab — instance-wide приложение. GHES — Client ID и secret GitHub App (App ID не нужен). |

Redirect URI выводится, а не задаётся: `<url>/auth/callback/<domain>`. Администратор копирует его при регистрации приложения.

Прокси и CA — стандартные env контейнера (`HTTPS_PROXY`, `NO_PROXY`, `SSL_CERT_FILE`). `SSL_CERT_FILE` работает (проверено на этапе 3): на Linux фича `rustls` у `reqwest` 0.13 берёт корни через `rustls-platform-verifier` 0.7 → `rustls-native-certs` 0.8, а тот читает `SSL_CERT_FILE` и `SSL_CERT_DIR`. Заданная переменная **заменяет** системное хранилище, а не дополняет его: файл должен содержать и корпоративный CA, и публичные корни, если хосты за ними. Язык интерфейса задаётся браузером и выбором пользователя, в конфиге его нет.

**Проверка при старте** — только статическая: схема, обязательные и неизвестные поля, формат `cookie_key`, уникальность доменов, `https`, незаданные `${…}`. Выводятся все ошибки сразу, затем выход с ненулевым кодом. Частичного запуска нет. Хосты по сети при старте не проверяются. Конфиг читается один раз, изменения применяются рестартом.

## 5. Вход и сессия

Тикеты: «[OAuth web flow для серверного приложения на GitLab self-managed и GHES](https://github.com/Phobym/longpole/issues/74)», «[API сервера и модель сессии](https://github.com/Phobym/longpole/issues/76)».

**Приложения у провайдеров:**

- GitLab — instance-wide confidential-приложение, Authorization Code + секрет + PKCE, scope `read_api`. Access живёт 2 ч, refresh одноразовый (повтор даёт `invalid_grant`).
- GHES — GitHub App с истекающими токенами (Actions, Contents, Metadata: read), web flow с секретом без PKCE. Access живёт 8 ч, refresh — 6 месяцев. На пользователя не больше 10 токенов: 11-й вход выбивает старейшую сессию, это принимаем.

**Сессия** — cookie `longpole_session`, зашифрованная AEAD-ключом `cookie_key` (например, `PrivateCookieJar` из `axum-extra`). По каждому хосту в ней лежат access, refresh, срок access и id пользователя у провайдера. Атрибуты `HttpOnly; Secure; SameSite=Lax; Path=/`. `Lax` нужен потому, что переход по «Посмотреть в Longpole» — межсайтовая GET-навигация. `Max-Age` 30 дней, скользящий: продлевается при обновлении токена.

**Вход:** `GET /auth/login?host=<domain>&return_to=<path>` отвечает 302 к провайдеру. `state`, PKCE verifier и `return_to` лежат в короткой зашифрованной cookie `longpole_oauth` (10 мин, `Path=/auth`). `GET /auth/callback/<domain>` сверяет `state`, меняет код на токены, пишет хост в сессию, удаляет `longpole_oauth` и отвечает 302 на `return_to`. `return_to` — только относительный путь (`/…`, но не `//…`), иначе это open redirect.

**Обновление токена** — лениво, на любом запросе, если до истечения access меньше 5 минут. Новая cookie уходит в том же ответе. От гонки одноразового refresh защищает in-memory single-flight по refresh-токену, результат удерживается около 60 с. Отсюда требование одной реплики.

**Истёкший или отозванный вход:** 401 от провайдера или `invalid_grant` — сервер убирает хост из cookie и отвечает `CmdError { code: 'authRequired', host }`. Фронт вызывает `login(host)` с `return_to` = текущий URL. Повторный вход у confidential-приложения тихий.

**Выход:** `POST /auth/logout?host=<domain>` убирает один хост из cookie. Отзыв токена у провайдера делается best effort и только для GitLab (`/oauth/revoke`).

**CSRF** — без токенов: `SameSite=Lax`, все вызовы API только `POST` с `Content-Type: application/json`, `Origin` сверяется с `url` из конфига.

## 6. API сервера

Тикет: «[API сервера и модель сессии](https://github.com/Phobym/longpole/issues/76)».

**Стиль** — RPC один к одному с командами Tauri: `POST /api/<command>`, в JSON-теле те же аргументы, в ответе тот же сериализованный `Cmd<T>`. Типы ts-rs переиспользуются.

**Операции сервера:** `hosts`, `hostProvider`, `projects`, `branches`, `pipelines`, `workflows`, `build`, `addProject` (только разбирает ввод и отдаёт `SavedProject`, список хранит браузер). `hosts` отдаёт хосты конфига и флаг «есть сессия» по каждому, вместе с логином у провайдера. `hostProvider` берёт провайдера из конфига без пробы. `report` и `findReport` на сервер не ходят (§9).

**Прогресс `build`** — NDJSON в ответе на `POST /api/build`: строки `{"progress":{loaded,total}}` и `{"queued":true}` (§11), последняя строка — `{"ok":{"report":<Report>,"entry":<HistoryEntry>,"fileName":<имя файла, как у ⌘S в Desktop>}}` или `{"err":…}`. Запись истории (`at`, ссылка без query, `request`, `label`) считает ядро, браузер кладёт её в «Недавние». Остальные операции отвечают одной строкой `{"ok":…}` или `{"err":…}`. Фронт читает поток через `fetch` и `ReadableStream`. Серверного id отчёта нет. Аргумент `fresh: true` — это «Перестроить» (§7).

**Ограничение хостами (SSRF):** `host` во всех операциях — домен из конфига, не URL. Неизвестный домен даёт `CmdError { code: 'unknownHost' }`. В режиме формы «ссылка» origin URL должен совпасть с хостом конфига. Исходящий URL из пользовательского ввода не строится.

**Прочие маршруты:** `/auth/*` (§5), `GET /healthz` (§13), статика фронта из бинарника. На любой другой `GET` сервер отдаёт `index.html` (§8).

## 7. Кэш отчётов

Тикет: «[Кэш отчётов: ключ, права и инвалидация](https://github.com/Phobym/longpole/issues/77)».

- **Что хранится:** JSON модели `Report` по одному пайплайну. Запрос `mr` сначала разрешается в id пайплайна и дальше идёт через тот же кэш. Агрегат не кэшируется.
- **Ключ:** `(домен хоста, числовой id проекта, id пайплайна)`. Путь проекта нормализуется в id (GitLab `project.id`, GitHub `repository.id`).
- **Права:** при каждой выдаче сервер делает лёгкий запрос от имени пользователя — GitLab `GET /projects/:id/pipelines/:pid`, GitHub `GET /repos/:o/:r/actions/runs/:id`. Ответы 403 и 404 возвращаются как обычный «не найден».
- **Незавершённые пайплайны** не кэшируются, пока корень или любой downstream не в терминальном статусе.
- **Ретрай после завершения:** в записи хранится `updated_at` (у GitHub ещё `run_attempt`) из проверки прав. Если значение не совпало, отчёт строится заново. Ретрай внутри downstream в GitLab может не поменять `updated_at` родителя — этот случай закрывает «Перестроить».
- **Ручной сброс:** пользователь нажимает «Перестроить» (`build` с `fresh`), администратор перезапускает контейнер. Админского эндпоинта нет.
- **Версия Longpole** в ключе не нужна: новый процесс стартует с пустым кэшем.
- **Размер:** LRU по суммарному размеру JSON, лимит `cache_max_mb`. TTL нет.
- **Где:** в памяти процесса. Одновременная сборка одного ключа выполняется один раз.
- **Язык:** при выдаче сервер подставляет `meta.locale` под язык запроса, `generated_at` остаётся исходным.

## 8. Отчёт по ссылке

Тикет: «[Формат ссылки на отчёт и способ её доставки в пайплайн](https://github.com/Phobym/longpole/issues/78)».

**Форма URL** — путь провайдера, в котором домен провайдера заменён на домен Longpole:

| Что | URL | Режим формы |
| --- | --- | --- |
| Пайплайн GitLab | `<url>/<domain>/<project-path>/-/pipelines/<id>` | пайплайн |
| Run GitHub | `<url>/<domain>/<owner>/<repo>/actions/runs/<id>` | пайплайн |
| MR GitLab | `<url>/<domain>/<project-path>/-/merge_requests/<iid>` | `mr` |
| PR GitHub | `<url>/<domain>/<owner>/<repo>/pull/<n>` | `mr` |
| Агрегат | URL экрана проекта `/project/<domain>/<path>?ref=…&workflow=…` | агрегат |

Хост определяется по домену из ссылки, неизвестный домен даёт `unknownHost`. Вместо пути проекта принимается и числовой id: ядро переводит его в путь (GitLab — GraphQL `projects(ids:)`, GitHub — `GET /repositories/:id`). Отчёт и подпись записи истории получают путь, а запрос в истории остаётся по id. «Добавить проект» id не принимает: «Мои проекты» хранят путь.

**Старый путь после переименования не открывается** (проверено 2026-10-12 на gitlab.com, `gitlab-org/gitlab-ce` → `gitlab-org/gitlab-foss`): веб и REST отвечают 301 на новый путь, но GraphQL `project(fullPath:)`, через который ядро читает пайплайны, по старому пути отдаёт `null` — пользователь видит «проект не найден». Поэтому ссылка из CI (§12) строится по числовому id: `$CI_PROJECT_ID` в GitLab, `$GITHUB_REPOSITORY_ID` в GitHub (клиент GitHub в ядре тоже не следует редиректам).

Если вход в хост ссылки не удался сразу после попытки (отказ у провайдера или 401 на свежий токен), второй тихий вход не делается — иначе цикл: SPA показывает экран входа.

**Адресная строка:** URL ссылки и есть страница отчёта. Он не подменяется на `/report/<локальный id>`, «Перестроить» оставляет его на месте.

**Роутинг:** в Selfhosted — browser history (`createBrowserHistory` TanStack Router, выбор по `DELIVERY`), в Desktop — hash, как сейчас (`app/src/app/form/router.tsx`). Сервер отдаёт `index.html` на любой неизвестный `GET`, кроме `/api`, `/auth`, `/healthz` и статики. Конфликта с отчётами нет: в домене всегда есть точка.

**Без входа в хост ссылки:** SPA сразу уходит на `login(host)` с возвратом на ту же ссылку. Сервер при этом не редиректит. Если OAuth вернул ошибку или отказ, показывается экран «Войдите в <домен>, чтобы увидеть отчёт» с кнопкой.

## 9. Фронт: транспорт и возможности поставки

Тикет: «[Граница транспорта и флаг поставки во фронте](https://github.com/Phobym/longpole/issues/75)».

**Граница транспорта** — публичные операции `shared/api`. У каждой две реализации с общими сигнатурами и `Result<T>`: Desktop через `invoke`, Selfhosted через `fetch` или браузер. `call(cmd)` остаётся деталью Desktop-реализации. У `shared/lib/dialogs` и `pickFolder` тоже две реализации.

| Группа | Операции | Selfhosted |
| --- | --- | --- |
| Удалённые | `hosts`, `hostProvider`, `projects`, `branches`, `pipelines`, `workflows`, `build`, `report`, `findReport` | `POST /api/*`. `build` держит отчёт в памяти вкладки и выдаёт локальный id. `report` и `findReport` читают только память вкладки, а `findReport` при промахе возвращает `null`, после чего повтор идёт через `build` и кэш |
| Локальные | `history`, `removeHistory`, `clearHistory`, `savedProjects`, `addProject`, `removeProject`, `getSettings`, `setSettings` | хранилище браузера, без сервера |
| Только Desktop | `setToken`, `removeToken`, `oauthLogin`, `oauthCancel`, `setCurrentReport` | нет; вместо входа — `login(host)` с редиректом на `/auth/login` и `logout(host)` |

**Ошибка транспорта:** `IpcError { code: 'ipc' }` переименовывается в `code: 'transport'`, одна ошибка на оба транспорта.

**Флаг поставки** задаётся при сборке: `VITE_DELIVERY` → `DELIVERY: 'desktop' | 'selfhosted'`, по умолчанию `desktop`. Реализация выбирается статическим импортом, мёртвая ветка вырезается, `@tauri-apps/*` не попадает в веб-бандл (проверка в тесте сборки).

**Возможности поставки** — константа `shared/config/capabilities.ts`, вычисленная из `DELIVERY`. В Selfhosted все `false`:

- `manageHosts` — свои хосты и токены;
- `addByFolder` — добавление проекта по папке;
- `nativeSave` — ⌘S через системный диалог;
- `oauthLoopback` — вход без ухода со страницы;
- `hashRouting` — маршруты в hash, без отчёта по ссылке (§ 8).

`DELIVERY` читают только `shared/api`, `shared/lib/dialogs` и `capabilities.ts`. Остальной код импортирует только `capabilities`, это закреплено линтером (steiger или `no-restricted-imports`).

**Dev:** мок моста `app/dev/mock.ts` работает только для Desktop. Selfhosted в dev ходит на настоящий сервер через proxy Vite.

## 10. Веб-интерфейс

Тикет: «[Веб-версия на месте функций Desktop](https://github.com/Phobym/longpole/issues/80)».

1. **Сохранить отчёт:** в шапке отчёта кнопка «Скачать HTML». Файл тот же самодостаточный и с тем же именем по умолчанию, что в Desktop, скачивается через Blob, HTML рендерится во фронте: шаблон лежит в статике текстом (`/report-template.txt`), имя приходит в ответе `build`. ⌘S не перехватывается.
2. **Добавление проекта:** тот же диалог «ссылка или поиск», без «Выбрать папку…».
3. **Настройки → Хосты:** хосты инстанса только для чтения, у каждого «Вошли как @login · Выйти» или «Войти». Нет «Добавить хост», ввода токена, источников токена, подсказок Linux и tips.
4. **Сайдбар «Аккаунты»:** хосты инстанса, у хоста без входа ссылка «Войти». «+ Подключить» скрыт.
5. **Приветствие, один хост:** одна кнопка «Войти через <провайдер> (<домен>)» → OAuth переходом всей страницы → сразу выбор проекта. Без `OauthWaiting` и без «Пропустить». Автоматического редиректа с `/` нет.
6. **Приветствие, несколько хостов:** по кнопке на каждый хост. Вход в любой ведёт к выбору проекта на нём, входить во все не обязательно.
7. **Отчёт по ссылке без входа** — §8.
8. **Выход:** «Выйти» у хоста стирает вход только в него. «Выйти из всего» нет.
9. **Перестроить:** кнопка в шапке отчёта по завершённому пайплайну вызывает `build` с `fresh`.

## 11. Защита и лимиты

Тикет: «[Защита сервера: заголовки и лимиты построения отчётов](https://github.com/Phobym/longpole/issues/81)».

**Заголовки** ставит сам сервер на каждый ответ:

- `Content-Security-Policy: default-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'`. Если UI показывает аватары с хоста, к `img-src` добавляются домены хостов из конфига.
- `X-Content-Type-Options: nosniff`.
- `Referrer-Policy: same-origin`.
- `Strict-Transport-Security: max-age=31536000` — только если `url` начинается с `https://`.

SPA работает без инлайн-скриптов. Самодостаточный `report.html` никогда не открывается на origin сервера: он собирается в браузере и только скачивается.

**Построения:**

- На реплику не больше `max_concurrent_builds` одновременно, лишние ждут в очереди. В потоке прогресса приходит `{"queued":true}`, UI показывает «В очереди…».
- На пользователя (сессию на хосте) не больше 2 одновременно. Третье построение получает `tooManyBuilds` («Дождитесь построения других отчётов»).
- Запросы с одинаковым ключом кэша объединяются (§7).
- Агрегат занимает один слот, запросы внутри него ограничивает семафор ядра (4).
- `build_timeout` отменяет построение, в кэш ничего не попадает, пользователь видит `buildTimeout` и «Повторить».

**Размер пайплайна** ограничивают лимиты ядра: 5000 джобов, глубина downstream 3, 10 страниц списков. При превышении ядро отвечает ошибкой `TooManyJobs`. Rate limit на `/api/*` и `/auth/*` — задача прокси или WAF. Лимит тела запроса — значение axum по умолчанию (2 МБ).

## 12. Ссылка в CI

Тикеты: «[Способы встроить ссылку «Посмотреть в Longpole» в GitLab и GitHub](https://github.com/Phobym/longpole/issues/72)», «[Формат ссылки на отчёт и способ её доставки в пайплайн](https://github.com/Phobym/longpole/issues/78)».

**GitLab.** Администратор заводит instance-level CI/CD-переменную `LONGPOLE_URL` и кладёт шаблон в общий репозиторий. Проект добавляет одну строку `include:`. В репозитории Longpole шаблон лежит в `ci/gitlab/longpole.yml`:

```yaml
longpole:
  stage: .pre
  image: alpine
  script:
    - echo "Посмотреть в Longpole: $LONGPOLE_URL/$CI_SERVER_HOST/$CI_PROJECT_ID/-/pipelines/$CI_PIPELINE_ID"
  environment:
    name: longpole
    url: $LONGPOLE_URL/$CI_SERVER_HOST/$CI_PROJECT_ID/-/pipelines/$CI_PIPELINE_ID
    action: access
```

`environment:url` даёт кнопку в виджете MR. Минимальный вариант без отдельного job — та же строка `echo` в любом существующем job.

**GitHub/GHES.** Переменная организации `vars.LONGPOLE_URL` и одна строка в любом job:

```yaml
- run: echo "[Посмотреть в Longpole](${{ vars.LONGPOLE_URL }}/${GITHUB_SERVER_URL#https://}/$GITHUB_REPOSITORY_ID/actions/runs/$GITHUB_RUN_ID)" >> "$GITHUB_STEP_SUMMARY"
```

Reusable action не делается.

**Документация** — раздел «Selfhosted» в README: деплой, регистрация OAuth-приложений, подключение CI. Подсказки в UI нет.

## 13. Эксплуатация

Тикет: «[Docker-образ в релизе, healthcheck, логи и метрики](https://github.com/Phobym/longpole/issues/82)».

**Dockerfile** в корне репозитория, многостадийный:

1. `node` — `npm ci` и сборка фронта с `VITE_DELIVERY=selfhosted`;
2. `rust` — статический бинарник `longpole-server` под musl со встроенной статикой фронта (например, `rust-embed`) и шаблоном самодостаточного HTML;
3. `gcr.io/distroless/static:nonroot` с единственным файлом `/longpole`, `EXPOSE 8080`.

`HEALTHCHECK` в Dockerfile нет: в distroless нет curl, проверку делает оркестратор. Образ не публикуется: администратор собирает его сам из тега `vX.Y.Z`. `release.yml` не меняется.

**Healthcheck:** `GET /healthz` отвечает `200 ok` без сессии и без обращений к хостам. Отдельного readiness нет.

**Логи:** `tracing` в stdout, уровень и формат задаются в §4. На `info` пишутся:

- при старте — версия, `url`, домены хостов;
- на каждый запрос — метод, команда или путь без query, статус, длительность;
- на каждое построение — домен, id проекта, id пайплайна, попадание в кэш, длительность;
- ошибки провайдера — код ответа.

Логин пользователя не пишется.

**Свой CA:** образ не меняется — администратор монтирует PEM-файл и задаёт `SSL_CERT_FILE` (§4). В файле должны быть все нужные корни: системное хранилище образа при этом не читается.

**Никогда не в логах:** access- и refresh-токены, значения cookie, `Authorization`, OAuth `code`/`state`, `client_secret`, `cookie_key`, query у `/auth/callback`. Секреты хранятся в типе-обёртке, у которого `Debug` и `Display` печатают `[redacted]`. URL логируются без query.

## 14. Ошибки

Новые коды `ErrorCode` (ts-rs) и что видит пользователь:

| Код | Когда | UI |
| --- | --- | --- |
| `authRequired` (+ `host`) | нет входа, 401 провайдера, `invalid_grant` | тихий `login(host)` с возвратом на текущий URL; при отказе — экран «Войдите в <домен>» |
| `unknownHost` | домен не из конфига | «Хост <домен> не подключён к этому Longpole» |
| `tooManyBuilds` | третье одновременное построение пользователя | «Дождитесь построения других отчётов» |
| `buildTimeout` | истёк `build_timeout` | ошибка с кнопкой «Повторить» |
| `transport` (было `ipc`) | сеть или мост недоступны | как сейчас `ipc` |

Отказ в правах на пайплайн (403/404 при проверке прав) — существующий «не найден». Ошибки конфигурации при старте в `ErrorCode` не входят: это вывод в stderr и выход с ненулевым кодом.

## 15. Этапы

Каждый этап — отдельный запуск `writing-plans` и отдельный PR. После каждого PR main зелёный, а Desktop работает как раньше. Этапы 1 и 2 независимы, 3 идёт после обоих, 4–7 — после 3.

1. **Ядро** (§3) — фича `desktop`, OAuth web flow, клиент из (хост, токен), тест пагинации на том же origin. Ядро собирается без `desktop`, `src-tauri` не меняется по поведению.
2. **Фронт** (§9) — граница `shared/api` и `dialogs` с двумя реализациями, `DELIVERY`, `capabilities`, правило линтера, `ipc` → `transport`. Selfhosted-реализация `fetch` готова, но работает против мока. Desktop-сборка идентична.
3. **Сервер — основа** (§4, §5, §6, заголовки из §11, `/healthz` и логи из §13) — крейт `server`, конфиг и его проверка, вход и сессия, RPC с NDJSON, статика и fallback на `index.html`. Пользователь входит и строит отчёт через форму.
4. **Сервер — кэш и лимиты** (§7, построения из §11) — кэш, `fresh`, очередь, лимит на пользователя, объединение запросов, таймаут.
5. **Веб-интерфейс и отчёт по ссылке** (§8, §10) — browser history, разбор URL провайдера, экраны из §10, «Скачать HTML», «Перестроить».
6. **Ссылка в CI** (§12) — шаблон GitLab, сниппет GitHub, раздел README.
7. **Dockerfile** (§13) — многостадийная сборка, инструкция администратора в README.

## 16. Тесты

- **Ядро:** сборка без `desktop` (`cargo check -p longpole-core --no-default-features`); пагинация не уходит на чужой origin; построение URL авторизации и обмен кода на фикстурах для GitLab (с PKCE) и GHES (без).
- **Сервер — конфиг:** подстановка `${…}`, все ошибки сразу, неизвестные поля, дубли доменов, `http` кроме localhost.
- **Сервер — сессия:** шифрование и расшифровка cookie; `return_to` отвергает `//…` и абсолютные URL; single-flight refresh — два параллельных запроса делают один refresh; `invalid_grant` → `authRequired` и хост удалён из cookie; `Origin` чужого сайта отклоняется.
- **Сервер — API:** `unknownHost`; NDJSON заканчивается `ok` или `err`; заголовки безопасности на ответах; fallback на `index.html` не задевает `/api` и `/auth`.
- **Сервер — кэш:** промах → построение; попадание выдаётся только после проверки прав; 403/404 → «не найден»; смена `updated_at` → перестроение; незавершённый пайплайн не кэшируется; вытеснение LRU по размеру; объединение одновременных запросов.
- **Сервер — лимиты:** третье построение пользователя → `tooManyBuilds`; очередь сверх `max_concurrent_builds`; таймаут → `buildTimeout`, в кэше пусто.
- **Логи:** секреты печатаются как `[redacted]`, URL callback без query.
- **Фронт:** в Selfhosted-бандле нет `@tauri-apps`; разбор четырёх форм URL из §8; экраны §10 при всех возможностях `false`; локальные операции пишут в хранилище браузера.

## 17. Зависимости и внешние шаги

**Новые зависимости.** Крейт `server`: `axum`, `axum-extra` (cookie), `tower-http`, `tracing`, `tracing-subscriber` (`json`), `rust-embed` или аналог, `tokio` с `rt-multi-thread` и `signal`. Ядро — только существующие.

**Администратор при деплое:**

1. Регистрирует приложения. GitLab: Admin → Applications, confidential, scope `read_api`, redirect `<url>/auth/callback/<domain>`. GHES: GitHub App с истекающими токенами, Actions/Contents/Metadata: read, тот же callback.
2. Генерирует `cookie_key`, пишет `config.yaml`, передаёт секреты через env.
3. Собирает образ из тега и ставит его за reverse proxy с TLS.
4. Заводит `LONGPOLE_URL` (GitLab — instance-level, GitHub — переменная организации) и публикует шаблон `include:`.

**Проверить при реализации:** ~~`SSL_CERT_FILE` с `reqwest` 0.13 + `rustls` (этап 3)~~ — работает, см. §4; редирект старого пути проекта в GitLab после переименования (этап 5).

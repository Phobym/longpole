# Вход через OAuth — спека

Дата: 2026-10-09. Добавляет вход через OAuth рядом с Personal Access Token для github.com, gitlab.com, GHES и GitLab self-managed. Базовые спеки: `docs/specs/2026-10-07-github-source-design.md` (токены GitHub), `docs/specs/2026-10-06-first-run-ux-design.md` (где запрашивается токен). Факты о провайдерах с источниками — `docs/research/2026-10-09-oauth-desktop.md`. Здесь только отличия.

## 1. Цель и рамки

Пользователь получает токен, не создавая PAT вручную: нажимает «Войти через GitHub/GitLab» и подтверждает вход в браузере.

Делается:

- GitHub и GHES — Device Flow через GitHub App с правами Actions: read, Contents: read, Metadata: read.
- GitLab и self-managed — Authorization Code + PKCE через loopback `http://127.0.0.1:<порт>/callback`, non-confidential приложение со scope `read_api`.
- `client_secret` нигде не используется. client_id для github.com и gitlab.com — константы в коде; для self-hosted его вводит пользователь.
- OAuth и PAT равноправны в интерфейсе. Цепочка поиска токена (связка ключей → `glab`/`gh` → env) не меняется.
- Автоматическое обновление access token по refresh token.

Не делается: отзыв токена на сервере при удалении, автоматическое определение client_id, OAuth App на GitHub (нужен scope `repo` с правом записи), Device Flow на GitLab (с 18.11 выключен по умолчанию у новых приложений), PKCE на GitHub (обмен кода требует секрет, на GHES PKCE нет).

## 2. Хранение

Запись в связке ключей по-прежнему одна на хост (`SERVICE`, account — хост). Значение — одно из двух:

- строка — PAT, как сейчас; старые записи читаются без миграции;
- JSON `{"kind":"oauth","access":…,"refresh":…,"expires_at":<unix-секунды>,"client_id":…}`.

Признак OAuth — значение разбирается как JSON с `kind == "oauth"`. PAT JSON-объектом не бывает. `hosts.json` не меняется.

## 3. Получение токена

`TokenStore::get(host)` остаётся единственной точкой чтения:

1. Запись — PAT: вернуть строку.
2. Запись OAuth и до `expires_at` больше 5 минут: вернуть `access`.
3. Иначе — обновить под мьютексом этого хоста. Внутри мьютекса запись читается заново: если её уже обновил другой поток, вернуть свежий `access` без запроса. Новая пара записывается в связку ключей до возврата.

Мьютекс обязателен: GitLab при refresh отзывает старую пару, а параллельные команды (`projects`, `branches`, `pipelines`) иначе выбьют друг друга.

Ошибки обновления:

- GitLab `invalid_grant`, GitHub `bad_refresh_token` (приходит с HTTP 200 в поле `error`; код сверить с docs при реализации) или 401 — refresh token отозван или истёк (GitHub — 6 месяцев). Запись удаляется вместе с хостом в `hosts.json`, `get` возвращает `None`, цепочка переходит к `glab`/`gh` и env, а в конце — `NoToken` с блоком входа.
- Сетевой сбой или 5xx — запись не трогается, ошибка `network` / `httpStatus` идёт наружу.

`find_token`, `find_github_token`, `source_for`, `gitlab::Client`, `github::Client` не меняются.

## 4. Модуль `core/src/oauth.rs`

Функции без состояния на `reqwest`. Базовый URL эндпоинтов — параметр, как `with_api_url` у клиентов, чтобы тесты подставляли wiremock.

- `github_device_start(base, client_id) -> DeviceCode { device_code, user_code, verification_uri, interval, expires_in }` — `POST /login/device/code`.
- `github_device_poll(base, client_id, &DeviceCode, cancel) -> Credential` — опрос `POST /login/oauth/access_token` с `grant_type=urn:ietf:params:oauth:grant-type:device_code`. `authorization_pending` — ждать `interval`; `slow_down` — интервал +5 с; `access_denied` → `OauthDenied`; `expired_token` → `OauthExpired`.
- `gitlab_pkce(base, client_id, open, cancel) -> Credential` — 32 случайных байта verifier и 16 байт state (`getrandom`), challenge S256 (`sha2` + `base64` URL-safe без паддинга). Слушатель `TcpListener` на `127.0.0.1:0`; `open` получает `/oauth/authorize?client_id&redirect_uri&response_type=code&state&scope=read_api&code_challenge&code_challenge_method=S256`. Запрос с чужим `state` получает 400, слушатель ждёт дальше. Запрос с верным `state` получает HTML «Можно вернуться в Longpole», слушатель закрывается, затем `POST /oauth/token` с `grant_type=authorization_code`, `code`, `redirect_uri`, `code_verifier`, `client_id`.
- `refresh(provider, base, &Credential) -> Credential` — GitHub: `POST /login/oauth/access_token` с `client_id`, `grant_type=refresh_token`, `refresh_token`; GitLab: `POST /oauth/token` с тем же и `redirect_uri`.

GitHub base — `https://<host>` (для GHES тот же хост). `expires_at` = сейчас + `expires_in`.

`cancel` — флаг отмены, проверяется между опросами и при ожидании слушателя. Общий таймаут любого flow — 10 минут.

## 5. Команды Tauri

- `oauth_start(host, client_id: Option<String>) -> OauthStart`. client_id: аргумент или константа для github.com/gitlab.com; для прочих хостов без аргумента — `OauthFailed` с `detail`. Пустая строка или строка с пробелами — `OauthFailed`. GitHub: возвращает `{ kind: "device", user_code, verification_uri }`. GitLab: запускает `gitlab_pkce` с `open` через `tauri-plugin-opener` и возвращает `{ kind: "browser" }`.
- `oauth_finish(host) -> ()` — ждёт завершения начатого flow и записывает `Credential` в связку ключей (`TokenStore::set_oauth`), добавляет хост в `hosts.json`.
- `oauth_cancel(host)` — выставляет `cancel`.

Начатый flow хранится в `AppState` по хосту; повторный `oauth_start` отменяет предыдущий.

`TokenSource` получает вариант `Oauth`; `list_hosts` отдаёт его для OAuth-записей.

## 6. Интерфейс

`TokenBlock` (под полем ссылки) и `TokenForm` (диалог хоста) показывают два равноправных пути: кнопку «Войти через GitHub» / «Войти через GitLab» и нынешнюю форму PAT.

- Self-hosted: над кнопкой входа — поле «Client ID» и ссылка на регистрацию приложения: GitLab — `https://<host>/-/user_settings/applications`, GHES — `https://<host>/settings/apps/new`. Для github.com и gitlab.com поля нет.
- GitHub: после нажатия — крупный `user_code`, кнопка «Скопировать», ссылка на `verification_uri` (системный браузер), «Отмена».
- GitLab: «Завершите вход в браузере», «Отмена».
- Успех — `onSaved()`, как после сохранения PAT. Отмена — возврат в исходное состояние без сообщения. Ошибка — текст под блоком с жирафом, как у PAT.
- Ошибка доступа к репозиторию github.com для OAuth-записи с нашим client_id дополняется ссылкой «Установить Longpole на организацию» — `https://github.com/apps/<slug>/installations/new`; без установки GitHub App на org репозитории org не видны. На GHES slug приложения нам неизвестен — ссылки нет, установку описывает README.
- Настройки: у хоста с `source: oauth` подпись «Вход через OAuth» и кнопка «Удалить».

Новые строки — во все локали `app/src/shared/i18n/locales/*.json`.

## 7. Ошибки

Новые `ErrorCode`:

- `oauthDenied` — пользователь отклонил вход (`access_denied`).
- `oauthExpired` — истёк device-код или 10-минутный таймаут.
- `oauthFailed` — прочее: неверный client_id, Device Flow не включён в приложении, ответ не разобрался, ошибка обмена кода. Текст ответа сервера — в `detail`.

Токены, refresh token, `code`, `verifier` не попадают в логи и `detail`.

## 8. Тесты

`core/tests/oauth.rs` на wiremock:

- Device Flow: `authorization_pending` → `slow_down` (интервал вырос) → токен; `access_denied` → `oauthDenied`; `expired_token` → `oauthExpired`; отмена прерывает опрос.
- PKCE: тест подменяет `open` — сам шлёт GET на `redirect_uri`; обмен идёт с верным `code_verifier` (сверка S256 с `code_challenge` из URL); чужой `state` → 400, flow ждёт дальше.
- Refresh: истекающая запись обновляется и перезаписывается; два параллельных `get` — ровно один запрос refresh (`expect(1)`); `invalid_grant` удаляет запись и хост; сетевой сбой оставляет запись.

`core/tests/tokens.rs`: PAT-строка читается как раньше; OAuth-запись даёт `TokenSource::Oauth`.

## 9. Зависимости и внешние шаги

`core`: `sha2`, `base64`, `getrandom` — проверить `cargo tree`, что версии уже в дереве, и взять их. `src-tauri`: уже подключённый `tauri-plugin-opener`.

Вне кода (владелец репо):

1. GitHub App «Longpole» на github.com: Device Flow включён, права Actions/Contents/Metadata: read, webhook выключен. Его client_id и slug — константы.
2. Приложение на gitlab.com: non-confidential, scope `read_api`, redirect `http://127.0.0.1/callback`. Application ID — константа.
3. README: как зарегистрировать приложение на self-hosted GitLab и GHES и куда вставить client_id.

## 10. Открытые вопросы

- С какой версии GHES Device Flow работает для GitHub App (для OAuth App — с 3.1.0). Старый GHES без него — пользователь остаётся на PAT.
- Какая роль нужна для группового приложения GitLab; инструкция в README описывает пользовательское приложение, ему роль не нужна.

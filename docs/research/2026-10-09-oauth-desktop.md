# OAuth для десктопа: GitHub и GitLab без client_secret

Дата обращения ко всем источникам: 2026-10-09. Исходники GitLab читались с
зеркала `github.com/gitlabhq/gitlabhq` (ветка `master`, коммит `7a83a0e` от
2026-10-09, плюс теги релизов). Raw-доступ к `gitlab.com/gitlab-org/gitlab` в этот
день отвечал `503 Upstream Gitaly has been exhausted`. Doorkeeper — тег `v5.9.3`:
эту версию фиксирует `Gemfile.lock` GitLab [gl-gemlock].

Ограничение: секрет в бинарь open-source десктопа не кладём. Поэтому годятся
только flow, в которых клиент не предъявляет `client_secret` ни при обмене, ни
при refresh.

## 1. Вывод

| Провайдер | Flow | Почему |
|---|---|---|
| github.com, GHES | **Device Flow** (OAuth App или GitHub App) | Это единственный flow GitHub без `client_secret`. В Authorization Code секрет обязателен даже с PKCE [gh-oauth], [gh-pkce-disc]. Refresh для токенов из device flow тоже идёт без секрета [gh-refresh]. На GHES Device Flow есть с 3.1.0 [ghes-31-rn]. |
| gitlab.com, GitLab self-managed | **PKCE + loopback** (`http://127.0.0.1/<path>`, любой порт) как основной; **Device Flow** как запасной | Non-confidential приложение с PKCE работает без секрета, refresh тоже [gl-oauth2], [dk-app]. Порт loopback при сравнении игнорируется (RFC 8252 §7.3, Doorkeeper ≥ 5.1) [dk-uri], [dk-cl]. Device Flow есть только с 17.2/17.3 (GA в 17.9). С 18.11 его включают галкой на каждое приложение, у новых приложений она выключена [gl-oauth2], [gl-dev-patch], [gl-app-model]. |

Что выбрать на GitHub:

- **GitHub App + Device Flow** — рекомендуемый вариант. Он даёт read-only права
  (`Actions: read`, `Contents: read`, `Metadata: read`) [gh-perm] и токены на
  8 ч с refresh на 6 мес. [gh-uat]. Цена — приложение надо **установить** на
  org/репо. На org это делает только owner, остальные шлют запрос [gh-install],
  [gh-uat-access].
- **OAuth App + Device Flow** проще для пользователя: установка не нужна. Но для
  приватных репо нужен scope `repo`, а он даёт **полный read/write** к коду.
  Read-only scope для кода и Actions нет [gh-scopes], [gh-runs]. Кроме того,
  в новых org по умолчанию включены OAuth App access restrictions, и без
  одобрения owner'а приватные ресурсы org недоступны [gh-oauth-restr].

Гибрид на GitLab: если инстанс отвечает на `POST /oauth/authorize_device`, а у
приложения стоит галка Device grant, можно предложить и его, например когда
браузер не на этой машине. По умолчанию — PKCE + loopback: он работает на любом
поддерживаемом self-managed и не требует от пользователя дополнительной галки.

Пользователь self-hosted вводит в Longpole **хост и client_id**. На GitLab это
«Application ID» non-confidential приложения, на GHES — Client ID приложения с
включённым Device Flow. Секрет не вводит.

## 2. GitHub (github.com и GHES)

### 2.1. Authorization Code + PKCE: секрет обязателен

- PKCE поддерживается для OAuth Apps и GitHub Apps с **2025-07-14**, только
  `S256`. Цитата: «GitHub does not distinguish between public and confidential
  clients» [gh-pkce-cl].
- В таблице параметров `POST /login/oauth/access_token` у `client_secret`
  стоит **Required**, с PKCE тоже [gh-oauth]. Для GitHub App так же: «Required.
  The client secret for your GitHub App» [gh-uat].
- Представитель GitHub (hpsin) в официальной community-дискуссии, 2025-07-15:
  «we don't have a "public client" concept yet, so we treat all clients the same
  and all of them require access to the client secret». 2025-10-27: «We want to
  move away from that» [gh-pkce-disc]. Это форум, а не документация, но
  документация говорит то же самое.
- **GHES:** в документации GHES 3.22 (`enterprise-server@latest`) написано:
  «`code_challenge` and `code_challenge_method` are not supported at this time»
  [ghes-oauth]. На GHES PKCE нет совсем.

Вывод: для GitHub Authorization Code нам недоступен.

### 2.2. Loopback redirect

Для справки, ведь без секрета web flow всё равно не работает. В разделе
«Loopback redirect urls» для OAuth App сказано: «The `redirect_uri` does not need
to match the port specified in the callback URL for the app». Пример:
callback `http://127.0.0.1/path`, redirect `http://127.0.0.1:1234/path`.
Рекомендация — `127.0.0.1` или `::1` вместо `localhost` [gh-oauth].
Без wildcard остальная часть URL должна совпадать точно [gh-oauth].
В GHES 3.0/3.1 этот раздел назывался «Localhost redirect urls» [ghes-30],
[ghes-31]. На странице про GitHub App правило для loopback не описано [gh-uat].

### 2.3. Device Flow

- `POST https://github.com/login/device/code` (`client_id`, `scope`) →
  пользователь вводит `user_code` на `/login/device` → опрос
  `POST /login/oauth/access_token` с `grant_type=urn:ietf:params:oauth:grant-type:device_code`.
  Цитата: «The `client_secret` is not needed for the device flow» [gh-oauth].
- **Device Flow надо включить в настройках приложения**, иначе вернётся ошибка
  `device_flow_disabled` [gh-oauth]. Для GitHub App требование то же [gh-uat].
- Код живёт 15 мин. Опрашивать не чаще `interval`, при `slow_down` добавлять
  +5 с. Ввод кода — не больше 50 попыток в час на приложение [gh-oauth].
- **GHES:** в release notes 3.1.0 (2021-06-03) есть «OAuth 2.0 Device
  Authorization Grant» [ghes-31-rn]. В документации 3.1 стоит пометка «The
  device flow is in public beta» [ghes-31], в 3.22 она описана без пометки
  [ghes-oauth]. Эндпоинты — `http(s)://HOSTNAME/login/device...`. Для GitHub App
  на GHES 3.22 Device Flow описан [ghes-uat]. С какой версии GHES он работает
  для GitHub App, я не установил. Сейчас поддерживаются версии ≥ 3.18: страница
  3.17 сообщает о прекращении поддержки 2026-08-25.

### 2.4. Сроки жизни и refresh

| | OAuth App | GitHub App (user access token) |
|---|---|---|
| Access token | по умолчанию не истекает. Отзывается, если не использовался год [gh-expiry]. Можно включить expiring tokens или запросить scope `offline_access`: тогда 8 ч [gh-oauth] | **8 ч** (`expires_in: 28800`) [gh-uat], [gh-refresh] |
| Refresh token | 6 мес. без использования (если expiring) [gh-oauth] | **6 мес.** (`refresh_token_expires_in: 15897600`) [gh-uat] |
| Секрет при refresh | «Required unless the token was generated using the device flow» [gh-oauth] | то же самое [gh-refresh] |
| Ротация | старые refresh и access после использования перестают работать [gh-oauth] | то же самое [gh-refresh] |

- У новых GitHub App истечение включено, пока владелец его не выключит
  [gh-refresh].
- GHES: «the GitHub Enterprise Server instance may not yet support expiring
  tokens». `offline_access` там может ни на что не влиять, и refresh-токена не
  будет [gh-oauth]. Код должен работать и с неистекающим токеном без refresh.
- Лимит: 10 токенов на комбинацию user/app/scope, при выпуске 11-го один из
  старых отзывается [gh-expiry]. Это касается повторных логинов на нескольких
  машинах.

### 2.5. Права

**OAuth App (scopes).** Для `GET /actions/runs`, `/actions/runs/{id}` и
`/actions/runs/{id}/jobs` на приватном репо нужен scope `repo`: «OAuth app
tokens and personal access tokens (classic) need the repo scope to use this
endpoint with a private repository» [gh-runs]. На странице contents scope для
`GET` не указан явно [gh-contents], но `repo` его покрывает: «full access to
public and private repositories». Read-only scope для кода или Actions **нет**,
`workflow` даёт запись workflow-файлов [gh-scopes]. Для одних публичных репо
scope не нужен.

**GitHub App (permissions)** [gh-perm]:

- `Actions: read` — `GET /repos/{o}/{r}/actions/runs`, `.../runs/{run_id}`,
  `.../runs/{run_id}/jobs`, `.../actions/jobs/{job_id}`;
- `Contents: read` — `GET /repos/{o}/{r}/contents/{path}`;
- `Metadata: read` — `GET /repos/{o}/{r}`.

Это совпадает с текущим fine-grained PAT (Actions: read + Contents: read).

**Установка GitHub App.** «An app does not need to be installed in order for a
user to authorize the app», но «The app can only access resources in an account
where it is installed». Если приложение установлено только на личный аккаунт,
ресурсы org ему недоступны [gh-uat-access]. Ставить на org может только owner.
App manager этого сделать не может. Участник может выбрать org, и тогда owner
получит запрос. На личный аккаунт приложение ставит кто угодно. При установке
выбираются репо: все или выбранные [gh-install].

## 3. GitLab (gitlab.com и self-managed)

### 3.1. Non-confidential приложение + PKCE

- Документация: PKCE позволяет обмен «on public clients without requiring
  access to the Client Secret at all». В примерах обмена и refresh для PKCE
  `client_secret` нет: `client_id=APP_ID&refresh_token=...&grant_type=refresh_token&redirect_uri=...`
  [gl-oauth2].
- В форме приложения есть чекбокс **Confidential**: «Enable only for confidential
  applications exclusively used by a trusted backend server ... Do not enable for
  native-mobile, single-page, or other JavaScript applications» [gl-form].
  Пользователь должен **снять** эту галку.
- Почему это работает, видно в Doorkeeper `by_uid_and_secret`:
  `return app if secret.blank? && !app.confidential?` [dk-app]. Значит, для
  non-confidential клиента достаточно `client_id`: и при обмене кода, и при
  refresh (`validate_client_match` сверяет `client` с приложением токена)
  [dk-refresh]. Для confidential приложения без секрета `client` = nil, и запрос
  отклоняется.
- `force_pkce` (Doorkeeper 5.7.1) GitLab не включает [gl-dk-init], [dk-cl]. PKCE
  не обязателен, но мы его шлём всегда.
- История: public clients без секрета и PKCE (RFC 7636) пришли в Doorkeeper
  5.0.0 [dk-cl]. В GitLab 13.0 стоит Doorkeeper 5.0.3, в 14.0 — 5.5.0.rc2, и в
  форме 14.0 чекбокс Confidential уже есть [gl-gemlock-tags]. С какой версии
  GitLab этот чекбокс появился, я не устанавливал. Для поддерживаемых версий
  это не важно.

### 3.2. Loopback redirect_uri и порт

- Doorkeeper `URIChecker.matches?`: «RFC8252, Paragraph 7.3 … if
  loopback_uri?(url) && loopback_uri?(client_url) → url.port = nil;
  client_url.port = nil». Проверка `IPAddr.new(uri.host).loopback?` [dk-uri].
  Значит, зарегистрированный `http://127.0.0.1/callback` совпадёт с
  `http://127.0.0.1:53817/callback`. Путь и схема должны совпадать.
  `localhost` не подходит: это не IP-литерал, `IPAddr.new` падает, и функция
  возвращает false.
- Поведение появилось в Doorkeeper 5.1.0: «Fix loopback IP redirect URIs to
  conform with RFC8252, p. 7.3» [dk-cl]. В GitLab 13.6 уже стоит Doorkeeper 5.3.3
  [gl-gemlock-tags].
- HTTP разрешён: `force_ssl_in_redirect_uri false` [gl-dk-init]. Документация:
  «You can use a non-SSL URL instead, but you should use an SSL URL»
  [gl-provider].
- RFC 8252 §7.3: сервер «MUST allow any port to be specified at the time of the
  request for loopback IP redirect URIs» [rfc8252].

### 3.3. Device Authorization Grant (RFC 8628)

- История [gl-oauth2]: «Introduced in GitLab 17.2 with a feature flag named
  oauth2_device_grant_flow», «Enabled by default in 17.3», «Generally available
  in GitLab 17.9. Feature flag oauth2_device_grant_flow removed». Тот же документ
  в списке flow пишет «(GitLab 17.1 and later)». Это внутреннее противоречие,
  опираюсь на history.
- Запросы: `POST /oauth/authorize_device` с `client_id`, `scope` → опрос
  `POST /oauth/token` с `grant_type=urn:ietf:params:oauth:grant-type:device_code`,
  `device_code`, `client_id`. Секрета в примерах нет [gl-oauth2]. Клиента
  аутентифицирует тот же `Doorkeeper::Server#client` [dk-dag-req], поэтому
  confidential приложению без секрета ответят `invalid_client` (вывод из
  [dk-app]). Device Flow тоже требует **non-confidential** приложения.
- **Новое в 18.11: галка на каждое приложение.** Патч
  `doorkeeper_device_authorization_patch.rb` добавляет
  `validate :device_code_enabled, error: AccessDenied` [gl-dev-patch]. В модели
  написано: колонка «enabled by default … so existing records have the option
  enabled but all new applications should have device_code_enabled set to
  false» [gl-app-model]. В форме это чекбокс «Device authorization grant»
  [gl-form]. Проверка `device_code_enabled` есть в теге `v18.11.0` и отсутствует
  в `v18.10.0` (сверка `grep` по тегам зеркала). На странице
  `integration/oauth_provider` этой галки нет [gl-provider].
- Код живёт 300 с, интервал опроса 5 с: это дефолты гема, GitLab их не
  переопределяет [gl-dag-init].

### 3.4. Срок жизни токенов

- «By default, access tokens expire after two hours (7200 seconds)» [gl-provider].
  В коде: `access_token_expires_in 7200` и
  `custom_access_token_expires_in { Gitlab::CurrentSettings.oauth_access_token_expires_in }`
  [gl-dk-init]. С 19.1 администратор self-managed может менять это значение
  [gl-provider].
- История: истечение введено в 14.3, возможность отказаться от него убрана в 15.0
  [gl-provider-16]. На любом GitLab ≥ 15.0 токен истекает, поэтому refresh
  обязателен.
- Ротация: «Refresh tokens may be used even after the access_token itself
  expires». Refresh инвалидирует прежние access и refresh [gl-oauth2]. Новый
  refresh token надо сохранять атомарно, иначе при гонке двух запросов один
  из них разлогинит пользователя (вывод).
- Секрет при refresh для non-confidential не нужен: см. 3.1 [gl-oauth2], [dk-app].

### 3.5. Scope

`read_api` доступен OAuth-приложениям: read-only доступ к API, в том числе к
проектам [gl-provider]. Тот же scope сейчас стоит у PAT, менять нечего.
Приложение запросит только те scopes, которые отмечены при его регистрации
(`enforce_configured_scopes`) [gl-dk-init].

## 4. Регистрация приложения на self-hosted

### GitLab

| Уровень | Где | Кто может |
|---|---|---|
| Пользовательское | Edit profile → Access → Applications [gl-provider] | любой пользователь. Админ может выключить настройку `user_oauth_applications`, тогда страница перенаправляет в профиль [gl-apps-ctrl] |
| Групповое | группа → Settings → Applications [gl-provider] | `authorize_admin_group!` [gl-grp-ctrl], то есть Owner группы (соответствие ability → роль я не проверял) |
| Instance-wide | Admin area → Applications [gl-provider] | администратор. Можно пометить **trusted**, тогда экран согласия пропускается [gl-provider] |

Что делает пользователь:

1. Name — любое. Redirect URI — `http://127.0.0.1/callback` (точная строка,
   которую мы зададим).
2. **Снять** «Confidential».
3. Для Device Flow (GitLab ≥ 18.11) — поставить «Device authorization grant».
4. Scopes — `read_api`.
5. Скопировать **Application ID** [gl-provider] и вставить в Longpole вместе с
   хостом. Секрет (поле Secret) не нужен.

На gitlab.com Longpole может зарегистрировать одно своё приложение и вшить его
Application ID. Это не секрет.

### GHES (и github.com)

- OAuth App или GitHub App регистрируется в Developer settings пользователя или
  в настройках org.
- Для обоих типов: **включить Device Flow** («Enable Device Flow») [gh-oauth],
  [gh-uat].
- GitHub App: Repository permissions Actions = Read, Contents = Read,
  Metadata = Read [gh-perm]. Затем **Install** на нужные org или репо. На org
  это делает owner [gh-install].
- В Longpole пользователь вводит **хост и Client ID**. Callback URL для device
  flow не используется, но форма регистрации может требовать какое-то значение.
  `Client secret` для device flow не нужен [gh-oauth].

На github.com Longpole регистрирует приложение сам и вшивает Client ID.

## 5. Риски и открытые вопросы

1. **GitHub OAuth App = `repo` (запись).** Токен с правом писать во все
   приватные репо пользователя хранится в связке ключей. Это хуже текущего
   fine-grained PAT. GitHub App решает проблему, но требует установки на org,
   а это зависит от owner'а [gh-install].
2. **OAuth App restrictions в org.** По умолчанию они включены, и без одобрения
   owner'а приватные данные org недоступны [gh-oauth-restr]. Для пользователя
   это выглядит как «логин прошёл, а репо не видно». Нужен понятный текст ошибки.
3. **GHES без expiring tokens.** Refresh может не прийти [gh-oauth]. Код должен
   работать и с неистекающим токеном.
4. **GitLab 18.11+: Device Flow выключен у новых приложений** [gl-app-model].
   Если строить GitLab только на device flow, пользователи будут упираться в
   `access_denied`. Отсюда PKCE + loopback как основной вариант.
5. **GitLab < 17.3 self-managed: Device Flow нет** (в 17.2 только за флагом)
   [gl-oauth2]. PKCE + loopback работает и там.
6. **Ротация refresh на GitLab и GitHub.** Использованный refresh умирает
   [gl-oauth2], [gh-refresh]. Несколько окон или процессов Longpole не должны
   обновлять токен параллельно, нужна блокировка вокруг refresh.
7. **Лимит 10 токенов** на user/app/scope у GitHub [gh-expiry]. Каждый новый
   логин (другая машина, переустановка) отзывает самый старый токен.
8. **Loopback-сервер.** Слушать только `127.0.0.1`, порт брать эфемерный,
   проверять `state`. RFC 8252 §8.3 рекомендует loopback IP вместо `localhost`
   [rfc8252]. Для GitLab это обязательно: `localhost` не матчится (см. 3.2).
9. **Не подтверждено:**
   - что GitHub App callback URL тоже игнорирует порт loopback (страница GitHub
     App молчит [gh-uat]). Для нас неважно: web flow GitHub недоступен из-за
     секрета;
   - с какой версии GHES Device Flow работает для **GitHub App** (для OAuth App —
     3.1.0);
   - точная роль для групповых приложений GitLab (`admin_group` → Owner — моё
     предположение);
   - позиция GitHub «we want to move away» [gh-pkce-disc]: сроков нет. Если
     GitHub введёт public clients, PKCE + loopback станет возможен и там.

## Источники

- [gh-oauth] https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps
- [gh-uat] https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-user-access-token-for-a-github-app
- [gh-refresh] https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/refreshing-user-access-tokens
- [gh-uat-access] https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/authenticating-with-a-github-app-on-behalf-of-a-user
- [gh-install] https://docs.github.com/en/apps/using-github-apps/installing-a-github-app-from-a-third-party
- [gh-scopes] https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps
- [gh-runs] https://docs.github.com/en/rest/actions/workflow-runs
- [gh-contents] https://docs.github.com/en/rest/repos/contents
- [gh-perm] https://docs.github.com/en/rest/authentication/permissions-required-for-github-apps
- [gh-expiry] https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/token-expiration-and-revocation
- [gh-oauth-restr] https://docs.github.com/en/organizations/managing-oauth-access-to-your-organizations-data/about-oauth-app-access-restrictions
- [gh-pkce-cl] https://github.blog/changelog/2025-07-14-pkce-support-for-oauth-and-github-app-authentication/
- [gh-pkce-disc] https://github.com/orgs/community/discussions/15752 (ответы hpsin от 2025-07-15 и 2025-10-27)
- [ghes-oauth] https://docs.github.com/en/enterprise-server@latest/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps (3.22)
- [ghes-uat] https://docs.github.com/en/enterprise-server@latest/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-user-access-token-for-a-github-app
- [ghes-30] https://docs.github.com/en/enterprise-server@3.0/developers/apps/building-oauth-apps/authorizing-oauth-apps
- [ghes-31] https://docs.github.com/en/enterprise-server@3.1/developers/apps/building-oauth-apps/authorizing-oauth-apps
- [ghes-31-rn] https://docs.github.com/en/enterprise-server@3.1/admin/release-notes (3.1.0, 2021-06-03)
- [gl-oauth2] https://docs.gitlab.com/api/oauth2/
- [gl-provider] https://docs.gitlab.com/integration/oauth_provider/
- [gl-provider-16] https://archives.docs.gitlab.com/16.11/ee/integration/oauth_provider.html (Access token expiration, History)
- [gl-dk-init] https://github.com/gitlabhq/gitlabhq/blob/master/config/initializers/doorkeeper.rb
- [gl-dag-init] https://github.com/gitlabhq/gitlabhq/blob/master/config/initializers/doorkeeper_device_authorization_grant.rb
- [gl-dev-patch] https://github.com/gitlabhq/gitlabhq/blob/master/config/initializers/doorkeeper_device_authorization_patch.rb (сверено с тегами v18.10.0 и v18.11.0)
- [gl-app-model] https://github.com/gitlabhq/gitlabhq/blob/master/app/models/authn/oauth_application.rb
- [gl-form] https://github.com/gitlabhq/gitlabhq/blob/master/app/views/shared/doorkeeper/applications/_form.html.haml
- [gl-apps-ctrl] https://github.com/gitlabhq/gitlabhq/blob/master/app/controllers/oauth/applications_controller.rb
- [gl-grp-ctrl] https://github.com/gitlabhq/gitlabhq/blob/master/app/controllers/groups/settings/applications_controller.rb
- [gl-gemlock] https://github.com/gitlabhq/gitlabhq/blob/master/Gemfile.lock (doorkeeper 5.9.3, doorkeeper-device_authorization_grant 1.0.3)
- [gl-gemlock-tags] `Gemfile.lock` и `_form.html.haml` на тегах v13.0.0, v13.6.0, v14.0.0 того же зеркала
- [dk-uri] https://github.com/doorkeeper-gem/doorkeeper/blob/v5.9.3/lib/doorkeeper/oauth/helpers/uri_checker.rb
- [dk-app] https://github.com/doorkeeper-gem/doorkeeper/blob/v5.9.3/lib/doorkeeper/orm/active_record/mixins/application.rb (`by_uid_and_secret`)
- [dk-refresh] https://github.com/doorkeeper-gem/doorkeeper/blob/v5.9.3/lib/doorkeeper/oauth/refresh_token_request.rb
- [dk-cl] https://github.com/doorkeeper-gem/doorkeeper/blob/v5.9.3/CHANGELOG.md (5.0.0 #1031, #997; 5.1.0 #1182; 5.7.1 #1705)
- [dk-dag-req] https://github.com/exop-group/doorkeeper-device_authorization_grant/blob/v1.0.3/lib/doorkeeper/device_authorization_grant/oauth/device_authorization_request.rb
- [rfc8252] https://www.rfc-editor.org/rfc/rfc8252 (§7.3, §8.3)
- [rfc8628] https://www.rfc-editor.org/rfc/rfc8628

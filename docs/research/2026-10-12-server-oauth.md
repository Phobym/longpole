# OAuth web flow для серверного Longpole: GitLab self-managed и GHES

Дата обращения ко всем источникам: 2026-10-12. Тикет: #74, карта: #71.
Исходники GitLab читались с зеркала `github.com/gitlabhq/gitlabhq` (ветка
`master`, коммит `a141981` от 2026-10-12). Doorkeeper — тег `v5.9.3`, его
фиксирует `Gemfile.lock` GitLab (см. [десктопное исследование][desk]).
Документация GHES — `enterprise-server@latest`, это 3.22.

Условие задачи. Selfhosted Longpole — многопользовательский stateless
веб-сервис. Сессия хранится в зашифрованной cookie, базы нет. Каждый
пользователь входит через OAuth, и сервер ходит в API от его имени. Хост и
OAuth-приложение задаёт администратор при деплое. Клиент **confidential**:
`client_secret` хранится на сервере.

## 1. Вывод

| | GitLab self-managed | GHES |
|---|---|---|
| Тип приложения | **Instance-wide** приложение в Admin area, галка **Confidential** включена. По желанию **trusted**: тогда экран согласия не показывается [gl-provider] | **GitHub App** (user access token через web flow). Запасной вариант — OAuth App [ghes-diff] |
| Flow | Authorization Code + `client_secret` + PKCE (S256) | Authorization Code + `client_secret`, **без PKCE**: на GHES его нет [ghes-oauth] |
| Redirect URI | `https://<longpole>/auth/gitlab/callback`. Точное совпадение, можно несколько строк [dk-uri], [gl-form] | `https://<longpole>/auth/github/callback`. У GitHub App до 10 callback URL [ghes-cb] |
| Права | scope `read_api`. Он покрывает и `GET /user` [gl-api], [gl-provider] | Repository permissions `Actions: read`, `Contents: read`, `Metadata: read` [gh-perm] |
| Access token | 2 ч. С 19.1 администратор может менять срок [gl-provider] | 8 ч [ghes-uat] |
| Refresh | Ротация: старая пара умирает сразу, повтор даёт `invalid_grant` [gl-oauth2], [dk-refresh] | Ротация, 6 мес. [ghes-refresh] |
| Отзыв | `POST /oauth/revoke` (RFC 7009) [gl-oauth2] | `DELETE /applications/{client_id}/token` или `/grant` [ghes-oauthapps] |
| Rate limits | По умолчанию **выключены**. Если включить — 7200 запросов в час на пользователя [gl-rl] | По умолчанию **выключены**, их задаёт site admin [ghes-rl] |

Главный риск для stateless-сессии — **ротация refresh-токена**. У обоих
провайдеров refresh-токен одноразовый. Если два параллельных запроса из одного
браузера обновят токен одновременно, один из них получит `invalid_grant`, а
браузер может остаться с cookie, где лежит уже мёртвый refresh-токен. Нужен
single-flight на refresh и тихий повторный вход как запасной путь (раздел 4).

## 2. GitLab self-managed

### 2.1. Тип приложения

- Приложение бывает трёх уровней: пользовательское, групповое и
  instance-wide. «These methods only differ by permission level».
  Instance-wide создаёт администратор в Admin area. Его можно пометить
  **trusted**: «The user authorization step is automatically skipped for this
  application» [gl-provider].
- Для сервера нужна галка **Confidential**: «Enable only for confidential
  applications exclusively used by a trusted backend server that can securely
  store the client secret» [gl-form]. Для confidential приложения Doorkeeper
  требует секрет и при обмене кода, и при refresh: `by_uid_and_secret` без
  секрета вернёт `nil`, а запрос получит `invalid_client` [dk-app], [dk-refresh].
- Рекомендация — instance-wide. Его не удалят вместе с аккаунтом сотрудника.
  Администратор GitLab и администратор Longpole обычно один и тот же человек.
  Групповое приложение тоже сработает: права определяются пользователем и
  scope, а не уровнем приложения [gl-provider]. Этот вывод я не проверял на
  живом инстансе.
- Удаление приложения удаляет «all grants and tokens associated with the
  application» [gl-provider]. Это аварийный выключатель для всех сессий сразу.

### 2.2. Redirect URI

- Поле Redirect URI — textarea, «Use one line per URI» [gl-form]. Doorkeeper
  разбивает строку и ищет точное совпадение: схема, хост, порт и путь. Query
  запроса не учитывается, если у зарегистрированного URI query нет
  [dk-uri] (`valid_for_authorization?`, `matches?`). Порт игнорируется только
  для loopback.
- HTTP разрешён (`force_ssl_in_redirect_uri false`) [gl-dk-init], но
  документация требует: «For production, use HTTPS for your redirect_uri»
  [gl-oauth2].
- `redirect_uri` в обмене кода должен совпадать с тем, что ушёл в authorize
  [gl-oauth2]. Значит, публичный URL Longpole должен быть известен серверу
  из конфига, а не из заголовка `Host`.

### 2.3. Scopes

- `read_api`: «Grants read access to the API, including all groups and
  projects…» [gl-provider]. В `lib/api/api.rb` стоит
  `allow_access_with_scope :read_api, if: ->(request) { request.get? || request.head? }`
  на весь API [gl-api]. Значит, `read_api` покрывает пайплайны, джобы, файлы
  репозитория, списки проектов и `GET /user`. Это тот же scope, что у десктопа
  (`app/core/src/oauth.rs`).
- `read_user` и `openid` отдельно не нужны. `openid` пригодится, только если
  захотим `id_token` вместо вызова `GET /user` (вывод).
- `enforce_configured_scopes`: запросить можно только scopes, отмеченные при
  регистрации [gl-dk-init].

### 2.4. Токены, refresh и ротация

- Access token живёт 7200 с [gl-provider] (`access_token_expires_in 7200` и
  `custom_access_token_expires_in` из настроек инстанса [gl-dk-init]).
  «OAuth access token expiry configurable by instance administrators
  introduced in GitLab 19.1» [gl-provider]. Значит, срок жизни надо брать из
  `expires_in` ответа, а не зашивать.
- Refresh: `client_id`, `client_secret`, `refresh_token`,
  `grant_type=refresh_token`, `redirect_uri` [gl-oauth2]. Запрос «Invalidates
  the existing access_token and refresh_token» и возвращает новую пару.
  Refresh работает и после истечения access token [gl-oauth2]. Срока жизни
  самого refresh-токена в документации нет.
- **Как именно идёт ротация.** В Doorkeeper два режима [dk-refresh]:
  - `refresh_token_revoked_on_use?` — старый refresh отзывается, только когда
    новый access token впервые используют. Этот режим допускает параллельные
    refresh. Он включается, если в `oauth_access_tokens` есть колонка
    `previous_refresh_token`;
  - иначе refresh идёт под `refresh_token.with_lock`: старый токен
    отзывается сразу, а повторное использование даёт
    `Errors::InvalidGrantReuse`.

  В `db/structure.sql` GitLab в таблице `oauth_access_tokens` колонки
  `previous_refresh_token` **нет** [gl-structure]. Значит, GitLab работает во
  втором режиме. Из двух параллельных refresh одним токеном один получает
  `invalid_grant`, и прежний access token тоже перестаёт работать
  [gl-oauth2].
- Авторизационный код живёт 10 мин. Это дефолт Doorkeeper, GitLab его не
  переопределяет [gl-dk-init].
- Длина токенов. `default_generator_method :hex` [gl-dk-init]. Токены —
  десятки символов, пара access+refresh спокойно помещается в cookie
  (RFC 6265 §6.1: браузер должен хранить не меньше 4096 байт на cookie
  [rfc6265]). Это вывод, точную длину я не мерил.

### 2.5. PKCE

- Документация называет PKCE «recommended for both client and server apps», а
  Authorization Code — «Recommended option for secure server-side apps»
  [gl-oauth2]. Терминов «confidential/public» на странице нет.
- `force_pkce` GitLab не включает [gl-dk-init]. PKCE с confidential клиентом
  работает: Doorkeeper проверяет `code_verifier`, если в authorize пришёл
  `code_challenge`, а секрет проверяет независимо от этого [dk-app]. RFC 9700
  §2.1.1: для confidential клиентов PKCE «RECOMMENDED» [rfc9700].
  Рекомендация — слать `S256` всегда. Это бесплатно и защищает от подмены кода.
- Где держать `state` и `code_verifier` без БД: в отдельной короткоживущей
  зашифрованной cookie, `SameSite=Lax`, `HttpOnly`, `Secure`. Редирект с
  GitLab на callback — top-level GET, поэтому Lax-cookie придёт (вывод).

### 2.6. Отзыв

- `POST /oauth/revoke` с `client_id`, `client_secret`, `token` → `200 {}`
  [gl-oauth2]. Это вызов «Выйти».
- Пользователь видит приложение в Edit profile → Access → Applications →
  Authorized applications и может нажать Revoke [gl-provider]. После этого
  refresh вернёт `invalid_grant`, и Longpole должен отправить пользователя на
  повторный вход.

### 2.7. Повторный вход без экрана согласия

GitLab переопределяет `Oauth::AuthorizationsController#new`: если
`skip_authorization?` (trusted) или `matching_token? && confidential?`, код
выдаётся без экрана согласия [gl-authz-ctrl]. `matching_token?` — «Active
access token issued for the same client and resource owner with the same set
of the scopes» [dk-authz-ctrl]. Значит, для confidential приложения повторный
authorize при живой сессии в GitLab — это просто цепочка редиректов. Ею можно
восстанавливаться после потерянного refresh, не беспокоя пользователя. Для
trusted приложения это работает всегда.

### 2.8. Rate limits

- «The following limits are disabled by default». Authenticated API — per
  user, по умолчанию 7200 запросов за 3600 с [gl-rl]. Дефолты в схеме:
  `throttle_authenticated_api_enabled DEFAULT false`,
  `..._requests_per_period DEFAULT 7200`, `..._period_in_seconds DEFAULT 3600`
  [gl-structure].
- Лимит считается на пользователя, а не на приложение. Запросы Longpole от
  имени пользователя делят квоту с его же CI-скриптами и PAT (вывод из «per
  user»). Ответы содержат `RateLimit-*`, при 429 приходит `Retry-After` [gl-rl].

## 3. GHES

### 3.1. GitHub App или OAuth App

| | GitHub App (user access token) | OAuth App |
|---|---|---|
| Права | `Actions: read`, `Contents: read`, `Metadata: read` [gh-perm]. «only has permissions that both the user and the app have» [ghes-uat] | Для приватных репо нужен scope `repo`, а это полный read/write. Read-only scope нет [desk] |
| Доступ к репо | Только там, где приложение **установлено**. На org ставит owner или админ репо [ghes-diff] | Всё, что видит пользователь. Ограничивается application policy org [ghes-diff] |
| Срок токена | 8 ч + refresh 6 мес. с ротацией. Владелец приложения может выключить истечение [ghes-uat], [ghes-exp] | «OAuth tokens remain active until they're revoked» [ghes-diff]. На странице GHES нет ни `offline_access`, ни expiring tokens [ghes-oauth] |
| Список проектов | `GET /user/installations` → `GET /user/installations/{id}/repositories` [ghes-inst] | `GET /user/repos`, `GET /orgs/:org/repos` [ghes-diff] |

Рекомендация — **GitHub App**: read-only права и короткие токены. Цена —
установка на каждую org, и её делает owner. Для selfhosted это одна операция
администратора при деплое, а не забота каждого пользователя, как на десктопе.

### 3.2. Web flow и redirect

- `GET http(s)://HOSTNAME/login/oauth/authorize`: `client_id`, `redirect_uri`,
  `state` (оба «strongly recommended») [ghes-uat].
- `POST /login/oauth/access_token`: `client_id`, `client_secret` (**Required**),
  `code`, `redirect_uri` [ghes-uat].
- GitHub App: «You can specify up to 10 callback URLs». Если `redirect_uri`
  не задан, берётся первый [ghes-cb]. `redirect_uri` «must match one of the
  URLs you provided as a "Callback URL"» и «can't contain any additional
  parameters» [ghes-uat].
- OAuth App: callback один. Хост и порт должны совпадать точно, а путь может
  быть поддиректорией callback [ghes-oauth].
- Код живёт 10 мин [ghes-oauth].

### 3.3. PKCE

На GHES PKCE нет: «`code_challenge` and `code_challenge_method` are not
supported at this time» [ghes-oauth]. На странице GitHub App этих параметров
тоже нет [ghes-uat]. Защита — только `state` и `client_secret`. Позже PKCE
можно включить без смены flow, если GHES его добавит (на github.com он есть
с 2025-07-14 [desk]).

### 3.4. Токены, refresh и ротация

- GitHub App: `expires_in` «will always be `28800`», `refresh_token_expires_in`
  «will always be `15897600`» (6 мес.). Если токен не истекает, refresh не
  приходит [ghes-uat].
- Refresh: `client_id`, `client_secret` («Required unless the user access token
  was generated using the device flow»), `grant_type=refresh_token`,
  `refresh_token`. «Once you use a refresh token, that refresh token and the old
  user access token will no longer work» [ghes-refresh]. Механизм ротации тот
  же, что на GitLab: одноразовый refresh. Есть ли у GitHub окно на повтор, в
  документации не сказано.
- Истечение включено у новых приложений, «unless you opt out». Выключается в
  Optional Features → «User-to-server token expiration» [ghes-refresh]. GHES
  3.22 пишет: «Owners of GitHub Apps can optionally configure these tokens to
  never expire instead, but this is not recommended» [ghes-exp].
- **Лимит 10 токенов** на user/app/scope и не больше 10 новых токенов в час.
  При 11-м токене отзывается самый старый неиспользованный, а если таких нет —
  тот, что использовали давнее всех. Превышение часового лимита вызывает
  повторный экран согласия [ghes-exp]. Для stateless-сервиса это значит:
  каждый браузер или устройство пользователя — отдельный токен. Одиннадцатый
  вход разлогинит один из старых, а цикл автоматических повторных входов
  упрётся в 10 в час.

### 3.5. Отзыв

- `DELETE /applications/{client_id}/token` (тело `access_token`) отзывает один
  токен. `DELETE /applications/{client_id}/grant` удаляет все токены
  приложения для пользователя. Оба работают и для `gho_`, и для `ghu_`
  [ghes-oauthapps]. Как аутентифицироваться, на странице GHES не сказано
  («Authentication works differently with this particular endpoint»). На
  github.com это basic auth `client_id:client_secret` — для GHES это не
  подтверждено.
- Пользователь отзывает доступ в настройках. Отзыв авторизации отзывает и все
  её токены [ghes-exp].

### 3.6. Rate limits

«Rate limits are disabled by default for GitHub Enterprise Server». Site admin
может их включить, в том числе secondary [ghes-rl]. Документация описывает
вторичные лимиты: 100 одновременных запросов, 900 очков в минуту на REST,
«No more than 2,000 OAuth access token requests per hour are allowed for GitHub
Apps and OAuth apps» [ghes-rl]. Последний лимит действует на всё приложение и
важен для массовых refresh: 2000 в час — потолок для всех пользователей
вместе (вывод). Для OAuth App: «OAuth tokens use the user's rate limit of
5,000 requests per hour», если лимиты включены [ghes-diff].

## 4. Последствия для stateless-сессии в cookie

1. **Гонка refresh.** Фронт шлёт несколько `fetch` параллельно. Если access
   token истёк, каждый запрос на сервере попытается обновить его одним и тем же
   refresh. На GitLab победит один, остальные получат `invalid_grant`
   [dk-refresh]. На GitHub поведение такое же по смыслу [ghes-refresh].
   Ответы с разными `Set-Cookie` придут в произвольном порядке, и последний
   может перезаписать хорошую cookie плохой.
   Что можно сделать (выводы, не из документации):
   - обновлять заранее (за N минут до `expires_in`) и только в одном
     endpoint, а не в каждом API-вызове;
   - single-flight на сервере: in-memory map по хэшу refresh-токена с
     результатом на несколько секунд. Это работает при одной реплике или
     sticky-сессиях;
   - single-flight на фронте: один промис refresh на вкладку. Между вкладками
     — `BroadcastChannel` или `navigator.locks`;
   - при `invalid_grant` не ставить пустую cookie, а повторить запрос с той
     cookie, что пришла теперь (её мог обновить победитель). Если не помогло —
     тихий повторный вход (2.7).
2. **Потерянный ответ.** Если ответ на refresh не дошёл до браузера (вкладку
   закрыли, сеть упала), у пользователя остаётся cookie с мёртвым refresh. Без
   БД это не исправить. Восстановление — повторный вход, на GitLab он тихий
   (2.7). На GHES тихий повтор для OAuth App описан («this step of the flow
   will automatically complete with the set of scopes the user has authorized»
   [ghes-oauth]). Для GitHub App не проверено.
3. **Нет серверного отзыва сессии.** Cookie живёт, пока жив refresh. Выйти
   везде можно только через отзыв на стороне провайдера (2.6, 3.5) или смену
   ключа cookie. Это вывод из «без БД».
4. **Секреты.** `client_secret` обоих провайдеров и ключ шифрования cookie —
   секреты деплоя. Утечка ключа cookie раскрывает refresh-токены всех, у кого
   есть перехваченные cookie, поэтому `HttpOnly`, `Secure` и `SameSite=Lax`
   обязательны (вывод).

## 5. Отличия от десктопного flow

| | Desktop (сейчас) [desk], `app/core/src/oauth.rs` | Selfhosted |
|---|---|---|
| Клиент | public, секрета нет | confidential, секрет на сервере |
| GitLab | non-confidential приложение, PKCE + loopback `http://127.0.0.1:<порт>/callback`, приложение регистрирует пользователь | confidential instance-wide приложение, Authorization Code + секрет + PKCE, HTTPS-callback на домене Longpole, регистрирует администратор |
| GitHub/GHES | Device Flow (секрет не нужен) | Web flow с секретом, без PKCE. Device Flow не нужен |
| Кто вводит хост и client_id | пользователь | администратор в конфиге деплоя |
| Хранение токенов | связка ключей ОС, один процесс | зашифрованная cookie в браузере, много параллельных запросов |
| Гонка refresh | блокировка в процессе | нет общей памяти между репликами; нужен single-flight и запасной повторный вход (раздел 4) |
| Повторный вход | открыть браузер или ввести код | тихий редирект (GitLab confidential/trusted) |
| Отзыв при выходе | не нужен (удалить из связки) | желательно вызвать revoke у провайдера |
| Установка GitHub App | каждый пользователь упирается в owner'а org | одна операция администратора |

## 6. Открытые вопросы

1. **GHES: GitHub App или OAuth App.** GitHub App даёт read-only права и
   короткие токены, но требует установки на каждую org. OAuth App не требует
   установки, но даёт `repo` (запись) и бессрочный токен в cookie. Пилот —
   GitLab, так что решение можно отложить.
2. **Выключать ли истечение токенов у GitHub App.** Без истечения нет ротации,
   а значит, нет гонки — но в cookie лежит бессрочный токен
   [ghes-refresh], [ghes-exp].
3. **Стратегия против гонки refresh.** Заранее обновлять плюс single-flight на
   фронте и in-memory на сервере? Какой режим масштабирования: одна реплика или
   sticky? От ответа зависит, хватит ли in-memory single-flight.
4. **Trusted на GitLab.** Пропуск экрана согласия удобен и гарантирует тихий
   повторный вход, но это решение администратора GitLab. Без trusted повторный
   вход тихий, только пока жив matching token (2.7).
5. **Выход.** Вызывать ли revoke у провайдера и как аутентифицироваться в
   GHES `DELETE /applications/{client_id}/token` (basic auth на GHES не
   подтверждён).
6. **Срок cookie.** Привязать к сроку refresh (GitHub — 6 мес., у GitLab
   срока нет) или ввести свой максимальный срок сессии?
7. **Лимит 10 токенов на GHES.** Нормально ли, что 11-й вход пользователя (другой
   браузер) разлогинит один из старых [ghes-exp]?
8. **Не подтверждено:** работает ли тихий повторный authorize для GitHub App
   так же, как для OAuth App; есть ли на GHES окно повторного использования
   refresh; групповое приложение GitLab вместо instance-wide на живом
   инстансе.

## Источники

- [desk] `docs/research/2026-10-09-oauth-desktop.md` — десктопное исследование и его источники
- [gl-oauth2] https://docs.gitlab.com/api/oauth2/
- [gl-provider] https://docs.gitlab.com/integration/oauth_provider/
- [gl-rl] https://docs.gitlab.com/administration/settings/user_and_ip_rate_limits/
- [gl-dk-init] https://github.com/gitlabhq/gitlabhq/blob/master/config/initializers/doorkeeper.rb
- [gl-form] https://github.com/gitlabhq/gitlabhq/blob/master/app/views/shared/doorkeeper/applications/_form.html.haml
- [gl-api] https://github.com/gitlabhq/gitlabhq/blob/master/lib/api/api.rb (строки 58–59)
- [gl-authz-ctrl] https://github.com/gitlabhq/gitlabhq/blob/master/app/controllers/oauth/authorizations_controller.rb (`#new`)
- [gl-structure] https://github.com/gitlabhq/gitlabhq/blob/master/db/structure.sql (`CREATE TABLE oauth_access_tokens`, `throttle_authenticated_api_*`)
- [dk-refresh] https://github.com/doorkeeper-gem/doorkeeper/blob/v5.9.3/lib/doorkeeper/oauth/refresh_token_request.rb
- [dk-app] https://github.com/doorkeeper-gem/doorkeeper/blob/v5.9.3/lib/doorkeeper/orm/active_record/mixins/application.rb
- [dk-uri] https://github.com/doorkeeper-gem/doorkeeper/blob/v5.9.3/lib/doorkeeper/oauth/helpers/uri_checker.rb
- [dk-authz-ctrl] https://github.com/doorkeeper-gem/doorkeeper/blob/v5.9.3/app/controllers/doorkeeper/authorizations_controller.rb
- [ghes-uat] https://docs.github.com/en/enterprise-server@latest/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-user-access-token-for-a-github-app
- [ghes-refresh] https://docs.github.com/en/enterprise-server@latest/apps/creating-github-apps/authenticating-with-a-github-app/refreshing-user-access-tokens
- [ghes-cb] https://docs.github.com/en/enterprise-server@latest/apps/creating-github-apps/registering-a-github-app/about-the-user-authorization-callback-url
- [ghes-oauth] https://docs.github.com/en/enterprise-server@latest/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps
- [ghes-diff] https://docs.github.com/en/enterprise-server@latest/apps/oauth-apps/building-oauth-apps/differences-between-github-apps-and-oauth-apps
- [ghes-exp] https://docs.github.com/en/enterprise-server@latest/authentication/keeping-your-account-and-data-secure/token-expiration-and-revocation
- [ghes-oauthapps] https://docs.github.com/en/enterprise-server@latest/rest/apps/oauth-applications
- [ghes-inst] https://docs.github.com/en/enterprise-server@latest/rest/apps/installations
- [ghes-rl] https://docs.github.com/en/enterprise-server@latest/rest/using-the-rest-api/rate-limits-for-the-rest-api
- [gh-perm] https://docs.github.com/en/rest/authentication/permissions-required-for-github-apps (github.com; для GHES права те же — по [desk], на GHES отдельно не сверял)
- [rfc9700] https://www.rfc-editor.org/rfc/rfc9700 (§2.1.1)
- [rfc6265] https://www.rfc-editor.org/rfc/rfc6265 (§6.1)
- RFC 7009 (token revocation): https://www.rfc-editor.org/rfc/rfc7009

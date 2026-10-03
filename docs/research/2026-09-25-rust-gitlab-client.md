# GraphQL-клиент GitLab на Rust: набор crate'ов

Дата обращения ко всем источникам: 2026-09-25. Версии взяты из crates.io API
(`https://crates.io/api/v1/crates/<name>`, поле `max_stable_version`) на ту же дату.

Эталон поведения: `src/gitlab.mjs`, `src/browse.mjs`, `test/gitlab.test.mjs`,
`test/browse.test.mjs`. Десктоп вызывает тот же `createClient` без своего `fetch`
(`desktop/app/main.mjs:107`), то есть сейчас запросы идут через Node `fetch`
внутри main-процесса Electron, а не через `net.fetch`.

## 1. Вывод

```toml
[dependencies]
reqwest    = { version = "0.13.5", default-features = false, features = ["rustls", "json"] }
tokio      = { version = "1", features = ["sync"] }          # уже есть через tauri 2.11.6
futures    = "0.3.34"                                          # try_join_all, BoxFuture
serde      = { version = "1.0.229", features = ["derive"] }
serde_json = "1.0.151"
thiserror  = "2.0.21"

[dev-dependencies]
tokio    = { version = "1", features = ["macros", "rt-multi-thread", "time"] }
wiremock = "0.6.5"
```

- HTTP: async `reqwest` 0.13, не `ureq` и не `tauri-plugin-http`.
- TLS: встроенный в `reqwest` 0.13 `rustls` + `rustls-platform-verifier`. Отдельно
  crate'ы TLS подключать не нужно. Корпоративный CA из системного хранилища он
  видит на macOS, Windows и Linux. Node `fetch` сегодня этого **не делает**.
- GraphQL: строковые запросы (`const PIPELINE_QUERY: &str`) + `serde`, как сейчас.
  Без `graphql_client`/`cynic`, без схемы GitLab в репозитории.
- Лимит 4: `tokio::sync::Semaphore` внутри клиента. Downstream-рекурсия: функция,
  которая возвращает `BoxFuture<'_, Result<RawPipeline>>`, плюс `try_join_all`.
- Ошибки: `thiserror`-enum с теми же русскими текстами в `#[error(...)]`. В Tauri
  они сериализуются строкой `to_string()`, так фронт видит то же `err.message`.
- Тесты: трейт `Gql` с одним методом (аналог `gql`-функции, которую тесты уже
  подменяют через `fakeGql`) для логики пагинации и рекурсии. `wiremock` для
  четырёх тестов самого `createClient`.

## 2. HTTP: reqwest или ureq

| | reqwest 0.13.5 | ureq 3.4.2 |
|---|---|---|
| Модель | async на tokio/hyper | «blocking I/O instead of async I/O … keeps dependencies to a minimum» [ureq] |
| TLS по умолчанию | rustls + platform verifier [reqwest-cl] | rustls + webpki-roots: «a static bundle of root certificates that do not update automatically» [ureq] |
| Системный CA | по умолчанию | только с фичей `platform-verifier` [ureq] |
| JSON | фича `json` (serde + serde_json) [reqwest-toml] | фича `json` [ureq] |

Почему reqwest:

- В Tauri уже есть tokio: `tauri` 2.11.6 зависит от `tokio ^1` с фичами
  `rt`, `rt-multi-thread`, `sync`, `fs`, `io-util` [tauri-deps], а async-команды
  выполняются через `async_runtime::spawn` [tauri-cmd]. С `ureq` каждый запрос
  пришлось бы оборачивать в `spawn_blocking` [tauri-rt], а семантика «не больше
  4 одновременно» ушла бы в пул потоков. С reqwest `gql` остаётся обычной
  `async fn`, как сейчас.
- У `ureq` меньше зависимостей (нет hyper/tower): его обязательные зависимости —
  это `base64`, `log`, `percent-encoding`, `ureq-proto`, `utf8-zero` [ureq-deps].
  Но для async-приложения экономия съедается обвязкой `spawn_blocking`.
- Сам `tauri` тянет `reqwest ^0.13` только на Android и iOS
  (`cfg(any(target_os = "android", all(target_vendor = "apple", not(target_os = "macos"))))`)
  [tauri-deps]. На десктопе reqwest будет нашей зависимостью, без дубля.

**`tauri-plugin-http` не брать.** Он переэкспортирует reqwest [tauri-http], но
версия 2.7.0 зависит от `reqwest = "0.12.16"` [tauri-http-toml]. В одном
бинарнике оказались бы 0.12 и 0.13. К тому же плагин нужен ради JS-`fetch` с
allowlist URL в capabilities [tauri-http], а нам JS-`fetch` не нужен: запросы
делает Rust-команда, как сейчас main-процесс.

Минимальные фичи: `default = ["default-tls", "charset", "http2", "system-proxy"]`,
а `default-tls = ["rustls"]` [reqwest-toml]. Берём `default-features = false` +
`["rustls", "json"]`:
- `http2` не нужен: GitLab отвечает и по HTTP/1.1, а без него не тянутся `h2`
  и `hyper/http2` [reqwest-toml].
- `charset` не нужен: GraphQL-ответ идёт в JSON (UTF-8).
- `system-proxy` ставит системные настройки прокси Windows/macOS
  [reqwest-docs]. См. раздел 7.

## 3. TLS и корпоративный CA

### Что делает сейчас Node `fetch`

- По умолчанию используется **bundled** Mozilla CA: «a snapshot of Mozilla CA store
  that is fixed at release time. It is identical on all supported platforms»
  [node-cli, `--use-bundled-ca`]. Системное хранилище не читается, поэтому
  корпоративный CA из Keychain или certmgr **сейчас не виден**.
- `NODE_EXTRA_CA_CERTS=file` добавляет PEM-сертификаты к корням. Переменная
  читается только при старте процесса [node-cli].
- `--use-system-ca` появился в v23.8.0, поддержка вне Windows/macOS — в v23.9.0.
  Переменная `NODE_USE_SYSTEM_CA=1` появилась в v24.6.0 / v22.19.0 [node-cli].
  На macOS учитываются Default и System Keychains с «Always Trust», на Windows —
  Local Machine и Current User, включая Enterprise Trust и Group Policy.
  На Linux читаются файл и каталог OpenSSL (`/etc/ssl/cert.pem`,
  `/etc/ssl/certs`, либо `SSL_CERT_FILE`/`SSL_CERT_DIR`) [node-cli].
- Electron `net.fetch` «using Chromium's network stack. This differs from Node's
  `fetch()`, which uses Node.js's HTTP stack» [electron-net]. Проект использует
  именно Node `fetch`. Работают ли `--use-system-ca` и `NODE_USE_SYSTEM_CA` в Node,
  встроенном в Electron, я не проверял.

### Три варианта в Rust

| Вариант | macOS | Windows | Linux | Источник |
|---|---|---|---|---|
| rustls + webpki-roots 1.0.9 | нет: статический Mozilla-bundle, аналог `--use-bundled-ca` | нет | нет | [ureq], [rpv-readme] |
| **rustls-platform-verifier 0.7.1** (дефолт reqwest 0.13) | да: `Security.framework`, «platform roots and keychain certificates», с revocation | да: «Windows API certificate verification», системное хранилище, с revocation | да: «System CA bundle» через `rustls-native-certs` + `openssl-probe`, проверка — webpki, без CRL | [rpv], [rpv-readme] |
| native-tls 0.2.18 | да: Secure Transport (`security-framework`) | да: SChannel | да: OpenSSL, нужна системная `libssl` | [native-tls] |

**Рекомендация: platform verifier (он и так дефолт reqwest 0.13).** Changelog
0.13.0: «rustls is now the default TLS backend, instead of native-tls», «rustls
roots features removed, rustls-platform-verifier is used by default»
[reqwest-cl]. Фича `rustls` = `__rustls-aws-lc-rs` + `dep:rustls-platform-verifier`
[reqwest-toml]. Перед native-tls у него два преимущества: не нужна OpenSSL на
Linux, и на macOS/Windows решение о доверии принимает сама ОС, со всеми
источниками (dis)trust [rpv-readme].

Оговорки:

1. **Это расхождение с текущим поведением, но в сторону «работает там, где раньше
   падало».** Сегодня self-hosted GitLab с корпоративным CA без
   `NODE_EXTRA_CA_CERTS` падает на TLS. В Rust-версии он заработает. Обратный
   случай (Node доверяет, Rust нет) возможен, только если CA лежит лишь в
   `NODE_EXTRA_CA_CERTS`, а не в системе.
2. **Паритет `NODE_EXTRA_CA_CERTS`.** Если нужен, читать PEM из переменной
   окружения (при старте, как Node) и передавать в
   `ClientBuilder::tls_certs_merge(Certificate::from_pem_bundle(..)?)`
   [reqwest-builder], [reqwest-cert]. В коде reqwest 0.13 непустые
   `root_certs` при `!tls_certs_only` превращаются в
   `rustls_platform_verifier::Verifier::new_with_extra_roots(..)` для
   `all(unix, not(android))` и `windows` [reqwest-src]. macOS попадает в
   `unix`, так что работает везде на десктопе. Не использовать `tls_certs_only`:
   он отключает системные корни [reqwest-builder].
3. **Linux:** системное хранилище читается один раз при старте [rpv-readme].
   `SSL_CERT_FILE` заменяет системный bundle, а не дополняет его
   [rustls-native-certs]. Если bundle не найден, README предлагает дополнить его
   webpki-roots через `new_with_extra_roots` [rpv-readme]. Для десктопа это
   спорно (YAGNI), пока нет жалоб.
4. **macOS:** требуется 10.14+ [rpv].
5. **Сборка:** криптопровайдер по умолчанию — `aws-lc-rs` [reqwest-cl]. Для
   non-FIPS нужен только C/C++-компилятор, CMake не требуется. На Windows нужен
   MSVC Build Tools (он уже нужен Tauri). NASM рекомендован на x86-64, но есть
   `prebuilt-nasm` [aws-lc-win], [aws-lc-req]. Если это мешает CI, можно взять
   `rustls-no-provider` + ring [reqwest-toml].

## 4. GraphQL: строки + serde или кодогенерация

| | Строки + serde (как сейчас) | graphql_client 0.16.0 | cynic 3.14.0 |
|---|---|---|---|
| Схема в репо | не нужна | нужна на этапе компиляции (`schema_path`) [gqlc] | нужна: `use_schema!`/`register_schema` в `build.rs` [cynic] |
| Интеграция с reqwest | своя, ~40 строк | фича `reqwest` требует `reqwest >=0.11, <=0.12` [gqlc-toml], с 0.13 не совместима | фича `http-reqwest` [cynic] |
| Кастомные скаляры (`CiPipelineID`, `Time`) | просто `String` | объявить вручную | `impl_scalar!` [cynic] |

Почему не кодогенерация:

- Схема GitLab большая. Интроспекция gitlab.com
  (`/-/graphql/introspection_result.json`) весит около 16 МБ (замер:
  `curl -w %{size_download}` = 16 349 667 байт). В production интроспекция
  возвращает этот статический файл [gl-start]. В исходниках GitLab схема
  генерируется `bundle exec rake gitlab:graphql:schema:dump` в
  `./tmp/tests/graphql/gitlab_schema.graphql` и не лежит готовым файлом
  [gl-fe].
- Self-hosted GitLab отстаёт от gitlab.com. API «versionless», удаление полей
  идёт после депрекации не менее чем на шесть релизов [gl-api]. Схема в репо
  фиксирует одну версию и даёт ложную уверенность: запрос, который собрался
  против схемы gitlab.com, всё равно может упасть на старом инстансе с
  `errors[]`. Текущий код ловит это рантайм-проверкой, и она остаётся нужна.
- Запросов шесть (`PIPELINE_QUERY`, `LIST_QUERY`, `MR_QUERY`, `PROJECTS_QUERY`,
  `BRANCHES_QUERY`, `PIPELINES_QUERY`).
  Все они уже проверены на живом GitLab. Перенос строк 1:1 — самый прямой
  способ сохранить поведение.

Что даёт serde вместо кодогенерации: `#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]` на структурах ответа. `Option<T>` там, где JS
проверяет `?.` или `null` (`project`, `pipeline`, `mergeRequest.headPipeline`,
`repository`, `commit`, `user`, `duration`, `downstreamPipeline`). Фрагменты
`... on CiBuildNeed { name } ... on CiJob { name }` оба дают `{ name }`, так что
хватит одной структуры `{ name: String }`.

Лимиты GitLab, на которые опирается код: страница не больше 100 узлов,
сложность запроса 250 для аутентифицированных, размер запроса 10 000 символов,
таймаут 30 секунд [gl-api]. Константы `first: 100`, `MAX_JOB_PAGES = 50`,
`MAX_LIST_PAGES = 10`, `PAGE = 20` переносятся как есть.

## 5. Параллельность, рекурсия, пагинация

**Лимит 4.** `tokio::sync::Semaphore::new(4)` в клиенте. Permit берётся на время
одного HTTP-запроса, как `acquire()`/`release()` в `createClient`. Семафор
FIFO: «permits are given out in the order they were requested», а permit
освобождается при drop [tokio-sem]. Это совпадает с очередью `waiting.shift()`
в JS. Нужна фича `sync`, и tauri её уже включает [tauri-deps].

**Рекурсия downstream до глубины 3.** С Rust 1.77 рекурсивные `async fn`
разрешены, если есть индирекция (`Box::pin`) [rust-177], [E0733]. Тауриевская
async-команда требует `Send`-future [tauri-cmd]. Надёжнее сразу вернуть явный
тип:

```rust
fn fetch_pipeline<'a, G: Gql>(gql: &'a G, project: String, id: String, depth: u8)
    -> BoxFuture<'a, Result<RawPipeline, GitlabError>>
{
    Box::pin(async move {
        // do { … } while (after) → loop с тем же лимитом MAX_JOB_PAGES
        let downstream = if depth < MAX_DOWNSTREAM_DEPTH {
            try_join_all(jobs.iter().filter_map(|j| /* ds */).map(|(job_id, p, id)| async move {
                Ok::<_, GitlabError>((job_id, fetch_pipeline(gql, p, id, depth + 1).await?))
            })).await?.into_iter().collect::<HashMap<_, _>>()
        } else { HashMap::new() };
        Ok(RawPipeline { project, pipeline, jobs, downstream })
    })
}
```

- `Promise.all` → `futures::future::try_join_all`: все downstream запускаются
  конкурентно, а реальную параллельность ограничивает семафор в `gql`, как
  сейчас. Разница: JS не отменяет остальные запросы после первой ошибки, а
  `try_join_all` бросает (drop) остальные futures. Результат (первая ошибка)
  тот же.
- `downstream` в JS — объект, ключи вставляются в порядке завершения.
  Потребитель (`src/model.mjs:73`) читает его только по ключу
  `raw.downstream[j.id]`, поэтому `HashMap` без порядка годится.
- `BoxFuture` — из `futures` [futures]. `tokio::task::JoinSet` не нужен: он
  требует `'static`, а значит `Arc` вокруг клиента.

**Пагинация.** `do … while (after)` и `for (page < MAX …)` переносятся в
`loop`/`for` с теми же лимитами и текстами ошибок. Курсор — `Option<String>`,
в variables он сериализуется как `null`, как `after: null` в JS.

## 6. Ошибки

Сейчас это `new Error('...')` с русским текстом. Наружу уходит только
`err.message` (`desktop/app/main.mjs:91,156`). Main-процесс один раз ветвится
по префиксу: `err.message.startsWith('Нет токена для')` (`main.mjs:104`).

Рекомендация: `thiserror` 2.0.21. Он генерирует `Display` из `#[error("...")]`
и «deliberately does not appear in your public API», то есть эквивалентен
ручной реализации [thiserror]. Варианты повторяют тексты 1:1:

```rust
#[derive(Debug, thiserror::Error)]
pub enum GitlabError {
    #[error("GitLab {host} ответил {status}: токен недействителен или нет доступа")] Unauthorized { host: String, status: u16 },
    #[error("GitLab {host} ответил {status}")] Status { host: String, status: u16 },
    #[error("GraphQL {host}: {}", .messages.join("; "))] Graphql { host: String, messages: Vec<String> },
    #[error("Проект {project} на {host} не найден или нет доступа")] ProjectNotFound { project: String, host: String },
    #[error("Нет токена для {host}. Выполни `glab auth login --hostname {host}` или задай GITLAB_TOKEN вместе с GITLAB_HOST={host}")] NoToken { host: String },
    // … остальные тексты из gitlab.mjs/browse.mjs
    #[error(transparent)] Http(#[from] reqwest::Error),
}
impl serde::Serialize for GitlabError { /* serializer.serialize_str(&self.to_string()) */ }
```

- Tauri требует, чтобы ошибка команды реализовывала `serde::Serialize`, и
  предлагает именно `thiserror` + ручной `Serialize` [tauri-cmd]. Строковая
  сериализация сохраняет контракт «фронт получает текст».
- Вариант `NoToken` заменяет `startsWith('Нет токена для')` на `matches!`. Это
  единственное место, где тип окупается. Остальное могло бы быть `String`,
  но enum не дороже.
- Порядок проверок как в `gql`: сначала 401/403, потом `!ok`, потом `errors[]`,
  потом `data`. GitLab на неверный токен отвечает `401` с
  `{"errors":[{"message":"Invalid token"}]}` [gl-api], поэтому 401 проверяется до
  разбора тела, как сейчас. Недоступные ресурсы в запросах приходят как `null`
  без ошибки [gl-api]. На это опираются проверки `project == null`.
- Ошибки транспорта и JSON (`#[from] reqwest::Error`) и сейчас не
  локализованы (текст `fetch failed`/`SyntaxError` от Node). Паритета по
  тексту здесь нет и не было.

## 7. Тестируемость без сети

В JS-тестах есть два слоя:

1. `fakeGql(handler)` подменяет **функцию `gql`** целиком: пагинация,
   рекурсия, `listPipelines`, `mrHeadPipeline`, весь `browse.test.mjs`.
2. Подменённый `fetch` проверяет сам `createClient`: URL, POST, Bearer, тело,
   401, `errors[]`, лимит 4.

Перенос:

- **Слой 1 → трейт `Gql`** с одним `async fn query(&self, q: &str, vars: Value)
  -> Result<Value, GitlabError>` и `fn host(&self) -> &str`. `async fn` в трейтах
  стабильна с Rust 1.75. Такие трейты не object-safe (нет `dyn`), для
  приватных трейтов это допустимо [rust-afit]. Функции берут `G: Gql`
  (дженерик, не `dyn`). Фейк в тестах — структура с замыканием: прямой аналог
  `fakeGql`, тесты переносятся построчно. Это не абстракция «на будущее»: в JS
  этот шов уже есть и используется в 11 из 18 тестов `gitlab`/`browse`. Ещё 4 теста
  (`resolveHost`/`resolveToken`) подменяют `exec`, то есть вызов `glab`. Это
  отдельный шов вне HTTP-клиента.
- **Слой 2 → `wiremock` 0.6.5**: «a background HTTP server on a random local
  port», матчеры `method`, `path`, header, `body_json`, `expect()` с проверкой
  при остановке, `received_requests()` [wiremock]. Он работает на tokio, так
  что рантайм уже есть. Сервер слушает по HTTP, а клиент строит
  `https://{host}/api/graphql`. Поэтому конструктор клиента принимает базовый
  URL (прод — `format!("https://{host}")`, тест — `server.uri()`), а `host()`
  остаётся для текстов ошибок. Это единственное отличие от JS-сигнатуры
  `createClient({ host, token, fetch })`.
  - Тест «не больше 4 одновременно»: `ResponseTemplate::set_delay(..)` и 10
    параллельных запросов. Максимум in-flight надёжнее считать своим `Respond`-
    impl со счётчиком, чем по `received_requests()`.
- `mockito` 1.7.2 умеет то же (async API, JSON-матчеры, параллельные тесты)
  [mockito]. Выбор между ними — вкусовой. `wiremock` сделан под async-first,
  а у `mockito` синхронные методы паникуют внутри рантайма [mockito].
- Трейт над транспортом (над `reqwest`) не нужен: слой 2 покрывает `wiremock`
  на реальном HTTP, а слой 1 — трейт `Gql`.

## 8. Что ещё меняется относительно Node (решить явно)

- **Прокси.** Reqwest по умолчанию читает `HTTP_PROXY`/`HTTPS_PROXY`/`ALL_PROXY`
  из окружения [reqwest-docs]. Node `fetch` делает это только с
  `--use-env-proxy` (v24.5.0 / v22.21.0) [node-cli]. Для строгого паритета —
  `ClientBuilder::no_proxy()` [reqwest-docs]. Для корпоративной сети это,
  скорее всего, желательное расхождение, как и системный CA.
- **Таймауты.** В reqwest `connect_timeout` «Default is `None`», общий
  `timeout` по умолчанию не задан [reqwest-builder]. Серверный лимит GitLab —
  30 секунд [gl-api]. Значение по умолчанию для Node `fetch` в этой работе не
  проверялось.

## Источники

- [reqwest-docs] https://docs.rs/reqwest/0.13.5/reqwest/
- [reqwest-builder] https://docs.rs/reqwest/latest/reqwest/struct.ClientBuilder.html
- [reqwest-cert] https://docs.rs/reqwest/latest/reqwest/tls/struct.Certificate.html
- [reqwest-cl] https://github.com/seanmonstar/reqwest/blob/master/CHANGELOG.md (v0.13.0)
- [reqwest-toml] https://github.com/seanmonstar/reqwest/blob/master/Cargo.toml (`[features]`)
- [reqwest-src] https://github.com/seanmonstar/reqwest/blob/master/src/async_impl/client.rs (ветка `rustls_platform_verifier::Verifier::new_with_extra_roots`)
- [ureq] https://docs.rs/ureq/latest/ureq/
- [ureq-deps] https://crates.io/api/v1/crates/ureq/3.4.2/dependencies
- [rpv] https://docs.rs/rustls-platform-verifier/latest/rustls_platform_verifier/
- [rpv-readme] https://github.com/rustls/rustls-platform-verifier/blob/main/README.md
- [rustls-native-certs] https://docs.rs/rustls-native-certs/latest/rustls_native_certs/
- [native-tls] https://docs.rs/native-tls/latest/native_tls/
- [aws-lc-req] https://aws.github.io/aws-lc-rs/requirements/index.html
- [aws-lc-win] https://aws.github.io/aws-lc-rs/requirements/windows.html
- [node-cli] https://github.com/nodejs/node/blob/main/doc/api/cli.md (`--use-bundled-ca`, `--use-system-ca`, `NODE_EXTRA_CA_CERTS`, `NODE_USE_SYSTEM_CA`, `--use-env-proxy`)
- [electron-net] https://www.electronjs.org/docs/latest/api/net
- [tauri-rt] https://docs.rs/tauri/latest/tauri/async_runtime/index.html
- [tauri-deps] https://crates.io/api/v1/crates/tauri/2.11.6/dependencies
- [tauri-cmd] https://v2.tauri.app/develop/calling-rust/
- [tauri-http] https://v2.tauri.app/plugin/http-client/
- [tauri-http-toml] https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/http/Cargo.toml
- [gqlc] https://docs.rs/graphql_client/latest/graphql_client/
- [gqlc-toml] https://github.com/graphql-rust/graphql-client/blob/main/graphql_client/Cargo.toml
- [cynic] https://docs.rs/cynic/latest/cynic/
- [gl-api] https://docs.gitlab.com/api/graphql/
- [gl-start] https://docs.gitlab.com/api/graphql/getting_started/
- [gl-fe] https://docs.gitlab.com/development/fe_guide/graphql/
- [tokio-sem] https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html
- [rust-177] https://blog.rust-lang.org/2024/03/21/Rust-1.77.0/
- [rust-afit] https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits/
- [E0733] https://doc.rust-lang.org/stable/error_codes/E0733.html
- [futures] https://docs.rs/futures/latest/futures/future/type.BoxFuture.html
- [thiserror] https://docs.rs/thiserror/latest/thiserror/
- [wiremock] https://docs.rs/wiremock/latest/wiremock/
- [mockito] https://docs.rs/mockito/latest/mockito/

//! Вход и сессия (спека `2026-10-12-delivery-variants-design.md`, § 5): `/auth/*`, зашифрованная cookie
//! `longpole_session`, ленивый refresh с in-memory single-flight, сверка `Origin`.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use axum::extract::{Path, Query, Request, State};
use axum::http::{HeaderMap, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::PrivateCookieJar;
use axum_extra::extract::cookie::{Cookie, SameSite};
use longpole_core::error::{Error, ErrorCode};
use longpole_core::web_oauth::{RefreshError, Tokens};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tokio::sync::OnceCell;

use crate::{AppState, HostApp};

const SESSION: &str = "longpole_session";
const PENDING: &str = "longpole_oauth";
const SESSION_AGE: time::Duration = time::Duration::days(30);
const PENDING_AGE: time::Duration = time::Duration::minutes(10);
/// Access обновляется заранее, чтобы не истечь посреди сборки отчёта.
const REFRESH_MARGIN: i64 = 300;
/// Сколько держать результат refresh: запрос со старой cookie получит те же токены, а не `invalid_grant`.
const REFRESH_HOLD: Duration = Duration::from_secs(60);

/// Вход в один хост; в cookie — по хостам, ключ — домен.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct HostSession {
    access: String,
    refresh: String,
    /// unix-секунды
    expires_at: i64,
    user_id: u64,
    pub(crate) login: String,
}

type Sessions = BTreeMap<String, HostSession>;

/// Что живёт между `/auth/login` и callback.
#[derive(Serialize, Deserialize)]
struct Pending {
    host: String,
    state: String,
    verifier: Option<String>,
    return_to: String,
}

/// Single-flight refresh по refresh-токену: GitLab выдаёт его на один раз, второй параллельный refresh
/// получил бы `invalid_grant`. Держится в памяти процесса — отсюда одна реплика (ADR 0002).
#[derive(Default)]
pub(crate) struct Refreshes(Mutex<HashMap<String, Refresh>>);

/// Когда начат refresh и его результат, общий для всех, кто пришёл с тем же refresh-токеном.
type Refresh = (Instant, Arc<OnceCell<Tokens>>);

impl Refreshes {
    async fn run(&self, host: &HostApp, refresh: &str) -> Result<Tokens, RefreshError> {
        let cell = {
            let mut map = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            map.retain(|_, (at, _)| at.elapsed() < REFRESH_HOLD);
            map.entry(refresh.to_owned())
                .or_insert_with(|| (Instant::now(), Arc::default()))
                .1
                .clone()
        };
        // неудача не запоминается: следующий запрос попробует снова
        cell.get_or_try_init(|| async { host.app().refresh(refresh).await })
            .await
            .cloned()
    }
}

fn sessions(jar: &PrivateCookieJar) -> Sessions {
    jar.get(SESSION)
        .and_then(|c| serde_json::from_str(c.value()).ok())
        .unwrap_or_default()
}

/// Логин из cookie, без обновления токена.
pub(crate) fn stored_login(jar: &PrivateCookieJar, domain: &str) -> Option<String> {
    sessions(jar).remove(domain).map(|s| s.login)
}

/// Скользящий `Max-Age`: продлевается при каждой записи; пустая сессия стирает cookie.
fn with_sessions(jar: PrivateCookieJar, sessions: &Sessions) -> PrivateCookieJar {
    if sessions.is_empty() {
        return jar.remove(Cookie::build(SESSION).path("/"));
    }
    let value = serde_json::to_string(sessions).expect("сессия сериализуется");
    jar.add(cookie(SESSION, value, "/", SESSION_AGE))
}

fn cookie(
    name: &'static str,
    value: String,
    path: &'static str,
    age: time::Duration,
) -> Cookie<'static> {
    Cookie::build((name, value))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path(path)
        .max_age(age)
        .build()
}

fn now() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

impl AppState {
    /// Вход в хост с живым access: если до истечения меньше 5 минут — refresh. Нет входа или refresh
    /// отвергнут (`invalid_grant`, 401) — `authRequired`, хост убран из cookie. Возвращённую cookie надо
    /// отдать в ответе.
    pub async fn access(
        &self,
        jar: PrivateCookieJar,
        domain: &str,
    ) -> (PrivateCookieJar, Result<String, Error>) {
        let (jar, session) = self.session(jar, domain).await;
        (jar, session.map(|s| s.access))
    }

    pub(crate) async fn session(
        &self,
        jar: PrivateCookieJar,
        domain: &str,
    ) -> (PrivateCookieJar, Result<HostSession, Error>) {
        let auth_required = || auth_required(domain);
        let mut all = sessions(&jar);
        let (Some(host), Some(session)) = (self.host(domain), all.get(domain).cloned()) else {
            return (jar, Err(auth_required()));
        };
        if session.expires_at - now() > REFRESH_MARGIN {
            return (jar, Ok(session));
        }
        match self.0.refreshes.run(host, &session.refresh).await {
            Ok(tokens) => {
                let session = HostSession {
                    access: tokens.access,
                    refresh: tokens.refresh,
                    expires_at: tokens.expires_at,
                    ..session
                };
                all.insert(domain.to_owned(), session.clone());
                (with_sessions(jar, &all), Ok(session))
            }
            Err(RefreshError::Revoked) => {
                all.remove(domain);
                (with_sessions(jar, &all), Err(auth_required()))
            }
            Err(RefreshError::Failed(e)) => (jar, Err(e)),
        }
    }
}

pub(crate) fn auth_required(domain: &str) -> Error {
    Error::new(ErrorCode::AuthRequired).with("host", domain)
}

/// Провайдер ответил 401 на живой access — вход отозван: хост уходит из cookie.
pub(crate) fn forget(jar: PrivateCookieJar, domain: &str) -> PrivateCookieJar {
    let mut all = sessions(&jar);
    all.remove(domain);
    with_sessions(jar, &all)
}

#[derive(Deserialize)]
pub(crate) struct LoginQuery {
    host: String,
    return_to: Option<String>,
}

/// `GET /auth/login?host=<domain>&return_to=<path>` → 302 к провайдеру.
pub(crate) async fn login(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Query(query): Query<LoginQuery>,
) -> Response {
    let return_to = query.return_to.unwrap_or_else(|| "/".into());
    let Some(host) = state.host(&query.host) else {
        return (StatusCode::BAD_REQUEST, "unknown host").into_response();
    };
    if !is_local_path(&return_to) {
        return (StatusCode::BAD_REQUEST, "return_to: only a relative path").into_response();
    }
    let authorize = match host.app().authorize() {
        Ok(authorize) => authorize,
        Err(e) => return failed(e),
    };
    let pending = Pending {
        host: query.host,
        state: authorize.state,
        verifier: authorize.verifier,
        return_to,
    };
    let value = serde_json::to_string(&pending).expect("сериализуется");
    let jar = jar.add(cookie(PENDING, value, "/auth", PENDING_AGE));
    (jar, found(&authorize.url)).into_response()
}

/// Только путь своего сайта: `//…` и `/\…` браузер понимает как другой хост — open redirect. Управляющие
/// символы браузер из `Location` выкидывает, и `/\t/…` стал бы `//…`.
fn is_local_path(path: &str) -> bool {
    path.starts_with('/')
        && !path.starts_with("//")
        && !path.starts_with("/\\")
        && !path.chars().any(char::is_control)
}

#[derive(Deserialize)]
pub(crate) struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
}

/// `GET /auth/callback/<domain>`: сверяет `state`, меняет код на токены, пишет хост в сессию.
/// Отказ у провайдера (`error=…`, кода нет) — возврат на `return_to` без входа.
pub(crate) async fn callback(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Path(domain): Path<String>,
    Query(query): Query<CallbackQuery>,
) -> Response {
    let pending = jar
        .get(PENDING)
        .and_then(|c| serde_json::from_str::<Pending>(c.value()).ok())
        .filter(|p| p.host == domain && query.state.as_deref() == Some(p.state.as_str()));
    let jar = jar.remove(Cookie::build(PENDING).path("/auth"));
    let (Some(pending), Some(host)) = (pending, state.host(&domain)) else {
        return (StatusCode::BAD_REQUEST, jar, "state mismatch").into_response();
    };
    let Some(code) = query.code else {
        return (jar, found(&pending.return_to)).into_response();
    };
    let app = host.app();
    let signed_in = async {
        let tokens = app.exchange(&code, pending.verifier.as_deref()).await?;
        let user = app.user(&tokens.access).await?;
        Ok::<_, Error>(HostSession {
            access: tokens.access,
            refresh: tokens.refresh,
            expires_at: tokens.expires_at,
            user_id: user.id,
            login: user.login,
        })
    };
    match signed_in.await {
        Ok(session) => {
            let mut all = sessions(&jar);
            all.insert(domain, session);
            (with_sessions(jar, &all), found(&pending.return_to)).into_response()
        }
        Err(e) => (jar, failed(e)).into_response(),
    }
}

#[derive(Deserialize)]
pub(crate) struct LogoutQuery {
    host: String,
}

/// `POST /auth/logout?host=<domain>`: стирает вход только в этот хост. GitLab ещё отзывает токен —
/// в фоне и best effort.
pub(crate) async fn logout(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Query(query): Query<LogoutQuery>,
) -> Response {
    let mut all = sessions(&jar);
    if let Some(session) = all.remove(&query.host) {
        tokio::spawn(async move {
            if let Some(host) = state.host(&query.host) {
                host.app().revoke(&session.access).await;
            }
        });
    }
    (with_sessions(jar, &all), StatusCode::NO_CONTENT).into_response()
}

/// CSRF без токенов (§ 5): `SameSite=Lax` у cookie, а любой не-`GET` — только с `Origin` инстанса,
/// под `/api` ещё и с `Content-Type: application/json` (форму с чужого сайта так не отправить).
pub(crate) async fn same_origin(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let safe = matches!(*request.method(), Method::GET | Method::HEAD);
    if !safe && !allowed(request.headers(), request.uri().path(), &state.0.origin) {
        return StatusCode::FORBIDDEN.into_response();
    }
    next.run(request).await
}

fn allowed(headers: &HeaderMap, path: &str, origin: &str) -> bool {
    let header = |name| headers.get(name).and_then(|v| v.to_str().ok());
    let json = header(header::CONTENT_TYPE)
        .and_then(|v| v.split(';').next())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"));
    header(header::ORIGIN) == Some(origin) && (json || !path.starts_with("/api/"))
}

fn found(location: &str) -> Response {
    (StatusCode::FOUND, [(header::LOCATION, location)]).into_response()
}

/// Ошибка входа: код ядра в лог (без секретов), пользователю — 502.
fn failed(e: Error) -> Response {
    tracing::warn!(code = ?e.code, host = e.params.get("host").map(String::as_str), "вход не удался");
    (StatusCode::BAD_GATEWAY, "sign-in failed").into_response()
}

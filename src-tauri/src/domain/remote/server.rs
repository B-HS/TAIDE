use std::collections::HashMap;

use axum::body::Bytes;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::Router;
use tauri::{AppHandle, Manager};
use tokio::net::TcpListener;
use tokio::sync::watch;

use super::commands::RemoteStore;
use super::types::{
    REMOTE_LINK_TOKEN_QUERY_KEY, REMOTE_LOGIN_NONCE_COOKIE_NAME, REMOTE_LOGIN_NONCE_TTL_MS, REMOTE_LOGIN_PATH, REMOTE_SESSION_COOKIE_NAME,
};
use super::{login_page, service, serving, ws};
use crate::infra::secret::{SecretAccount, SecretStoreState};
use crate::state::AppState;

pub fn build_router(app: AppHandle) -> Router {
    Router::new()
        .route("/__taide/ws", get(ws_upgrade_route))
        .route("/__taide/file", get(serving::file_range))
        .route(REMOTE_LOGIN_PATH, get(login_get_route).post(login_post_route))
        .fallback(serving::serve_static)
        .layer(middleware::from_fn_with_state(app.clone(), auth_middleware))
        .with_state(app)
}

pub async fn serve(listener: TcpListener, router: Router, mut shutdown_rx: watch::Receiver<()>) {
    let shutdown_signal = async move {
        let _ = shutdown_rx.changed().await;
    };

    if let Err(error) = axum::serve(listener, router.into_make_service())
        .with_graceful_shutdown(shutdown_signal)
        .await
    {
        log::warn!("원격 접속 서버가 오류로 종료되었습니다: {error}");
    }
}

fn extract_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';').find_map(|part| {
        let (key, value) = part.trim().split_once('=')?;
        (key == name).then(|| value.to_string())
    })
}

fn extract_link_token(uri: &Uri) -> Option<String> {
    let query = uri.query()?;
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == REMOTE_LINK_TOKEN_QUERY_KEY).then(|| value.to_string())
    })
}

fn build_session_cookie_header(session_token: &str) -> String {
    format!("{REMOTE_SESSION_COOKIE_NAME}={session_token}; HttpOnly; SameSite=Strict; Path=/")
}

fn build_login_nonce_cookie_header(nonce: &str) -> String {
    let max_age_seconds = REMOTE_LOGIN_NONCE_TTL_MS / 1_000;
    format!("{REMOTE_LOGIN_NONCE_COOKIE_NAME}={nonce}; HttpOnly; SameSite=Strict; Path=/; Max-Age={max_age_seconds}")
}

fn clear_login_nonce_cookie_header() -> String {
    format!("{REMOTE_LOGIN_NONCE_COOKIE_NAME}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0")
}

fn append_cookie(response: &mut Response, cookie_header: &str) {
    if let Ok(value) = HeaderValue::from_str(cookie_header) {
        response.headers_mut().append(header::SET_COOKIE, value);
    }
}

/// Best-effort "is this connection plaintext" check for the login page's
/// insecure-connection notice. The server itself never terminates TLS (it
/// only ever binds loopback plain HTTP — `commands.rs`), so this only
/// flips to "secure" when a reverse proxy/tunnel in front of it announces
/// `X-Forwarded-Proto: https`.
fn is_insecure_connection(headers: &HeaderMap) -> bool {
    headers.get("x-forwarded-proto").and_then(|value| value.to_str().ok()) != Some("https")
}

/// Fail-closed: a keyring access error (locked keychain, permission denial,
/// backend outage) must never be treated the same as "no password set" — that
/// would demote `auth_middleware`'s link-only fast path back to the
/// pre-password legacy behavior (instant session on `?t=` alone) whenever the
/// keyring happens to be unreachable. Only a definite `Ok(None)` — keyring
/// reachable, no entry stored — counts as "not configured"; `Ok(Some(_))` and
/// every `Err` both count as "configured" and route through the login form.
fn is_password_configured(app: &AppHandle) -> bool {
    !matches!(app.state::<SecretStoreState>().0.get(SecretAccount::RemoteAccess), Ok(None))
}

async fn auth_middleware(State(app): State<AppHandle>, request: Request, next: Next) -> Response {
    let remote = app.state::<RemoteStore>();

    let origin = request.headers().get(header::ORIGIN).and_then(|value| value.to_str().ok());
    let host = request.headers().get(header::HOST).and_then(|value| value.to_str().ok());
    if !service::is_allowed_origin(origin, host) {
        return (StatusCode::FORBIDDEN, "origin not allowed").into_response();
    }

    if let Some(session_token) = extract_cookie(request.headers(), REMOTE_SESSION_COOKIE_NAME) {
        if remote.has_active_session(&session_token) {
            return next.run(request).await;
        }
    }

    if request.uri().path() == REMOTE_LOGIN_PATH {
        return next.run(request).await;
    }

    let password_configured = is_password_configured(&app);

    if let Some(link_token) = extract_link_token(request.uri()) {
        let Some(nonce) = remote.consume_link_token(&link_token) else {
            return (StatusCode::UNAUTHORIZED, "invalid or expired link").into_response();
        };

        if !password_configured {
            let Some(session_token) = remote.promote_nonce_to_session(&nonce) else {
                return (StatusCode::UNAUTHORIZED, "invalid or expired link").into_response();
            };
            let mut response = next.run(request).await;
            append_cookie(&mut response, &build_session_cookie_header(&session_token));
            return response;
        }

        let mut response = Redirect::to(REMOTE_LOGIN_PATH).into_response();
        append_cookie(&mut response, &build_login_nonce_cookie_header(&nonce));
        return response;
    }

    let password_only_login = app.state::<AppState>().settings.read().remote_password_only_login;
    if password_configured && password_only_login {
        return Redirect::to(REMOTE_LOGIN_PATH).into_response();
    }

    (StatusCode::UNAUTHORIZED, "authentication required").into_response()
}

fn login_response(status: StatusCode, html: String) -> Response {
    (
        status,
        [
            (header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8")),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
            (
                header::CONTENT_SECURITY_POLICY,
                HeaderValue::from_static(login_page::LOGIN_PAGE_CSP),
            ),
        ],
        Html(html),
    )
        .into_response()
}

fn lockout_remaining_seconds(remote: &RemoteStore) -> Option<u64> {
    remote.login_lockout_remaining_ms().map(|remaining_ms| remaining_ms.div_ceil(1_000))
}

async fn login_get_route(State(app): State<AppHandle>, headers: HeaderMap) -> Response {
    if !is_password_configured(&app) {
        return Redirect::to("/").into_response();
    }

    let remote = app.state::<RemoteStore>();
    let language = app.state::<AppState>().settings.read().language.clone();
    let html = login_page::render(login_page::LoginPageParams {
        language: &language,
        failed: false,
        locked_remaining_seconds: lockout_remaining_seconds(&remote),
        insecure: is_insecure_connection(&headers),
    });
    login_response(StatusCode::OK, html)
}

async fn login_post_route(State(app): State<AppHandle>, headers: HeaderMap, body: Bytes) -> Response {
    let origin = headers.get(header::ORIGIN).and_then(|value| value.to_str().ok());
    let Some(origin) = origin else {
        return (StatusCode::FORBIDDEN, "origin required").into_response();
    };
    let host = headers.get(header::HOST).and_then(|value| value.to_str().ok());
    if !service::is_allowed_origin(Some(origin), host) {
        return (StatusCode::FORBIDDEN, "origin not allowed").into_response();
    }

    let remote = app.state::<RemoteStore>();
    let language = app.state::<AppState>().settings.read().language.clone();
    let insecure = is_insecure_connection(&headers);

    if let Some(remaining_seconds) = lockout_remaining_seconds(&remote) {
        let html = login_page::render(login_page::LoginPageParams {
            language: &language,
            failed: false,
            locked_remaining_seconds: Some(remaining_seconds),
            insecure,
        });
        return login_response(StatusCode::TOO_MANY_REQUESTS, html);
    }

    let secret = app.state::<SecretStoreState>();
    let stored_hash = match secret.0.get(SecretAccount::RemoteAccess) {
        Ok(Some(hash)) => hash,
        _ => return (StatusCode::FORBIDDEN, "password not configured").into_response(),
    };

    let password_only_login = app.state::<AppState>().settings.read().remote_password_only_login;
    let nonce_cookie = extract_cookie(&headers, REMOTE_LOGIN_NONCE_COOKIE_NAME);
    if nonce_cookie.is_none() && !password_only_login {
        return (StatusCode::UNAUTHORIZED, "authentication required").into_response();
    }

    let fields = parse_urlencoded_body(&body);
    let candidate_password = fields.get("password").cloned().unwrap_or_default();

    if !service::verify_password(&stored_hash, &candidate_password) {
        remote.record_login_failure();
        let html = login_page::render(login_page::LoginPageParams {
            language: &language,
            failed: true,
            locked_remaining_seconds: lockout_remaining_seconds(&remote),
            insecure,
        });
        return login_response(StatusCode::UNAUTHORIZED, html);
    }

    let session_token = match nonce_cookie {
        Some(nonce) => match remote.promote_nonce_to_session(&nonce) {
            Some(token) => token,
            None => {
                remote.record_login_failure();
                let html = login_page::render(login_page::LoginPageParams {
                    language: &language,
                    failed: true,
                    locked_remaining_seconds: lockout_remaining_seconds(&remote),
                    insecure,
                });
                return login_response(StatusCode::UNAUTHORIZED, html);
            }
        },
        None => remote.issue_session_without_nonce(),
    };

    remote.record_login_success();
    let mut response = Redirect::to("/").into_response();
    append_cookie(&mut response, &build_session_cookie_header(&session_token));
    append_cookie(&mut response, &clear_login_nonce_cookie_header());
    response
}

/// Minimal `application/x-www-form-urlencoded` body parser. axum's `Form`
/// extractor needs the `form` feature, which this crate doesn't enable
/// (`default-features = false, features = ["http1", "tokio", "ws"]` in
/// `Cargo.toml`) — adding it would pull in `serde_urlencoded`/`form_urlencoded`
/// as new direct dependencies, so the login form's single `password` field is
/// decoded by hand instead.
fn parse_urlencoded_body(bytes: &[u8]) -> HashMap<String, String> {
    let raw = String::from_utf8_lossy(bytes);
    raw.split('&')
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            Some((decode_form_component(key), decode_form_component(value)))
        })
        .collect()
}

fn decode_form_component(raw: &str) -> String {
    let mut decoded = Vec::with_capacity(raw.len());
    let mut bytes = raw.bytes();
    while let Some(byte) = bytes.next() {
        match byte {
            b'+' => decoded.push(b' '),
            b'%' => match (bytes.next().and_then(hex_value), bytes.next().and_then(hex_value)) {
                (Some(high), Some(low)) => decoded.push((high << 4) | low),
                _ => decoded.push(b'%'),
            },
            other => decoded.push(other),
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

async fn ws_upgrade_route(State(app): State<AppHandle>, upgrade: WebSocketUpgrade) -> impl IntoResponse {
    upgrade.on_upgrade(move |socket| ws::handle_socket(socket, app))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencoded_바디에서_필드를_읽는다() {
        let fields = parse_urlencoded_body(b"password=hunter2");
        assert_eq!(fields.get("password"), Some(&"hunter2".to_string()));
    }

    #[test]
    fn urlencoded_바디의_퍼센트_인코딩을_해독한다() {
        let fields = parse_urlencoded_body(b"password=a%26b%3Dc");
        assert_eq!(fields.get("password"), Some(&"a&b=c".to_string()));
    }

    #[test]
    fn urlencoded_바디의_플러스는_공백으로_해독한다() {
        let fields = parse_urlencoded_body(b"password=a+b");
        assert_eq!(fields.get("password"), Some(&"a b".to_string()));
    }

    #[test]
    fn x_forwarded_proto가_https이면_보안_연결로_판단한다() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-proto", HeaderValue::from_static("https"));
        assert!(!is_insecure_connection(&headers));
    }

    #[test]
    fn x_forwarded_proto가_없으면_평문_연결로_판단한다() {
        assert!(is_insecure_connection(&HeaderMap::new()));
    }
}

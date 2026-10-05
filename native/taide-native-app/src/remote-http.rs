use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Bytes;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::get;
use futures_util::future::BoxFuture;
use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo, TokioTimer};
use hyper_util::service::TowerToHyperService;
use taide_infra::secret::SecretAccount;
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::remote::RemoteStatus;
use taide_remote::store::RemoteStore;
use taide_remote::types::{
    REMOTE_LINK_TOKEN_QUERY_KEY, REMOTE_LOGIN_NONCE_TTL_MS, REMOTE_LOGIN_PATH,
};
use taide_remote::{login_page, service};
use taide_runtime::AppServices;
use tokio::net::TcpListener;
use tokio::sync::{oneshot, watch};
use tokio::task::JoinSet;

use crate::remote_serving::{self, AssetResolver};

pub const SESSION_COOKIE_NAME: &str = "taide_remote_session";
pub const LOGIN_NONCE_COOKIE_NAME: &str = "taide_remote_login_nonce";
const SHUTDOWN_GRACE: Duration = Duration::from_millis(2_000);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_millis(10_000);
const ACCEPT_RETRY_DELAY: Duration = Duration::from_secs(1);
const MILLIS_PER_SECOND: u64 = 1_000;
const HEX_SHIFT: u32 = 4;
const HEX_RADIX_DIGITS: u8 = 10;

pub type SocketAction =
    Arc<dyn Fn(WebSocket, Arc<AppServices>, String) -> BoxFuture<'static, ()> + Send + Sync>;

pub struct Ports {
    pub socket: SocketAction,
    pub assets: AssetResolver,
}

#[derive(Clone)]
pub(crate) struct Context {
    pub services: Arc<AppServices>,
    pub ports: Arc<Ports>,
}

pub fn build_router(services: Arc<AppServices>, ports: Arc<Ports>) -> Router {
    let context = Context { services, ports };
    Router::new()
        .route("/__taide/ws", get(ws_upgrade_route))
        .route("/__taide/file", get(remote_serving::file_range))
        .route("/__taide/file/{*path}", get(remote_serving::file_path))
        .route(
            REMOTE_LOGIN_PATH,
            get(login_get_route).post(login_post_route),
        )
        .fallback(remote_serving::serve_static)
        .layer(middleware::from_fn_with_state(
            context.clone(),
            auth_middleware,
        ))
        .with_state(context)
}

pub async fn start(services: Arc<AppServices>, ports: Arc<Ports>) -> AppResult<RemoteStatus> {
    if services.remote.is_running() {
        return Ok(services.remote.status());
    }
    let _operation = services
        .tasks
        .begin_operation("native-remote-start")
        .ok_or_else(|| AppError::Internal("remote task supervisor stopped".into()))?;
    if services.state.is_shutting_down() {
        return Err(AppError::Internal(
            "remote server unavailable during shutdown".into(),
        ));
    }
    let listener = TcpListener::bind(("127.0.0.1", 0u16)).await?;
    let port = u32::from(listener.local_addr()?.port());
    let router = build_router(services.clone(), ports);
    let (shutdown_sender, shutdown_receiver) = watch::channel(());
    let (ready_sender, ready_receiver) = oneshot::channel();
    let server_services = services.clone();
    let server = services
        .tasks
        .spawn_transient_handle("native-remote-server", async move {
            if ready_receiver.await.is_err() {
                return;
            }
            accept_loop(server_services, listener, router, shutdown_receiver).await;
        });
    let Some(server) = server else {
        return Err(AppError::Internal("remote task supervisor stopped".into()));
    };
    if services.state.is_shutting_down() {
        server.abort();
        return Err(AppError::Internal(
            "remote server unavailable during shutdown".into(),
        ));
    }
    if !services.remote.mark_started(port, shutdown_sender, server) {
        return Ok(services.remote.status());
    }
    if services.state.is_shutting_down() {
        stop(&services);
        return Err(AppError::Internal(
            "remote server unavailable during shutdown".into(),
        ));
    }
    let status = services.remote.status();
    services
        .events
        .publish(AppEvent::RemoteStateChanged { status });
    if ready_sender.send(()).is_err() {
        stop(&services);
        return Err(AppError::Internal(
            "remote server stopped before accepting connections".into(),
        ));
    }
    Ok(status)
}

async fn accept_loop(
    services: Arc<AppServices>,
    listener: TcpListener,
    router: Router,
    mut shutdown: watch::Receiver<()>,
) {
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            biased;
            _ = shutdown.changed() => break,
            accepted = listener.accept() => {
                let (stream, _) = match accepted {
                    Ok(accepted) => accepted,
                    Err(error) => {
                        if !matches!(error.kind(), std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::ConnectionAborted | std::io::ErrorKind::ConnectionReset) {
                            log::warn!("원격 HTTP accept 오류: {error}");
                            tokio::select! {
                                _ = shutdown.changed() => break,
                                _ = tokio::time::sleep(ACCEPT_RETRY_DELAY) => {}
                            }
                        }
                        continue;
                    }
                };
                let Some(operation) = services.tasks.begin_operation("native-remote-connection") else { break };
                let route = router.clone();
                let mut closed = shutdown.clone();
                connections.spawn(async move {
                    let _operation = operation;
                    let connection = http1::Builder::new().timer(TokioTimer::new()).header_read_timeout(HANDSHAKE_TIMEOUT)
                        .serve_connection(TokioIo::new(stream), TowerToHyperService::new(route)).with_upgrades();
                    tokio::pin!(connection);
                    tokio::select! {
                        result = &mut connection => {
                            if let Err(error) = result { log::warn!("원격 HTTP 연결 종료 오류: {error}"); }
                        }
                        _ = closed.changed() => {
                            connection.as_mut().graceful_shutdown();
                            let _ = tokio::time::timeout(SHUTDOWN_GRACE, &mut connection).await;
                        }
                    }
                });
            }
            _ = connections.join_next(), if !connections.is_empty() => {}
        }
    }
    drop(listener);
    let drain = async { while connections.join_next().await.is_some() {} };
    let _ = tokio::time::timeout(SHUTDOWN_GRACE, drain).await;
    connections.shutdown().await;
}

pub fn stop(services: &AppServices) {
    let Some(mut shutdown) = services.remote.take_shutdown_state() else {
        return;
    };
    if let Some(signal) = shutdown.shutdown_tx.take() {
        let _ = signal.send(());
    }
    if let Some(mut handle) = shutdown.server_handle.take() {
        let abort = handle.abort_handle();
        if !services
            .tasks
            .spawn_transient("native-remote-stop", async move {
                tokio::select! {
                    _ = &mut handle => {}
                    _ = tokio::time::sleep(SHUTDOWN_GRACE) => { handle.abort(); }
                }
            })
        {
            abort.abort();
        }
    }
    services.events.publish(AppEvent::RemoteStateChanged {
        status: RemoteStatus::default(),
    });
}

pub async fn apply_toggle(
    services: Arc<AppServices>,
    ports: Arc<Ports>,
    was_enabled: bool,
    enabled: bool,
) {
    if was_enabled == enabled {
        return;
    }
    if enabled {
        if let Err(error) = start(services, ports).await {
            log::warn!("원격 접속 서버 시작 실패: {error}");
        }
        return;
    }
    stop(&services);
}

pub fn refresh_password_configured_cache(services: &AppServices) {
    let configured = !matches!(
        services.secrets.0.get(SecretAccount::RemoteAccess),
        Ok(None)
    );
    services.remote.set_password_configured(configured);
}

pub(crate) fn extract_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| {
            let (key, value) = part.trim().split_once('=')?;
            (key == name).then(|| value.to_string())
        })
}

fn extract_link_token(uri: &Uri) -> Option<String> {
    uri.query()?.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == REMOTE_LINK_TOKEN_QUERY_KEY).then(|| value.to_string())
    })
}

fn session_cookie(token: &str, secure: bool) -> String {
    let suffix = if secure { "; Secure" } else { "" };
    format!("{SESSION_COOKIE_NAME}={token}; HttpOnly; SameSite=Strict; Path=/{suffix}")
}

fn nonce_cookie(nonce: &str, secure: bool) -> String {
    let seconds = REMOTE_LOGIN_NONCE_TTL_MS / MILLIS_PER_SECOND;
    let suffix = if secure { "; Secure" } else { "" };
    format!(
        "{LOGIN_NONCE_COOKIE_NAME}={nonce}; HttpOnly; SameSite=Strict; Path=/; Max-Age={seconds}{suffix}"
    )
}

fn append_cookie(response: &mut Response, cookie: &str) {
    if let Ok(value) = HeaderValue::from_str(cookie) {
        response.headers_mut().append(header::SET_COOKIE, value);
    }
}

fn is_insecure_connection(host: Option<&str>, allowed: &[String], headers: &HeaderMap) -> bool {
    let Some(host) = host else { return true };
    let hostname = service::host_header_hostname(host);
    if service::is_loopback_hostname(&hostname)
        || !allowed
            .iter()
            .any(|entry| service::host_matches_allowed_entry(&hostname, entry))
    {
        return true;
    }
    headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .is_none_or(|value| !value.trim().eq_ignore_ascii_case("https"))
}

async fn auth_middleware(State(context): State<Context>, request: Request, next: Next) -> Response {
    let services = &context.services;
    let Some(_operation) = services.tasks.begin_operation("native-remote-http") else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let remote = &services.remote;
    let allowed = services.state.settings.read().remote_allowed_hosts.clone();
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok());
    if !service::is_allowed_host(host, &allowed, remote.port()) {
        return (StatusCode::FORBIDDEN, "host not allowed").into_response();
    }
    let origin = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok());
    if request.method() == Method::POST && origin.is_none() {
        return (StatusCode::FORBIDDEN, "origin required").into_response();
    }
    if !service::is_allowed_origin(origin, host) {
        return (StatusCode::FORBIDDEN, "origin not allowed").into_response();
    }
    if extract_cookie(request.headers(), SESSION_COOKIE_NAME)
        .is_some_and(|token| remote.has_active_session(&token))
        || request.uri().path() == REMOTE_LOGIN_PATH
    {
        return next.run(request).await;
    }
    let configured = !matches!(
        services.secrets.0.get(SecretAccount::RemoteAccess),
        Ok(None)
    );
    if let Some(token) = extract_link_token(request.uri()) {
        let Some(nonce) = remote.consume_link_token(&token) else {
            return (StatusCode::UNAUTHORIZED, "invalid or expired link").into_response();
        };
        let secure = !is_insecure_connection(host, &allowed, request.headers());
        if !configured {
            let Some(token) = remote.promote_nonce_to_session(&nonce) else {
                return (StatusCode::UNAUTHORIZED, "invalid or expired link").into_response();
            };
            let mut response = next.run(request).await;
            append_cookie(&mut response, &session_cookie(&token, secure));
            return response;
        }
        let mut response = Redirect::to(REMOTE_LOGIN_PATH).into_response();
        append_cookie(&mut response, &nonce_cookie(&nonce, secure));
        return response;
    }
    if configured && services.state.settings.read().remote_password_only_login {
        return Redirect::to(REMOTE_LOGIN_PATH).into_response();
    }
    (StatusCode::UNAUTHORIZED, "authentication required").into_response()
}

fn login_response(status: StatusCode, html: String) -> Response {
    (
        status,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            ),
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

fn lockout_remaining_seconds(remote: &RemoteStore, has_nonce: bool) -> Option<u64> {
    remote
        .login_lockout_remaining_ms(has_nonce)
        .map(|value| value.div_ceil(MILLIS_PER_SECOND))
}

async fn login_get_route(State(context): State<Context>, headers: HeaderMap) -> Response {
    let services = &context.services;
    if matches!(
        services.secrets.0.get(SecretAccount::RemoteAccess),
        Ok(None)
    ) {
        return Redirect::to("/").into_response();
    }
    let settings = services.state.settings.read().clone();
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok());
    let has_nonce = extract_cookie(&headers, LOGIN_NONCE_COOKIE_NAME).is_some();
    login_response(
        StatusCode::OK,
        login_page::render(login_page::LoginPageParams {
            language: &settings.language,
            failed: false,
            link_expired: false,
            locked_remaining_seconds: lockout_remaining_seconds(&services.remote, has_nonce),
            insecure: is_insecure_connection(host, &settings.remote_allowed_hosts, &headers),
        }),
    )
}

async fn login_post_route(
    State(context): State<Context>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let services = &context.services;
    let remote = &services.remote;
    let settings = services.state.settings.read().clone();
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok());
    let insecure = is_insecure_connection(host, &settings.remote_allowed_hosts, &headers);
    let nonce = extract_cookie(&headers, LOGIN_NONCE_COOKIE_NAME);
    let has_nonce = nonce.is_some();
    if nonce.is_none() && !settings.remote_password_only_login {
        return (StatusCode::UNAUTHORIZED, "authentication required").into_response();
    }
    let page = |status, failed, expired, locked| {
        login_response(
            status,
            login_page::render(login_page::LoginPageParams {
                language: &settings.language,
                failed,
                link_expired: expired,
                locked_remaining_seconds: locked,
                insecure,
            }),
        )
    };
    if nonce
        .as_ref()
        .is_some_and(|nonce| !remote.has_pending_nonce(nonce))
    {
        return page(StatusCode::UNAUTHORIZED, false, true, None);
    }
    if let Some(seconds) = lockout_remaining_seconds(remote, has_nonce) {
        return page(StatusCode::TOO_MANY_REQUESTS, false, false, Some(seconds));
    }
    let stored = match services.secrets.0.get(SecretAccount::RemoteAccess) {
        Ok(Some(hash)) => hash,
        _ => return (StatusCode::FORBIDDEN, "password not configured").into_response(),
    };
    let fields = parse_urlencoded_body(&body);
    let candidate = fields.get("password").cloned().unwrap_or_default();
    if !service::verify_password(&stored, &candidate) {
        remote.record_login_failure(has_nonce);
        return page(
            StatusCode::UNAUTHORIZED,
            true,
            false,
            lockout_remaining_seconds(remote, has_nonce),
        );
    }
    let token = match nonce {
        Some(nonce) => match remote.promote_nonce_to_session(&nonce) {
            Some(token) => token,
            None => return page(StatusCode::UNAUTHORIZED, false, true, None),
        },
        None => remote.issue_session_without_nonce(),
    };
    remote.record_login_success(has_nonce);
    let mut response = Redirect::to("/").into_response();
    append_cookie(&mut response, &session_cookie(&token, !insecure));
    append_cookie(
        &mut response,
        &format!("{LOGIN_NONCE_COOKIE_NAME}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0"),
    );
    response
}

fn parse_urlencoded_body(bytes: &[u8]) -> HashMap<String, String> {
    String::from_utf8_lossy(bytes)
        .split('&')
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
            b'%' => match (
                bytes.next().and_then(hex_value),
                bytes.next().and_then(hex_value),
            ) {
                (Some(high), Some(low)) => decoded.push((high << HEX_SHIFT) | low),
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
        b'a'..=b'f' => Some(byte - b'a' + HEX_RADIX_DIGITS),
        b'A'..=b'F' => Some(byte - b'A' + HEX_RADIX_DIGITS),
        _ => None,
    }
}

struct SocketTask(tokio::task::JoinHandle<()>);

impl Drop for SocketTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn ws_upgrade_route(
    State(context): State<Context>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Response {
    let Some(token) = extract_cookie(&headers, SESSION_COOKIE_NAME) else {
        return (StatusCode::UNAUTHORIZED, "session required").into_response();
    };
    let digest = service::digest_hex(&token);
    let Some(operation) = context
        .services
        .tasks
        .begin_operation("native-remote-upgrade")
    else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    upgrade.on_upgrade(move |socket| async move {
        let _operation = operation;
        let services = context.services.clone();
        let Some(task) =
            context
                .services
                .tasks
                .spawn_transient_handle("native-remote-socket", async move {
                    (context.ports.socket)(socket, services, digest).await;
                })
        else {
            return;
        };
        let mut task = SocketTask(task);
        drop((&mut task.0).await);
    })
}

#[cfg(test)]
#[path = "remote-http-tests.rs"]
mod tests;

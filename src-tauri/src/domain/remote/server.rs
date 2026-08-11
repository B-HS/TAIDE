use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use tauri::{AppHandle, Manager};
use tokio::net::TcpListener;
use tokio::sync::watch;

use super::commands::RemoteStore;
use super::types::{REMOTE_LINK_TOKEN_QUERY_KEY, REMOTE_SESSION_COOKIE_NAME};
use super::{service, serving, ws};

pub fn build_router(app: AppHandle) -> Router {
    Router::new()
        .route("/__taide/ws", get(ws_upgrade_route))
        .route("/__taide/file", get(serving::file_range))
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

fn extract_session_cookie(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        (name == REMOTE_SESSION_COOKIE_NAME).then(|| value.to_string())
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

async fn auth_middleware(State(app): State<AppHandle>, request: Request, next: Next) -> Response {
    let remote = app.state::<RemoteStore>();

    let origin = request.headers().get(header::ORIGIN).and_then(|value| value.to_str().ok());
    let host = request.headers().get(header::HOST).and_then(|value| value.to_str().ok());
    if !service::is_allowed_origin(origin, host) {
        return (StatusCode::FORBIDDEN, "origin not allowed").into_response();
    }

    if let Some(session_token) = extract_session_cookie(request.headers()) {
        if remote.has_active_session(&session_token) {
            return next.run(request).await;
        }
    }

    let Some(link_token) = extract_link_token(request.uri()) else {
        return (StatusCode::UNAUTHORIZED, "authentication required").into_response();
    };
    let Some(session_token) = remote.consume_link_token(&link_token) else {
        return (StatusCode::UNAUTHORIZED, "invalid or expired link").into_response();
    };

    let mut response = next.run(request).await;
    if let Ok(cookie_value) = HeaderValue::from_str(&build_session_cookie_header(&session_token)) {
        response.headers_mut().append(header::SET_COOKIE, cookie_value);
    }
    response
}

async fn ws_upgrade_route(State(app): State<AppHandle>, upgrade: WebSocketUpgrade) -> impl IntoResponse {
    upgrade.on_upgrade(move |socket| ws::handle_socket(socket, app))
}

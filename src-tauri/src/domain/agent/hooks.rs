use taide_runtime::{agent_actions, agent_hook_reconcile, agent_hook_server, TaskSupervisor};
use tauri::{AppHandle, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use super::commands::{self, AgentHooksStore, AgentStore, HooksServerInfo};
use super::service;
use super::types::{
    AGENT_NAME_CLAUDE, HOOKS_AGENT_QUERY_KEY, HOOKS_HTTP_PATH, HOOKS_READ_TIMEOUT_MS, HOOKS_TOKEN_QUERY_KEY, MAX_HOOKS_REQUEST_BYTES,
};
use crate::error::{AppError, AppResult};
use crate::infra::home;
use crate::platform::event_sink::TauriEventSink;
use crate::state::AppState;

pub async fn ensure_hooks_server_started(app: &AppHandle) -> AppResult<HooksServerInfo> {
    let store = app.state::<AgentHooksStore>();
    let tasks = app.state::<TaskSupervisor>();
    agent_hook_server::start_hooks_server(
        &store,
        &tasks,
        || async {
            let listener = TcpListener::bind("127.0.0.1:0").await.map_err(AppError::from)?;
            let port = listener.local_addr().map_err(AppError::from)?.port();
            let token = uuid::Uuid::new_v4().simple().to_string();
            Ok((HooksServerInfo { port, token }, listener))
        },
        |listener| {
            let app_handle = app.clone();
            tasks.spawn_transient_handle("agent-hooks-accept", async move {
                loop {
                    let Ok((stream, _)) = listener.accept().await else {
                        break;
                    };
                    let connection_app = app_handle.clone();
                    app_handle
                        .state::<TaskSupervisor>()
                        .spawn_transient("agent-hooks-connection", async move {
                            let _ = handle_connection(stream, connection_app).await;
                        });
                }
            })
        },
        || app.state::<AppState>().is_shutting_down(),
    )
    .await
}

pub fn stop_hooks_server(app: &AppHandle) {
    let store = app.state::<AgentHooksStore>();
    agent_hook_server::stop_hooks_server(&store);
}

/// Reconciles a flipped `agent_hooks_enabled` settings value — installs hooks into every open
/// project when the toggle turns on, uninstalls them and stops the hooks server when it turns
/// off, no-op when the value didn't change. Registered into
/// `settings::commands::SettingsToggleObservers` by `lib.rs`'s assembly so the settings domain
/// never calls into this one directly (audit R5#6, T1-I §1.4).
pub async fn apply_agent_hooks_toggle(app: &AppHandle, was_enabled: bool, enabled: bool) {
    let state = app.state::<AppState>();
    let tasks = app.state::<TaskSupervisor>();
    agent_hook_reconcile::apply_agent_hooks_toggle(
        &state,
        &tasks,
        was_enabled,
        enabled,
        agent_hook_reconcile::AgentHookReconcilePorts::new(
            home::home_dir_env,
            || commands::resolve_claude_hook_emitter(&tasks),
            || ensure_hooks_server_started(app),
            commands::TAIDE_CLI_TARGET_PATH,
        ),
        || stop_hooks_server(app),
    )
    .await;
}

pub async fn reconcile_installed_hooks(app: &AppHandle) {
    let state = app.state::<AppState>();
    let tasks = app.state::<TaskSupervisor>();
    agent_hook_reconcile::reconcile_installed_hooks(
        &state,
        &tasks,
        agent_hook_reconcile::AgentHookReconcilePorts::new(
            home::home_dir_env,
            || commands::resolve_claude_hook_emitter(&tasks),
            || ensure_hooks_server_started(app),
            commands::TAIDE_CLI_TARGET_PATH,
        ),
    )
    .await;
}

pub async fn uninstall_hooks_from_open_projects(app: &AppHandle) {
    let state = app.state::<AppState>();
    let tasks = app.state::<TaskSupervisor>();
    agent_hook_reconcile::uninstall_hooks_from_open_projects(&state, &tasks, home::home_dir_env).await;
}

pub use taide_runtime::agent_hook_reconcile::remove_taide_hooks_from_roots;

pub use taide_agent::service::build_hook_url;

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|window| window == b"\r\n\r\n")
}

fn request_path(request_line: &str) -> Option<&str> {
    let path_and_query = request_line.split_whitespace().nth(1)?;
    Some(path_and_query.split('?').next().unwrap_or(path_and_query))
}

fn parse_query_param(request_line: &str, key: &str) -> Option<String> {
    let path_and_query = request_line.split_whitespace().nth(1)?;
    let (_, query) = path_and_query.split_once('?')?;
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(name, _)| *name == key)
        .map(|(_, value)| value.to_string())
}

fn content_length(header_text: &str) -> usize {
    header_text
        .split("\r\n")
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        .unwrap_or(0)
        .min(MAX_HOOKS_REQUEST_BYTES)
}

async fn read_request(stream: &mut TcpStream) -> std::io::Result<(String, Vec<u8>)> {
    let mut buf = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];

    let header_end = loop {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Ok((String::new(), Vec::new()));
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = find_header_end(&buf) {
            break pos;
        }
        if buf.len() > MAX_HOOKS_REQUEST_BYTES {
            return Ok((String::new(), Vec::new()));
        }
    };

    let header_text = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let body_len = content_length(&header_text);
    let body_start = header_end + 4;

    while buf.len() < body_start + body_len {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }

    let body = buf.get(body_start..(body_start + body_len).min(buf.len())).unwrap_or(&[]).to_vec();
    Ok((header_text, body))
}

async fn write_response(stream: &mut TcpStream, status_line: &str) -> std::io::Result<()> {
    let response = format!("{status_line}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    stream.write_all(response.as_bytes()).await
}

async fn handle_connection(mut stream: TcpStream, app: AppHandle) -> std::io::Result<()> {
    let read = tokio::time::timeout(std::time::Duration::from_millis(HOOKS_READ_TIMEOUT_MS), read_request(&mut stream)).await;
    let Ok(read) = read else {
        return Ok(());
    };
    let (header_text, body) = read?;
    let Some(request_line) = header_text.split("\r\n").next().filter(|line| !line.is_empty()) else {
        return Ok(());
    };

    let store = app.state::<AgentHooksStore>();
    let Some(expected_token) = store.server_info().map(|info| info.token) else {
        return write_response(&mut stream, "HTTP/1.1 503 Service Unavailable").await;
    };

    let is_valid_path = request_path(request_line) == Some(HOOKS_HTTP_PATH);
    let provided_token = parse_query_param(request_line, HOOKS_TOKEN_QUERY_KEY).unwrap_or_default();
    let is_valid_token = service::constant_time_eq(provided_token.as_bytes(), expected_token.as_bytes());

    if !is_valid_path || !is_valid_token {
        return write_response(&mut stream, "HTTP/1.1 403 Forbidden").await;
    }

    let agent_name = parse_query_param(request_line, HOOKS_AGENT_QUERY_KEY).unwrap_or_else(|| AGENT_NAME_CLAUDE.to_string());

    let Ok(payload) = serde_json::from_slice::<service::HookPayload>(&body) else {
        return write_response(&mut stream, "HTTP/1.1 400 Bad Request").await;
    };

    apply_hook_payload(&app, &agent_name, &payload);
    write_response(&mut stream, "HTTP/1.1 200 OK").await
}

fn apply_hook_payload(app: &AppHandle, agent_name: &str, payload: &service::HookPayload) {
    let state = app.state::<AppState>();
    let agents = app.state::<AgentStore>();
    let agent_hooks = app.state::<AgentHooksStore>();
    let tasks = app.state::<TaskSupervisor>();
    agent_actions::apply_hook_payload(&state, &agents, &agent_hooks, &TauriEventSink(app), &tasks, agent_name, payload);
}

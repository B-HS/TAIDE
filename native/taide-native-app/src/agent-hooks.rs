use std::sync::{Arc, OnceLock};
use std::time::Duration;

use taide_agent::constants::{
    AGENT_NAME_CLAUDE, CLAUDE_VERSION_TIMEOUT_SECONDS, HOOKS_AGENT_QUERY_KEY, HOOKS_HTTP_PATH,
    HOOKS_READ_TIMEOUT_MS, HOOKS_TOKEN_QUERY_KEY, MAX_HOOKS_REQUEST_BYTES,
};
use taide_agent::service::{self, HookEmitter};
use taide_agent::store::HooksServerInfo;
use taide_model::app_event::AppEvent;
use taide_model::error::AppResult;
use taide_runtime::{
    AppServices, EventSink, TaskSupervisor, agent_actions, agent_hook_reconcile, agent_hook_server,
    agent_probe,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const LOOPBACK_BIND: &str = "127.0.0.1:0";
const READ_CHUNK_BYTES: usize = 4096;
const HEADER_SEPARATOR: &[u8] = b"\r\n\r\n";
const CLAUDE_VERSION_FLAG: &str = "--version";
#[cfg(unix)]
pub(crate) const TAIDE_CLI_TARGET_PATH: &str = "/usr/local/bin/taide";
#[cfg(windows)]
pub(crate) const TAIDE_CLI_TARGET_PATH: &str = "C:/Program Files/TAIDE/bin/taide.exe";

struct HookEvents(Arc<dyn EventSink>);

impl EventSink for HookEvents {
    fn publish(&self, event: AppEvent) {
        self.0.publish(event);
    }
}

pub async fn ensure_started(services: Arc<AppServices>) -> AppResult<HooksServerInfo> {
    agent_hook_server::start_hooks_server(
        &services.agent_hooks,
        &services.tasks,
        || async {
            let listener = TcpListener::bind(LOOPBACK_BIND).await?;
            let port = listener.local_addr()?.port();
            let token = uuid::Uuid::new_v4().simple().to_string();
            Ok((HooksServerInfo { port, token }, listener))
        },
        |listener| {
            let accepted_services = services.clone();
            services
                .tasks
                .spawn_transient_handle("native-agent-hooks-accept", async move {
                    loop {
                        let Ok((stream, _)) = listener.accept().await else {
                            break;
                        };
                        let connection_services = accepted_services.clone();
                        accepted_services.tasks.spawn_transient(
                            "native-agent-hooks-connection",
                            async move {
                                let _ = handle_connection(stream, connection_services).await;
                            },
                        );
                    }
                })
        },
        || services.state.is_shutting_down(),
    )
    .await
}

pub fn stop(services: &AppServices) {
    agent_hook_server::stop_hooks_server(&services.agent_hooks);
}

pub async fn apply_toggle(services: Arc<AppServices>, was_enabled: bool, enabled: bool) {
    apply_toggle_with_home(
        services,
        was_enabled,
        enabled,
        taide_infra::home::home_dir_env,
    )
    .await;
}

async fn apply_toggle_with_home(
    services: Arc<AppServices>,
    was_enabled: bool,
    enabled: bool,
    resolve_home: impl FnOnce() -> Option<String>,
) {
    agent_hook_reconcile::apply_agent_hooks_toggle(
        &services.state,
        &services.tasks,
        was_enabled,
        enabled,
        agent_hook_reconcile::AgentHookReconcilePorts::new(
            resolve_home,
            || resolve_claude_hook_emitter(&services.tasks),
            || ensure_started(services.clone()),
            TAIDE_CLI_TARGET_PATH,
        ),
        || stop(&services),
    )
    .await;
}

pub async fn reconcile_installed(services: Arc<AppServices>) {
    agent_hook_reconcile::reconcile_installed_hooks(
        &services.state,
        &services.tasks,
        agent_hook_reconcile::AgentHookReconcilePorts::new(
            taide_infra::home::home_dir_env,
            || resolve_claude_hook_emitter(&services.tasks),
            || ensure_started(services.clone()),
            TAIDE_CLI_TARGET_PATH,
        ),
    )
    .await;
}

pub(crate) async fn resolve_claude_hook_emitter(tasks: &TaskSupervisor) -> HookEmitter {
    static EMITTER: OnceLock<HookEmitter> = OnceLock::new();
    agent_probe::resolve_claude_hook_emitter(
        tasks,
        &EMITTER,
        Duration::from_secs(CLAUDE_VERSION_TIMEOUT_SECONDS),
        || {
            std::process::Command::new(AGENT_NAME_CLAUDE)
                .arg(CLAUDE_VERSION_FLAG)
                .output()
                .map(|output| output.stdout)
        },
    )
    .await
}

fn request_path(request_line: &str) -> Option<&str> {
    let path_and_query = request_line.split_whitespace().nth(1)?;
    Some(path_and_query.split('?').next().unwrap_or(path_and_query))
}

fn query_param(request_line: &str, key: &str) -> Option<String> {
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
    let mut buffer = Vec::with_capacity(READ_CHUNK_BYTES);
    let mut chunk = [0u8; READ_CHUNK_BYTES];
    let header_end = loop {
        let count = stream.read(&mut chunk).await?;
        if count == 0 {
            return Ok((String::new(), Vec::new()));
        }
        buffer.extend_from_slice(&chunk[..count]);
        if let Some(position) = buffer
            .windows(HEADER_SEPARATOR.len())
            .position(|window| window == HEADER_SEPARATOR)
        {
            break position;
        }
        if buffer.len() > MAX_HOOKS_REQUEST_BYTES {
            return Ok((String::new(), Vec::new()));
        }
    };
    let header_text = String::from_utf8_lossy(&buffer[..header_end]).to_string();
    let body_length = content_length(&header_text);
    let body_start = header_end + HEADER_SEPARATOR.len();
    while buffer.len() < body_start + body_length {
        let count = stream.read(&mut chunk).await?;
        if count == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..count]);
    }
    let body = buffer
        .get(body_start..(body_start + body_length).min(buffer.len()))
        .unwrap_or(&[])
        .to_vec();
    Ok((header_text, body))
}

async fn write_response(stream: &mut TcpStream, status_line: &str) -> std::io::Result<()> {
    stream
        .write_all(
            format!("{status_line}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes(),
        )
        .await
}

async fn handle_connection(
    mut stream: TcpStream,
    services: Arc<AppServices>,
) -> std::io::Result<()> {
    let Ok(read) = tokio::time::timeout(
        Duration::from_millis(HOOKS_READ_TIMEOUT_MS),
        read_request(&mut stream),
    )
    .await
    else {
        return Ok(());
    };
    let (headers, body) = read?;
    let Some(request_line) = headers.split("\r\n").next().filter(|line| !line.is_empty()) else {
        return Ok(());
    };
    let Some(expected_token) = services.agent_hooks.server_info().map(|info| info.token) else {
        return write_response(&mut stream, "HTTP/1.1 503 Service Unavailable").await;
    };
    let provided_token = query_param(request_line, HOOKS_TOKEN_QUERY_KEY).unwrap_or_default();
    if request_path(request_line) != Some(HOOKS_HTTP_PATH)
        || !service::constant_time_eq(provided_token.as_bytes(), expected_token.as_bytes())
    {
        return write_response(&mut stream, "HTTP/1.1 403 Forbidden").await;
    }
    let agent = query_param(request_line, HOOKS_AGENT_QUERY_KEY)
        .unwrap_or_else(|| AGENT_NAME_CLAUDE.to_string());
    let Ok(payload) = serde_json::from_slice::<service::HookPayload>(&body) else {
        return write_response(&mut stream, "HTTP/1.1 400 Bad Request").await;
    };
    agent_actions::apply_hook_payload(
        &services.state,
        &services.agents,
        &services.agent_hooks,
        &HookEvents(services.events.clone()),
        &services.tasks,
        &agent,
        &payload,
    );
    write_response(&mut stream, "HTTP/1.1 200 OK").await
}

#[cfg(test)]
#[path = "agent-hooks-tests.rs"]
mod tests;

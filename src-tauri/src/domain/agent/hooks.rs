use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use super::commands::{self, AgentHooksStore, AgentStore, HooksServerInfo};
use super::service;
use super::types::{HOOKS_HTTP_PATH, HOOKS_READ_TIMEOUT_MS, HOOKS_TOKEN_QUERY_KEY, HOOKS_URL_MARKER, MAX_HOOKS_REQUEST_BYTES};
use crate::error::{AppError, AppResult};
use crate::events::AgentStateChanged;
use crate::state::AppState;

pub async fn ensure_hooks_server_started(app: &AppHandle) -> AppResult<HooksServerInfo> {
    let store = app.state::<AgentHooksStore>();
    if let Some(info) = store.server_info() {
        return Ok(info);
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(AppError::from)?;
    let port = listener.local_addr().map_err(AppError::from)?.port();
    let token = uuid::Uuid::new_v4().simple().to_string();
    let info = HooksServerInfo { port, token };

    let app_handle = app.clone();
    let accept_handle = tauri::async_runtime::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            let connection_app = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                let _ = handle_connection(stream, connection_app).await;
            });
        }
    });
    store.set_server(info.clone(), accept_handle);

    Ok(info)
}

pub fn stop_hooks_server(app: &AppHandle) {
    if let Some(handle) = app.state::<AgentHooksStore>().take_server() {
        handle.abort();
    }
}

pub async fn reconcile_installed_hooks(app: &AppHandle) {
    let is_enabled = app.state::<AppState>().settings.read().agent_hooks_enabled;
    if !is_enabled {
        return;
    }

    let Ok(server) = ensure_hooks_server_started(app).await else {
        return;
    };
    let hook_url = build_hook_url(&server);

    let roots: Vec<String> = {
        let state = app.state::<AppState>();
        let guard = state.projects.read();
        guard.values().map(|project| project.root.clone()).collect()
    };

    for root in roots {
        let Ok(value) = commands::read_settings_local(&root) else {
            continue;
        };
        if !service::has_taide_hook_entries(&value) || service::has_hook_entries_for_url(&value, &hook_url) {
            continue;
        }
        let updated = service::inject_taide_hook_entries(value, &hook_url);
        if let Err(error) = commands::write_settings_local(&root, &updated) {
            log::warn!("hooks URL 갱신 실패 ({root}): {error}");
        }
    }
}

pub async fn uninstall_hooks_from_open_projects(app: &AppHandle) {
    let roots: Vec<String> = {
        let state = app.state::<AppState>();
        let guard = state.projects.read();
        guard.values().map(|project| project.root.clone()).collect()
    };
    remove_taide_hooks_from_roots(&roots);
}

pub fn remove_taide_hooks_from_roots(roots: &[String]) {
    for root in roots {
        let Ok(value) = commands::read_settings_local(root) else {
            continue;
        };
        if !service::has_taide_hook_entries(&value) {
            continue;
        }
        let updated = service::remove_taide_hook_entries(value);
        if let Err(error) = commands::write_settings_local(root, &updated) {
            log::warn!("hooks 제거 실패 ({root}): {error}");
        }
    }
}

pub fn build_hook_url(info: &HooksServerInfo) -> String {
    format!(
        "http://127.0.0.1:{}{HOOKS_HTTP_PATH}?{HOOKS_TOKEN_QUERY_KEY}={}&{HOOKS_URL_MARKER}",
        info.port, info.token
    )
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|window| window == b"\r\n\r\n")
}

fn request_path(request_line: &str) -> Option<&str> {
    let path_and_query = request_line.split_whitespace().nth(1)?;
    Some(path_and_query.split('?').next().unwrap_or(path_and_query))
}

fn parse_query_token(request_line: &str) -> Option<String> {
    let path_and_query = request_line.split_whitespace().nth(1)?;
    let (_, query) = path_and_query.split_once('?')?;
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| *key == HOOKS_TOKEN_QUERY_KEY)
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
    let provided_token = parse_query_token(request_line).unwrap_or_default();
    let is_valid_token = service::constant_time_eq(provided_token.as_bytes(), expected_token.as_bytes());

    if !is_valid_path || !is_valid_token {
        return write_response(&mut stream, "HTTP/1.1 403 Forbidden").await;
    }

    let Ok(payload) = serde_json::from_slice::<service::HookPayload>(&body) else {
        return write_response(&mut stream, "HTTP/1.1 400 Bad Request").await;
    };

    apply_hook_payload(&app, &payload);
    write_response(&mut stream, "HTTP/1.1 200 OK").await
}

fn apply_hook_payload(app: &AppHandle, payload: &service::HookPayload) {
    let Some(activity) = service::map_hook_event_to_activity(&payload.hook_event_name) else {
        return;
    };

    let projects: Vec<_> = {
        let state = app.state::<AppState>();
        let guard = state.projects.read();
        guard.iter().map(|(id, project)| (id.clone(), project.root.clone())).collect()
    };

    let Some(project_id) = service::match_project_by_cwd(&payload.cwd, &projects).cloned() else {
        return;
    };

    let agents = app.state::<AgentStore>();
    let agent_hooks = app.state::<AgentHooksStore>();
    agent_hooks.set_project_override(project_id.clone(), activity);

    let current = agents.agents_for(&project_id);
    if current.is_empty() {
        return;
    }

    let updated: Vec<_> = current
        .into_iter()
        .map(|mut agent| {
            if service::is_hook_managed_agent(&agent.name) {
                agent.activity = activity;
            }
            agent
        })
        .collect();

    if let Some(changed) = agents.diff(&project_id, &updated) {
        let _ = AgentStateChanged {
            project_id,
            agents: changed,
        }
        .emit(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_project_root(name: &str) -> String {
        let dir = std::env::temp_dir().join(format!("taide-hooks-uninstall-test-{name}-{}", uuid::Uuid::new_v4()));
        dir.to_string_lossy().to_string()
    }

    #[test]
    fn 훅_비활성화시_taide_항목만_제거하고_사용자_항목은_보존한다() {
        let root = make_project_root("keep-user-hooks");

        let existing = serde_json::json!({
            "hooks": {
                "Stop": [{ "hooks": [{ "type": "command", "command": "echo done" }] }]
            }
        });
        let injected = service::inject_taide_hook_entries(existing, "http://127.0.0.1:9999/claude/hook?token=abc&taide=1");
        commands::write_settings_local(&root, &injected).expect("write settings");

        remove_taide_hooks_from_roots(std::slice::from_ref(&root));

        let after = commands::read_settings_local(&root).expect("read settings");
        assert!(!service::has_taide_hook_entries(&after));
        let stop_entries = after["hooks"]["Stop"].as_array().expect("stop entries");
        assert_eq!(stop_entries.len(), 1);
        assert_eq!(stop_entries[0]["hooks"][0]["command"], "echo done");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn taide_항목이_없는_프로젝트는_건드리지_않는다() {
        let root = make_project_root("no-taide-hooks");

        let existing = serde_json::json!({
            "hooks": {
                "Stop": [{ "hooks": [{ "type": "command", "command": "echo done" }] }]
            }
        });
        commands::write_settings_local(&root, &existing).expect("write settings");

        remove_taide_hooks_from_roots(std::slice::from_ref(&root));

        let after = commands::read_settings_local(&root).expect("read settings");
        assert_eq!(after, existing);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 존재하지_않는_프로젝트가_섞여도_나머지는_계속_처리한다() {
        let missing_root = make_project_root("missing-parent-remains-absent");
        let real_root = make_project_root("real-project");

        let injected = service::inject_taide_hook_entries(serde_json::json!({}), "http://127.0.0.1:9999/claude/hook?token=abc&taide=1");
        commands::write_settings_local(&real_root, &injected).expect("write settings");

        remove_taide_hooks_from_roots(&[missing_root.clone(), real_root.clone()]);

        let after = commands::read_settings_local(&real_root).expect("read settings");
        assert!(!service::has_taide_hook_entries(&after));

        std::fs::remove_dir_all(&real_root).ok();
    }
}

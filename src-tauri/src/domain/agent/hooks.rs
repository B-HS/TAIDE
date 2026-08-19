use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use super::commands::{self, AgentHooksStore, AgentStore, HooksServerInfo};
use super::service;
use super::types::{
    AGENT_NAME_CLAUDE, AGENT_NAME_CODEX, AGENT_NAME_GEMINI, HOOKS_AGENT_QUERY_KEY, HOOKS_HTTP_PATH, HOOKS_READ_TIMEOUT_MS,
    HOOKS_TOKEN_QUERY_KEY, HOOKS_URL_MARKER, MAX_HOOKS_REQUEST_BYTES,
};
#[cfg(test)]
use super::types::{CODEX_MANAGED_HOOK_EVENTS, GEMINI_MANAGED_HOOK_EVENTS};
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

/// Reconciles a flipped `agent_hooks_enabled` settings value — installs hooks into every open
/// project when the toggle turns on, uninstalls them and stops the hooks server when it turns
/// off, no-op when the value didn't change. Registered into
/// `settings::commands::SettingsToggleObservers` by `lib.rs`'s assembly so the settings domain
/// never calls into this one directly (audit R5#6, T1-I §1.4).
pub async fn apply_agent_hooks_toggle(app: &AppHandle, was_enabled: bool, enabled: bool) {
    if was_enabled == enabled {
        return;
    }
    if enabled {
        reconcile_installed_hooks(app).await;
    } else {
        uninstall_hooks_from_open_projects(app).await;
        stop_hooks_server(app);
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
    let hook_url = build_hook_url(&server, AGENT_NAME_CLAUDE);

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

    reconcile_user_level_hooks(&server, commands::home_dir_env().as_deref());
}

fn reconcile_user_level_hooks(server: &HooksServerInfo, home_env: Option<&str>) {
    for agent_name in [AGENT_NAME_CODEX, AGENT_NAME_GEMINI] {
        let Ok(path) = service::user_level_hooks_path(agent_name, home_env) else {
            continue;
        };
        let Ok(value) = commands::read_user_level_hooks(&path) else {
            continue;
        };
        if !service::has_taide_marker_anywhere(&value) {
            continue;
        }

        let hook_url = build_hook_url(server, agent_name);
        let command = service::build_command_hook_shell_command(commands::TAIDE_CLI_TARGET_PATH, &hook_url);
        let events = service::managed_hook_events_for(agent_name);
        if service::has_command_hook_entries_for_command(&value, events, &command) {
            continue;
        }

        let timeout = service::user_level_hook_command_timeout(agent_name);
        let updated = service::inject_taide_command_hook_entries(value, events, &command, timeout);
        if let Err(error) = commands::write_user_level_hooks(&path, &updated) {
            log::warn!("hooks URL 갱신 실패 (사용자 레벨, {agent_name}): {error}");
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
    remove_taide_hooks_from_user_level_files(commands::home_dir_env().as_deref());
}

fn remove_taide_hooks_from_user_level_files(home_env: Option<&str>) {
    for agent_name in [AGENT_NAME_CODEX, AGENT_NAME_GEMINI] {
        let Ok(path) = service::user_level_hooks_path(agent_name, home_env) else {
            continue;
        };
        let Ok(value) = commands::read_user_level_hooks(&path) else {
            log::warn!("hooks 제거 실패 (사용자 레벨, {agent_name}): 읽기 실패");
            continue;
        };
        if !service::has_taide_marker_anywhere(&value) {
            continue;
        }
        let events = service::managed_hook_events_for(agent_name);
        let updated = service::remove_taide_command_hook_entries(value, events);
        if let Err(error) = commands::write_user_level_hooks(&path, &updated) {
            log::warn!("hooks 제거 실패 (사용자 레벨, {agent_name}): {error}");
        }
    }
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

pub fn build_hook_url(info: &HooksServerInfo, agent_name: &str) -> String {
    format!(
        "http://127.0.0.1:{}{HOOKS_HTTP_PATH}?{HOOKS_TOKEN_QUERY_KEY}={}&{HOOKS_AGENT_QUERY_KEY}={agent_name}&{HOOKS_URL_MARKER}",
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
    if !service::is_hook_managed_agent(agent_name) {
        return;
    }
    let Some(activity) = service::map_hook_event_to_activity(agent_name, &payload.hook_event_name) else {
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
    agent_hooks.set_project_override(project_id.clone(), agent_name.to_string(), activity);

    let current = agents.agents_for(&project_id);
    if current.is_empty() {
        return;
    }

    let updated: Vec<_> = current
        .into_iter()
        .map(|mut agent| {
            if agent.name == agent_name {
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

    fn make_fake_home(name: &str) -> String {
        let dir = std::env::temp_dir().join(format!("taide-hooks-user-level-test-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create fake home");
        dir.to_string_lossy().to_string()
    }

    #[test]
    fn 훅_비활성화시_codex_gemini_사용자_레벨_taide_항목만_제거하고_사용자_항목은_보존한다() {
        let home = make_fake_home("keep-user-hooks");

        let codex_path = service::user_level_hooks_path(AGENT_NAME_CODEX, Some(&home)).unwrap();
        let codex_existing = serde_json::json!({
            "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "echo codex-user-hook" }] }] }
        });
        let codex_injected = service::inject_taide_command_hook_entries(
            codex_existing,
            CODEX_MANAGED_HOOK_EVENTS,
            "taide hook --url http://127.0.0.1:9999/claude/hook?token=abc&agent=codex&taide=1",
            5,
        );
        commands::write_user_level_hooks(&codex_path, &codex_injected).expect("write codex hooks");

        let gemini_path = service::user_level_hooks_path(AGENT_NAME_GEMINI, Some(&home)).unwrap();
        let gemini_injected = service::inject_taide_command_hook_entries(
            serde_json::json!({}),
            GEMINI_MANAGED_HOOK_EVENTS,
            "taide hook --url http://127.0.0.1:9999/claude/hook?token=abc&agent=gemini&taide=1",
            5_000,
        );
        commands::write_user_level_hooks(&gemini_path, &gemini_injected).expect("write gemini settings");

        remove_taide_hooks_from_user_level_files(Some(&home));

        let codex_after = commands::read_user_level_hooks(&codex_path).expect("read codex hooks");
        assert!(!service::has_taide_marker_anywhere(&codex_after));
        let stop_entries = codex_after["hooks"]["Stop"].as_array().expect("stop entries");
        assert_eq!(stop_entries.len(), 1);
        assert_eq!(stop_entries[0]["hooks"][0]["command"], "echo codex-user-hook");

        let gemini_after = commands::read_user_level_hooks(&gemini_path).expect("read gemini settings");
        assert!(gemini_after.get("hooks").is_none());

        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn taide_항목이_없는_사용자_레벨_파일은_건드리지_않는다() {
        let home = make_fake_home("no-taide-hooks");
        let codex_path = service::user_level_hooks_path(AGENT_NAME_CODEX, Some(&home)).unwrap();
        let existing = serde_json::json!({ "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "echo done" }] }] } });
        commands::write_user_level_hooks(&codex_path, &existing).expect("write codex hooks");

        remove_taide_hooks_from_user_level_files(Some(&home));

        let after = commands::read_user_level_hooks(&codex_path).expect("read codex hooks");
        assert_eq!(after, existing);

        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn 사용자_레벨_파일이_존재하지_않아도_패닉없이_넘어간다() {
        let home = make_fake_home("missing-files");
        remove_taide_hooks_from_user_level_files(Some(&home));
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn 재부팅으로_포트가_바뀌면_사용자_레벨_command_hook을_새_url로_재주입한다() {
        let home = make_fake_home("stale-port-heals");
        let codex_path = service::user_level_hooks_path(AGENT_NAME_CODEX, Some(&home)).unwrap();
        let stale_command = service::build_command_hook_shell_command(
            commands::TAIDE_CLI_TARGET_PATH,
            "http://127.0.0.1:9999/claude/hook?token=old&agent=codex&taide=1",
        );
        let existing = service::inject_taide_command_hook_entries(serde_json::json!({}), CODEX_MANAGED_HOOK_EVENTS, &stale_command, 5);
        commands::write_user_level_hooks(&codex_path, &existing).expect("write codex hooks");

        let server = HooksServerInfo {
            port: 10000,
            token: "new-token".to_string(),
        };
        reconcile_user_level_hooks(&server, Some(&home));

        let after = commands::read_user_level_hooks(&codex_path).expect("read codex hooks");
        let fresh_command =
            service::build_command_hook_shell_command(commands::TAIDE_CLI_TARGET_PATH, &build_hook_url(&server, AGENT_NAME_CODEX));
        assert!(service::has_command_hook_entries_for_command(
            &after,
            CODEX_MANAGED_HOOK_EVENTS,
            &fresh_command
        ));

        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn 이미_최신_url이면_재주입하지_않고_그대로_둔다() {
        let home = make_fake_home("already-fresh");
        let gemini_path = service::user_level_hooks_path(AGENT_NAME_GEMINI, Some(&home)).unwrap();
        let server = HooksServerInfo {
            port: 20000,
            token: "current-token".to_string(),
        };
        let fresh_command =
            service::build_command_hook_shell_command(commands::TAIDE_CLI_TARGET_PATH, &build_hook_url(&server, AGENT_NAME_GEMINI));
        let existing = service::inject_taide_command_hook_entries(serde_json::json!({}), GEMINI_MANAGED_HOOK_EVENTS, &fresh_command, 5_000);
        commands::write_user_level_hooks(&gemini_path, &existing).expect("write gemini settings");

        reconcile_user_level_hooks(&server, Some(&home));

        let after = commands::read_user_level_hooks(&gemini_path).expect("read gemini settings");
        assert_eq!(after, existing);

        std::fs::remove_dir_all(&home).ok();
    }
}

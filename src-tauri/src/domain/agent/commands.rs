use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use parking_lot::Mutex;
use tauri::State;

use super::hooks;
use super::service;
use super::types::{AgentActivity, AgentHooksStatus, CliInstallStatus, DetectedAgent, HookInstallScope, ProjectAgents};
use crate::domain::terminal::commands::TerminalStore;
use crate::error::{AppError, AppResult};
use crate::ids::ProjectId;
use crate::state::AppState;

#[cfg(unix)]
pub(super) const TAIDE_CLI_TARGET_PATH: &str = "/usr/local/bin/taide";
#[cfg(windows)]
pub(super) const TAIDE_CLI_TARGET_PATH: &str = "C:/Program Files/TAIDE/bin/taide.exe";

#[derive(Default)]
struct AgentStoreInner {
    agents: HashMap<ProjectId, Vec<DetectedAgent>>,
    wait_markers: HashSet<String>,
    activity_last_active: HashMap<String, Instant>,
}

#[derive(Default)]
pub struct AgentStore(Mutex<AgentStoreInner>);

fn last_known_activity(inner: &AgentStoreInner, session_id: &str) -> AgentActivity {
    inner
        .agents
        .values()
        .flatten()
        .find(|agent| agent.session_id == session_id)
        .map(|agent| agent.activity)
        .unwrap_or(AgentActivity::Unknown)
}

impl AgentStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn diff(&self, project_id: &ProjectId, current: &[DetectedAgent]) -> Option<Vec<DetectedAgent>> {
        let mut guard = self.0.lock();
        let previous = guard.agents.entry(project_id.clone()).or_default();
        if service::agents_changed(previous, current) {
            *previous = current.to_vec();
            Some(current.to_vec())
        } else {
            None
        }
    }

    pub fn agents_for(&self, project_id: &ProjectId) -> Vec<DetectedAgent> {
        self.0.lock().agents.get(project_id).cloned().unwrap_or_default()
    }

    pub fn compute_activity(&self, session_id: &str, probe_active: bool) -> AgentActivity {
        let mut guard = self.0.lock();
        if probe_active {
            guard.activity_last_active.insert(session_id.to_string(), Instant::now());
            return AgentActivity::Working;
        }
        let ms_since_active = guard
            .activity_last_active
            .get(session_id)
            .map(|instant| instant.elapsed().as_millis() as u64);
        let previous = last_known_activity(&guard, session_id);
        service::classify_activity(previous, ms_since_active)
    }

    pub fn prune_activity(&self, valid_session_ids: &HashSet<String>) {
        self.0
            .lock()
            .activity_last_active
            .retain(|session_id, _| valid_session_ids.contains(session_id));
    }

    pub fn register_wait_marker(&self, marker: String) {
        self.0.lock().wait_markers.insert(marker);
    }

    pub fn forget_wait_marker(&self, marker: &str) {
        self.0.lock().wait_markers.remove(marker);
    }

    pub fn take_all_markers(&self) -> Vec<String> {
        std::mem::take(&mut self.0.lock().wait_markers).into_iter().collect()
    }
}

#[derive(Clone)]
pub struct HooksServerInfo {
    pub port: u16,
    pub token: String,
}

#[derive(Default)]
struct AgentHooksStoreInner {
    server: Option<HooksServerInfo>,
    accept_handle: Option<tauri::async_runtime::JoinHandle<()>>,
    project_overrides: HashMap<(ProjectId, String), (AgentActivity, Instant)>,
}

#[derive(Default)]
pub struct AgentHooksStore(Mutex<AgentHooksStoreInner>);

impl AgentHooksStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn server_info(&self) -> Option<HooksServerInfo> {
        self.0.lock().server.clone()
    }

    pub fn set_server(&self, info: HooksServerInfo, accept_handle: tauri::async_runtime::JoinHandle<()>) {
        let mut guard = self.0.lock();
        guard.server = Some(info);
        guard.accept_handle = Some(accept_handle);
    }

    /// 서버 정보·accept 핸들을 회수하고 남은 hook override 도 버린다(배지 오염 방지).
    pub fn take_server(&self) -> Option<tauri::async_runtime::JoinHandle<()>> {
        let mut guard = self.0.lock();
        guard.server = None;
        guard.project_overrides.clear();
        guard.accept_handle.take()
    }

    pub fn set_project_override(&self, project_id: ProjectId, agent_name: String, activity: AgentActivity) {
        self.0
            .lock()
            .project_overrides
            .insert((project_id, agent_name), (activity, Instant::now()));
    }

    pub fn clear_project_override(&self, project_id: &ProjectId, agent_name: &str) {
        self.0
            .lock()
            .project_overrides
            .remove(&(project_id.clone(), agent_name.to_string()));
    }

    pub fn fresh_project_override(&self, project_id: &ProjectId, agent_name: &str) -> Option<AgentActivity> {
        let guard = self.0.lock();
        let key = (project_id.clone(), agent_name.to_string());
        let (activity, set_at) = guard.project_overrides.get(&key)?;
        if set_at.elapsed().as_millis() as u64 >= super::types::HOOK_OVERRIDE_STALE_MS {
            return None;
        }
        Some(*activity)
    }
}

pub fn cleanup_all_wait_markers(store: &AgentStore) {
    let temp_dir = std::env::temp_dir();
    for marker in store.take_all_markers() {
        if let Ok(path) = service::validate_wait_marker_path(&marker, &temp_dir) {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn ensure_project_open(state: &AppState, project_id: &ProjectId) -> AppResult<()> {
    if state.projects.read().contains_key(project_id) {
        return Ok(());
    }
    Err(AppError::NotFound(format!("project not open: {project_id}")))
}

fn project_root(state: &AppState, project_id: &ProjectId) -> AppResult<String> {
    state
        .projects
        .read()
        .get(project_id)
        .map(|project| project.root.clone())
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))
}

#[cfg(unix)]
struct ProcessInfo {
    comm: String,
    state: Option<char>,
    cmdline: String,
}

#[cfg(unix)]
fn resolve_process_info(pid: u32) -> Option<ProcessInfo> {
    let output = std::process::Command::new("ps")
        .args(["-o", "comm=,state=,args=", "-p", &pid.to_string()])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut parts = trimmed.split_whitespace();
    let comm = parts.next()?.to_string();
    let state = parts.next().and_then(|token| token.chars().next());
    let cmdline = parts.collect::<Vec<_>>().join(" ");
    Some(ProcessInfo { comm, state, cmdline })
}

#[cfg(windows)]
fn process_snapshot(system: &sysinfo::System) -> Vec<service::ProcessSnapshot> {
    system
        .processes()
        .values()
        .map(|process| service::ProcessSnapshot {
            pid: process.pid().as_u32(),
            parent_pid: process.parent().map(|pid| pid.as_u32()),
            name: service::strip_windows_exe_suffix(&process.name().to_string_lossy()).to_string(),
            cmdline: process
                .cmd()
                .iter()
                .map(|part| part.to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join(" "),
        })
        .collect()
}

#[cfg(windows)]
fn windows_activity_state(status: sysinfo::ProcessStatus) -> char {
    match status {
        sysinfo::ProcessStatus::Run => 'R',
        _ => 'S',
    }
}

#[cfg(unix)]
pub fn detect_agents_for_pids(pids: Vec<(String, u32)>) -> Vec<service::DetectedAgentProbe> {
    pids.into_iter()
        .filter_map(|(session_id, pid)| {
            let info = resolve_process_info(pid)?;
            let name = service::detect_agent_name(&info.comm, &info.cmdline)?;
            Some(service::DetectedAgentProbe {
                session_id,
                name: name.to_string(),
                pid,
                state: info.state,
            })
        })
        .collect()
}

#[cfg(windows)]
pub fn detect_agents_for_pids(pids: Vec<(String, u32)>) -> Vec<service::DetectedAgentProbe> {
    if pids.is_empty() {
        return Vec::new();
    }

    let system = sysinfo::System::new_all();
    let snapshot = process_snapshot(&system);

    pids.into_iter()
        .filter_map(|(session_id, shell_pid)| {
            let (agent_pid, name) = service::find_descendant_agent(&snapshot, shell_pid)?;
            let state = system
                .process(sysinfo::Pid::from_u32(agent_pid))
                .map(|process| windows_activity_state(process.status()));
            Some(service::DetectedAgentProbe {
                session_id,
                name: name.to_string(),
                pid: agent_pid,
                state,
            })
        })
        .collect()
}

pub async fn detect_agents_for_pids_blocking(pids: Vec<(String, u32)>) -> AppResult<Vec<service::DetectedAgentProbe>> {
    tauri::async_runtime::spawn_blocking(move || detect_agents_for_pids(pids))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))
}

pub fn resolve_activity(
    agents: &AgentStore,
    hooks_store: &AgentHooksStore,
    project_id: &ProjectId,
    probe: &service::DetectedAgentProbe,
) -> AgentActivity {
    let probe_active = service::is_probe_active(probe.state);
    let heuristic = agents.compute_activity(&probe.session_id, probe_active);

    if !service::is_hook_managed_agent(&probe.name) {
        return heuristic;
    }
    if probe_active {
        hooks_store.clear_project_override(project_id, &probe.name);
        return heuristic;
    }
    hooks_store.fresh_project_override(project_id, &probe.name).unwrap_or(heuristic)
}

pub fn build_detected_agents(
    agents: &AgentStore,
    hooks_store: &AgentHooksStore,
    project_id: &ProjectId,
    probes: Vec<service::DetectedAgentProbe>,
) -> Vec<DetectedAgent> {
    probes
        .into_iter()
        .map(|probe| {
            let activity = resolve_activity(agents, hooks_store, project_id, &probe);
            DetectedAgent {
                session_id: probe.session_id,
                name: probe.name,
                pid: probe.pid,
                activity,
            }
        })
        .collect()
}

fn resolve_cli_install_status() -> CliInstallStatus {
    let target = Path::new(TAIDE_CLI_TARGET_PATH);
    match std::fs::symlink_metadata(target) {
        Ok(_) => {
            let resolved = std::fs::canonicalize(target).ok().map(|path| path.to_string_lossy().to_string());
            service::build_cli_install_status(TAIDE_CLI_TARGET_PATH, true, resolved)
        }
        Err(_) => service::build_cli_install_status(TAIDE_CLI_TARGET_PATH, false, None),
    }
}

fn settings_local_path(root: &str) -> PathBuf {
    Path::new(root).join(".claude").join("settings.local.json")
}

fn read_json_file_or_empty_object(path: &Path) -> AppResult<serde_json::Value> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(serde_json::json!({})),
        Err(error) => Err(AppError::from(error)),
    }
}

/// hook URL 에 토큰이 들어가므로 소유자 전용 권한으로 기록한다.
fn write_json_file_private_atomic(path: &Path, value: &serde_json::Value) -> AppResult<()> {
    let text = serde_json::to_string_pretty(value)?;
    crate::infra::persist::write_private_atomic(path, text.as_bytes())
}

pub(super) fn read_settings_local(root: &str) -> AppResult<serde_json::Value> {
    read_json_file_or_empty_object(&settings_local_path(root))
}

pub(super) fn write_settings_local(root: &str, value: &serde_json::Value) -> AppResult<()> {
    write_json_file_private_atomic(&settings_local_path(root), value)
}

pub(super) fn read_user_level_hooks(path: &Path) -> AppResult<serde_json::Value> {
    read_json_file_or_empty_object(path)
}

pub(super) fn write_user_level_hooks(path: &Path, value: &serde_json::Value) -> AppResult<()> {
    write_json_file_private_atomic(path, value)
}

pub(super) fn home_dir_env() -> Option<String> {
    std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).ok()
}

fn resolve_user_level_hooks_installed(agent_name: &str) -> AppResult<bool> {
    let home = home_dir_env();
    let path = service::user_level_hooks_path(agent_name, home.as_deref())?;
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let value: serde_json::Value = serde_json::from_str(&text)?;
            Ok(service::has_taide_marker_anywhere(&value))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(AppError::from(error)),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn agent_list(
    state: State<'_, AppState>,
    terminals: State<'_, TerminalStore>,
    agents: State<'_, AgentStore>,
    agent_hooks: State<'_, AgentHooksStore>,
    project_id: ProjectId,
) -> AppResult<ProjectAgents> {
    ensure_project_open(&state, &project_id)?;

    let pids = terminals.foreground_pids(&project_id);
    let probes = detect_agents_for_pids_blocking(pids).await?;

    let detected = build_detected_agents(&agents, &agent_hooks, &project_id, probes);
    Ok(ProjectAgents {
        project_id,
        agents: detected,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn agent_release_marker(state: State<'_, AppState>, agents: State<'_, AgentStore>, marker: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let path = service::validate_wait_marker_path(&marker, &std::env::temp_dir())?;

    let result = match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::from(error)),
    };
    agents.forget_wait_marker(&marker);
    result
}

#[tauri::command]
#[specta::specta]
pub async fn agent_cli_status() -> AppResult<CliInstallStatus> {
    Ok(resolve_cli_install_status())
}

#[tauri::command]
#[specta::specta]
pub async fn agent_hooks_status(state: State<'_, AppState>, project_id: ProjectId, agent_name: String) -> AppResult<AgentHooksStatus> {
    let scope = service::hook_scope_for_agent(&agent_name)?;
    let installed = match scope {
        HookInstallScope::Project => {
            let root = project_root(&state, &project_id)?;
            let value = read_settings_local(&root)?;
            service::has_taide_hook_entries(&value)
        }
        HookInstallScope::User => resolve_user_level_hooks_installed(&agent_name)?,
    };
    Ok(AgentHooksStatus {
        agent_name,
        scope,
        installed,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn agent_hooks_install(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    agent_name: String,
) -> AppResult<AgentHooksStatus> {
    let scope = service::hook_scope_for_agent(&agent_name)?;
    match scope {
        HookInstallScope::Project => {
            if !state.settings.read().agent_hooks_enabled {
                return Err(AppError::InvalidArgument("agent hooks are disabled in settings".to_string()));
            }
            let root = project_root(&state, &project_id)?;
            let server = hooks::ensure_hooks_server_started(&app).await?;
            let hook_url = hooks::build_hook_url(&server, &agent_name);
            let value = read_settings_local(&root)?;
            let value = service::inject_taide_hook_entries(value, &hook_url);
            write_settings_local(&root, &value)?;
            Ok(AgentHooksStatus {
                agent_name,
                scope,
                installed: true,
            })
        }
        HookInstallScope::User => {
            if !state.settings.read().agent_hooks_enabled {
                return Err(AppError::InvalidArgument("agent hooks are disabled in settings".to_string()));
            }
            if !resolve_cli_install_status().installed {
                return Err(AppError::InvalidArgument("taide CLI is not installed yet".to_string()));
            }
            let server = hooks::ensure_hooks_server_started(&app).await?;
            let hook_url = hooks::build_hook_url(&server, &agent_name);
            let command = service::build_command_hook_shell_command(TAIDE_CLI_TARGET_PATH, &hook_url);
            let events = service::managed_hook_events_for(&agent_name);
            let timeout = service::user_level_hook_command_timeout(&agent_name);
            let path = service::user_level_hooks_path(&agent_name, home_dir_env().as_deref())?;
            let value = read_user_level_hooks(&path)?;
            let value = service::inject_taide_command_hook_entries(value, events, &command, timeout);
            write_user_level_hooks(&path, &value)?;
            Ok(AgentHooksStatus {
                agent_name,
                scope,
                installed: true,
            })
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn agent_hooks_uninstall(state: State<'_, AppState>, project_id: ProjectId, agent_name: String) -> AppResult<AgentHooksStatus> {
    let scope = service::hook_scope_for_agent(&agent_name)?;
    match scope {
        HookInstallScope::Project => {
            let root = project_root(&state, &project_id)?;
            let value = read_settings_local(&root)?;
            if service::has_taide_hook_entries(&value) {
                let value = service::remove_taide_hook_entries(value);
                write_settings_local(&root, &value)?;
            }
        }
        HookInstallScope::User => {
            let path = service::user_level_hooks_path(&agent_name, home_dir_env().as_deref())?;
            let value = read_user_level_hooks(&path)?;
            if service::has_taide_marker_anywhere(&value) {
                let events = service::managed_hook_events_for(&agent_name);
                let value = service::remove_taide_command_hook_entries(value, events);
                write_user_level_hooks(&path, &value)?;
            }
        }
    }
    Ok(AgentHooksStatus {
        agent_name,
        scope,
        installed: false,
    })
}

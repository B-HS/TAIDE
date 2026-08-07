use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use parking_lot::Mutex;
use tauri::State;

use super::hooks;
use super::service;
use super::types::{AgentActivity, AgentHooksStatus, CliInstallStatus, DetectedAgent, ProjectAgents};
use crate::domain::terminal::commands::TerminalStore;
use crate::error::{AppError, AppResult};
use crate::ids::ProjectId;
use crate::state::AppState;

#[cfg(unix)]
const TAIDE_CLI_TARGET_PATH: &str = "/usr/local/bin/taide";
#[cfg(windows)]
const TAIDE_CLI_TARGET_PATH: &str = "C:/Program Files/TAIDE/bin/taide.exe";

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
    project_overrides: HashMap<ProjectId, (AgentActivity, Instant)>,
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

    pub fn set_server(&self, info: HooksServerInfo) {
        self.0.lock().server = Some(info);
    }

    pub fn set_project_override(&self, project_id: ProjectId, activity: AgentActivity) {
        self.0.lock().project_overrides.insert(project_id, (activity, Instant::now()));
    }

    pub fn fresh_project_override(&self, project_id: &ProjectId) -> Option<AgentActivity> {
        let guard = self.0.lock();
        let (activity, set_at) = guard.project_overrides.get(project_id)?;
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

pub fn resolve_activity(
    agents: &AgentStore,
    hooks_store: &AgentHooksStore,
    project_id: &ProjectId,
    probe: &service::DetectedAgentProbe,
) -> AgentActivity {
    if let Some(activity) = hooks_store.fresh_project_override(project_id) {
        return activity;
    }
    agents.compute_activity(&probe.session_id, service::is_probe_active(probe.state))
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

fn read_settings_local(root: &str) -> AppResult<serde_json::Value> {
    let path = settings_local_path(root);
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(serde_json::json!({})),
        Err(error) => Err(AppError::from(error)),
    }
}

fn write_settings_local(root: &str, value: &serde_json::Value) -> AppResult<()> {
    let path = settings_local_path(root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(value)?;
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, text)?;
    std::fs::rename(&tmp_path, &path)?;
    Ok(())
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
    let probes = tauri::async_runtime::spawn_blocking(move || detect_agents_for_pids(pids))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;

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
pub async fn agent_hooks_status(state: State<'_, AppState>, project_id: ProjectId) -> AppResult<AgentHooksStatus> {
    let root = project_root(&state, &project_id)?;
    let value = read_settings_local(&root)?;
    Ok(AgentHooksStatus {
        installed: service::has_taide_hook_entries(&value),
    })
}

#[tauri::command]
#[specta::specta]
pub async fn agent_hooks_install(app: tauri::AppHandle, state: State<'_, AppState>, project_id: ProjectId) -> AppResult<AgentHooksStatus> {
    if !state.settings.read().agent_hooks_enabled {
        return Err(AppError::InvalidArgument("agent hooks are disabled in settings".to_string()));
    }
    let root = project_root(&state, &project_id)?;
    let server = hooks::ensure_hooks_server_started(&app).await?;
    let hook_url = hooks::build_hook_url(&server);
    let value = read_settings_local(&root)?;
    let value = service::inject_taide_hook_entries(value, &hook_url);
    write_settings_local(&root, &value)?;
    Ok(AgentHooksStatus { installed: true })
}

#[tauri::command]
#[specta::specta]
pub async fn agent_hooks_uninstall(state: State<'_, AppState>, project_id: ProjectId) -> AppResult<AgentHooksStatus> {
    let root = project_root(&state, &project_id)?;
    let value = read_settings_local(&root)?;
    let value = service::remove_taide_hook_entries(value);
    write_settings_local(&root, &value)?;
    Ok(AgentHooksStatus { installed: false })
}

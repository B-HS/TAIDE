use std::collections::{HashMap, HashSet};
use std::path::Path;

use parking_lot::Mutex;
use tauri::State;

use super::service;
use super::types::{CliInstallStatus, DetectedAgent, ProjectAgents};
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
}

#[derive(Default)]
pub struct AgentStore(Mutex<AgentStoreInner>);

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

#[cfg(unix)]
fn resolve_process_info(pid: u32) -> Option<(String, String)> {
    let output = std::process::Command::new("ps")
        .args(["-o", "comm=,args=", "-p", &pid.to_string()])
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

    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let comm = parts.next().unwrap_or_default().to_string();
    let cmdline = parts.next().unwrap_or_default().trim().to_string();
    Some((comm, cmdline))
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

#[cfg(unix)]
pub fn detect_agents_for_pids(pids: Vec<(String, u32)>) -> Vec<DetectedAgent> {
    pids.into_iter()
        .filter_map(|(session_id, pid)| {
            let (comm, cmdline) = resolve_process_info(pid)?;
            let name = service::detect_agent_name(&comm, &cmdline)?;
            Some(DetectedAgent {
                session_id,
                name: name.to_string(),
                pid,
            })
        })
        .collect()
}

#[cfg(windows)]
pub fn detect_agents_for_pids(pids: Vec<(String, u32)>) -> Vec<DetectedAgent> {
    if pids.is_empty() {
        return Vec::new();
    }

    let system = sysinfo::System::new_all();
    let snapshot = process_snapshot(&system);

    pids.into_iter()
        .filter_map(|(session_id, shell_pid)| {
            let (agent_pid, name) = service::find_descendant_agent(&snapshot, shell_pid)?;
            Some(DetectedAgent {
                session_id,
                name: name.to_string(),
                pid: agent_pid,
            })
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

#[tauri::command]
#[specta::specta]
pub async fn agent_list(
    state: State<'_, AppState>,
    terminals: State<'_, TerminalStore>,
    project_id: ProjectId,
) -> AppResult<ProjectAgents> {
    ensure_project_open(&state, &project_id)?;

    let pids = terminals.foreground_pids(&project_id);
    let agents = tauri::async_runtime::spawn_blocking(move || detect_agents_for_pids(pids))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;

    Ok(ProjectAgents { project_id, agents })
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

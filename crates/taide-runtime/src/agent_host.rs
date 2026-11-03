#[cfg(unix)]
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use taide_agent::service::{self, DetectedAgentProbe};
use taide_agent::store::AgentStore;
use taide_model::agent::CliInstallStatus;
use taide_model::error::{AppError, AppResult};

use crate::{agent_probe, TaskSupervisor};

const CLI_BUNDLE_ONLY: &str = "taide CLI shell command is only available in the installed TAIDE.app (not a dev build)";
const CLI_NOT_MANAGED: &str = "the existing 'taide' command is not managed by TAIDE and was not removed";
#[cfg(unix)]
const PS_OUTPUT_FORMAT: &str = "pid=,comm=,args=";
#[cfg(unix)]
const PS_PID_SEPARATOR: &str = ",";

#[cfg(unix)]
pub fn resolve_process_infos(pids: &[u32]) -> HashMap<u32, service::ProcessInfo> {
    if pids.is_empty() {
        return HashMap::new();
    }
    let joined = pids.iter().map(u32::to_string).collect::<Vec<_>>().join(PS_PID_SEPARATOR);
    let Ok(output) = std::process::Command::new("ps")
        .args(["-o", PS_OUTPUT_FORMAT, "-p", &joined])
        .output()
    else {
        return HashMap::new();
    };
    service::parse_ps_process_infos(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(unix)]
pub fn resolve_agent_names(pids: &[u32]) -> HashMap<u32, Option<&'static str>> {
    let infos = resolve_process_infos(pids);
    pids.iter()
        .map(|pid| {
            let name = infos
                .get(pid)
                .and_then(|info| service::detect_agent_name(&info.comm, &info.cmdline));
            (*pid, name)
        })
        .collect()
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
pub fn detect_agents_for_pids(pids: Vec<(String, u32)>) -> Vec<DetectedAgentProbe> {
    if pids.is_empty() {
        return Vec::new();
    }
    let snapshot = process_snapshot(&sysinfo::System::new_all());
    pids.into_iter()
        .filter_map(|(session_id, shell_pid)| {
            let (agent_pid, name) = service::find_descendant_agent(&snapshot, shell_pid)?;
            Some(DetectedAgentProbe {
                session_id,
                name,
                pid: agent_pid,
            })
        })
        .collect()
}

#[cfg(unix)]
pub async fn detect_agents_for_pids_blocking(
    tasks: &TaskSupervisor,
    agents: &AgentStore,
    pids: Vec<(String, u32)>,
) -> AppResult<Vec<DetectedAgentProbe>> {
    agent_probe::probe_process_names(tasks, agents, pids, |unresolved| resolve_agent_names(&unresolved)).await
}

#[cfg(windows)]
pub async fn detect_agents_for_pids_blocking(
    tasks: &TaskSupervisor,
    _agents: &AgentStore,
    pids: Vec<(String, u32)>,
) -> AppResult<Vec<DetectedAgentProbe>> {
    agent_probe::probe_process_tree(tasks, pids, detect_agents_for_pids).await
}

pub fn cli_install_status(target: &Path) -> CliInstallStatus {
    let target_path = target.to_string_lossy();
    match std::fs::symlink_metadata(target) {
        Ok(_) => {
            let resolved = std::fs::canonicalize(target).ok().map(|path| path.to_string_lossy().to_string());
            let dangling = resolved.is_none();
            service::build_cli_install_status(&target_path, true, resolved, dangling)
        }
        Err(_) => service::build_cli_install_status(&target_path, false, None, false),
    }
}

/// How an administrator AppleScript for the `taide` shell command ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminPrompt {
    Completed,
    Cancelled,
}

/// The `taide` shell command state after an install or uninstall request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliCommandOutcome {
    pub status: CliInstallStatus,
    /// True when the user dismissed the administrator prompt, which leaves the command unchanged.
    pub is_cancelled: bool,
}

fn cli_command_outcome(source: &Path, prompt: AdminPrompt) -> CliCommandOutcome {
    CliCommandOutcome {
        status: cli_install_status(source),
        is_cancelled: prompt == AdminPrompt::Cancelled,
    }
}

fn cli_install_target(exe: &Path) -> AppResult<PathBuf> {
    service::resolve_cli_install_target(exe).ok_or_else(|| AppError::InvalidArgument(CLI_BUNDLE_ONLY.to_string()))
}

/// Links `source` to the CLI sidecar of the app bundle that runs `exe`, asking for administrator
/// rights through `run_script` unless the link already points there.
pub fn cli_install(exe: &Path, source: &Path, run_script: impl FnOnce(&str) -> AppResult<AdminPrompt>) -> AppResult<CliCommandOutcome> {
    let target = cli_install_target(exe)?;
    let already_linked = std::fs::read_link(source).is_ok_and(|existing| existing == target);
    let prompt = if already_linked {
        AdminPrompt::Completed
    } else {
        run_script(&service::build_cli_install_apple_script(&target, source))?
    };
    Ok(cli_command_outcome(source, prompt))
}

/// Removes the TAIDE-owned `source` link, asking for administrator rights through `run_script`
/// only when a plain removal is denied.
pub fn cli_uninstall(exe: &Path, source: &Path, run_script: impl FnOnce(&str) -> AppResult<AdminPrompt>) -> AppResult<CliCommandOutcome> {
    cli_install_target(exe)?;
    match std::fs::symlink_metadata(source) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(cli_command_outcome(source, AdminPrompt::Completed));
        }
        Err(error) => return Err(AppError::from(error)),
    }
    let owned = std::fs::read_link(source).is_ok_and(|link_target| service::is_cli_symlink_owned(&link_target));
    if !owned {
        return Err(AppError::InvalidArgument(CLI_NOT_MANAGED.to_string()));
    }
    let prompt = match std::fs::remove_file(source) {
        Ok(()) => AdminPrompt::Completed,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => AdminPrompt::Completed,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            run_script(&service::build_cli_uninstall_apple_script(source))?
        }
        Err(error) => return Err(AppError::from(error)),
    };
    Ok(cli_command_outcome(source, prompt))
}

/// Runs an administrator AppleScript with `program` (`osascript`); a dismissed password prompt is
/// [`AdminPrompt::Cancelled`] rather than an error.
pub fn run_admin_osascript(program: &Path, script: &str) -> AppResult<AdminPrompt> {
    let output = std::process::Command::new(program)
        .args(service::build_osascript_args(script))
        .output()?;
    if output.status.success() {
        return Ok(AdminPrompt::Completed);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if service::is_osascript_user_cancelled(output.status.code(), &stderr) {
        return Ok(AdminPrompt::Cancelled);
    }
    Err(AppError::Internal(format!("osascript failed: {stderr}")))
}

#[cfg(test)]
#[path = "agent_host_tests.rs"]
mod tests;

#[cfg(unix)]
use std::collections::HashMap;
use std::path::Path;

use taide_agent::service::{self, DetectedAgentProbe};
use taide_agent::store::AgentStore;
use taide_model::agent::CliInstallStatus;
use taide_model::error::AppResult;

use crate::{agent_probe, TaskSupervisor};

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

#[cfg(test)]
#[path = "agent_host_tests.rs"]
mod tests;

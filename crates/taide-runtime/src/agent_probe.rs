#[cfg(unix)]
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

use taide_agent::service::{self, DetectedAgentProbe, HookEmitter};
#[cfg(unix)]
use taide_agent::store::AgentStore;
use taide_model::error::{AppError, AppResult};
use tokio::sync::oneshot;

use crate::{TaskOperationLease, TaskSupervisor};

struct ProbeResult<T> {
    value: T,
    _operation: TaskOperationLease,
}

fn probe_shutdown_error() -> AppError {
    AppError::Forbidden("agent runtime is shutting down".to_string())
}

async fn run_probe<T: Send + 'static>(tasks: &TaskSupervisor, work: impl FnOnce() -> T + Send + 'static) -> AppResult<ProbeResult<T>> {
    let operation = tasks.begin_operation("agent-probe").ok_or_else(probe_shutdown_error)?;
    let (sender, receiver) = oneshot::channel();
    let owner = operation.clone();
    let worker = tasks
        .spawn_blocking_transient_handle("agent-probe-worker", move || {
            let _owner = owner;
            drop(sender.send(work()));
        })
        .ok_or_else(probe_shutdown_error)?;
    worker.await.map_err(|error| {
        if error.is_cancelled() {
            return probe_shutdown_error();
        }
        AppError::Internal(error.to_string())
    })?;
    let value = receiver.await.map_err(|_| probe_shutdown_error())?;
    Ok(ProbeResult {
        value,
        _operation: operation,
    })
}

/// Resolves uncached Unix PIDs through the host's lazy OS port and registered supervisor.
#[cfg(unix)]
pub async fn probe_process_names(
    tasks: &TaskSupervisor,
    agents: &AgentStore,
    pids: Vec<(String, u32)>,
    resolve: impl FnOnce(Vec<u32>) -> HashMap<u32, Option<&'static str>> + Send + 'static,
) -> AppResult<Vec<DetectedAgentProbe>> {
    if pids.is_empty() {
        return Ok(Vec::new());
    }
    let unresolved = agents.unresolved_pids(&pids);
    if unresolved.is_empty() {
        return Ok(agents.probes_for(pids));
    }
    let resolved = run_probe(tasks, move || resolve(unresolved)).await?;
    agents.remember_process_names(resolved.value);
    Ok(agents.probes_for(pids))
}

/// Preserves the host's process-tree detection policy while tracking the actual blocking worker.
pub async fn probe_process_tree(
    tasks: &TaskSupervisor,
    pids: Vec<(String, u32)>,
    resolve: impl FnOnce(Vec<(String, u32)>) -> Vec<DetectedAgentProbe> + Send + 'static,
) -> AppResult<Vec<DetectedAgentProbe>> {
    if pids.is_empty() {
        return Ok(Vec::new());
    }
    let resolved = run_probe(tasks, move || resolve(pids)).await?;
    Ok(resolved.value)
}

/// Caches the emitter choice without treating the caller's deadline as blocking-worker completion.
pub async fn resolve_claude_hook_emitter(
    tasks: &TaskSupervisor,
    cache: &OnceLock<HookEmitter>,
    deadline: Duration,
    probe: impl FnOnce() -> std::io::Result<Vec<u8>> + Send + 'static,
) -> HookEmitter {
    if let Some(cached) = cache.get() {
        return *cached;
    }
    let detected = detect_claude_hook_emitter(tasks, deadline, probe).await;
    *cache.get_or_init(|| detected)
}

async fn detect_claude_hook_emitter(
    tasks: &TaskSupervisor,
    deadline: Duration,
    probe: impl FnOnce() -> std::io::Result<Vec<u8>> + Send + 'static,
) -> HookEmitter {
    let Ok(Ok(resolved)) = tokio::time::timeout(deadline, run_probe(tasks, probe)).await else {
        return HookEmitter::DevTty;
    };
    let Ok(stdout) = resolved.value else {
        return HookEmitter::DevTty;
    };
    let supported = service::parse_claude_version(&String::from_utf8_lossy(&stdout)).is_some_and(service::supports_terminal_sequence);
    if supported {
        HookEmitter::TerminalSequence
    } else {
        HookEmitter::DevTty
    }
}

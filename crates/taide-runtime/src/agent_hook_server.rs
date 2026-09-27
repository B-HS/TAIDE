use std::future::Future;

use taide_agent::store::{AgentHooksStore, HooksServerInfo};
use taide_model::error::{AppError, AppResult};
use tokio::task::JoinHandle;

use crate::TaskSupervisor;

/// Keeps an uncached server start admitted through binding, accept registration and store publication.
pub async fn start_hooks_server<B, BF, L, S>(
    store: &AgentHooksStore,
    tasks: &TaskSupervisor,
    bind: B,
    start_accept: S,
    is_shutting_down: impl FnOnce() -> bool,
) -> AppResult<HooksServerInfo>
where
    B: FnOnce() -> BF,
    BF: Future<Output = AppResult<(HooksServerInfo, L)>>,
    S: FnOnce(L) -> Option<JoinHandle<()>>,
{
    if let Some(info) = store.server_info() {
        return Ok(info);
    }
    let _operation = tasks
        .begin_operation("agent-hooks-server-start")
        .ok_or_else(|| AppError::Internal("hook server task supervisor stopped".to_string()))?;

    let (info, binding) = bind().await?;
    let Some(accept_handle) = start_accept(binding) else {
        return Err(AppError::Internal("hook server task supervisor stopped".to_string()));
    };
    if is_shutting_down() {
        accept_handle.abort();
        return Err(AppError::Internal("hook server unavailable during shutdown".to_string()));
    }
    Ok(store.set_server(info, accept_handle))
}

/// Clears the stored server and project overrides before aborting the accept task.
pub fn stop_hooks_server(store: &AgentHooksStore) {
    if let Some(handle) = store.take_server() {
        handle.abort();
    }
}

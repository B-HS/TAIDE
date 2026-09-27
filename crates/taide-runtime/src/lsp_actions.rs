use std::path::Path;
use std::sync::Arc;

use taide_lsp::install::LspInstallStore;
use taide_lsp::session::LspLifecycleSnapshot;
use taide_lsp::store::{LspSessionEntry, LspStore};
use taide_lsp::{manifest, service};
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::lsp::{LspServerId, LspSessionInfo};

use crate::EventSink;

const REINITIALIZE_FAILURE_MESSAGE: &str =
    "초기화 핸드셰이크 재시도를 모두 소진해 서버를 재연결하지 못했습니다. 수동으로 다시 시작해주세요.";

fn find_entry(store: &LspStore, session_id: &str) -> AppResult<Arc<LspSessionEntry>> {
    store
        .get(session_id)
        .ok_or_else(|| AppError::NotFound(format!("lsp session not found: {session_id}")))
}

fn emit_status(events: &dyn EventSink, session_id: &str, snapshot: LspLifecycleSnapshot) {
    events.publish(AppEvent::LspSessionStatusChanged {
        session_id: session_id.to_string(),
        status: snapshot.status,
        last_error: snapshot.last_error,
        generation: snapshot.generation,
    });
}

/// Writes to the current process using its existing serialized protocol writer.
pub async fn lsp_send(store: &LspStore, session_id: String, message: String) -> AppResult<()> {
    let entry = find_entry(store, &session_id)?;
    let proc = entry
        .proc
        .lock()
        .clone()
        .ok_or_else(|| AppError::Internal("language server not ready".to_string()))?;
    proc.write_message(&message).await
}

/// Confirms reinitialization only for the current generation of a crashed session.
pub fn lsp_confirm_reinitialize(events: &dyn EventSink, store: &LspStore, session_id: String, generation: u32) -> AppResult<()> {
    let entry = find_entry(store, &session_id)?;
    if let Some(snapshot) = entry.lifecycle.confirm_reinitialized(generation) {
        emit_status(events, &session_id, snapshot);
    }
    Ok(())
}

/// Records reinitialization failure only for the current generation of a crashed session.
pub fn lsp_report_reinitialize_failure(events: &dyn EventSink, store: &LspStore, session_id: String, generation: u32) -> AppResult<()> {
    let entry = find_entry(store, &session_id)?;
    if let Some(snapshot) = entry
        .lifecycle
        .report_reinitialize_failure(generation, REINITIALIZE_FAILURE_MESSAGE.to_string())
    {
        emit_status(events, &session_id, snapshot);
    }
    Ok(())
}

/// Returns the existing session snapshots for one project without starting a server.
pub fn lsp_sessions(store: &LspStore, project_id: ProjectId) -> AppResult<Vec<LspSessionInfo>> {
    Ok(store.sessions_for_project(&project_id))
}

/// Resolves the bundled server before applying its existing root-discovery strategy.
pub fn lsp_resolve_root(server_id: LspServerId, file_path: String) -> AppResult<Option<String>> {
    let spec = manifest::find_spec(server_id.as_str())
        .ok_or_else(|| AppError::InvalidArgument(format!("unknown language server: {server_id}")))?;
    Ok(service::find_root(&spec, Path::new(&file_path)).map(|root| root.to_string_lossy().to_string()))
}

/// Requests installation cancellation without releasing slots held by running worker leases.
pub fn lsp_install_cancel(install_store: &LspInstallStore, server_id: LspServerId) -> AppResult<()> {
    install_store.cancel(&server_id);
    Ok(())
}

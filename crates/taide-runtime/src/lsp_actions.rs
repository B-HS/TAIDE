use std::path::Path;

use taide_lsp::install::LspInstallStore;
use taide_lsp::store::LspStore;
use taide_lsp::{manifest, service};
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::lsp::{LspServerId, LspSessionInfo};

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

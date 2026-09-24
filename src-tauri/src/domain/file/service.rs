use std::path::Path;

use taide_infra::persist;
use taide_infra::root_guard;
use taide_model::error::AppResult;

use crate::state::AppState;

pub use taide_file::service::*;

/// Resolves a save against open projects or a CLI-opened path, writes it atomically while
/// preserving file mode, marks the self-write, and clears the owning project's hot-exit mirror.
pub fn save_file_within_open_projects(state: &AppState, path: &Path, content: &str) -> AppResult<()> {
    let projects = state.projects.read().clone();
    let (project_id, resolved) = root_guard::resolve_owning_project_or_cli_opened(&projects, &state.cli_opened_paths.read(), path)?;

    persist::write_atomic_preserving_mode(&resolved, content.as_bytes())?;
    state.self_writes.mark(&resolved);
    match project_id {
        Some(project_id) => clear_mirror(&state.paths, &project_id, &resolved),
        None => Ok(()),
    }
}

use tauri::AppHandle;

use crate::domain::project::capability::ProjectCapability;
use crate::domain::project::types::Project;
use crate::ids::ProjectId;
use crate::state::AppState;

use super::commands;

/// Rewrites the IDE lockfile's `workspaceFolders` on both attach and detach — the lockfile is what
/// Claude Code reads to pick a candidate IDE, so it must track the open-project set immediately
/// instead of freezing at the snapshot taken when the IDE server started (see
/// `commands::refresh_lockfile`).
pub struct IdeLockfileCapability;

impl ProjectCapability for IdeLockfileCapability {
    fn attach(&self, app: &AppHandle, _state: &AppState, _project: &Project) {
        commands::refresh_lockfile(app);
    }

    fn detach(&self, app: &AppHandle, _state: &AppState, _project_id: &ProjectId) {
        commands::refresh_lockfile(app);
    }
}

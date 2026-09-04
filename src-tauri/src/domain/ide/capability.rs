use tauri::AppHandle;

use crate::domain::project::capability::{ProjectAttachment, ProjectCapability};
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
    /// Writes the lockfile in the build phase and registers nothing: the whole attach is file IO
    /// against a snapshot of `AppState::projects` the opening command already committed, so it is
    /// strictly better off outside the mutation guard (`architecture.md` §2.1 — no IO under a lock).
    fn build_attachment(&self, app: &AppHandle, _state: &AppState, _project: &Project) -> ProjectAttachment {
        commands::refresh_lockfile(app);
        ProjectAttachment::none()
    }

    fn detach(&self, app: &AppHandle, _state: &AppState, _project_id: &ProjectId) {
        commands::refresh_lockfile(app);
    }
}

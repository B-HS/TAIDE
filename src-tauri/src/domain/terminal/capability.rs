use std::path::Path;

use tauri::{AppHandle, Manager};

use crate::domain::project::capability::ProjectCapability;
use crate::domain::project::types::CapabilityKind;
use crate::ids::ProjectId;
use crate::state::AppState;

use super::commands::TerminalStore;

/// Reaps every pty session scoped to the closing project ([`TerminalStore::kill_project`]).
/// Correctness-sensitive, not just cleanup: before `project_close` ran this, nothing called
/// `pty_kill` for a closing project's sessions — they kept running, attached to nothing, until the
/// whole app quit (`TerminalStore::kill_all`).
pub struct TerminalCapability;

impl ProjectCapability for TerminalCapability {
    fn detected_kind(&self, _root: &Path) -> Option<CapabilityKind> {
        Some(CapabilityKind::Terminal)
    }

    fn detach(&self, app: &AppHandle, _state: &AppState, project_id: &ProjectId) {
        app.state::<TerminalStore>().kill_project(project_id);
    }
}

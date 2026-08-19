use tauri::AppHandle;

use crate::domain::project::capability::ProjectCapability;
use crate::domain::project::types::Project;
use crate::state::AppState;

use super::hooks;

/// Reconciles installed agent hooks against the new open-project set when a project is attached.
/// Spawned rather than awaited (the attach walk runs under `AppState::begin_mutation`, and the
/// reconcile does network/file IO of its own); `reconcile_installed_hooks` no-ops when the
/// `agent_hooks_enabled` setting is off. Detach is intentionally the no-op default — hook removal
/// on project close is C14 follow-up work, out of this extension point's initial scope.
pub struct AgentHooksCapability;

impl ProjectCapability for AgentHooksCapability {
    fn attach(&self, app: &AppHandle, _state: &AppState, _project: &Project) {
        let hooks_handle = app.clone();
        tauri::async_runtime::spawn(async move {
            hooks::reconcile_installed_hooks(&hooks_handle).await;
        });
    }
}

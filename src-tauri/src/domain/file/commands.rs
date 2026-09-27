use taide_runtime::file_actions;
use tauri::{AppHandle, State};

use super::service::{MirrorEntry, UntitledMirrorEntry};
use super::types::OpenedFile;
use crate::error::AppResult;
use crate::ids::{ProjectId, TabId};
use crate::plugin_port::PluginRuntimePort;
use crate::state::{AppState, FlushScope};

/// Delegates authorized file opening to the runtime, loading plugin overlays after access validation.
#[tauri::command]
#[specta::specta]
pub async fn file_open(
    app: AppHandle,
    state: State<'_, AppState>,
    plugins: State<'_, PluginRuntimePort>,
    path: String,
) -> AppResult<OpenedFile> {
    file_actions::file_open(&state, path, || (plugins.language_overlays)(&app)).await
}

/// Delegates mutation-guarded blocking saves to the shared runtime action.
#[tauri::command]
#[specta::specta]
pub async fn file_save(_app: AppHandle, state: State<'_, AppState>, path: String, content: String) -> AppResult<()> {
    file_actions::file_save(&state, path, content).await
}

#[tauri::command]
#[specta::specta]
pub async fn file_create(state: State<'_, AppState>, path: String, is_dir: bool) -> AppResult<()> {
    file_actions::file_create(&state, path, is_dir).await
}

#[tauri::command]
#[specta::specta]
pub async fn file_rename(state: State<'_, AppState>, from: String, to: String) -> AppResult<()> {
    file_actions::file_rename(&state, from, to).await
}

#[tauri::command]
#[specta::specta]
pub async fn file_delete(state: State<'_, AppState>, path: String) -> AppResult<()> {
    file_actions::file_delete(&state, path).await
}

/// Delegates mutation-guarded blocking copies to the runtime action.
#[tauri::command]
#[specta::specta]
pub async fn file_copy(state: State<'_, AppState>, from: String, to: String) -> AppResult<()> {
    file_actions::file_copy(&state, from, to).await
}

/// Delegates dirty mirror writes without acquiring the global mutation guard.
#[tauri::command]
#[specta::specta]
pub async fn file_mirror_dirty(
    _app: AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    path: String,
    content: String,
) -> AppResult<Option<f64>> {
    file_actions::file_mirror_dirty(&state, project_id, path, content).await
}

#[tauri::command]
#[specta::specta]
pub async fn file_list_mirrors(state: State<'_, AppState>, project_id: ProjectId) -> AppResult<Vec<MirrorEntry>> {
    file_actions::file_list_mirrors(&state, project_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn file_clear_mirror(state: State<'_, AppState>, project_id: ProjectId, path: String) -> AppResult<()> {
    file_actions::file_clear_mirror(&state, project_id, path).await
}

#[tauri::command]
#[specta::specta]
pub async fn file_prune_mirrors(state: State<'_, AppState>, project_id: ProjectId, keep_paths: Vec<String>) -> AppResult<()> {
    file_actions::file_prune_mirrors(&state, project_id, keep_paths).await
}

/// Delegates validated untitled mirror writes without acquiring the global mutation guard.
#[tauri::command]
#[specta::specta]
pub async fn file_mirror_untitled(state: State<'_, AppState>, project_id: ProjectId, tab_id: TabId, content: String) -> AppResult<()> {
    file_actions::file_mirror_untitled(&state, project_id, tab_id, content).await
}

#[tauri::command]
#[specta::specta]
pub async fn file_list_untitled_mirrors(state: State<'_, AppState>, project_id: ProjectId) -> AppResult<Vec<UntitledMirrorEntry>> {
    file_actions::file_list_untitled_mirrors(&state, project_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn file_clear_untitled_mirror(state: State<'_, AppState>, project_id: ProjectId, tab_id: TabId) -> AppResult<()> {
    file_actions::file_clear_untitled_mirror(&state, project_id, tab_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn file_prune_untitled_mirrors(state: State<'_, AppState>, project_id: ProjectId, keep_tab_ids: Vec<TabId>) -> AppResult<()> {
    file_actions::file_prune_untitled_mirrors(&state, project_id, keep_tab_ids).await
}

/// Confirms the calling window has finished flushing its dirty editor models to the hot-exit
/// mirror in response to a `HotExitFlushRequested` carrying `scope`, then resumes whatever
/// teardown that scope's request deferred — once *every* window expected to confirm has done so.
///
/// `window: tauri::Window` is Tauri-injected as whichever window's webview made this call, so a
/// window can only ever confirm on its own behalf. `scope` must be echoed back from the request
/// the window is answering, which is what keeps three concurrent handshakes apart: the app exit
/// ([`FlushScope::All`] — main plus any currently-open `editor-*` windows, see
/// `AppState::begin_hot_exit_flush`), one auxiliary window's close
/// ([`FlushScope::Window`]), and one project's close ([`FlushScope::Project`], which every window
/// answers because a project spans windows).
///
/// A no-op if that scope's flush was already completed (by every window confirming, or by the
/// timeout fallback), if the scope has no handshake at all, or if this window was not among the
/// ones it expected.
#[tauri::command]
#[specta::specta]
pub async fn file_flush_complete(
    app: AppHandle,
    window: tauri::Window<tauri::Wry>,
    state: State<'_, AppState>,
    scope: FlushScope,
) -> AppResult<()> {
    // Reaching `Ready` is the whole signal for every scope but the app exit: the auxiliary window's
    // close (`domain::window::commands::handle_auxiliary_close_requested`) and the project close
    // (`domain::project::commands::project_close`) each await their own handshake and resume
    // themselves, so nothing is dispatched from here.
    if state.complete_flush(&scope, window.label()) && matches!(scope, FlushScope::All) {
        app.exit(0);
    }
    Ok(())
}

#[tauri::command]
pub async fn file_read_raw(state: State<'_, AppState>, path: String) -> Result<tauri::ipc::Response, crate::error::AppError> {
    let bytes = file_actions::file_read_raw(&state, path).await?;
    Ok(tauri::ipc::Response::new(bytes))
}

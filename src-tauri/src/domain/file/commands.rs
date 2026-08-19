use std::path::Path;

use tauri::{AppHandle, State};

use super::service;
use super::service::{MirrorEntry, UntitledMirrorEntry};
use super::types::OpenedFile;
use crate::domain::plugin::commands::{self as plugin_commands, PluginStore};
use crate::error::AppResult;
use crate::ids::{ProjectId, TabId};
use crate::infra::root_guard;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn file_open(state: State<'_, AppState>, plugins: State<'_, PluginStore>, path: String) -> AppResult<OpenedFile> {
    let projects = state.projects.read().clone();
    let (_, resolved) = root_guard::resolve_owning_project(&projects, Path::new(&path))?;

    let loaded_plugins = plugin_commands::ensure_loaded(&plugins, &state.paths.plugins_dir());
    service::open_file(&resolved, &loaded_plugins)
}

#[tauri::command]
#[specta::specta]
pub async fn file_save(state: State<'_, AppState>, path: String, content: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    let (project_id, resolved) = root_guard::resolve_owning_project(&projects, Path::new(&path))?;

    service::save_file(&resolved, &content)?;
    state.self_writes.mark(&resolved);
    service::clear_mirror(&state.paths, &project_id, &resolved)
}

#[tauri::command]
#[specta::specta]
pub async fn file_create(state: State<'_, AppState>, path: String, is_dir: bool) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    let (_, resolved) = root_guard::resolve_owning_project(&projects, Path::new(&path))?;

    service::create_entry(&resolved, is_dir)?;
    state.self_writes.mark(&resolved);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn file_rename(state: State<'_, AppState>, from: String, to: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    let (_, resolved_from) = root_guard::resolve_owning_project(&projects, Path::new(&from))?;
    let (_, resolved_to) = root_guard::resolve_owning_project(&projects, Path::new(&to))?;

    service::rename_entry(&resolved_from, &resolved_to)?;
    state.self_writes.mark(&resolved_from);
    state.self_writes.mark(&resolved_to);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn file_delete(state: State<'_, AppState>, path: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    let (_, resolved) = root_guard::resolve_owning_project(&projects, Path::new(&path))?;

    service::delete_entry(&resolved)?;
    state.self_writes.mark(&resolved);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn file_copy(state: State<'_, AppState>, from: String, to: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    let (_, resolved_from) = root_guard::resolve_owning_project(&projects, Path::new(&from))?;
    let (_, resolved_to) = root_guard::resolve_owning_project(&projects, Path::new(&to))?;

    service::copy_entry(&resolved_from, &resolved_to)?;
    state.self_writes.mark(&resolved_to);
    Ok(())
}

/// Not guarded by `AppState::begin_mutation` — this command only reads `state.projects` (an
/// `RwLock` read, not the mutation lock) to resolve the project root; it never touches `AppState`
/// itself, and `persist::write_atomic`'s UUID temp-file + rename (`persist.rs`) already serializes
/// concurrent writers to the same mirror file (last writer wins, atomically), the same rationale
/// `lsp_send` (`lsp/commands.rs`) uses to skip the lock. Frequency is the practical reason this
/// matters: this fires on a 500ms debounce timer while the user types, far more often than
/// saves/git operations, so gating it behind the global mutation lock would queue every keystroke's
/// mirror write behind unrelated long-held mutations for no correctness benefit — ordering relative
/// to `file_save`'s own `clear_mirror` is guaranteed by the frontend's save-epoch guard
/// (`editor-pane.tsx`'s `persistMirror`), not by lock ordering. The real cost of keeping the lock
/// here would surface at shutdown: `handle_close_requested`'s hot-exit flush would then wait behind
/// a long lock holder (e.g. `git_push`) and blow through `HOT_EXIT_FLUSH_TIMEOUT_MS`, losing every
/// unflushed mirror instead of writing it — the opposite of what hot exit exists for.
#[tauri::command]
#[specta::specta]
pub async fn file_mirror_dirty(state: State<'_, AppState>, project_id: ProjectId, path: String, content: String) -> AppResult<Option<f64>> {
    let projects = state.projects.read().clone();
    let root = root_guard::project_root(&projects, &project_id)?;
    let resolved = root_guard::ensure_within_root(&root, Path::new(&path))?;

    service::mirror_dirty(&state.paths, &project_id, &resolved, &path, &content)
}

#[tauri::command]
#[specta::specta]
pub async fn file_list_mirrors(state: State<'_, AppState>, project_id: ProjectId) -> AppResult<Vec<MirrorEntry>> {
    let projects = state.projects.read().clone();
    root_guard::project_root(&projects, &project_id)?;

    service::list_mirrors(&state.paths, &project_id)
}

#[tauri::command]
#[specta::specta]
pub async fn file_clear_mirror(state: State<'_, AppState>, project_id: ProjectId, path: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    let root = root_guard::project_root(&projects, &project_id)?;
    let resolved = root_guard::ensure_within_root(&root, Path::new(&path))?;

    service::clear_mirror(&state.paths, &project_id, &resolved)
}

#[tauri::command]
#[specta::specta]
pub async fn file_prune_mirrors(state: State<'_, AppState>, project_id: ProjectId, keep_paths: Vec<String>) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    root_guard::project_root(&projects, &project_id)?;

    service::prune_mirrors(&state.paths, &project_id, &keep_paths)
}

/// Not guarded by `AppState::begin_mutation` — same rationale as `file_mirror_dirty` above: reads
/// only `state.projects` to validate the project exists, never touches `AppState` otherwise, and
/// `persist::write_atomic` already serializes concurrent writers to the same untitled-mirror file.
#[tauri::command]
#[specta::specta]
pub async fn file_mirror_untitled(state: State<'_, AppState>, project_id: ProjectId, tab_id: TabId, content: String) -> AppResult<()> {
    let projects = state.projects.read().clone();
    root_guard::project_root(&projects, &project_id)?;
    root_guard::ensure_safe_component(tab_id.as_str())?;

    service::mirror_untitled(&state.paths, &project_id, &tab_id, &content)
}

#[tauri::command]
#[specta::specta]
pub async fn file_list_untitled_mirrors(state: State<'_, AppState>, project_id: ProjectId) -> AppResult<Vec<UntitledMirrorEntry>> {
    let projects = state.projects.read().clone();
    root_guard::project_root(&projects, &project_id)?;

    service::list_untitled_mirrors(&state.paths, &project_id)
}

#[tauri::command]
#[specta::specta]
pub async fn file_clear_untitled_mirror(state: State<'_, AppState>, project_id: ProjectId, tab_id: TabId) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    root_guard::project_root(&projects, &project_id)?;
    root_guard::ensure_safe_component(tab_id.as_str())?;

    service::clear_untitled_mirror(&state.paths, &project_id, &tab_id)
}

#[tauri::command]
#[specta::specta]
pub async fn file_prune_untitled_mirrors(state: State<'_, AppState>, project_id: ProjectId, keep_tab_ids: Vec<TabId>) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    root_guard::project_root(&projects, &project_id)?;

    service::prune_untitled_mirrors(&state.paths, &project_id, &keep_tab_ids)
}

/// Confirms the calling window has finished flushing every dirty editor
/// model to the hot-exit mirror in response to `HotExitFlushRequested`, then
/// resumes the app exit that the main window's `CloseRequested` deferred
/// once every window expected to confirm has done so (Wave I: main plus any
/// currently-open `editor-*` auxiliary windows — see
/// `AppState::begin_hot_exit_flush`). A no-op if the flush was already
/// completed (by every window confirming, or by the timeout fallback).
#[tauri::command]
#[specta::specta]
pub async fn file_flush_complete(app: AppHandle, window: tauri::Window<tauri::Wry>, state: State<'_, AppState>) -> AppResult<()> {
    if state.complete_hot_exit_flush(window.label()) {
        app.exit(0);
    }
    Ok(())
}

#[tauri::command]
pub async fn file_read_raw(state: State<'_, AppState>, path: String) -> Result<tauri::ipc::Response, crate::error::AppError> {
    let projects = state.projects.read().clone();
    let (_, resolved) = root_guard::resolve_owning_project(&projects, Path::new(&path))?;
    let bytes = service::read_raw(&resolved)?;
    Ok(tauri::ipc::Response::new(bytes))
}

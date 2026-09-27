use taide_runtime::layout_actions;
use tauri::{AppHandle, State};

use super::service;
use super::types::{DropEdge, OpenTabInSplitRequest, ProjectLayout, ShellViewPatch, TabKind, TabPathChange, TabPathChangeResult};
use crate::error::AppResult;
use crate::ids::{PaneId, ProjectId, TabId};
use crate::platform::event_sink::TauriEventSink;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn layout_get(state: State<'_, AppState>, project_id: ProjectId) -> AppResult<ProjectLayout> {
    layout_actions::layout_get(&state, project_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_open_tab(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    kind: TabKind,
    title: String,
    target: Option<PaneId>,
    preview: bool,
) -> AppResult<ProjectLayout> {
    layout_actions::layout_open_tab(&TauriEventSink(&app), &state, project_id, kind, title, target, preview).await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_close_tab(app: AppHandle, state: State<'_, AppState>, tab_id: TabId) -> AppResult<ProjectLayout> {
    let (_, _, updated) = service::close_tab_and_finish(&app, &state, &tab_id).await?;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_activate_tab(app: AppHandle, state: State<'_, AppState>, tab_id: TabId) -> AppResult<ProjectLayout> {
    layout_actions::layout_activate_tab(&TauriEventSink(&app), &state, tab_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_move_tab(
    app: AppHandle,
    state: State<'_, AppState>,
    tab_id: TabId,
    pane_id: PaneId,
    index: u32,
) -> AppResult<ProjectLayout> {
    layout_actions::layout_move_tab(&TauriEventSink(&app), &state, tab_id, pane_id, index).await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_split(
    app: AppHandle,
    state: State<'_, AppState>,
    pane_id: PaneId,
    edge: DropEdge,
    tab_id: TabId,
) -> AppResult<ProjectLayout> {
    layout_actions::layout_split(&TauriEventSink(&app), &state, pane_id, edge, tab_id).await
}

/// Opens a *new* tab in a *new* pane beside `target_pane` — what the terminal's context menu calls
/// "split", and what `layout_split` is not: that one moves a tab that already exists. Doing this in
/// one command rather than `layout_open_tab` + `layout_split` is a correctness requirement, not a
/// round-trip saving: `layout_open_tab` activates the new tab in the *source* pane first, which
/// unmounts the terminal the user is looking at (replaying its whole ring buffer) and then, once
/// the split remounts that tab elsewhere, spawns its shell a second time. One mutation guard and
/// one `layout:changed` make that sequence unrepresentable. See
/// `docs/acknowledge/2026-09-04-usability-batch4-contract.md` §F.2 and the `open_tab_in_split`
/// service function.
///
/// `request.edge` must be directional: `DropEdge::Center` means "into the target pane", which is
/// [`layout_open_tab`]'s job, so it is rejected as `InvalidArgument` rather than silently treated
/// as one. `File` kinds run the same pre-flight `ensure_file_tab_target_exists` gate
/// `layout_open_tab` runs, for the same reason: a tab must never outlive the path it was opened for.
#[tauri::command]
#[specta::specta]
pub async fn layout_open_tab_in_split(
    app: AppHandle,
    state: State<'_, AppState>,
    request: OpenTabInSplitRequest,
) -> AppResult<ProjectLayout> {
    layout_actions::layout_open_tab_in_split(&TauriEventSink(&app), &state, request).await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_resize(app: AppHandle, state: State<'_, AppState>, pane_id: PaneId, sizes: Vec<f32>) -> AppResult<ProjectLayout> {
    layout_actions::layout_resize(&TauriEventSink(&app), &state, pane_id, sizes).await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_focus_pane(app: AppHandle, state: State<'_, AppState>, pane_id: PaneId) -> AppResult<ProjectLayout> {
    layout_actions::layout_focus_pane(&TauriEventSink(&app), &state, pane_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_pin_tab(app: AppHandle, state: State<'_, AppState>, tab_id: TabId, pinned: bool) -> AppResult<ProjectLayout> {
    layout_actions::layout_pin_tab(&TauriEventSink(&app), &state, tab_id, pinned).await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_preview(app: AppHandle, state: State<'_, AppState>, tab_id: TabId, preview: bool) -> AppResult<ProjectLayout> {
    layout_actions::layout_set_preview(&TauriEventSink(&app), &state, tab_id, preview).await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_reopen_closed(app: AppHandle, state: State<'_, AppState>, project_id: ProjectId) -> AppResult<ProjectLayout> {
    layout_actions::layout_reopen_closed(&TauriEventSink(&app), &state, project_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_view_state(
    app: AppHandle,
    state: State<'_, AppState>,
    tab_id: TabId,
    view_state: Option<String>,
) -> AppResult<ProjectLayout> {
    layout_actions::layout_set_view_state(&TauriEventSink(&app), &state, tab_id, view_state).await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_dirty(app: AppHandle, state: State<'_, AppState>, tab_id: TabId, dirty: bool) -> AppResult<ProjectLayout> {
    layout_actions::layout_set_dirty(&TauriEventSink(&app), &state, tab_id, dirty).await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_terminal_session(
    app: AppHandle,
    state: State<'_, AppState>,
    tab_id: TabId,
    session_id: String,
) -> AppResult<ProjectLayout> {
    layout_actions::layout_set_terminal_session(&TauriEventSink(&app), &state, tab_id, session_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_open_untitled(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    target: Option<PaneId>,
) -> AppResult<ProjectLayout> {
    layout_actions::layout_open_untitled(&TauriEventSink(&app), &state, project_id, target).await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_convert_untitled(app: AppHandle, state: State<'_, AppState>, tab_id: TabId, path: String) -> AppResult<ProjectLayout> {
    layout_actions::layout_convert_untitled(&TauriEventSink(&app), &state, tab_id, path).await
}

/// Makes open tabs follow a path change the file domain has already committed to disk — a rename
/// repoints every `TabKind::File` tab under the old path (a directory rename moves the whole
/// subtree), a delete closes them. Audit §4-B A3: nothing used to update a tab's stored path, so a
/// renamed file's tab kept pointing at a path that no longer exists (`⌘S` then recreated the old
/// directory through `write_atomic`'s `create_dir_all` and forked the file), and reopening the file
/// under its new name produced a second tab for the same document.
///
/// Called by the frontend *after* `file_rename`/`file_delete` succeeds rather than from inside those
/// commands: the file domain owns paths on disk, not the tab bar, and wiring it here would make
/// `domain::file` depend on `domain::layout` (the same domain-coupling boundary
/// `docs/architecture.md` §2 keeps between `file` and `tree`, where the watcher event — not the file
/// command — drives the tree's own cleanup).
///
/// Reports what changed (`moved`/`closed_paths`) so the frontend can move the per-path state only it
/// owns — monaco model, hot-exit mirror, `FILE.CONTENT` cache, "reopen with" override. When no *open*
/// tab addressed the changed path there is nothing to report, and no revision bump or
/// `layout:changed` event is produced: nothing on screen moved, so making every window refetch the
/// layout would be pure waste.
///
/// A rename with nothing to report can still have changed the layout, though — the closed-tab stack
/// follows a rename silently so "Reopen Closed Tab" cannot resurrect a path that no longer exists —
/// which is why the store write-back is gated on `TabPathChangeOutcome::layout_changed` rather than
/// on there being something to report. Skipping it there (the original shape of this early return)
/// discarded that rewrite along with the whole working clone.
///
/// `from`/`to`/`path` are checked against the project's own root before anything is matched. They are
/// *not* canonicalized (the rename is already done, so `from` no longer exists on disk — see contract
/// §3 S8), but they must still be inside the project: `Deleted { path: "/" }` is "at or under" every
/// absolute path, which would close every file tab in the project and let the frontend's
/// `releaseClosedFileTabPath` discard each one's hot-exit mirror — i.e. every unsaved draft.
#[tauri::command]
#[specta::specta]
pub async fn layout_apply_path_change(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    change: TabPathChange,
) -> AppResult<TabPathChangeResult> {
    layout_actions::layout_apply_path_change(&TauriEventSink(&app), &state, project_id, change).await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_shell_view(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    patch: ShellViewPatch,
) -> AppResult<ProjectLayout> {
    layout_actions::layout_set_shell_view(&TauriEventSink(&app), &state, project_id, patch).await
}

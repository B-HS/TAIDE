use std::path::Path;

use tauri::{AppHandle, Manager, State};

use super::service;
use super::types::{DropEdge, ProjectLayout, ShellViewPatch, Tab, TabKind, TabWindowTarget};
use crate::domain::window::commands::{open_auxiliary_window, WindowStore};
use crate::error::{AppError, AppResult};
use crate::ids::{PaneId, ProjectId, TabId};
use crate::infra::root_guard;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn layout_get(state: State<'_, AppState>, project_id: ProjectId) -> AppResult<ProjectLayout> {
    state
        .layouts
        .read()
        .get(&project_id)
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("layout not found: {project_id}")))
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
    service::open_tab_and_finish(&app, &state, project_id, kind, title, target, preview).await
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
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = service::locate_project_with_tab(&layouts, &tab_id)?;
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    service::activate_tab(layout, &tab_id)?;

    let updated = service::finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
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
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = service::locate_project_with_tab(&layouts, &tab_id)?;
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    service::move_tab(layout, &tab_id, &pane_id, index as usize)?;

    let updated = service::finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
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
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = service::locate_project_with_pane(&layouts, &pane_id)?;
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    service::split(layout, &pane_id, edge, &tab_id)?;

    let updated = service::finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_resize(app: AppHandle, state: State<'_, AppState>, pane_id: PaneId, sizes: Vec<f32>) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = service::locate_project_with_pane(&layouts, &pane_id)?;
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    service::resize(layout, &pane_id, sizes)?;

    let updated = service::finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_focus_pane(app: AppHandle, state: State<'_, AppState>, pane_id: PaneId) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = service::locate_project_with_pane(&layouts, &pane_id)?;
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    service::focus_pane(layout, &pane_id)?;

    let updated = service::finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_pin_tab(app: AppHandle, state: State<'_, AppState>, tab_id: TabId, pinned: bool) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = service::locate_project_with_tab(&layouts, &tab_id)?;
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    service::pin_tab(layout, &tab_id, pinned)?;

    let updated = service::finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_preview(app: AppHandle, state: State<'_, AppState>, tab_id: TabId, preview: bool) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = service::locate_project_with_tab(&layouts, &tab_id)?;
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    service::set_preview(layout, &tab_id, preview)?;

    let updated = service::finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_reopen_closed(app: AppHandle, state: State<'_, AppState>, project_id: ProjectId) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    service::reopen_closed(layout);

    let updated = service::finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_view_state(
    app: AppHandle,
    state: State<'_, AppState>,
    tab_id: TabId,
    view_state: Option<String>,
) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = service::locate_project_with_tab(&layouts, &tab_id)?;
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    service::set_view_state(layout, &tab_id, view_state)?;

    let updated = service::finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_dirty(app: AppHandle, state: State<'_, AppState>, tab_id: TabId, dirty: bool) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = service::locate_project_with_tab(&layouts, &tab_id)?;
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    service::set_dirty(layout, &tab_id, dirty)?;

    let updated = service::finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_terminal_session(
    app: AppHandle,
    state: State<'_, AppState>,
    tab_id: TabId,
    session_id: String,
) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = service::locate_project_with_tab(&layouts, &tab_id)?;
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    service::set_terminal_session(layout, &tab_id, session_id)?;

    let updated = service::finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_open_untitled(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    target: Option<PaneId>,
) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    let pane_id = target.unwrap_or_else(|| layout.focused_pane.clone());
    let index = service::next_untitled_index(layout);
    let tab = Tab {
        id: TabId::new(),
        kind: TabKind::Untitled { index },
        title: format!("Untitled-{index}"),
        pinned: false,
        preview: false,
        dirty: false,
        view_state: None,
    };
    service::open_tab(layout, &pane_id, tab, false)?;

    let updated = service::finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_convert_untitled(app: AppHandle, state: State<'_, AppState>, tab_id: TabId, path: String) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    let (_, resolved) = root_guard::resolve_owning_project(&projects, Path::new(&path))?;
    let title = resolved
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| AppError::InvalidArgument(format!("invalid path: {path}")))?;

    let mut layouts = state.layouts.read().clone();
    let project_id = service::locate_project_with_tab(&layouts, &tab_id)?;
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    service::convert_untitled_to_file(layout, &tab_id, resolved.to_string_lossy().into_owned(), title)?;

    let updated = service::finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

/// Closes and forgets the OS window for any auxiliary window entry left with an empty pane tree
/// after a tab move — "빈 보조 창은 정리" (contract §3.4/item 2). Removing the layout entry here
/// (rather than waiting for the window's own `CloseRequested`) means `window::service::
/// plan_return_of_auxiliary_window_tabs` finds nothing left to do when that close fires a moment
/// later — harmless, since it's already a no-op-safe idempotent lookup by slot.
fn cleanup_emptied_auxiliary_windows(app: &AppHandle, windows: &WindowStore, project_id: &ProjectId, layout: &mut ProjectLayout) {
    let emptied_slots: Vec<u32> = layout
        .auxiliary_windows
        .iter()
        .filter(|window| service::is_layout_tree_empty(&window.root))
        .map(|window| window.slot)
        .collect();

    for slot in emptied_slots {
        layout.auxiliary_windows.retain(|window| window.slot != slot);
        if let Some(label) = windows.label_for(project_id, slot) {
            if let Some(webview_window) = app.get_webview_window(&label) {
                let _ = webview_window.close();
            }
        }
    }
}

/// Moves a tab to the main window, an already-open auxiliary window, or a brand-new one —
/// "Move into New Window"/"Move back to Main Window" (contract §3.2). `NewAuxiliary` reserves a
/// slot and opens the real OS window *before* touching the layout, so a window-creation failure
/// never leaves the layout half-mutated; if the subsequent move somehow fails anyway (unreachable
/// in practice, since the tab was already located above), `layout::service::move_tab_to_new_window`
/// itself rolls back the just-inserted empty window entry, and this command additionally closes the
/// now-pointless OS window it just opened.
#[tauri::command]
#[specta::specta]
pub async fn layout_move_tab_to_window(
    app: AppHandle,
    state: State<'_, AppState>,
    windows: State<'_, WindowStore>,
    tab_id: TabId,
    target: TabWindowTarget,
) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = service::locate_project_with_tab(&layouts, &tab_id)?;
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    match target {
        TabWindowTarget::Main => {
            service::move_tab_to_main(layout, &tab_id)?;
        }
        TabWindowTarget::Existing { slot } => {
            service::move_tab_to_existing_window(layout, &tab_id, slot)?;
        }
        TabWindowTarget::NewAuxiliary => {
            let slot = service::next_window_slot(layout);
            let info = open_auxiliary_window(&app, &state, &windows, project_id.clone(), slot).await?;
            if let Err(error) = service::move_tab_to_new_window(layout, &tab_id, slot) {
                if let Some(webview_window) = app.get_webview_window(&info.label) {
                    let _ = webview_window.close();
                }
                return Err(error);
            }
        }
    }

    cleanup_emptied_auxiliary_windows(&app, &windows, &project_id, layout);

    let updated = service::finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_shell_view(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    patch: ShellViewPatch,
) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    service::apply_shell_view_patch(layout, &patch);

    let updated = service::finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

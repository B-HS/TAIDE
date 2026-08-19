use std::collections::HashMap;
use std::path::Path;

use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use super::service;
use super::types::{ClosedTab, DropEdge, ProjectLayout, ShellViewPatch, Tab, TabKind, TabWindowTarget};
use crate::domain::window::commands::{open_auxiliary_window, WindowStore};
use crate::error::{AppError, AppResult};
use crate::events::LayoutChanged;
use crate::ids::{PaneId, ProjectId, TabId};
use crate::infra::root_guard;
use crate::state::AppState;

/// Resolves which project owns `tab_id`, searching every pane tree that project's layout owns —
/// the main tree *and* every auxiliary window's tree (`service::all_roots`) — not just `layout.root`.
/// A tab moved into an auxiliary window (`layout_move_tab_to_window`) no longer lives in the main
/// tree at all, so scoping this to `layout.root` would make every layout command targeting it
/// (close/activate/move/pin/set_dirty/move back to main, ...) fail with a spurious NotFound.
fn locate_project_with_tab(layouts: &HashMap<ProjectId, ProjectLayout>, tab_id: &TabId) -> AppResult<ProjectId> {
    layouts
        .iter()
        .find_map(|(project_id, layout)| {
            service::all_roots(layout)
                .any(|root| service::find_tab(root, tab_id).is_some())
                .then(|| project_id.clone())
        })
        .ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))
}

/// Same rationale as [`locate_project_with_tab`], for pane ids — a pane inside an auxiliary window's
/// tree must resolve too (split/resize/focus_pane target panes there once a tab has moved there).
fn locate_project_with_pane(layouts: &HashMap<ProjectId, ProjectLayout>, pane_id: &PaneId) -> AppResult<ProjectId> {
    layouts
        .iter()
        .find_map(|(project_id, layout)| {
            service::all_roots(layout)
                .any(|root| service::contains_pane(root, pane_id))
                .then(|| project_id.clone())
        })
        .ok_or_else(|| AppError::NotFound(format!("pane not found: {pane_id}")))
}

fn get_layout_mut<'a>(layouts: &'a mut HashMap<ProjectId, ProjectLayout>, project_id: &ProjectId) -> AppResult<&'a mut ProjectLayout> {
    layouts
        .get_mut(project_id)
        .ok_or_else(|| AppError::NotFound(format!("layout not found: {project_id}")))
}

fn finish_mutation(app: &AppHandle, state: &AppState, project_id: &ProjectId, layout: &mut ProjectLayout) -> ProjectLayout {
    let snapshot = layout.clone();
    state.dirty_layouts.write().insert(project_id.clone());

    let _ = LayoutChanged {
        project_id: project_id.clone(),
        revision: snapshot.revision,
    }
    .emit(app);

    snapshot
}

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
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    let preview = preview && state.settings.read().enable_preview_tabs;
    let pane_id = target.unwrap_or_else(|| layout.focused_pane.clone());
    let tab = Tab {
        id: TabId::new(),
        kind,
        title,
        pinned: false,
        preview: false,
        dirty: false,
        view_state: None,
    };
    service::open_tab(layout, &pane_id, tab, preview)?;

    let updated = finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

/// 탭을 닫고 후처리(레이아웃 갱신 이벤트 발신 + IDE 도메인의 pending diff 해소 + 터미널 탭이면
/// pty 세션 회수)까지 마친다. Tauri 커맨드(`layout_close_tab`)와 IDE 도메인의
/// `close_tab`/`closeAllDiffTabs` 도구 핸들러가 동일한 경로를 타도록 공유한다 — ClaudeDiff 탭이
/// 어떤 경로로 닫히든 pending 요청이 반드시 해소되고, 터미널 탭이 어떤 경로로 닫히든 그 pty 가
/// 반드시 죽는다. 레이아웃 read-clone-write 경합을 막기 위해 뮤테이션 가드는 이 함수가 직접 잡는다.
///
/// The pty reap is the "탭 닫기 시 pty_kill" half of the T0 #21 fix (`docs/acknowledge/
/// 2026-08-18-audit-t0-fix-contract.md` §2.3) — `project_close`'s `TerminalStore::kill_project`
/// only reaps sessions when the *project* closes, and before this fix nothing called `pty_kill` when
/// an individual terminal tab closed; the session lingered, attached to nothing, until the owning
/// project or the whole app closed.
pub async fn close_tab_and_finish(app: &AppHandle, state: &AppState, tab_id: &TabId) -> AppResult<(ProjectId, ClosedTab, ProjectLayout)> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = locate_project_with_tab(&layouts, tab_id)?;
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    let closed = service::close_tab(layout, tab_id)?;

    let updated = finish_mutation(app, state, &project_id, layout);
    *state.layouts.write() = layouts;

    crate::domain::ide::commands::reconcile_closed_tab(app, &closed.tab);
    if let TabKind::Terminal { session_id, .. } = &closed.tab.kind {
        app.state::<crate::domain::terminal::commands::TerminalStore>()
            .kill_session(session_id);
    }

    Ok((project_id, closed, updated))
}

#[tauri::command]
#[specta::specta]
pub async fn layout_close_tab(app: AppHandle, state: State<'_, AppState>, tab_id: TabId) -> AppResult<ProjectLayout> {
    let (_, _, updated) = close_tab_and_finish(&app, &state, &tab_id).await?;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_activate_tab(app: AppHandle, state: State<'_, AppState>, tab_id: TabId) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = locate_project_with_tab(&layouts, &tab_id)?;
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    service::activate_tab(layout, &tab_id)?;

    let updated = finish_mutation(&app, &state, &project_id, layout);
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
    let project_id = locate_project_with_tab(&layouts, &tab_id)?;
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    service::move_tab(layout, &tab_id, &pane_id, index as usize)?;

    let updated = finish_mutation(&app, &state, &project_id, layout);
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
    let project_id = locate_project_with_pane(&layouts, &pane_id)?;
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    service::split(layout, &pane_id, edge, &tab_id)?;

    let updated = finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_resize(app: AppHandle, state: State<'_, AppState>, pane_id: PaneId, sizes: Vec<f32>) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = locate_project_with_pane(&layouts, &pane_id)?;
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    service::resize(layout, &pane_id, sizes)?;

    let updated = finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_focus_pane(app: AppHandle, state: State<'_, AppState>, pane_id: PaneId) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = locate_project_with_pane(&layouts, &pane_id)?;
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    service::focus_pane(layout, &pane_id)?;

    let updated = finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_pin_tab(app: AppHandle, state: State<'_, AppState>, tab_id: TabId, pinned: bool) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = locate_project_with_tab(&layouts, &tab_id)?;
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    service::pin_tab(layout, &tab_id, pinned)?;

    let updated = finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_preview(app: AppHandle, state: State<'_, AppState>, tab_id: TabId, preview: bool) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = locate_project_with_tab(&layouts, &tab_id)?;
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    service::set_preview(layout, &tab_id, preview)?;

    let updated = finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_reopen_closed(app: AppHandle, state: State<'_, AppState>, project_id: ProjectId) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    service::reopen_closed(layout);

    let updated = finish_mutation(&app, &state, &project_id, layout);
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
    let project_id = locate_project_with_tab(&layouts, &tab_id)?;
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    service::set_view_state(layout, &tab_id, view_state)?;

    let updated = finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_dirty(app: AppHandle, state: State<'_, AppState>, tab_id: TabId, dirty: bool) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = locate_project_with_tab(&layouts, &tab_id)?;
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    service::set_dirty(layout, &tab_id, dirty)?;

    let updated = finish_mutation(&app, &state, &project_id, layout);
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
    let project_id = locate_project_with_tab(&layouts, &tab_id)?;
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    service::set_terminal_session(layout, &tab_id, session_id)?;

    let updated = finish_mutation(&app, &state, &project_id, layout);
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
    let layout = get_layout_mut(&mut layouts, &project_id)?;

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

    let updated = finish_mutation(&app, &state, &project_id, layout);
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
    let project_id = locate_project_with_tab(&layouts, &tab_id)?;
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    service::convert_untitled_to_file(layout, &tab_id, resolved.to_string_lossy().into_owned(), title)?;

    let updated = finish_mutation(&app, &state, &project_id, layout);
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
    let project_id = locate_project_with_tab(&layouts, &tab_id)?;
    let layout = get_layout_mut(&mut layouts, &project_id)?;

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

    let updated = finish_mutation(&app, &state, &project_id, layout);
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
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    service::apply_shell_view_patch(layout, &patch);

    let updated = finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::super::types::PaneNode;
    use super::*;

    /// Regression coverage for the commands-layer locator helpers themselves — every layout
    /// `#[tauri::command]` (`layout_close_tab`, `layout_activate_tab`, `layout_move_tab_to_window`
    /// back to `Main`, ...) funnels through `locate_project_with_tab`/`locate_project_with_pane`
    /// first, so a regression here would silently make every one of those commands return
    /// `NotFound` for anything living in an auxiliary window — exactly the class of bug the service
    /// layer's own tests (`layout::service`'s `move_tab_to_new_window`/`move_tab_to_main` tests)
    /// can't catch, since they call service functions directly and never exercise this
    /// `HashMap<ProjectId, ProjectLayout>` project-resolution step at all.
    #[test]
    fn locate_project_with_tab_과_pane_은_보조_창_트리도_찾는다() {
        let mut layout = service::default_layout();
        let source_pane = layout.focused_pane.clone();

        let PaneNode::Leaf { tabs, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let tab_id = tabs[0].id.clone();

        service::move_tab_to_new_window(&mut layout, &tab_id, 1).expect("move into new window");
        let aux_pane_id = layout.auxiliary_windows[0].focused_pane.clone();

        let mut layouts = HashMap::new();
        let project_id = ProjectId::new();
        layouts.insert(project_id.clone(), layout);

        assert_eq!(
            locate_project_with_tab(&layouts, &tab_id).expect("보조 창으로 옮긴 탭도 찾아야 한다"),
            project_id,
            "탭이 보조 창 트리에만 있어도 프로젝트를 찾아야 한다"
        );
        assert_eq!(
            locate_project_with_pane(&layouts, &aux_pane_id).expect("보조 창의 pane 도 찾아야 한다"),
            project_id,
            "pane 이 보조 창 트리에만 있어도 프로젝트를 찾아야 한다"
        );
        assert_eq!(
            locate_project_with_pane(&layouts, &source_pane).expect("main 트리의 pane 은 여전히 찾아야 한다"),
            project_id
        );

        assert!(
            locate_project_with_tab(&layouts, &TabId::new()).is_err(),
            "존재하지 않는 탭은 NotFound 여야 한다"
        );
    }
}

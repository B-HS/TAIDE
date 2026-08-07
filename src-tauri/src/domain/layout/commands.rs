use std::collections::HashMap;
use std::path::Path;

use tauri::{AppHandle, State};
use tauri_specta::Event;

use super::service;
use super::types::{ClosedTab, DropEdge, ProjectLayout, Tab, TabKind};
use crate::error::{AppError, AppResult};
use crate::events::{LayoutChanged, ProjectFocusKindChanged};
use crate::ids::{PaneId, ProjectId, TabId};
use crate::infra::root_guard;
use crate::state::AppState;

fn locate_project_with_tab(layouts: &HashMap<ProjectId, ProjectLayout>, tab_id: &TabId) -> AppResult<ProjectId> {
    layouts
        .iter()
        .find_map(|(project_id, layout)| service::find_tab(&layout.root, tab_id).map(|_| project_id.clone()))
        .ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))
}

fn locate_project_with_pane(layouts: &HashMap<ProjectId, ProjectLayout>, pane_id: &PaneId) -> AppResult<ProjectId> {
    layouts
        .iter()
        .find_map(|(project_id, layout)| {
            if service::contains_pane(&layout.root, pane_id) {
                Some(project_id.clone())
            } else {
                None
            }
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

    if let Some(kind) = service::focus_kind(&snapshot) {
        let _ = ProjectFocusKindChanged {
            project_id: project_id.clone(),
            kind,
        }
        .emit(app);
    }

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

/// 탭을 닫고 후처리(레이아웃 갱신 이벤트 발신 + IDE 도메인의 pending diff 해소)까지 마친다.
/// Tauri 커맨드(`layout_close_tab`)와 IDE 도메인의 `close_tab`/`closeAllDiffTabs` 도구 핸들러가
/// 동일한 경로를 타도록 공유한다 — ClaudeDiff 탭이 어떤 경로로 닫히든 pending 요청이 반드시 해소된다.
/// 레이아웃 read-clone-write 경합을 막기 위해 뮤테이션 가드는 이 함수가 직접 잡는다.
pub async fn close_tab_and_finish(app: &AppHandle, state: &AppState, tab_id: &TabId) -> AppResult<(ProjectId, ClosedTab, ProjectLayout)> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = locate_project_with_tab(&layouts, tab_id)?;
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    let closed = service::close_tab(layout, tab_id)?;

    let updated = finish_mutation(app, state, &project_id, layout);
    *state.layouts.write() = layouts;

    crate::domain::ide::commands::reconcile_closed_tab(app, &closed.tab);

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

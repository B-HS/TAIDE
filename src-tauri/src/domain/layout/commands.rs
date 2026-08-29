use std::path::Path;

use tauri::{AppHandle, Manager, State};

use super::service;
use super::types::{DropEdge, ProjectLayout, ShellViewPatch, Tab, TabKind, TabPathChange, TabPathChangeResult, TabWindowTarget};
use crate::domain::window::commands::{open_auxiliary_window, WindowStore};
use crate::error::{AppError, AppErrorKind, AppResult};
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

/// Where a layout-mutation command finds the `ProjectLayout` it's about to mutate — the three
/// lookup shapes the 13 skeleton-identical commands below used inline before being unified into
/// [`run_layout_mutation`]: resolve the owning project from a tab id, from a pane id, or (when the
/// caller already names the project directly) skip lookup entirely.
enum LayoutLocate {
    WithTab(TabId),
    WithPane(PaneId),
    Direct(ProjectId),
}

/// Runs the read-clone-locate-mutate-writeback skeleton shared by 13 layout commands, preserving the
/// exact lock semantics each had before unification: the single `begin_mutation` guard acquisition
/// below is this function's only `.await`, and it spans `layouts.read().clone()` all the way through
/// the `finish_mutation` + `layouts.write()` writeback, so no other mutator can interleave a write
/// into that window. `mutate` is required to stay synchronous by contract — to avoid growing this
/// function's await-point count and to keep the shared `FnOnce(&mut ProjectLayout) -> AppResult<()>`
/// signature every caller passes — not because an `.await` inside it would itself be unsound: the
/// guard is a `tokio::sync::MutexGuard` that is routinely held across `.await` points elsewhere in
/// this file (e.g. `layout_move_tab_to_window` below, which awaits `open_auxiliary_window` while
/// still holding it).
async fn run_layout_mutation<F>(app: &AppHandle, state: &State<'_, AppState>, locate: LayoutLocate, mutate: F) -> AppResult<ProjectLayout>
where
    F: FnOnce(&mut ProjectLayout) -> AppResult<()>,
{
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = match locate {
        LayoutLocate::WithTab(tab_id) => service::locate_project_with_tab(&layouts, &tab_id)?,
        LayoutLocate::WithPane(pane_id) => service::locate_project_with_pane(&layouts, &pane_id)?,
        LayoutLocate::Direct(project_id) => project_id,
    };
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    mutate(layout)?;

    let updated = service::finish_mutation(app, state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_activate_tab(app: AppHandle, state: State<'_, AppState>, tab_id: TabId) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::activate_tab(layout, &tab_id)
    })
    .await
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
    run_layout_mutation(&app, &state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::move_tab(layout, &tab_id, &pane_id, index as usize)
    })
    .await
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
    run_layout_mutation(&app, &state, LayoutLocate::WithPane(pane_id.clone()), move |layout| {
        service::split(layout, &pane_id, edge, &tab_id)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_resize(app: AppHandle, state: State<'_, AppState>, pane_id: PaneId, sizes: Vec<f32>) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::WithPane(pane_id.clone()), move |layout| {
        service::resize(layout, &pane_id, sizes)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_focus_pane(app: AppHandle, state: State<'_, AppState>, pane_id: PaneId) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::WithPane(pane_id.clone()), move |layout| {
        service::focus_pane(layout, &pane_id)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_pin_tab(app: AppHandle, state: State<'_, AppState>, tab_id: TabId, pinned: bool) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::pin_tab(layout, &tab_id, pinned)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_preview(app: AppHandle, state: State<'_, AppState>, tab_id: TabId, preview: bool) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::set_preview(layout, &tab_id, preview)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_reopen_closed(app: AppHandle, state: State<'_, AppState>, project_id: ProjectId) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::Direct(project_id), move |layout| {
        service::reopen_closed(layout);
        Ok(())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_view_state(
    app: AppHandle,
    state: State<'_, AppState>,
    tab_id: TabId,
    view_state: Option<String>,
) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::set_view_state(layout, &tab_id, view_state)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_dirty(app: AppHandle, state: State<'_, AppState>, tab_id: TabId, dirty: bool) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::set_dirty(layout, &tab_id, dirty)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_terminal_session(
    app: AppHandle,
    state: State<'_, AppState>,
    tab_id: TabId,
    session_id: String,
) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::set_terminal_session(layout, &tab_id, session_id)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_open_untitled(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    target: Option<PaneId>,
) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::Direct(project_id), move |layout| {
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
        Ok(())
    })
    .await
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

/// Rejects a [`TabPathChange`] naming anything outside `root`, by path components — the same
/// containment rule `service::retargeted_path`/`is_same_or_under` match tabs with, so a path this
/// accepts is exactly a path the matcher can act on.
///
/// Deliberately a string containment check rather than `root_guard::ensure_within_root`: the file
/// command this follows has already completed, so `from` (and a deleted `path`) is gone from disk
/// and cannot be canonicalized against anything. The project root itself is canonical (recorded that
/// way when the project is opened — `domain::project::service`), and tab paths are built by walking
/// it, so both sides of this comparison come from the same canonical prefix.
fn ensure_change_within_root(root: &Path, change: &TabPathChange) -> AppResult<()> {
    let paths: &[&String] = match change {
        TabPathChange::Renamed { from, to } => &[from, to],
        TabPathChange::Deleted { path } => &[path],
    };
    for path in paths {
        if !Path::new(path.as_str()).starts_with(root) {
            return Err(AppError::localized(
                AppErrorKind::Forbidden,
                "error.path.outsideProjectRoot",
                format!("path is outside the project root: {path}"),
            )
            .with_arg("path", path.as_str()));
        }
    }
    Ok(())
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
    let projects = state.projects.read().clone();
    let root = root_guard::project_root(&projects, &project_id)?;
    ensure_change_within_root(&root, &change)?;

    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    let outcome = service::apply_tab_path_change(layout, &change);
    if outcome.is_empty() {
        let snapshot = layout.clone();
        if outcome.layout_changed {
            state.dirty_layouts.write().insert(project_id.clone());
            *state.layouts.write() = layouts;
        }
        return Ok(TabPathChangeResult {
            layout: snapshot,
            moved: outcome.moved,
            closed_paths: outcome.closed_paths,
        });
    }

    let updated = service::finish_mutation(&app, &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(TabPathChangeResult {
        layout: updated,
        moved: outcome.moved,
        closed_paths: outcome.closed_paths,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_shell_view(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    patch: ShellViewPatch,
) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::Direct(project_id), move |layout| {
        service::apply_shell_view_patch(layout, &patch);
        Ok(())
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn 개명(from: &str, to: &str) -> TabPathChange {
        TabPathChange::Renamed {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    #[test]
    fn 프로젝트_루트_안의_경로_변경은_통과한다() {
        let root = Path::new("/repo");

        assert!(ensure_change_within_root(root, &개명("/repo/a.txt", "/repo/b.txt")).is_ok());
        assert!(ensure_change_within_root(
            root,
            &TabPathChange::Deleted {
                path: "/repo/src/nested".to_string()
            }
        )
        .is_ok());
        assert!(
            ensure_change_within_root(root, &개명("/repo", "/repo")).is_ok(),
            "루트 자신은 루트 하위다"
        );
    }

    #[test]
    fn 루트_밖_경로와_슬래시는_거절된다() {
        let root = Path::new("/repo");

        assert!(
            ensure_change_within_root(root, &TabPathChange::Deleted { path: "/".to_string() }).is_err(),
            "'/' 는 모든 절대 경로의 조상이라 프로젝트의 모든 파일 탭을 닫고 미저장 미러를 지운다"
        );
        assert!(ensure_change_within_root(root, &개명("/repo/a.txt", "/elsewhere/b.txt")).is_err());
        assert!(ensure_change_within_root(root, &개명("/elsewhere/a.txt", "/repo/b.txt")).is_err());
        assert!(
            ensure_change_within_root(root, &개명("/repo-old/a.txt", "/repo-old/b.txt")).is_err(),
            "성분 단위 비교라 이름이 접두사인 형제 디렉토리는 루트 안이 아니다"
        );
    }
}

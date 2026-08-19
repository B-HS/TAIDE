use std::path::Path;

use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use super::service;
use super::types::{Project, ProjectRef};
use crate::domain::file::types::FsChange;
use crate::domain::git::commands::GitStore;
use crate::domain::git::watch as git_watch;
use crate::domain::layout::service as layout_service;
use crate::domain::terminal::commands::TerminalStore;
use crate::domain::tree::commands::TreeStore;
use crate::error::AppResult;
use crate::events::{FsChanged, GitRefsChanged, GitStatusChanged, ProjectActivated, ProjectClosed, ProjectListChanged, ProjectOpened};
use crate::ids::ProjectId;
use crate::infra::self_write::resolve_from_app;
use crate::infra::watcher;
use crate::state::AppState;

pub fn attach_watcher(app: &AppHandle, state: &AppState, project_id: &ProjectId, root: &str) {
    let emit_handle = app.clone();
    let emit_project = project_id.clone();

    match watcher::start_watch(std::path::PathBuf::from(root), move |change: FsChange| {
        let change = resolve_from_app(&emit_handle.state::<AppState>().self_writes, change);
        let _ = FsChanged {
            project_id: emit_project.clone(),
            change,
        }
        .emit(&emit_handle);
    }) {
        Ok(handle) => {
            state.watchers.write().insert(project_id.clone(), handle);
        }
        Err(error) => log::warn!("파일 감시를 시작하지 못했습니다 ({root}): {error}"),
    }
}

pub fn attach_git_watcher(app: &AppHandle, state: &AppState, project_id: &ProjectId, root: &str) {
    let git_dir = std::path::Path::new(root).join(".git");
    if !git_dir.is_dir() {
        return;
    }

    let emit_handle = app.clone();
    let emit_project = project_id.clone();

    match watcher::start_watch(git_dir.clone(), move |change: FsChange| {
        let mut needs_status = false;
        let mut needs_refs = false;

        for path in &change.paths {
            match git_watch::classify_git_change(std::path::Path::new(path)) {
                Some(git_watch::GitInvalidation::Status) => needs_status = true,
                Some(git_watch::GitInvalidation::Refs) => needs_refs = true,
                None => {}
            }
        }

        if needs_status {
            let _ = GitStatusChanged {
                project_id: emit_project.clone(),
            }
            .emit(&emit_handle);
        }
        if needs_refs {
            let _ = GitRefsChanged {
                project_id: emit_project.clone(),
            }
            .emit(&emit_handle);
        }
    }) {
        Ok(handle) => {
            state.git_watchers.write().insert(project_id.clone(), handle);
        }
        Err(error) => log::warn!("git 감시를 시작하지 못했습니다 ({}): {error}", git_dir.display()),
    }
}

fn emit_list_changed(app: &AppHandle, state: &AppState) {
    let projects = service::list_projects(&state.session.read());
    let _ = ProjectListChanged { projects }.emit(app);
}

#[tauri::command]
#[specta::specta]
pub async fn project_list(state: State<'_, AppState>) -> AppResult<Vec<ProjectRef>> {
    Ok(service::list_projects(&state.session.read()))
}

#[tauri::command]
#[specta::specta]
pub async fn project_get(state: State<'_, AppState>, project_id: ProjectId) -> AppResult<Project> {
    service::get_project(&state.projects.read(), &project_id)
}

#[tauri::command]
#[specta::specta]
pub async fn project_get_active(state: State<'_, AppState>) -> AppResult<Option<ProjectId>> {
    Ok(state.session.read().active_project.clone())
}

#[tauri::command]
#[specta::specta]
pub async fn project_open(app: AppHandle, state: State<'_, AppState>, path: String) -> AppResult<service::ProjectOpenResult> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();
    let mut projects = state.projects.read().clone();

    let result = service::open_project(&state.paths, &mut session, &mut projects, Path::new(&path))?;

    *state.session.write() = session;
    *state.projects.write() = projects;

    if !result.already_open {
        let layout = layout_service::load_layout(&state.paths, &result.project.id);
        state.layouts.write().insert(result.project.id.clone(), layout);
        attach_watcher(&app, &state, &result.project.id, &result.project.root);
        attach_git_watcher(&app, &state, &result.project.id, &result.project.root);

        crate::domain::ide::commands::refresh_lockfile(&app);

        let hooks_handle = app.clone();
        tauri::async_runtime::spawn(async move {
            crate::domain::agent::hooks::reconcile_installed_hooks(&hooks_handle).await;
        });

        let _ = ProjectOpened {
            project: result.project.clone(),
        }
        .emit(&app);
        emit_list_changed(&app, &state);
    }

    Ok(result)
}

/// Closes `project_id` and reaps every resource that only makes sense while the project is open.
/// See `architecture.md` §6.3 for the authoritative list of what a project close must reclaim.
/// `asset://` read access needs no entry of its own in that list any more: `infra::asset_protocol`
/// decides per-request from `state.projects`, and the `*state.projects.write() = projects;` above
/// (via `service::close_project`, which removes `project_id`) already revokes it before this
/// function's other reaps even run. Two of those other reaps are correctness-sensitive, not just
/// cleanup:
///
/// - **Layout**: a layout marked dirty but not yet caught by `flush_dirty_layouts`'s periodic
///   2-second timer would otherwise be discarded unsaved — the `state.layouts.write().remove`
///   below drops the in-memory copy, and the periodic flusher's next pass over `dirty_layouts`
///   would then find nothing left in `state.layouts` for this `project_id` and silently skip it
///   (see that function's `filter_map`, which also `log::warn!`s on exactly this miss as a safety
///   net for any other path that removes a layout without flushing first). Flushing synchronously
///   here, before the removal, closes that window.
/// - **Terminals**: `TerminalStore::kill_project` reaps every pty session scoped to this project.
///   Before this, nothing called `pty_kill` for a closing project's sessions — they kept running,
///   attached to nothing, until the whole app quit (`TerminalStore::kill_all`).
///
/// `GitStore`/`TreeStore` removal below is plain cache eviction (no correctness risk either way —
/// `resolve_repo_root`/`ensure_entry` transparently rebuild a missing entry on next access) but
/// still matters: without it, reopening the same folder resurrects a possibly-stale repo root or
/// directory listing instead of resolving fresh, and the entry otherwise sits in memory for the
/// rest of the app's lifetime regardless of whether the project ever reopens.
#[tauri::command]
#[specta::specta]
pub async fn project_close(
    app: AppHandle,
    state: State<'_, AppState>,
    git_store: State<'_, GitStore>,
    tree_store: State<'_, TreeStore>,
    project_id: ProjectId,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();
    let mut projects = state.projects.read().clone();

    service::close_project(&state.paths, &mut session, &mut projects, &project_id)?;

    let active_project = session.active_project.clone();
    *state.session.write() = session;
    *state.projects.write() = projects;

    if state.dirty_layouts.write().remove(&project_id) {
        if let Some(layout) = state.layouts.read().get(&project_id).cloned() {
            if let Err(error) = layout_service::save_layout(&state.paths, &project_id, &layout) {
                log::warn!("프로젝트 종료 시 레이아웃 저장 실패 ({project_id}): {error}");
            }
        }
    }

    state.layouts.write().remove(&project_id);
    state.watchers.write().remove(&project_id);
    state.git_watchers.write().remove(&project_id);
    app.state::<TerminalStore>().kill_project(&project_id);
    git_store.remove(&project_id);
    tree_store.remove(&project_id);

    crate::domain::ide::commands::refresh_lockfile(&app);

    let _ = ProjectClosed {
        project_id: project_id.clone(),
    }
    .emit(&app);
    let _ = ProjectActivated {
        project_id: active_project,
    }
    .emit(&app);
    emit_list_changed(&app, &state);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn project_activate(app: AppHandle, state: State<'_, AppState>, project_id: ProjectId) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    service::activate_project(&state.paths, &mut session, &project_id)?;

    *state.session.write() = session;

    let _ = ProjectActivated {
        project_id: Some(project_id),
    }
    .emit(&app);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn project_reorder(app: AppHandle, state: State<'_, AppState>, ids: Vec<ProjectId>) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    service::reorder_projects(&state.paths, &mut session, &ids)?;

    *state.session.write() = session;
    emit_list_changed(&app, &state);

    Ok(())
}

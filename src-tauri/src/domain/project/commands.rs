use std::path::Path;

use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use super::service;
use super::types::{Project, ProjectRef};
use crate::domain::file::types::FsChange;
use crate::domain::git::watch as git_watch;
use crate::domain::layout::service as layout_service;
use crate::error::AppResult;
use crate::events::{FsChanged, GitRefsChanged, GitStatusChanged, ProjectActivated, ProjectClosed, ProjectListChanged, ProjectOpened};
use crate::ids::ProjectId;
use crate::infra::watcher;
use crate::state::AppState;

pub fn allow_asset_access(app: &AppHandle, root: &str) {
    if let Err(error) = app.asset_protocol_scope().allow_directory(root, true) {
        log::warn!("asset scope 등록 실패 {root}: {error}");
    }
}

pub fn attach_watcher(app: &AppHandle, state: &AppState, project_id: &ProjectId, root: &str) {
    let emit_handle = app.clone();
    let emit_project = project_id.clone();

    match watcher::start_watch(std::path::PathBuf::from(root), move |change: FsChange| {
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
        allow_asset_access(&app, &result.project.root);
        attach_watcher(&app, &state, &result.project.id, &result.project.root);
        attach_git_watcher(&app, &state, &result.project.id, &result.project.root);

        let _ = ProjectOpened {
            project: result.project.clone(),
        }
        .emit(&app);
        emit_list_changed(&app, &state);
    }

    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn project_close(app: AppHandle, state: State<'_, AppState>, project_id: ProjectId) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();
    let mut projects = state.projects.read().clone();

    service::close_project(&state.paths, &mut session, &mut projects, &project_id)?;

    let active_project = session.active_project.clone();
    *state.session.write() = session;
    *state.projects.write() = projects;
    state.layouts.write().remove(&project_id);
    state.watchers.write().remove(&project_id);
    state.git_watchers.write().remove(&project_id);

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

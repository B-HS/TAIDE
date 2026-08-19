use std::path::Path;

use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use super::capability::ProjectCapabilities;
use super::service;
use super::types::{Project, ProjectRef};
use crate::error::AppResult;
use crate::events::{ProjectActivated, ProjectClosed, ProjectListChanged, ProjectOpened};
use crate::ids::ProjectId;
use crate::state::AppState;

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
        app.state::<ProjectCapabilities>().attach_all(&app, &state, &result.project);

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
/// (via `service::close_project`, which removes `project_id`) already revokes it before any
/// capability's detach even runs. The reaps themselves are owned by the registered
/// [`ProjectCapabilities`], whose detach walk runs in registration order — an order that is part
/// of the correctness contract (dirty-layout flush before removal, terminal reap during close);
/// see `lib.rs`'s `project_capabilities` and each capability's `detach` doc.
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

    app.state::<ProjectCapabilities>().detach_all(&app, &state, &project_id);

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

use std::path::Path;

use tauri::State;

use super::service;
use super::types::OpenedFile;
use crate::error::AppResult;
use crate::ids::ProjectId;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn file_open(path: String) -> AppResult<OpenedFile> {
    service::open_file(Path::new(&path))
}

#[tauri::command]
#[specta::specta]
pub async fn file_save(state: State<'_, AppState>, path: String, content: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    let (project_id, resolved) = service::resolve_owning_project(&projects, Path::new(&path))?;

    service::save_file(&resolved, &content)?;
    service::clear_mirror(&state.paths, &project_id, &resolved)
}

#[tauri::command]
#[specta::specta]
pub async fn file_create(state: State<'_, AppState>, path: String, is_dir: bool) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    let (_, resolved) = service::resolve_owning_project(&projects, Path::new(&path))?;

    service::create_entry(&resolved, is_dir)
}

#[tauri::command]
#[specta::specta]
pub async fn file_rename(state: State<'_, AppState>, from: String, to: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    let (_, resolved_from) = service::resolve_owning_project(&projects, Path::new(&from))?;
    let (_, resolved_to) = service::resolve_owning_project(&projects, Path::new(&to))?;

    service::rename_entry(&resolved_from, &resolved_to)
}

#[tauri::command]
#[specta::specta]
pub async fn file_delete(state: State<'_, AppState>, path: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    let (_, resolved) = service::resolve_owning_project(&projects, Path::new(&path))?;

    service::delete_entry(&resolved)
}

#[tauri::command]
#[specta::specta]
pub async fn file_copy(state: State<'_, AppState>, from: String, to: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    let (_, resolved_from) = service::resolve_owning_project(&projects, Path::new(&from))?;
    let (_, resolved_to) = service::resolve_owning_project(&projects, Path::new(&to))?;

    service::copy_entry(&resolved_from, &resolved_to)
}

#[tauri::command]
#[specta::specta]
pub async fn file_mirror_dirty(state: State<'_, AppState>, project_id: ProjectId, path: String, content: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    let root = service::project_root(&projects, &project_id)?;
    let resolved = service::ensure_within_root(&root, Path::new(&path))?;

    service::mirror_dirty(&state.paths, &project_id, &resolved, &content)
}

#[tauri::command]
pub async fn file_read_raw(state: State<'_, AppState>, path: String) -> Result<tauri::ipc::Response, crate::error::AppError> {
    let projects = state.projects.read().clone();
    let (_, resolved) = service::resolve_owning_project(&projects, Path::new(&path))?;
    let bytes = service::read_raw(&resolved)?;
    Ok(tauri::ipc::Response::new(bytes))
}

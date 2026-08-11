use std::collections::HashMap;
use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use tauri::{AppHandle, State};
use tauri_specta::Event;

use super::service;
use super::types::{BlameLine, CommitOptions, DiffMode, DiffSides, GitBranch, GitRemote, GitStashEntry, GitStatus, GutterHunk, LogEntry};
use crate::error::{AppError, AppResult};
use crate::events::{GitRefsChanged, GitStatusChanged};
use crate::ids::ProjectId;
use crate::state::AppState;

#[derive(Default)]
pub struct GitStore(Mutex<HashMap<ProjectId, PathBuf>>);

impl GitStore {
    pub fn new() -> Self {
        Self::default()
    }
}

fn resolve_repo_root(state: &State<'_, AppState>, store: &State<'_, GitStore>, project_id: &ProjectId) -> AppResult<PathBuf> {
    if let Some(cached) = store.0.lock().get(project_id) {
        return Ok(cached.clone());
    }

    let root = state
        .projects
        .read()
        .get(project_id)
        .map(|project| project.root.clone())
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))?;

    let repo_root = service::discover(Path::new(&root))?;
    store.0.lock().insert(project_id.clone(), repo_root.clone());
    Ok(repo_root)
}

fn emit_status_changed(app: &AppHandle, project_id: &ProjectId) {
    let _ = GitStatusChanged {
        project_id: project_id.clone(),
    }
    .emit(app);
}

fn emit_refs_changed(app: &AppHandle, project_id: &ProjectId) {
    let _ = GitRefsChanged {
        project_id: project_id.clone(),
    }
    .emit(app);
}

#[tauri::command]
#[specta::specta]
pub async fn git_init(app: AppHandle, state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let root = state
        .projects
        .read()
        .get(&project_id)
        .map(|project| project.root.clone())
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))?;

    service::init(Path::new(&root))?;
    store.0.lock().remove(&project_id);
    emit_status_changed(&app, &project_id);
    emit_refs_changed(&app, &project_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn git_status(state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId) -> AppResult<GitStatus> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::status(&repo_root)
}

#[tauri::command]
#[specta::specta]
pub async fn git_diff_file(
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    path: String,
    mode: DiffMode,
) -> AppResult<DiffSides> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    tauri::async_runtime::spawn_blocking(move || service::diff_file(&repo_root, &path, mode))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

#[tauri::command]
#[specta::specta]
pub async fn git_show_file(
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    rev: String,
    path: String,
) -> AppResult<String> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::show_file(&repo_root, &rev, &path)
}

#[tauri::command]
#[specta::specta]
pub async fn git_log(
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    skip: u32,
    take: u32,
) -> AppResult<Vec<LogEntry>> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    tauri::async_runtime::spawn_blocking(move || service::log(&repo_root, skip as usize, take as usize))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

#[tauri::command]
#[specta::specta]
pub async fn git_ahead_behind(
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
) -> AppResult<service::AheadBehind> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::ahead_behind(&repo_root)
}

#[tauri::command]
#[specta::specta]
pub async fn git_remotes(state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId) -> AppResult<Vec<GitRemote>> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::remotes(&repo_root)
}

#[tauri::command]
#[specta::specta]
pub async fn git_gutter(
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    path: String,
) -> AppResult<Vec<GutterHunk>> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::gutter(&repo_root, &path)
}

#[tauri::command]
#[specta::specta]
pub async fn git_blame_range(
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    path: String,
    from: u32,
    to: u32,
) -> AppResult<Vec<BlameLine>> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    tauri::async_runtime::spawn_blocking(move || service::blame_range(&repo_root, &path, from, to))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

#[tauri::command]
#[specta::specta]
pub async fn git_stage(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    paths: Vec<String>,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::stage(&repo_root, &paths)?;
    emit_status_changed(&app, &project_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn git_unstage(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    paths: Vec<String>,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::unstage(&repo_root, &paths)?;
    emit_status_changed(&app, &project_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn git_discard(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    paths: Vec<String>,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::discard(&repo_root, &paths)?;
    emit_status_changed(&app, &project_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn git_commit(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    message: String,
    opts: CommitOptions,
) -> AppResult<String> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    let oid = service::commit(&repo_root, &message, &opts)?;
    emit_status_changed(&app, &project_id);
    emit_refs_changed(&app, &project_id);
    Ok(oid)
}

#[tauri::command]
#[specta::specta]
pub async fn git_push(app: AppHandle, state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::push(&repo_root)?;
    emit_refs_changed(&app, &project_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn git_pull(app: AppHandle, state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::pull(&repo_root)?;
    emit_status_changed(&app, &project_id);
    emit_refs_changed(&app, &project_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn git_fetch(app: AppHandle, state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::fetch(&repo_root)?;
    emit_refs_changed(&app, &project_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn git_current_user(state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId) -> AppResult<Option<String>> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::current_user(&repo_root)
}

#[tauri::command]
#[specta::specta]
pub async fn git_branches(state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId) -> AppResult<Vec<GitBranch>> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::branches(&repo_root)
}

#[tauri::command]
#[specta::specta]
pub async fn git_branch_create(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    name: String,
    checkout: bool,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::branch_create(&repo_root, &name, checkout)?;
    emit_refs_changed(&app, &project_id);
    if checkout {
        emit_status_changed(&app, &project_id);
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn git_branch_checkout(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    name: String,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::branch_checkout(&repo_root, &name)?;
    emit_status_changed(&app, &project_id);
    emit_refs_changed(&app, &project_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn git_branch_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    name: String,
    force: bool,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::branch_delete(&repo_root, &name, force)?;
    emit_refs_changed(&app, &project_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn git_stash_list(
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
) -> AppResult<Vec<GitStashEntry>> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::stash_list(&repo_root)
}

#[tauri::command]
#[specta::specta]
pub async fn git_stash_push(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    message: Option<String>,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::stash_push(&repo_root, message.as_deref())?;
    emit_status_changed(&app, &project_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn git_stash_apply(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    index: u32,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::stash_apply(&repo_root, index)?;
    emit_status_changed(&app, &project_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn git_stash_drop(state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId, index: u32) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::stash_drop(&repo_root, index)
}

#[tauri::command]
#[specta::specta]
pub async fn git_discard_hunk(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    path: String,
    hunk_start: u32,
    hunk_end: u32,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    tauri::async_runtime::spawn_blocking(move || service::discard_hunk(&repo_root, &path, hunk_start, hunk_end))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(&app, &project_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn git_undo_last_commit(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::undo_last_commit(&repo_root)?;
    emit_status_changed(&app, &project_id);
    emit_refs_changed(&app, &project_id);
    Ok(())
}

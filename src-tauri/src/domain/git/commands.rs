use std::collections::HashMap;
use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use super::service;
use super::types::{
    BlameLine, CommitFile, CommitOptions, ConflictSides, DiffMode, DiffSides, GitBranch, GitRemote, GitStashEntry, GitStatus, GutterHunk,
    LogEntry, RevertOutcome, StagedDiffText, TagCreateOptions, TagInfo,
};
use crate::domain::plugin::service::{self as plugin_service, PluginStore};
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

    /// Forgets `project_id`'s cached repo root, called by `GitCacheCapability::detach` during
    /// `project_close` so a
    /// project reopened at the same path resolves its repo root fresh instead of reusing a cache
    /// entry keyed by a `ProjectId` that no session will ever look up again — see `resolve_repo_root`,
    /// which otherwise happily serves that stale entry forever (the map is never pruned by size or
    /// age, only by this explicit removal).
    pub fn remove(&self, project_id: &ProjectId) {
        self.0.lock().remove(project_id);
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
    plugins: State<'_, PluginStore>,
    project_id: ProjectId,
    path: String,
    mode: DiffMode,
) -> AppResult<DiffSides> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    let loaded_plugins = plugin_service::ensure_loaded(&plugins, &state.paths.plugins_dir());
    let language_overlays = plugin_service::language_overlays(&loaded_plugins);
    tauri::async_runtime::spawn_blocking(move || service::diff_file(&repo_root, &path, mode, &language_overlays))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

#[tauri::command]
#[specta::specta]
pub async fn git_diff_staged_text(
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
) -> AppResult<StagedDiffText> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    tauri::async_runtime::spawn_blocking(move || service::diff_staged_text(&repo_root))
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

/// Runs `git push` on a blocking thread **without** `AppState::begin_mutation` (audit R4#3, C11
/// axis A). What the old guard actually covered was audited before removal: `service::push`
/// shells out to `git push`, which reads local refs/objects and — on success — updates the
/// remote-tracking ref inside `.git`; it never touches the working tree, so serializing it with
/// the app's file mutations (`file_save`, replace, ...) protected nothing. Same-repo `.git`
/// integrity against concurrent app git commands (commit/stage/pull) is enforced by git's own
/// index/ref locks, exactly as when the user runs `git push` in a terminal beside the app, and
/// command-level ordering (commit-then-push) is already sequenced by the frontend awaiting each
/// command. The subprocess wait moved into `spawn_blocking` so the network round-trip no longer
/// pins an async worker thread either (architecture.md §2.1's other half).
#[tauri::command]
#[specta::specta]
pub async fn git_push(app: AppHandle, state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId) -> AppResult<()> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    tauri::async_runtime::spawn_blocking(move || service::push(&repo_root))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_refs_changed(&app, &project_id);
    Ok(())
}

/// Still runs the whole `git pull` under `AppState::begin_mutation`, but on a blocking thread via
/// `begin_mutation_blocking` (the `search_replace` T0#17 pattern) so the subprocess wait no
/// longer pins an async worker thread. The lock's real protection here is pull's merge/rebase
/// phase rewriting working-tree files, which must stay serialized against `file_save` and every
/// other app file mutation. Splitting the network fetch phase out of the guard (the axis-A goal,
/// contract 2026-08-19 §1.1) was audited and deliberately **not** done: `service::pull` shells
/// out to `git pull`, which fuses fetch and the config-dependent integration step (merge vs
/// `pull.rebase` vs `branch.<name>.rebase`) inside one subprocess — replicating the split as
/// `git fetch` outside the lock plus a hand-rolled second step would change pull semantics for
/// rebase-configured repos, so the fetch stays under the lock and the separation is deferred
/// (contract §1.0: hold over a half-correct split).
#[tauri::command]
#[specta::specta]
pub async fn git_pull(app: AppHandle, state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId) -> AppResult<()> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    let app_for_task = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let app_state = app_for_task.state::<AppState>();
        let _guard = app_state.begin_mutation_blocking();
        service::pull(&repo_root)
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(&app, &project_id);
    emit_refs_changed(&app, &project_id);
    Ok(())
}

/// Runs `git fetch` on a blocking thread **without** `AppState::begin_mutation` (audit R4#3),
/// with the same audit as [`git_push`]: `git fetch` updates remote-tracking refs and
/// `FETCH_HEAD` inside `.git` and never touches the working tree, so app file mutations needed
/// no serialization with it, and `.git` integrity against concurrent app git commands is git's
/// own ref-lock job (the terminal-git precedent). The one interleaving the old lock did exclude
/// — this fetch rewriting `FETCH_HEAD` between the fetch and merge phases *inside* a
/// concurrently running [`git_pull`] — is accepted: both fetches target the same configured
/// remote, so the for-merge `FETCH_HEAD` entries are computed from the same branch config and
/// the pull still integrates a valid fetched upstream head, identical to running `git fetch` in
/// a terminal during a pull, which git is built to tolerate.
#[tauri::command]
#[specta::specta]
pub async fn git_fetch(app: AppHandle, state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId) -> AppResult<()> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    tauri::async_runtime::spawn_blocking(move || service::fetch(&repo_root))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
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

#[tauri::command]
#[specta::specta]
pub async fn git_conflict_sides(
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    path: String,
) -> AppResult<ConflictSides> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    tauri::async_runtime::spawn_blocking(move || service::conflict_sides(&repo_root, &path))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

#[tauri::command]
#[specta::specta]
pub async fn git_resolve_conflict(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    path: String,
    content: String,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::resolve_conflict(&repo_root, &path, &content)?;
    emit_status_changed(&app, &project_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn git_stage_hunk(
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
    tauri::async_runtime::spawn_blocking(move || service::stage_hunk(&repo_root, &path, hunk_start, hunk_end))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(&app, &project_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn git_unstage_hunk(
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
    tauri::async_runtime::spawn_blocking(move || service::unstage_hunk(&repo_root, &path, hunk_start, hunk_end))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(&app, &project_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn git_stage_lines(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    path: String,
    line_start: u32,
    line_end: u32,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    tauri::async_runtime::spawn_blocking(move || service::stage_lines(&repo_root, &path, line_start, line_end))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(&app, &project_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn git_unstage_lines(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    path: String,
    line_start: u32,
    line_end: u32,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    tauri::async_runtime::spawn_blocking(move || service::unstage_lines(&repo_root, &path, line_start, line_end))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(&app, &project_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn git_commit_files(
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    rev: String,
) -> AppResult<Vec<CommitFile>> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    tauri::async_runtime::spawn_blocking(move || service::commit_files(&repo_root, &rev))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

#[tauri::command]
#[specta::specta]
pub async fn git_file_log(
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    path: String,
    skip: u32,
    take: u32,
) -> AppResult<Vec<LogEntry>> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    tauri::async_runtime::spawn_blocking(move || service::file_log(&repo_root, &path, skip as usize, take as usize))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

#[tauri::command]
#[specta::specta]
pub async fn git_revert_commit(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    rev: String,
) -> AppResult<RevertOutcome> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    let outcome = tauri::async_runtime::spawn_blocking(move || service::revert_commit(&repo_root, &rev))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(&app, &project_id);
    if !outcome.conflicted {
        emit_refs_changed(&app, &project_id);
    }
    Ok(outcome)
}

#[tauri::command]
#[specta::specta]
pub async fn git_tags(state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId) -> AppResult<Vec<TagInfo>> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    tauri::async_runtime::spawn_blocking(move || service::tags(&repo_root))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

#[tauri::command]
#[specta::specta]
pub async fn git_tag_create(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    name: String,
    target: String,
    opts: TagCreateOptions,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::tag_create(&repo_root, &name, &target, &opts)?;
    emit_refs_changed(&app, &project_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn git_tag_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    name: String,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::tag_delete(&repo_root, &name)?;
    emit_refs_changed(&app, &project_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn git_checkout_remote_branch(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    remote_ref: String,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    service::checkout_remote_branch(&repo_root, &remote_ref)?;
    emit_status_changed(&app, &project_id);
    emit_refs_changed(&app, &project_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove는_해당_프로젝트의_캐시된_repo_root만_지운다() {
        let store = GitStore::new();
        let closing = ProjectId::new();
        let staying = ProjectId::new();
        store.0.lock().insert(closing.clone(), PathBuf::from("/tmp/closing-repo"));
        store.0.lock().insert(staying.clone(), PathBuf::from("/tmp/staying-repo"));

        store.remove(&closing);

        assert!(!store.0.lock().contains_key(&closing), "닫힌 프로젝트의 캐시는 제거되어야 한다");
        assert!(store.0.lock().contains_key(&staying), "다른 프로젝트의 캐시는 남아 있어야 한다");
    }
}

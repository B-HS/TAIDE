use std::path::{Path, PathBuf};
use std::time::Instant;

use taide_git::service;
use taide_git::store::{GitStore, StatusRead};
use taide_infra::language::LanguageOverlay;
use taide_infra::perf::{self, SpanSlot};
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::git::{
    BlameLine, CommitFile, CommitOptions, ConflictSides, DiffMode, DiffSides, GitBranch, GitRemote, GitStashEntry, GitStatus, GutterHunk,
    LogEntry, RevertOutcome, StagedDiffText, TagCreateOptions, TagInfo,
};
use taide_model::ids::ProjectId;

use crate::{AppState, EventSink};

fn resolve_repo_root(state: &AppState, store: &GitStore, project_id: &ProjectId) -> AppResult<PathBuf> {
    if let Some(cached) = store.cached_repo_root(project_id) {
        return Ok(cached);
    }

    let root = state
        .projects
        .read()
        .get(project_id)
        .map(|project| project.root.clone())
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))?;

    let repo_root = service::discover(Path::new(&root))?;
    store.cache_repo_root(project_id.clone(), repo_root.clone());
    Ok(repo_root)
}

fn emit_status_changed(events: &dyn EventSink, store: &GitStore, project_id: &ProjectId) {
    store.invalidate_status(project_id);
    events.publish(AppEvent::GitStatusChanged {
        project_id: project_id.clone(),
    });
}

fn emit_refs_changed(events: &dyn EventSink, store: &GitStore, project_id: &ProjectId) {
    store.invalidate_status(project_id);
    events.publish(AppEvent::GitRefsChanged {
        project_id: project_id.clone(),
    });
}

/// Executes the shared Git action without toolkit state.
pub async fn git_init(events: &dyn EventSink, state: &AppState, store: &GitStore, project_id: ProjectId) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let root = state
        .projects
        .read()
        .get(&project_id)
        .map(|project| project.root.clone())
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))?;

    tokio::task::spawn_blocking(move || service::init(Path::new(&root)))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    store.remove(&project_id);
    emit_status_changed(events, store, &project_id);
    emit_refs_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_status(
    install_invalidation_listeners: impl FnOnce(),
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
) -> AppResult<GitStatus> {
    let _span = perf::span(SpanSlot::GitStatus);
    install_invalidation_listeners();
    let repo_root = resolve_repo_root(state, store, &project_id)?;

    let pending = match store.read_status(&project_id, Instant::now()) {
        StatusRead::Fresh(status) => return Ok(status),
        StatusRead::Stale(pending) => pending,
    };

    let status = tokio::task::spawn_blocking(move || service::status(&repo_root))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    store.finish_status(&project_id, pending, &status);
    Ok(status)
}

/// Executes the shared Git action without toolkit state.
pub async fn git_diff_file(
    state: &AppState,
    store: &GitStore,
    load_language_overlays: impl FnOnce() -> Vec<LanguageOverlay>,
    project_id: ProjectId,
    path: String,
    mode: DiffMode,
    before_path: Option<String>,
) -> AppResult<DiffSides> {
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let language_overlays = load_language_overlays();
    tokio::task::spawn_blocking(move || service::diff_file(&repo_root, &path, mode, before_path.as_deref(), &language_overlays))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_diff_staged_text(state: &AppState, store: &GitStore, project_id: ProjectId) -> AppResult<StagedDiffText> {
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::diff_staged_text(&repo_root))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_show_file(state: &AppState, store: &GitStore, project_id: ProjectId, rev: String, path: String) -> AppResult<String> {
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::show_file(&repo_root, &rev, &path))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_log(state: &AppState, store: &GitStore, project_id: ProjectId, skip: u32, take: u32) -> AppResult<Vec<LogEntry>> {
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::log(&repo_root, skip as usize, take as usize))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_ahead_behind(state: &AppState, store: &GitStore, project_id: ProjectId) -> AppResult<service::AheadBehind> {
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::ahead_behind(&repo_root))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_remotes(state: &AppState, store: &GitStore, project_id: ProjectId) -> AppResult<Vec<GitRemote>> {
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::remotes(&repo_root))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_gutter(state: &AppState, store: &GitStore, project_id: ProjectId, path: String) -> AppResult<Vec<GutterHunk>> {
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::gutter(&repo_root, &path))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_blame_range(
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    path: String,
    from: u32,
    to: u32,
) -> AppResult<Vec<BlameLine>> {
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::blame_range(&repo_root, &path, from, to))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_stage(
    events: &dyn EventSink,
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    paths: Vec<String>,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::stage(&repo_root, &paths))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_unstage(
    events: &dyn EventSink,
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    paths: Vec<String>,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::unstage(&repo_root, &paths))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_discard(
    events: &dyn EventSink,
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    paths: Vec<String>,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::discard(&repo_root, &paths))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_commit(
    events: &dyn EventSink,
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    message: String,
    opts: CommitOptions,
) -> AppResult<String> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let oid = tokio::task::spawn_blocking(move || service::commit(&repo_root, &message, &opts))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(events, store, &project_id);
    emit_refs_changed(events, store, &project_id);
    Ok(oid)
}

/// Executes the shared Git action without toolkit state.
pub async fn git_push(events: &dyn EventSink, state: &AppState, store: &GitStore, project_id: ProjectId) -> AppResult<()> {
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let repo_lock = store.push_fetch_lock(&repo_root);
    let _repo_guard = repo_lock.lock().await;
    tokio::task::spawn_blocking(move || service::push(&repo_root))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_refs_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_pull(events: &dyn EventSink, state: &AppState, store: &GitStore, project_id: ProjectId) -> AppResult<()> {
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let _guard = state.begin_mutation().await;
    tokio::task::spawn_blocking(move || service::pull(&repo_root))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(events, store, &project_id);
    emit_refs_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_fetch(events: &dyn EventSink, state: &AppState, store: &GitStore, project_id: ProjectId) -> AppResult<()> {
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let repo_lock = store.push_fetch_lock(&repo_root);
    let _repo_guard = repo_lock.lock().await;
    tokio::task::spawn_blocking(move || service::fetch(&repo_root))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_refs_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_current_user(state: &AppState, store: &GitStore, project_id: ProjectId) -> AppResult<Option<String>> {
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::current_user(&repo_root))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_branches(state: &AppState, store: &GitStore, project_id: ProjectId) -> AppResult<Vec<GitBranch>> {
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::branches(&repo_root))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_branch_create(
    events: &dyn EventSink,
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    name: String,
    checkout: bool,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::branch_create(&repo_root, &name, checkout))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_refs_changed(events, store, &project_id);
    if checkout {
        emit_status_changed(events, store, &project_id);
    }
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_branch_checkout(
    events: &dyn EventSink,
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    name: String,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::branch_checkout(&repo_root, &name))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(events, store, &project_id);
    emit_refs_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_branch_delete(
    events: &dyn EventSink,
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    name: String,
    force: bool,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::branch_delete(&repo_root, &name, force))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_refs_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_stash_list(state: &AppState, store: &GitStore, project_id: ProjectId) -> AppResult<Vec<GitStashEntry>> {
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::stash_list(&repo_root))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_stash_push(
    events: &dyn EventSink,
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    message: Option<String>,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::stash_push(&repo_root, message.as_deref()))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_stash_apply(
    events: &dyn EventSink,
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    index: u32,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::stash_apply(&repo_root, index))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_stash_drop(state: &AppState, store: &GitStore, project_id: ProjectId, index: u32) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::stash_drop(&repo_root, index))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_discard_hunk(
    events: &dyn EventSink,
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    path: String,
    hunk_start: u32,
    hunk_end: u32,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::discard_hunk(&repo_root, &path, hunk_start, hunk_end))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_undo_last_commit(events: &dyn EventSink, state: &AppState, store: &GitStore, project_id: ProjectId) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::undo_last_commit(&repo_root))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(events, store, &project_id);
    emit_refs_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_conflict_sides(state: &AppState, store: &GitStore, project_id: ProjectId, path: String) -> AppResult<ConflictSides> {
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::conflict_sides(&repo_root, &path))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_resolve_conflict(
    events: &dyn EventSink,
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    path: String,
    content: String,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::resolve_conflict(&repo_root, &path, &content))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_stage_hunk(
    events: &dyn EventSink,
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    path: String,
    hunk_start: u32,
    hunk_end: u32,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::stage_hunk(&repo_root, &path, hunk_start, hunk_end))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_unstage_hunk(
    events: &dyn EventSink,
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    path: String,
    hunk_start: u32,
    hunk_end: u32,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::unstage_hunk(&repo_root, &path, hunk_start, hunk_end))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_stage_lines(
    events: &dyn EventSink,
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    path: String,
    line_start: u32,
    line_end: u32,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::stage_lines(&repo_root, &path, line_start, line_end))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_unstage_lines(
    events: &dyn EventSink,
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    path: String,
    line_start: u32,
    line_end: u32,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::unstage_lines(&repo_root, &path, line_start, line_end))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_commit_files(state: &AppState, store: &GitStore, project_id: ProjectId, rev: String) -> AppResult<Vec<CommitFile>> {
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::commit_files(&repo_root, &rev))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_file_log(
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    path: String,
    skip: u32,
    take: u32,
) -> AppResult<Vec<LogEntry>> {
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::file_log(&repo_root, &path, skip as usize, take as usize))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_revert_commit(
    events: &dyn EventSink,
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    rev: String,
) -> AppResult<RevertOutcome> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let outcome = tokio::task::spawn_blocking(move || service::revert_commit(&repo_root, &rev))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(events, store, &project_id);
    if !outcome.conflicted {
        emit_refs_changed(events, store, &project_id);
    }
    Ok(outcome)
}

/// Executes the shared Git action without toolkit state.
pub async fn git_tags(state: &AppState, store: &GitStore, project_id: ProjectId) -> AppResult<Vec<TagInfo>> {
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::tags(&repo_root))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_tag_create(
    events: &dyn EventSink,
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    name: String,
    target: String,
    opts: TagCreateOptions,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::tag_create(&repo_root, &name, &target, &opts))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_refs_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_tag_delete(
    events: &dyn EventSink,
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    name: String,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::tag_delete(&repo_root, &name))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_refs_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_checkout_remote_branch(
    events: &dyn EventSink,
    state: &AppState,
    store: &GitStore,
    project_id: ProjectId,
    remote_ref: String,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    tokio::task::spawn_blocking(move || service::checkout_remote_branch(&repo_root, &remote_ref))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(events, store, &project_id);
    emit_refs_changed(events, store, &project_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use taide_model::paths::AppPaths;
    use uuid::Uuid;

    use super::*;

    struct InspectingSink {
        store: GitStore,
        events: Mutex<Vec<AppEvent>>,
    }

    impl EventSink for InspectingSink {
        fn publish(&self, event: AppEvent) {
            let project_id = match &event {
                AppEvent::GitStatusChanged { project_id } | AppEvent::GitRefsChanged { project_id } => project_id,
                _ => panic!("Git 이벤트만 허용하는 fixture"),
            };
            assert!(matches!(self.store.read_status(project_id, Instant::now()), StatusRead::Stale(_)));
            self.events.lock().unwrap().push(event);
        }
    }

    fn cached_fixture() -> (GitStore, ProjectId, InspectingSink) {
        let store = GitStore::new();
        let project_id = ProjectId::from("prj-cached-fixture".to_string());
        let StatusRead::Stale(pending) = store.read_status(&project_id, Instant::now()) else {
            panic!("빈 cache는 stale이어야 한다");
        };
        store.finish_status(
            &project_id,
            pending,
            &GitStatus {
                rows: Vec::new(),
                branch: None,
                ahead: 0,
                behind: 0,
                has_remote: false,
            },
        );
        let sink = InspectingSink {
            store: store.clone(),
            events: Mutex::new(Vec::new()),
        };
        (store, project_id, sink)
    }

    #[test]
    fn status_이벤트는_cache를_먼저_무효화한다() {
        let (store, project_id, sink) = cached_fixture();
        emit_status_changed(&sink, &store, &project_id);
        assert_eq!(*sink.events.lock().unwrap(), [AppEvent::GitStatusChanged { project_id }]);
    }

    #[test]
    fn refs_이벤트도_cache를_먼저_무효화한다() {
        let (store, project_id, sink) = cached_fixture();
        emit_refs_changed(&sink, &store, &project_id);
        assert_eq!(*sink.events.lock().unwrap(), [AppEvent::GitRefsChanged { project_id }]);
    }

    #[test]
    fn status와_refs의_발행_순서는_보존된다() {
        let (store, project_id, sink) = cached_fixture();
        emit_status_changed(&sink, &store, &project_id);
        emit_refs_changed(&sink, &store, &project_id);
        assert_eq!(
            *sink.events.lock().unwrap(),
            [
                AppEvent::GitStatusChanged {
                    project_id: project_id.clone()
                },
                AppEvent::GitRefsChanged { project_id },
            ]
        );
    }

    #[test]
    fn repo_root는_같은_project_id의_기존_cache를_먼저_소비한다() {
        let dir = std::env::temp_dir().join(format!("taide-git-root-{}", Uuid::new_v4()));
        let state = AppState::new(AppPaths::new(dir.clone()));
        let store = GitStore::new();
        let project_id = ProjectId::from("prj-root-fixture".to_string());
        store.cache_repo_root(project_id.clone(), dir.clone());
        assert_eq!(resolve_repo_root(&state, &store, &project_id).unwrap(), dir);
        assert!(matches!(
            resolve_repo_root(&state, &store, &ProjectId::from("prj-other".to_string())),
            Err(AppError::NotFound(_))
        ));
        assert!(!state.paths.data_dir.exists());
    }
}

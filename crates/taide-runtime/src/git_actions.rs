use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{oneshot, OwnedMutexGuard};

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

use crate::{AppState, EventSink, TaskOperationLease, TaskSupervisor};

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

/// Supplies shared runtime state and ownership for Git actions.
#[derive(Clone, Copy)]
pub struct GitActionContext<'a> {
    state: &'a AppState,
    store: &'a GitStore,
    tasks: &'a TaskSupervisor,
}

impl<'a> GitActionContext<'a> {
    /// Uses the supervisor registered by the application host.
    pub fn new(state: &'a AppState, store: &'a GitStore, tasks: &'a TaskSupervisor) -> Self {
        Self { state, store, tasks }
    }
}

#[derive(Clone)]
struct GitOperation {
    _owner: Arc<GitOperationInner>,
}

struct GitOperationInner {
    _guard: Option<OwnedMutexGuard<()>>,
    _lease: TaskOperationLease,
}

impl GitOperation {
    fn begin(tasks: &TaskSupervisor, guard: Option<OwnedMutexGuard<()>>) -> AppResult<Self> {
        let lease = tasks.begin_operation("git-operation").ok_or_else(git_shutdown_error)?;
        Ok(Self {
            _owner: Arc::new(GitOperationInner {
                _guard: guard,
                _lease: lease,
            }),
        })
    }
}

fn git_shutdown_error() -> AppError {
    AppError::Forbidden("git runtime is shutting down".to_string())
}

async fn run_git_worker<T: Send + 'static>(
    tasks: &TaskSupervisor,
    operation: &GitOperation,
    work: impl FnOnce() -> T + Send + 'static,
) -> AppResult<T> {
    let (sender, receiver) = oneshot::channel();
    let owner = operation.clone();
    let worker = tasks
        .spawn_blocking_transient_handle("git-worker", move || {
            let _owner = owner;
            drop(sender.send(work()));
        })
        .ok_or_else(git_shutdown_error)?;
    worker.await.map_err(|error| {
        if error.is_cancelled() {
            git_shutdown_error()
        } else {
            AppError::Internal(error.to_string())
        }
    })?;
    receiver.await.map_err(|_| git_shutdown_error())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_init(events: &dyn EventSink, context: GitActionContext<'_>, project_id: ProjectId) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let root = state
        .projects
        .read()
        .get(&project_id)
        .map(|project| project.root.clone())
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))?;

    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || service::init(Path::new(&root))).await??;
    store.remove(&project_id);
    emit_status_changed(events, store, &project_id);
    emit_refs_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_status(
    install_invalidation_listeners: impl FnOnce(),
    context: GitActionContext<'_>,
    project_id: ProjectId,
) -> AppResult<GitStatus> {
    let GitActionContext { state, store, tasks } = context;
    let _span = perf::span(SpanSlot::GitStatus);
    install_invalidation_listeners();
    let repo_root = resolve_repo_root(state, store, &project_id)?;

    let pending = match store.read_status(&project_id, Instant::now()) {
        StatusRead::Fresh(status) => return Ok(status),
        StatusRead::Stale(pending) => pending,
    };

    let operation = GitOperation::begin(tasks, None)?;
    let status = run_git_worker(tasks, &operation, move || service::status(&repo_root)).await??;
    store.finish_status(&project_id, pending, &status);
    Ok(status)
}

/// Executes the shared Git action without toolkit state.
pub async fn git_diff_file(
    context: GitActionContext<'_>,
    load_language_overlays: impl FnOnce() -> Vec<LanguageOverlay>,
    project_id: ProjectId,
    path: String,
    mode: DiffMode,
    before_path: Option<String>,
) -> AppResult<DiffSides> {
    let GitActionContext { state, store, tasks } = context;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let language_overlays = load_language_overlays();
    let operation = GitOperation::begin(tasks, None)?;
    run_git_worker(tasks, &operation, move || {
        service::diff_file(&repo_root, &path, mode, before_path.as_deref(), &language_overlays)
    })
    .await?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_diff_staged_text(context: GitActionContext<'_>, project_id: ProjectId) -> AppResult<StagedDiffText> {
    let GitActionContext { state, store, tasks } = context;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, None)?;
    run_git_worker(tasks, &operation, move || service::diff_staged_text(&repo_root)).await?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_show_file(context: GitActionContext<'_>, project_id: ProjectId, rev: String, path: String) -> AppResult<String> {
    let GitActionContext { state, store, tasks } = context;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, None)?;
    run_git_worker(tasks, &operation, move || service::show_file(&repo_root, &rev, &path)).await?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_log(context: GitActionContext<'_>, project_id: ProjectId, skip: u32, take: u32) -> AppResult<Vec<LogEntry>> {
    let GitActionContext { state, store, tasks } = context;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, None)?;
    run_git_worker(tasks, &operation, move || service::log(&repo_root, skip as usize, take as usize)).await?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_ahead_behind(context: GitActionContext<'_>, project_id: ProjectId) -> AppResult<service::AheadBehind> {
    let GitActionContext { state, store, tasks } = context;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, None)?;
    run_git_worker(tasks, &operation, move || service::ahead_behind(&repo_root)).await?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_remotes(context: GitActionContext<'_>, project_id: ProjectId) -> AppResult<Vec<GitRemote>> {
    let GitActionContext { state, store, tasks } = context;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, None)?;
    run_git_worker(tasks, &operation, move || service::remotes(&repo_root)).await?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_gutter(context: GitActionContext<'_>, project_id: ProjectId, path: String) -> AppResult<Vec<GutterHunk>> {
    let GitActionContext { state, store, tasks } = context;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, None)?;
    run_git_worker(tasks, &operation, move || service::gutter(&repo_root, &path)).await?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_blame_range(
    context: GitActionContext<'_>,
    project_id: ProjectId,
    path: String,
    from: u32,
    to: u32,
) -> AppResult<Vec<BlameLine>> {
    let GitActionContext { state, store, tasks } = context;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, None)?;
    run_git_worker(tasks, &operation, move || service::blame_range(&repo_root, &path, from, to)).await?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_stage(events: &dyn EventSink, context: GitActionContext<'_>, project_id: ProjectId, paths: Vec<String>) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || service::stage(&repo_root, &paths)).await??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_unstage(
    events: &dyn EventSink,
    context: GitActionContext<'_>,
    project_id: ProjectId,
    paths: Vec<String>,
) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || service::unstage(&repo_root, &paths)).await??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_discard(
    events: &dyn EventSink,
    context: GitActionContext<'_>,
    project_id: ProjectId,
    paths: Vec<String>,
) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || service::discard(&repo_root, &paths)).await??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_commit(
    events: &dyn EventSink,
    context: GitActionContext<'_>,
    project_id: ProjectId,
    message: String,
    opts: CommitOptions,
) -> AppResult<String> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    let oid = run_git_worker(tasks, &operation, move || service::commit(&repo_root, &message, &opts)).await??;
    emit_status_changed(events, store, &project_id);
    emit_refs_changed(events, store, &project_id);
    Ok(oid)
}

/// Executes the shared Git action without toolkit state.
pub async fn git_push(events: &dyn EventSink, context: GitActionContext<'_>, project_id: ProjectId) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let repo_lock = store.push_fetch_lock(&repo_root);
    let _repo_guard = repo_lock.lock_owned().await;
    let operation = GitOperation::begin(tasks, Some(_repo_guard))?;
    run_git_worker(tasks, &operation, move || service::push(&repo_root)).await??;
    emit_refs_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_pull(events: &dyn EventSink, context: GitActionContext<'_>, project_id: ProjectId) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let _guard = state.begin_owned_mutation().await;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || service::pull(&repo_root)).await??;
    emit_status_changed(events, store, &project_id);
    emit_refs_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_fetch(events: &dyn EventSink, context: GitActionContext<'_>, project_id: ProjectId) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let repo_lock = store.push_fetch_lock(&repo_root);
    let _repo_guard = repo_lock.lock_owned().await;
    let operation = GitOperation::begin(tasks, Some(_repo_guard))?;
    run_git_worker(tasks, &operation, move || service::fetch(&repo_root)).await??;
    emit_refs_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_current_user(context: GitActionContext<'_>, project_id: ProjectId) -> AppResult<Option<String>> {
    let GitActionContext { state, store, tasks } = context;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, None)?;
    run_git_worker(tasks, &operation, move || service::current_user(&repo_root)).await?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_branches(context: GitActionContext<'_>, project_id: ProjectId) -> AppResult<Vec<GitBranch>> {
    let GitActionContext { state, store, tasks } = context;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, None)?;
    run_git_worker(tasks, &operation, move || service::branches(&repo_root)).await?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_branch_create(
    events: &dyn EventSink,
    context: GitActionContext<'_>,
    project_id: ProjectId,
    name: String,
    checkout: bool,
) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || service::branch_create(&repo_root, &name, checkout)).await??;
    emit_refs_changed(events, store, &project_id);
    if checkout {
        emit_status_changed(events, store, &project_id);
    }
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_branch_checkout(
    events: &dyn EventSink,
    context: GitActionContext<'_>,
    project_id: ProjectId,
    name: String,
) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || service::branch_checkout(&repo_root, &name)).await??;
    emit_status_changed(events, store, &project_id);
    emit_refs_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_branch_delete(
    events: &dyn EventSink,
    context: GitActionContext<'_>,
    project_id: ProjectId,
    name: String,
    force: bool,
) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || service::branch_delete(&repo_root, &name, force)).await??;
    emit_refs_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_stash_list(context: GitActionContext<'_>, project_id: ProjectId) -> AppResult<Vec<GitStashEntry>> {
    let GitActionContext { state, store, tasks } = context;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, None)?;
    run_git_worker(tasks, &operation, move || service::stash_list(&repo_root)).await?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_stash_push(
    events: &dyn EventSink,
    context: GitActionContext<'_>,
    project_id: ProjectId,
    message: Option<String>,
) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || service::stash_push(&repo_root, message.as_deref())).await??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_stash_apply(events: &dyn EventSink, context: GitActionContext<'_>, project_id: ProjectId, index: u32) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || service::stash_apply(&repo_root, index)).await??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_stash_drop(context: GitActionContext<'_>, project_id: ProjectId, index: u32) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || service::stash_drop(&repo_root, index)).await?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_discard_hunk(
    events: &dyn EventSink,
    context: GitActionContext<'_>,
    project_id: ProjectId,
    path: String,
    hunk_start: u32,
    hunk_end: u32,
) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || {
        service::discard_hunk(&repo_root, &path, hunk_start, hunk_end)
    })
    .await??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_undo_last_commit(events: &dyn EventSink, context: GitActionContext<'_>, project_id: ProjectId) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || service::undo_last_commit(&repo_root)).await??;
    emit_status_changed(events, store, &project_id);
    emit_refs_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_conflict_sides(context: GitActionContext<'_>, project_id: ProjectId, path: String) -> AppResult<ConflictSides> {
    let GitActionContext { state, store, tasks } = context;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, None)?;
    run_git_worker(tasks, &operation, move || service::conflict_sides(&repo_root, &path)).await?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_resolve_conflict(
    events: &dyn EventSink,
    context: GitActionContext<'_>,
    project_id: ProjectId,
    path: String,
    content: String,
) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || service::resolve_conflict(&repo_root, &path, &content)).await??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_stage_hunk(
    events: &dyn EventSink,
    context: GitActionContext<'_>,
    project_id: ProjectId,
    path: String,
    hunk_start: u32,
    hunk_end: u32,
) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || {
        service::stage_hunk(&repo_root, &path, hunk_start, hunk_end)
    })
    .await??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_unstage_hunk(
    events: &dyn EventSink,
    context: GitActionContext<'_>,
    project_id: ProjectId,
    path: String,
    hunk_start: u32,
    hunk_end: u32,
) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || {
        service::unstage_hunk(&repo_root, &path, hunk_start, hunk_end)
    })
    .await??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_stage_lines(
    events: &dyn EventSink,
    context: GitActionContext<'_>,
    project_id: ProjectId,
    path: String,
    line_start: u32,
    line_end: u32,
) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || {
        service::stage_lines(&repo_root, &path, line_start, line_end)
    })
    .await??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_unstage_lines(
    events: &dyn EventSink,
    context: GitActionContext<'_>,
    project_id: ProjectId,
    path: String,
    line_start: u32,
    line_end: u32,
) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || {
        service::unstage_lines(&repo_root, &path, line_start, line_end)
    })
    .await??;
    emit_status_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_commit_files(context: GitActionContext<'_>, project_id: ProjectId, rev: String) -> AppResult<Vec<CommitFile>> {
    let GitActionContext { state, store, tasks } = context;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, None)?;
    run_git_worker(tasks, &operation, move || service::commit_files(&repo_root, &rev)).await?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_file_log(
    context: GitActionContext<'_>,
    project_id: ProjectId,
    path: String,
    skip: u32,
    take: u32,
) -> AppResult<Vec<LogEntry>> {
    let GitActionContext { state, store, tasks } = context;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, None)?;
    run_git_worker(tasks, &operation, move || {
        service::file_log(&repo_root, &path, skip as usize, take as usize)
    })
    .await?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_revert_commit(
    events: &dyn EventSink,
    context: GitActionContext<'_>,
    project_id: ProjectId,
    rev: String,
) -> AppResult<RevertOutcome> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    let outcome = run_git_worker(tasks, &operation, move || service::revert_commit(&repo_root, &rev)).await??;
    emit_status_changed(events, store, &project_id);
    if !outcome.conflicted {
        emit_refs_changed(events, store, &project_id);
    }
    Ok(outcome)
}

/// Executes the shared Git action without toolkit state.
pub async fn git_tags(context: GitActionContext<'_>, project_id: ProjectId) -> AppResult<Vec<TagInfo>> {
    let GitActionContext { state, store, tasks } = context;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, None)?;
    run_git_worker(tasks, &operation, move || service::tags(&repo_root)).await?
}

/// Executes the shared Git action without toolkit state.
pub async fn git_tag_create(
    events: &dyn EventSink,
    context: GitActionContext<'_>,
    project_id: ProjectId,
    name: String,
    target: String,
    opts: TagCreateOptions,
) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || service::tag_create(&repo_root, &name, &target, &opts)).await??;
    emit_refs_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_tag_delete(events: &dyn EventSink, context: GitActionContext<'_>, project_id: ProjectId, name: String) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || service::tag_delete(&repo_root, &name)).await??;
    emit_refs_changed(events, store, &project_id);
    Ok(())
}

/// Executes the shared Git action without toolkit state.
pub async fn git_checkout_remote_branch(
    events: &dyn EventSink,
    context: GitActionContext<'_>,
    project_id: ProjectId,
    remote_ref: String,
) -> AppResult<()> {
    let GitActionContext { state, store, tasks } = context;
    let _guard = state.begin_owned_mutation().await;
    let repo_root = resolve_repo_root(state, store, &project_id)?;
    let operation = GitOperation::begin(tasks, Some(_guard))?;
    run_git_worker(tasks, &operation, move || service::checkout_remote_branch(&repo_root, &remote_ref)).await??;
    emit_status_changed(events, store, &project_id);
    emit_refs_changed(events, store, &project_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::Duration;

    use taide_lsp::install::LspInstallStore;
    use taide_lsp::store::LspStore;
    use taide_model::paths::AppPaths;
    use taide_terminal::store::TerminalStore;
    use uuid::Uuid;

    use super::*;
    use crate::ExitDrain;

    const FIXTURE_TIMEOUT_MS: u64 = 2_000;
    const BLOCKED_WORK_OBSERVATION_MS: u64 = 20;

    struct WorkerRelease(std::sync::mpsc::Sender<()>);

    impl Drop for WorkerRelease {
        fn drop(&mut self) {
            let _ = self.0.send(());
        }
    }

    async fn abort_blocked_request(tasks: &TaskSupervisor, guard: OwnedMutexGuard<()>) -> WorkerRelease {
        let (started, started_rx) = tokio::sync::oneshot::channel();
        let (release, release_rx) = std::sync::mpsc::channel();
        let release = WorkerRelease(release);
        let operation = GitOperation::begin(tasks, Some(guard)).unwrap();
        let request_tasks = tasks.clone();
        let request = tokio::spawn(async move {
            run_git_worker(&request_tasks, &operation, move || {
                started.send(()).unwrap();
                release_rx.recv().unwrap();
            })
            .await
            .unwrap();
        });
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), started_rx)
            .await
            .unwrap()
            .unwrap();
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        assert!(tasks.tracked_count() > 0);
        release
    }

    #[tokio::test]
    async fn 요청_abort_뒤에도_시작한_git_worker의_mutation_guard를_유지한다() {
        let dir = std::env::temp_dir().join(format!("taide-git-owner-{}", Uuid::new_v4()));
        let state = AppState::new(AppPaths::new(dir));
        let runtime = tokio::runtime::Handle::current();
        let tasks = TaskSupervisor::new(runtime.clone());
        let release = abort_blocked_request(&tasks, state.begin_owned_mutation().await).await;
        assert!(
            tokio::time::timeout(Duration::from_millis(BLOCKED_WORK_OBSERVATION_MS), state.begin_mutation())
                .await
                .is_err()
        );
        let mut drain = ExitDrain::default();
        let (finished, mut finished_rx) = oneshot::channel();
        assert!(drain.begin(
            &runtime,
            tasks.clone(),
            LspInstallStore::new(),
            LspStore::new(),
            TerminalStore::new(),
            move || {
                let _ = finished.send(());
            },
        ));
        assert!(
            tokio::time::timeout(Duration::from_millis(BLOCKED_WORK_OBSERVATION_MS), &mut finished_rx)
                .await
                .is_err()
        );
        assert!(!drain.is_ready());
        drop(release);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), finished_rx)
            .await
            .unwrap()
            .unwrap();
        assert!(drain.is_ready());
        assert_eq!(tasks.tracked_count(), 0);
        drop(
            tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), state.begin_mutation())
                .await
                .unwrap(),
        );
        assert!(!state.paths.data_dir.exists());
    }

    #[tokio::test]
    async fn repo_guard는_요청_abort_뒤에도_같은_repo만_직렬화하고_실제_worker_완료시_해제된다() {
        let dir = std::env::temp_dir().join(format!("taide-git-repo-owner-{}", Uuid::new_v4()));
        let state = AppState::new(AppPaths::new(dir.clone()));
        let store = GitStore::new();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let repo_lock = store.push_fetch_lock(&dir.join("repo"));
        let release = abort_blocked_request(&tasks, repo_lock.clone().lock_owned().await).await;
        assert!(
            tokio::time::timeout(Duration::from_millis(BLOCKED_WORK_OBSERVATION_MS), repo_lock.lock())
                .await
                .is_err()
        );
        drop(
            tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), state.begin_mutation())
                .await
                .unwrap(),
        );
        let other_repo = store.push_fetch_lock(&dir.join("other-repo"));
        assert!(other_repo.try_lock().is_ok());
        drop(release);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), tasks.shutdown())
            .await
            .unwrap();
        assert!(repo_lock.try_lock().is_ok());
        assert_eq!(tasks.tracked_count(), 0);
        assert!(!dir.exists());
    }

    #[tokio::test]
    async fn 정상_root는_worker_완료_뒤에도_post_await_owner와_guard의_실제_drop을_기다린다() {
        let dir = std::env::temp_dir().join(format!("taide-git-post-owner-{}", Uuid::new_v4()));
        let state = AppState::new(AppPaths::new(dir));
        let runtime = tokio::runtime::Handle::current();
        let tasks = TaskSupervisor::new(runtime.clone());
        let operation = GitOperation::begin(&tasks, Some(state.begin_owned_mutation().await)).unwrap();
        assert_eq!(
            run_git_worker(&tasks, &operation, || "fixture-result").await.unwrap(),
            "fixture-result"
        );
        assert_eq!(tasks.tracked_count(), 1);
        let mut drain = ExitDrain::default();
        let (finished, mut finished_rx) = oneshot::channel();
        assert!(drain.begin(
            &runtime,
            tasks.clone(),
            LspInstallStore::new(),
            LspStore::new(),
            TerminalStore::new(),
            move || {
                let _ = finished.send(());
            },
        ));
        assert!(
            tokio::time::timeout(Duration::from_millis(BLOCKED_WORK_OBSERVATION_MS), &mut finished_rx)
                .await
                .is_err()
        );
        assert!(!drain.is_ready());
        assert!(
            tokio::time::timeout(Duration::from_millis(BLOCKED_WORK_OBSERVATION_MS), state.begin_mutation())
                .await
                .is_err()
        );
        drop(operation);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), finished_rx)
            .await
            .unwrap()
            .unwrap();
        assert!(drain.is_ready());
        assert_eq!(tasks.tracked_count(), 0);
        drop(
            tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), state.begin_mutation())
                .await
                .unwrap(),
        );
        assert!(!state.paths.data_dir.exists());
    }

    #[tokio::test]
    async fn worker는_기존_서비스_오류와_panic을_구분하고_owner를_완료로_남기지_않는다() {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let operation = GitOperation::begin(&tasks, None).unwrap();
        assert!(matches!(
            run_git_worker(&tasks, &operation, || Err::<(), _>(AppError::InvalidArgument("fixture-service-error".to_string())))
                .await
                .unwrap(),
            Err(AppError::InvalidArgument(message)) if message == "fixture-service-error"
        ));
        assert!(matches!(
            run_git_worker(&tasks, &operation, || panic!("fixture-worker-panic")).await,
            Err(AppError::Internal(message)) if message.contains("fixture-worker-panic")
        ));
        assert_eq!(tasks.tracked_count(), 1);
        drop(operation);
        assert_eq!(tasks.tracked_count(), 0);
    }

    #[tokio::test]
    async fn stop과_worker_입장의_경합은_서비스를_실행하지_않고_거절한다() {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let operation = GitOperation::begin(&tasks, None).unwrap();
        tasks.stop_all();
        assert!(matches!(
            run_git_worker(&tasks, &operation, || panic!("closed supervisor must not run Git")).await,
            Err(AppError::Forbidden(message)) if message == "git runtime is shutting down"
        ));
        assert!(GitOperation::begin(&tasks, None).is_err());
        assert_eq!(tasks.tracked_count(), 1);
        drop(operation);
        tasks.shutdown().await;
        assert_eq!(tasks.tracked_count(), 0);
    }

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

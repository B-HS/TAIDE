use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;
use tauri::{AppHandle, State};
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

/// Two independent per-project caches, kept as separate fields (not merged into one map) because
/// they're keyed differently on purpose. `repo_roots` is keyed by `ProjectId` — the open-project
/// session `resolve_repo_root` resolves once and remembers. `push_fetch_locks` is keyed by the
/// canonical repo root path itself: the resource `git_push`/`git_fetch` actually race on is the
/// repo, not whichever session happens to be looking at it right now, and [`GitStore::
/// push_fetch_lock`]'s cross-project serialization depends on that being a path key (contract
/// 2026-08-25 §1-b).
#[derive(Default)]
pub struct GitStore {
    repo_roots: Mutex<HashMap<ProjectId, PathBuf>>,
    push_fetch_locks: Mutex<HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>>,
}

impl GitStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forgets `project_id`'s cached repo root, called by `GitCacheCapability::detach` during
    /// `project_close`, and by [`git_init`] right after re-initializing a repo at the project's
    /// root — so a project reopened at the same path (or one just re-initialized in place) resolves
    /// its repo root fresh instead of reusing a cache entry keyed by a `ProjectId` that no session
    /// will ever look up again — see `resolve_repo_root`, which otherwise happily serves that stale
    /// entry forever (the map is never pruned by size or age, only by this explicit removal).
    ///
    /// Also evicts that root's [`Self::push_fetch_lock`] entry, but **only when nothing is currently
    /// using it** — `Arc::strong_count(lock) == 1` means this map holds the only clone, i.e. no
    /// in-flight `git_push`/`git_fetch` for the closing project still has one. An in-flight call's
    /// own clone would keep the `Mutex` alive regardless of what this map does, so eviction is never
    /// a soundness risk to *that* call — what it would break is serialization for the narrower case
    /// where the same repo path is reopened as a *new* project before the old call finishes: the
    /// reopened project's next push/fetch would otherwise `entry(...).or_insert_with(...)` a fresh,
    /// unrelated `Mutex` instead of joining the still-running one, silently defeating same-repo
    /// serialization for that overlap. Skipping eviction while a clone is live avoids that; the entry
    /// is picked up by a later `remove` once the in-flight call has dropped its own guard. This is
    /// deliberately best-effort, not exhaustive: a project closed mid-push/fetch and never reopened
    /// at that root again leaves its entry in the map for the rest of the app's lifetime (the same
    /// unbounded-until-explicit-removal shape this doc's own first paragraph above already accepts
    /// for `repo_roots`) — a bounded leak of at most one `Arc<Mutex<()>>` per such project, not a
    /// growing one, since ordinary closes (the common case: no push/fetch in flight) still evict
    /// normally.
    pub fn remove(&self, project_id: &ProjectId) {
        let root = self.repo_roots.lock().remove(project_id);
        let Some(root) = root else { return };
        let mut locks = self.push_fetch_locks.lock();
        if locks.get(&root).is_some_and(|lock| Arc::strong_count(lock) == 1) {
            locks.remove(&root);
        }
    }

    /// Returns the [`tokio::sync::Mutex`] every `git_push`/`git_fetch` call for `repo_root` acquires
    /// and holds for its whole subprocess wait, creating the entry on first use — the repo-path-keyed
    /// serialization contract 2026-08-25 §1-b calls for: concurrent pushes/fetches on the *same* repo
    /// now queue instead of racing straight into git's own `.git` ref/lock-file contention (the
    /// "transient lock-contention error" contract 2026-08-19 §4's `git_fetch` doc accepted as a cost
    /// is now avoided for the push/fetch-vs-push/fetch pairing specifically; git's own locks remain
    /// the safety net for every other overlap, unchanged).
    ///
    /// Lock-ordering: neither `git_push` nor `git_fetch` holds `AppState::begin_mutation` (audit
    /// R4#3/contract 2026-08-19 §4 — network git's only local effect is a refs update, which needed
    /// no serialization against working-tree mutations), so this lock is the *only* one either
    /// command ever takes. It is never acquired while `begin_mutation` is held, and never itself
    /// guards an acquisition of `begin_mutation` — so it adds no new lock-ordering relationship for a
    /// deadlock to hide in (contract §1-b's "데드락 신설 0" requirement is met by construction, not by
    /// a fixed acquisition order between two locks that are simply never both held at once).
    ///
    /// Cross-repo concurrency is preserved because the map is keyed by path: two different repo roots
    /// always get independent `Arc`s and their push/fetch calls never wait on each other.
    ///
    /// `git_pull` deliberately never calls this — its full-repo `begin_mutation` hold is unchanged
    /// (contract §1-b: "git_pull 의 전체 락 유지 불변"), so a pull can still overlap a push/fetch on
    /// the same repo exactly as it could before this change (the accepted degradation contract
    /// 2026-08-19 §4/§5 already documented for that pair — widening this lock to include pull was
    /// out of this contract's scope and would need its own risk analysis against that unchanged
    /// guarantee).
    ///
    /// This lock's hold time is entirely at the mercy of the `run_git` subprocess it wraps:
    /// `run_git` has no timeout and no kill path (`std::process::Command::output()`, awaited to
    /// completion), so a push/fetch stalled on an unreachable remote or a blocked credential prompt
    /// holds this lock — and queues every later `git_push`/`git_fetch` for the same repo behind it
    /// — for as long as that subprocess stays alive, which can be the rest of the app's lifetime.
    /// This is not a defect this lock introduces; it's the pre-existing unbounded `run_git` wait
    /// (unchanged by this contract) now reaching same-repo callers instead of staying scoped to just
    /// the one call that triggered it. The trade is deliberate: before this lock, N concurrent
    /// stalled calls on the same repo each pinned their own blocking-pool thread indefinitely; now
    /// at most one does, and the rest wait on this async mutex instead — a bounded resource
    /// (blocking-pool exhaustion) is traded for an unbounded one (same-repo push/fetch latency).
    /// Giving `run_git` a timeout/kill path is tracked as separate follow-up work, not part of this
    /// contract (see `docs/quality-assurance/2026-08-11-qa6-checklist.md`'s d-35 section for the
    /// same-repo-stall scenario this leaves for realistic-conditions QA).
    pub fn push_fetch_lock(&self, repo_root: &Path) -> Arc<tokio::sync::Mutex<()>> {
        self.push_fetch_locks
            .lock()
            .entry(repo_root.to_path_buf())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }
}

fn resolve_repo_root(state: &State<'_, AppState>, store: &State<'_, GitStore>, project_id: &ProjectId) -> AppResult<PathBuf> {
    if let Some(cached) = store.repo_roots.lock().get(project_id) {
        return Ok(cached.clone());
    }

    let root = state
        .projects
        .read()
        .get(project_id)
        .map(|project| project.root.clone())
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))?;

    let repo_root = service::discover(Path::new(&root))?;
    store.repo_roots.lock().insert(project_id.clone(), repo_root.clone());
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

    tauri::async_runtime::spawn_blocking(move || service::init(Path::new(&root)))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    store.remove(&project_id);
    emit_status_changed(&app, &project_id);
    emit_refs_changed(&app, &project_id);
    Ok(())
}

/// Runs the status read on a blocking thread like every other git query command here: dropping
/// `update_index` (audit R4#11) made stat-stale entries re-hash on **every** call until some
/// index-writing operation refreshes them, and this event-driven path re-runs after each
/// `GitStatusChanged`, so the synchronous libgit2 work must not pin an async worker thread
/// (architecture.md §2.1, Phase E C11-GIT-2). Surface and return value are unchanged.
#[tauri::command]
#[specta::specta]
pub async fn git_status(state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId) -> AppResult<GitStatus> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    tauri::async_runtime::spawn_blocking(move || service::status(&repo_root))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
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

/// Holds `AppState::begin_mutation` across the index write — staging must stay serialized with
/// every other guarded mutation exactly as before — but runs the in-process libgit2 work
/// (`index.add_path`/`remove_path` + `index.write`) on a blocking thread: those are synchronous
/// libgit2 calls that previously pinned an async worker for the duration (architecture.md §2.1,
/// contract 2026-08-25 §1-a). Same guard-held `spawn_blocking` shape as `git_commit`/
/// `git_stage_hunk` — lock semantics unchanged, only thread-pool occupancy moved.
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
    tauri::async_runtime::spawn_blocking(move || service::stage(&repo_root, &paths))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(&app, &project_id);
    Ok(())
}

/// Same guard-held `spawn_blocking` shape as [`git_stage`] — `repo.reset_default` is synchronous
/// libgit2 work (contract 2026-08-25 §1-a).
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
    tauri::async_runtime::spawn_blocking(move || service::unstage(&repo_root, &paths))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(&app, &project_id);
    Ok(())
}

/// Same guard-held `spawn_blocking` shape as [`git_stage`] — `service::discard` mixes synchronous
/// libgit2 checkout with a filesystem trash call for untracked paths, both blocking work
/// (contract 2026-08-25 §1-a).
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
    tauri::async_runtime::spawn_blocking(move || service::discard(&repo_root, &paths))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(&app, &project_id);
    Ok(())
}

/// Holds `AppState::begin_mutation` across the whole commit — the staged-state read, the commit,
/// and the resulting index/ref writes must stay serialized with every other guarded mutation
/// exactly as before — but runs the `git` subprocesses (`add -A` when staging all, `commit`,
/// `rev-parse`) on a blocking thread: `git add -A` stats the entire working tree, seconds on a
/// large repo, which previously pinned an async worker for the duration (architecture.md §2.1,
/// audit R4#3's commit clause, Phase E T1H-C-02). Same guard-held `spawn_blocking` shape as
/// `git_revert_commit`/`git_stage_hunk` — lock semantics unchanged.
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
    let oid = tauri::async_runtime::spawn_blocking(move || service::commit(&repo_root, &message, &opts))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(&app, &project_id);
    emit_refs_changed(&app, &project_id);
    Ok(oid)
}

/// Runs `git push` on a blocking thread **without** `AppState::begin_mutation` (audit R4#3, C11
/// axis A). What the old guard actually covered was audited before removal: `service::push`
/// shells out to `git push`, which reads local refs/objects and — on success — updates the
/// remote-tracking ref inside `.git`; git itself never touches the working tree, so serializing
/// it with the app's file mutations (`file_save`, replace, ...) protected nothing. The one
/// exception is repo-configured hook code: a `pre-push` hook (or `core.hooksPath` equivalent) can
/// write arbitrary working-tree files, and those writes are no longer serialized against app
/// mutations — deliberately accepted as the same guarantee level as running `git push` in a
/// terminal beside the app, rather than freezing every mutation for a network round-trip on
/// behalf of repo-owned scripts (Phase E T1H-C5). Same-repo `.git` integrity against concurrent
/// app git commands (commit/stage/pull) is enforced by git's own index/ref locks, exactly as with
/// terminal git, and command-level ordering (commit-then-push) is already sequenced by the
/// frontend awaiting each command. The subprocess wait moved into `spawn_blocking` so the network
/// round-trip no longer pins an async worker thread either (architecture.md §2.1's other half).
///
/// Additionally holds [`GitStore::push_fetch_lock`] across the whole subprocess wait, so a second
/// `git_push`/`git_fetch` for the *same* repo queues behind this one instead of racing it straight
/// into git's own ref-lock contention (contract 2026-08-25 §1-b — see that method's doc for why
/// this introduces no new lock-ordering hazard with `begin_mutation`, and for the unbounded-wait
/// cost this accepts when the underlying `run_git` subprocess stalls).
#[tauri::command]
#[specta::specta]
pub async fn git_push(app: AppHandle, state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId) -> AppResult<()> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    let repo_lock = store.push_fetch_lock(&repo_root);
    let _repo_guard = repo_lock.lock().await;
    tauri::async_runtime::spawn_blocking(move || service::push(&repo_root))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_refs_changed(&app, &project_id);
    Ok(())
}

/// Still runs the whole `git pull` under `AppState::begin_mutation` — acquired with the async
/// `begin_mutation().await` and then **held while** the subprocess wait runs on a blocking thread
/// (the same guard-held `spawn_blocking` shape as `git_revert_commit`/`git_stage_hunk`), so the
/// wait no longer pins an async worker thread. The lock's real protection here is pull's
/// merge/rebase phase rewriting working-tree files, which must stay serialized against
/// `file_save` and every other app file mutation. Waiting for the lock **inside**
/// `spawn_blocking` (via `begin_mutation_blocking`) is deliberately avoided: a pull parked on the
/// lock would occupy a blocking-pool thread for its whole wait + network duration, and enough
/// parked pulls plus one guard holder that itself needs a blocking thread (`git_revert_commit`,
/// hunk staging, ...) deadlocks the pool (Phase E GIT-1). Splitting the network fetch phase out
/// of the guard (the axis-A goal, contract 2026-08-19 §1.1) was audited and deliberately **not**
/// done: `service::pull` shells out to `git pull`, which fuses fetch and the config-dependent
/// integration step (merge vs `pull.rebase` vs `branch.<name>.rebase`) inside one subprocess —
/// replicating the split as `git fetch` outside the lock plus a hand-rolled second step would
/// change pull semantics for rebase-configured repos, so the fetch stays under the lock and the
/// separation is deferred (contract §1.0: hold over a half-correct split).
#[tauri::command]
#[specta::specta]
pub async fn git_pull(app: AppHandle, state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId) -> AppResult<()> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    let _guard = state.begin_mutation().await;
    tauri::async_runtime::spawn_blocking(move || service::pull(&repo_root))
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
/// own ref-lock job (the terminal-git precedent). What the old lock did exclude and this accepts
/// (Phase E T1H-C6/C-09): a fetch overlapping a concurrently running [`git_pull`] on the same
/// repo. Repository integrity is preserved either way — when both proceed, the for-merge
/// `FETCH_HEAD` entries come from the same branch config against the same remote so the pull
/// still integrates a valid upstream head; but git's `.git` locks (`FETCH_HEAD.lock`, per-ref
/// locks) serialize by **failing** the loser, not by queueing it, so one side can surface a
/// transient lock-contention error where the old serialization made that impossible. That
/// retryable failure is the accepted cost — the same failure mode as running `git fetch` in a
/// terminal during a pull.
///
/// Additionally holds [`GitStore::push_fetch_lock`] across the whole subprocess wait — the same
/// addition [`git_push`] documents (contract 2026-08-25 §1-b; see that method's doc for the
/// unbounded-wait cost this accepts when the underlying `run_git` subprocess stalls). This is what
/// actually removes the "transient lock-contention error" for a same-repo push-vs-fetch or
/// fetch-vs-fetch overlap — a pairing the paragraph above does not cover: both now queue on this
/// lock instead of racing into git's own `.git` lock files. The one overlap this lock deliberately
/// still lets race (unchanged from before this addition) is fetch-vs-`git_pull`, since `git_pull`
/// never takes this lock — see this paragraph's own doc above and [`GitStore::push_fetch_lock`]'s.
#[tauri::command]
#[specta::specta]
pub async fn git_fetch(app: AppHandle, state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId) -> AppResult<()> {
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    let repo_lock = store.push_fetch_lock(&repo_root);
    let _repo_guard = repo_lock.lock().await;
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

/// Same guard-held `spawn_blocking` shape as [`git_stage`] — branch creation (+ optional
/// checkout) is synchronous libgit2 work (contract 2026-08-25 §1-a). `checkout` is `Copy`, so
/// moving it into the closure leaves the outer binding usable for the post-await branch below.
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
    tauri::async_runtime::spawn_blocking(move || service::branch_create(&repo_root, &name, checkout))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_refs_changed(&app, &project_id);
    if checkout {
        emit_status_changed(&app, &project_id);
    }
    Ok(())
}

/// Same guard-held `spawn_blocking` shape as [`git_stage`] — `repo.checkout_tree` is synchronous
/// libgit2 work (contract 2026-08-25 §1-a).
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
    tauri::async_runtime::spawn_blocking(move || service::branch_checkout(&repo_root, &name))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(&app, &project_id);
    emit_refs_changed(&app, &project_id);
    Ok(())
}

/// Same guard-held `spawn_blocking` shape as [`git_stage`] — the merge-ancestry check
/// (`repo.graph_descendant_of`) plus `branch.delete()` are synchronous libgit2 work (contract
/// 2026-08-25 §1-a).
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
    tauri::async_runtime::spawn_blocking(move || service::branch_delete(&repo_root, &name, force))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
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

/// Same guard-held `spawn_blocking` shape as [`git_stage`] — `repo.stash_save` is synchronous
/// libgit2 work (contract 2026-08-25 §1-a). `message` is moved whole into the closure and
/// `.as_deref()`'d there, since the borrow it produces can't outlive the move.
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
    tauri::async_runtime::spawn_blocking(move || service::stash_push(&repo_root, message.as_deref()))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(&app, &project_id);
    Ok(())
}

/// Same guard-held `spawn_blocking` shape as [`git_stage`] — `repo.stash_apply` (+ its safe
/// checkout) is synchronous libgit2 work (contract 2026-08-25 §1-a).
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
    tauri::async_runtime::spawn_blocking(move || service::stash_apply(&repo_root, index))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_status_changed(&app, &project_id);
    Ok(())
}

/// Same guard-held `spawn_blocking` shape as [`git_stage`] — `repo.stash_drop` is synchronous
/// libgit2 work (contract 2026-08-25 §1-a).
#[tauri::command]
#[specta::specta]
pub async fn git_stash_drop(state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId, index: u32) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let repo_root = resolve_repo_root(&state, &store, &project_id)?;
    tauri::async_runtime::spawn_blocking(move || service::stash_drop(&repo_root, index))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
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
    tauri::async_runtime::spawn_blocking(move || service::undo_last_commit(&repo_root))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
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

/// Same guard-held `spawn_blocking` shape as [`git_stage`] — the working-tree write plus
/// `index.add_path`/`index.write` are synchronous IO/libgit2 work (contract 2026-08-25 §1-a).
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
    tauri::async_runtime::spawn_blocking(move || service::resolve_conflict(&repo_root, &path, &content))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
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

/// Same guard-held `spawn_blocking` shape as [`git_stage`] — `repo.tag`/`repo.tag_lightweight`
/// are synchronous libgit2 work (contract 2026-08-25 §1-a).
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
    tauri::async_runtime::spawn_blocking(move || service::tag_create(&repo_root, &name, &target, &opts))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_refs_changed(&app, &project_id);
    Ok(())
}

/// Same guard-held `spawn_blocking` shape as [`git_stage`] — `repo.tag_delete` is synchronous
/// libgit2 work (contract 2026-08-25 §1-a).
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
    tauri::async_runtime::spawn_blocking(move || service::tag_delete(&repo_root, &name))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    emit_refs_changed(&app, &project_id);
    Ok(())
}

/// Same guard-held `spawn_blocking` shape as [`git_stage`] — creating the local tracking branch
/// (when needed) plus `repo.checkout_tree` are synchronous libgit2 work (contract 2026-08-25
/// §1-a).
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
    tauri::async_runtime::spawn_blocking(move || service::checkout_remote_branch(&repo_root, &remote_ref))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
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
        store.repo_roots.lock().insert(closing.clone(), PathBuf::from("/tmp/closing-repo"));
        store.repo_roots.lock().insert(staying.clone(), PathBuf::from("/tmp/staying-repo"));

        store.remove(&closing);

        assert!(
            !store.repo_roots.lock().contains_key(&closing),
            "닫힌 프로젝트의 캐시는 제거되어야 한다"
        );
        assert!(
            store.repo_roots.lock().contains_key(&staying),
            "다른 프로젝트의 캐시는 남아 있어야 한다"
        );
    }

    #[test]
    fn push_fetch_lock은_같은_repo_root에_대해_동일한_락을_반환한다() {
        let store = GitStore::new();
        let root = PathBuf::from("/tmp/same-repo");

        let first = store.push_fetch_lock(&root);
        let second = store.push_fetch_lock(&root);

        assert!(
            Arc::ptr_eq(&first, &second),
            "같은 repo 경로는 같은 Arc<Mutex> 를 공유해야 동시 push/fetch 가 직렬화된다"
        );
    }

    #[test]
    fn push_fetch_lock은_다른_repo_root에_대해_독립된_락을_반환한다() {
        let store = GitStore::new();

        let a = store.push_fetch_lock(&PathBuf::from("/tmp/repo-a"));
        let b = store.push_fetch_lock(&PathBuf::from("/tmp/repo-b"));

        assert!(
            !Arc::ptr_eq(&a, &b),
            "다른 repo 는 독립된 락을 가져야 서로의 push/fetch 를 막지 않는다"
        );
    }

    /// §1-b 의 핵심 동시성 계약: 동일 repo 에 대한 두 번째 획득 시도는 첫 번째가 락을 들고 있는
    /// 동안 완료되지 않아야 한다 — `git_push`/`git_fetch` 가 이 `Arc<Mutex>` 를 그대로 들고
    /// 있는 것과 동일한 모양을 재현한다(state.rs 의 `begin_mutation_blocking` 상호배제 테스트와
    /// 같은 검증 패턴).
    #[tokio::test]
    async fn 동일_repo_락은_동시_보유를_막고_다른_repo_락은_병행된다() {
        let store = Arc::new(GitStore::new());
        let root = PathBuf::from("/tmp/serialize-me");

        let held_lock = store.push_fetch_lock(&root);
        let held_guard = held_lock.lock().await;

        let waiter_store = store.clone();
        let waiter_root = root.clone();
        let waiter = tokio::spawn(async move {
            let lock = waiter_store.push_fetch_lock(&waiter_root);
            let _guard = lock.lock().await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(
            !waiter.is_finished(),
            "먼저 락을 쥔 push/fetch 가 끝나기 전까지 같은 repo 의 두 번째 호출은 대기해야 한다"
        );

        let other_repo_lock = store.push_fetch_lock(&PathBuf::from("/tmp/other-repo"));
        let other_repo_result = tokio::time::timeout(std::time::Duration::from_millis(50), other_repo_lock.lock()).await;
        assert!(
            other_repo_result.is_ok(),
            "다른 repo 의 push/fetch 는 대기 중인 동일 repo 호출과 무관하게 즉시 진행되어야 한다"
        );

        drop(held_guard);
        waiter.await.expect("대기 중이던 태스크가 패닉했다");
    }

    #[test]
    fn remove는_사용중이_아닌_push_fetch_락은_함께_제거한다() {
        let store = GitStore::new();
        let project_id = ProjectId::new();
        let root = PathBuf::from("/tmp/idle-repo");
        store.repo_roots.lock().insert(project_id.clone(), root.clone());
        store.push_fetch_lock(&root);

        store.remove(&project_id);

        assert!(
            !store.push_fetch_locks.lock().contains_key(&root),
            "아무도 쥐고 있지 않은 push/fetch 락은 프로젝트가 닫힐 때 함께 제거되어야 누수가 없다"
        );
    }

    #[test]
    fn remove는_사용중인_push_fetch_락은_보존하고_이후_close에서_회수한다() {
        let store = GitStore::new();
        let project_id = ProjectId::new();
        let root = PathBuf::from("/tmp/busy-repo");
        store.repo_roots.lock().insert(project_id.clone(), root.clone());
        let in_flight = store.push_fetch_lock(&root);

        store.remove(&project_id);
        assert!(
            store.push_fetch_locks.lock().contains_key(&root),
            "진행 중인 push/fetch 가 쥔 Arc 가 있으면 같은 repo 가 재오픈됐을 때 새 호출도 그 락을 공유해야 하므로 즉시 제거하면 안 된다"
        );

        drop(in_flight);
        store.repo_roots.lock().insert(project_id.clone(), root.clone());
        store.remove(&project_id);
        assert!(
            !store.push_fetch_locks.lock().contains_key(&root),
            "진행 중이던 호출이 끝난 뒤 다시 close 하면 그때는 회수되어야 한다"
        );
    }
}

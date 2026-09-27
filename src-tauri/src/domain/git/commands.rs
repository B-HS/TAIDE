use serde::de::DeserializeOwned;
use taide_runtime::git_actions::{self, GitActionContext};
use taide_runtime::TaskSupervisor;
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

pub use taide_git::store::GitStore;

use super::service;
use super::types::{
    BlameLine, CommitFile, CommitOptions, ConflictSides, DiffMode, DiffSides, GitBranch, GitRemote, GitStashEntry, GitStatus, GutterHunk,
    LogEntry, RevertOutcome, StagedDiffText, TagCreateOptions, TagInfo,
};
use crate::error::AppResult;
use crate::events::{FsChanged, GitRefsChanged, GitStatusChanged};
use crate::ids::ProjectId;
use crate::platform::event_sink::TauriEventSink;
use crate::plugin_port::PluginRuntimePort;
use crate::state::AppState;

fn git_context<'a>(app: &'a AppHandle, state: &'a AppState, store: &'a GitStore) -> GitActionContext<'a> {
    GitActionContext::new(state, store, app.state::<TaskSupervisor>().inner())
}

fn listen_status_invalidation<E: Event + DeserializeOwned>(app: &AppHandle, project_id_of: fn(&E) -> &ProjectId) {
    let store_handle = app.clone();
    E::listen_any(app, move |event| {
        store_handle.state::<GitStore>().invalidate_status(project_id_of(&event.payload));
    });
}

fn ensure_status_invalidation_listeners(app: &AppHandle, store: &GitStore) {
    store.ensure_invalidation_listeners(|| {
        listen_status_invalidation::<FsChanged>(app, |payload| &payload.project_id);
        listen_status_invalidation::<GitStatusChanged>(app, |payload| &payload.project_id);
        listen_status_invalidation::<GitRefsChanged>(app, |payload| &payload.project_id);
    });
}

#[tauri::command]
#[specta::specta]
pub async fn git_init(app: AppHandle, state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId) -> AppResult<()> {
    git_actions::git_init(&TauriEventSink(&app), git_context(&app, &state, &store), project_id).await
}

/// Runs the status read on a blocking thread like every other git query command here: dropping
/// `update_index` (audit R4#11) made stat-stale entries re-hash on **every** call until some
/// index-writing operation refreshes them, and this event-driven path re-runs after each
/// `GitStatusChanged`, so the synchronous libgit2 work must not pin an async worker thread
/// (architecture.md §2.1, Phase E C11-GIT-2).
///
/// Answers from [`StatusCache`] when the last result is still current, which is what keeps one file
/// save from paying a full worktree walk once per open window (research 3b §2-C — five always-mounted
/// consumers, and the `.git`/`fs:changed` invalidation axes both fan out to every window). The walk
/// itself, the returned value and the IPC surface are unchanged; a cache hit differs from a miss only
/// in not having recomputed a result that nothing has invalidated. `update_index` stays off — it is a
/// separate decision this cache deliberately does not revisit (research 3b §7, 사용자 2차 결정 7).
///
/// The `app` parameter carries no wire surface (`AppHandle`/`State` arguments are stripped from the
/// generated bindings, as `git_init`'s already are) and exists only to let the cache install its
/// subscriptions on first use — see [`GitStore::ensure_invalidation_listeners`].
#[tauri::command]
#[specta::specta]
pub async fn git_status(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
) -> AppResult<GitStatus> {
    git_actions::git_status(
        || ensure_status_invalidation_listeners(&app, &store),
        git_context(&app, &state, &store),
        project_id,
    )
    .await
}

/// `before_path` is the row's pre-change path and feeds the **original (left) side only** — the
/// modified side is always `path`. Pass it for a renamed row (the HEAD entry of a staged rename, and
/// the index entry of an unstaged one, both live at the old path); reading both sides at the new path
/// left the original empty and drew the whole file as an addition instead of its actual edit (audit
/// §4-B B11). `null` keeps both sides on `path`, which is what every non-rename row wants.
#[tauri::command]
#[specta::specta]
pub async fn git_diff_file(
    app: AppHandle,
    store: State<'_, GitStore>,
    plugins: State<'_, PluginRuntimePort>,
    project_id: ProjectId,
    path: String,
    mode: DiffMode,
    before_path: Option<String>,
) -> AppResult<DiffSides> {
    git_actions::git_diff_file(
        git_context(&app, &app.state::<AppState>(), &store),
        || (plugins.language_overlays)(&app),
        project_id,
        path,
        mode,
        before_path,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn git_diff_staged_text(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
) -> AppResult<StagedDiffText> {
    git_actions::git_diff_staged_text(git_context(&app, &state, &store), project_id).await
}

/// Same query-side `spawn_blocking` shape as [`git_status`] — reading a tree entry and its blob out of
/// the object database is synchronous libgit2 IO (§2 M-1).
#[tauri::command]
#[specta::specta]
pub async fn git_show_file(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    rev: String,
    path: String,
) -> AppResult<String> {
    git_actions::git_show_file(git_context(&app, &state, &store), project_id, rev, path).await
}

#[tauri::command]
#[specta::specta]
pub async fn git_log(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    skip: u32,
    take: u32,
) -> AppResult<Vec<LogEntry>> {
    git_actions::git_log(git_context(&app, &state, &store), project_id, skip, take).await
}

/// Same query-side `spawn_blocking` shape as [`git_status`] — the two `graph_ahead_behind` revwalks are
/// synchronous libgit2 work (§2 M-1).
#[tauri::command]
#[specta::specta]
pub async fn git_ahead_behind(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
) -> AppResult<service::AheadBehind> {
    git_actions::git_ahead_behind(git_context(&app, &state, &store), project_id).await
}

/// Same query-side `spawn_blocking` shape as [`git_status`] — opening the repository reads `.git/config`
/// off disk (§2 M-1).
#[tauri::command]
#[specta::specta]
pub async fn git_remotes(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
) -> AppResult<Vec<GitRemote>> {
    git_actions::git_remotes(git_context(&app, &state, &store), project_id).await
}

/// Same query-side `spawn_blocking` shape as [`git_status`], and the one that mattered most: this is the
/// editor's gutter hot path — re-run on every `fs:changed` for the open file — and it diffs HEAD against
/// the working tree, reading the file off disk each time (§2 M-1).
#[tauri::command]
#[specta::specta]
pub async fn git_gutter(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    path: String,
) -> AppResult<Vec<GutterHunk>> {
    git_actions::git_gutter(git_context(&app, &state, &store), project_id, path).await
}

#[tauri::command]
#[specta::specta]
pub async fn git_blame_range(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    path: String,
    from: u32,
    to: u32,
) -> AppResult<Vec<BlameLine>> {
    git_actions::git_blame_range(git_context(&app, &state, &store), project_id, path, from, to).await
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
    git_actions::git_stage(&TauriEventSink(&app), git_context(&app, &state, &store), project_id, paths).await
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
    git_actions::git_unstage(&TauriEventSink(&app), git_context(&app, &state, &store), project_id, paths).await
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
    git_actions::git_discard(&TauriEventSink(&app), git_context(&app, &state, &store), project_id, paths).await
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
    git_actions::git_commit(&TauriEventSink(&app), git_context(&app, &state, &store), project_id, message, opts).await
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
/// this introduces no new lock-ordering hazard with `begin_mutation`, and for the queueing
/// cost this accepts when the underlying `run_git` subprocess stalls).
#[tauri::command]
#[specta::specta]
pub async fn git_push(app: AppHandle, state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId) -> AppResult<()> {
    git_actions::git_push(&TauriEventSink(&app), git_context(&app, &state, &store), project_id).await
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
    git_actions::git_pull(&TauriEventSink(&app), git_context(&app, &state, &store), project_id).await
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
/// addition [`git_push`] documents (contract 2026-08-25 §1-b; see that method's doc for the queueing
/// cost this accepts when the underlying `run_git` subprocess stalls). This is what
/// actually removes the "transient lock-contention error" for a same-repo push-vs-fetch or
/// fetch-vs-fetch overlap — a pairing the paragraph above does not cover: both now queue on this
/// lock instead of racing into git's own `.git` lock files. The one overlap this lock deliberately
/// still lets race (unchanged from before this addition) is fetch-vs-`git_pull`, since `git_pull`
/// never takes this lock — see this paragraph's own doc above and [`GitStore::push_fetch_lock`]'s.
#[tauri::command]
#[specta::specta]
pub async fn git_fetch(app: AppHandle, state: State<'_, AppState>, store: State<'_, GitStore>, project_id: ProjectId) -> AppResult<()> {
    git_actions::git_fetch(&TauriEventSink(&app), git_context(&app, &state, &store), project_id).await
}

/// Same query-side `spawn_blocking` shape as [`git_status`] — `repo.signature()` resolves the identity
/// through the repository/global/system config files on disk (§2 M-1).
#[tauri::command]
#[specta::specta]
pub async fn git_current_user(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
) -> AppResult<Option<String>> {
    git_actions::git_current_user(git_context(&app, &state, &store), project_id).await
}

/// Same query-side `spawn_blocking` shape as [`git_status`] — enumerating branches walks the loose refs
/// and packed-refs file, and each entry's upstream lookup reads config (§2 M-1).
#[tauri::command]
#[specta::specta]
pub async fn git_branches(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
) -> AppResult<Vec<GitBranch>> {
    git_actions::git_branches(git_context(&app, &state, &store), project_id).await
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
    git_actions::git_branch_create(&TauriEventSink(&app), git_context(&app, &state, &store), project_id, name, checkout).await
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
    git_actions::git_branch_checkout(&TauriEventSink(&app), git_context(&app, &state, &store), project_id, name).await
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
    git_actions::git_branch_delete(&TauriEventSink(&app), git_context(&app, &state, &store), project_id, name, force).await
}

/// Same query-side `spawn_blocking` shape as [`git_status`] — `repo.stash_foreach` walks the stash reflog
/// on disk (§2 M-1).
#[tauri::command]
#[specta::specta]
pub async fn git_stash_list(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
) -> AppResult<Vec<GitStashEntry>> {
    git_actions::git_stash_list(git_context(&app, &state, &store), project_id).await
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
    git_actions::git_stash_push(&TauriEventSink(&app), git_context(&app, &state, &store), project_id, message).await
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
    git_actions::git_stash_apply(&TauriEventSink(&app), git_context(&app, &state, &store), project_id, index).await
}

/// Same guard-held `spawn_blocking` shape as [`git_stage`] — `repo.stash_drop` is synchronous
/// libgit2 work (contract 2026-08-25 §1-a).
#[tauri::command]
#[specta::specta]
pub async fn git_stash_drop(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    index: u32,
) -> AppResult<()> {
    git_actions::git_stash_drop(git_context(&app, &state, &store), project_id, index).await
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
    git_actions::git_discard_hunk(
        &TauriEventSink(&app),
        git_context(&app, &state, &store),
        project_id,
        path,
        hunk_start,
        hunk_end,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn git_undo_last_commit(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
) -> AppResult<()> {
    git_actions::git_undo_last_commit(&TauriEventSink(&app), git_context(&app, &state, &store), project_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn git_conflict_sides(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    path: String,
) -> AppResult<ConflictSides> {
    git_actions::git_conflict_sides(git_context(&app, &state, &store), project_id, path).await
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
    git_actions::git_resolve_conflict(&TauriEventSink(&app), git_context(&app, &state, &store), project_id, path, content).await
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
    git_actions::git_stage_hunk(
        &TauriEventSink(&app),
        git_context(&app, &state, &store),
        project_id,
        path,
        hunk_start,
        hunk_end,
    )
    .await
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
    git_actions::git_unstage_hunk(
        &TauriEventSink(&app),
        git_context(&app, &state, &store),
        project_id,
        path,
        hunk_start,
        hunk_end,
    )
    .await
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
    git_actions::git_stage_lines(
        &TauriEventSink(&app),
        git_context(&app, &state, &store),
        project_id,
        path,
        line_start,
        line_end,
    )
    .await
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
    git_actions::git_unstage_lines(
        &TauriEventSink(&app),
        git_context(&app, &state, &store),
        project_id,
        path,
        line_start,
        line_end,
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn git_commit_files(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    rev: String,
) -> AppResult<Vec<CommitFile>> {
    git_actions::git_commit_files(git_context(&app, &state, &store), project_id, rev).await
}

#[tauri::command]
#[specta::specta]
pub async fn git_file_log(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
    path: String,
    skip: u32,
    take: u32,
) -> AppResult<Vec<LogEntry>> {
    git_actions::git_file_log(git_context(&app, &state, &store), project_id, path, skip, take).await
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
    git_actions::git_revert_commit(&TauriEventSink(&app), git_context(&app, &state, &store), project_id, rev).await
}

#[tauri::command]
#[specta::specta]
pub async fn git_tags(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, GitStore>,
    project_id: ProjectId,
) -> AppResult<Vec<TagInfo>> {
    git_actions::git_tags(git_context(&app, &state, &store), project_id).await
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
    git_actions::git_tag_create(
        &TauriEventSink(&app),
        git_context(&app, &state, &store),
        project_id,
        name,
        target,
        opts,
    )
    .await
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
    git_actions::git_tag_delete(&TauriEventSink(&app), git_context(&app, &state, &store), project_id, name).await
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
    git_actions::git_checkout_remote_branch(&TauriEventSink(&app), git_context(&app, &state, &store), project_id, remote_ref).await
}

#[cfg(test)]
mod tests {
    const INVALIDATION_EVENT_COUNT: usize = 3;

    #[test]
    fn 상태_캐시_무효화_구독은_조회_전에_같은_세_이벤트를_등록한다() {
        let source = include_str!("commands.rs").split("#[cfg(test)]").next().expect("실제 명령 구현");
        let registration = source
            .split("fn ensure_status_invalidation_listeners(")
            .nth(1)
            .expect("구독 adapter")
            .split("#[tauri::command]")
            .next()
            .expect("구독 범위");
        assert!(registration.contains("store.ensure_invalidation_listeners(||"));
        assert_eq!(
            registration.matches("listen_status_invalidation::<").count(),
            INVALIDATION_EVENT_COUNT
        );
        assert!(registration.contains("listen_status_invalidation::<FsChanged>"));
        assert!(registration.contains("listen_status_invalidation::<GitStatusChanged>"));
        assert!(registration.contains("listen_status_invalidation::<GitRefsChanged>"));

        let status = source.split("pub async fn git_status(").nth(1).expect("상태 명령");
        let subscription = status
            .find("ensure_status_invalidation_listeners(&app, &store)")
            .expect("최초 구독");
        let delegation = status.find("git_actions::git_status(").expect("공유 action 위임");
        assert!(delegation < subscription);
        let runtime = include_str!("../../../../crates/taide-runtime/src/git_actions.rs");
        let action = runtime.split("pub async fn git_status(").nth(1).expect("공유 status action");
        let subscription = action.find("install_invalidation_listeners();").expect("주입 구독 port");
        let resolution = action.find("resolve_repo_root(state, store, &project_id)").expect("루트 해석");
        let read = action.find("store.read_status(&project_id, Instant::now())").expect("캐시 읽기");
        assert!(subscription < resolution && resolution < read);
    }

    #[test]
    fn git_status_의_바인딩_표면은_캐시_도입_뒤에도_그대로다() {
        let bindings = include_str!("../../../../src/shared/api/bindings.ts");
        let line = bindings
            .lines()
            .find(|line| line.trim_start().starts_with("gitStatus:"))
            .expect("bindings.ts 에 gitStatus 항목이 있어야 한다");

        assert!(
            line.contains("(projectId: ProjectId)"),
            "인자는 projectId 하나뿐이어야 한다: {line}"
        );
        assert!(
            line.contains("typedError<GitStatus, AppError>"),
            "반환 타입이 바뀌면 안 된다: {line}"
        );
        assert!(
            line.contains(r#"__TAURI_INVOKE("git_status", { projectId })"#),
            "invoke 페이로드에 새 키가 붙으면 안 된다: {line}"
        );
    }
}

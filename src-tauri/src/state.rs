use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::time::Duration;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::domain::layout::types::ProjectLayout;
use crate::domain::project::types::{Project, SessionState};
use crate::domain::settings::types::Settings;
use crate::ids::ProjectId;
use crate::infra::root_guard;
use crate::infra::self_write::SelfWriteTracker;
use crate::infra::watcher::WatcherHandle;
use crate::paths::AppPaths;

/// Which dirty editor models one flush handshake asks the frontend to write to the hot-exit mirror
/// before the backend does something that would otherwise lose them.
///
/// The handshake itself (`crate::events::HotExitFlushRequested` out,
/// `domain::file::commands::file_flush_complete` back, with a timeout fallback) started life as an
/// app-exit-only mechanism. Three separate teardowns destroy a webview or drop a project's state
/// while the mirror debounce (`HOT_EXIT_MIRROR_DEBOUNCE_MS`) may still be holding the last half
/// second of typing, and only one of them used to run it — so the scope travels with the request
/// and with every confirmation, and one window confirming a project close can never be mistaken
/// for the same window confirming the app's exit.
///
/// Serialized externally-tagged, which is what gives the frontend the union it reads:
/// `"all" | { window: label } | { project: projectId }`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FlushScope {
    /// Every window, every project — the app is exiting.
    All,
    /// One window's own models, because that OS window is about to be destroyed. Keyed by the
    /// Tauri label (`editor-<n>`) rather than by project so two auxiliary windows closing at the
    /// same time each wait for their own confirmation.
    Window(String),
    /// Every window's models belonging to one project, because that project is being closed and
    /// its `file_mirror_dirty` writes stop being accepted the moment it leaves `AppState`.
    Project(ProjectId),
}

/// Tracks one scope's close-intercept handshake between the event that started it and every
/// window that must individually confirm its flush. Absent (no map entry) -> `Pending(expected
/// window labels)` on the first close attempt (which is what guards re-entrant close requests —
/// mashing Cmd+Q, or clicking an auxiliary window's ✕ twice — from re-emitting the flush event),
/// `Pending` -> `Ready` once every expected window has confirmed
/// (`AppState::complete_flush`) or the timeout fallback force-completes it (which is what
/// guards a double `AppHandle::exit`). Wave I: before multi-window support, `Pending` carried no
/// payload because the main window was the only window that could ever confirm — see
/// `docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md` §3.1.
#[derive(Debug, Clone, PartialEq, Eq)]
enum FlushPhase {
    Pending(HashSet<String>),
    Ready,
}

struct FlushHandshake {
    /// Distinguishes this handshake from the next one over the same scope — a `Window` label and
    /// a `Project` id are both reusable (`editor-1` is reissued to the next auxiliary window, a
    /// project can be closed, reopened and closed again), so a timeout task that fires late must
    /// not force-complete whatever handshake happens to occupy its scope by then.
    token: u64,
    phase: FlushPhase,
    /// Woken when this handshake reaches `Ready`, for the one caller that awaits its own
    /// handshake inline ([`FlushTicket::wait`]) rather than reacting from a spawned task.
    completed: Option<tokio::sync::oneshot::Sender<()>>,
}

/// What [`AppState::begin_flush`] hands the caller that owns a handshake: the token its timeout
/// fallback must present, and a one-shot completion signal.
pub struct FlushTicket {
    pub token: u64,
    completed: tokio::sync::oneshot::Receiver<()>,
}

impl FlushTicket {
    /// Waits for every expected window to confirm, giving up after `timeout`. `false` means the
    /// caller must proceed anyway — a window that never answered (its webview crashed, its
    /// listener threw) can never be allowed to hold a project close, or an app exit, open forever.
    ///
    /// Dropping the ticket instead is the right shape for callers that cannot await — a
    /// `WindowEvent` handler — which react from a spawned timeout task and from
    /// `file_flush_complete` instead.
    pub async fn wait(self, timeout: Duration) -> bool {
        tokio::time::timeout(timeout, self.completed)
            .await
            .is_ok_and(|received| received.is_ok())
    }
}

fn mark_ready(handshake: &mut FlushHandshake) {
    handshake.phase = FlushPhase::Ready;
    if let Some(sender) = handshake.completed.take() {
        let _ = sender.send(());
    }
}

pub struct AppState {
    pub paths: AppPaths,
    pub session: RwLock<SessionState>,
    pub projects: RwLock<HashMap<ProjectId, Project>>,
    pub layouts: RwLock<HashMap<ProjectId, ProjectLayout>>,
    pub settings: RwLock<Settings>,
    pub dirty_layouts: RwLock<HashSet<ProjectId>>,
    pub watchers: RwLock<HashMap<ProjectId, WatcherHandle>>,
    pub git_watchers: RwLock<HashMap<ProjectId, WatcherHandle>>,
    pub self_writes: SelfWriteTracker,
    pub cli_opened_paths: RwLock<HashSet<PathBuf>>,
    mutation_guard: tokio::sync::Mutex<()>,
    flush_handshakes: parking_lot::Mutex<HashMap<FlushScope, FlushHandshake>>,
    next_flush_token: AtomicU64,
    shutting_down: std::sync::atomic::AtomicBool,
}

impl AppState {
    pub fn new(paths: AppPaths) -> Self {
        Self {
            paths,
            session: RwLock::new(SessionState::default()),
            projects: RwLock::new(HashMap::new()),
            layouts: RwLock::new(HashMap::new()),
            settings: RwLock::new(Settings::default()),
            dirty_layouts: RwLock::new(HashSet::new()),
            watchers: RwLock::new(HashMap::new()),
            git_watchers: RwLock::new(HashMap::new()),
            self_writes: SelfWriteTracker::new(),
            cli_opened_paths: RwLock::new(HashSet::new()),
            mutation_guard: tokio::sync::Mutex::new(()),
            flush_handshakes: parking_lot::Mutex::new(HashMap::new()),
            next_flush_token: AtomicU64::new(0),
            shutting_down: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub async fn begin_mutation(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.mutation_guard.lock().await
    }

    /// Records a path the user handed to TAIDE through the `taide` CLI — a cold-start argv or a
    /// `tauri-plugin-single-instance` relay, both funnelled through
    /// `domain::agent::commands::queue_external_open` — so
    /// `root_guard::resolve_owning_project_or_cli_opened` lets that one file through the
    /// open-project boundary for the rest of this process. Claude Code's Ctrl+G hands `$EDITOR` a
    /// temp file under the OS tmpdir, which no open project's root ever contains; without this
    /// entry the file could neither be opened as a tab nor read nor saved back. Only those two
    /// OS-level entry points feed the set — no IPC command can add to it, so neither the webview
    /// nor a remote session can widen the boundary on its own. Stored canonicalized
    /// (`root_guard::canonicalize_lenient`) so a later lookup of the same file under another
    /// spelling (`/var/…` vs `/private/var/…` on macOS) still matches; a path that cannot be
    /// canonicalized even leniently is not recorded.
    pub fn authorize_cli_opened_path(&self, path: &Path) {
        if let Ok(resolved) = root_guard::canonicalize_lenient(path) {
            self.cli_opened_paths.write().insert(resolved);
        }
    }

    /// Sync counterpart to [`Self::begin_mutation`] for callers already running on a blocking
    /// thread (`tokio::task::spawn_blocking`) that cannot `.await`. Panics if called from within
    /// an async execution context — see `tokio::sync::Mutex::blocking_lock`'s own panic contract.
    ///
    /// Reserved for **short, bounded lock waits** (the T0#17 `search_replace` per-file shape): the
    /// caller occupies a blocking-pool thread for its entire wait, while guard **holders**
    /// themselves dispatch work onto the same pool — as of contract 2026-08-25 §1-a/§1-d that's no
    /// longer a handful of commands but effectively every guard-holding git mutation in
    /// `domain::git::commands` (stage/unstage/discard, commit, push/pull, branch
    /// create/checkout/delete, stash push/apply/drop, hunk/line stage/discard, undo-last-commit,
    /// resolve-conflict, revert-commit, tag create/delete, checkout-remote-branch — ~20 commands)
    /// plus `domain::terminal::commands::pty_spawn` and, since the d-50 S2 batch,
    /// `domain::file::commands::file_save`/`file_copy` — so enough long-parked waiters exhaust the
    /// pool and deadlock against a holder waiting for a free thread (Phase E GIT-1). A command
    /// whose lock wait can be long must acquire the guard with `begin_mutation().await` on the
    /// async side first and only then enter `spawn_blocking` with the guard held.
    ///
    /// This method's own **waiter** side stayed narrow through that same batch — its sole caller
    /// remains `domain::search::commands::search_replace` (no re-entry was introduced by the
    /// holder-side growth above) — but the safety margin that keeps that one waiter from
    /// deadlocking against the ~20 holders now leans on a fact outside this file: `search_replace`
    /// is remote-exposed, and [`crate::domain::remote::types::REMOTE_DISPATCH_MAX_CONCURRENT`]
    /// caps how many of it a remote client can have in flight at once at 128 — well under the
    /// blocking pool's default 512 threads. See that constant's doc for the other half of this
    /// margin; the two docs must be re-checked together if either side changes.
    ///
    /// That margin is a thread count, so it is also spent by blocking work that never touches this
    /// guard at all, and the same batch added several such occupants of the one pool: every `file`
    /// read/write command (`file_open`/`file_save`/`file_copy`/`file_mirror_dirty`), the seven git
    /// queries M-1 moved off the async workers, the tree prefetch — and, unlike all of those,
    /// `domain::terminal::commands::pty_write`, whose blocking write has **no upper bound**: it
    /// parks on the child's stdin pipe for as long as the child refuses to read, holding both a pool
    /// thread and the session's writer mutex, and every further write to that session parks behind
    /// it on its own pool thread. `file_save`/`file_copy` are the ones that make a shortage
    /// self-sustaining rather than merely slow, since they wait for a free pool thread *while
    /// holding this guard*. Nothing here is close to 512 in practice (a stuck terminal contributes
    /// one thread per queued write, and the frontend now serializes those per session —
    /// `entities/terminal/session-write-order.ts`), but the margin is no longer "one waiter vs. the
    /// pool" and must be re-checked whenever an unbounded blocking call is added.
    pub fn begin_mutation_blocking(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.mutation_guard.blocking_lock()
    }

    /// Marks the app as shutting down — set once from `domain::window::commands::handle_close_requested`
    /// (the first moment the main window's close is known to be underway, before the hot-exit flush
    /// handshake even starts) and again, as a backstop for exit paths that skip that handler
    /// entirely (Cmd+Q's `NSApplication terminate:` on macOS bypasses `CloseRequested`, going
    /// straight to `RunEvent::Exit` — see `handle_menu_event`'s doc), from the `RunEvent::
    /// ExitRequested`/`Exit` arm. `Relaxed` ordering is enough: every reader
    /// ([`Self::is_shutting_down`]) only ever needs "has this been set at all", never a specific
    /// happens-before relationship with some other write.
    pub fn begin_shutdown(&self) {
        self.shutting_down.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Polled once per iteration by `domain::project::commands::restore_project_watchers`'s boot-restore loop so a
    /// close/quit requested partway through a multi-project restore stops attaching the rest
    /// immediately instead of running every remaining project's `FileIdMap` walk (and briefly
    /// taking `begin_mutation` per project) while the app is already on its way out.
    pub fn is_shutting_down(&self) -> bool {
        self.shutting_down.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Starts a flush handshake for `scope`, recording which window labels must each individually
    /// confirm (via [`Self::complete_flush`]) before whatever deferred teardown asked for it may
    /// proceed. Returns `None` when that scope already has a handshake — the caller is a
    /// re-entrant close attempt (mashing Cmd+Q, clicking an auxiliary window's ✕ twice) and must
    /// keep blocking without re-emitting the flush event.
    ///
    /// The expected set is whatever the caller can see at this instant: every open window for
    /// [`FlushScope::All`] and [`FlushScope::Project`], the one closing window for
    /// [`FlushScope::Window`]. A window that opens *after* this point was never asked to flush
    /// anything this handshake needs, and its confirmation is ignored.
    pub fn begin_flush(&self, scope: FlushScope, expected_windows: HashSet<String>) -> Option<FlushTicket> {
        let mut handshakes = self.flush_handshakes.lock();
        if handshakes.contains_key(&scope) {
            return None;
        }

        let token = self.next_flush_token.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (sender, receiver) = tokio::sync::oneshot::channel();
        handshakes.insert(
            scope,
            FlushHandshake {
                token,
                phase: FlushPhase::Pending(expected_windows),
                completed: Some(sender),
            },
        );
        Some(FlushTicket {
            token,
            completed: receiver,
        })
    }

    /// Records `window_label`'s flush confirmation for `scope`. Returns `true` only once every
    /// window that scope expected has confirmed (or was dropped mid-flush via
    /// [`Self::forget_hot_exit_flush_window`]), so exactly one caller proceeds to the teardown the
    /// handshake was deferring. A confirmation from a window outside the expected set (a window
    /// opened after the flush started, or one answering a different scope's request) is a no-op.
    pub fn complete_flush(&self, scope: &FlushScope, window_label: &str) -> bool {
        let mut handshakes = self.flush_handshakes.lock();
        let Some(handshake) = handshakes.get_mut(scope) else {
            return false;
        };
        let FlushPhase::Pending(pending) = &mut handshake.phase else {
            return false;
        };
        pending.remove(window_label);
        if !pending.is_empty() {
            return false;
        }
        mark_ready(handshake);
        true
    }

    /// Force-completes `scope`'s handshake regardless of which windows have confirmed so far —
    /// the timeout fallback. Returns `true` only for whichever of {a window's own confirmation
    /// reaching zero pending, this fallback} observes the handshake as still pending, so exactly
    /// one of them proceeds to the deferred teardown.
    ///
    /// `token` is what keeps a fallback that fires late from force-completing a *different*
    /// handshake that has since taken the same scope over: the same auxiliary window label, or the
    /// same project closed a second time after being reopened.
    pub fn force_complete_flush(&self, scope: &FlushScope, token: u64) -> bool {
        let mut handshakes = self.flush_handshakes.lock();
        let Some(handshake) = handshakes.get_mut(scope) else {
            return false;
        };
        if handshake.token != token || matches!(handshake.phase, FlushPhase::Ready) {
            return false;
        }
        mark_ready(handshake);
        true
    }

    /// Drops `scope`'s handshake entirely, whatever phase it is in, so the scope can host a fresh
    /// one. Every scope but [`FlushScope::All`] names something that comes back — an auxiliary
    /// window label is reissued, a project can be reopened — and a leftover `Ready` entry would
    /// make the next [`Self::begin_flush`] on it return `None` and skip the flush.
    pub fn clear_flush(&self, scope: &FlushScope) {
        self.flush_handshakes.lock().remove(scope);
    }

    /// Consumes `scope`'s handshake if it has already reached `Ready`, reporting whether it had.
    ///
    /// This is how an auxiliary window's second `CloseRequested` — the one this module itself
    /// triggers with `close()` after the flush confirmed — tells itself apart from the user's
    /// first click, which must be intercepted instead.
    pub fn take_completed_flush(&self, scope: &FlushScope) -> bool {
        let mut handshakes = self.flush_handshakes.lock();
        if !matches!(handshakes.get(scope), Some(handshake) if matches!(handshake.phase, FlushPhase::Ready)) {
            return false;
        }
        handshakes.remove(scope);
        true
    }

    /// [`FlushScope::All`] shorthand — the main window's `CloseRequested` starting the app-exit
    /// handshake. `true` only for the first `CloseRequested` after boot; later re-entrant attempts
    /// return `false` so the caller skips re-emitting the flush event while still blocking the
    /// close.
    pub fn begin_hot_exit_flush(&self, expected_windows: HashSet<String>) -> bool {
        self.begin_flush(FlushScope::All, expected_windows).is_some()
    }

    /// [`FlushScope::All`] shorthand — one window confirming it flushed for the app exit. `true`
    /// for the single caller that must now actually exit the app.
    pub fn complete_hot_exit_flush(&self, window_label: &str) -> bool {
        self.complete_flush(&FlushScope::All, window_label)
    }

    /// The window is gone. Drops `window_label` from the app-exit handshake's still-pending set
    /// without counting as a confirmation — used when a window closes (or is destroyed)
    /// independently while an app-exit flush it was never going to answer is already in flight, so
    /// its absence can't stall the exit until the timeout fallback. Mirrors
    /// [`Self::complete_hot_exit_flush`]'s "last one out transitions to `Ready`" behavior:
    /// dropping the *last* still-pending window must also unblock the exit — otherwise a flush
    /// every other window already confirmed sits stuck until [`Self::force_complete_hot_exit_flush`]'s
    /// timeout fires, even though nothing is actually still pending.
    ///
    /// It also drops that window's own [`FlushScope::Window`] handshake, for the same reason and
    /// with the same force: a destroyed webview answers nothing, and its label will be reissued to
    /// the next auxiliary window.
    pub fn forget_hot_exit_flush_window(&self, window_label: &str) -> bool {
        self.clear_flush(&FlushScope::Window(window_label.to_string()));
        self.complete_flush(&FlushScope::All, window_label)
    }

    /// Force-completes the app exit regardless of which windows have confirmed so far. Returns
    /// `true` only for whichever of {a window's own confirmation reaching zero pending, this
    /// timeout fallback} observes the flush as still pending, so exactly one of them proceeds to
    /// actually exit the app. Unlike [`Self::force_complete_flush`] it carries no token: the app
    /// exits exactly once, so [`FlushScope::All`] never hosts a second handshake.
    pub fn force_complete_hot_exit_flush(&self) -> bool {
        let mut handshakes = self.flush_handshakes.lock();
        match handshakes.get_mut(&FlushScope::All) {
            Some(handshake) if matches!(handshake.phase, FlushPhase::Ready) => false,
            Some(handshake) => {
                mark_ready(handshake);
                true
            }
            None => {
                let token = self.next_flush_token.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                handshakes.insert(
                    FlushScope::All,
                    FlushHandshake {
                        token,
                        phase: FlushPhase::Ready,
                        completed: None,
                    },
                );
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(values: &[&str]) -> HashSet<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn cli_로_전달된_경로는_정규화되어_허용_목록에_기록된다() {
        let state = AppState::new(AppPaths::new(std::path::PathBuf::from("/tmp")));
        let dir = std::env::temp_dir().join(format!("taide-cli-opened-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("claude-prompt.md");
        std::fs::write(&file, b"prompt").unwrap();

        state.authorize_cli_opened_path(&file);

        let canonical = std::fs::canonicalize(&file).unwrap();
        assert!(state.cli_opened_paths.read().contains(&canonical));
        assert_eq!(state.cli_opened_paths.read().len(), 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 아직_없는_파일도_부모가_존재하면_허용_목록에_기록된다() {
        let state = AppState::new(AppPaths::new(std::path::PathBuf::from("/tmp")));
        let dir = std::env::temp_dir().join(format!("taide-cli-opened-new-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("not-yet-written.md");

        state.authorize_cli_opened_path(&file);

        let expected = std::fs::canonicalize(&dir).unwrap().join("not-yet-written.md");
        assert!(state.cli_opened_paths.read().contains(&expected));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 재진입_close_요청은_다시_시작하지_않는다() {
        let state = AppState::new(AppPaths::new(std::path::PathBuf::from("/tmp")));

        assert!(state.begin_hot_exit_flush(labels(&["main"])));
        assert!(
            !state.begin_hot_exit_flush(labels(&["main"])),
            "이미 pending 이면 재시작하지 않는다"
        );
    }

    #[test]
    fn 단일_창은_기존과_동일하게_한_번의_확인으로_완료된다() {
        let state = AppState::new(AppPaths::new(std::path::PathBuf::from("/tmp")));

        state.begin_hot_exit_flush(labels(&["main"]));
        assert!(state.complete_hot_exit_flush("main"));
    }

    #[test]
    fn 창이_여러개면_전부_확인해야_완료된다() {
        let state = AppState::new(AppPaths::new(std::path::PathBuf::from("/tmp")));

        state.begin_hot_exit_flush(labels(&["main", "editor-1"]));
        assert!(!state.complete_hot_exit_flush("main"), "editor-1 이 아직 남아있다");
        assert!(state.complete_hot_exit_flush("editor-1"), "마지막 창까지 확인되면 완료된다");
    }

    #[test]
    fn 완료후_재확인은_다시_true를_반환하지_않는다() {
        let state = AppState::new(AppPaths::new(std::path::PathBuf::from("/tmp")));

        state.begin_hot_exit_flush(labels(&["main"]));
        assert!(state.complete_hot_exit_flush("main"));
        assert!(!state.complete_hot_exit_flush("main"), "이미 Ready 다");
    }

    #[test]
    fn 기대하지_않은_창의_확인은_무시된다() {
        let state = AppState::new(AppPaths::new(std::path::PathBuf::from("/tmp")));

        state.begin_hot_exit_flush(labels(&["main"]));
        assert!(!state.complete_hot_exit_flush("editor-9"), "기대 목록에 없는 창이다");
        assert!(state.complete_hot_exit_flush("main"), "실제 기대 목록은 여전히 유효하다");
    }

    #[test]
    fn 창을_잊으면_남은_창만으로_완료된다() {
        let state = AppState::new(AppPaths::new(std::path::PathBuf::from("/tmp")));

        state.begin_hot_exit_flush(labels(&["main", "editor-1"]));
        assert!(!state.forget_hot_exit_flush_window("editor-1"), "main 이 아직 남아 있다");
        assert!(state.complete_hot_exit_flush("main"), "editor-1 은 이미 잊혀졌다");
    }

    #[test]
    fn 마지막으로_남은_창을_잊으면_그_자체로_완료된다() {
        let state = AppState::new(AppPaths::new(std::path::PathBuf::from("/tmp")));

        state.begin_hot_exit_flush(labels(&["main", "editor-1"]));
        assert!(!state.forget_hot_exit_flush_window("main"), "editor-1 이 아직 남아 있다");
        assert!(
            state.forget_hot_exit_flush_window("editor-1"),
            "마지막 대기 창을 잊는 순간 true 로 완료를 알려야 한다"
        );
        assert!(!state.complete_hot_exit_flush("main"), "이미 forget 으로 완료되었다");
    }

    #[test]
    fn 타임아웃_강제완료는_대기중이던_창수와_무관하게_한번만_성공한다() {
        let state = AppState::new(AppPaths::new(std::path::PathBuf::from("/tmp")));

        state.begin_hot_exit_flush(labels(&["main", "editor-1"]));
        assert!(state.force_complete_hot_exit_flush());
        assert!(!state.force_complete_hot_exit_flush(), "이미 Ready 다");
        assert!(!state.complete_hot_exit_flush("main"), "타임아웃으로 이미 완료되었다");
    }

    fn state() -> AppState {
        AppState::new(AppPaths::new(std::path::PathBuf::from("/tmp")))
    }

    fn window_scope(label: &str) -> FlushScope {
        FlushScope::Window(label.to_string())
    }

    fn project_scope(id: &str) -> FlushScope {
        FlushScope::Project(ProjectId::from(id.to_string()))
    }

    #[test]
    fn 스코프가_다르면_핸드셰이크는_서로_간섭하지_않는다() {
        let state = state();
        let aux = window_scope("editor-1");
        let project = project_scope("prj-a");

        assert!(state.begin_hot_exit_flush(labels(&["main", "editor-1"])));
        assert!(state.begin_flush(aux.clone(), labels(&["editor-1"])).is_some());
        assert!(state.begin_flush(project.clone(), labels(&["main", "editor-1"])).is_some());

        assert!(state.complete_flush(&aux, "editor-1"), "창 스코프는 그 창만으로 완료된다");
        assert!(
            !state.complete_hot_exit_flush("editor-1"),
            "창 스코프 확인이 앱 종료 핸드셰이크를 앞당기면 안 된다"
        );
        assert!(!state.complete_flush(&project, "editor-1"), "프로젝트는 main 이 남아 있다");
        assert!(state.complete_flush(&project, "main"));
        assert!(state.complete_hot_exit_flush("main"));
    }

    #[test]
    fn 같은_스코프의_재진입_요청은_새_핸드셰이크를_만들지_않는다() {
        let state = state();
        let aux = window_scope("editor-1");

        assert!(state.begin_flush(aux.clone(), labels(&["editor-1"])).is_some());
        assert!(
            state.begin_flush(aux, labels(&["editor-1"])).is_none(),
            "두 번째 닫기 클릭은 flush 요청을 다시 보내면 안 된다"
        );
    }

    #[test]
    fn 완료된_창_핸드셰이크는_한_번만_회수된다() {
        let state = state();
        let aux = window_scope("editor-1");

        state.begin_flush(aux.clone(), labels(&["editor-1"]));
        assert!(!state.take_completed_flush(&aux), "아직 pending 이면 회수되지 않는다");
        assert!(state.complete_flush(&aux, "editor-1"));
        assert!(state.take_completed_flush(&aux), "확인된 핸드셰이크는 후속 닫기에서 소비된다");
        assert!(!state.take_completed_flush(&aux), "소비된 뒤에는 남아 있지 않다");
    }

    /// The whole reason a token exists: `editor-1` is reissued to the next auxiliary window, so a
    /// timeout task from the *previous* window must not force-complete the new one's handshake and
    /// close a window the user never asked to close.
    #[test]
    fn 이전_핸드셰이크의_타임아웃은_같은_스코프의_새_핸드셰이크를_강제완료하지_않는다() {
        let state = state();
        let aux = window_scope("editor-1");

        let first = state.begin_flush(aux.clone(), labels(&["editor-1"])).expect("첫 핸드셰이크");
        state.clear_flush(&aux);
        let second = state.begin_flush(aux.clone(), labels(&["editor-1"])).expect("두 번째 핸드셰이크");

        assert_ne!(first.token, second.token);
        assert!(!state.force_complete_flush(&aux, first.token), "낡은 토큰은 거절된다");
        assert!(state.force_complete_flush(&aux, second.token));
    }

    #[test]
    fn 창_스코프_타임아웃_강제완료는_한_번만_성공한다() {
        let state = state();
        let aux = window_scope("editor-1");

        let ticket = state.begin_flush(aux.clone(), labels(&["editor-1"])).expect("핸드셰이크");
        assert!(state.force_complete_flush(&aux, ticket.token));
        assert!(!state.force_complete_flush(&aux, ticket.token), "이미 Ready 다");
        assert!(!state.complete_flush(&aux, "editor-1"), "타임아웃으로 이미 완료되었다");
    }

    #[test]
    fn 없는_스코프의_확인과_강제완료는_무시된다() {
        let state = state();
        let aux = window_scope("editor-9");

        assert!(!state.complete_flush(&aux, "editor-9"));
        assert!(!state.force_complete_flush(&aux, 0));
    }

    /// Wave 2 #12: a project can be closed, reopened and closed again, so a leftover `Ready` entry
    /// must never make the second close skip its flush.
    #[test]
    fn 프로젝트_핸드셰이크를_정리하면_같은_프로젝트를_다시_닫을_수_있다() {
        let state = state();
        let project = project_scope("prj-a");

        state.begin_flush(project.clone(), labels(&["main"]));
        assert!(state.complete_flush(&project, "main"));
        state.clear_flush(&project);

        assert!(
            state.begin_flush(project, labels(&["main"])).is_some(),
            "정리된 스코프는 새 핸드셰이크를 받을 수 있어야 한다"
        );
    }

    /// A destroyed webview answers nothing — its own window-scoped handshake has to go with it, or
    /// the label it is about to hand back to the next auxiliary window starts out already taken.
    #[test]
    fn 창을_잊으면_그_창의_창스코프_핸드셰이크도_사라진다() {
        let state = state();
        let aux = window_scope("editor-1");

        state.begin_flush(aux.clone(), labels(&["editor-1"]));
        state.forget_hot_exit_flush_window("editor-1");

        assert!(!state.complete_flush(&aux, "editor-1"), "핸드셰이크가 남아 있으면 안 된다");
        assert!(state.begin_flush(aux, labels(&["editor-1"])).is_some());
    }

    #[tokio::test]
    async fn 대기_티켓은_모든_창이_확인하면_깨어난다() {
        let state = std::sync::Arc::new(state());
        let project = project_scope("prj-a");
        let ticket = state
            .begin_flush(project.clone(), labels(&["main", "editor-1"]))
            .expect("핸드셰이크");

        let confirming = state.clone();
        let confirming_scope = project.clone();
        tokio::spawn(async move {
            confirming.complete_flush(&confirming_scope, "main");
            confirming.complete_flush(&confirming_scope, "editor-1");
        });

        assert!(
            ticket.wait(std::time::Duration::from_secs(5)).await,
            "모든 창이 확인하면 타임아웃 전에 깨어나야 한다"
        );
    }

    #[tokio::test]
    async fn 대기_티켓은_확인이_없으면_타임아웃하고_진행을_막지_않는다() {
        let state = state();
        let ticket = state.begin_flush(project_scope("prj-a"), labels(&["main"])).expect("핸드셰이크");

        assert!(
            !ticket.wait(std::time::Duration::from_millis(20)).await,
            "확인이 오지 않으면 false 로 돌아와 호출부가 그대로 진행해야 한다"
        );
    }

    #[tokio::test]
    async fn 대기_티켓은_강제완료로도_깨어난다() {
        let state = std::sync::Arc::new(state());
        let project = project_scope("prj-a");
        let ticket = state.begin_flush(project.clone(), labels(&["main"])).expect("핸드셰이크");
        let token = ticket.token;

        let forcing = state.clone();
        let forcing_scope = project.clone();
        tokio::spawn(async move {
            forcing.force_complete_flush(&forcing_scope, token);
        });

        assert!(ticket.wait(std::time::Duration::from_secs(5)).await);
    }

    #[test]
    fn 종료_플래그는_기본값이_false이고_begin_shutdown_이후_true다() {
        let state = AppState::new(AppPaths::new(std::path::PathBuf::from("/tmp")));

        assert!(!state.is_shutting_down());
        state.begin_shutdown();
        assert!(state.is_shutting_down());
        state.begin_shutdown();
        assert!(state.is_shutting_down(), "재호출해도 여전히 true 여야 한다");
    }

    #[tokio::test]
    async fn begin_mutation_blocking_은_비동기_begin_mutation_과_동일한_락을_공유한다() {
        let state = std::sync::Arc::new(AppState::new(AppPaths::new(std::path::PathBuf::from("/tmp"))));

        let async_guard = state.begin_mutation().await;

        let blocking_state = state.clone();
        let blocking_task = tokio::task::spawn_blocking(move || {
            let _guard = blocking_state.begin_mutation_blocking();
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(
            !blocking_task.is_finished(),
            "async 쪽이 begin_mutation 의 guard 를 쥐고 있는 동안 begin_mutation_blocking 이 통과하면 안 된다 — 서로 다른 락이면 spawn_blocking 안의 파일별 replace 가 file_save 등 다른 뮤테이션과 직렬화되지 않는다"
        );

        drop(async_guard);
        blocking_task.await.expect("blocking task panicked");
    }
}

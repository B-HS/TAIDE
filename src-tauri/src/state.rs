use std::collections::{HashMap, HashSet};

use parking_lot::RwLock;

use crate::domain::layout::types::ProjectLayout;
use crate::domain::project::types::{Project, SessionState};
use crate::domain::settings::types::Settings;
use crate::ids::ProjectId;
use crate::infra::self_write::SelfWriteTracker;
use crate::infra::watcher::WatcherHandle;
use crate::paths::AppPaths;

/// Tracks the hot-exit close-intercept handshake between the main window's
/// `CloseRequested` event and every currently-open window's individual flush
/// completion. `Idle` -> `Pending(expected window labels)` on the first close
/// attempt (guards re-entrant close requests — e.g. mashing Cmd+Q — from
/// re-emitting the flush event), `Pending` -> `Ready` once every expected
/// window has confirmed (`AppState::complete_hot_exit_flush`) or the timeout
/// fallback force-completes it (guards a double `AppHandle::exit`). Wave I:
/// before multi-window support, `Pending` carried no payload because the main
/// window was the only window that could ever confirm — see
/// `docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md` §3.1.
#[derive(Debug, Clone, PartialEq, Eq)]
enum HotExitFlushPhase {
    Idle,
    Pending(HashSet<String>),
    Ready,
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
    mutation_guard: tokio::sync::Mutex<()>,
    hot_exit_flush: parking_lot::Mutex<HotExitFlushPhase>,
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
            mutation_guard: tokio::sync::Mutex::new(()),
            hot_exit_flush: parking_lot::Mutex::new(HotExitFlushPhase::Idle),
        }
    }

    pub async fn begin_mutation(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.mutation_guard.lock().await
    }

    /// Sync counterpart to [`Self::begin_mutation`] for callers already running on a blocking
    /// thread (`tokio::task::spawn_blocking`) that cannot `.await`. Panics if called from within
    /// an async execution context — see `tokio::sync::Mutex::blocking_lock`'s own panic contract.
    ///
    /// Reserved for **short, bounded lock waits** (the T0#17 `search_replace` per-file shape): the
    /// caller occupies a blocking-pool thread for its entire wait, while guard **holders**
    /// themselves dispatch work onto the same pool (`git_pull`, `git_revert_commit`, hunk
    /// staging, ...) — so enough long-parked waiters exhaust the pool and deadlock against a
    /// holder waiting for a free thread (Phase E GIT-1). A command whose lock wait can be long
    /// must acquire the guard with `begin_mutation().await` on the async side first and only then
    /// enter `spawn_blocking` with the guard held.
    pub fn begin_mutation_blocking(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.mutation_guard.blocking_lock()
    }

    /// Starts the hot-exit close-intercept handshake, recording which window
    /// labels must each individually confirm their flush (via
    /// `complete_hot_exit_flush`) before the app actually exits — main plus
    /// whichever `editor-*` auxiliary windows happen to be open at the
    /// moment the main window's close was requested. Returns `true` only for
    /// the first `CloseRequested` after boot; later re-entrant close
    /// attempts return `false` so the caller can skip re-emitting the
    /// flush-requested event while still blocking the close.
    pub fn begin_hot_exit_flush(&self, expected_windows: HashSet<String>) -> bool {
        let mut phase = self.hot_exit_flush.lock();
        if !matches!(*phase, HotExitFlushPhase::Idle) {
            return false;
        }
        *phase = HotExitFlushPhase::Pending(expected_windows);
        true
    }

    /// Records `window_label`'s flush confirmation. Returns `true` only once
    /// every window expected by `begin_hot_exit_flush` has confirmed (or was
    /// dropped mid-flush via `forget_hot_exit_flush_window`), so exactly one
    /// caller proceeds to actually exit the app. A confirmation from a
    /// window outside the expected set (e.g. a fresh auxiliary window opened
    /// after the flush already started) is a no-op — that window was never
    /// asked to flush anything hot-exit needs.
    pub fn complete_hot_exit_flush(&self, window_label: &str) -> bool {
        let mut phase = self.hot_exit_flush.lock();
        let HotExitFlushPhase::Pending(pending) = &mut *phase else {
            return false;
        };
        pending.remove(window_label);
        if !pending.is_empty() {
            return false;
        }
        *phase = HotExitFlushPhase::Ready;
        true
    }

    /// Drops `window_label` from the still-pending confirmation set without
    /// counting as a flush confirmation. Used when a window closes (or is
    /// destroyed) independently while a main-window hot-exit flush it was
    /// never going to answer is already in flight, so its absence can't
    /// stall the exit until the timeout fallback. Mirrors
    /// `complete_hot_exit_flush`'s "last one out transitions to `Ready`"
    /// behavior: dropping the *last* still-pending window must also unblock
    /// the exit — otherwise a flush that every other window already
    /// confirmed sits stuck until `force_complete_hot_exit_flush`'s timeout
    /// fires, even though nothing is actually still pending.
    pub fn forget_hot_exit_flush_window(&self, window_label: &str) -> bool {
        let mut phase = self.hot_exit_flush.lock();
        let HotExitFlushPhase::Pending(pending) = &mut *phase else {
            return false;
        };
        pending.remove(window_label);
        if !pending.is_empty() {
            return false;
        }
        *phase = HotExitFlushPhase::Ready;
        true
    }

    /// Force-completes the flush regardless of which windows have confirmed
    /// so far. Returns `true` only for whichever of {a window's own
    /// confirmation reaching zero pending, this timeout fallback} observes
    /// the flush as still pending, so exactly one of them proceeds to
    /// actually exit the app.
    pub fn force_complete_hot_exit_flush(&self) -> bool {
        let mut phase = self.hot_exit_flush.lock();
        if matches!(*phase, HotExitFlushPhase::Ready) {
            return false;
        }
        *phase = HotExitFlushPhase::Ready;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(values: &[&str]) -> HashSet<String> {
        values.iter().map(|value| value.to_string()).collect()
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

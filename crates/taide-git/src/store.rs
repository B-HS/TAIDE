use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use taide_model::git::GitStatus;
use taide_model::ids::ProjectId;

const STATUS_CACHE_TTL: Duration = Duration::from_secs(2);

#[derive(Debug)]
struct StatusCacheEntry {
    status: GitStatus,
    observed_at: Instant,
}

#[derive(Debug, Default)]
struct StatusSlot {
    identity: Arc<()>,
    generation: u64,
    entry: Option<StatusCacheEntry>,
}

#[derive(Debug, Clone)]
pub struct PendingStatus {
    identity: Arc<()>,
    generation: u64,
    observed_at: Instant,
}

pub enum StatusRead {
    Fresh(GitStatus),
    Stale(PendingStatus),
}

#[derive(Debug, Default)]
struct StatusCache {
    slots: Mutex<HashMap<ProjectId, StatusSlot>>,
}

impl StatusCache {
    fn read(&self, project_id: &ProjectId, now: Instant, ttl: Duration) -> StatusRead {
        let mut slots = self.slots.lock();
        let slot = slots.entry(project_id.clone()).or_default();

        if let Some(entry) = &slot.entry {
            if now.duration_since(entry.observed_at) < ttl {
                return StatusRead::Fresh(entry.status.clone());
            }
            slot.entry = None;
        }

        StatusRead::Stale(PendingStatus {
            identity: slot.identity.clone(),
            generation: slot.generation,
            observed_at: now,
        })
    }

    fn finish(&self, project_id: &ProjectId, pending: PendingStatus, status: &GitStatus) {
        let mut slots = self.slots.lock();
        let Some(slot) = slots.get_mut(project_id) else {
            return;
        };
        if slot.generation != pending.generation || !Arc::ptr_eq(&slot.identity, &pending.identity) {
            return;
        }
        slot.entry = Some(StatusCacheEntry {
            status: status.clone(),
            observed_at: pending.observed_at,
        });
    }

    fn invalidate(&self, project_id: &ProjectId) {
        let mut slots = self.slots.lock();
        let Some(slot) = slots.get_mut(project_id) else {
            return;
        };
        slot.generation = slot.generation.wrapping_add(1);
        slot.entry = None;
    }

    fn forget(&self, project_id: &ProjectId) {
        self.slots.lock().remove(project_id);
    }
}

#[derive(Default)]
struct GitStoreInner {
    repo_roots: Mutex<HashMap<ProjectId, PathBuf>>,
    push_fetch_locks: Mutex<HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>>,
    status_cache: StatusCache,
    invalidation_listeners: OnceLock<()>,
}

#[derive(Clone, Default)]
pub struct GitStore(Arc<GitStoreInner>);

impl GitStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forgets `project_id`'s cached repo root **and its cached status**, called by
    /// `GitCacheCapability::detach` during `project_close`, and by `git_init` right after
    /// re-initializing a repo at the project's root — so a project reopened at the same path (or one
    /// just re-initialized in place) resolves its repo root fresh instead of reusing a cache entry
    /// keyed by a `ProjectId` that no session will ever look up again — see `resolve_repo_root`,
    /// which otherwise happily serves that stale entry forever (the map is never pruned by size or
    /// age, only by this explicit removal). The `StatusCache` slot is dropped **first**, ahead of
    /// the `repo_roots` early return below, so its reclamation never becomes conditional on a
    /// repo-root entry still being there — `architecture.md` §6.3 requires every per-project entry
    /// this store owns to be gone when `project_close` returns, and a slot left behind would
    /// resurrect a closed project's status the moment the same `ProjectId` is seen again.
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
        self.0.status_cache.forget(project_id);
        let root = self.0.repo_roots.lock().remove(project_id);
        let Some(root) = root else { return };
        let mut locks = self.0.push_fetch_locks.lock();
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
    /// This lock's hold time is bounded only by the `run_git` subprocess it wraps: a push/fetch
    /// stalled on an unreachable remote or a blocked credential prompt holds this lock — and queues
    /// every later `git_push`/`git_fetch` for the same repo behind it — until that subprocess ends.
    /// This is not a defect this lock introduces; it is the underlying `run_git` wait now reaching
    /// same-repo callers instead of staying scoped to just the one call that triggered it. The trade
    /// is deliberate: before this lock, N concurrent stalled calls on the same repo each pinned their
    /// own blocking-pool thread; now at most one does, and the rest wait on this async mutex instead.
    ///
    /// That wait is no longer unbounded, which is the one thing this paragraph used to say it was:
    /// the d-50 S3 batch gave `run_git` a deadline and a kill path
    /// (`service::GIT_COMMAND_TIMEOUT_SECS`, plus `GIT_PIPE_DRAIN_TIMEOUT_SECS` for the post-exit
    /// pipe drain), so the worst case here is that bound per subprocess rather than the rest of the
    /// app's lifetime. It is a *per subprocess* bound, though — a command that runs several
    /// (`commit` runs three) multiplies it — and 300s is still long enough that
    /// `docs/quality-assurance/2026-08-11-qa6-checklist.md`'s d-35 same-repo-stall scenario stays
    /// worth exercising under realistic conditions.
    pub fn push_fetch_lock(&self, repo_root: &Path) -> Arc<tokio::sync::Mutex<()>> {
        self.0
            .push_fetch_locks
            .lock()
            .entry(repo_root.to_path_buf())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    /// Drops `project_id`'s cached status so the next `git_status` recomputes, and discards any
    /// computation already in flight for it (`StatusCache::finish`).
    ///
    /// Called from two places for two different reasons. The event subscriptions
    /// ([`Self::ensure_invalidation_listeners`]) are the **complete** set — every signal that makes
    /// the frontend re-ask for a status passes through one of them. The direct calls in
    /// `emit_status_changed`/`emit_refs_changed` and in `watch::build_git_watcher_handle`'s
    /// callback are the **ordering** guarantee for the paths this domain owns: `Manager::emit`
    /// hands the payload to the webviews *before* it runs Rust listeners, so invalidating ahead of
    /// the emit is what makes it impossible — not merely unlikely — for a refetch triggered by that
    /// event to be served the pre-change status.
    pub fn invalidate_status(&self, project_id: &ProjectId) {
        self.0.status_cache.invalidate(project_id);
    }

    pub fn cached_repo_root(&self, project_id: &ProjectId) -> Option<PathBuf> {
        self.0.repo_roots.lock().get(project_id).cloned()
    }

    pub fn cache_repo_root(&self, project_id: ProjectId, repo_root: PathBuf) {
        self.0.repo_roots.lock().insert(project_id, repo_root);
    }

    pub fn read_status(&self, project_id: &ProjectId, now: Instant) -> StatusRead {
        self.0.status_cache.read(project_id, now, STATUS_CACHE_TTL)
    }

    pub fn finish_status(&self, project_id: &ProjectId, pending: PendingStatus, status: &GitStatus) {
        self.0.status_cache.finish(project_id, pending, status);
    }

    pub fn ensure_invalidation_listeners(&self, register: impl FnOnce()) {
        self.0.invalidation_listeners.get_or_init(register);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_status(branch: &str) -> GitStatus {
        GitStatus {
            rows: Vec::new(),
            branch: Some(branch.to_string()),
            ahead: 0,
            behind: 0,
            has_remote: false,
        }
    }

    fn fill_status(cache: &StatusCache, project_id: &ProjectId, now: Instant, status: &GitStatus) {
        let StatusRead::Stale(pending) = cache.read(project_id, now, STATUS_CACHE_TTL) else {
            panic!("채워지지 않은 캐시는 계산을 요구해야 한다");
        };
        cache.finish(project_id, pending, status);
    }

    fn is_fresh(read: &StatusRead, branch: &str) -> bool {
        matches!(read, StatusRead::Fresh(status) if status.branch.as_deref() == Some(branch))
    }

    const BUDGET_READ_SPACING_MS: u64 = 100;

    const BUDGET_READS: u64 = 5;
    const LOCK_WAIT_TIMEOUT_MS: u64 = 50;

    #[test]
    fn 저장된_status는_ttl_안에서는_다시_계산하지_않고_그대로_돌려준다() {
        let cache = StatusCache::default();
        let project_id = ProjectId::new();
        let base = Instant::now();
        fill_status(&cache, &project_id, base, &sample_status("main"));

        let read = cache.read(&project_id, base + STATUS_CACHE_TTL - Duration::from_millis(1), STATUS_CACHE_TTL);

        assert!(is_fresh(&read, "main"), "TTL 안의 조회는 저장된 결과를 그대로 받아야 한다");
    }

    #[test]
    fn 저장된_status는_ttl_경계에서_만료된다() {
        let cache = StatusCache::default();
        let project_id = ProjectId::new();
        let base = Instant::now();
        fill_status(&cache, &project_id, base, &sample_status("main"));

        assert!(
            matches!(
                cache.read(&project_id, base + STATUS_CACHE_TTL, STATUS_CACHE_TTL),
                StatusRead::Stale(_)
            ),
            "TTL 과 정확히 같은 나이는 이미 만료로 취급해야 워처가 놓친 변경의 노출 창이 TTL 을 넘지 않는다"
        );
    }

    #[test]
    fn 계산이_ttl보다_오래_걸리면_저장되자마자_만료다() {
        let cache = StatusCache::default();
        let project_id = ProjectId::new();
        let base = Instant::now();
        let StatusRead::Stale(pending) = cache.read(&project_id, base, STATUS_CACHE_TTL) else {
            panic!("빈 캐시는 계산을 요구해야 한다");
        };
        cache.finish(&project_id, pending, &sample_status("main"));

        assert!(
            matches!(
                cache.read(&project_id, base + STATUS_CACHE_TTL, STATUS_CACHE_TTL),
                StatusRead::Stale(_)
            ),
            "나이는 계산이 끝난 시각이 아니라 저장소를 읽은 시각 기준이어야 한다"
        );
    }

    #[test]
    fn 무효화_이후에는_저장된_status를_돌려주지_않는다() {
        let cache = StatusCache::default();
        let project_id = ProjectId::new();
        let base = Instant::now();
        fill_status(&cache, &project_id, base, &sample_status("main"));

        cache.invalidate(&project_id);

        assert!(
            matches!(cache.read(&project_id, base, STATUS_CACHE_TTL), StatusRead::Stale(_)),
            "TTL 이 남아 있어도 무효화된 결과는 서빙되면 안 된다"
        );
    }

    #[test]
    fn 계산_중에_들어온_무효화는_그_계산_결과를_버린다() {
        let cache = StatusCache::default();
        let project_id = ProjectId::new();
        let base = Instant::now();
        let StatusRead::Stale(pending) = cache.read(&project_id, base, STATUS_CACHE_TTL) else {
            panic!("빈 캐시는 계산을 요구해야 한다");
        };

        cache.invalidate(&project_id);
        cache.finish(&project_id, pending, &sample_status("main"));

        assert!(
            matches!(cache.read(&project_id, base, STATUS_CACHE_TTL), StatusRead::Stale(_)),
            "libgit2 가 워크트리를 도는 사이에 변경이 보고됐다면 그 결과는 이미 낡은 것이므로 저장되면 안 된다"
        );
    }

    #[test]
    fn 계산_중에_프로젝트가_닫히면_결과를_저장하지_않는다() {
        let cache = StatusCache::default();
        let project_id = ProjectId::new();
        let base = Instant::now();
        let StatusRead::Stale(pending) = cache.read(&project_id, base, STATUS_CACHE_TTL) else {
            panic!("빈 캐시는 계산을 요구해야 한다");
        };

        cache.forget(&project_id);
        cache.finish(&project_id, pending, &sample_status("main"));

        assert!(
            cache.slots.lock().is_empty(),
            "닫힌 프로젝트의 슬롯이 뒤늦게 끝난 계산으로 되살아나면 회수 계약(architecture.md §6.3)이 깨진다"
        );
    }

    #[test]
    fn 슬롯_회수_뒤_같은_id를_재조회해도_이전_계산은_새_슬롯을_덮지_않는다() {
        let cache = StatusCache::default();
        let project_id = ProjectId::new();
        let base = Instant::now();
        let StatusRead::Stale(pending) = cache.read(&project_id, base, STATUS_CACHE_TTL) else {
            panic!("빈 캐시는 계산을 요구해야 한다");
        };

        cache.forget(&project_id);
        fill_status(&cache, &project_id, base, &sample_status("release"));
        cache.finish(&project_id, pending, &sample_status("main"));

        assert!(
            is_fresh(&cache.read(&project_id, base, STATUS_CACHE_TTL), "release"),
            "회수된 슬롯의 늦은 완료가 같은 ID의 새 슬롯을 덮으면 안 된다"
        );
    }

    #[test]
    fn 무효화는_다른_프로젝트의_캐시를_건드리지_않는다() {
        let cache = StatusCache::default();
        let changed = ProjectId::new();
        let untouched = ProjectId::new();
        let base = Instant::now();
        fill_status(&cache, &changed, base, &sample_status("main"));
        fill_status(&cache, &untouched, base, &sample_status("release"));

        cache.invalidate(&changed);

        assert!(matches!(cache.read(&changed, base, STATUS_CACHE_TTL), StatusRead::Stale(_)));
        assert!(is_fresh(&cache.read(&untouched, base, STATUS_CACHE_TTL), "release"));
    }

    #[test]
    fn 모르는_프로젝트에_대한_무효화는_슬롯을_만들지_않는다() {
        let cache = StatusCache::default();

        cache.invalidate(&ProjectId::new());

        assert!(
            cache.slots.lock().is_empty(),
            "status 를 한 번도 묻지 않은 프로젝트의 fs:changed 까지 슬롯을 만들면 맵이 열린 프로젝트 수보다 커진다"
        );
    }

    #[test]
    fn 무효화_한_번마다_status_계산은_한_번뿐이다() {
        let cache = StatusCache::default();
        let project_id = ProjectId::new();
        let base = Instant::now();
        let mut computed = 0u64;

        for step in 0..BUDGET_READS {
            match cache.read(
                &project_id,
                base + Duration::from_millis(step * BUDGET_READ_SPACING_MS),
                STATUS_CACHE_TTL,
            ) {
                StatusRead::Fresh(_) => {}
                StatusRead::Stale(pending) => {
                    computed += 1;
                    cache.finish(&project_id, pending, &sample_status("main"));
                }
            }
        }

        assert_eq!(computed, 1, "한 번의 무효화 뒤 {BUDGET_READS}회 조회는 계산 1회로 끝나야 한다");
    }

    #[test]
    fn 매번_무효화되면_계산_횟수는_캐시_도입_전과_같다() {
        let cache = StatusCache::default();
        let project_id = ProjectId::new();
        let base = Instant::now();
        let mut computed = 0u64;

        for step in 0..BUDGET_READS {
            cache.invalidate(&project_id);
            match cache.read(
                &project_id,
                base + Duration::from_millis(step * BUDGET_READ_SPACING_MS),
                STATUS_CACHE_TTL,
            ) {
                StatusRead::Fresh(_) => {}
                StatusRead::Stale(pending) => {
                    computed += 1;
                    cache.finish(&project_id, pending, &sample_status("main"));
                }
            }
        }

        assert_eq!(
            computed, BUDGET_READS,
            "변경이 계속 보고되는 동안에는 캐시가 한 번도 서빙되면 안 된다"
        );
    }

    #[test]
    fn remove는_repo_root가_없어도_status_캐시를_회수한다() {
        let store = GitStore::new();
        let project_id = ProjectId::new();
        fill_status(&store.0.status_cache, &project_id, Instant::now(), &sample_status("main"));

        store.remove(&project_id);

        assert!(
            store.0.status_cache.slots.lock().is_empty(),
            "repo_root 를 해석한 적 없는 프로젝트도 status 슬롯은 닫힐 때 회수되어야 한다"
        );
    }

    #[test]
    fn remove는_닫는_프로젝트의_status_캐시만_회수한다() {
        let store = GitStore::new();
        let closing = ProjectId::new();
        let staying = ProjectId::new();
        let base = Instant::now();
        store
            .0
            .repo_roots
            .lock()
            .insert(closing.clone(), PathBuf::from("/tmp/closing-repo"));
        fill_status(&store.0.status_cache, &closing, base, &sample_status("main"));
        fill_status(&store.0.status_cache, &staying, base, &sample_status("release"));

        store.remove(&closing);

        assert!(!store.0.status_cache.slots.lock().contains_key(&closing));
        assert!(is_fresh(&store.0.status_cache.read(&staying, base, STATUS_CACHE_TTL), "release"));
    }

    #[test]
    fn invalidate_status는_status_캐시를_비운다() {
        let store = GitStore::new();
        let project_id = ProjectId::new();
        let base = Instant::now();
        fill_status(&store.0.status_cache, &project_id, base, &sample_status("main"));

        store.invalidate_status(&project_id);

        assert!(matches!(
            store.0.status_cache.read(&project_id, base, STATUS_CACHE_TTL),
            StatusRead::Stale(_)
        ));
    }

    #[test]
    fn remove는_해당_프로젝트의_캐시된_repo_root만_지운다() {
        let store = GitStore::new();
        let closing = ProjectId::new();
        let staying = ProjectId::new();
        store
            .0
            .repo_roots
            .lock()
            .insert(closing.clone(), PathBuf::from("/tmp/closing-repo"));
        store
            .0
            .repo_roots
            .lock()
            .insert(staying.clone(), PathBuf::from("/tmp/staying-repo"));

        store.remove(&closing);

        assert!(
            !store.0.repo_roots.lock().contains_key(&closing),
            "닫힌 프로젝트의 캐시는 제거되어야 한다"
        );
        assert!(
            store.0.repo_roots.lock().contains_key(&staying),
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

        tokio::time::sleep(Duration::from_millis(LOCK_WAIT_TIMEOUT_MS)).await;
        assert!(
            !waiter.is_finished(),
            "먼저 락을 쥔 push/fetch 가 끝나기 전까지 같은 repo 의 두 번째 호출은 대기해야 한다"
        );

        let other_repo_lock = store.push_fetch_lock(&PathBuf::from("/tmp/other-repo"));
        let other_repo_result = tokio::time::timeout(Duration::from_millis(LOCK_WAIT_TIMEOUT_MS), other_repo_lock.lock()).await;
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
        store.0.repo_roots.lock().insert(project_id.clone(), root.clone());
        store.push_fetch_lock(&root);

        store.remove(&project_id);

        assert!(
            !store.0.push_fetch_locks.lock().contains_key(&root),
            "아무도 쥐고 있지 않은 push/fetch 락은 프로젝트가 닫힐 때 함께 제거되어야 누수가 없다"
        );
    }

    #[test]
    fn remove는_사용중인_push_fetch_락은_보존하고_이후_close에서_회수한다() {
        let store = GitStore::new();
        let project_id = ProjectId::new();
        let root = PathBuf::from("/tmp/busy-repo");
        store.0.repo_roots.lock().insert(project_id.clone(), root.clone());
        let in_flight = store.push_fetch_lock(&root);

        store.remove(&project_id);
        assert!(
            store.0.push_fetch_locks.lock().contains_key(&root),
            "진행 중인 push/fetch 가 쥔 Arc 가 있으면 같은 repo 가 재오픈됐을 때 새 호출도 그 락을 공유해야 하므로 즉시 제거하면 안 된다"
        );

        drop(in_flight);
        store.0.repo_roots.lock().insert(project_id.clone(), root.clone());
        store.remove(&project_id);
        assert!(
            !store.0.push_fetch_locks.lock().contains_key(&root),
            "진행 중이던 호출이 끝난 뒤 다시 close 하면 그때는 회수되어야 한다"
        );
    }

    #[test]
    fn 무효화_구독_초기화는_복제본_사이에서_한_번만_실행된다() {
        let store = GitStore::new();
        let clone = store.clone();
        let mut registrations = 0;

        store.ensure_invalidation_listeners(|| registrations += 1);
        clone.ensure_invalidation_listeners(|| registrations += 1);

        assert_eq!(registrations, 1);
    }
}

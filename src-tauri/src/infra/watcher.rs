use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::event::{EventKind, ModifyKind};
use notify::RecursiveMode;
use notify_debouncer_full::file_id::{get_file_id, FileId};
use notify_debouncer_full::{new_debouncer_opt, DebounceEventResult, DebouncedEvent, Debouncer, FileIdCache};

use crate::constants::{is_ignored_dir, WATCH_DEBOUNCE_MS};
use crate::domain::file::types::{FsChange, FsChangeKind};
use crate::error::{AppError, AppErrorKind, AppResult};
use crate::infra::persist;

/// One live watch. Dropping it stops the debouncer thread, which is how `project_close` and
/// `FileWatcherCapability::detach` end a watch (`AppState::watchers`/`git_watchers` hold these).
///
/// The `notify-debouncer-full` cache type parameter is [`ScopedIdCache`], not the crate's
/// `RecommendedCache` — see that type's doc for why, and for the one behavior it trades away.
pub struct WatcherHandle {
    _debouncer: Debouncer<notify::RecommendedWatcher, ScopedIdCache>,
}

/// Which of the two watchers a [`start_watch`] call is: the project-root watcher, whose consumers
/// (tree, editor, git status) have no use for churn inside `IGNORED_DIR_NAMES` directories, and the
/// `.git`-directory watcher (`domain::git::watch`), where that same name list means nothing —
/// `refs/heads/**` mirrors *branch* names, so a perfectly ordinary `build/login` or `dist` branch
/// would otherwise have every one of its ref events dropped. The git watcher's own noise floor is
/// already handled downstream by `classify_git_change` (only `index`/`HEAD`/`refs/**` survive, and
/// `objects/**` is dropped), so it needs no name filtering here at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchScope {
    Project,
    GitDir,
}

/// The `notify-debouncer-full` file-ID cache, restricted to the paths this app actually reports.
///
/// # Why this exists
///
/// The crate's own `RecommendedCache` (macOS/Windows: `FileIdMap`) indexes the watch root with
/// `WalkDir::new(root).follow_links(true).max_depth(usize::MAX)` and `stat`s **every** entry —
/// `IGNORED_DIR_NAMES` is not applied, so `node_modules`/`target`/`.git` are walked in full. On this
/// repository that is 557,474 entries against the 1,461 the app ever reports on: a 3.79s stat walk
/// per project open (warm), and a `HashMap<PathBuf, FileId>` of roughly 105MB resident **per open
/// project**. Every `remove_path` — one per rename half — is an O(n) `retain` over that same map.
/// The walk also re-runs on any `Create` of a directory (`add_event`'s `EventKind::Create` arm calls
/// `add_path` with the root's recursive mode), so a single `npm install` re-indexes the whole tree.
/// Numbers and the full cost breakdown: `docs/architecture.md` §2.3, research
/// `docs/research/2026-09-04-batch4-topics1-5-research.md` 주제 3b §2-A/§2-B.
///
/// This cache indexes exactly the paths `group_relevant_changes` can still emit: it prunes at the
/// ignored directory rather than below it, keeping the ignored directory's **own** entry (that path
/// survives the event filter — see [`is_ignored_path`]) while never descending into it.
///
/// # What it costs — and how that differs from d-35's rejected `NoCache`
///
/// macOS FSEvents ships no rename cookie, so `handle_rename_to`'s `file_ids_match` is the only way
/// the debouncer ever stitches a rename's two halves. d-35 §4-e
/// (`docs/acknowledge/2026-08-25-d35-rust-hardening-contract.md`) rejected switching to `NoCache`
/// because losing stitching **everywhere** also loses `push_rename_event`'s re-attribution: other
/// events queued for the old path in the same debounce window (a content `Modified` right before the
/// rename) stay pinned to a path that no longer exists.
///
/// A scoped cache keeps stitching for every path pair the app reports on. It only stops stitching
/// when one side sits under an ignored directory, and there the outcome is the same either way:
/// a stitched `Modify(Name(Both))` carries `paths = [old, new]`, of which
/// `group_relevant_changes` drops the ignored half anyway, and two unstitched halves arrive as
/// `Modify(Name(From))` + `Modify(Name(To))` — both mapped to `FsChangeKind::Renamed` by
/// [`map_event_kind`] — of which the ignored half is dropped by the same filter. Either way the
/// surviving side is one `Renamed` path, and `ipc-sync-provider.tsx` refreshes its parent directory.
/// Tab-following (`layout_apply_path_change`) is **not** affected at all: it is driven by
/// `useRenameEntry`'s own `onSuccess`, never by a `fs:changed` `Renamed` group.
///
/// The one residual loss is the re-attribution above, narrowed to a single sequence: modify a file
/// and move it across an ignore boundary within the same 300ms window. The `Modified` then stays on
/// the old (now gone) path instead of following the move — a stale `FILE.CONTENT` invalidation for a
/// path nothing can read, which is benign where d-35's global version was not.
///
/// # Symlinks
///
/// `FileIdMap` follows symlinked directories; this cache indexes the link entry itself (via
/// `get_file_id`, which resolves it) but does not descend. notify's FSEvents backend canonicalizes
/// the watch root (`notify-8.2.0/src/fsevent.rs`), so events never arrive addressed through a
/// symlink path inside the root — the entries `follow_links(true)` adds can never be looked up, and
/// skipping them also removes WalkDir's symlink-loop exposure.
///
/// # Platforms
///
/// One implementation for every target, deliberately: the release target is macOS only
/// (`.github/workflows/*` builds and tests Rust on `macos-latest`), and a `cfg`-swapped cache type
/// would make [`WatcherHandle`] a different type per platform. On Linux the cache is redundant
/// rather than wrong — inotify supplies rename cookies, so `trackers_match` stitches before
/// `file_ids_match` is consulted.
pub struct ScopedIdCache {
    root: PathBuf,
    scope: WatchScope,
    paths: HashMap<PathBuf, FileId>,
    indexed_entries: Arc<AtomicUsize>,
}

impl ScopedIdCache {
    pub fn new(root: PathBuf, scope: WatchScope) -> Self {
        Self {
            root,
            scope,
            paths: HashMap::new(),
            indexed_entries: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// How many paths are currently indexed — the memory axis contract §C.2 tracks ("워처 IdCache
    /// 엔트리 수").
    pub fn entry_count(&self) -> usize {
        self.paths.len()
    }

    /// A live view of [`entry_count`](Self::entry_count) that survives the cache being moved into
    /// the debouncer, which owns it behind its own private lock from then on.
    pub fn entry_counter(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.indexed_entries)
    }

    fn index(&mut self, path: &Path) {
        let Ok(file_id) = get_file_id(path) else {
            return;
        };
        self.paths.insert(path.to_path_buf(), file_id);
    }

    /// Whether `path` is an ignored directory whose children must not be indexed.
    ///
    /// The watch root itself is exempt no matter what it is called: a project living at
    /// `~/dev/build` would otherwise index nothing but its own root entry — the same shape of bug
    /// [`is_ignored_path`] documents for ancestors above the root.
    fn prunes_children_of(&self, path: &Path) -> bool {
        if self.scope == WatchScope::GitDir || path == self.root {
            return false;
        }
        path.file_name().and_then(|name| name.to_str()).map(is_ignored_dir).unwrap_or(false)
    }

    fn publish_entry_count(&self) {
        self.indexed_entries.store(self.paths.len(), Ordering::Relaxed);
    }
}

impl FileIdCache for ScopedIdCache {
    fn cached_file_id(&self, path: &Path) -> Option<impl AsRef<FileId>> {
        self.paths.get(path)
    }

    fn add_path(&mut self, path: &Path, recursive_mode: RecursiveMode) {
        if is_ignored_path(path, &self.root, self.scope) {
            return;
        }

        self.index(path);
        if !self.prunes_children_of(path) {
            let recursive = recursive_mode == RecursiveMode::Recursive;
            let mut pending = vec![path.to_path_buf()];

            while let Some(directory) = pending.pop() {
                let Ok(entries) = std::fs::read_dir(&directory) else {
                    continue;
                };
                for entry in entries.flatten() {
                    let child = entry.path();
                    self.index(&child);
                    if recursive && entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) && !self.prunes_children_of(&child) {
                        pending.push(child);
                    }
                }
            }
        }

        self.publish_entry_count();
    }

    fn remove_path(&mut self, path: &Path) {
        self.paths.retain(|cached, _| !cached.starts_with(path));
        self.publish_entry_count();
    }
}

/// `on_change` receives every non-empty [`FsChangeKind`] group from **one** debounce tick together,
/// never one group per call — `infra::self_write::resolve_from_app` depends on seeing a whole
/// batch at once to resolve `from_app` consistently for a path that (thanks to how
/// `write_atomic`'s create-then-rename surfaces to `notify`) can legitimately appear in more than
/// one of those groups in the same tick.
pub fn start_watch<F>(root: PathBuf, scope: WatchScope, on_change: F) -> AppResult<WatcherHandle>
where
    F: Fn(Vec<FsChange>) + Send + Sync + 'static,
{
    let on_change: Arc<F> = Arc::new(on_change);
    let watch_root = root.clone();

    let cache = ScopedIdCache::new(root.clone(), scope);
    let indexed_entries = cache.entry_counter();

    let mut debouncer = new_debouncer_opt::<_, notify::RecommendedWatcher, _>(
        Duration::from_millis(WATCH_DEBOUNCE_MS),
        None,
        move |result: DebounceEventResult| {
            let events = match result {
                Ok(events) => events,
                Err(errors) => {
                    for error in errors {
                        log::error!("파일 감시 오류: {error}");
                    }
                    return;
                }
            };

            let changes = group_relevant_changes(&events, &watch_root, scope);
            if !changes.is_empty() {
                on_change(changes);
            }
        },
        cache,
        notify::Config::default(),
    )
    .map_err(|error| {
        AppError::localized(
            AppErrorKind::Internal,
            "error.watcher.startFailed",
            format!("failed to start the file watcher: {error}"),
        )
        .with_arg("detail", &error)
    })?;

    let started = Instant::now();
    debouncer.watch(&root, RecursiveMode::Recursive).map_err(|error| {
        AppError::localized(
            AppErrorKind::Internal,
            "error.watcher.registerFailed",
            format!("failed to register the file watch: {error}"),
        )
        .with_arg("detail", &error)
    })?;

    log::debug!(
        "파일 감시 인덱싱 완료: {} 엔트리 / {}ms ({})",
        indexed_entries.load(Ordering::Relaxed),
        started.elapsed().as_millis(),
        root.display()
    );

    Ok(WatcherHandle { _debouncer: debouncer })
}

fn group_relevant_changes(events: &[DebouncedEvent], root: &Path, scope: WatchScope) -> Vec<FsChange> {
    let mut created = Vec::new();
    let mut modified = Vec::new();
    let mut removed = Vec::new();
    let mut renamed = Vec::new();

    for event in events {
        let Some(kind) = map_event_kind(&event.kind) else {
            continue;
        };

        for path in &event.paths {
            if is_ignored_path(path, root, scope) || persist::is_temp_sibling(path) {
                continue;
            }
            let path_str = path.to_string_lossy().to_string();
            match kind {
                FsChangeKind::Created => created.push(path_str),
                FsChangeKind::Modified => modified.push(path_str),
                FsChangeKind::Removed => removed.push(path_str),
                FsChangeKind::Renamed => renamed.push(path_str),
            }
        }
    }

    [
        (FsChangeKind::Created, created),
        (FsChangeKind::Modified, modified),
        (FsChangeKind::Removed, removed),
        (FsChangeKind::Renamed, renamed),
    ]
    .into_iter()
    .filter(|(_, paths)| !paths.is_empty())
    .map(|(kind, paths)| FsChange {
        kind,
        paths,
        from_app: false,
    })
    .collect()
}

fn map_event_kind(kind: &EventKind) -> Option<FsChangeKind> {
    match kind {
        EventKind::Create(_) => Some(FsChangeKind::Created),
        EventKind::Modify(ModifyKind::Name(_)) => Some(FsChangeKind::Renamed),
        EventKind::Modify(_) => Some(FsChangeKind::Modified),
        EventKind::Remove(_) => Some(FsChangeKind::Removed),
        _ => None,
    }
}

/// Matches `IGNORED_DIR_NAMES` against exactly one slice of `path`: its **ancestor directory
/// components below `root`**. Two ends are excluded on purpose.
///
/// The ancestors *above* `root` are excluded because checking the full absolute path used to mean a
/// project nested under any ancestor happening to be named `target`/`dist`/`build`/`.git`/... (an
/// extremely common layout — e.g. `~/dev/target/my-app`) silently dropped every single event the
/// watcher ever produced, since every path under it necessarily contains that ancestor component
/// too. See `docs/acknowledge/2026-08-18-audit-t0-fix-contract.md` §2.4 (#1).
///
/// The **last** component is excluded because the ignore list names *directories*, and the last
/// component is the entry the event is actually about — very often a file. A file named `build` or
/// `dist` sitting at the project root is shown in the tree (`domain::tree::service::read_children`
/// only skips ignored names when the entry is a directory), so dropping its events left the editor
/// and git status permanently blind to it. Whether the last component happens to be a directory
/// can't be decided from the path alone (it may already be deleted), and a create/remove event for
/// the ignored directory *itself* is a single event whose children stay filtered — so leaking that
/// one is cheaper than keeping the file-shaped hole.
///
/// Falls back to checking the full path only if `path` doesn't have `root` as a prefix at all (e.g.
/// a symlinked watch root notify reports through its resolved form) — an abnormal case where the
/// more conservative behavior is the safer default.
///
/// [`ScopedIdCache`] indexes exactly the complement of this predicate, so the file-ID cache and the
/// event filter can never disagree about which paths this watcher speaks for.
fn is_ignored_path(path: &Path, root: &Path, scope: WatchScope) -> bool {
    if scope == WatchScope::GitDir {
        return false;
    }

    let relative = path.strip_prefix(root).unwrap_or(path);
    let Some(ancestors) = relative.parent() else {
        return false;
    };
    ancestors
        .components()
        .any(|component| component.as_os_str().to_str().map(is_ignored_dir).unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("taide-watcher-test-{name}-{}", uuid::Uuid::new_v4()))
    }

    fn write_file(path: &Path) {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        std::fs::write(path, b"x").expect("write file");
    }

    /// A project-shaped tree: two reported files, one ignored directory holding a nested tree, and
    /// one *file* whose name collides with the ignore list.
    fn build_fixture_tree(root: &Path) {
        write_file(&root.join("src").join("main.rs"));
        write_file(&root.join("src").join("nested").join("deep.rs"));
        write_file(&root.join("build"));
        write_file(&root.join("node_modules").join("pkg").join("index.js"));
        write_file(&root.join("node_modules").join("pkg").join("nested").join("more.js"));
        write_file(&root.join("target").join("debug").join("taide"));
    }

    fn cleanup(dir: &Path) {
        std::fs::remove_dir_all(dir).ok();
    }

    fn is_cached(cache: &ScopedIdCache, path: &Path) -> bool {
        cache.cached_file_id(path).is_some()
    }

    #[test]
    fn 감시_루트_안의_무시_디렉토리_하위_경로는_필터링된다() {
        let root = Path::new("/repo");
        assert!(is_ignored_path(&root.join("node_modules/pkg/index.js"), root, WatchScope::Project));
        assert!(is_ignored_path(&root.join(".git/HEAD"), root, WatchScope::Project));
        assert!(!is_ignored_path(&root.join("src/main.rs"), root, WatchScope::Project));
    }

    #[test]
    fn 무시_판정은_마지막_성분을_보지_않는다() {
        let root = Path::new("/repo");
        assert!(
            !is_ignored_path(&root.join("build"), root, WatchScope::Project),
            "무시 목록은 디렉토리 이름이다 — 'build' 라는 이름의 파일이 루트에 있으면 트리에 보이므로 그 변경도 보여야 한다"
        );
        assert!(!is_ignored_path(&root.join("src/dist"), root, WatchScope::Project));
        assert!(
            is_ignored_path(&root.join("dist/index.js"), root, WatchScope::Project),
            "같은 이름이 조상 성분이면 그대로 무시된다"
        );
    }

    #[test]
    fn 감시_루트_밖의_조상_경로에_무시_이름이_있어도_필터링되지_않는다() {
        let root = Path::new("/Users/dev/target/my-project");
        assert!(
            !is_ignored_path(&root.join("src/main.rs"), root, WatchScope::Project),
            "루트 자체가 조상 경로에 'target' 을 포함해도 루트 하위 파일은 무시되면 안 된다"
        );
    }

    #[test]
    fn git_워처는_무시_목록을_적용하지_않는다() {
        let git_root = Path::new("/repo/.git");
        assert!(
            !is_ignored_path(&git_root.join("refs/heads/build/login"), git_root, WatchScope::GitDir),
            "refs/heads 아래 성분은 브랜치 이름이지 디렉토리 무시 대상이 아니다"
        );
        assert!(!is_ignored_path(&git_root.join("refs/heads/dist"), git_root, WatchScope::GitDir));
        assert!(!is_ignored_path(&git_root.join("index"), git_root, WatchScope::GitDir));
    }

    #[test]
    fn 경로가_루트_밖이면_전체_경로_기준으로_보수적으로_필터링된다() {
        let root = Path::new("/repo");
        assert!(is_ignored_path(Path::new("/elsewhere/.git/HEAD"), root, WatchScope::Project));
    }

    #[test]
    fn 이벤트_종류가_변경_종류로_매핑된다() {
        assert_eq!(
            map_event_kind(&EventKind::Create(notify::event::CreateKind::File)),
            Some(FsChangeKind::Created)
        );
        assert_eq!(
            map_event_kind(&EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Content))),
            Some(FsChangeKind::Modified)
        );
        assert_eq!(
            map_event_kind(&EventKind::Modify(ModifyKind::Name(notify::event::RenameMode::Both))),
            Some(FsChangeKind::Renamed)
        );
        assert_eq!(
            map_event_kind(&EventKind::Remove(notify::event::RemoveKind::File)),
            Some(FsChangeKind::Removed)
        );
        assert_eq!(map_event_kind(&EventKind::Access(notify::event::AccessKind::Any)), None);
    }

    #[test]
    fn write_atomic가_남기는_임시_형제_경로는_그룹에서_제외된다() {
        let root = Path::new("/repo");
        let target = root.join("src/main.rs");
        let temp = persist::temp_sibling(&target);

        let event = DebouncedEvent::new(
            notify::Event::new(EventKind::Create(notify::event::CreateKind::File))
                .add_path(temp)
                .add_path(target.clone()),
            std::time::Instant::now(),
        );

        let changes = group_relevant_changes(&[event], root, WatchScope::Project);

        assert_eq!(changes.len(), 1, "임시 형제 경로를 제외하면 최종 경로 하나만 남아야 한다");
        assert_eq!(changes[0].paths, vec![target.to_string_lossy().to_string()]);
    }

    #[test]
    fn 루트의_무시_이름_파일_변경은_그룹에_남는다() {
        let root = Path::new("/repo");
        let target = root.join("build");

        let event = DebouncedEvent::new(
            notify::Event::new(EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Content))).add_path(target.clone()),
            std::time::Instant::now(),
        );

        let changes = group_relevant_changes(&[event], root, WatchScope::Project);

        assert_eq!(
            changes.len(),
            1,
            "무시 목록은 조상 디렉토리에만 적용된다 — 'build' 라는 이름의 파일 변경은 드롭되면 안 된다"
        );
        assert_eq!(changes[0].paths, vec![target.to_string_lossy().to_string()]);
    }

    #[test]
    fn git_워처는_무시_이름_성분이_있는_브랜치_ref_도_그룹에_남긴다() {
        let git_root = Path::new("/repo/.git");
        let target = git_root.join("refs/heads/build/login");

        let event = DebouncedEvent::new(
            notify::Event::new(EventKind::Create(notify::event::CreateKind::File)).add_path(target.clone()),
            std::time::Instant::now(),
        );

        let changes = group_relevant_changes(&[event], git_root, WatchScope::GitDir);

        assert_eq!(
            changes.len(),
            1,
            "git 워처가 보는 것은 ref 이름이지 디렉토리 무시 대상이 아니다 — 'build/login' 브랜치 ref 이벤트가 드롭되면 브랜치 UI 가 갱신되지 않는다"
        );
        assert_eq!(changes[0].paths, vec![target.to_string_lossy().to_string()]);
    }

    #[test]
    fn 무시된_경로만_있으면_변경_그룹이_비어있다() {
        let event = DebouncedEvent::new(
            notify::Event::new(EventKind::Create(notify::event::CreateKind::File))
                .add_path(PathBuf::from("/repo/node_modules/pkg/index.js")),
            std::time::Instant::now(),
        );

        assert!(group_relevant_changes(&[event], Path::new("/repo"), WatchScope::Project).is_empty());
    }

    #[test]
    fn 캐시는_무시_디렉토리_하위를_인덱싱하지_않는다() {
        let root = temp_root("scoped-cache");
        build_fixture_tree(&root);

        let mut cache = ScopedIdCache::new(root.clone(), WatchScope::Project);
        cache.add_path(&root, RecursiveMode::Recursive);

        assert!(is_cached(&cache, &root.join("src").join("main.rs")));
        assert!(is_cached(&cache, &root.join("src").join("nested").join("deep.rs")));
        assert!(
            is_cached(&cache, &root.join("build")),
            "'build' 라는 이름의 파일은 이벤트가 살아남으므로 캐시에도 있어야 한다"
        );
        assert!(!is_cached(&cache, &root.join("node_modules").join("pkg").join("index.js")));
        assert!(!is_cached(&cache, &root.join("node_modules").join("pkg")));
        assert!(!is_cached(&cache, &root.join("target").join("debug").join("taide")));

        cleanup(&root);
    }

    #[test]
    fn 캐시는_무시_디렉토리_자신은_한_엔트리로_남긴다() {
        let root = temp_root("ignored-dir-entry");
        build_fixture_tree(&root);

        let mut cache = ScopedIdCache::new(root.clone(), WatchScope::Project);
        cache.add_path(&root, RecursiveMode::Recursive);

        assert!(
            is_cached(&cache, &root.join("node_modules")),
            "무시 디렉토리 자신의 이벤트는 group_relevant_changes 를 통과하므로(마지막 성분 제외 규칙) 그 rename 도 짝지어져야 한다"
        );
        assert!(is_cached(&cache, &root.join("target")));

        cleanup(&root);
    }

    #[test]
    fn 캐시_엔트리_수는_무시_디렉토리를_제외한_트리와_같다() {
        let root = temp_root("entry-count");
        build_fixture_tree(&root);

        let mut cache = ScopedIdCache::new(root.clone(), WatchScope::Project);
        let counter = cache.entry_counter();
        cache.add_path(&root, RecursiveMode::Recursive);

        let expected = [
            root.clone(),
            root.join("src"),
            root.join("src").join("main.rs"),
            root.join("src").join("nested"),
            root.join("src").join("nested").join("deep.rs"),
            root.join("build"),
            root.join("node_modules"),
            root.join("target"),
        ];
        assert_eq!(
            cache.entry_count(),
            expected.len(),
            "무시 디렉토리는 자신 1엔트리로 끝나고 하위는 전혀 인덱싱되지 않아야 한다"
        );
        for path in &expected {
            assert!(is_cached(&cache, path), "{} 가 인덱싱되지 않았다", path.display());
        }
        assert_eq!(
            counter.load(Ordering::Relaxed),
            cache.entry_count(),
            "카운터는 인덱스 크기를 그대로 반영한다"
        );

        cleanup(&root);
    }

    #[test]
    fn 감시_루트_이름이_무시_목록에_있어도_하위는_인덱싱된다() {
        let parent = temp_root("ignored-root-name");
        let root = parent.join("build");
        write_file(&root.join("src").join("main.rs"));

        let mut cache = ScopedIdCache::new(root.clone(), WatchScope::Project);
        cache.add_path(&root, RecursiveMode::Recursive);

        assert!(
            is_cached(&cache, &root.join("src").join("main.rs")),
            "루트 자신의 이름이 무시 목록에 있어도 그 프로젝트는 정상 감시 대상이다"
        );

        cleanup(&parent);
    }

    #[test]
    fn git_스코프_캐시는_무시_이름_디렉토리도_인덱싱한다() {
        let root = temp_root("git-scope");
        write_file(&root.join("refs").join("heads").join("build").join("login"));
        write_file(&root.join("index"));

        let mut cache = ScopedIdCache::new(root.clone(), WatchScope::GitDir);
        cache.add_path(&root, RecursiveMode::Recursive);

        assert!(
            is_cached(&cache, &root.join("refs").join("heads").join("build").join("login")),
            "'build' 는 여기서 브랜치 이름이므로 캐시에서 잘려서는 안 된다"
        );
        assert!(is_cached(&cache, &root.join("index")));

        cleanup(&root);
    }

    #[test]
    fn 비재귀_추가는_직계_자식까지만_인덱싱한다() {
        let root = temp_root("non-recursive");
        build_fixture_tree(&root);

        let mut cache = ScopedIdCache::new(root.clone(), WatchScope::Project);
        cache.add_path(&root, RecursiveMode::NonRecursive);

        assert!(is_cached(&cache, &root.join("src")));
        assert!(!is_cached(&cache, &root.join("src").join("main.rs")));

        cleanup(&root);
    }

    #[test]
    fn 무시_디렉토리_생성은_그_하위를_다시_인덱싱하지_않는다() {
        let root = temp_root("ignored-dir-created");
        write_file(&root.join("src").join("main.rs"));

        let mut cache = ScopedIdCache::new(root.clone(), WatchScope::Project);
        cache.add_path(&root, RecursiveMode::Recursive);
        let before = cache.entry_count();

        write_file(&root.join("node_modules").join("pkg").join("index.js"));
        cache.add_path(&root.join("node_modules"), RecursiveMode::Recursive);

        assert_eq!(
            cache.entry_count(),
            before + 1,
            "npm install 이 만든 node_modules 는 자신 1엔트리만 추가돼야 한다 (debouncer 의 Create 처리가 add_path 를 부른다)"
        );

        cleanup(&root);
    }

    #[test]
    fn remove_path_는_경로_접두사_하위를_함께_지운다() {
        let root = temp_root("remove-path");
        build_fixture_tree(&root);
        write_file(&root.join("src-old").join("legacy.rs"));

        let mut cache = ScopedIdCache::new(root.clone(), WatchScope::Project);
        cache.add_path(&root, RecursiveMode::Recursive);
        cache.remove_path(&root.join("src"));

        assert!(!is_cached(&cache, &root.join("src")));
        assert!(!is_cached(&cache, &root.join("src").join("nested").join("deep.rs")));
        assert!(
            is_cached(&cache, &root.join("src-old").join("legacy.rs")),
            "접두사 비교는 컴포넌트 단위다 — 'src-old' 는 'src' 의 하위가 아니다"
        );

        cleanup(&root);
    }

    #[test]
    fn 무시_경계를_넘는_이동은_짝짓기_대신_양쪽_이벤트로_관측된다() {
        let root = temp_root("cross-boundary-rename");
        build_fixture_tree(&root);

        let mut cache = ScopedIdCache::new(root.clone(), WatchScope::Project);
        cache.add_path(&root, RecursiveMode::Recursive);

        assert!(
            cache
                .cached_file_id(&root.join("node_modules").join("pkg").join("index.js"))
                .is_none(),
            "무시 디렉토리 하위는 file_id 가 없으므로 debouncer 의 file_ids_match 가 성립하지 않는다"
        );

        let from = root.join("node_modules").join("pkg").join("index.js");
        let to = root.join("src").join("index.js");
        let events = [
            DebouncedEvent::new(
                notify::Event::new(EventKind::Modify(ModifyKind::Name(notify::event::RenameMode::From))).add_path(from),
                std::time::Instant::now(),
            ),
            DebouncedEvent::new(
                notify::Event::new(EventKind::Modify(ModifyKind::Name(notify::event::RenameMode::To))).add_path(to.clone()),
                std::time::Instant::now(),
            ),
        ];

        let changes = group_relevant_changes(&events, &root, WatchScope::Project);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].kind, FsChangeKind::Renamed);
        assert_eq!(
            changes[0].paths,
            vec![to.to_string_lossy().to_string()],
            "짝지어지지 않은 두 이벤트도 둘 다 Renamed 로 매핑되고, 무시 디렉토리 쪽 절반은 기존 필터가 떨궈 소비자가 보는 결과는 짝지어졌을 때와 같다"
        );

        cleanup(&root);
    }
}

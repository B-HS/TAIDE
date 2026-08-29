use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use notify::event::{EventKind, ModifyKind};
use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult, DebouncedEvent, Debouncer, RecommendedCache};

use crate::constants::{is_ignored_dir, WATCH_DEBOUNCE_MS};
use crate::domain::file::types::{FsChange, FsChangeKind};
use crate::error::{AppError, AppErrorKind, AppResult};
use crate::infra::persist;

/// Uses `notify-debouncer-full`'s platform-default `RecommendedCache` (macOS: `FileIdMap`, not
/// `NoCache`) deliberately — switching to `NoCache` was considered and rejected (d-35 §4-e,
/// `docs/acknowledge/2026-08-25-d35-rust-hardening-contract.md`): macOS FSEvents provides no
/// pairing cookie between a rename's old and new sides, so without `FileIdMap`'s inode-based
/// matching, an unmatched rename half gets misreported as a `Modified` on a path that no longer
/// exists.
pub struct WatcherHandle {
    _debouncer: Debouncer<notify::RecommendedWatcher, RecommendedCache>,
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

    let mut debouncer = new_debouncer(
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
    )
    .map_err(|error| {
        AppError::localized(
            AppErrorKind::Internal,
            "error.watcher.startFailed",
            format!("failed to start the file watcher: {error}"),
        )
        .with_arg("detail", &error)
    })?;

    debouncer.watch(&root, RecursiveMode::Recursive).map_err(|error| {
        AppError::localized(
            AppErrorKind::Internal,
            "error.watcher.registerFailed",
            format!("failed to register the file watch: {error}"),
        )
        .with_arg("detail", &error)
    })?;

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
}

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use notify::event::{EventKind, ModifyKind};
use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult, DebouncedEvent, Debouncer, RecommendedCache};

use crate::constants::{is_ignored_dir, WATCH_DEBOUNCE_MS};
use crate::domain::file::types::{FsChange, FsChangeKind};
use crate::error::{AppError, AppResult};
use crate::infra::persist;

pub struct WatcherHandle {
    _debouncer: Debouncer<notify::RecommendedWatcher, RecommendedCache>,
}

/// `on_change` receives every non-empty [`FsChangeKind`] group from **one** debounce tick together,
/// never one group per call — `infra::self_write::resolve_from_app` depends on seeing a whole
/// batch at once to resolve `from_app` consistently for a path that (thanks to how
/// `write_atomic`'s create-then-rename surfaces to `notify`) can legitimately appear in more than
/// one of those groups in the same tick.
pub fn start_watch<F>(root: PathBuf, on_change: F) -> AppResult<WatcherHandle>
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

            let changes = group_relevant_changes(&events, &watch_root);
            if !changes.is_empty() {
                on_change(changes);
            }
        },
    )
    .map_err(|error| AppError::Internal(format!("파일 감시자를 시작하지 못했습니다: {error}")))?;

    debouncer
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|error| AppError::Internal(format!("파일 감시 등록에 실패했습니다: {error}")))?;

    Ok(WatcherHandle { _debouncer: debouncer })
}

fn group_relevant_changes(events: &[DebouncedEvent], root: &Path) -> Vec<FsChange> {
    let mut created = Vec::new();
    let mut modified = Vec::new();
    let mut removed = Vec::new();
    let mut renamed = Vec::new();

    for event in events {
        let Some(kind) = map_event_kind(&event.kind) else {
            continue;
        };

        for path in &event.paths {
            if is_ignored_path(path, root) || persist::is_temp_sibling(path) {
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

/// Checks `IGNORED_DIR_NAMES` only against `path`'s components **below `root`** — the directory
/// actually being watched — never the ancestor path leading up to it. Checking the full absolute
/// path used to mean a project (or the `.git` directory `attach_git_watcher` watches on its own)
/// nested under any ancestor happening to be named `target`/`dist`/`build`/`.git`/... (an extremely
/// common layout — e.g. `~/dev/target/my-app`, or the git watcher's own root, which is literally
/// named `.git`) silently dropped every single event the watcher ever produced, since every path
/// under it necessarily contains that ancestor component too. Falls back to checking the full path
/// only if `path` doesn't have `root` as a prefix at all (e.g. a symlinked watch root notify
/// reports through its resolved form) — an abnormal case where the old, more conservative behavior
/// is the safer default. See `docs/acknowledge/2026-08-18-audit-t0-fix-contract.md` §2.4 (#1).
fn is_ignored_path(path: &Path, root: &Path) -> bool {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .any(|component| component.as_os_str().to_str().map(is_ignored_dir).unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 감시_루트_안의_무시_디렉토리_하위_경로는_필터링된다() {
        let root = Path::new("/repo");
        assert!(is_ignored_path(&root.join("node_modules/pkg/index.js"), root));
        assert!(is_ignored_path(&root.join(".git/HEAD"), root));
        assert!(!is_ignored_path(&root.join("src/main.rs"), root));
    }

    #[test]
    fn 감시_루트_밖의_조상_경로에_무시_이름이_있어도_필터링되지_않는다() {
        let root = Path::new("/Users/dev/target/my-project");
        assert!(
            !is_ignored_path(&root.join("src/main.rs"), root),
            "루트 자체가 조상 경로에 'target' 을 포함해도 루트 하위 파일은 무시되면 안 된다"
        );
    }

    #[test]
    fn 감시_루트_자체가_git_디렉토리면_그_하위_파일은_무시되지_않는다() {
        let git_root = Path::new("/repo/.git");
        assert!(
            !is_ignored_path(&git_root.join("HEAD"), git_root),
            "git 워처는 루트 자체가 .git 이므로, 그 하위 파일이 전부 무시되면 워처가 완전히 무력화된다"
        );
    }

    #[test]
    fn 경로가_루트_밖이면_전체_경로_기준으로_보수적으로_필터링된다() {
        let root = Path::new("/repo");
        assert!(is_ignored_path(Path::new("/elsewhere/.git/HEAD"), root));
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

        let changes = group_relevant_changes(&[event], root);

        assert_eq!(changes.len(), 1, "임시 형제 경로를 제외하면 최종 경로 하나만 남아야 한다");
        assert_eq!(changes[0].paths, vec![target.to_string_lossy().to_string()]);
    }

    #[test]
    fn 무시된_경로만_있으면_변경_그룹이_비어있다() {
        let event = DebouncedEvent::new(
            notify::Event::new(EventKind::Create(notify::event::CreateKind::File))
                .add_path(PathBuf::from("/repo/node_modules/pkg/index.js")),
            std::time::Instant::now(),
        );

        assert!(group_relevant_changes(&[event], Path::new("/repo")).is_empty());
    }
}

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use notify::event::{EventKind, ModifyKind};
use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult, DebouncedEvent, Debouncer, RecommendedCache};

use crate::constants::{is_ignored_dir, WATCH_DEBOUNCE_MS};
use crate::domain::file::types::{FsChange, FsChangeKind};
use crate::error::{AppError, AppResult};

pub struct WatcherHandle {
    _debouncer: Debouncer<notify::RecommendedWatcher, RecommendedCache>,
}

pub fn start_watch<F>(root: PathBuf, on_change: F) -> AppResult<WatcherHandle>
where
    F: Fn(FsChange) + Send + Sync + 'static,
{
    let on_change: Arc<F> = Arc::new(on_change);

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

            for change in group_relevant_changes(&events) {
                on_change(change);
            }
        },
    )
    .map_err(|error| AppError::Internal(format!("파일 감시자를 시작하지 못했습니다: {error}")))?;

    debouncer
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|error| AppError::Internal(format!("파일 감시 등록에 실패했습니다: {error}")))?;

    Ok(WatcherHandle { _debouncer: debouncer })
}

fn group_relevant_changes(events: &[DebouncedEvent]) -> Vec<FsChange> {
    let mut created = Vec::new();
    let mut modified = Vec::new();
    let mut removed = Vec::new();
    let mut renamed = Vec::new();

    for event in events {
        let Some(kind) = map_event_kind(&event.kind) else {
            continue;
        };

        for path in &event.paths {
            if is_ignored_path(path) {
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

fn is_ignored_path(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str().to_str().map(is_ignored_dir).unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 무시_디렉토리_하위_경로는_필터링된다() {
        assert!(is_ignored_path(Path::new("/repo/node_modules/pkg/index.js")));
        assert!(is_ignored_path(Path::new("/repo/.git/HEAD")));
        assert!(!is_ignored_path(Path::new("/repo/src/main.rs")));
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
    fn 무시된_경로만_있으면_변경_그룹이_비어있다() {
        let event = DebouncedEvent::new(
            notify::Event::new(EventKind::Create(notify::event::CreateKind::File))
                .add_path(PathBuf::from("/repo/node_modules/pkg/index.js")),
            std::time::Instant::now(),
        );

        assert!(group_relevant_changes(&[event]).is_empty());
    }
}

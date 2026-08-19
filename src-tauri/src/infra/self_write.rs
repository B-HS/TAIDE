use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use crate::domain::file::types::FsChange;

/// How long a path marked via [`SelfWriteTracker::mark`] stays eligible to be recognized as the
/// app's own write by [`resolve_from_app`] — long enough to comfortably outlive the filesystem
/// watcher's own debounce window (`WATCH_DEBOUNCE_MS`) plus dispatch latency, short enough that a
/// genuinely external edit to the same path minutes later is never mistaken for an echo.
const SELF_WRITE_TTL_MS: u64 = 2_000;

/// Tracks paths TAIDE itself just wrote to disk (`file_save`/`file_create`/`file_rename`/
/// `file_delete`/`file_copy`/`search_replace`), so the filesystem watcher can mark the `FsChanged`
/// event those writes inevitably trigger as `from_app: true` instead of indistinguishable from an
/// external edit (X1#10, `docs/acknowledge/2026-08-19-xa-wiring-cleanup-contract.md` §1.1). A missed
/// mark (the write path was never recorded, or its marker already expired) simply falls back to
/// today's behavior — `from_app: false` — which is the safe direction: this tracker only ever adds a
/// `true` a caller can positively earn, never removes one.
#[derive(Default)]
pub struct SelfWriteTracker(Mutex<HashMap<PathBuf, Instant>>);

impl SelfWriteTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records that the app itself just wrote `path` to disk.
    pub fn mark(&self, path: &Path) {
        self.0.lock().insert(path.to_path_buf(), Instant::now());
    }

    /// Reports whether `path` was [`mark`](Self::mark)ed within [`SELF_WRITE_TTL_MS`] — and, either
    /// way, consumes the entry so a single recorded write is never matched twice. Consuming on a
    /// stale hit too (not just a fresh one) keeps a marker from lingering past its own TTL check into
    /// a *later*, unrelated watcher batch that happens to touch the same path again.
    fn take_recent(&self, path: &Path) -> bool {
        let Some(marked_at) = self.0.lock().remove(path) else {
            return false;
        };
        marked_at.elapsed() < Duration::from_millis(SELF_WRITE_TTL_MS)
    }
}

/// Decides `change.from_app` for one watcher-grouped [`FsChange`] and returns the change with that
/// field applied — every other field passes through untouched. Marks the whole group `from_app: true`
/// only when **every** path in it was just written by the app (`tracker.take_recent` for each);
/// a group mixing a self-write with an untracked/external path is conservatively left `false` rather
/// than guessing, since mislabeling an external change as the app's own (the one direction this
/// mechanism must never do — see the module doc) is worse than under-marking. Every path checked is
/// consumed from `tracker` regardless of the group's final verdict, so a marker is never matched by a
/// second, later, unrelated batch.
pub fn resolve_from_app(tracker: &SelfWriteTracker, change: FsChange) -> FsChange {
    let matches: Vec<bool> = change.paths.iter().map(|path| tracker.take_recent(Path::new(path))).collect();
    let from_app = !matches.is_empty() && matches.into_iter().all(|matched| matched);
    FsChange { from_app, ..change }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::file::types::FsChangeKind;

    fn change(paths: &[&str]) -> FsChange {
        FsChange {
            kind: FsChangeKind::Modified,
            paths: paths.iter().map(|path| path.to_string()).collect(),
            from_app: false,
        }
    }

    #[test]
    fn 표시된_경로_하나짜리_변경은_from_app이_true가_된다() {
        let tracker = SelfWriteTracker::new();
        tracker.mark(Path::new("/repo/a.rs"));

        let resolved = resolve_from_app(&tracker, change(&["/repo/a.rs"]));

        assert!(resolved.from_app);
    }

    #[test]
    fn 표시되지_않은_경로는_from_app이_false로_유지된다() {
        let tracker = SelfWriteTracker::new();

        let resolved = resolve_from_app(&tracker, change(&["/repo/a.rs"]));

        assert!(!resolved.from_app, "외부 변경은 절대 from_app 으로 오마킹되면 안 된다");
    }

    #[test]
    fn 일부만_표시된_묶음은_전체가_from_app_false로_보수적으로_처리된다() {
        let tracker = SelfWriteTracker::new();
        tracker.mark(Path::new("/repo/a.rs"));

        let resolved = resolve_from_app(&tracker, change(&["/repo/a.rs", "/repo/external.rs"]));

        assert!(
            !resolved.from_app,
            "묶음 안에 앱이 쓰지 않은 경로가 하나라도 있으면 전체를 외부 변경으로 취급해야 한다"
        );
    }

    #[test]
    fn 전부_표시된_묶음은_from_app이_true가_된다() {
        let tracker = SelfWriteTracker::new();
        tracker.mark(Path::new("/repo/a.rs"));
        tracker.mark(Path::new("/repo/b.rs"));

        let resolved = resolve_from_app(&tracker, change(&["/repo/a.rs", "/repo/b.rs"]));

        assert!(resolved.from_app);
    }

    #[test]
    fn ttl이_지난_마킹은_from_app으로_인정되지_않는다() {
        let tracker = SelfWriteTracker::new();
        tracker.0.lock().insert(
            PathBuf::from("/repo/a.rs"),
            Instant::now() - Duration::from_millis(SELF_WRITE_TTL_MS + 500),
        );

        let resolved = resolve_from_app(&tracker, change(&["/repo/a.rs"]));

        assert!(!resolved.from_app, "TTL 이 지난 마킹은 더 이상 자기-쓰기로 인정되면 안 된다");
    }

    #[test]
    fn 마킹은_한_번만_소비된다() {
        let tracker = SelfWriteTracker::new();
        tracker.mark(Path::new("/repo/a.rs"));

        let first = resolve_from_app(&tracker, change(&["/repo/a.rs"]));
        let second = resolve_from_app(&tracker, change(&["/repo/a.rs"]));

        assert!(first.from_app, "첫 확인은 마킹을 소비하며 true 를 반환해야 한다");
        assert!(!second.from_app, "같은 마킹이 두 번째 묶음에서 재사용되면 안 된다");
    }

    #[test]
    fn 빈_경로_묶음은_from_app이_false다() {
        let tracker = SelfWriteTracker::new();

        let resolved = resolve_from_app(&tracker, change(&[]));

        assert!(!resolved.from_app);
    }
}

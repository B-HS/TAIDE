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

/// Decides `from_app` for every [`FsChange`] group `infra::watcher::start_watch` produced from
/// **one** debounce tick, and returns the same changes with that field applied — every other field
/// passes through untouched. Callers must pass every group from a single tick together in one call;
/// splitting them across separate calls silently breaks the batching this function relies on (see
/// below).
///
/// A group is `from_app: true` only when **every** path in it was just written by the app
/// (`tracker.take_recent`); a group mixing a self-write with an untracked/external path is
/// conservatively left `false` rather than guessing, since mislabeling an external change as the
/// app's own (the one direction this mechanism must never do — see the module doc) is worse than
/// under-marking.
///
/// Each *unique path* is checked against `tracker` at most once per call, and that one verdict is
/// reused for every group the path appears in. This matters because `write_atomic`'s
/// create-temp-then-rename sequence (`persist.rs`) can make a single logical save surface as the
/// *same* final path in more than one of `group_relevant_changes`'s four kind buckets (e.g. both
/// `Created` and `Renamed`) within the same tick, and `SelfWriteTracker::take_recent` consumes its
/// marker on the very first lookup. Checking each group independently — this function's original
/// design — meant only the first-processed group of the batch ever saw a match; every other group
/// touching the same path always fell back to `from_app: false` and forced a tree refresh anyway,
/// defeating the point of marking it at all (X1#10,
/// `docs/acknowledge/2026-08-19-xa-wiring-cleanup-contract.md` §4's follow-up finding).
pub fn resolve_from_app(tracker: &SelfWriteTracker, changes: Vec<FsChange>) -> Vec<FsChange> {
    let mut verdicts: HashMap<String, bool> = HashMap::new();
    for change in &changes {
        for path in &change.paths {
            verdicts.entry(path.clone()).or_insert_with(|| tracker.take_recent(Path::new(path)));
        }
    }

    changes
        .into_iter()
        .map(|change| {
            let from_app = !change.paths.is_empty() && change.paths.iter().all(|path| verdicts[path]);
            FsChange { from_app, ..change }
        })
        .collect()
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

    /// Resolves a single [`FsChange`] as its own one-group batch — most tests here only need to
    /// assert on one group's verdict, so this keeps them from having to unwrap a `Vec` every time.
    fn resolve_one(tracker: &SelfWriteTracker, change: FsChange) -> FsChange {
        resolve_from_app(tracker, vec![change])
            .into_iter()
            .next()
            .expect("배치에 그룹이 하나 있어야 한다")
    }

    #[test]
    fn 표시된_경로_하나짜리_변경은_from_app이_true가_된다() {
        let tracker = SelfWriteTracker::new();
        tracker.mark(Path::new("/repo/a.rs"));

        let resolved = resolve_one(&tracker, change(&["/repo/a.rs"]));

        assert!(resolved.from_app);
    }

    #[test]
    fn 표시되지_않은_경로는_from_app이_false로_유지된다() {
        let tracker = SelfWriteTracker::new();

        let resolved = resolve_one(&tracker, change(&["/repo/a.rs"]));

        assert!(!resolved.from_app, "외부 변경은 절대 from_app 으로 오마킹되면 안 된다");
    }

    #[test]
    fn 일부만_표시된_묶음은_전체가_from_app_false로_보수적으로_처리된다() {
        let tracker = SelfWriteTracker::new();
        tracker.mark(Path::new("/repo/a.rs"));

        let resolved = resolve_one(&tracker, change(&["/repo/a.rs", "/repo/external.rs"]));

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

        let resolved = resolve_one(&tracker, change(&["/repo/a.rs", "/repo/b.rs"]));

        assert!(resolved.from_app);
    }

    #[test]
    fn ttl이_지난_마킹은_from_app으로_인정되지_않는다() {
        let tracker = SelfWriteTracker::new();
        tracker.0.lock().insert(
            PathBuf::from("/repo/a.rs"),
            Instant::now() - Duration::from_millis(SELF_WRITE_TTL_MS + 500),
        );

        let resolved = resolve_one(&tracker, change(&["/repo/a.rs"]));

        assert!(!resolved.from_app, "TTL 이 지난 마킹은 더 이상 자기-쓰기로 인정되면 안 된다");
    }

    #[test]
    fn 마킹은_서로_다른_배치_사이에서는_한_번만_소비된다() {
        let tracker = SelfWriteTracker::new();
        tracker.mark(Path::new("/repo/a.rs"));

        let first = resolve_one(&tracker, change(&["/repo/a.rs"]));
        let second = resolve_one(&tracker, change(&["/repo/a.rs"]));

        assert!(first.from_app, "첫 배치는 마킹을 소비하며 true 를 반환해야 한다");
        assert!(!second.from_app, "같은 마킹이 두 번째(별개) 배치에서 재사용되면 안 된다");
    }

    #[test]
    fn 같은_배치_안에서_같은_경로가_여러_그룹에_나타나도_전부_from_app이_true로_일치한다() {
        // write_atomic 의 temp-sibling-then-rename 이 notify 를 거쳐 같은 최종 경로를
        // Created/Modified/Renamed 여러 그룹으로 나눠 보고하는 실측 패턴(self_write.rs 모듈
        // 문서 참조)을 재현한다 — 마킹은 경로당 1개뿐이지만 세 그룹 모두 true 로 일치해야 한다.
        let tracker = SelfWriteTracker::new();
        tracker.mark(Path::new("/repo/a.rs"));

        let batch = vec![
            FsChange {
                kind: FsChangeKind::Created,
                paths: vec!["/repo/a.rs".to_string()],
                from_app: false,
            },
            FsChange {
                kind: FsChangeKind::Modified,
                paths: vec!["/repo/a.rs".to_string(), "/repo/a.rs".to_string()],
                from_app: false,
            },
            FsChange {
                kind: FsChangeKind::Renamed,
                paths: vec!["/repo/a.rs".to_string()],
                from_app: false,
            },
        ];

        let resolved = resolve_from_app(&tracker, batch);

        assert!(
            resolved.iter().all(|change| change.from_app),
            "한 배치 안의 모든 그룹이 true 로 일치해야 한다"
        );
    }

    #[test]
    fn 같은_배치_안에서도_외부_경로가_섞인_그룹은_false로_남는다() {
        let tracker = SelfWriteTracker::new();
        tracker.mark(Path::new("/repo/a.rs"));

        let batch = vec![
            FsChange {
                kind: FsChangeKind::Created,
                paths: vec!["/repo/a.rs".to_string()],
                from_app: false,
            },
            FsChange {
                kind: FsChangeKind::Modified,
                paths: vec!["/repo/a.rs".to_string(), "/repo/external.rs".to_string()],
                from_app: false,
            },
        ];

        let resolved = resolve_from_app(&tracker, batch);

        assert!(resolved[0].from_app, "앱이 쓴 경로만 있는 그룹은 true 여야 한다");
        assert!(!resolved[1].from_app, "외부 경로가 섞인 그룹은 배치 안이라도 false 로 남아야 한다");
    }

    #[test]
    fn 빈_경로_묶음은_from_app이_false다() {
        let tracker = SelfWriteTracker::new();

        let resolved = resolve_one(&tracker, change(&[]));

        assert!(!resolved.from_app);
    }
}

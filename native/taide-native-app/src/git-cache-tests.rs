use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use taide_model::app_event::AppEvent;
use taide_model::error::AppError;
use taide_model::file::{FsChange, FsChangeKind};
use taide_model::ids::ProjectId;

use super::{CacheKey, Changes, GitCache, Invalidation, MAX_REQUESTS_PER_FRAME, Request};

const CHANGED: &str = "/synthetic/project/changed.txt";
const UNCHANGED: &str = "/synthetic/project/unchanged.txt";
const STALE_AFTER: Duration = Duration::from_secs(60);
const BEFORE_STALE: Duration = Duration::from_secs(30);
const AFTER_STALE: Duration = Duration::from_secs(61);
const DISCARD_AFTER: Duration = Duration::from_secs(10 * 60);
const FRAME: Duration = Duration::from_millis(16);
const EXTRA_REQUESTS: usize = 4;
const FAILURE: &str = "synthetic repository";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Read {
    Status,
    Log,
    Diff(String),
    CommitFiles,
    CurrentUser,
    BlameOverlay,
}

impl CacheKey for Read {
    fn is_mutable(&self) -> bool {
        !matches!(self, Self::CommitFiles)
    }

    fn follows_worktree(&self, paths: &HashSet<String>) -> bool {
        match self {
            Self::Status => true,
            Self::Diff(path) => paths.contains(path),
            _ => false,
        }
    }

    fn stale_after(&self) -> Option<Duration> {
        match self {
            Self::CommitFiles | Self::CurrentUser => None,
            Self::BlameOverlay => Some(Duration::ZERO),
            _ => Some(STALE_AFTER),
        }
    }
}

fn reads() -> [Read; 6] {
    [
        Read::Status,
        Read::Log,
        Read::Diff(CHANGED.into()),
        Read::Diff(UNCHANGED.into()),
        Read::CommitFiles,
        Read::CurrentUser,
    ]
}

fn observe(cache: &mut GitCache<Read, u32>, project: &ProjectId, keys: &[Read], now: Instant) {
    for key in keys {
        cache.observe(project, key.clone(), now);
    }
}

fn value(cache: &GitCache<Read, u32>, project: &ProjectId, key: &Read) -> Option<u32> {
    cache.get(project, key)?.as_ref().ok().copied()
}

fn is_failure(cache: &GitCache<Read, u32>, project: &ProjectId) -> bool {
    matches!(
        cache.get(project, &Read::Status),
        Some(Err(AppError::NotFound(message))) if message == FAILURE
    )
}

fn requested(requests: &[Request<Read>]) -> HashSet<Read> {
    requests.iter().map(|request| request.key.clone()).collect()
}

fn answer(cache: &mut GitCache<Read, u32>, requests: &[Request<Read>], value: u32, now: Instant) {
    for request in requests {
        cache.begin(request);
        cache.finish(request, Ok(value), now);
    }
}

fn settled(project: &ProjectId, now: Instant) -> GitCache<Read, u32> {
    let mut cache = GitCache::default();
    observe(&mut cache, project, &reads(), now);
    let requests = cache.requests(now);
    assert_eq!(requested(&requests), reads().into_iter().collect());
    answer(&mut cache, &requests, 1, now);
    observe(&mut cache, project, &reads(), now + FRAME);
    assert!(cache.requests(now + FRAME).is_empty());
    cache
}

#[test]
fn 변경_기록은_git_상태_ref_재스캔을_저장소_범위로_fs_변경은_경로로_프로젝트별로_모은다() {
    let changes = Changes::default();
    let [repository, worktree, rescan, other] = [(); 4].map(|_| ProjectId::new());
    changes.record(&AppEvent::GitStatusChanged {
        project_id: repository.clone(),
    });
    changes.record(&AppEvent::GitRefsChanged {
        project_id: repository.clone(),
    });
    for path in [CHANGED, UNCHANGED] {
        changes.record(&AppEvent::FsChanged {
            project_id: worktree.clone(),
            change: FsChange {
                kind: FsChangeKind::Modified,
                paths: vec![path.into()],
                from_app: false,
            },
        });
    }
    changes.record(&AppEvent::FsRescanRequired {
        project_id: rescan.clone(),
    });
    changes.record(&AppEvent::ProjectClosed {
        project_id: other.clone(),
    });
    let taken = changes.take();
    assert_eq!(
        taken,
        HashMap::from([
            (repository, Invalidation::repository()),
            (
                worktree,
                Invalidation {
                    repository: false,
                    paths: [CHANGED, UNCHANGED].map(String::from).into(),
                },
            ),
            (rescan, Invalidation::repository()),
        ])
    );
    assert!(changes.take().is_empty());
}

#[test]
fn 저장소_변경은_불변_범위를_빼고_다시_읽고_fs_변경은_status와_그_경로의_읽기만_다시_읽는다() {
    let project = ProjectId::new();
    let now = Instant::now();
    let mut cache = settled(&project, now);

    cache.invalidate(
        &project,
        &Invalidation {
            repository: false,
            paths: HashSet::from([CHANGED.to_owned()]),
        },
    );
    observe(&mut cache, &project, &reads(), now);
    let requests = cache.requests(now);
    assert_eq!(
        requested(&requests),
        HashSet::from([Read::Status, Read::Diff(CHANGED.into())])
    );
    answer(&mut cache, &requests, 2, now);
    assert_eq!(
        value(&cache, &project, &Read::Diff(UNCHANGED.into())),
        Some(1)
    );

    cache.invalidate(&project, &Invalidation::repository());
    observe(&mut cache, &project, &reads(), now);
    let requests = cache.requests(now);
    assert_eq!(
        requested(&requests),
        reads()
            .into_iter()
            .filter(|read| *read != Read::CommitFiles)
            .collect()
    );
    answer(&mut cache, &requests, 3, now);
    assert_eq!(value(&cache, &project, &Read::CommitFiles), Some(1));
    assert_eq!(value(&cache, &project, &Read::Log), Some(3));

    cache.invalidate(&ProjectId::new(), &Invalidation::repository());
    observe(&mut cache, &project, &reads(), now);
    assert!(cache.requests(now).is_empty());
}

#[test]
fn 읽는_중에_무효화된_응답은_값을_쓰되_한번_더_읽고_오류도_값으로_남긴다() {
    let project = ProjectId::new();
    let now = Instant::now();
    let mut cache = GitCache::<Read, u32>::default();
    observe(&mut cache, &project, &[Read::Status], now);
    let [first] = cache.requests(now).try_into().unwrap();
    cache.begin(&first);
    cache.invalidate(&project, &Invalidation::repository());
    observe(&mut cache, &project, &[Read::Status], now);
    assert!(cache.requests(now).is_empty(), "one read at a time");
    cache.finish(&first, Ok(1), now);
    assert_eq!(value(&cache, &project, &Read::Status), Some(1));

    observe(&mut cache, &project, &[Read::Status], now);
    let [second] = cache.requests(now).try_into().unwrap();
    assert_eq!(second.generation, first.generation + 1);
    cache.invalidate(&project, &Invalidation::repository());
    cache.begin(&second);
    cache.finish(&second, Err(AppError::NotFound(FAILURE.into())), now);
    assert!(is_failure(&cache, &project));
    observe(&mut cache, &project, &[Read::Status], now);
    let [third] = cache.requests(now).try_into().unwrap();
    cache.begin(&third);
    assert!(
        is_failure(&cache, &project),
        "the previous answer stays until the next one lands"
    );
}

#[test]
fn 다시_붙은_관찰자는_stale_시간이_지난_값만_다시_읽고_10분_관찰이_없으면_버린다() {
    let project = ProjectId::new();
    let start = Instant::now();
    let keys = [
        Read::Status,
        Read::CommitFiles,
        Read::CurrentUser,
        Read::BlameOverlay,
    ];
    let mut cache = GitCache::<Read, u32>::default();
    observe(&mut cache, &project, &keys, start);
    let requests = cache.requests(start);
    answer(&mut cache, &requests, 1, start);

    observe(&mut cache, &project, &keys, start + FRAME);
    assert!(cache.requests(start + FRAME).is_empty());
    assert!(cache.requests(start + FRAME + FRAME).is_empty());

    let early = start + BEFORE_STALE;
    observe(&mut cache, &project, &keys, early);
    let requests = cache.requests(early);
    assert_eq!(requested(&requests), HashSet::from([Read::BlameOverlay]));
    answer(&mut cache, &requests, 2, early);

    let watched = start + AFTER_STALE;
    observe(&mut cache, &project, &keys, watched);
    assert!(
        cache.requests(watched).is_empty(),
        "an observer that never detached does not refetch by age"
    );
    assert!(cache.requests(watched + FRAME).is_empty());

    let late = watched + FRAME + FRAME;
    observe(&mut cache, &project, &keys, late);
    assert_eq!(
        requested(&cache.requests(late)),
        HashSet::from([Read::Status, Read::BlameOverlay])
    );

    let forgotten = late + DISCARD_AFTER;
    assert!(cache.requests(forgotten).is_empty());
    assert!(cache.get(&project, &Read::CommitFiles).is_none());
}

#[test]
fn 한_프레임_요청은_상한까지만_내고_시작하지_못한_요청은_다음_프레임에_다시_낸다() {
    let project = ProjectId::new();
    let now = Instant::now();
    let keys: Vec<Read> = (0..MAX_REQUESTS_PER_FRAME + EXTRA_REQUESTS)
        .map(|index| Read::Diff(format!("/synthetic/project/{index}.txt")))
        .collect();
    let mut cache = GitCache::<Read, u32>::default();
    observe(&mut cache, &project, &keys, now);
    let refused = cache.requests(now);
    assert_eq!(refused.len(), MAX_REQUESTS_PER_FRAME);

    observe(&mut cache, &project, &keys, now);
    let first = cache.requests(now);
    assert_eq!(first.len(), MAX_REQUESTS_PER_FRAME);
    for request in &first {
        cache.begin(request);
    }
    observe(&mut cache, &project, &keys, now);
    let rest = cache.requests(now);
    assert_eq!(rest.len(), EXTRA_REQUESTS);
    assert_eq!(
        requested(&first)
            .union(&requested(&rest))
            .cloned()
            .collect::<HashSet<_>>(),
        keys.into_iter().collect()
    );
}

#[test]
fn 연결을_다시_맺으면_읽는_중이던_항목을_다시_읽고_닫힌_프로젝트는_지운다() {
    let project = ProjectId::new();
    let now = Instant::now();
    let mut cache = settled(&project, now);
    cache.invalidate(&project, &Invalidation::repository());
    observe(&mut cache, &project, &[Read::Log], now);
    let [request] = cache.requests(now).try_into().unwrap();
    cache.begin(&request);
    cache.cancel_fetches();
    observe(&mut cache, &project, &[Read::Log], now);
    assert_eq!(requested(&cache.requests(now)), HashSet::from([Read::Log]));

    cache.retain(&HashSet::from([project.clone()]));
    assert_eq!(value(&cache, &project, &Read::Log), Some(1));
    cache.retain(&HashSet::new());
    assert!(cache.get(&project, &Read::Log).is_none());
}

#[test]
fn 커밋_diff는_불변이고_파일_이력은_저장소_변경을_따르며_파일_읽기는_그_경로만_따른다() {
    let path = "/synthetic/a.txt".to_owned();
    let commit = super::Key::CommitDiff {
        rev: "rev".into(),
        parent: None,
        path: path.clone(),
        before_path: path.clone(),
    };
    let history = super::Key::FileLog { path: path.clone() };
    let file = super::Key::File { path: path.clone() };
    let changed = HashSet::from([path]);
    assert!(!commit.is_mutable());
    assert!(!commit.follows_worktree(&changed));
    assert_eq!(commit.stale_after(), None);
    assert!(history.is_mutable());
    assert!(!history.follows_worktree(&changed));
    assert!(!file.is_mutable());
    assert!(file.follows_worktree(&changed));
    assert!(!file.follows_worktree(&HashSet::from(["/synthetic/b.txt".to_owned()])));
    assert_eq!(history.stale_after(), file.stale_after());
}

#[test]
fn 편집기_gutter는_그_경로_fs_변경을_따르고_blame_overlay는_붙을_때마다_다시_읽고_사용자는_시간으로_낡지_않는다()
 {
    let path = "/synthetic/a.txt".to_owned();
    let changed = HashSet::from([path.clone()]);
    let other = HashSet::from(["/synthetic/b.txt".to_owned()]);
    let gutter = super::Key::Gutter { path: path.clone() };
    let line = super::Key::BlameLine {
        path: path.clone(),
        line: 1,
    };
    let overlay = super::Key::BlameOverlay {
        path: path.clone(),
        lines: 3,
    };
    let sides = super::Key::ConflictSides { path: path.clone() };
    let user = super::Key::CurrentUser;
    for key in [&gutter, &line, &overlay, &sides, &user] {
        assert!(key.is_mutable(), "{key:?}");
    }
    assert!(gutter.follows_worktree(&changed));
    assert!(!gutter.follows_worktree(&other));
    for key in [&line, &overlay, &sides, &user] {
        assert!(!key.follows_worktree(&changed), "{key:?}");
    }
    assert_eq!(overlay.stale_after(), Some(Duration::ZERO));
    assert_eq!(user.stale_after(), None);
    assert_eq!(gutter.stale_after(), line.stale_after());

    let project = ProjectId::new();
    let now = Instant::now();
    let mut cache = GitCache::<super::Key, u32>::default();
    cache.observe(&project, overlay.clone(), now);
    let requests = cache.requests(now);
    assert_eq!(requests.len(), 1);
    cache.begin(&requests[0]);
    cache.finish(&requests[0], Ok(1), now);
    cache.observe(&project, overlay.clone(), now);
    assert!(
        cache.requests(now).is_empty(),
        "켜진 동안은 다시 읽지 않습니다"
    );
    assert!(cache.requests(now).is_empty());
    cache.observe(&project, overlay.clone(), now);
    assert_eq!(
        cache.requests(now).len(),
        1,
        "다시 켜면 곧바로 다시 읽습니다"
    );

    cache.observe(&project, sides.clone(), now);
    let requests = cache.requests(now);
    cache.begin(&requests[0]);
    cache.finish(&requests[0], Ok(2), now);
    cache.observe(&project, sides.clone(), now);
    assert!(cache.requests(now).is_empty());
    cache.refetch(&project, &sides);
    cache.observe(&project, sides.clone(), now);
    let requests = cache.requests(now);
    assert_eq!(requests.len(), 1, "누를 때마다 다시 읽습니다");
    cache.begin(&requests[0]);
    cache.refetch(&project, &sides);
    cache.finish(&requests[0], Ok(3), now);
    cache.observe(&project, sides.clone(), now);
    assert_eq!(
        cache.requests(now).len(),
        1,
        "읽는 중에 다시 누르면 한 번 더 읽습니다"
    );
}

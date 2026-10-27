use super::*;
use taide_model::file::{FsChange, FsChangeKind};

#[test]
fn 변경_기록은_git_상태_ref_파일_변경만_프로젝트별로_모은다() {
    let alpha = ProjectId::new();
    let beta = ProjectId::new();
    let changes = Changes::default();
    changes.record(&AppEvent::GitStatusChanged {
        project_id: alpha.clone(),
    });
    changes.record(&AppEvent::GitRefsChanged {
        project_id: alpha.clone(),
    });
    changes.record(&AppEvent::FsChanged {
        project_id: beta.clone(),
        change: FsChange {
            kind: FsChangeKind::Modified,
            paths: vec!["/synthetic/beta/file.txt".into()],
            from_app: false,
        },
    });
    changes.record(&AppEvent::FsRescanRequired {
        project_id: ProjectId::new(),
    });

    assert_eq!(changes.take(), HashSet::from([alpha, beta]));
    assert!(changes.take().is_empty());
}

#[test]
fn 보이는_프로젝트만_한번씩_읽고_무효화되면_다시_읽는다() {
    let shown = ProjectId::new();
    let hidden = ProjectId::new();
    let mut branches = Branches::default();

    assert_eq!(
        branches.requests([shown.clone(), shown.clone()]),
        vec![shown.clone()]
    );
    assert!(branches.requests([shown.clone()]).is_empty());
    branches.invalidate([shown.clone(), hidden.clone()]);
    assert!(branches.requests([shown.clone()]).is_empty());
    branches.finish(&shown, Some("synthetic/main".into()));
    assert_eq!(branches.branch(&shown), Some("synthetic/main"));
    assert_eq!(branches.requests([shown.clone()]), vec![shown.clone()]);
    branches.finish(&shown, None);
    assert_eq!(branches.branch(&shown), None);
    assert!(branches.branch(&hidden).is_none());

    branches.invalidate([shown.clone()]);
    assert_eq!(branches.requests([shown.clone()]), vec![shown.clone()]);
    branches.retry(&shown);
    assert_eq!(branches.requests([shown.clone()]), vec![shown.clone()]);
    branches.finish(&shown, Some("synthetic/other".into()));

    branches.retain(&HashSet::new());
    assert!(branches.branch(&shown).is_none());
    branches.finish(&shown, Some("synthetic/late".into()));
    assert!(branches.branch(&shown).is_none());
}

#[test]
fn 연결을_다시_맺으면_읽는_중이던_branch만_다시_요청한다() {
    let fetching = ProjectId::new();
    let settled = ProjectId::new();
    let mut branches = Branches::default();
    assert_eq!(
        branches.requests([fetching.clone(), settled.clone()]),
        vec![fetching.clone(), settled.clone()]
    );
    branches.finish(&settled, Some("synthetic/main".into()));
    assert!(
        branches
            .requests([fetching.clone(), settled.clone()])
            .is_empty()
    );

    branches.cancel_fetches();

    assert_eq!(
        branches.requests([fetching.clone(), settled.clone()]),
        vec![fetching]
    );
    assert_eq!(branches.branch(&settled), Some("synthetic/main"));
}

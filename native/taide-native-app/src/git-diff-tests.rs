use taide_model::git::{GitChangeKind, StatusRow};
use taide_model::ids::TabId;

use super::{Tabs, is_stageable};

fn conflicted_row() -> StatusRow {
    StatusRow {
        path: "src/a.ts".into(),
        abs_path: "/repo/src/a.ts".into(),
        orig_path: None,
        orig_abs_path: None,
        staged: None,
        unstaged: Some(GitChangeKind::Conflicted),
        is_conflicted: true,
    }
}

#[test]
fn 충돌_파일과_파일_비교는_hunk_stage를_제공하지_않고_나머지는_제공한다() {
    let conflicted = [conflicted_row()];
    assert!(!is_stageable(None, "/repo/src/a.ts", &conflicted));
    assert!(!is_stageable(None, "src/a.ts", &conflicted));
    assert!(is_stageable(None, "/repo/src/b.ts", &conflicted));
    let modified = [StatusRow {
        unstaged: Some(GitChangeKind::Modified),
        is_conflicted: false,
        ..conflicted_row()
    }];
    assert!(is_stageable(None, "/repo/src/a.ts", &modified));
    assert!(!is_stageable(Some("/repo/src/b.ts"), "/repo/src/a.ts", &[]));
}

#[test]
fn 탭_보기_방식은_처음_좌우이고_토글되며_그리지_않은_탭은_초기화된다() {
    let mut tabs = Tabs::default();
    let (first, second) = (TabId::new(), TabId::new());
    tabs.shown(first.clone());
    tabs.shown(second.clone());
    assert!(tabs.is_side_by_side(&first));
    tabs.toggle(&first);
    assert!(!tabs.is_side_by_side(&first));
    let shown = tabs.finish_frame();
    assert!(shown.contains(&first) && shown.contains(&second));

    tabs.shown(second.clone());
    let shown = tabs.finish_frame();
    assert!(!shown.contains(&first));
    tabs.shown(first.clone());
    assert!(
        tabs.is_side_by_side(&first),
        "다시 마운트된 탭은 좌우 보기입니다"
    );
}

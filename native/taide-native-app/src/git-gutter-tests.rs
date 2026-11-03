use taide_model::git::{GutterHunk, HunkKind, StatusRow};
use taide_model::ids::{PaneId, ProjectId, TabId};

use super::{Editors, hunk_at, is_conflicted, selected_lines};

fn row(abs_path: &str, is_conflicted: bool) -> StatusRow {
    StatusRow {
        path: "a.txt".into(),
        abs_path: abs_path.into(),
        staged: None,
        unstaged: None,
        is_conflicted,
        orig_path: None,
        orig_abs_path: None,
    }
}

#[test]
fn 선택_줄_범위는_여러_줄이_다음_줄_첫_칸에서_끝나면_그_줄을_뺀다() {
    assert_eq!(selected_lines(2, 4, true), (2, 3));
    assert_eq!(selected_lines(2, 4, false), (2, 4));
    assert_eq!(selected_lines(3, 3, true), (3, 3), "한 줄 선택은 그대로");
}

#[test]
fn hunk_찾기와_충돌_판정은_원본_규칙을_따른다() {
    let hunks = [
        GutterHunk {
            kind: HunkKind::Modified,
            start: 2,
            end: 3,
        },
        GutterHunk {
            kind: HunkKind::Deleted,
            start: 6,
            end: 6,
        },
    ];
    assert_eq!(hunk_at(&hunks, 3), Some((2, 3)));
    assert_eq!(hunk_at(&hunks, 6), Some((6, 6)));
    assert_eq!(hunk_at(&hunks, 4), None);
    let rows = [row("/r/a.txt", true), row("/r/b.txt", false)];
    assert!(is_conflicted(&rows, "/r/a.txt"));
    assert!(!is_conflicted(&rows, "/r/b.txt"));
    assert!(!is_conflicted(&rows, "a.txt"), "절대 경로로만 비교합니다");
}

#[test]
fn 창의_편집기_상태는_다른_파일을_보이면_새로_시작하고_그리지_않으면_지운다() {
    let (pane, project, tab) = (PaneId::new(), ProjectId::new(), TabId::new());
    let mut editors = Editors::default();
    let editor = editors.shown(&pane, &project, &tab, "/r/a.txt");
    editor.pending_hunk = Some((1, 2));
    editor.blame.toggle_overlay(3);
    editors.finish_frame();
    let editor = editors.shown(&pane, &project, &tab, "/r/a.txt");
    assert_eq!(editor.pending_hunk, Some((1, 2)));
    let other = TabId::new();
    let editor = editors.shown(&pane, &project, &other, "/r/b.txt");
    assert_eq!(editor.pending_hunk, None);
    assert_eq!(
        editor.blame.overlay(),
        None,
        "경로가 바뀌면 overlay가 꺼집니다"
    );
    assert!(editors.of_tab(&pane, &tab).is_none());
    assert!(editors.of_tab(&pane, &other).is_some());
    editors.finish_frame();
    editors.finish_frame();
    assert!(editors.is_empty());
}

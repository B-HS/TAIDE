use std::collections::HashSet;
use std::time::Instant;

use taide_model::app_event::AppEvent;
use taide_model::ids::{ProjectId, TabId};
use taide_native_ui::diff_view::Hunk;

use super::{Event, Inbox, RECOMPUTE_DELAY, State};

const HUNK: Hunk = Hunk {
    old_start: 1,
    old_lines: 1,
    new_start: 1,
    new_lines: 1,
};

#[test]
fn 수신함은_diff_요청과_요청_id가_있는_close_tab만_기록한다() {
    let inbox = Inbox::default();
    let project = ProjectId::new();
    inbox.record(&AppEvent::IdeDiffRequested {
        request_id: "r1".into(),
        project_id: project.clone(),
        old_path: "/old".into(),
        new_path: "/new".into(),
        new_contents: "proposed".into(),
        tab_name: "title".into(),
    });
    inbox.record(&AppEvent::IdeCloseTabRequested {
        tab_name: "title".into(),
        request_id: None,
    });
    inbox.record(&AppEvent::IdeCloseTabRequested {
        tab_name: "title".into(),
        request_id: Some("r1".into()),
    });
    assert_eq!(
        inbox.take(),
        vec![
            Event::Requested {
                request_id: "r1".into(),
                project,
                path: "/new".into(),
                contents: "proposed".into(),
                title: "title".into(),
            },
            Event::Closed("r1".into()),
        ]
    );
    assert!(inbox.take().is_empty());
}

#[test]
fn 첫_줄_diff는_바로_이후_변경은_200ms_뒤에_한_번만_계산한다() {
    let mut state = State::default();
    let tab = TabId::new();
    let now = Instant::now();
    state.insert("r".into(), "text".into());
    state.shown("r", tab.clone(), (1, 1), now);
    assert_eq!(state.due(now), vec![(tab.clone(), (1, 1))]);
    state.shown("r", tab.clone(), (1, 1), now);
    assert!(
        state.due(now).is_empty(),
        "진행 중인 같은 revision 은 다시 내지 않음"
    );
    state.finish(&tab, (1, 1), Ok(vec![HUNK]), Some((1, 1)));
    assert_eq!(state.hunks(&tab), [HUNK]);

    state.shown("r", tab.clone(), (1, 2), now);
    state.shown("r", tab.clone(), (1, 3), now + RECOMPUTE_DELAY / 2);
    assert!(
        state.due(now + RECOMPUTE_DELAY).is_empty(),
        "마지막 변경부터 다시 200ms"
    );
    assert_eq!(
        state.next_due(now + RECOMPUTE_DELAY),
        Some(RECOMPUTE_DELAY / 2)
    );
    assert_eq!(
        state.due(now + RECOMPUTE_DELAY + RECOMPUTE_DELAY / 2),
        vec![(tab.clone(), (1, 3))]
    );
    state.finish(&tab, (1, 3), Ok(Vec::new()), Some((1, 4)));
    assert_eq!(
        state.hunks(&tab),
        [HUNK],
        "현재 revision 과 다른 응답은 버림"
    );
}

#[test]
fn 그려진_요청이_어느_창에도_없으면_대기_목록에서_지운다() {
    let mut state = State::default();
    let (shown, hidden) = (TabId::new(), TabId::new());
    state.insert("shown".into(), "a".into());
    state.insert("waiting".into(), "b".into());
    state.shown("shown", shown.clone(), (1, 1), Instant::now());
    state.retain(&HashSet::new(), &HashSet::from([shown.clone()]));
    assert_eq!(
        state.contents("shown"),
        "a",
        "다른 창으로 옮겨도 탭이 있으면 유지"
    );
    state.retain(&HashSet::from([shown.clone()]), &HashSet::new());
    assert_eq!(state.contents("shown"), "");
    assert_eq!(state.contents("waiting"), "b", "탭이 열리기 전 요청은 유지");
    state.retain(&HashSet::from([hidden]), &HashSet::new());
    assert!(state.hunks(&shown).is_empty());
    assert!(state.begin_resolve("waiting"));
    assert!(!state.begin_resolve("waiting"), "진행 중이면 무시");
    state.finish_resolve("waiting");
    assert!(!state.is_resolving("waiting"));
}

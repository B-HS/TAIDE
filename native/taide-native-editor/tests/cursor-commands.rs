use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::cursor_commands::{CursorCommand, run_cursor_command};
use taide_native_editor::document::EditorError;
use taide_native_editor::editing::{compose_text, type_text};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::language_configuration::UntokenizedLines;
use taide_native_editor::line_commands::LineCommandContext;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{ScrollPosition, Selection, SelectionSet, ViewId, ViewKey};

const DOCUMENT_LIMIT: usize = 2;
const VIEW_LIMIT: usize = 4;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 64 * 1024;
const TAB_SIZE: u32 = 4;
const CONTEXT: LineCommandContext<'static> = LineCommandContext {
    indent: IndentOptions {
        tab_size: TAB_SIZE,
        insert_spaces: true,
    },
    language: None,
    syntax: &UntokenizedLines,
    compare: None,
    transforms: None,
    word_rules: None,
};

fn fixture(text: &str, ranges: &[(usize, usize)], read_only: bool) -> (EditorStore, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_file(
            "/synthetic/cursor-commands.txt".into(),
            OpenedFile {
                path: "/synthetic/cursor-commands.txt".into(),
                content: text.into(),
                language_id: "plaintext".into(),
                byte_size: text.len().try_into().unwrap(),
                line_count: text.lines().count().try_into().unwrap(),
                tier: FileSizeTier::Normal,
                read_only,
                encoding_lossy: false,
                modified_ms: 1.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "main".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    set_ranges(&mut store, view, ranges);
    (store, view)
}

fn set_ranges(store: &mut EditorStore, view: ViewId, ranges: &[(usize, usize)]) {
    let current = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: ranges
                    .iter()
                    .map(|(anchor, head)| Selection {
                        anchor: *anchor,
                        head: *head,
                    })
                    .collect(),
            },
            current.scroll,
            current.folds,
        )
        .unwrap();
}

fn ranges(store: &EditorStore, view: ViewId) -> Vec<(usize, usize)> {
    store
        .views()
        .get(view)
        .unwrap()
        .selection
        .selections
        .iter()
        .map(|s| (s.anchor, s.head))
        .collect()
}

fn run(store: &mut EditorStore, view: ViewId, command: CursorCommand) {
    run_cursor_command(store, view, command, CONTEXT).unwrap();
}

#[test]
fn 줄_끝_커서는_빈_선택과_다음_줄_첫_열을_제외하고_마지막_선택_열을_유지한다() {
    let (mut store, view) = fixture("abcd\r\nef\r\nghij", &[(1, 10), (13, 13)], false);
    run(&mut store, view, CursorCommand::LineEnds);
    assert_eq!(ranges(&store, view), [(4, 4), (8, 8)]);
    set_ranges(&mut store, view, &[(1, 12)]);
    run(&mut store, view, CursorCommand::LineEnds);
    assert_eq!(ranges(&store, view), [(4, 4), (8, 8), (12, 12)]);
}

#[test]
fn 위아래_커서는_선택을_옮기고_겹침을_병합하며_읽기전용에서는_추가하지_않는다() {
    let (mut store, view) = fixture("abcd\nef\nghij", &[(1, 3)], false);
    run(&mut store, view, CursorCommand::AddBelow);
    assert_eq!(ranges(&store, view), [(1, 3), (6, 7)]);
    run(&mut store, view, CursorCommand::AddAbove);
    assert_eq!(ranges(&store, view).len(), 2);
    let (mut store, view) = fixture("ab\ncd", &[(1, 1)], true);
    run(&mut store, view, CursorCommand::AddBelow);
    assert_eq!(ranges(&store, view), [(1, 1)]);
    run(&mut store, view, CursorCommand::ExpandLine);
    assert_eq!(ranges(&store, view), [(0, 3)]);
}

#[test]
fn 줄_선택은_역방향도_정방향으로_바꾸고_반복하면_다음_줄까지_확장한다() {
    let (mut store, view) = fixture("ab\ncd\nef", &[(4, 1)], false);
    run(&mut store, view, CursorCommand::ExpandLine);
    assert_eq!(ranges(&store, view), [(0, 6)]);
    run(&mut store, view, CursorCommand::ExpandLine);
    assert_eq!(ranges(&store, view), [(0, 8)]);
}

#[test]
fn 상하_전체_커서는_첫_선택의_열과_순서를_사용하고_포커스는_회전한다() {
    let (mut store, view) = fixture("abcd\nx\nefgh", &[(6, 6)], false);
    run(&mut store, view, CursorCommand::ToBottom);
    assert_eq!(ranges(&store, view), [(6, 6), (8, 8)]);
    run(&mut store, view, CursorCommand::FocusNext);
    assert_eq!(ranges(&store, view), [(8, 8), (6, 6)]);
    run(&mut store, view, CursorCommand::FocusPrevious);
    assert_eq!(ranges(&store, view), [(6, 6), (8, 8)]);
    run(&mut store, view, CursorCommand::ToTop);
    assert_eq!(ranges(&store, view), [(6, 6), (1, 1)]);
}

#[test]
fn 커서_이력은_선택과_스크롤을_복원하고_문서_수정_뒤_초기화한다() {
    let (mut store, view) = fixture("abc", &[(0, 0)], false);
    let current = store.views().get(view).unwrap().clone();
    let scroll = ScrollPosition { x: 10.0, y: 20.0 };
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: 2, head: 2 }],
            },
            scroll.clone(),
            current.folds,
        )
        .unwrap();
    run(&mut store, view, CursorCommand::Undo);
    assert_eq!(ranges(&store, view), [(0, 0)]);
    assert_eq!(
        store.views().get(view).unwrap().scroll,
        ScrollPosition::default()
    );
    run(&mut store, view, CursorCommand::Redo);
    assert_eq!(ranges(&store, view), [(2, 2)]);
    assert_eq!(store.views().get(view).unwrap().scroll, scroll);
    type_text(&mut store, view, "X").unwrap();
    run(&mut store, view, CursorCommand::Undo);
    assert_eq!(ranges(&store, view), [(3, 3)]);
}

#[test]
fn 선택한_텍스트_좌우_이동은_한_문자를_교환하고_실행취소_한_단위로_복원한다() {
    let (mut store, view) = fixture("abcd", &[(1, 3)], false);
    run(&mut store, view, CursorCommand::MoveCaretLeft);
    let document = store.views().get(view).unwrap().document;
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "bcad"
    );
    assert_eq!(ranges(&store, view), [(0, 2)]);
    assert!(store.undo(document).unwrap());
    assert_eq!(ranges(&store, view), [(1, 3)]);
}

#[test]
fn 일치_추가는_빈_커서를_먼저_단어로_확장하고_건너뛰기와_순환을_유지한다() {
    let (mut store, view) = fixture("cat concatenate cat CAT cat", &[(1, 1)], false);
    run(&mut store, view, CursorCommand::AddNextMatch);
    assert_eq!(ranges(&store, view), [(0, 3)]);
    run(&mut store, view, CursorCommand::AddNextMatch);
    assert_eq!(ranges(&store, view), [(0, 3), (16, 19)]);
    run(&mut store, view, CursorCommand::MoveNextMatch);
    assert_eq!(ranges(&store, view), [(0, 3), (24, 27)]);
    run(&mut store, view, CursorCommand::AddNextMatch);
    assert_eq!(ranges(&store, view).len(), 2);
    run(&mut store, view, CursorCommand::AddPreviousMatch);
    assert_eq!(ranges(&store, view), [(0, 3), (24, 27), (16, 19)]);
}

#[test]
fn 전체_일치는_기존_선택을_주커서로_유지하고_선택_시작_검색은_대소문자를_구분하지_않는다() {
    let (mut store, view) = fixture("cat CAT cats", &[(5, 5)], false);
    run(&mut store, view, CursorCommand::SelectMatches);
    assert_eq!(ranges(&store, view), [(4, 7)]);
    set_ranges(&mut store, view, &[(0, 0)]);
    set_ranges(&mut store, view, &[(4, 7)]);
    run(&mut store, view, CursorCommand::SelectMatches);
    assert_eq!(ranges(&store, view), [(4, 7), (0, 3), (8, 11)]);
}

#[test]
fn 스마트_선택은_부분단어_단어_줄_전체로_확장하고_축소_외부_이동을_구분한다() {
    let (mut store, view) = fixture("  fooBar  \nnext", &[(6, 6)], false);
    run(&mut store, view, CursorCommand::Expand);
    assert_eq!(ranges(&store, view), [(5, 8)]);
    run(&mut store, view, CursorCommand::Expand);
    assert_eq!(ranges(&store, view), [(2, 8)]);
    run(&mut store, view, CursorCommand::Expand);
    assert_eq!(ranges(&store, view), [(0, 10)]);
    run(&mut store, view, CursorCommand::Shrink);
    assert_eq!(ranges(&store, view), [(2, 8)]);
    set_ranges(&mut store, view, &[(12, 12)]);
    run(&mut store, view, CursorCommand::Shrink);
    assert_eq!(ranges(&store, view), [(12, 12)]);
}

#[test]
fn 커서_반복_추가는_짧은_줄에서_잃은_표시열을_다음_긴_줄에서_복원한다() {
    let (mut store, view) = fixture("abcd\nx\nabcd", &[(3, 3)], false);
    run(&mut store, view, CursorCommand::AddBelow);
    run(&mut store, view, CursorCommand::AddBelow);
    assert_eq!(ranges(&store, view), [(3, 3), (6, 6), (10, 10)]);
}

#[test]
fn 선택_앵커는_편집과_문서_실행취소를_따라가고_다른_뷰로_공유하지_않는다() {
    let (mut store, view) = fixture("abcdef", &[(4, 4)], false);
    run(&mut store, view, CursorCommand::SetAnchor);
    assert_eq!(store.selection_anchor(view), Some(4));
    set_ranges(&mut store, view, &[(0, 0)]);
    type_text(&mut store, view, "前").unwrap();
    assert_eq!(store.selection_anchor(view), Some(7));
    let document = store.views().get(view).unwrap().document;
    assert!(store.undo(document).unwrap());
    assert_eq!(store.selection_anchor(view), Some(4));
    assert!(store.redo(document).unwrap());
    assert_eq!(store.selection_anchor(view), Some(7));
    let other = store
        .attach_view(
            ViewKey {
                window: "other".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    assert_eq!(store.selection_anchor(other), None);
    set_ranges(&mut store, view, &[(7, 7)]);
    type_text(&mut store, view, "X").unwrap();
    assert_eq!(store.selection_anchor(view), Some(8));
}

#[test]
fn 겹친_선택은_마지막_방향과_주커서를_보존하고_접한_비어있지_않은_선택은_병합하지_않는다() {
    for (input, expected) in [
        (vec![(0, 3), (4, 2)], vec![(4, 0)]),
        (vec![(0, 2), (2, 4)], vec![(0, 2), (2, 4)]),
        (vec![(0, 2), (2, 2)], vec![(0, 2)]),
        (vec![(4, 4), (2, 4), (3, 1)], vec![(4, 1)]),
    ] {
        let selection = SelectionSet {
            primary: input.len() - 1,
            selections: input
                .into_iter()
                .map(|(anchor, head)| Selection { anchor, head })
                .collect(),
        }
        .normalized();
        assert_eq!(
            selection
                .selections
                .iter()
                .map(|selection| (selection.anchor, selection.head))
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(selection.primary, expected.len() - 1);
    }
    let (mut store, view) = fixture("abcd", &[(0, 3), (4, 2)], false);
    type_text(&mut store, view, "한").unwrap();
    assert_eq!(
        store
            .documents()
            .snapshot(store.views().get(view).unwrap().document)
            .unwrap()
            .rope
            .to_string(),
        "한"
    );
    assert_eq!(ranges(&store, view), [("한".len(), "한".len())]);
}

#[test]
fn 다중_조합의_잘못된_utf8_범위는_상태를_바꾸지_않고_거절한다() {
    let (mut store, view) = fixture("한글", &[(0, 0), (3, 3)], false);
    for bytes in [1..2, 4..3, 0..7] {
        assert_eq!(
            compose_text(&mut store, view, bytes, "X"),
            Err(EditorError::InvalidBoundary)
        );
        assert_eq!(
            store
                .documents()
                .snapshot(store.views().get(view).unwrap().document)
                .unwrap()
                .rope
                .to_string(),
            "한글"
        );
        assert_eq!(ranges(&store, view), [(0, 0), (3, 3)]);
    }
}

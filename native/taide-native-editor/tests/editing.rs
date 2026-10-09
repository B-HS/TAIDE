use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{Edit, EditorError, UndoGroup};
use taide_native_editor::editing::{
    ClipboardText, Motion, clipboard_text, compose_text, cut, delete_backward, delete_forward,
    delete_to_line_start, delete_word, grapheme_boundary, insert_line_break, line_content_range,
    move_selection, paste, replace_selections, reveal_position, select_all, selected_text,
    type_text,
};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
use taide_native_editor::view::{Composition, Selection, SelectionSet, ViewId, ViewKey};

const DOCUMENT_LIMIT: usize = 2;
const VIEW_LIMIT: usize = 4;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 64 * 1024;
const CHUNK_PREFIX: usize = 4095;
const REVEAL_LINE: f64 = 2.0;
const TAB_SIZE: u32 = 4;
const WIDE_TAB_SIZE: u32 = 8;
const PAGE_LINES: isize = 10;
const SPACES: IndentOptions = IndentOptions {
    tab_size: TAB_SIZE,
    insert_spaces: true,
};

fn crlf_fixture(text: &str) -> (EditorStore, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_file(
            "/synthetic/crlf.txt".into(),
            OpenedFile {
                path: "/synthetic/crlf.txt".into(),
                content: text.into(),
                language_id: "plaintext".into(),
                byte_size: text.len().try_into().unwrap(),
                line_count: text.lines().count().try_into().unwrap(),
                tier: FileSizeTier::Normal,
                read_only: false,
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
    (store, view)
}

fn content(store: &EditorStore, view: ViewId) -> String {
    store
        .documents()
        .snapshot(store.views().get(view).unwrap().document)
        .unwrap()
        .rope
        .to_string()
}

fn select(store: &mut EditorStore, view: ViewId, ranges: &[(usize, usize)]) {
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

fn selections(store: &EditorStore, view: ViewId) -> Vec<(usize, usize)> {
    store
        .views()
        .get(view)
        .unwrap()
        .selection
        .selections
        .iter()
        .map(|selection| (selection.anchor, selection.head))
        .collect()
}

fn undo(store: &mut EditorStore, view: ViewId) -> String {
    let document = store.views().get(view).unwrap().document;
    assert!(store.undo(document).unwrap());
    content(store, view)
}

fn type_each(store: &mut EditorStore, view: ViewId, text: &str) {
    for character in text.chars() {
        assert!(type_text(store, view, character.encode_utf8(&mut [0; 4])).unwrap());
    }
}

fn vertical(lines: isize, tab_size: u32) -> Motion {
    Motion::Vertical { lines, tab_size }
}

#[test]
fn 빈_선택_복사는_현재_줄과_문서_개행을_담고_다중_커서는_monaco_순서로_결합한다() {
    let text = "alpha\nbeta\ngamma";
    let (mut store, view) = fixture(text);
    select(&mut store, view, &[(7, 7)]);
    assert_eq!(
        clipboard_text(&store, view, false).unwrap(),
        Some(ClipboardText {
            text: "beta\n".into(),
            from_empty_selection: true,
            multicursor: None,
        })
    );
    assert_eq!(
        clipboard_text(&store, view, true).unwrap().unwrap().text,
        "beta\r\n"
    );
    select(&mut store, view, &[(text.len(), text.len())]);
    assert_eq!(
        clipboard_text(&store, view, false).unwrap().unwrap().text,
        "gamma\n"
    );
    select(&mut store, view, &[(12, 12), (0, 0), (2, 2)]);
    assert_eq!(
        clipboard_text(&store, view, false).unwrap(),
        Some(ClipboardText {
            text: "alpha\n\ngamma\n".into(),
            from_empty_selection: false,
            multicursor: Some(vec!["alpha\n".into(), "gamma\n".into()]),
        })
    );
    select(&mut store, view, &[(8, 8), (2, 0)]);
    assert_eq!(
        clipboard_text(&store, view, false).unwrap().unwrap().text,
        "al\nbeta\n"
    );
    select(&mut store, view, &[(0, 2), (3, 3)]);
    assert_eq!(
        clipboard_text(&store, view, false).unwrap(),
        Some(ClipboardText {
            text: "al".into(),
            from_empty_selection: false,
            multicursor: None,
        })
    );
    select(&mut store, view, &[(0, 8)]);
    assert_eq!(
        clipboard_text(&store, view, true).unwrap().unwrap().text,
        "alpha\r\nbe"
    );
    let (mut crlf, crlf_view) = crlf_fixture("one\r\ntwo");
    select(&mut crlf, crlf_view, &[(1, 1)]);
    assert_eq!(
        clipboard_text(&crlf, crlf_view, false)
            .unwrap()
            .unwrap()
            .text,
        "one\r\n"
    );
    let (empty, empty_view) = fixture("");
    assert_eq!(
        clipboard_text(&empty, empty_view, false)
            .unwrap()
            .unwrap()
            .text,
        "\n"
    );
}

#[test]
fn 빈_선택_잘라내기는_줄_전체를_지우고_마지막_줄은_앞_개행을_함께_지운다() {
    let (mut store, view) = fixture("alpha\nbeta\ngamma");
    select(&mut store, view, &[(7, 7)]);
    assert!(cut(&mut store, view).unwrap());
    assert_eq!(content(&store, view), "alpha\ngamma");
    assert_eq!(selections(&store, view), [(6, 6)]);
    select(&mut store, view, &[(9, 9)]);
    assert!(cut(&mut store, view).unwrap());
    assert_eq!(content(&store, view), "alpha");
    assert_eq!(selections(&store, view), [(5, 5)]);
    select(&mut store, view, &[(2, 2), (4, 4)]);
    assert!(cut(&mut store, view).unwrap());
    assert_eq!(content(&store, view), "");
    assert_eq!(selections(&store, view), [(0, 0)]);
    assert!(!cut(&mut store, view).unwrap());
    assert_eq!(undo(&mut store, view), "alpha");
    assert_eq!(undo(&mut store, view), "alpha\ngamma");
    assert_eq!(undo(&mut store, view), "alpha\nbeta\ngamma");
    let (mut tail, tail_view) = fixture("a\nb\nc");
    select(&mut tail, tail_view, &[(4, 4), (2, 2)]);
    assert!(cut(&mut tail, tail_view).unwrap());
    assert_eq!(content(&tail, tail_view), "a\n");
    assert_eq!(selections(&tail, tail_view), [(2, 2)]);
    let (mut selected, selected_view) = fixture("abcdef");
    select(&mut selected, selected_view, &[(4, 1)]);
    assert!(cut(&mut selected, selected_view).unwrap());
    assert_eq!(content(&selected, selected_view), "aef");
    assert_eq!(selections(&selected, selected_view), [(1, 1)]);
}

#[test]
fn 줄_복사_붙여넣기는_현재_줄_위에_삽입하고_일반_붙여넣기는_문서_개행과_다중_커서_분배를_따른다() {
    let (mut store, view) = fixture("abc\ndef");
    select(&mut store, view, &[(5, 5)]);
    let copied = clipboard_text(&store, view, false).unwrap().unwrap();
    assert!(paste(&mut store, view, &copied.text, Some(&copied)).unwrap());
    assert_eq!(content(&store, view), "abc\ndef\ndef");
    assert_eq!(selections(&store, view), [(9, 9)]);
    assert!(paste(&mut store, view, &copied.text, None).unwrap());
    assert_eq!(content(&store, view), "abc\ndef\nddef\nef");
    assert_eq!(selections(&store, view), [(13, 13)]);
    assert_eq!(undo(&mut store, view), "abc\ndef\ndef");
    assert_eq!(undo(&mut store, view), "abc\ndef");
    select(&mut store, view, &[(0, 3)]);
    assert!(paste(&mut store, view, &copied.text, Some(&copied)).unwrap());
    assert_eq!(content(&store, view), "def\n\ndef");
    assert!(!paste(&mut store, view, "", None).unwrap());
    let (mut crlf, crlf_view) = crlf_fixture("one\r\ntwo");
    select(&mut crlf, crlf_view, &[(3, 3)]);
    assert!(paste(&mut crlf, crlf_view, "\nx\ry", None).unwrap());
    assert_eq!(content(&crlf, crlf_view), "one\r\nx\r\ny\r\ntwo");
    select(&mut crlf, crlf_view, &[(1, 1)]);
    let line = clipboard_text(&crlf, crlf_view, false).unwrap().unwrap();
    assert!(paste(&mut crlf, crlf_view, "one\n", Some(&line)).unwrap());
    assert_eq!(content(&crlf, crlf_view), "one\r\none\r\nx\r\ny\r\ntwo");
    assert_eq!(selections(&crlf, crlf_view), [(6, 6)]);
    let (mut multiple, multiple_view) = fixture("1\n2");
    select(&mut multiple, multiple_view, &[(3, 3), (1, 1)]);
    assert!(paste(&mut multiple, multiple_view, "a\nb\n", None).unwrap());
    assert_eq!(content(&multiple, multiple_view), "1a\n2b");
    assert_eq!(selections(&multiple, multiple_view), [(5, 5), (2, 2)]);
    assert!(paste(&mut multiple, multiple_view, "z", None).unwrap());
    assert_eq!(content(&multiple, multiple_view), "1az\n2bz");
    let pieces = ClipboardText {
        text: "X\n\nY\n".into(),
        from_empty_selection: false,
        multicursor: Some(vec!["X\n".into(), "Y\n".into()]),
    };
    assert!(paste(&mut multiple, multiple_view, &pieces.text, Some(&pieces)).unwrap());
    assert_eq!(content(&multiple, multiple_view), "1azX\n\n2bzY\n");
}

#[test]
fn 연속_입력과_삭제는_monaco_undo_stop_규칙으로_묶인다() {
    let (mut store, view) = fixture("");
    type_each(&mut store, view, "abc def ghi");
    assert_eq!(undo(&mut store, view), "abc def");
    assert_eq!(undo(&mut store, view), "abc");
    assert_eq!(undo(&mut store, view), "");
    type_each(&mut store, view, "a  b");
    assert_eq!(undo(&mut store, view), "a  ");
    assert_eq!(undo(&mut store, view), "a");
    assert_eq!(undo(&mut store, view), "");
    type_each(&mut store, view, "xy");
    assert!(delete_backward(&mut store, view, SPACES).unwrap());
    assert_eq!(content(&store, view), "x");
    assert_eq!(undo(&mut store, view), "xy");
    assert_eq!(undo(&mut store, view), "");
    type_each(&mut store, view, "a");
    assert!(insert_line_break(&mut store, view, SPACES).unwrap());
    type_each(&mut store, view, "b");
    assert_eq!(content(&store, view), "a\nb");
    assert_eq!(undo(&mut store, view), "a");
    assert_eq!(undo(&mut store, view), "");
    type_each(&mut store, view, "a");
    move_selection(&mut store, view, Motion::Left, false).unwrap();
    move_selection(&mut store, view, Motion::Right, false).unwrap();
    type_each(&mut store, view, "b");
    assert_eq!(undo(&mut store, view), "a");
    assert_eq!(undo(&mut store, view), "");
    type_each(&mut store, view, "a");
    assert!(paste(&mut store, view, "P", None).unwrap());
    type_each(&mut store, view, "b");
    assert_eq!(undo(&mut store, view), "aP");
    assert_eq!(undo(&mut store, view), "a");
    assert_eq!(undo(&mut store, view), "");
    type_each(&mut store, view, "한");
    let caret = selections(&store, view)[0].1;
    assert!(compose_text(&mut store, view, caret..caret, "글").unwrap());
    assert_eq!(content(&store, view), "한글");
    assert_eq!(undo(&mut store, view), "");
    let (mut deleted, deleted_view) = fixture("abcdef\ngh");
    select(&mut deleted, deleted_view, &[(9, 9)]);
    for expected in ["abcdef\ng", "abcdef\n", "abcdef", "abcde"] {
        assert!(delete_backward(&mut deleted, deleted_view, SPACES).unwrap());
        assert_eq!(content(&deleted, deleted_view), expected);
    }
    assert_eq!(undo(&mut deleted, deleted_view), "abcdef\n");
    assert_eq!(undo(&mut deleted, deleted_view), "abcdef\ngh");
    select(&mut deleted, deleted_view, &[(0, 0)]);
    assert!(!delete_backward(&mut deleted, deleted_view, SPACES).unwrap());
    assert!(delete_forward(&mut deleted, deleted_view).unwrap());
    assert!(delete_forward(&mut deleted, deleted_view).unwrap());
    assert_eq!(content(&deleted, deleted_view), "cdef\ngh");
    assert_eq!(undo(&mut deleted, deleted_view), "abcdef\ngh");
}

#[test]
fn 다른_출처의_편집은_입력_run을_끊고_공유_view의_undo_선택을_보존한다() {
    let (mut store, view) = fixture("");
    let document = store.views().get(view).unwrap().document;
    type_each(&mut store, view, "a");
    let revision = store.documents().snapshot(document).unwrap().revision;
    store
        .apply(
            document,
            Transaction {
                revision,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: "Z".into(),
                }],
                group: UndoGroup(revision),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
    type_each(&mut store, view, "bc");
    assert_eq!(content(&store, view), "Zabc");
    assert_eq!(undo(&mut store, view), "Za");
    assert_eq!(selections(&store, view), [(2, 2)]);
    assert_eq!(undo(&mut store, view), "a");
    assert_eq!(undo(&mut store, view), "");
}

#[test]
fn 단어_이동과_삭제는_monaco_word_separators와_공백_휴리스틱을_따른다() {
    let text = "foo.bar  baz_qux\n  next";
    let (mut store, view) = fixture(text);
    for expected in [3, 7, 16, 23, 23] {
        move_selection(&mut store, view, Motion::WordRight, false).unwrap();
        assert_eq!(selections(&store, view), [(expected, expected)]);
    }
    for expected in [19, 17, 9, 4, 0, 0] {
        move_selection(&mut store, view, Motion::WordLeft, false).unwrap();
        assert_eq!(selections(&store, view), [(expected, expected)]);
    }
    select(&mut store, view, &[(4, 4)]);
    move_selection(&mut store, view, Motion::WordRight, true).unwrap();
    assert_eq!(selections(&store, view), [(4, 7)]);
    move_selection(&mut store, view, Motion::WordLeft, false).unwrap();
    assert_eq!(selections(&store, view), [(4, 4)]);
    for (caret, forward, expected) in [
        (9, false, "foo.barbaz_qux\n  next"),
        (7, false, "foo.  baz_qux\n  next"),
        (7, true, "foo.barbaz_qux\n  next"),
        (3, true, "foobar  baz_qux\n  next"),
        (17, false, "foo.bar  baz_qux  next"),
        (16, true, "foo.bar  baz_quxnext"),
        (0, false, text),
        (text.len(), true, text),
    ] {
        let (mut store, view) = fixture(text);
        select(&mut store, view, &[(caret, caret)]);
        assert_eq!(
            delete_word(&mut store, view, forward).unwrap(),
            expected != text
        );
        assert_eq!(content(&store, view), expected);
    }
    select(&mut store, view, &[(1, 3)]);
    assert!(delete_word(&mut store, view, false).unwrap());
    assert_eq!(content(&store, view), "f.bar  baz_qux\n  next");
    assert_eq!(undo(&mut store, view), text);
    for (range, expected, caret) in [
        ((9, 9), "baz_qux\n  next", 0),
        ((17, 17), "foo.bar  baz_qux  next", 16),
        ((7, 4), "  baz_qux\n  next", 0),
        ((0, 0), text, 0),
    ] {
        let (mut store, view) = fixture(text);
        select(&mut store, view, &[range]);
        assert_eq!(
            delete_to_line_start(&mut store, view).unwrap(),
            expected != text
        );
        assert_eq!(content(&store, view), expected);
        assert_eq!(selections(&store, view), [(caret, caret)]);
    }
}

#[test]
fn 줄_처음_이동은_첫_비공백과_열1을_번갈아_가고_선택_확장은_anchor를_유지한다() {
    let (mut store, view) = fixture("  indented\nplain\n   ");
    select(&mut store, view, &[(6, 6)]);
    for expected in [2, 0, 2] {
        move_selection(&mut store, view, Motion::LineStart, false).unwrap();
        assert_eq!(selections(&store, view), [(expected, expected)]);
    }
    move_selection(&mut store, view, Motion::LineEnd, true).unwrap();
    assert_eq!(selections(&store, view), [(2, 10)]);
    select(&mut store, view, &[(13, 13)]);
    for _ in 0..2 {
        move_selection(&mut store, view, Motion::LineStart, false).unwrap();
        assert_eq!(selections(&store, view), [(11, 11)]);
    }
    select(&mut store, view, &[(19, 19)]);
    move_selection(&mut store, view, Motion::LineStart, true).unwrap();
    assert_eq!(selections(&store, view), [(19, 17)]);
    move_selection(&mut store, view, Motion::DocumentStart, false).unwrap();
    assert_eq!(selections(&store, view), [(0, 0)]);
    move_selection(&mut store, view, Motion::DocumentEnd, true).unwrap();
    assert_eq!(selections(&store, view), [(0, 20)]);
}

#[test]
fn 세로_이동은_goal_column을_보존하고_가로_이동과_편집에서_초기화한다() {
    let (mut store, view) = fixture("abcdef\nab\nabcdef");
    let document = store.views().get(view).unwrap().document;
    select(&mut store, view, &[(5, 5)]);
    for expected in [9, 15] {
        move_selection(&mut store, view, vertical(1, TAB_SIZE), false).unwrap();
        assert_eq!(selections(&store, view), [(expected, expected)]);
    }
    assert!(store.views().get(view).unwrap().goal_columns.is_some());
    move_selection(&mut store, view, Motion::Left, false).unwrap();
    assert!(store.views().get(view).unwrap().goal_columns.is_none());
    for (lines, expected) in [(-1, 9), (-1, 4), (-1, 0), (1, 9), (1, 14)] {
        move_selection(&mut store, view, vertical(lines, TAB_SIZE), false).unwrap();
        assert_eq!(selections(&store, view), [(expected, expected)]);
    }
    select(&mut store, view, &[(12, 12)]);
    move_selection(&mut store, view, vertical(1, TAB_SIZE), false).unwrap();
    assert_eq!(selections(&store, view), [(16, 16)]);
    move_selection(&mut store, view, vertical(-1, TAB_SIZE), false).unwrap();
    assert_eq!(selections(&store, view), [(9, 9)]);
    move_selection(&mut store, view, vertical(-1, TAB_SIZE), false).unwrap();
    assert_eq!(selections(&store, view), [(2, 2)]);
    move_selection(&mut store, view, vertical(PAGE_LINES, TAB_SIZE), true).unwrap();
    assert_eq!(selections(&store, view), [(2, 16)]);
    move_selection(&mut store, view, vertical(-PAGE_LINES, TAB_SIZE), false).unwrap();
    assert_eq!(selections(&store, view), [(0, 0)]);
    select(&mut store, view, &[(1, 5)]);
    move_selection(&mut store, view, vertical(1, TAB_SIZE), false).unwrap();
    assert_eq!(selections(&store, view), [(9, 9)]);
    select(&mut store, view, &[(15, 11)]);
    move_selection(&mut store, view, vertical(-1, TAB_SIZE), false).unwrap();
    assert_eq!(selections(&store, view), [(8, 8)]);
    select(&mut store, view, &[(5, 5)]);
    move_selection(&mut store, view, vertical(1, TAB_SIZE), false).unwrap();
    assert!(type_text(&mut store, view, "x").unwrap());
    move_selection(&mut store, view, vertical(1, TAB_SIZE), false).unwrap();
    assert_eq!(selections(&store, view), [(14, 14)]);
    assert!(store.undo(document).unwrap());
    let (mut tabs, tabs_view) = fixture("\tx\n        y");
    for (tab_size, expected) in [(TAB_SIZE, 8), (WIDE_TAB_SIZE, 12)] {
        select(&mut tabs, tabs_view, &[(2, 2)]);
        move_selection(&mut tabs, tabs_view, vertical(1, tab_size), false).unwrap();
        assert_eq!(selections(&tabs, tabs_view), [(expected, expected)]);
    }
    let (mut wide, wide_view) = fixture("한글ab\nabcdef");
    select(&mut wide, wide_view, &[(6, 6)]);
    move_selection(&mut wide, wide_view, vertical(1, TAB_SIZE), false).unwrap();
    assert_eq!(selections(&wide, wide_view), [(13, 13)]);
    move_selection(&mut wide, wide_view, vertical(-1, TAB_SIZE), false).unwrap();
    assert_eq!(selections(&wide, wide_view), [(6, 6)]);
}

#[test]
fn reveal은_monaco_utf16_보정을_단일_view에만_적용하고_문서_undo를_변경하지_않는다() {
    let text = "zero\r\n𐐀e\u{301}漢\r\nlast";
    let (mut store, view) = fixture(text);
    let current = store.views().get(view).unwrap().clone();
    let second = store
        .attach_view(
            ViewKey {
                window: "auxiliary".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            current.document,
        )
        .unwrap();
    let initial = store.documents().snapshot(current.document).unwrap();
    let composition = Composition {
        revision: initial.revision,
        replace: 0..1,
        preedit: "漢".into(),
    };
    store
        .set_composition(second, Some(composition.clone()))
        .unwrap();
    let second_state = store.views().get(second).unwrap().clone();
    let start = "zero\r\n".len();
    let end = "zero\r\n𐐀e\u{301}漢".len();
    for (line, column, expected) in [
        (REVEAL_LINE, 1.0, start),
        (REVEAL_LINE, 2.0, start),
        (REVEAL_LINE, 3.0, start + "𐐀".len()),
        (REVEAL_LINE, 4.0, start + "𐐀e".len()),
        (REVEAL_LINE, 5.0, start + "𐐀e\u{301}".len()),
        (REVEAL_LINE, 6.0, end),
        (REVEAL_LINE, f64::INFINITY, end),
        (REVEAL_LINE, f64::NEG_INFINITY, start),
        (REVEAL_LINE, f64::NAN, start),
        (2.9, 3.9, start + "𐐀".len()),
        (0.0, f64::INFINITY, 0),
        (f64::NEG_INFINITY, 9.0, 0),
        (f64::NAN, f64::NAN, 0),
        (f64::INFINITY, 1.0, text.len()),
    ] {
        store
            .set_composition(view, Some(composition.clone()))
            .unwrap();
        assert_eq!(
            reveal_position(&mut store, view, line, column).unwrap(),
            expected
        );
        let state = store.views().get(view).unwrap();
        assert_eq!(
            state.selection,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: expected,
                    head: expected
                }]
            }
        );
        assert!(state.composition.is_none());
        let after = store.documents().snapshot(current.document).unwrap();
        assert_eq!(after.revision, initial.revision);
        assert_eq!(after.dirty, initial.dirty);
        assert_eq!(after.rope, initial.rope);
    }
    let other = store.views().get(second).unwrap();
    assert_eq!(other.selection, second_state.selection);
    assert_eq!(other.composition, second_state.composition);
    assert!(!store.undo(current.document).unwrap());
}

fn fixture(text: &str) -> (EditorStore, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let tab = TabId::new();
    let document = store
        .open_untitled(tab.clone(), text, "plaintext".into())
        .unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "main".into(),
                pane: PaneId::new(),
                tab,
            },
            document,
        )
        .unwrap();
    (store, view)
}

#[test]
fn 실제_rope_편집은_grapheme_crlf_선택과_undo를_보존한다() {
    let text = format!("{}e\u{301}🇰🇷\r\n한글\n", "x".repeat(CHUNK_PREFIX));
    let (mut store, view) = fixture(&text);
    let document = store.views().get(view).unwrap().document;
    let snapshot = store.documents().snapshot(document).unwrap();
    let combined_end = CHUNK_PREFIX + "e\u{301}".len();
    assert_eq!(
        grapheme_boundary(&snapshot, CHUNK_PREFIX, true),
        Ok(combined_end)
    );
    assert_eq!(
        grapheme_boundary(&snapshot, combined_end, false),
        Ok(CHUNK_PREFIX)
    );
    let flag_end = combined_end + "🇰🇷".len();
    assert_eq!(
        grapheme_boundary(&snapshot, flag_end, false),
        Ok(combined_end)
    );
    assert_eq!(
        grapheme_boundary(&snapshot, flag_end, true),
        Ok(flag_end + "\r\n".len())
    );
    let last = snapshot.rope.len_lines() - 1;
    assert_eq!(line_content_range(&snapshot, last), text.len()..text.len());
    move_selection(&mut store, view, Motion::DocumentEnd, false).unwrap();
    assert!(replace_selections(&mut store, view, "", Some(false)).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        text.trim_end_matches('\n')
    );
    assert!(store.undo(document).unwrap());
    select_all(&mut store, view).unwrap();
    assert_eq!(selected_text(&store, view).unwrap(), text);
    assert!(replace_selections(&mut store, view, "日本語", None).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "日本語"
    );
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        text
    );
}

#[test]
fn 다중_선택_입력과_경계_거절은_문서에_원자적으로_적용된다() {
    let (mut store, view) = fixture("abc def");
    let current = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 1,
                selections: vec![
                    Selection { anchor: 0, head: 3 },
                    Selection { anchor: 4, head: 7 },
                ],
            },
            current.scroll,
            current.folds,
        )
        .unwrap();
    replace_selections(&mut store, view, "한", None).unwrap();
    let snapshot = store.documents().snapshot(current.document).unwrap();
    assert_eq!(snapshot.rope.to_string(), "한 한");
    let selections = &store.views().get(view).unwrap().selection;
    assert_eq!(selections.primary, 1);
    assert_eq!(selections.selections[1].head, "한 한".len());
    assert_eq!(
        grapheme_boundary(&snapshot, 1, true),
        Err(EditorError::InvalidBoundary)
    );
    move_selection(&mut store, view, Motion::Left, true).unwrap();
    assert_eq!(selected_text(&store, view).unwrap(), "한\n한");
    assert!(store.undo(current.document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(current.document)
            .unwrap()
            .rope
            .to_string(),
        "abc def"
    );
    let current = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 1,
                selections: vec![
                    Selection { anchor: 0, head: 1 },
                    Selection { anchor: 1, head: 2 },
                ],
            },
            current.scroll,
            current.folds,
        )
        .unwrap();
    replace_selections(&mut store, view, "x", None).unwrap();
    assert_eq!(
        store
            .documents()
            .snapshot(current.document)
            .unwrap()
            .rope
            .to_string(),
        "xxc def"
    );
    assert_eq!(store.views().get(view).unwrap().selection.primary, 1);
}

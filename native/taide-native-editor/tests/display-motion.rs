use std::sync::Arc;

use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::display_map::{DisplayMap, WrapSettings, WrappingIndent};
use taide_native_editor::document::EditorError;
use taide_native_editor::editing::{Motion, move_selection, move_selection_displayed, type_text};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{Selection, SelectionSet, ViewId, ViewKey, WrapAffinities};

const DOCUMENT_LIMIT: usize = 1;
const VIEW_LIMIT: usize = 1;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 4096;
const TAB_SIZE: u32 = 4;
const FULL_WIDTH_COLUMNS: f64 = 2.0;
const WRAP_COLUMN: u32 = 10;
const PAGE_ROWS: isize = 3;
const WORDS: &str = "alpha beta gamma delta\nxy\n";
const INDENTED: &str = "    aaaa aaaa";
const UNWRAPPED: &str = "\tab\tcd 한글\nxy\n\n    indented line\nlast";
const DOWN: Motion = Motion::Vertical {
    lines: 1,
    tab_size: TAB_SIZE,
};
const UP: Motion = Motion::Vertical {
    lines: -1,
    tab_size: TAB_SIZE,
};

fn fixture(text: &str) -> (EditorStore, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_file(
            "/synthetic/motion.txt".into(),
            OpenedFile {
                path: "/synthetic/motion.txt".into(),
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

fn wrapped(store: &EditorStore, view: ViewId) -> DisplayMap {
    let document = store
        .documents()
        .snapshot(store.views().get(view).unwrap().document)
        .unwrap();
    DisplayMap::build(
        &document,
        Some(WrapSettings {
            wrap_column: WRAP_COLUMN,
            tab_size: TAB_SIZE,
            full_width_columns: FULL_WIDTH_COLUMNS,
            wrapping_indent: WrappingIndent::Same,
        }),
    )
}

fn place(store: &mut EditorStore, view: ViewId, carets: &[(usize, usize)]) {
    let current = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: carets
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
    store.set_wrap_affinities(view, None).unwrap();
}

fn step(store: &mut EditorStore, view: ViewId, motion: Motion, extend: bool) {
    let display = wrapped(store, view);
    move_selection_displayed(store, view, motion, extend, &display).unwrap();
}

fn carets(store: &EditorStore, view: ViewId) -> Vec<(usize, usize, bool)> {
    let state = store.views().get(view).unwrap();
    let revision = store.documents().snapshot(state.document).unwrap().revision;
    state
        .selection
        .selections
        .iter()
        .enumerate()
        .map(|(index, selection)| {
            (
                selection.anchor,
                selection.head,
                state.head_at_row_end(index, revision),
            )
        })
        .collect()
}

fn caret(store: &EditorStore, view: ViewId) -> (usize, usize, bool) {
    carets(store, view)[0]
}

#[test]
fn wrap_세로_이동은_표시_줄을_옮기고_goal_column을_표시_줄_기준으로_보존한다() {
    let (mut store, view) = fixture(WORDS);
    place(&mut store, view, &[(2, 2)]);
    for (motion, head) in [
        (DOWN, 8),
        (DOWN, 13),
        (DOWN, 19),
        (DOWN, 25),
        (DOWN, 26),
        (UP, 25),
        (UP, 19),
        (UP, 13),
        (UP, 8),
        (UP, 2),
        (UP, 0),
        (DOWN, 8),
    ] {
        step(&mut store, view, motion, false);
        assert_eq!(caret(&store, view), (head, head, false));
    }
    place(&mut store, view, &[(2, 2)]);
    for (lines, head) in [(PAGE_ROWS, 19), (-PAGE_ROWS, 2)] {
        step(
            &mut store,
            view,
            Motion::Vertical {
                lines,
                tab_size: TAB_SIZE,
            },
            false,
        );
        assert_eq!(caret(&store, view), (head, head, false));
    }
    step(&mut store, view, DOWN, true);
    assert_eq!(caret(&store, view), (2, 8, false));
}

#[test]
fn wrap_end는_표시_줄_끝의_윗_줄에_머물고_다시_누르면_문서_줄_끝으로_간다() {
    let (mut store, view) = fixture(WORDS);
    for (start, first, second) in [(2, (6, true), (22, false)), (8, (11, true), (22, false))] {
        place(&mut store, view, &[(start, start)]);
        for (head, at_row_end) in [first, second] {
            step(&mut store, view, Motion::LineEnd, false);
            assert_eq!(caret(&store, view), (head, head, at_row_end));
        }
    }
    place(&mut store, view, &[(19, 19)]);
    step(&mut store, view, Motion::LineEnd, false);
    assert_eq!(caret(&store, view), (22, 22, false));
    place(&mut store, view, &[(2, 2), (8, 8)]);
    step(&mut store, view, Motion::LineEnd, true);
    assert_eq!(carets(&store, view), [(2, 6, true), (8, 11, true)]);
}

#[test]
fn wrap_home은_이어지는_표시_줄의_시작으로_가고_거기서는_문서_줄_기준으로_움직인다() {
    let (mut store, view) = fixture(WORDS);
    place(&mut store, view, &[(8, 8)]);
    for head in [6, 0] {
        step(&mut store, view, Motion::LineStart, false);
        assert_eq!(caret(&store, view), (head, head, false));
    }
    place(&mut store, view, &[(2, 2)]);
    step(&mut store, view, Motion::LineEnd, false);
    assert_eq!(caret(&store, view), (6, 6, true));
    step(&mut store, view, Motion::LineStart, false);
    assert_eq!(caret(&store, view), (0, 0, false));
    let (mut indented, indented_view) = fixture(INDENTED);
    place(&mut indented, indented_view, &[(11, 11)]);
    for head in [9, 4, 0, 4] {
        step(&mut indented, indented_view, Motion::LineStart, false);
        assert_eq!(caret(&indented, indented_view), (head, head, false));
    }
}

#[test]
fn wrap_경계에서_오른쪽_이동은_윗_줄_끝에_머물고_왼쪽_이동은_아랫_줄_시작에_놓인다() {
    let (mut store, view) = fixture(WORDS);
    place(&mut store, view, &[(5, 5)]);
    for (motion, head, at_row_end) in [
        (Motion::Right, 6, true),
        (Motion::Right, 7, false),
        (Motion::Left, 6, false),
        (Motion::Left, 5, false),
    ] {
        step(&mut store, view, motion, false);
        assert_eq!(caret(&store, view), (head, head, at_row_end));
    }
    for (motion, expected) in [(Motion::Right, (6, 6, true)), (Motion::Left, (2, 2, false))] {
        place(&mut store, view, &[(2, 2)]);
        step(&mut store, view, Motion::LineEnd, true);
        assert_eq!(caret(&store, view), (2, 6, true));
        step(&mut store, view, motion, false);
        assert_eq!(caret(&store, view), expected);
    }
}

#[test]
fn wrap_세로_이동은_캐럿이_머문_표시_줄에서_출발하고_짧은_줄의_끝에서도_윗_줄에_머문다() {
    let (mut store, view) = fixture(WORDS);
    place(&mut store, view, &[(2, 2)]);
    step(&mut store, view, Motion::LineEnd, false);
    for (motion, head) in [(DOWN, 11), (UP, 6)] {
        step(&mut store, view, motion, false);
        assert_eq!(caret(&store, view), (head, head, true));
    }
    place(&mut store, view, &[(6, 6)]);
    step(&mut store, view, DOWN, false);
    assert_eq!(caret(&store, view), (11, 11, false));
}

#[test]
fn wrap_이어지는_줄의_들여쓰기는_세로_이동의_열에_포함되고_들여쓰기_안으로는_들어가지_않는다() {
    let (mut store, view) = fixture(INDENTED);
    for (start, down) in [(6, 11), (2, 9)] {
        place(&mut store, view, &[(start, start)]);
        for (motion, head) in [(DOWN, down), (UP, start)] {
            step(&mut store, view, motion, false);
            assert_eq!(caret(&store, view), (head, head, false));
        }
    }
    place(&mut store, view, &[(2, 2)]);
    for (head, at_row_end) in [(9, true), (13, false)] {
        step(&mut store, view, Motion::LineEnd, false);
        assert_eq!(caret(&store, view), (head, head, at_row_end));
    }
}

#[test]
fn wrap_표시_줄_선호는_선택이나_문서가_바뀌면_사라지고_선택_수와_맞아야_한다() {
    let (mut store, view) = fixture(WORDS);
    place(&mut store, view, &[(2, 2)]);
    step(&mut store, view, Motion::LineEnd, false);
    let kept = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            kept.selection.clone(),
            kept.scroll.clone(),
            Vec::new(),
        )
        .unwrap();
    assert_eq!(caret(&store, view), (6, 6, true));
    let revision = store.documents().snapshot(kept.document).unwrap().revision;
    assert_eq!(
        store.set_wrap_affinities(
            view,
            Some(WrapAffinities {
                revision,
                heads_at_row_end: vec![true, true],
            })
        ),
        Err(EditorError::InvalidBoundary)
    );
    assert!(type_text(&mut store, view, "x").unwrap());
    assert_eq!(caret(&store, view), (7, 7, false));
    store.undo(kept.document).unwrap();
    assert_eq!(caret(&store, view), (6, 6, false));
    step(&mut store, view, Motion::LineEnd, false);
    assert_eq!(caret(&store, view), (11, 11, true));
    for head in [12, 11] {
        store
            .set_view_state(
                view,
                SelectionSet {
                    primary: 0,
                    selections: vec![Selection { anchor: head, head }],
                },
                kept.scroll.clone(),
                Vec::new(),
            )
            .unwrap();
        assert_eq!(caret(&store, view), (head, head, false));
        assert_eq!(store.views().get(view).unwrap().wrap_affinities, None);
    }
}

#[test]
fn 표시_맵은_view에_보관되고_문서와_revision이_다른_표시_맵은_이동에_쓰이지_않는다() {
    let (mut store, view) = fixture(WORDS);
    let display = wrapped(&store, view);
    store
        .set_display(view, Some(Arc::new(display.clone())))
        .unwrap();
    assert_eq!(
        store.views().get(view).unwrap().display.as_deref(),
        Some(&display)
    );
    assert_eq!(store.take_display(view).unwrap().as_deref(), Some(&display));
    assert_eq!(store.views().get(view).unwrap().display, None);
    place(&mut store, view, &[(2, 2)]);
    assert!(type_text(&mut store, view, "x").unwrap());
    move_selection_displayed(&mut store, view, Motion::LineEnd, false, &display).unwrap();
    assert_eq!(caret(&store, view), (23, 23, false));
    store
        .set_display(view, Some(Arc::new(display.clone())))
        .unwrap();
    assert_eq!(
        store.detach_view(view).unwrap().display.as_deref(),
        Some(&display)
    );
    assert_eq!(store.set_display(view, None), Err(EditorError::NotFound));
}

#[test]
fn identity_표시_맵의_이동은_표시_맵_없는_이동과_같은_선택과_goal_column을_만든다() {
    let (mut plain, plain_view) = fixture(UNWRAPPED);
    let (mut displayed, displayed_view) = fixture(UNWRAPPED);
    for (motion, extend) in [
        (DOWN, false),
        (Motion::LineEnd, false),
        (DOWN, false),
        (DOWN, true),
        (UP, false),
        (Motion::LineStart, false),
        (Motion::Right, false),
        (Motion::Right, true),
        (Motion::Left, false),
        (
            Motion::Vertical {
                lines: PAGE_ROWS,
                tab_size: TAB_SIZE,
            },
            false,
        ),
        (Motion::LineStart, true),
        (Motion::LineStart, false),
        (Motion::DocumentEnd, false),
        (UP, false),
        (Motion::WordLeft, false),
        (Motion::LineEnd, true),
        (DOWN, false),
        (DOWN, false),
    ] {
        move_selection(&mut plain, plain_view, motion, extend).unwrap();
        let document = displayed
            .documents()
            .snapshot(displayed.views().get(displayed_view).unwrap().document)
            .unwrap();
        let identity = DisplayMap::identity(document.rope.len_lines(), document.revision);
        move_selection_displayed(&mut displayed, displayed_view, motion, extend, &identity)
            .unwrap();
        let expected = plain.views().get(plain_view).unwrap();
        let actual = displayed.views().get(displayed_view).unwrap();
        assert_eq!(actual.selection, expected.selection);
        assert_eq!(
            actual
                .goal_columns
                .as_ref()
                .map(|goal| &goal.leftover_visible_columns),
            expected
                .goal_columns
                .as_ref()
                .map(|goal| &goal.leftover_visible_columns)
        );
        assert_eq!(actual.wrap_affinities, None);
    }
}

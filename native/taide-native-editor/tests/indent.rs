use taide_model::file::{EditorConfigIndentStyle, EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::editing::{delete_backward, insert_line_break, outdent, tab};
use taide_native_editor::indent::{IndentOptions, resolve};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{Selection, SelectionSet, ViewId, ViewKey};

const BASE_SIZE: u32 = 4;
const SPACE_SIZE: u32 = 2;
const TAB_WIDTH: u32 = 8;
const DOCUMENT_LIMIT: usize = 1;
const VIEW_LIMIT: usize = 1;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 1024;
const SPACES: IndentOptions = IndentOptions {
    tab_size: BASE_SIZE,
    insert_spaces: true,
};
const TABS: IndentOptions = IndentOptions {
    tab_size: BASE_SIZE,
    insert_spaces: false,
};

type Case = (
    &'static str,
    (usize, usize),
    IndentOptions,
    &'static str,
    (usize, usize),
);

fn fixture(text: &str, range: (usize, usize)) -> (EditorStore, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_file(
            "/synthetic/indent.txt".into(),
            OpenedFile {
                path: "/synthetic/indent.txt".into(),
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
    let current = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: range.0,
                    head: range.1,
                }],
            },
            current.scroll,
            current.folds,
        )
        .unwrap();
    (store, view)
}

fn assert_cases(cases: &[Case], edit: impl Fn(&mut EditorStore, ViewId, IndentOptions) -> bool) {
    for (text, range, indent, expected, selection) in cases {
        let (mut store, view) = fixture(text, *range);
        let document = store.views().get(view).unwrap().document;
        assert_eq!(
            edit(&mut store, view, *indent),
            text != expected,
            "{text:?}"
        );
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            *expected,
            "{text:?}"
        );
        let after = store.views().get(view).unwrap().selection.selections[0];
        assert_eq!((after.anchor, after.head), *selection, "{text:?}");
        if text != expected {
            assert!(store.undo(document).unwrap());
            assert_eq!(
                store
                    .documents()
                    .snapshot(document)
                    .unwrap()
                    .rope
                    .to_string(),
                *text
            );
            assert!(!store.undo(document).unwrap());
        }
    }
}

#[test]
fn 줄바꿈은_문서_개행과_커서_앞_선행_공백을_유지하고_설정_방식으로_정규화한다() {
    assert_cases(
        &[
            ("    foo", (7, 7), SPACES, "    foo\n    ", (12, 12)),
            ("    foo", (2, 2), SPACES, "  \n    foo", (5, 5)),
            (
                "\tfoo\r\nbar",
                (4, 4),
                SPACES,
                "\tfoo\r\n    \r\nbar",
                (10, 10),
            ),
            ("one\r\ntwo", (8, 8), SPACES, "one\r\ntwo\r\n", (10, 10)),
            ("      x", (7, 7), TABS, "      x\n\t  ", (11, 11)),
            ("  ab", (2, 4), SPACES, "  \n  ", (5, 5)),
            ("\t\tx", (3, 3), TABS, "\t\tx\n\t\t", (6, 6)),
        ],
        |store, view, indent| insert_line_break(store, view, indent).unwrap(),
    );
}

#[test]
fn tab은_선택_줄을_들여쓰고_빈_선택은_다음_탭_정지로_이동한다() {
    assert_cases(
        &[
            ("a\n\nb\nc", (0, 5), SPACES, "    a\n\n    b\nc", (0, 13)),
            ("ab", (0, 2), SPACES, "    ab", (0, 6)),
            ("abcd", (1, 3), SPACES, "a   d", (4, 4)),
            ("ab", (2, 2), SPACES, "ab  ", (4, 4)),
            (" ", (1, 1), SPACES, "    ", (4, 4)),
            ("    ", (4, 4), SPACES, "        ", (8, 8)),
            ("", (0, 0), SPACES, "    ", (4, 4)),
            ("x\ny", (0, 3), TABS, "\tx\n\ty", (0, 5)),
            ("ab", (1, 1), TABS, "a\tb", (2, 2)),
            ("a\nb", (3, 0), SPACES, "    a\n    b", (11, 0)),
            ("  a\n   b", (0, 8), SPACES, "    a\n    b", (0, 11)),
            ("한a", (3, 3), SPACES, "한  a", (5, 5)),
        ],
        |store, view, indent| tab(store, view, indent).unwrap(),
    );
}

#[test]
fn shift_tab은_단일_커서와_선택_줄을_이전_탭_정지로_내어쓴다() {
    assert_cases(
        &[
            ("    x", (5, 5), SPACES, "x", (1, 1)),
            ("        x", (6, 6), SPACES, "    x", (4, 4)),
            ("  x", (3, 3), SPACES, "x", (1, 1)),
            ("\tx\n    y\nz", (0, 10), SPACES, "x\ny\nz", (0, 5)),
            ("        ", (8, 8), SPACES, "    ", (4, 4)),
            ("x", (1, 1), SPACES, "x", (1, 1)),
            ("\t\tx", (3, 3), TABS, "\tx", (2, 2)),
            ("        x", (2, 9), SPACES, "    x", (2, 5)),
            ("      x", (7, 7), SPACES, "    x", (5, 5)),
            ("    a\n\n    b\nc", (13, 0), SPACES, "a\n\nb\nc", (5, 0)),
        ],
        |store, view, indent| outdent(store, view, indent).unwrap(),
    );
}

#[test]
fn 들여쓰기_안의_backspace는_이전_탭_정지까지_지우고_본문에서는_grapheme_하나를_지운다() {
    assert_cases(
        &[
            ("        x", (8, 8), SPACES, "    x", (4, 4)),
            ("      x", (6, 6), SPACES, "    x", (4, 4)),
            ("\tx", (1, 1), SPACES, "x", (0, 0)),
            ("  x", (1, 1), SPACES, " x", (0, 0)),
            ("    x", (5, 5), SPACES, "    ", (4, 4)),
            ("    e\u{301}", (7, 7), SPACES, "    ", (4, 4)),
            ("x", (0, 0), SPACES, "x", (0, 0)),
        ],
        |store, view, indent| delete_backward(store, view, indent).unwrap(),
    );
}

#[test]
fn 파일_들여쓰기는_원본_style별_크기_우선순위와_미지정_축을_보존한다() {
    let current = IndentOptions {
        tab_size: BASE_SIZE,
        insert_spaces: true,
    };
    assert_eq!(resolve(&EditorConfigOptions::default(), current), current);
    for (style, size, spaces) in [
        (EditorConfigIndentStyle::Space, SPACE_SIZE, true),
        (EditorConfigIndentStyle::Tab, TAB_WIDTH, false),
    ] {
        let config = EditorConfigOptions {
            indent_style: Some(style),
            indent_size: Some(SPACE_SIZE),
            tab_width: Some(TAB_WIDTH),
            ..Default::default()
        };
        assert_eq!(
            resolve(&config, current),
            IndentOptions {
                tab_size: size,
                insert_spaces: spaces,
            }
        );
    }
    for (config, expected) in [
        (
            EditorConfigOptions {
                indent_style: Some(EditorConfigIndentStyle::Space),
                ..Default::default()
            },
            IndentOptions {
                tab_size: BASE_SIZE,
                insert_spaces: true,
            },
        ),
        (
            EditorConfigOptions {
                indent_style: Some(EditorConfigIndentStyle::Tab),
                indent_size: Some(SPACE_SIZE),
                ..Default::default()
            },
            IndentOptions {
                tab_size: SPACE_SIZE,
                insert_spaces: false,
            },
        ),
        (
            EditorConfigOptions {
                indent_size: Some(SPACE_SIZE),
                ..Default::default()
            },
            IndentOptions {
                tab_size: SPACE_SIZE,
                insert_spaces: false,
            },
        ),
        (
            EditorConfigOptions {
                indent_style: Some(EditorConfigIndentStyle::Space),
                tab_width: Some(TAB_WIDTH),
                ..Default::default()
            },
            IndentOptions {
                tab_size: TAB_WIDTH,
                insert_spaces: true,
            },
        ),
    ] {
        assert_eq!(
            resolve(
                &config,
                IndentOptions {
                    insert_spaces: false,
                    ..current
                }
            ),
            expected
        );
    }
}

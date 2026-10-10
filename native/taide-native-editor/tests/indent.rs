use taide_model::file::{EditorConfigIndentStyle, EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{Edit, UndoGroup};
use taide_native_editor::editing::{delete_backward, insert_line_break, outdent, tab};
use taide_native_editor::indent::{IndentConfiguration, IndentOptions, guess, resolve};
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
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

fn opened(text: &str, editor_config: EditorConfigOptions) -> OpenedFile {
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
        editor_config,
    }
}

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
            opened(text, EditorConfigOptions::default()),
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

#[test]
fn 들여쓰기_감지는_설치된_monaco의_원본_사례와_일치한다() {
    for row in include_str!("fixtures/indentation-guesses.tsv").lines() {
        let [name, hex, default_size, default_spaces, size, spaces]: [&str; 6] =
            row.split('\t').collect::<Vec<_>>().try_into().unwrap();
        let bytes = hex
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect::<Vec<_>>();
        let rope = ropey::Rope::from_str(&String::from_utf8(bytes).unwrap());
        let defaults = IndentOptions {
            tab_size: default_size.parse().unwrap(),
            insert_spaces: default_spaces.parse().unwrap(),
        };
        assert_eq!(
            guess(&rope, defaults),
            IndentOptions {
                tab_size: size.parse().unwrap(),
                insert_spaces: spaces.parse().unwrap(),
            },
            "{name} with {defaults:?}"
        );
    }
}

#[test]
fn 감지는_처음_1만_줄만_읽고_긴_줄의_내용과_빈_줄을_폭으로_세지_않는다() {
    const LIMIT: usize = 10_000;
    const LONG_CONTENT: usize = 65_537;
    let after_limit = format!("{}\tchild\n\tend", "root\n".repeat(LIMIT));
    assert_eq!(guess(&ropey::Rope::from_str(&after_limit), SPACES), SPACES);
    let at_limit = format!("{}  child\n\tend", "root\n".repeat(LIMIT - 1));
    assert_eq!(
        guess(&ropey::Rope::from_str(&at_limit), SPACES),
        IndentOptions {
            tab_size: SPACE_SIZE,
            insert_spaces: true,
        }
    );
    let long = format!("{}\n  child\n\t\n    nested\nend", "a".repeat(LONG_CONTENT));
    assert_eq!(
        guess(&ropey::Rope::from_str(&long), SPACES),
        IndentOptions {
            tab_size: SPACE_SIZE,
            insert_spaces: true,
        }
    );
}

#[test]
fn 문서_감지값은_편집마다_재추측하지_않고_전역_들여쓰기_변경에_갱신된다() {
    let (mut store, view) = fixture("root\n  child\n    nested\nend", (0, 0));
    let document = store.views().get(view).unwrap().document;
    let configuration = IndentConfiguration {
        defaults: SPACES,
        detect_indentation: true,
    };
    let detected = IndentOptions {
        tab_size: SPACE_SIZE,
        insert_spaces: true,
    };
    assert_eq!(
        store
            .configure_indentation(document, configuration)
            .unwrap(),
        detected
    );
    let original = store.documents().snapshot(document).unwrap();
    assert_eq!(original.indent_options, Some(detected));
    assert_eq!(original.revision, 0);
    assert!(!original.dirty);
    store
        .apply(
            document,
            Transaction {
                revision: original.revision,
                edits: vec![Edit {
                    bytes: 0..original.rope.len_bytes(),
                    text: "\ta\n\tb".into(),
                }],
                group: UndoGroup(1),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
    assert_eq!(
        store
            .configure_indentation(document, configuration)
            .unwrap(),
        detected
    );
    assert_eq!(
        store
            .configure_indentation(
                document,
                IndentConfiguration {
                    detect_indentation: false,
                    ..configuration
                }
            )
            .unwrap(),
        SPACES
    );
    assert_eq!(
        store
            .configure_indentation(document, configuration)
            .unwrap(),
        TABS
    );
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store.documents().snapshot(document).unwrap().indent_options,
        Some(TABS)
    );
}

#[test]
fn editorconfig는_감지보다_우선하고_메타데이터_변경은_오래된_감지값을_노출하지_않는다() {
    let source = "root\n  child\n    nested\nend";
    let (mut store, view) = fixture(source, (0, 0));
    let document = store.views().get(view).unwrap().document;
    let configuration = IndentConfiguration {
        defaults: SPACES,
        detect_indentation: true,
    };
    store
        .configure_indentation(document, configuration)
        .unwrap();
    let config = EditorConfigOptions {
        indent_style: Some(EditorConfigIndentStyle::Tab),
        indent_size: Some(SPACE_SIZE),
        tab_width: Some(TAB_WIDTH),
        ..Default::default()
    };
    let mut file = opened(source, config);
    file.read_only = true;
    store
        .observe_file(
            document,
            std::path::Path::new("/synthetic/indent.txt"),
            file,
        )
        .unwrap();
    assert_eq!(
        store.documents().snapshot(document).unwrap().indent_options,
        None
    );
    let expected = IndentOptions {
        tab_size: TAB_WIDTH,
        insert_spaces: false,
    };
    assert_eq!(
        store
            .configure_indentation(document, configuration)
            .unwrap(),
        expected
    );
    let snapshot = store.documents().snapshot(document).unwrap();
    assert_eq!(snapshot.indent_options, Some(expected));
    assert!(snapshot.metadata.read_only);
    assert_eq!(snapshot.revision, 0);
    assert!(!snapshot.dirty);
    let mut file = opened(source, EditorConfigOptions::default());
    file.language_id = "json".into();
    store
        .observe_file(
            document,
            std::path::Path::new("/synthetic/indent.txt"),
            file,
        )
        .unwrap();
    assert_eq!(
        store.documents().snapshot(document).unwrap().indent_options,
        None
    );
    assert_eq!(
        store
            .configure_indentation(document, configuration)
            .unwrap()
            .tab_size,
        SPACE_SIZE
    );
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

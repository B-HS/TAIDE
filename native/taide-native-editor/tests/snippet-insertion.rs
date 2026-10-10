use std::ops::Range;
use std::path::PathBuf;

use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{DocumentId, Edit, EditorError, UndoGroup};
use taide_native_editor::snippet_expansion::expand;
use taide_native_editor::snippet_insertion::{PreparedSnippet, insert};
use taide_native_editor::snippet_syntax::{FinalTabstopOptions, ParseLimits, parse_complete};
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
use taide_native_editor::view::{
    Composition, ScrollPosition, Selection, SelectionSet, ViewId, ViewKey,
};

const BYTE_LIMIT: usize = 1024;
const VIEW_LIMIT: usize = 2;
const UNDO_LIMIT: usize = 8;
const NESTING_LIMIT: usize = 32;
const MARKER_LIMIT: usize = 128;
const MIRRORED_TEMPLATE: &str = "${2:late}${1:한}$1$0";
const MATCHING_UNDO_GROUP: u64 = 1;
const SESSION_INDENT_WIDTH: usize = BYTE_LIMIT * 3 / 4;
const SESSION_CHOICE_WIDTH: usize = BYTE_LIMIT / 2;
const SESSION_CURSOR_CHOICE_COUNT: usize = MARKER_LIMIT / 2 + 1;

fn limits() -> ParseLimits {
    ParseLimits {
        max_bytes: BYTE_LIMIT,
        max_nesting: NESTING_LIMIT,
        max_markers: MARKER_LIMIT,
    }
}

fn fixture(text: &str, read_only: bool) -> (EditorStore, DocumentId, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: VIEW_LIMIT,
        max_undo_groups: UNDO_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_file(
            PathBuf::from("/synthetic/snippet.rs"),
            OpenedFile {
                path: "/synthetic/snippet.rs".into(),
                content: text.into(),
                encoding_lossy: false,
                language_id: "rust".into(),
                tier: FileSizeTier::Normal,
                byte_size: text.len().try_into().unwrap(),
                line_count: 1,
                modified_ms: 0.0,
                read_only,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "synthetic".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    (store, document, view)
}

fn prepared(template: &str, replace: Range<usize>) -> PreparedSnippet {
    let markers = parse_complete(
        template,
        limits(),
        FinalTabstopOptions {
            insert: true,
            enforce: false,
        },
        |_, _| None,
    )
    .unwrap();
    let expansion = expand(
        &markers,
        limits(),
        |_| Ok(None),
        |_, _| Err(EditorError::Refused),
    )
    .unwrap();
    PreparedSnippet { replace, expansion }
}

#[test]
fn snippet_session_limits는_들여쓰기와_choice의_합계를_수락전에_검증한다() {
    let text = " ".repeat(SESSION_INDENT_WIDTH);
    let (mut store, document, view) = fixture(&text, false);
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: text.len(),
                    head: text.len(),
                }],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let expected = store.views().get(view).unwrap().clone();
    let template = format!("${{1|a,{}|}}$0", "b".repeat(SESSION_CHOICE_WIDTH));
    let result = insert(
        &mut store,
        &expected,
        0,
        vec![prepared(&template, text.len()..text.len())],
        limits(),
    );
    assert!(matches!(result, Err(EditorError::Capacity)));
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        text
    );
    assert_eq!(
        store.views().get(view).unwrap().selection,
        expected.selection
    );
    assert!(!store.undo(document).unwrap());
}

#[test]
fn snippet_session_limits는_커서별_choice_메타데이터를_합산한다() {
    let (mut store, document, view) = fixture("aa", false);
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![
                    Selection { anchor: 0, head: 0 },
                    Selection { anchor: 2, head: 2 },
                ],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let expected = store.views().get(view).unwrap().clone();
    let choices = std::iter::repeat_n("a", SESSION_CURSOR_CHOICE_COUNT)
        .collect::<Vec<_>>()
        .join(",");
    let template = format!("${{1|{choices}|}}$0");
    let result = insert(
        &mut store,
        &expected,
        0,
        vec![prepared(&template, 0..0), prepared(&template, 2..2)],
        limits(),
    );
    assert!(matches!(result, Err(EditorError::Capacity)));
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "aa"
    );
    assert_eq!(
        store.views().get(view).unwrap().selection,
        expected.selection
    );
    assert!(!store.undo(document).unwrap());
}

#[test]
fn 실제_snippet삽입은_다중커서_기본값과_범위_primary_공유뷰_독립undo를_보존한다() {
    let (mut store, document, view) = fixture("aa__bb", false);
    store
        .apply(
            document,
            Transaction {
                revision: 0,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: "z".into(),
                }],
                group: UndoGroup(MATCHING_UNDO_GROUP),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
    let before = SelectionSet {
        primary: 0,
        selections: vec![
            Selection { anchor: 7, head: 5 },
            Selection { anchor: 1, head: 3 },
        ],
    };
    store
        .set_view_state(view, before.clone(), ScrollPosition::default(), Vec::new())
        .unwrap();
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
    store
        .set_view_state(
            other,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: 7, head: 7 }],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let expected_view = store.views().get(view).unwrap().clone();
    let inserted = insert(
        &mut store,
        &expected_view,
        1,
        vec![
            prepared(MIRRORED_TEMPLATE, 5..7),
            prepared(MIRRORED_TEMPLATE, 1..3),
        ],
        limits(),
    )
    .unwrap();
    let left = "zlate한한".len();
    let right = "zlate한한__".len();
    let right_first = right + "late".len();
    let expected = "zlate한한__late한한";
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        expected
    );
    assert_eq!(inserted.revision, 2);
    assert_eq!(inserted.document, document);
    assert_eq!(inserted.view, view);
    assert_eq!(inserted.snippets[0].bytes, right..expected.len());
    assert_eq!(inserted.snippets[1].bytes, 1..left);
    assert_eq!(
        inserted.selection,
        SelectionSet {
            primary: 0,
            selections: vec![
                Selection {
                    anchor: right_first,
                    head: right_first + "한".len()
                },
                Selection {
                    anchor: right_first + "한".len(),
                    head: expected.len()
                },
                Selection {
                    anchor: "zlate".len(),
                    head: "zlate한".len()
                },
                Selection {
                    anchor: "zlate한".len(),
                    head: left
                },
            ],
        }
    );
    assert_eq!(
        store.views().get(view).unwrap().selection,
        inserted.selection
    );
    assert_eq!(
        store.views().get(other).unwrap().selection.selections[0].head,
        expected.len()
    );
    let after_snippet = store.documents().snapshot(document).unwrap();
    store
        .apply(
            document,
            Transaction {
                revision: after_snippet.revision,
                edits: vec![Edit {
                    bytes: expected.len()..expected.len(),
                    text: "!".into(),
                }],
                group: UndoGroup(MATCHING_UNDO_GROUP),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        expected
    );
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "zaa__bb"
    );
    assert_eq!(store.views().get(view).unwrap().selection, before);
    assert!(store.redo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        expected
    );
    assert_eq!(
        store.views().get(view).unwrap().selection,
        inserted.selection
    );
    assert!(store.undo(document).unwrap());
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "aa__bb"
    );
    assert!(!store.undo(document).unwrap());
}

#[test]
fn snippet삽입_거절과_빈삽입은_문서_선택_조합_undo를_부분변경하지_않는다() {
    let (mut store, document, view) = fixture("한__", false);
    let original = store.documents().snapshot(document).unwrap();
    let before = store.views().get(view).unwrap().clone();
    for result in [
        insert(&mut store, &before, 1, vec![prepared("x", 0..0)], limits()),
        insert(&mut store, &before, 0, vec![prepared("x", 1..1)], limits()),
        insert(&mut store, &before, 0, Vec::new(), limits()),
        insert(
            &mut store,
            &before,
            0,
            vec![prepared("x", 0..0)],
            ParseLimits {
                max_bytes: 0,
                ..limits()
            },
        ),
        insert(
            &mut store,
            &before,
            0,
            vec![prepared(&"x".repeat(BYTE_LIMIT), 0..0)],
            limits(),
        ),
    ] {
        assert!(result.is_err());
        let after = store.documents().snapshot(document).unwrap();
        assert_eq!(after.rope, original.rope);
        assert_eq!(after.revision, original.revision);
        assert_eq!(after.dirty, original.dirty);
        assert_eq!(store.views().get(view).unwrap().selection, before.selection);
        assert!(!store.undo(document).unwrap());
    }
    let mut invalid = prepared("${1:한}", 0..0);
    invalid.expansion.placeholders[0].bytes.start = 1;
    assert!(matches!(
        insert(&mut store, &before, 0, vec![invalid], limits()),
        Err(EditorError::InvalidBoundary)
    ));
    let composition = Composition {
        revision: 0,
        replace: 0.."한".len(),
        preedit: "문".into(),
    };
    store
        .set_composition(view, Some(composition.clone()))
        .unwrap();
    assert!(matches!(
        insert(&mut store, &before, 0, vec![prepared("x", 0..0)], limits()),
        Err(EditorError::Refused)
    ));
    assert_eq!(
        store.views().get(view).unwrap().composition,
        Some(composition)
    );
    store.set_composition(view, None).unwrap();
    let empty = insert(&mut store, &before, 0, vec![prepared("", 0..0)], limits()).unwrap();
    assert_eq!(empty.revision, original.revision);
    assert_eq!(
        store.documents().snapshot(document).unwrap().rope,
        original.rope
    );
    assert!(!store.undo(document).unwrap());
    let (mut readonly_store, readonly_document, readonly_view) = fixture("readonly", true);
    let readonly_before = readonly_store.views().get(readonly_view).unwrap().clone();
    assert!(matches!(
        insert(
            &mut readonly_store,
            &readonly_before,
            0,
            vec![prepared("x", 0..0)],
            limits()
        ),
        Err(EditorError::ReadOnly)
    ));
    assert_eq!(
        readonly_store
            .documents()
            .snapshot(readonly_document)
            .unwrap()
            .rope
            .to_string(),
        "readonly"
    );
    assert_eq!(
        readonly_store.undo(readonly_document),
        Err(EditorError::ReadOnly)
    );
}

#[test]
fn snippet삽입은_같은revision의_선택변경과_중복범위_owner회수를_거절한다() {
    let (mut store, document, view) = fixture("abc", false);
    let before = store.views().get(view).unwrap().clone();
    let changed = SelectionSet {
        primary: 0,
        selections: vec![Selection { anchor: 1, head: 1 }],
    };
    store
        .set_view_state(view, changed.clone(), ScrollPosition::default(), Vec::new())
        .unwrap();
    assert!(matches!(
        insert(&mut store, &before, 0, vec![prepared("x", 0..0)], limits()),
        Err(EditorError::Refused)
    ));
    assert_eq!(store.views().get(view).unwrap().selection, changed);
    assert_eq!(store.documents().snapshot(document).unwrap().revision, 0);
    assert!(!store.undo(document).unwrap());
    let overlapping = SelectionSet {
        primary: 0,
        selections: vec![
            Selection { anchor: 0, head: 2 },
            Selection { anchor: 1, head: 3 },
        ],
    };
    store
        .set_view_state(
            view,
            overlapping.clone(),
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let current = store.views().get(view).unwrap().clone();
    assert!(matches!(
        insert(
            &mut store,
            &current,
            0,
            vec![prepared("x", 0..2), prepared("y", 1..3)],
            limits()
        ),
        Err(EditorError::InvalidBoundary)
    ));
    assert_eq!(
        store.views().get(view).unwrap().selection,
        overlapping.normalized()
    );
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![
                    Selection { anchor: 0, head: 0 },
                    Selection { anchor: 3, head: 3 },
                ],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let current = store.views().get(view).unwrap().clone();
    assert!(matches!(
        insert(
            &mut store,
            &current,
            0,
            vec![prepared("x", 0..2), prepared("y", 1..3)],
            limits()
        ),
        Err(EditorError::Overlap)
    ));
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "abc"
    );
    assert!(!store.undo(document).unwrap());
    store
        .set_view_state(view, changed, ScrollPosition::default(), Vec::new())
        .unwrap();
    let current = store.views().get(view).unwrap().clone();
    let inserted = insert(
        &mut store,
        &current,
        0,
        vec![prepared("${0:끝}", 1..1)],
        limits(),
    )
    .unwrap();
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "a끝bc"
    );
    assert_eq!(
        inserted.selection.selections,
        vec![Selection {
            anchor: 1,
            head: 1 + "끝".len()
        }]
    );
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store.views().get(view).unwrap().selection,
        current.selection
    );
    let closed = store.views().get(view).unwrap().clone();
    store.detach_view(view).unwrap();
    assert!(matches!(
        insert(&mut store, &closed, 0, vec![prepared("x", 1..1)], limits()),
        Err(EditorError::NotFound)
    ));
}

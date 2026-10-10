use std::ops::Range;
use std::path::PathBuf;

use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{DocumentId, Edit, EditorError, UndoGroup};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::snippet_expansion::expand;
use taide_native_editor::snippet_insertion::{PreparedSnippet, insert};
use taide_native_editor::snippet_session::Session;
use taide_native_editor::snippet_syntax::{
    FinalTabstopOptions, ParseLimits, RegexMetadata, parse_complete,
};
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
const FOLLOW_TAB_SIZE: u32 = 4;
const NESTED_MARKER_CAPACITY: usize = 3;
const NESTED_DEPTH: usize = 20;
const REPLACED_PARENT_MARKERS: usize = 4;

#[test]
fn snippet_choice_api는_primary커서와_부분입력후_전체mirror선택을_보존한다() {
    let (mut store, document, view) = fixture("\n", false);
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 1,
                selections: vec![
                    Selection { anchor: 0, head: 0 },
                    Selection { anchor: 1, head: 1 },
                ],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let owner = store.views().get(view).unwrap().clone();
    let template = "${1|red,green|} $1${2:tail}$0";
    let insertion = insert(
        &mut store,
        &owner,
        0,
        vec![prepared(template, 0..0), prepared(template, 1..1)],
        limits(),
    )
    .unwrap();
    let mut session = taide_native_editor::snippet_session::Session::new(
        &store,
        insertion,
        taide_native_editor::indent::IndentOptions {
            tab_size: FOLLOW_TAB_SIZE,
            insert_spaces: true,
        },
        limits(),
    )
    .unwrap();
    let snapshot = store.documents().snapshot(document).unwrap();
    let choice = session.active_choice(&store).unwrap().unwrap();
    assert_eq!(snapshot.rope.byte_to_line(choice.bytes.start), 1);
    assert_eq!(snapshot.rope.byte_slice(choice.bytes).to_string(), "red");
    assert_eq!(choice.options, ["red", "green"]);
    taide_native_editor::editing::type_text(&mut store, view, "\u{1f600}").unwrap();
    assert!(session.synchronize(&store));
    let before = store.documents().snapshot(document).unwrap();
    session.select_active(&mut store).unwrap();
    assert_eq!(
        store.documents().snapshot(document).unwrap().revision,
        before.revision
    );
    let selection = &store.views().get(view).unwrap().selection;
    assert_eq!(selection.primary, 2);
    assert_eq!(selection.selections.len(), 4);
    for selection in &selection.selections {
        assert_eq!(
            before
                .rope
                .byte_slice(selection.anchor..selection.head)
                .to_string(),
            "\u{1f600}"
        );
    }
    session.replace(&mut store, "green", None).unwrap();
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "green greentail\ngreen greentail"
    );
    session
        .step(&mut store, true, |_| Err(EditorError::Refused))
        .unwrap();
    assert!(session.active_choice(&store).unwrap().is_none());
    assert_eq!(store.views().get(view).unwrap().selection.primary, 1);
}

#[test]
fn snippet_follow는_기본입력의_여러_revision과_unicode_mirror뒤_다음tabstop을_보존한다() {
    let (mut store, document, view) = fixture("", false);
    let owner = store.views().get(view).unwrap().clone();
    let insertion = insert(
        &mut store,
        &owner,
        0,
        vec![prepared("${1:한}$1 ${2:next}$0", 0..0)],
        limits(),
    )
    .unwrap();
    let mut session = taide_native_editor::snippet_session::Session::new(
        &store,
        insertion,
        taide_native_editor::indent::IndentOptions {
            tab_size: FOLLOW_TAB_SIZE,
            insert_spaces: true,
        },
        limits(),
    )
    .unwrap();
    for text in ["😀", "a", "b"] {
        taide_native_editor::editing::type_text(&mut store, view, text).unwrap();
    }
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "😀ab😀ab next"
    );
    assert!(session.synchronize(&store));
    let decorations = session.decorations(&store).unwrap();
    assert!(
        decorations
            .iter()
            .any(|item| item.is_active && item.bytes == (0..6))
    );
    assert!(
        decorations
            .iter()
            .any(|item| item.is_active && item.bytes == (6..12)),
        "{decorations:?}"
    );
    assert!(
        session
            .step(&mut store, true, |_| Err(EditorError::Refused))
            .unwrap()
    );
    assert_eq!(
        store.views().get(view).unwrap().selection.selections,
        [Selection {
            anchor: 13,
            head: 17
        }]
    );
}

#[test]
fn snippet_follow는_활성placeholder_밖의_편집을_취소하고_문서변경을_되돌리지_않는다() {
    let (mut store, document, view) = fixture("prefix ", false);
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: 7, head: 7 }],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let owner = store.views().get(view).unwrap().clone();
    let insertion = insert(
        &mut store,
        &owner,
        0,
        vec![prepared("${1:name}$0", 7..7)],
        limits(),
    )
    .unwrap();
    let mut session = taide_native_editor::snippet_session::Session::new(
        &store,
        insertion,
        taide_native_editor::indent::IndentOptions {
            tab_size: FOLLOW_TAB_SIZE,
            insert_spaces: true,
        },
        limits(),
    )
    .unwrap();
    let revision = store.documents().snapshot(document).unwrap().revision;
    store
        .apply(
            document,
            Transaction {
                revision,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: "!".into(),
                }],
                group: UndoGroup(0),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
    assert!(!session.synchronize(&store));
    assert!(!session.is_active());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "!prefix name"
    );
}

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

fn active_session(
    store: &mut EditorStore,
    view: ViewId,
    template: &str,
    limits: ParseLimits,
) -> Session {
    let owner = store.views().get(view).unwrap().clone();
    let revision = store.documents().snapshot(owner.document).unwrap().revision;
    let snippets = owner
        .selection
        .selections
        .iter()
        .map(|selection| {
            prepared(
                template,
                selection.anchor.min(selection.head)..selection.anchor.max(selection.head),
            )
        })
        .collect();
    let insertion = insert(store, &owner, revision, snippets, limits).unwrap();
    Session::new(
        store,
        insertion,
        IndentOptions {
            tab_size: FOLLOW_TAB_SIZE,
            insert_spaces: true,
        },
        limits,
    )
    .unwrap()
}

fn insert_nested(
    store: &mut EditorStore,
    view: ViewId,
    session: &mut Session,
    template: &str,
) -> Result<(), EditorError> {
    let owner = store.views().get(view).unwrap().clone();
    let revision = store.documents().snapshot(owner.document).unwrap().revision;
    let snippets = owner
        .selection
        .selections
        .iter()
        .map(|selection| {
            prepared(
                template,
                selection.anchor.min(selection.head)..selection.anchor.max(selection.head),
            )
        })
        .collect();
    session.insert_nested(store, &owner, revision, snippets)
}

fn selected_text(store: &EditorStore, view: ViewId) -> Vec<String> {
    let owner = store.views().get(view).unwrap();
    let document = store.documents().snapshot(owner.document).unwrap();
    owner
        .selection
        .selections
        .iter()
        .map(|selection| {
            document
                .rope
                .byte_slice(
                    selection.anchor.min(selection.head)..selection.anchor.max(selection.head),
                )
                .to_string()
        })
        .collect()
}

#[test]
fn snippet_nested는_다중커서_choice와_unicode_mirror_주커서_바깥tabstop을_보존한다() {
    let (mut store, document, view) = fixture("\n", false);
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 1,
                selections: vec![
                    Selection { anchor: 0, head: 0 },
                    Selection { anchor: 1, head: 1 },
                ],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let mut session = active_session(&mut store, view, "${1:con} $1 ${2:tail}$0", limits());
    insert_nested(&mut store, view, &mut session, "(${1|x,y|}$1 ${2:z})$0").unwrap();
    assert_eq!(selected_text(&store, view), vec!["x"; 8]);
    assert_eq!(store.views().get(view).unwrap().selection.primary, 4);
    let choice = session.active_choice(&store).unwrap().unwrap();
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .byte_to_line(choice.bytes.start),
        1
    );
    assert_eq!(choice.options, ["x", "y"]);
    session.replace(&mut store, "\u{1f600}", None).unwrap();
    session
        .step(&mut store, true, |_| Err(EditorError::Refused))
        .unwrap();
    assert_eq!(selected_text(&store, view), vec!["z"; 4]);
    assert_eq!(store.views().get(view).unwrap().selection.primary, 2);
    session
        .step(&mut store, true, |_| Err(EditorError::Refused))
        .unwrap();
    assert!(session.is_active());
    assert_eq!(selected_text(&store, view), vec![""; 4]);
    session
        .step(&mut store, true, |_| Err(EditorError::Refused))
        .unwrap();
    assert_eq!(selected_text(&store, view), ["tail", "tail"]);
    assert_eq!(store.views().get(view).unwrap().selection.primary, 1);
    session
        .step(&mut store, false, |_| Err(EditorError::Refused))
        .unwrap();
    session
        .step(&mut store, false, |_| Err(EditorError::Refused))
        .unwrap();
    session
        .step(&mut store, false, |_| Err(EditorError::Refused))
        .unwrap();
    assert_eq!(selected_text(&store, view), vec!["\u{1f600}"; 8]);
    assert_eq!(store.views().get(view).unwrap().selection.primary, 4);
}

#[test]
fn snippet_nested의_병합용량_거절은_문서_선택_revision_undo를_부분변경하지_않는다() {
    let (mut store, document, view) = fixture("", false);
    let limits = ParseLimits {
        max_markers: NESTED_MARKER_CAPACITY,
        ..limits()
    };
    let mut session = active_session(&mut store, view, "${1:x}${2:y}$0", limits);
    let before = store.documents().snapshot(document).unwrap();
    let selection = store.views().get(view).unwrap().selection.clone();
    assert_eq!(
        insert_nested(&mut store, view, &mut session, "${1:a}$0"),
        Err(EditorError::Capacity)
    );
    let after = store.documents().snapshot(document).unwrap();
    assert_eq!(after.rope, before.rope);
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.dirty, before.dirty);
    assert_eq!(store.views().get(view).unwrap().selection, selection);
    assert!(session.is_active());
    insert_nested(&mut store, view, &mut session, "plain").unwrap();
    assert!(session.is_active());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "plainy"
    );
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store.documents().snapshot(document).unwrap().rope,
        before.rope
    );
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        ""
    );
    assert!(!store.undo(document).unwrap());
}

#[test]
fn snippet_nested는_깊은_반복삽입에서도_탭위치가_합쳐지지_않는다() {
    let (mut store, _, view) = fixture("", false);
    let mut session = active_session(&mut store, view, "${1:x}:${2:tail}$0", limits());
    for _ in 0..NESTED_DEPTH {
        insert_nested(&mut store, view, &mut session, "${1:x}:${2:t}$0").unwrap();
        assert!(session.is_active());
        assert_eq!(selected_text(&store, view), ["x"]);
    }
    assert_eq!(
        session.decorations(&store).unwrap().len(),
        NESTED_MARKER_CAPACITY + NESTED_DEPTH * 2
    );
    for _ in 0..NESTED_DEPTH {
        session
            .step(&mut store, true, |_| Err(EditorError::Refused))
            .unwrap();
        assert_eq!(selected_text(&store, view), ["t"]);
        session
            .step(&mut store, true, |_| Err(EditorError::Refused))
            .unwrap();
        assert_eq!(selected_text(&store, view), [""]);
        assert!(session.is_active());
    }
    session
        .step(&mut store, true, |_| Err(EditorError::Refused))
        .unwrap();
    assert_eq!(selected_text(&store, view), ["tail"]);
    session
        .step(&mut store, true, |_| Err(EditorError::Refused))
        .unwrap();
    assert!(!session.is_active());
    assert_eq!(selected_text(&store, view), [""]);
}

#[test]
fn snippet_nested는_교체한_부모의_기존자식_탭위치를_제거한다() {
    let (mut store, _, view) = fixture("", false);
    let mut session = active_session(&mut store, view, "${1:${2:old}} ${3:tail}$0", limits());
    insert_nested(&mut store, view, &mut session, "${1:in}$0").unwrap();
    assert_eq!(
        session.decorations(&store).unwrap().len(),
        REPLACED_PARENT_MARKERS
    );
    assert_eq!(selected_text(&store, view), ["in"]);
    session
        .step(&mut store, true, |_| Err(EditorError::Refused))
        .unwrap();
    assert_eq!(selected_text(&store, view), [""]);
    session
        .step(&mut store, true, |_| Err(EditorError::Refused))
        .unwrap();
    assert_eq!(selected_text(&store, view), ["tail"]);
}

#[test]
fn snippet_nested는_안쪽변환과_바깥변환의_커서별_들여쓰기문맥을_유지한다() {
    let (mut store, document, view) = fixture("  ", false);
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: 2, head: 2 }],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let prepare = |template: &str, replace| {
        let markers = parse_complete(
            template,
            limits(),
            FinalTabstopOptions {
                insert: true,
                enforce: false,
            },
            |source, _| {
                Some(RegexMetadata {
                    source: source.to_owned(),
                    ignore_case: false,
                    global: false,
                })
            },
        )
        .unwrap();
        PreparedSnippet {
            replace,
            expansion: expand(
                &markers,
                limits(),
                |_| Ok(None),
                |_, value| Ok(value.to_owned()),
            )
            .unwrap(),
        }
    };
    let owner = store.views().get(view).unwrap().clone();
    let insertion = insert(
        &mut store,
        &owner,
        0,
        vec![prepare("${1:con}\n    $1\n${2:end}${2/(.*)/$1/}$0", 2..2)],
        limits(),
    )
    .unwrap();
    let mut session = Session::new(
        &store,
        insertion,
        IndentOptions {
            tab_size: FOLLOW_TAB_SIZE,
            insert_spaces: true,
        },
        limits(),
    )
    .unwrap();
    let owner = store.views().get(view).unwrap().clone();
    let revision = store.documents().snapshot(document).unwrap().revision;
    let snippets = owner
        .selection
        .selections
        .iter()
        .map(|selection| prepare("${1:a}${1/(.*)/$1/}$0", selection.anchor..selection.head))
        .collect();
    session
        .insert_nested(&mut store, &owner, revision, snippets)
        .unwrap();
    let mut contexts = Vec::new();
    session
        .step(&mut store, true, |request| {
            contexts.push((
                request.cursor_index,
                request.line_leading_whitespace.to_owned(),
            ));
            Ok(format!("{}\nvalue", request.value))
        })
        .unwrap();
    assert_eq!(contexts, [(0, "  ".to_owned()), (1, "    ".to_owned())]);
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "  aa\n  value\n    aa\n    value\nendend"
    );
    session
        .step(&mut store, true, |_| Err(EditorError::Refused))
        .unwrap();
    contexts.clear();
    session
        .step(&mut store, true, |request| {
            contexts.push((
                request.cursor_index,
                request.line_leading_whitespace.to_owned(),
            ));
            Ok(request.value.to_owned())
        })
        .unwrap();
    assert_eq!(contexts, [(0, "  ".to_owned())]);
    assert!(!session.is_active());
}

#[test]
fn snippet_nested의_오래된준비물_잘못된경계_조합_문서용량은_삽입전에_거절한다() {
    let (mut store, document, view) = fixture("", false);
    let mut session = active_session(&mut store, view, "${1:한}${2:tail}$0", limits());
    let before = store.documents().snapshot(document).unwrap();
    let owner = store.views().get(view).unwrap().clone();
    assert_eq!(
        session.insert_nested(
            &mut store,
            &owner,
            before.revision - 1,
            vec![prepared("x", 0..3)]
        ),
        Err(EditorError::StaleRevision)
    );
    assert_eq!(
        session.insert_nested(
            &mut store,
            &owner,
            before.revision,
            vec![prepared("x", 1..3)]
        ),
        Err(EditorError::InvalidBoundary)
    );
    assert_eq!(
        session.insert_nested(
            &mut store,
            &owner,
            before.revision,
            vec![prepared(&"x".repeat(BYTE_LIMIT), 0..3)]
        ),
        Err(EditorError::Capacity)
    );
    store
        .set_composition(
            view,
            Some(Composition {
                revision: before.revision,
                replace: 0..3,
                preedit: "문".into(),
            }),
        )
        .unwrap();
    assert_eq!(
        session.insert_nested(
            &mut store,
            &owner,
            before.revision,
            vec![prepared("x", 0..3)]
        ),
        Err(EditorError::Refused)
    );
    let after = store.documents().snapshot(document).unwrap();
    assert_eq!(after.rope, before.rope);
    assert_eq!(after.revision, before.revision);
    assert_eq!(store.views().get(view).unwrap().selection, owner.selection);
    assert!(session.is_active());
    store.set_composition(view, None).unwrap();
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        ""
    );
    assert!(!store.undo(document).unwrap());
}

#[test]
fn snippet_nested_context는_살아있는_상위placeholder와_역방향이동을_보존한다() {
    let (mut store, _, view) = fixture("", false);
    let mut session = active_session(
        &mut store,
        view,
        "${1:outer ${2:con}} $1 ${3:tail}$0",
        limits(),
    );
    session
        .step(&mut store, true, |_| Err(EditorError::Refused))
        .unwrap();
    assert_eq!(selected_text(&store, view), ["con", "con"]);
    insert_nested(&mut store, view, &mut session, "${1:in}$0").unwrap();
    let decorations = session.decorations(&store).unwrap();
    assert_eq!(
        decorations
            .iter()
            .filter(|placeholder| placeholder.is_active)
            .count(),
        REPLACED_PARENT_MARKERS
    );
    assert_eq!(selected_text(&store, view), ["in", "in"]);
    session
        .step(&mut store, false, |_| Err(EditorError::Refused))
        .unwrap();
    assert_eq!(selected_text(&store, view), ["outer in", "outer in"]);
    session
        .step(&mut store, true, |_| Err(EditorError::Refused))
        .unwrap();
    assert_eq!(selected_text(&store, view), ["in", "in"]);
    session
        .step(&mut store, true, |_| Err(EditorError::Refused))
        .unwrap();
    session
        .step(&mut store, true, |_| Err(EditorError::Refused))
        .unwrap();
    assert_eq!(selected_text(&store, view), ["tail"]);
}

#[test]
fn snippet_nested_context의_살아있는_들여쓰기문맥_합계는_수락전에_용량을_검사한다() {
    let initial = " ".repeat(SESSION_INDENT_WIDTH);
    let (mut store, document, view) = fixture(&initial, false);
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: initial.len(),
                    head: initial.len(),
                }],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let mut session = active_session(&mut store, view, "${1:x}${2:y}$0", limits());
    let before = store.documents().snapshot(document).unwrap();
    let selection = store.views().get(view).unwrap().selection.clone();
    assert_eq!(
        insert_nested(&mut store, view, &mut session, "${1:a}$0"),
        Err(EditorError::Capacity)
    );
    assert_eq!(
        store.documents().snapshot(document).unwrap().rope,
        before.rope
    );
    assert_eq!(
        store.documents().snapshot(document).unwrap().revision,
        before.revision
    );
    assert_eq!(store.views().get(view).unwrap().selection, selection);
    assert!(session.is_active());
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        initial
    );
    assert!(!store.undo(document).unwrap());
}

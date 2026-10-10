use std::path::PathBuf;

use lsp_types::{CompletionItem, InsertTextFormat};
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::completion::{Candidate, PreparationOptions};
use taide_native_editor::document::{DocumentId, Edit, EditorError, UndoGroup};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::lsp::{LspRange, Position};
use taide_native_editor::snippet_insertion::insert;
use taide_native_editor::snippet_session::Session;
use taide_native_editor::snippet_syntax::ParseLimits;
use taide_native_editor::snippet_variables::SelectionVariables;
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
use taide_native_editor::view::{
    Composition, ScrollPosition, Selection, SelectionSet, ViewId, ViewKey,
};

const BYTE_LIMIT: usize = 4096;
const VIEW_LIMIT: usize = 2;
const UNDO_LIMIT: usize = 8;
const NESTING_LIMIT: usize = 32;
const MARKER_LIMIT: usize = 128;
const TAB_SIZE: u32 = 4;

fn limits() -> ParseLimits {
    ParseLimits {
        max_bytes: BYTE_LIMIT,
        max_nesting: NESTING_LIMIT,
        max_markers: MARKER_LIMIT,
    }
}

fn options() -> PreparationOptions {
    PreparationOptions {
        alternate: false,
        indent: IndentOptions {
            tab_size: TAB_SIZE,
            insert_spaces: true,
        },
        limits: limits(),
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
            PathBuf::from("/synthetic/completion.rs"),
            OpenedFile {
                path: "/synthetic/completion.rs".into(),
                content: text.into(),
                encoding_lossy: false,
                language_id: "rust".into(),
                tier: FileSizeTier::Normal,
                byte_size: text.len().try_into().unwrap(),
                line_count: text.lines().count().try_into().unwrap(),
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

fn select(store: &mut EditorStore, view: ViewId, heads: &[usize]) {
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: heads
                    .iter()
                    .map(|head| Selection {
                        anchor: *head,
                        head: *head,
                    })
                    .collect(),
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
}

fn candidate(
    store: &EditorStore,
    document: DocumentId,
    position: Position,
    start: u32,
    text: &str,
    snippet: bool,
) -> Candidate {
    Candidate::new(
        &store.documents().snapshot(document).unwrap(),
        position,
        LspRange::new(Position::new(position.line, start), position),
        CompletionItem {
            label: "candidate".into(),
            insert_text: Some(text.into()),
            insert_text_format: snippet.then_some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
    )
    .unwrap()
}

#[test]
fn 일반후보는_dollar를_보존하며_다중커서_공유뷰와_분리undo를_사용한다() {
    let (mut store, document, view) = fixture("con\n한con\nfoof", false);
    store
        .apply(
            document,
            Transaction {
                revision: 0,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: " ".into(),
                }],
                group: UndoGroup(0),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
    select(&mut store, view, &[4, 11, 16]);
    let before = store.views().get(view).unwrap().clone();
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
    let snapshot = store.documents().snapshot(document).unwrap();
    let item = candidate(
        &store,
        document,
        Position::new(0, 4),
        1,
        "$1 ${NAME} \\ 한",
        false,
    );
    let prepared = item
        .prepare(
            &snapshot,
            &before.selection,
            options(),
            |_, _| None,
            |_, _| panic!("plain text must not resolve variables"),
            |_, _, _| panic!("plain text must not evaluate transforms"),
        )
        .unwrap();
    let insertion = insert(&mut store, &before, snapshot.revision, prepared, limits()).unwrap();
    let expected = " $1 ${NAME} \\ 한\n한$1 ${NAME} \\ 한\nfoof$1 ${NAME} \\ 한";
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        expected
    );
    assert_eq!(insertion.snippets.len(), 3);
    assert!(
        insertion
            .snippets
            .iter()
            .all(|snippet| snippet.placeholders.is_empty())
    );
    assert!(store.views().get(other).is_some());
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        " con\n한con\nfoof"
    );
    assert_eq!(store.views().get(view).unwrap().selection, before.selection);
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "con\n한con\nfoof"
    );
    assert!(store.redo(document).unwrap());
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
}

#[test]
fn 스니펫후보는_선택변수_커서번호_choice와_mirror_tabstop을_같은session에서_소비한다() {
    let (mut store, document, view) = fixture("con\ncon", false);
    select(&mut store, view, &[3, 7]);
    let before = store.views().get(view).unwrap().clone();
    let snapshot = store.documents().snapshot(document).unwrap();
    let item = candidate(
        &store,
        document,
        Position::new(0, 3),
        0,
        "$CURSOR_NUMBER:${1|한,두|}:$1:${2:끝}:$0",
        true,
    );
    let prepared = item
        .prepare(
            &snapshot,
            &before.selection,
            options(),
            |_, _| None,
            |cursor, context| {
                SelectionVariables::new(&snapshot, &before.selection, cursor, BYTE_LIMIT)?.resolve(
                    context,
                    None,
                    |_, _| Ok(None),
                )
            },
            |_, _, _| Err(EditorError::Refused),
        )
        .unwrap();
    let insertion = insert(&mut store, &before, snapshot.revision, prepared, limits()).unwrap();
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "1:한:한:끝:\n2:한:한:끝:"
    );
    let mut session = Session::new(&store, insertion, options().indent, limits()).unwrap();
    assert!(session.is_active());
    assert_eq!(
        session.active_choice(&store).unwrap().unwrap().options,
        &["한", "두"]
    );
    assert!(session.replace(&mut store, "두", None).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "1:두:두:끝:\n2:두:두:끝:"
    );
    assert!(
        session
            .step(&mut store, true, |_| Err(EditorError::Refused))
            .unwrap()
    );
    assert_eq!(
        store.views().get(view).unwrap().selection.selections.len(),
        2
    );
    assert!(session.active_choice(&store).unwrap().is_none());
    assert!(session.replace(&mut store, "종료", None).unwrap());
    assert!(
        session
            .step(&mut store, true, |_| Err(EditorError::Refused))
            .unwrap()
    );
    assert!(!session.is_active());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "1:두:두:종료:\n2:두:두:종료:"
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
        "con\ncon"
    );
}

#[test]
fn 후보의_여러줄_들여쓰기와_crlf는_각커서_삽입위치를_사용한다() {
    let (mut store, document, view) = fixture("  con\r\n\tcon", false);
    select(&mut store, view, &[5, 11]);
    let before = store.views().get(view).unwrap().clone();
    let snapshot = store.documents().snapshot(document).unwrap();
    let item = candidate(
        &store,
        document,
        Position::new(0, 5),
        2,
        "if {\n\t${1:한}\n}$0",
        true,
    );
    let prepared = item
        .prepare(
            &snapshot,
            &before.selection,
            options(),
            |_, _| None,
            |_, _| Ok(None),
            |_, _, _| Err(EditorError::Refused),
        )
        .unwrap();
    let insertion = insert(&mut store, &before, snapshot.revision, prepared, limits()).unwrap();
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "  if {\r\n    한\r\n  }\r\n\tif {\r\n        한\r\n    }"
    );
    assert_eq!(insertion.selection.selections.len(), 2);
    for selection in &insertion.selection.selections {
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .byte_slice(selection.anchor..selection.head)
                .to_string(),
            "한"
        );
    }
}

#[test]
fn 변수평가_오류와_다중커서_총량제한은_부분삽입하지_않는다() {
    let (mut store, document, view) = fixture("con\ncon", false);
    select(&mut store, view, &[3, 7]);
    let before = store.views().get(view).unwrap().clone();
    let snapshot = store.documents().snapshot(document).unwrap();
    let item = candidate(
        &store,
        document,
        Position::new(0, 3),
        0,
        "$TM_FILENAME",
        true,
    );
    assert!(matches!(
        item.prepare(
            &snapshot,
            &before.selection,
            options(),
            |_, _| None,
            |cursor, _| if cursor == 0 {
                Ok(Some("first".into()))
            } else {
                Err(EditorError::Refused)
            },
            |_, _, _| Err(EditorError::Refused)
        ),
        Err(EditorError::Refused)
    ));
    let mut bounded = options();
    bounded.limits.max_bytes = "candidate".len();
    let plain = candidate(&store, document, Position::new(0, 3), 0, "candidate", false);
    assert!(matches!(
        plain.prepare(
            &snapshot,
            &before.selection,
            bounded,
            |_, _| None,
            |_, _| Ok(None),
            |_, _, _| Err(EditorError::Refused)
        ),
        Err(EditorError::Capacity)
    ));
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "con\ncon"
    );
    assert_eq!(store.views().get(view).unwrap().selection, before.selection);
    assert!(!store.undo(document).unwrap());
}

#[test]
fn readonly_조합중_선택변경과_오래된준비물은_삽입을_거절한다() {
    let (mut store, document, view) = fixture("con", false);
    select(&mut store, view, &[3]);
    let before = store.views().get(view).unwrap().clone();
    let snapshot = store.documents().snapshot(document).unwrap();
    let item = candidate(&store, document, Position::new(0, 3), 0, "console", false);
    let mut readonly = snapshot.clone();
    readonly.metadata.read_only = true;
    assert!(matches!(
        item.prepare(
            &readonly,
            &before.selection,
            options(),
            |_, _| None,
            |_, _| Ok(None),
            |_, _, _| Err(EditorError::Refused)
        ),
        Err(EditorError::ReadOnly)
    ));
    let prepared = item
        .prepare(
            &snapshot,
            &before.selection,
            options(),
            |_, _| None,
            |_, _| Ok(None),
            |_, _, _| Err(EditorError::Refused),
        )
        .unwrap();
    store
        .set_composition(
            view,
            Some(Composition {
                revision: snapshot.revision,
                replace: 3..3,
                preedit: "한".into(),
            }),
        )
        .unwrap();
    assert!(matches!(
        insert(&mut store, &before, snapshot.revision, prepared, limits()),
        Err(EditorError::Refused)
    ));
    store.set_composition(view, None).unwrap();
    let prepared = item
        .prepare(
            &snapshot,
            &before.selection,
            options(),
            |_, _| None,
            |_, _| Ok(None),
            |_, _, _| Err(EditorError::Refused),
        )
        .unwrap();
    select(&mut store, view, &[2]);
    assert!(matches!(
        insert(&mut store, &before, snapshot.revision, prepared, limits()),
        Err(EditorError::Refused)
    ));
    select(&mut store, view, &[3]);
    let prepared = item
        .prepare(
            &snapshot,
            &before.selection,
            options(),
            |_, _| None,
            |_, _| Ok(None),
            |_, _, _| Err(EditorError::Refused),
        )
        .unwrap();
    store
        .apply(
            document,
            Transaction {
                revision: snapshot.revision,
                edits: vec![Edit {
                    bytes: 3..3,
                    text: "x".into(),
                }],
                group: UndoGroup(0),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
    let current = store.views().get(view).unwrap().clone();
    assert!(matches!(
        insert(&mut store, &current, snapshot.revision, prepared, limits()),
        Err(EditorError::StaleRevision)
    ));
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "conx"
    );
}

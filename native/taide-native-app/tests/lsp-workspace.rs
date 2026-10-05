use taide_lsp::native::protocol::lsp_types::{self, WorkspaceEdit};
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_app::lsp::{ProtocolDocument, WorkspaceEditEvent};
use taide_native_app::lsp_workspace::{EditFailure, apply_open};
use taide_native_editor::document::EditorError;
use taide_native_editor::editing::replace_selections;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::ViewKey;

const DOCUMENT_LIMIT: usize = 2;
const VIEW_LIMIT: usize = 4;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 1024;
const PATH: &str = "/synthetic/workspace.rs";
const URI: &str = "file:///synthetic/workspace.rs";
const TEXT: &str = "文😀\n";
const PROTOCOL_REVISION: u64 = 7;
const STALE_PROTOCOL_REVISION: u64 = 6;

fn event(document: ProtocolDocument, edit: WorkspaceEdit) -> WorkspaceEditEvent {
    let (completion, _) = tokio::sync::oneshot::channel();
    WorkspaceEditEvent {
        edit,
        documents: vec![document],
        completion,
    }
}

fn insert(uri: &str, version: u64, text: &str) -> WorkspaceEdit {
    serde_json::from_value(serde_json::json!({"documentChanges":[{
        "textDocument":{"uri":uri,"version":version},"edits":[{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":0}},"newText":text}]
    }]})).unwrap()
}

#[test]
fn workspace_편집은_document_changes_우선_연속편집_undo와_버전_late_edit_알수없는대상을_검사한다() {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let id = store
        .open_file(
            PATH.into(),
            OpenedFile {
                path: PATH.into(),
                content: TEXT.into(),
                language_id: "rust".into(),
                byte_size: TEXT.len().try_into().unwrap(),
                line_count: TEXT.lines().count().try_into().unwrap(),
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
            id,
        )
        .unwrap();
    let document = ProtocolDocument {
        snapshot: store.documents().snapshot(id).unwrap(),
        uri: URI.into(),
        revision: Some(PROTOCOL_REVISION),
    };
    let first = insert(URI, PROTOCOL_REVISION, "first:");
    let mut second = insert(URI, PROTOCOL_REVISION, "second:");
    let Some(lsp_types::DocumentChanges::Edits(ref mut edits)) = second.document_changes else {
        panic!("expected edits")
    };
    let lsp_types::OneOf::Left(edit) = edits[0].edits.remove(0) else {
        panic!("expected text edit")
    };
    edits[0]
        .edits
        .push(lsp_types::OneOf::Right(lsp_types::AnnotatedTextEdit {
            text_edit: edit,
            annotation_id: "synthetic".into(),
        }));
    let Some(lsp_types::DocumentChanges::Edits(mut edits)) = first.document_changes else {
        panic!("expected edits")
    };
    let Some(lsp_types::DocumentChanges::Edits(second_edits)) = second.document_changes else {
        panic!("expected edits")
    };
    edits.extend(second_edits);
    let request = event(
        document.clone(),
        WorkspaceEdit {
            changes: Some(std::collections::HashMap::from([(
                URI.parse().unwrap(),
                vec![lsp_types::TextEdit::new(
                    lsp_types::Range::default(),
                    "ignored:".into(),
                )],
            )])),
            document_changes: Some(lsp_types::DocumentChanges::Edits(edits)),
            change_annotations: None,
        },
    );
    let outcome = apply_open(&mut store, &request);
    assert_eq!(outcome.failure, None);
    assert_eq!(outcome.changed, vec![id]);
    assert_eq!(
        store.documents().snapshot(id).unwrap().rope.to_string(),
        format!("second:first:{TEXT}")
    );
    store.undo(id).unwrap();
    assert_eq!(
        store.documents().snapshot(id).unwrap().rope.to_string(),
        format!("first:{TEXT}")
    );
    store.undo(id).unwrap();
    assert_eq!(
        store.documents().snapshot(id).unwrap().rope.to_string(),
        TEXT
    );
    let current = ProtocolDocument {
        snapshot: store.documents().snapshot(id).unwrap(),
        ..document
    };
    for (request, expected) in [
        (event(current.clone(), insert(URI, STALE_PROTOCOL_REVISION, "stale:")), EditFailure::StaleVersion),
        (event(current.clone(), insert("file:///outside/unopened.rs", PROTOCOL_REVISION, "outside:")), EditFailure::UnconnectedDocument),
        (event(current.clone(), serde_json::from_value(serde_json::json!({"documentChanges":[{"kind":"create","uri":"file:///synthetic/new.rs"}]})).unwrap()), EditFailure::UnconnectedResource),
    ] {
        let before = store.documents().snapshot(id).unwrap();
        let outcome = apply_open(&mut store, &request);
        assert_eq!(outcome.failure, Some(expected));
        assert!(outcome.changed.is_empty());
        assert_eq!(store.documents().snapshot(id).unwrap().revision, before.revision);
        assert_eq!(store.documents().snapshot(id).unwrap().rope, before.rope);
    }
    replace_selections(&mut store, view, "late:", None).unwrap();
    let outcome = apply_open(
        &mut store,
        &event(current, insert(URI, PROTOCOL_REVISION, "old:")),
    );
    assert_eq!(
        outcome.failure,
        Some(EditFailure::Editor(EditorError::StaleRevision))
    );
    assert!(outcome.changed.is_empty());
    assert_eq!(
        store.documents().snapshot(id).unwrap().rope.to_string(),
        format!("late:{TEXT}")
    );
}

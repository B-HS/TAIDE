use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{DocumentKey, EditorError};
use taide_native_editor::editing::replace_selections;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::ViewKey;

const DOCUMENT_LIMIT: usize = 4;
const VIEW_LIMIT: usize = 4;
const HISTORY_LIMIT: usize = 4;
const BYTE_LIMIT: usize = 1024;
const PATH: &str = "/synthetic/disposed.rs";
const CONTENT: &str = "fn main() {}";

fn file() -> OpenedFile {
    OpenedFile {
        path: PATH.into(),
        content: CONTENT.into(),
        language_id: "rust".into(),
        byte_size: CONTENT.len().try_into().unwrap(),
        line_count: 1,
        tier: FileSizeTier::Normal,
        read_only: false,
        encoding_lossy: false,
        modified_ms: 0.0,
        editor_config: EditorConfigOptions::default(),
    }
}

#[test]
fn 모델_폐기_기록은_실제_성공만_보존하고_view_detach와_실패를_제외한다() {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let untracked = store.open_file(PATH.into(), file()).unwrap();
    store.release_document(untracked).unwrap();
    assert!(store.pending_document_disposals().is_empty());
    store.track_document_disposals();
    let document = store.open_file(PATH.into(), file()).unwrap();
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
    assert_eq!(
        store.release_document(document),
        Err(EditorError::AttachedViews)
    );
    assert_eq!(
        store.discard_document(document, 0),
        Err(EditorError::AttachedViews)
    );
    replace_selections(&mut store, view, "dirty:", None).unwrap();
    let revision = store.documents().snapshot(document).unwrap().revision;
    store.detach_view(view).unwrap();
    assert!(store.pending_document_disposals().is_empty());
    assert_eq!(
        store.release_document(document),
        Err(EditorError::UnsavedChanges)
    );
    assert_eq!(
        store.discard_document(document, 0),
        Err(EditorError::StaleRevision)
    );
    assert!(store.pending_document_disposals().is_empty());
    store.discard_document(document, revision).unwrap();
    let key = DocumentKey::File(PATH.into());
    assert_eq!(
        store.pending_document_disposals(),
        std::slice::from_ref(&key)
    );
    store.track_document_disposals();
    assert_eq!(
        store.pending_document_disposals(),
        std::slice::from_ref(&key)
    );
    assert_eq!(store.release_document(document), Err(EditorError::NotFound));
    let reopened = store.open_file(PATH.into(), file()).unwrap();
    assert_ne!(document, reopened);
    store.release_document(reopened).unwrap();
    assert_eq!(store.pending_document_disposals(), &[key.clone(), key]);
    store.acknowledge_document_disposals();
    assert!(store.pending_document_disposals().is_empty());
    let tab = TabId::new();
    let untitled = store.restore_untitled(tab.clone(), None).unwrap();
    store.discard_document(untitled, 0).unwrap();
    assert_eq!(
        store.pending_document_disposals(),
        &[DocumentKey::Untitled(tab)]
    );
}

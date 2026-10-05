use std::path::PathBuf;

use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{DocumentKey, Edit, EditorError, UndoGroup};
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
use taide_native_editor::view::ViewKey;

const DOCUMENT_LIMIT: usize = 4;
const VIEW_LIMIT: usize = 8;
const HISTORY_LIMIT: usize = 4;
const BYTE_LIMIT: usize = 64;
const PATH: &str = "/synthetic/saved.rs";

fn store() -> EditorStore {
    EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap()
}

fn file(content: &str) -> OpenedFile {
    OpenedFile {
        path: PATH.into(),
        content: content.into(),
        language_id: "rust".into(),
        byte_size: content.len().try_into().unwrap(),
        line_count: content.lines().count().try_into().unwrap(),
        tier: FileSizeTier::Normal,
        read_only: false,
        encoding_lossy: false,
        modified_ms: 1.0,
        editor_config: EditorConfigOptions::default(),
    }
}

#[test]
fn untitled_복원은_빈_mirror도_미저장으로_취급하고_live_body와_용량을_보호한다() {
    let mut store = store();
    let tab = TabId::new();
    assert_eq!(
        store.restore_untitled(tab.clone(), Some(&"x".repeat(BYTE_LIMIT + 1))),
        Err(EditorError::Capacity)
    );
    assert!(store.documents().is_empty());
    let document = store.restore_untitled(tab.clone(), Some("")).unwrap();
    assert!(store.documents().snapshot(document).unwrap().dirty);
    assert_eq!(
        store.release_document(document),
        Err(EditorError::UnsavedChanges)
    );
    assert_eq!(
        store.restore_untitled(tab.clone(), Some("obsolete")),
        Ok(document)
    );
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        ""
    );
    let saved = store.save_snapshot(document).unwrap();
    store
        .convert_untitled_save(saved, PathBuf::from(PATH), file(""), tab.clone())
        .unwrap();
    assert!(!store.documents().snapshot(document).unwrap().dirty);
    assert_eq!(
        store
            .documents()
            .find(&DocumentKey::File(PathBuf::from(PATH))),
        Some(document)
    );
    assert!(
        store
            .documents()
            .find(&DocumentKey::Untitled(tab))
            .is_none()
    );
    let fresh = store.restore_untitled(TabId::new(), None).unwrap();
    assert!(!store.documents().snapshot(fresh).unwrap().dirty);
}

#[test]
fn untitled_저장은_dirty_합류를_거절하고_clean_문서_view와_늦은_편집을_원자적으로_합류한다() {
    let mut store = store();
    store.track_document_disposals();
    let source_tab = TabId::new();
    let target_tab = TabId::new();
    let pane = PaneId::new();
    let target = store.open_file(PathBuf::from(PATH), file("disk")).unwrap();
    let target_key = ViewKey {
        window: "main".into(),
        pane: pane.clone(),
        tab: target_tab.clone(),
    };
    let target_view = store.attach_view(target_key.clone(), target).unwrap();
    let auxiliary = store
        .attach_view(
            ViewKey {
                window: "auxiliary".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            target,
        )
        .unwrap();
    let source = store
        .restore_untitled(source_tab.clone(), Some("draft"))
        .unwrap();
    let source_key = ViewKey {
        window: "main".into(),
        pane,
        tab: source_tab.clone(),
    };
    let source_view = store.attach_view(source_key.clone(), source).unwrap();
    let snapshot = store.save_snapshot(source).unwrap();
    store
        .apply(
            target,
            Transaction {
                revision: 0,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: "unsaved".into(),
                }],
                group: UndoGroup(0),
                origin: Some(target_view),
                selection_after: None,
            },
        )
        .unwrap();
    assert_eq!(
        store.convert_untitled_save(
            snapshot.clone(),
            PathBuf::from(PATH),
            file("draft"),
            target_tab.clone()
        ),
        Err(EditorError::UnsavedChanges)
    );
    assert_eq!(store.documents().len(), 2);
    assert_eq!(
        store.documents().snapshot(source).unwrap().key,
        DocumentKey::Untitled(source_tab.clone())
    );
    assert!(store.undo(target).unwrap());
    assert_eq!(
        store.convert_untitled_save(
            snapshot.clone(),
            PathBuf::from(PATH),
            file("draft"),
            target_tab.clone()
        ),
        Err(EditorError::UnsavedChanges)
    );
    let saved_target = store.save_snapshot(target).unwrap();
    assert!(store.pending_document_disposals().is_empty());
    assert!(store.mark_saved(saved_target, None).unwrap());
    store
        .apply(
            source,
            Transaction {
                revision: 0,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: "later ".into(),
                }],
                group: UndoGroup(0),
                origin: Some(source_view),
                selection_after: None,
            },
        )
        .unwrap();
    assert_eq!(
        store.convert_untitled_save(snapshot, PathBuf::from(PATH), file("draft"), target_tab),
        Ok(Some(target))
    );
    assert_eq!(
        store.pending_document_disposals(),
        &[
            DocumentKey::File(PathBuf::from(PATH)),
            DocumentKey::Untitled(source_tab)
        ]
    );
    assert_eq!(store.documents().len(), 1);
    assert_eq!(
        store.documents().snapshot(target).err(),
        Some(EditorError::NotFound)
    );
    assert_eq!(
        store.documents().snapshot(source).unwrap().rope.to_string(),
        "later draft"
    );
    assert!(store.documents().snapshot(source).unwrap().dirty);
    assert_eq!(store.views().find(&target_key), Some(target_view));
    assert!(store.views().find(&source_key).is_none());
    assert!(store.views().get(source_view).is_none());
    assert_eq!(store.views().get(target_view).unwrap().document, source);
    assert_eq!(store.views().get(auxiliary).unwrap().document, source);
    assert!(store.undo(source).unwrap());
    assert!(store.documents().snapshot(source).unwrap().dirty);
    assert_eq!(
        store.documents().snapshot(source).unwrap().rope.to_string(),
        "draft"
    );
}

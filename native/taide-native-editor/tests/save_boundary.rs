use std::path::PathBuf;

use taide_model::file::{EditorConfigOptions, FileSizeTier, LARGE_FILE_BYTES, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{Edit, UndoGroup};
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
use taide_native_editor::view::ViewKey;

const DOCUMENT_COUNT: usize = 2;
const VIEW_COUNT: usize = 2;
const HISTORY_COUNT: usize = 4;
const SAVE_TIME: f64 = 2.0;

#[test]
fn 저장_snapshot은_같은_typing_group도_분리해_저장중_편집의_undo를_보존한다() {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_COUNT,
        max_views: VIEW_COUNT,
        max_undo_groups: HISTORY_COUNT,
        max_document_bytes: LARGE_FILE_BYTES as usize,
    })
    .unwrap();
    let document = store
        .open_file(
            PathBuf::from("/synthetic/save.rs"),
            OpenedFile {
                path: "/synthetic/save.rs".into(),
                content: String::new(),
                language_id: "rust".into(),
                byte_size: 0,
                line_count: 1,
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
    store
        .apply(
            document,
            Transaction {
                revision: 0,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: "a".into(),
                }],
                group: UndoGroup(0),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
    let pending = store.save_snapshot(document).unwrap();
    store
        .apply(
            document,
            Transaction {
                revision: 1,
                edits: vec![Edit {
                    bytes: 1..1,
                    text: "b".into(),
                }],
                group: UndoGroup(0),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
    assert!(!store.mark_saved(pending, Some(SAVE_TIME)).unwrap());
    assert!(store.undo(document).unwrap());
    let snapshot = store.documents().snapshot(document).unwrap();
    assert_eq!(snapshot.rope.to_string(), "a");
    assert!(snapshot.dirty);
    assert!(store.redo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "ab"
    );
}

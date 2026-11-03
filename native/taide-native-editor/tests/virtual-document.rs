use taide_model::ids::TabId;
use taide_native_editor::document::{DocumentKey, EditorError, VirtualKey, VirtualSide};
use taide_native_editor::store::{EditorLimits, EditorStore};

const DOCUMENT_LIMIT: usize = 8;
const VIEW_LIMIT: usize = 8;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 64;

fn store() -> EditorStore {
    EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap()
}

#[test]
fn 가상_문서는_처음_내용이_깨끗한_기준이고_교체하면_새_기준이_된다() {
    let mut store = store();
    let owner = TabId::new();
    let key = |side| VirtualKey {
        owner: owner.clone(),
        side,
    };
    let original = store
        .open_virtual(key(VirtualSide::Original), "a\nb\n", "rust".into(), true)
        .unwrap();
    let modified = store
        .open_virtual(key(VirtualSide::Modified), "a\nc\n", "rust".into(), true)
        .unwrap();

    assert_ne!(original, modified);
    assert_eq!(
        store
            .documents()
            .find(&DocumentKey::Virtual(key(VirtualSide::Original))),
        Some(original)
    );
    let snapshot = store.documents().snapshot(original).unwrap();
    assert!(!snapshot.dirty);
    assert!(snapshot.metadata.read_only);
    assert_eq!(snapshot.metadata.language_id, "rust");

    assert!(!store.replace_virtual(original, "a\nb\n", "rust").unwrap());
    let before = store.documents().snapshot(original).unwrap().revision;
    assert!(store.replace_virtual(original, "x\n", "plaintext").unwrap());
    let replaced = store.documents().snapshot(original).unwrap();
    assert_eq!(replaced.rope.to_string(), "x\n");
    assert_eq!(replaced.metadata.language_id, "plaintext");
    assert!(replaced.revision > before);
    assert!(!replaced.dirty);
    assert_eq!(store.undo(original), Err(EditorError::ReadOnly));

    assert_eq!(
        store.replace_virtual(original, &"y".repeat(BYTE_LIMIT + 1), "plaintext"),
        Err(EditorError::Capacity)
    );
    let untitled = store
        .open_untitled(TabId::new(), "", "plaintext".into())
        .unwrap();
    assert_eq!(
        store.replace_virtual(untitled, "z", "plaintext"),
        Err(EditorError::InvalidIdentity)
    );
}

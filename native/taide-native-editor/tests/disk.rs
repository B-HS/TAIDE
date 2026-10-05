use std::path::{Path, PathBuf};

use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{DiskChoice, EditorError};
use taide_native_editor::editing::replace_selections;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{Composition, ScrollPosition, Selection, SelectionSet, ViewKey};

const PATH: &str = "/synthetic/observed.rs";
const OTHER_PATH: &str = "/synthetic/other.rs";
const DOCUMENT_LIMIT: usize = 4;
const VIEW_LIMIT: usize = 8;
const HISTORY_LIMIT: usize = 4;
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

fn view_key(window: &str) -> ViewKey {
    ViewKey {
        window: window.into(),
        pane: PaneId::new(),
        tab: TabId::new(),
    }
}

#[test]
fn dirty_파일_관찰과_keep_mine은_초안_revision_undo를_보존한다() {
    let mut store = store();
    let document = store.open_file(PathBuf::from(PATH), file("disk")).unwrap();
    let view = store.attach_view(view_key("main"), document).unwrap();
    replace_selections(&mut store, view, "draft ", None).unwrap();
    let before = store.documents().snapshot(document).unwrap();
    assert!(
        store
            .observe_file(document, Path::new(PATH), file("external"))
            .unwrap()
    );
    let observed = store.documents().snapshot(document).unwrap();
    assert_eq!(observed.revision, before.revision);
    assert_eq!(observed.rope, before.rope);
    assert!(observed.dirty);
    assert!(
        store
            .choose_disk(
                document,
                before.revision,
                Path::new(PATH),
                file("draft disk"),
                DiskChoice::KeepMine
            )
            .unwrap()
    );
    assert!(!store.has_disk_conflict(document).unwrap());
    assert!(store.documents().snapshot(document).unwrap().dirty);
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
        "disk"
    );
    assert!(store.documents().snapshot(document).unwrap().dirty);
    assert!(store.redo(document).unwrap());
    assert_eq!(
        store.documents().snapshot(document).unwrap().rope,
        before.rope
    );
}

#[test]
fn view_disk는_stale_선택을_거절하고_공유_view를_정리하며_이전_save를_거절한다() {
    let mut store = store();
    let document = store
        .open_file(PathBuf::from(PATH), file("original"))
        .unwrap();
    let main = store.attach_view(view_key("main"), document).unwrap();
    let auxiliary = store.attach_view(view_key("auxiliary"), document).unwrap();
    replace_selections(&mut store, main, "draft ", None).unwrap();
    let before = store.documents().snapshot(document).unwrap();
    let saved = store.save_snapshot(document).unwrap();
    let end = before.rope.len_bytes();
    for view in [main, auxiliary] {
        store
            .set_view_state(
                view,
                SelectionSet {
                    primary: 0,
                    selections: vec![Selection {
                        anchor: end,
                        head: end,
                    }],
                },
                ScrollPosition::default(),
                std::iter::once(0..end).collect(),
            )
            .unwrap();
        store
            .set_composition(
                view,
                Some(Composition {
                    revision: before.revision,
                    replace: end..end,
                    preedit: "한".into(),
                }),
            )
            .unwrap();
    }
    assert_eq!(
        store.choose_disk(
            document,
            0,
            Path::new(PATH),
            file("x"),
            DiskChoice::ViewDisk
        ),
        Err(EditorError::StaleRevision)
    );
    assert_eq!(
        store.choose_disk(
            document,
            before.revision,
            Path::new(OTHER_PATH),
            file("x"),
            DiskChoice::ViewDisk
        ),
        Err(EditorError::InvalidIdentity)
    );
    assert_eq!(
        store.choose_disk(
            document,
            before.revision,
            Path::new(PATH),
            file(&"x".repeat(BYTE_LIMIT + 1)),
            DiskChoice::ViewDisk
        ),
        Err(EditorError::Capacity)
    );
    let mut refused = file("x");
    refused.tier = FileSizeTier::Refused;
    assert_eq!(
        store.choose_disk(
            document,
            before.revision,
            Path::new(PATH),
            refused,
            DiskChoice::ViewDisk
        ),
        Err(EditorError::Refused)
    );
    assert_eq!(
        store.documents().snapshot(document).unwrap().rope,
        before.rope
    );
    assert!(store.views().get(auxiliary).unwrap().composition.is_some());
    let mut disk = file("한");
    disk.encoding_lossy = true;
    assert!(
        !store
            .choose_disk(
                document,
                before.revision,
                Path::new(PATH),
                disk,
                DiskChoice::ViewDisk
            )
            .unwrap()
    );
    let after = store.documents().snapshot(document).unwrap();
    assert_eq!(after.rope.to_string(), "한");
    assert!(after.metadata.read_only);
    assert!(!after.dirty);
    for view in [main, auxiliary] {
        let view = store.views().get(view).unwrap();
        assert_eq!(view.selection.selections[0].head, after.rope.len_bytes());
        assert!(view.composition.is_none());
        assert!(view.folds.is_empty());
    }
    assert_eq!(store.mark_saved(saved, None), Err(EditorError::StaleSave));
    assert_eq!(store.undo(document), Err(EditorError::ReadOnly));
    assert!(!store.has_disk_conflict(document).unwrap());
}

#[test]
fn 동일_저장_관찰은_undo를_유지하고_실제_교체만_초기화하며_동일_mirror도_dirty다() {
    let mut store = store();
    let document = store.open_file(PathBuf::from(PATH), file("disk")).unwrap();
    let view = store.attach_view(view_key("main"), document).unwrap();
    replace_selections(&mut store, view, "saved ", None).unwrap();
    let snapshot = store.save_snapshot(document).unwrap();
    assert!(store.mark_saved(snapshot, None).unwrap());
    let revision = store.documents().snapshot(document).unwrap().revision;
    assert!(
        !store
            .observe_file(document, Path::new(PATH), file("saved disk"))
            .unwrap()
    );
    assert_eq!(
        store.documents().snapshot(document).unwrap().revision,
        revision
    );
    assert!(store.undo(document).unwrap());
    assert!(store.documents().snapshot(document).unwrap().dirty);
    assert!(store.redo(document).unwrap());
    assert!(
        !store
            .observe_file(document, Path::new(PATH), file("replacement"))
            .unwrap()
    );
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "replacement"
    );
    assert!(!store.undo(document).unwrap());
    let mut restored = self::store();
    let document = restored
        .open_file_with_draft(PathBuf::from(PATH), file("same"), Some("same"))
        .unwrap();
    assert!(restored.documents().snapshot(document).unwrap().dirty);
    assert!(!restored.has_disk_conflict(document).unwrap());
    assert_eq!(
        restored.release_document(document),
        Err(EditorError::UnsavedChanges)
    );
}

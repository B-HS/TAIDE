use std::path::PathBuf;

use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{DocumentKey, DocumentMetadata, EditorError};
use taide_native_editor::editing::replace_selections;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{ScrollPosition, Selection, SelectionSet, ViewKey};

const DOCUMENT_LIMIT: usize = 4;
const VIEW_LIMIT: usize = 8;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 1024;
const OLD_PATH: &str = "/synthetic/old.txt";
const NEW_PATH: &str = "/synthetic/new.rs";
const SCROLL_X: f32 = 12.0;
const SCROLL_Y: f32 = 24.0;

fn file(path: &str, content: &str) -> OpenedFile {
    OpenedFile {
        path: path.into(),
        content: content.into(),
        language_id: "plaintext".into(),
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
fn 파일_경로_전환은_본문_dirty_공유_view를_유지하고_undo와_늦은_save를_분리한다() {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    store.track_document_disposals();
    let source = store
        .open_file(OLD_PATH.into(), file(OLD_PATH, "body\r\n"))
        .unwrap();
    let views = ["main", "auxiliary"].map(|window| {
        store
            .attach_view(
                ViewKey {
                    window: window.into(),
                    pane: PaneId::new(),
                    tab: TabId::new(),
                },
                source,
            )
            .unwrap()
    });
    let stale = store.documents().snapshot(source).unwrap();
    replace_selections(&mut store, views[0], "unsaved:", None).unwrap();
    let saved_before_rename = store.save_snapshot(source).unwrap();
    let requested = store.documents().snapshot(source).unwrap();
    let selection = SelectionSet {
        primary: 0,
        selections: vec![Selection { anchor: 1, head: 1 }],
    };
    let scroll = ScrollPosition {
        x: SCROLL_X,
        y: SCROLL_Y,
    };
    let folds = std::iter::once(0..1).collect::<Vec<_>>();
    store
        .set_view_state(views[1], selection.clone(), scroll.clone(), folds.clone())
        .unwrap();
    let target = store
        .open_file(NEW_PATH.into(), file(NEW_PATH, "stale target"))
        .unwrap();
    let target_view = store
        .attach_view(
            ViewKey {
                window: "main".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            target,
        )
        .unwrap();
    replace_selections(&mut store, target_view, "dirty:", None).unwrap();
    store
        .set_view_state(
            target_view,
            SelectionSet::default(),
            ScrollPosition::default(),
            folds.clone(),
        )
        .unwrap();
    let mut metadata = DocumentMetadata::from_opened(&file(NEW_PATH, "body\r\n"));
    metadata.language_id = "rust".into();
    assert_eq!(
        store.retarget_file(&stale, NEW_PATH.into(), metadata.clone()),
        Err(EditorError::StaleRevision)
    );
    assert_eq!(
        store.retarget_file(&requested, PathBuf::from("relative"), metadata.clone()),
        Err(EditorError::InvalidIdentity)
    );
    assert_eq!(
        store.documents().snapshot(source).unwrap().key,
        DocumentKey::File(OLD_PATH.into())
    );
    assert!(store.pending_document_disposals().is_empty());
    assert_eq!(
        store.retarget_file(&requested, NEW_PATH.into(), metadata),
        Ok(Some(target))
    );
    assert_eq!(
        store.pending_document_disposals(),
        &[
            DocumentKey::File(NEW_PATH.into()),
            DocumentKey::File(OLD_PATH.into())
        ]
    );
    store.acknowledge_document_disposals();
    let current = store.documents().snapshot(source).unwrap();
    assert_eq!(current.key, DocumentKey::File(NEW_PATH.into()));
    assert_eq!(current.rope.to_string(), "unsaved:body\r\n");
    assert!(current.dirty);
    assert_eq!(current.metadata.language_id, "rust");
    assert!(
        store
            .documents()
            .find(&DocumentKey::File(OLD_PATH.into()))
            .is_none()
    );
    assert_eq!(
        store.documents().find(&DocumentKey::File(NEW_PATH.into())),
        Some(source)
    );
    assert_eq!(
        store.documents().snapshot(target).err(),
        Some(EditorError::NotFound)
    );
    assert_eq!(store.views().get(views[1]).unwrap().selection, selection);
    assert_eq!(store.views().get(views[1]).unwrap().scroll, scroll);
    assert_eq!(store.views().get(target_view).unwrap().document, source);
    assert!(store.views().get(target_view).unwrap().folds.is_empty());
    assert_eq!(store.views().get(views[1]).unwrap().folds, folds);
    assert!(!store.undo(source).unwrap());
    assert!(!store.redo(source).unwrap());
    assert_eq!(
        store.mark_saved(saved_before_rename, None),
        Err(EditorError::InvalidIdentity)
    );
    let old_save = store.save_snapshot(source).unwrap();
    store
        .retarget_file(&current, NEW_PATH.into(), current.metadata.clone())
        .unwrap();
    assert!(store.pending_document_disposals().is_empty());
    assert_eq!(
        store.mark_saved(old_save, None),
        Err(EditorError::StaleSave)
    );
    let save = store.save_snapshot(source).unwrap();
    assert!(store.mark_saved(save, None).unwrap());
    assert!(!store.documents().snapshot(source).unwrap().dirty);
}

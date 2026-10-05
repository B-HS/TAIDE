use taide_model::app::{AppFileTarget, PromptTemplateId};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{DocumentId, DocumentKey, Edit, EditorError, UndoGroup};
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
use taide_native_editor::view::{Composition, ScrollPosition, Selection, SelectionSet, ViewKey};

const DOCUMENT_LIMIT: usize = 8;
const VIEW_LIMIT: usize = 8;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 256;
const ORIGINAL: &str = "{\"editorFontSize\":14}";
const RAW: &str = "{\"editorFontSize\":999}";
const CANONICAL: &str = "{\"editorFontSize\":40}";

fn store() -> EditorStore {
    EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap()
}

fn replace(store: &mut EditorStore, document: DocumentId, content: &str) {
    let current = store.documents().snapshot(document).unwrap();
    store
        .apply(
            document,
            Transaction {
                revision: current.revision,
                edits: vec![Edit {
                    bytes: 0..current.rope.len_bytes(),
                    text: content.into(),
                }],
                group: UndoGroup(current.revision),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
}

#[test]
fn app_file는_닫힌_target별_공유문서와_숨김_dirty를_유지하고_파일_경계를_분리한다() {
    let mut store = store();
    let target = AppFileTarget::Settings;
    let document = store.open_app_file(target, ORIGINAL).unwrap();
    let initial = store.documents().snapshot(document).unwrap();
    assert_eq!(initial.key, DocumentKey::AppFile(target));
    assert_eq!(initial.metadata.language_id, "json");
    assert!(initial.metadata.disk_modified_ms.is_none());
    assert!(!initial.metadata.read_only);
    assert!(!initial.dirty);
    assert!(!store.has_disk_conflict(document).unwrap());
    let key = ViewKey {
        window: "main".into(),
        pane: PaneId::new(),
        tab: TabId::new(),
    };
    let view = store.attach_view(key.clone(), document).unwrap();
    let auxiliary = store
        .attach_view(
            ViewKey {
                window: "auxiliary".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    replace(&mut store, document, RAW);
    store.detach_view(view).unwrap();
    store.detach_view(auxiliary).unwrap();
    assert_eq!(
        store.release_document(document),
        Err(EditorError::UnsavedChanges)
    );
    assert_eq!(store.open_app_file(target, "obsolete"), Ok(document));
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        RAW
    );
    let remounted = store.attach_view(key, document).unwrap();
    assert_eq!(store.views().get(remounted).unwrap().document, document);
    for id in [
        PromptTemplateId::AutoTabDefault,
        PromptTemplateId::InlineEditDefault,
        PromptTemplateId::CommitMessageDefault,
    ] {
        let prompt = AppFileTarget::Prompt { id };
        let separate = store.open_app_file(prompt, "{}").unwrap();
        assert_ne!(separate, document);
        assert_eq!(
            store.documents().find(&DocumentKey::AppFile(prompt)),
            Some(separate)
        );
        assert!(!store.documents().snapshot(separate).unwrap().dirty);
    }
    assert_eq!(store.documents().len(), 4);
    let requested = store.save_snapshot(document).unwrap();
    assert_eq!(
        store.mark_saved(requested.clone(), None),
        Err(EditorError::InvalidIdentity)
    );
    assert_eq!(
        store.mark_app_file_saved(
            requested.clone(),
            AppFileTarget::Prompt {
                id: PromptTemplateId::AutoTabDefault
            },
            "{}"
        ),
        Err(EditorError::InvalidIdentity)
    );
    assert_eq!(
        store.mark_app_file_saved(requested, target, &"x".repeat(BYTE_LIMIT + 1)),
        Err(EditorError::Capacity)
    );
    let unchanged = store.documents().snapshot(document).unwrap();
    assert_eq!(unchanged.rope.to_string(), RAW);
    assert!(unchanged.dirty);
    assert!(!store.has_disk_conflict(document).unwrap());
    assert_eq!(
        store.observe_file(
            document,
            std::path::Path::new("/__app-file__/settings.json"),
            file()
        ),
        Err(EditorError::InvalidIdentity)
    );
}

fn file() -> taide_model::file::OpenedFile {
    taide_model::file::OpenedFile {
        path: "/__app-file__/settings.json".into(),
        content: ORIGINAL.into(),
        language_id: "json".into(),
        byte_size: ORIGINAL.len().try_into().unwrap(),
        line_count: 1,
        tier: taide_model::file::FileSizeTier::Normal,
        read_only: false,
        encoding_lossy: false,
        modified_ms: 1.0,
        editor_config: Default::default(),
    }
}

#[test]
fn app_file_저장은_추가편집을_보호하고_clean일_때만_canonical을_반영한다() {
    let mut store = store();
    let target = AppFileTarget::Settings;
    let document = store.open_app_file(target, ORIGINAL).unwrap();
    replace(&mut store, document, RAW);
    let first = store.save_snapshot(document).unwrap();
    replace(&mut store, document, "later draft");
    assert!(
        !store
            .mark_app_file_saved(first.clone(), target, CANONICAL)
            .unwrap()
    );
    let later = store.documents().snapshot(document).unwrap();
    assert_eq!(later.rope.to_string(), "later draft");
    assert!(later.dirty);
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        RAW
    );
    assert!(store.documents().snapshot(document).unwrap().dirty);
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
    let before = store.documents().snapshot(document).unwrap();
    let end = before.rope.len_bytes();
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
                replace: 0..1,
                preedit: "한".into(),
            }),
        )
        .unwrap();
    let second = store.save_snapshot(document).unwrap();
    assert!(
        store
            .mark_app_file_saved(second, target, CANONICAL)
            .unwrap()
    );
    let clean = store.documents().snapshot(document).unwrap();
    assert_eq!(clean.rope.to_string(), CANONICAL);
    assert!(!clean.dirty);
    assert!(clean.revision > before.revision);
    assert_eq!(
        store.views().get(view).unwrap().selection.selections[0].head,
        CANONICAL.len()
    );
    assert!(store.views().get(view).unwrap().composition.is_none());
    assert!(store.views().get(view).unwrap().folds.is_empty());
    assert!(!store.undo(document).unwrap());
    assert_eq!(
        store.mark_app_file_saved(first, target, ORIGINAL),
        Err(EditorError::StaleSave)
    );
    assert!(!store.has_disk_conflict(document).unwrap());
    let saved = store.save_snapshot(document).unwrap();
    store.detach_view(view).unwrap();
    store.release_document(document).unwrap();
    let fresh = store.open_app_file(target, ORIGINAL).unwrap();
    assert_ne!(fresh, document);
    assert_eq!(
        store.mark_app_file_saved(saved, target, CANONICAL),
        Err(EditorError::NotFound)
    );
    assert_eq!(
        store.documents().snapshot(fresh).unwrap().rope.to_string(),
        ORIGINAL
    );
}

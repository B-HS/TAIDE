use std::path::PathBuf;

use taide_model::file::{EditorConfigOptions, FileSizeTier, LARGE_FILE_BYTES, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{DocumentId, DocumentKey, Edit, EditorError, UndoGroup};
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
use taide_native_editor::view::{
    Composition, EditOperation, EditRun, GoalColumns, ScrollPosition, Selection, SelectionSet,
    ViewId, ViewKey,
};

const DOCUMENT_COUNT: usize = 4;
const VIEW_COUNT: usize = 8;
const HISTORY_COUNT: usize = 2;
const SCROLL_Y: f32 = 100.0;
const FIRST_SAVE_TIME: f64 = 2.0;
const RESTORE_BYTE_LIMIT: usize = 8;
const GOAL_LEFTOVER: isize = 3;

#[test]
fn goal_column과_edit_run은_view에만_남고_선택_변경은_goal_column만_지운다() {
    let mut store = store();
    let document = open(&mut store, "ab\ncd");
    let first = view(&mut store, document, "main");
    let second = view(&mut store, document, "auxiliary");
    assert_eq!(
        store.set_goal_columns(
            first,
            Some(GoalColumns {
                revision: 0,
                leftover_visible_columns: Vec::new(),
            })
        ),
        Err(EditorError::InvalidBoundary)
    );
    let goal = GoalColumns {
        revision: 0,
        leftover_visible_columns: vec![GOAL_LEFTOVER],
    };
    let run = EditRun {
        operation: EditOperation::TypingOther,
        group: UndoGroup(0),
        revision: 0,
        selection: caret(0),
    };
    store.set_goal_columns(first, Some(goal.clone())).unwrap();
    store.set_edit_run(first, Some(run.clone())).unwrap();
    store
        .set_view_state(
            first,
            caret(0),
            ScrollPosition {
                x: 0.0,
                y: SCROLL_Y,
            },
            Vec::new(),
        )
        .unwrap();
    assert_eq!(store.views().get(first).unwrap().goal_columns, Some(goal));
    store
        .set_view_state(first, caret(1), ScrollPosition::default(), Vec::new())
        .unwrap();
    let moved = store.views().get(first).unwrap();
    assert_eq!(moved.goal_columns, None);
    assert_eq!(moved.edit_run, Some(run));
    let other = store.views().get(second).unwrap();
    assert_eq!(other.goal_columns, None);
    assert_eq!(other.edit_run, None);
    store.set_edit_run(first, None).unwrap();
    assert_eq!(store.views().get(first).unwrap().edit_run, None);
    store.detach_view(first).unwrap();
    assert_eq!(store.set_edit_run(first, None), Err(EditorError::NotFound));
    assert_eq!(
        store.set_goal_columns(first, None),
        Err(EditorError::NotFound)
    );
}

#[test]
fn clean_file_갱신은_공유_view를_유지하고_dirty_문서와_잘못된_경로를_거절한다() {
    let mut store = store();
    let path = PathBuf::from("/synthetic/main.rs");
    let document = open(&mut store, "abcdef");
    let first = view(&mut store, document, "main");
    let second = view(&mut store, document, "auxiliary");
    let end = store
        .documents()
        .snapshot(document)
        .unwrap()
        .rope
        .len_bytes();
    store
        .set_view_state(
            first,
            caret(end),
            ScrollPosition::default(),
            std::iter::once(0..end).collect(),
        )
        .unwrap();
    store
        .set_composition(
            second,
            Some(Composition {
                revision: 0,
                replace: 0..1,
                preedit: "한".into(),
            }),
        )
        .unwrap();
    assert_eq!(
        store.refresh_clean_file(document, &PathBuf::from("/synthetic/other.rs"), file("new")),
        Err(EditorError::InvalidIdentity)
    );
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "abcdef"
    );
    store
        .refresh_clean_file(document, &path, file("한"))
        .unwrap();
    let clean = store.documents().snapshot(document).unwrap();
    assert!(!clean.dirty);
    assert_eq!(clean.rope.to_string(), "한");
    assert_eq!(
        store.views().get(first).unwrap().selection,
        caret("한".len())
    );
    assert!(store.views().get(first).unwrap().folds.is_empty());
    assert!(store.views().get(second).unwrap().composition.is_none());
    let revision = edit(&mut store, document, first, 0..0, "dirty", 0);
    assert_eq!(
        store.refresh_clean_file(document, &path, file("replace")),
        Err(EditorError::UnsavedChanges)
    );
    let dirty = store.documents().snapshot(document).unwrap();
    assert_eq!(dirty.revision, revision);
    assert_eq!(dirty.rope.to_string(), "dirty한");
    assert!(store.undo(document).unwrap());
    assert!(store.documents().snapshot(document).unwrap().dirty);
    assert!(!store.undo(document).unwrap());
}

#[test]
fn mirror_복원은_disk_baseline을_유지하고_용량_실패와_공유_문서를_원자적으로_보호한다() {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_COUNT,
        max_views: VIEW_COUNT,
        max_undo_groups: HISTORY_COUNT,
        max_document_bytes: RESTORE_BYTE_LIMIT,
    })
    .unwrap();
    let key = PathBuf::from("/synthetic/main.rs");
    assert_eq!(
        store.open_file_with_draft(key.clone(), file("abc"), Some("too long draft")),
        Err(EditorError::Capacity)
    );
    assert!(store.documents().is_empty());
    let document = store
        .open_file_with_draft(key.clone(), file("abc"), Some("draft"))
        .unwrap();
    let restored = store.documents().snapshot(document).unwrap();
    assert_eq!(restored.rope.to_string(), "draft");
    assert!(restored.dirty);
    assert_eq!(
        store.open_file_with_draft(key, file("external disk"), Some("too long obsolete draft")),
        Ok(document)
    );
    assert_eq!(
        store.documents().snapshot(document).unwrap().rope,
        restored.rope
    );
    let saved = store.save_snapshot(document).unwrap();
    assert!(store.mark_saved(saved, None).unwrap());
}

#[test]
fn 명시적_폐기는_공유_view와_stale_revision을_보호하고_승인한_문서만_회수한다() {
    let mut store = store();
    let document = open(&mut store, "abc");
    let primary = view(&mut store, document, "main");
    let second = view(&mut store, document, "auxiliary");
    let revision = edit(&mut store, document, primary, 0..0, "draft", 0);
    assert_eq!(
        store.discard_document(document, revision),
        Err(EditorError::AttachedViews)
    );
    store.detach_view(primary).unwrap();
    assert_eq!(
        store.discard_document(document, revision),
        Err(EditorError::AttachedViews)
    );
    store.detach_view(second).unwrap();
    assert_eq!(
        store.release_document(document),
        Err(EditorError::UnsavedChanges)
    );
    assert_eq!(
        store.discard_document(document, revision - 1),
        Err(EditorError::StaleRevision)
    );
    assert!(store.documents().snapshot(document).unwrap().dirty);
    store.discard_document(document, revision).unwrap();
    assert!(store.documents().is_empty());
    assert!(
        store
            .documents()
            .find(&DocumentKey::File(PathBuf::from("/synthetic/main.rs")))
            .is_none()
    );
}

fn store() -> EditorStore {
    EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_COUNT,
        max_views: VIEW_COUNT,
        max_undo_groups: HISTORY_COUNT,
        max_document_bytes: LARGE_FILE_BYTES as usize,
    })
    .unwrap()
}

fn file(content: &str) -> OpenedFile {
    OpenedFile {
        path: "/synthetic/main.rs".into(),
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

fn open(store: &mut EditorStore, content: &str) -> DocumentId {
    store
        .open_file(PathBuf::from("/synthetic/main.rs"), file(content))
        .unwrap()
}

fn view(store: &mut EditorStore, document: DocumentId, window: &str) -> ViewId {
    store
        .attach_view(
            ViewKey {
                window: window.into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap()
}

fn caret(byte: usize) -> SelectionSet {
    SelectionSet {
        primary: 0,
        selections: vec![Selection {
            anchor: byte,
            head: byte,
        }],
    }
}

fn edit(
    store: &mut EditorStore,
    document: DocumentId,
    view: ViewId,
    bytes: std::ops::Range<usize>,
    text: &str,
    group: u64,
) -> u64 {
    let revision = store.documents().snapshot(document).unwrap().revision;
    store
        .apply(
            document,
            Transaction {
                revision,
                edits: vec![Edit {
                    bytes,
                    text: text.into(),
                }],
                group: UndoGroup(group),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap()
}

#[test]
fn 공유_문서는_분할별_선택과_scroll을_분리하고_저장중_편집과_undo를_보존한다() {
    let mut store = store();
    let document = open(&mut store, "abc");
    let first = view(&mut store, document, "main");
    let second = view(&mut store, document, "auxiliary");
    store
        .set_view_state(
            first,
            caret(1),
            ScrollPosition {
                x: 0.0,
                y: SCROLL_Y,
            },
            Vec::new(),
        )
        .unwrap();
    store
        .set_view_state(
            second,
            caret("abc".len()),
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    edit(&mut store, document, first, 1..1, "한", 0);
    assert_eq!(
        store.views().get(second).unwrap().selection,
        caret("a한bc".len())
    );
    assert_eq!(store.views().get(first).unwrap().scroll.y, SCROLL_Y);
    assert_eq!(open(&mut store, "stale disk text"), document);
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "a한bc"
    );
    let pending = store.save_snapshot(document).unwrap();
    edit(
        &mut store,
        document,
        first,
        "a한bc".len().."a한bc".len(),
        "!",
        1,
    );
    assert_eq!(pending.rope().to_string(), "a한bc");
    assert!(!store.mark_saved(pending, Some(FIRST_SAVE_TIME)).unwrap());
    assert!(store.undo(document).unwrap());
    assert!(store.documents().snapshot(document).unwrap().dirty);
    assert!(store.redo(document).unwrap());
    assert!(store.documents().snapshot(document).unwrap().dirty);
    assert_eq!(
        store.release_document(document),
        Err(EditorError::AttachedViews)
    );
    store.detach_view(first).unwrap();
    store.detach_view(second).unwrap();
    assert_eq!(
        store.release_document(document),
        Err(EditorError::UnsavedChanges)
    );
    let pending = store.save_snapshot(document).unwrap();
    assert!(store.mark_saved(pending, Some(FIRST_SAVE_TIME)).unwrap());
    store.release_document(document).unwrap();
    assert!(store.documents().is_empty());
    let next = open(&mut store, "new disk");
    assert_ne!(next, document);
}

#[test]
fn 원자적_transaction은_stale_unicode_readonly_상한과_잘못된_선택을_거절한다() {
    let mut store = store();
    let document = open(&mut store, "a한b");
    let first = view(&mut store, document, "main");
    for (revision, bytes, selection, expected) in [
        (1, 0..0, None, EditorError::StaleRevision),
        (0, 2..2, None, EditorError::InvalidBoundary),
        (0, 0..0, Some(caret(2)), EditorError::InvalidBoundary),
    ] {
        let result = store.apply(
            document,
            Transaction {
                revision,
                edits: vec![Edit {
                    bytes,
                    text: String::new(),
                }],
                group: UndoGroup(0),
                origin: Some(first),
                selection_after: selection,
            },
        );
        assert_eq!(result, Err(expected));
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            "a한b"
        );
        assert_eq!(store.documents().snapshot(document).unwrap().revision, 0);
    }
    let mut readonly = file("lossy source");
    readonly.encoding_lossy = true;
    let document = store
        .open_file(PathBuf::from("/synthetic/lossy.rs"), readonly)
        .unwrap();
    assert!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .metadata
            .read_only
    );
    let first = view(&mut store, document, "main");
    assert_eq!(
        store.apply(
            document,
            Transaction {
                revision: 0,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: "changed".into()
                }],
                group: UndoGroup(0),
                origin: Some(first),
                selection_after: None
            }
        ),
        Err(EditorError::ReadOnly)
    );
    let mut refused = file("");
    refused.tier = FileSizeTier::Refused;
    assert_eq!(
        store.open_file(PathBuf::from("/synthetic/refused.rs"), refused),
        Err(EditorError::Refused)
    );
}

#[test]
fn 문서_초안과_인접_edit의_오른쪽_선택_경계를_보존한다() {
    let mut store = store();
    let draft = store
        .open_untitled(TabId::new(), "restored draft", "plaintext".into())
        .unwrap();
    assert!(store.documents().snapshot(draft).unwrap().dirty);
    let document = open(&mut store, "abc");
    let first = view(&mut store, document, "main");
    store
        .set_view_state(first, caret(1), ScrollPosition::default(), Vec::new())
        .unwrap();
    store
        .apply(
            document,
            Transaction {
                revision: 0,
                edits: vec![
                    Edit {
                        bytes: 0..1,
                        text: "X".into(),
                    },
                    Edit {
                        bytes: 1..1,
                        text: "한".into(),
                    },
                ],
                group: UndoGroup(0),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
    assert_eq!(
        store.views().get(first).unwrap().selection,
        caret("X한".len())
    );
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "X한bc"
    );
}

#[test]
fn preedit은_view에만_남고_undo_group과_저장_완료의_revision을_지킨다() {
    let mut store = store();
    let document = open(&mut store, "");
    let first = view(&mut store, document, "main");
    let second = view(&mut store, document, "auxiliary");
    store
        .set_composition(
            first,
            Some(Composition {
                revision: 0,
                replace: 0..0,
                preedit: "ㅎ".into(),
            }),
        )
        .unwrap();
    assert!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string()
            .is_empty()
    );
    assert!(store.views().get(second).unwrap().composition.is_none());
    edit(&mut store, document, first, 0..0, "한", 0);
    edit(&mut store, document, first, "한".len().."한".len(), "글", 0);
    assert!(store.views().get(first).unwrap().composition.is_none());
    assert!(store.undo(document).unwrap());
    assert!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string()
            .is_empty()
    );
    assert!(store.redo(document).unwrap());
    let older = store.save_snapshot(document).unwrap();
    edit(
        &mut store,
        document,
        first,
        "한글".len().."한글".len(),
        "!",
        1,
    );
    let newest = store.save_snapshot(document).unwrap();
    store.mark_saved(newest, Some(FIRST_SAVE_TIME)).unwrap();
    assert_eq!(
        store.mark_saved(older, Some(1.0)),
        Err(EditorError::StaleSave)
    );
    assert!(!store.documents().snapshot(document).unwrap().dirty);
    let key = DocumentKey::File(PathBuf::from("/synthetic/main.rs"));
    assert_eq!(store.documents().find(&key), Some(document));
}

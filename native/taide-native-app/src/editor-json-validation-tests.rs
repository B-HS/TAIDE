use super::*;

use taide_model::file::{EditorConfigOptions, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{Edit, UndoGroup};
use taide_native_editor::store::{EditorLimits, Transaction};
use taide_native_editor::view::ViewKey;

const TEXT: &str = "/*note*/{\"\u{1f600}\":1,}";
const DEADLINE: Duration = Duration::from_secs(5);
const BYTE_LIMIT: usize = 1024;

fn fixture() -> (EditorStore, DocumentId, taide_native_editor::view::ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 2,
        max_views: 2,
        max_undo_groups: 2,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let tab = TabId::new();
    let id = store
        .open_untitled(tab.clone(), TEXT, "json".into())
        .unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "json-validation-test".into(),
                pane: PaneId::new(),
                tab,
            },
            id,
        )
        .unwrap();
    (store, id, view)
}

#[test]
fn json_구문검사는_원본_default의_주석과_끝쉼표_byte_오류와_취소를_보존한다() {
    let (store, id, _) = fixture();
    let mut snapshot = store.documents().snapshot(id).unwrap();
    snapshot.metadata.read_only = true;
    let markers = build(&snapshot, &AtomicBool::new(false)).unwrap();
    assert_eq!(markers.len(), 2);
    assert!(
        markers
            .iter()
            .all(|marker| marker.message.severity
                == taide_native_editor::diagnostics::Severity::Error)
    );
    assert!(
        markers
            .iter()
            .any(|marker| &TEXT[marker.bytes.clone()] == "/*note*/"
                && marker.message.code.as_deref() == Some("521"))
    );
    assert!(
        markers
            .iter()
            .any(|marker| &TEXT[marker.bytes.clone()] == ","
                && marker.message.source.as_deref() == Some("json"))
    );
    assert_eq!(
        build(&snapshot, &AtomicBool::new(true)),
        Err(EditorError::StaleRevision)
    );
    assert_eq!(
        store.documents().snapshot(id).unwrap().rope.to_string(),
        TEXT
    );
}

#[tokio::test]
async fn json_구문작업은_변경을_지연하고_이전_응답과_닫힌_문서의_작업을_회수한다() {
    let (mut store, id, view) = fixture();
    let mut state = State::default();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let context = Context::default();
    let now = Instant::now();
    assert!(state.tick(&store, &tasks, &context, now).is_empty());
    store
        .apply(
            id,
            Transaction {
                revision: 0,
                edits: vec![Edit {
                    bytes: 0..TEXT.len(),
                    text: "{}".into(),
                }],
                group: UndoGroup(1),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
    assert!(state.tick(&store, &tasks, &context, now).is_empty());
    assert!(state.entries[&id].receiver.is_none());
    assert!(!state.entries[&id].complete);
    assert_eq!(state.entries[&id].ready_at, now + DEBOUNCE);
    let results = tokio::time::timeout(DEADLINE, async {
        loop {
            let completed = state.tick(&store, &tasks, &context, now + DEBOUNCE);
            if !completed.is_empty() {
                break completed;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0.revision, 1);
    assert!(results[0].1.as_ref().unwrap().is_empty());
    assert!(
        state
            .tick(&store, &tasks, &context, now + DEBOUNCE)
            .is_empty()
    );
    store.detach_view(view).unwrap();
    assert!(
        state
            .tick(&store, &tasks, &context, now + DEBOUNCE)
            .is_empty()
    );
    assert!(state.documents().is_empty());
    state.clear();
    tokio::time::timeout(DEADLINE, async {
        while tasks.tracked_count() != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(state.active.load(Ordering::Acquire), 0);
}

#[test]
fn json_구문검사는_메타schema_의미진단과_요청불가_schema경고를_파일과_untitled에_공급한다() {
    const META: &str = r#"{"$schema":"http://json-schema.org/draft-07/schema#","type":"foo"}"#;
    const REMOTE: &str = r#"{"$schema":"./local.schema.json","a":1,}"#;
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 2,
        max_views: 2,
        max_undo_groups: 2,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let file = store
        .open_file(
            "/workspace/한 글/schema.json".into(),
            OpenedFile {
                path: "/workspace/한 글/schema.json".into(),
                content: META.into(),
                encoding_lossy: false,
                language_id: "json".into(),
                tier: FileSizeTier::Normal,
                byte_size: u32::try_from(META.len()).unwrap(),
                line_count: 1,
                modified_ms: 0.0,
                read_only: true,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    let markers = build(
        &store.documents().snapshot(file).unwrap(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(markers.len(), 1);
    assert_eq!(&META[markers[0].bytes.clone()], "\"foo\"");
    assert_eq!(markers[0].message.code.as_deref(), Some("1"));
    assert_eq!(
        markers[0].message.severity,
        taide_native_editor::diagnostics::Severity::Warning
    );
    assert!(
        markers[0]
            .message
            .text
            .starts_with("Value is not accepted. Valid values: \"array\"")
    );
    let tab = TabId::new();
    let untitled = store
        .open_untitled(tab.clone(), REMOTE, "json".into())
        .unwrap();
    let markers = build(
        &store.documents().snapshot(untitled).unwrap(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(markers.len(), 2);
    assert_eq!(&REMOTE[markers[0].bytes.clone()], "\"./local.schema.json\"");
    assert_eq!(markers[0].message.code.as_deref(), Some("768"));
    assert!(
        markers[0]
            .message
            .text
            .ends_with("No schema request service available")
    );
    assert_eq!(markers[1].message.code.as_deref(), Some("519"));
    assert_eq!(
        store.documents().snapshot(file).unwrap().rope.to_string(),
        META
    );
}

use super::*;
use std::path::PathBuf;

use taide_lsp::native::protocol::lsp_types::{NumberOrString, Position, Range};
use taide_model::ids::TabId;
use taide_native_editor::store::{EditorLimits, EditorStore};

fn document(store: &mut EditorStore, path: &str) -> DocumentSnapshot {
    let id = store
        .open_untitled(TabId::new(), "synthetic", "rust".into())
        .unwrap();
    let mut snapshot = store.documents().snapshot(id).unwrap();
    snapshot.key = DocumentKey::File(PathBuf::from(path));
    snapshot
}

fn diagnostic(severity: Option<DiagnosticSeverity>, line: u32) -> Diagnostic {
    Diagnostic {
        severity,
        range: Range::new(Position::new(line, 0), Position::new(line, 1)),
        message: "synthetic diagnostic".into(),
        code: Some(NumberOrString::String("raw-code".into())),
        source: Some("synthetic-server".into()),
        data: Some(serde_json::json!({"quickfix": "raw-data"})),
        ..Default::default()
    }
}

#[test]
fn marker_store는_세션별_덮어쓰기와_폐기_모델수명_raw_진단을_보존한다() {
    let mut editor = EditorStore::new(EditorLimits {
        max_documents: 2,
        max_views: 2,
        max_undo_groups: 1,
        max_document_bytes: 1024,
    })
    .unwrap();
    let first = document(&mut editor, "/synthetic/one.rs");
    let second = document(&mut editor, "/synthetic/two.rs");
    let owner = Owner::new();
    let other = Owner::new();
    let mut store = Store::default();
    store.retain_documents(HashSet::from([first.id, second.id]));
    store.reconcile(HashMap::from([
        (owner, HashSet::from([first.id, second.id])),
        (other, HashSet::from([first.id])),
    ]));
    let errors = vec![
        diagnostic(None, 0),
        diagnostic(
            Some(serde_json::from_value(serde_json::json!(99)).unwrap()),
            1,
        ),
    ];
    store.publish(owner, &first, errors.clone());
    store.publish(
        other,
        &first,
        vec![diagnostic(Some(DiagnosticSeverity::WARNING), 0)],
    );
    store.publish(
        owner,
        &second,
        vec![
            diagnostic(Some(DiagnosticSeverity::INFORMATION), 0),
            diagnostic(Some(DiagnosticSeverity::HINT), 0),
        ],
    );
    assert_eq!(store.counts(), &[2, 1, 1, 1]);
    let revision = store.revision().clone();
    store.publish(owner, &first, errors.clone());
    assert!(Arc::ptr_eq(&revision, store.revision()));
    assert_eq!(
        store.batches[&(owner, first.id)].diagnostics[0].as_ref(),
        &errors[0]
    );
    store.reconcile(store.bindings.clone());
    assert!(Arc::ptr_eq(&revision, store.revision()));
    store.publish(owner, &first, Vec::new());
    assert_eq!(store.counts(), &[0, 1, 1, 1]);
    store.publish(owner, &first, errors.clone());
    store.reconcile(HashMap::from([(other, HashSet::from([first.id]))]));
    assert_eq!(store.counts(), &[0, 1, 0, 0]);
    store.publish(owner, &first, errors.clone());
    assert_eq!(store.counts(), &[0, 1, 0, 0]);
    let replacement = Owner::new();
    assert_ne!(replacement, owner);
    store.reconcile(HashMap::from([
        (other, HashSet::from([first.id])),
        (replacement, HashSet::from([first.id])),
    ]));
    store.publish(replacement, &first, errors.clone());
    assert_eq!(store.counts(), &[2, 1, 0, 0]);
    store.remove_document(first.id);
    assert_eq!(store.counts(), &[0, 0, 0, 0]);
    store.publish(other, &first, errors);
    assert_eq!(store.batches().count(), 0);
    store.retain_documents(HashSet::from([first.id]));
    store.publish(replacement, &first, vec![diagnostic(None, 0)]);
    assert_eq!(store.counts(), &[1, 0, 0, 0]);
    store.reconcile(HashMap::new());
    assert_eq!(store.counts(), &[0, 0, 0, 0]);
}

#[test]
fn 같은_raw의_새_revision_재발행은_이전_표시_좌표를_재사용하지_않는다() {
    let mut editor = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: 1024,
    })
    .unwrap();
    let first = document(&mut editor, "/synthetic/version.rs");
    let owner = Owner::new();
    let mut store = Store::default();
    store.retain_documents(HashSet::from([first.id]));
    store.reconcile(HashMap::from([(owner, HashSet::from([first.id]))]));
    let raw = vec![diagnostic(None, 0)];
    store.publish(owner, &first, raw.clone());
    let before = store.revision().clone();
    let mut next = first.clone();
    next.revision += 1;
    store.publish(owner, &next, raw);
    assert!(!Arc::ptr_eq(&before, store.revision()));
    assert_eq!(store.counts(), &[1, 0, 0, 0]);
}

#[test]
fn 표시_공급은_owner_원본과_편집_버전_언어_폐기를_분리한다() {
    use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
    use taide_native_editor::document::{Edit, UndoGroup};
    use taide_native_editor::store::Transaction;
    let mut editor = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: 1024,
    })
    .unwrap();
    let path = "/synthetic/markers.rs";
    let id = editor
        .open_file(
            path.into(),
            OpenedFile {
                path: path.into(),
                content: "abc def".into(),
                language_id: "rust".into(),
                byte_size: 7,
                line_count: 1,
                tier: FileSizeTier::Normal,
                read_only: false,
                encoding_lossy: false,
                modified_ms: 0.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    let snapshot = editor.documents().snapshot(id).unwrap();
    let first = Owner::new();
    let second = Owner::new();
    let mut store = Store::default();
    store.retain_documents(HashSet::from([id]));
    store.reconcile(HashMap::from([
        (first, HashSet::from([id])),
        (second, HashSet::from([id])),
    ]));
    let raw = diagnostic(None, 0);
    let invalid = Diagnostic {
        range: Range::new(Position::new(1, 0), Position::new(0, 0)),
        ..raw.clone()
    };
    store.publish(first, &snapshot, vec![raw.clone(), invalid]);
    store.publish(
        second,
        &snapshot,
        vec![diagnostic(Some(DiagnosticSeverity::WARNING), 0)],
    );
    let initial = store.display(&editor, &snapshot).unwrap();
    assert_eq!(initial.markers().len(), 2);
    assert!(Arc::ptr_eq(
        &initial,
        &store.display(&editor, &snapshot).unwrap()
    ));
    assert_eq!(store.counts(), &[2, 1, 0, 0]);
    assert_eq!(store.problems(&editor).len(), 2);
    assert_eq!(
        initial.markers()[0].message.code.as_deref(),
        Some("raw-code")
    );
    assert_eq!(store.batches[&(first, id)].diagnostics[0].data, raw.data);
    editor
        .apply(
            id,
            Transaction {
                revision: snapshot.revision,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: "X".into(),
                }],
                group: UndoGroup(0),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
    let current = editor.documents().snapshot(id).unwrap();
    let tracked = store.display(&editor, &current).unwrap();
    assert_eq!(tracked.markers()[0].bytes, 1..2);
    assert!(
        store
            .problems(&editor)
            .iter()
            .all(|problem| problem.marker.bytes == (1..2))
    );
    store.publish(first, &current, vec![raw.clone()]);
    let refreshed = store.display(&editor, &current).unwrap();
    assert_eq!(
        refreshed
            .markers()
            .iter()
            .filter(|marker| marker.bytes == (0..1))
            .count(),
        1
    );
    let mut language = current.clone();
    language.metadata.language_id = "python".into();
    assert!(
        store
            .display(&editor, &language)
            .unwrap()
            .markers()
            .is_empty()
    );
    assert_eq!(store.counts(), &[1, 1, 0, 0]);
    store.reconcile(HashMap::from([(second, HashSet::from([id]))]));
    assert_eq!(store.display(&editor, &current).unwrap().markers().len(), 1);
    store.remove_document(id);
    assert!(
        store
            .display(&editor, &current)
            .unwrap()
            .markers()
            .is_empty()
    );
    assert_eq!(store.counts(), &[0, 0, 0, 0]);
}

#[test]
fn 문제_이동_목록은_표시_500개_상한과_빈범위_단어_확장을_원본좌표와_분리한다() {
    use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
    const COUNT: usize = 501;
    let mut editor = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: 1024,
    })
    .unwrap();
    let path = "/synthetic/navigation.rs";
    let id = editor
        .open_file(
            path.into(),
            OpenedFile {
                path: path.into(),
                content: "abc def".into(),
                language_id: "rust".into(),
                byte_size: 7,
                line_count: 1,
                tier: FileSizeTier::Normal,
                read_only: false,
                encoding_lossy: false,
                modified_ms: 0.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    let snapshot = editor.documents().snapshot(id).unwrap();
    let owner = Owner::new();
    let mut store = Store::default();
    store.retain_documents(HashSet::from([id]));
    store.reconcile(HashMap::from([(owner, HashSet::from([id]))]));
    let raw = Diagnostic {
        range: Range::new(Position::new(0, 2), Position::new(0, 2)),
        ..diagnostic(None, 0)
    };
    let mut diagnostics = vec![raw; COUNT];
    diagnostics.push(diagnostic(Some(DiagnosticSeverity::HINT), 0));
    store.publish(owner, &snapshot, diagnostics);
    assert_eq!(
        store.display(&editor, &snapshot).unwrap().markers().len(),
        MAX_DISPLAY_MARKERS
    );
    let problems = store.problems(&editor);
    assert_eq!(problems.len(), COUNT);
    assert!(
        problems
            .iter()
            .all(|problem| problem.marker.bytes == (2..2) && problem.initial_range == (0..3))
    );
    assert!(
        problems
            .iter()
            .all(|problem| problem.resource == "file:///synthetic/navigation.rs")
    );
    store.remove_document(id);
    assert!(store.problems(&editor).is_empty());
}

#[test]
fn builtin_json_진단은_실제_untitled_키와_native_슬롯으로_표시하고_외부_회수와_독립이다() {
    let mut editor = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: 1024,
    })
    .unwrap();
    let tab = TabId::new();
    let id = editor
        .open_untitled(tab.clone(), "\u{1f600}bad", "json".into())
        .unwrap();
    let snapshot = editor.documents().snapshot(id).unwrap();
    let marker = Marker {
        bytes: 4..7,
        message: Arc::new(Message {
            severity: Severity::Error,
            text: "invalid JSON".into(),
            source: Some("json".into()),
            code: Some("test".into()),
        }),
    };
    let mut store = Store::default();
    store.retain_builtin(&editor, HashSet::from([id]));
    store
        .publish_builtin(&snapshot, vec![marker.clone()], "Untitled-1".into())
        .unwrap();
    assert_eq!(store.counts(), &[1, 0, 0, 0]);
    assert!(store.bindings.is_empty());
    assert!(store.batches.is_empty());
    let batch = store.batches().next().unwrap();
    assert_eq!(batch.key, DocumentKey::Untitled(tab.clone()));
    assert_eq!(batch.diagnostics[0].range.start, Position::new(0, 2));
    assert_eq!(batch.title, "Untitled-1");
    assert_eq!(
        store.display(&editor, &snapshot).unwrap().markers(),
        &[marker.clone()]
    );
    assert_eq!(
        store.problems(&editor)[0].resource,
        format!("untitled:{tab}")
    );
    store.reconcile(HashMap::new());
    assert_eq!(store.counts(), &[1, 0, 0, 0]);
    let invalid = Marker {
        bytes: 1..7,
        ..marker.clone()
    };
    assert_eq!(
        store.publish_builtin(&snapshot, vec![invalid], "Untitled-1".into()),
        Err(EditorError::InvalidBoundary)
    );
    assert_eq!(store.counts(), &[1, 0, 0, 0]);
    store.remove_document(id);
    store
        .publish_builtin(&snapshot, vec![marker], "Untitled-1".into())
        .unwrap();
    assert_eq!(
        store.counts(),
        &[0, 0, 0, 0],
        "removed document must reject late native publication"
    );
    store.retain_builtin(&editor, HashSet::new());
    assert_eq!(store.counts(), &[0, 0, 0, 0]);
    assert!(store.problems(&editor).is_empty());
    assert!(
        store
            .display(&editor, &snapshot)
            .unwrap()
            .markers()
            .is_empty()
    );
}

#[test]
fn builtin_json과_외부_진단은_같은_문서에서_공존하고_각_공급자만_회수한다() {
    let mut editor = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: 1024,
    })
    .unwrap();
    let path = PathBuf::from("/synthetic/native-and-external.json");
    let id = editor
        .open_file(
            path.clone(),
            taide_model::file::OpenedFile {
                path: path.to_string_lossy().into_owned(),
                content: "true".into(),
                language_id: "json".into(),
                byte_size: 4,
                line_count: 1,
                tier: taide_model::file::FileSizeTier::Normal,
                read_only: false,
                encoding_lossy: false,
                modified_ms: 0.0,
                editor_config: Default::default(),
            },
        )
        .unwrap();
    let snapshot = editor.documents().snapshot(id).unwrap();
    let owner = Owner::new();
    let mut store = Store::default();
    store.retain_documents(HashSet::from([id]));
    store.reconcile(HashMap::from([(owner, HashSet::from([id]))]));
    store.retain_builtin(&editor, HashSet::from([id]));
    store.publish(
        owner,
        &snapshot,
        vec![diagnostic(Some(DiagnosticSeverity::WARNING), 0)],
    );
    store
        .publish_builtin(
            &snapshot,
            vec![Marker {
                bytes: 0..4,
                message: Arc::new(Message {
                    severity: Severity::Error,
                    text: "native".into(),
                    source: Some("json".into()),
                    code: None,
                }),
            }],
            "native-and-external.json".into(),
        )
        .unwrap();
    assert_eq!(store.counts(), &[1, 1, 0, 0]);
    assert_eq!(
        store.display(&editor, &snapshot).unwrap().markers().len(),
        2
    );
    assert_eq!(store.problems(&editor).len(), 2);
    store.retain_builtin(&editor, HashSet::new());
    assert_eq!(store.counts(), &[0, 1, 0, 0]);
    assert_eq!(store.problems(&editor).len(), 1);
    store.reconcile(HashMap::new());
    assert_eq!(store.counts(), &[0, 0, 0, 0]);
}

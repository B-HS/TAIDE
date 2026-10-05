use super::*;

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

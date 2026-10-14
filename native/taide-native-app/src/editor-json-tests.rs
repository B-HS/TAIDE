use super::*;

use std::time::Duration;
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{Edit, UndoGroup};
use taide_native_editor::store::{EditorLimits, Transaction};
use taide_native_editor::view::{ViewId, ViewKey};

const DEADLINE: Duration = Duration::from_secs(5);
const TEXT: &str = "{\n  \"\u{1f600}\": {\n    \"value\": 1\n  },\n  \"list\": [1, 2]\n}";

fn fixture(text: &str) -> (EditorStore, DocumentId, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 8,
        max_views: 8,
        max_undo_groups: 4,
        max_document_bytes: LARGE_FILE_BYTES as usize,
    })
    .unwrap();
    let tab = TabId::new();
    let document = store
        .open_untitled(tab.clone(), text, "json".into())
        .unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "native-json-test".into(),
                pane: PaneId::new(),
                tab,
            },
            document,
        )
        .unwrap();
    (store, document, view)
}

async fn settle(
    state: &mut State,
    store: &EditorStore,
    tasks: &TaskSupervisor,
) -> Vec<(DocumentSnapshot, Result<Models, EditorError>)> {
    let context = Context::default();
    tokio::time::timeout(DEADLINE, async {
        let mut results = Vec::new();
        loop {
            results.extend(state.tick(store, tasks, &context));
            if state.entries.values().all(|entry| entry.complete) {
                return results;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap()
}

async fn drained(tasks: &TaskSupervisor) {
    tokio::time::timeout(DEADLINE, async {
        while tasks.tracked_count() != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[test]
fn native_json은_실제_untitled와_읽기전용_snapshot의_byte_심볼과_접기를_생성한다() {
    let (store, id, _) = fixture(TEXT);
    let mut snapshot = store.documents().snapshot(id).unwrap();
    snapshot.metadata.read_only = true;
    let models = build(&snapshot, &AtomicBool::new(false)).unwrap();
    assert!(models.symbols.describes(&snapshot));
    assert_eq!(
        models
            .symbols
            .symbols()
            .iter()
            .map(|symbol| symbol.name.as_str())
            .collect::<Vec<_>>(),
        ["\u{1f600}", "list", "value", "0", "1"]
    );
    let value = models
        .symbols
        .symbols()
        .iter()
        .find(|symbol| symbol.name == "value")
        .unwrap();
    assert_eq!(value.parent, Some(0));
    assert_eq!(value.container_label, "\u{1f600}");
    assert_eq!(&TEXT[value.selection.clone()], "\"value\"");
    assert_eq!(
        models
            .folds
            .regions()
            .iter()
            .map(|region| (region.start_line, region.end_line))
            .collect::<Vec<_>>(),
        [(0, 4), (1, 2)]
    );
    assert!(models.symbols.sticky_model().unwrap().describes(&snapshot));
    let mut changed = snapshot.clone();
    changed.metadata.language_id = "plaintext".into();
    assert!(!models.symbols.describes(&changed));
    assert!(!models.folds.describes(&changed));
    assert!(matches!(
        build(&snapshot, &AtomicBool::new(true)),
        Err(EditorError::StaleRevision)
    ));
}

#[tokio::test]
async fn native_json은_내용변경과_뷰닫힘에서_이전_결과를_회수하고_project없이_공급한다() {
    let (mut store, id, view) = fixture(TEXT);
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let mut state = State::default();
    let context = Context::default();
    assert!(state.tick(&store, &tasks, &context).is_empty());
    store
        .apply(
            id,
            Transaction {
                revision: 0,
                edits: vec![Edit {
                    bytes: 0..TEXT.len(),
                    text: "{\n\"new\": []\n}".into(),
                }],
                group: UndoGroup(1),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
    let results = settle(&mut state, &store, &tasks).await;
    assert_eq!(results.len(), 1);
    let mut symbols = crate::editor_symbols::State::default();
    let mut folding = crate::editor_folding::State::default();
    for (current, result) in results {
        assert_eq!(current.revision, 1);
        let result = result.unwrap();
        symbols.install_builtin(&current, result.symbols).unwrap();
        folding.install_builtin(&current, result.folds);
    }
    let current = store.documents().snapshot(id).unwrap();
    let palette = symbols.palette(None, Some(&current));
    assert!(!palette.is_pending);
    assert_eq!(palette.entries.unwrap()[0].name, "new");
    assert!(folding.builtin_model(&current).is_some());
    store.detach_view(view).unwrap();
    assert!(state.tick(&store, &tasks, &context).is_empty());
    assert!(state.documents().is_empty());
    symbols.retain_builtin(&state.documents());
    folding.retain_builtin(&state.documents());
    assert!(symbols.builtin_model(&current).is_none());
    assert!(folding.builtin_model(&current).is_none());
    state.clear();
    drained(&tasks).await;
    assert_eq!(state.active.load(Ordering::Acquire), 0);
}

#[test]
fn native_json은_깊은_container_표시와_본문_상한을_검사하고_잘못된_심볼_byte를_거절한다() {
    let depth = 2048;
    let text = format!(
        "{}null{}",
        "{\"long_property_name\":".repeat(depth),
        "}".repeat(depth)
    );
    let (store, id, _) = fixture(&text);
    let snapshot = store.documents().snapshot(id).unwrap();
    assert!(matches!(
        build(&snapshot, &AtomicBool::new(false)),
        Err(EditorError::Capacity)
    ));
    let (store, id, _) = fixture(TEXT);
    let snapshot = store.documents().snapshot(id).unwrap();
    let model = build(&snapshot, &AtomicBool::new(false)).unwrap();
    let mut symbols = model.symbols.symbols().to_vec();
    symbols[0].selection.start = TEXT.find('\u{1f600}').unwrap() + 1;
    assert!(matches!(
        DocumentSymbols::from_symbols(&snapshot, symbols),
        Err(EditorError::InvalidBoundary)
    ));
}

#[test]
fn native_json은_외부_심볼의_우선순위와_접기_합성_및_팔레트_세대를_보존한다() {
    let (store, id, _) = fixture(TEXT);
    let snapshot = store.documents().snapshot(id).unwrap();
    let native = build(&snapshot, &AtomicBool::new(false)).unwrap();
    let value = native
        .symbols
        .symbols()
        .iter()
        .find(|symbol| symbol.name == "value")
        .unwrap()
        .clone();
    let external = Arc::new(
        DocumentSymbols::from_symbols(
            &snapshot,
            vec![Symbol {
                parent: None,
                container_label: String::new(),
                bytes: value.selection.clone(),
                ..value
            }],
        )
        .unwrap(),
    );
    let project = taide_model::ids::ProjectId::new();
    let provider = crate::editor_symbols::ProviderIdentity {
        owner: crate::diagnostics::Owner::new(),
        generation: 1,
        capability_revision: 0,
    };
    let providers = HashSet::from([provider]);
    let mut symbols = crate::editor_symbols::State::default();
    let native_sticky = native.symbols.sticky_model().unwrap().clone();
    symbols.install_builtin(&snapshot, native.symbols).unwrap();
    let native_generation = symbols.palette(None, Some(&snapshot)).generation;
    let request = symbols
        .observe(
            &project,
            snapshot.clone(),
            providers.clone(),
            std::time::Instant::now(),
        )
        .unwrap();
    assert!(
        symbols
            .accept(
                &request,
                &snapshot,
                providers.clone(),
                Ok(crate::editor_symbols::Response {
                    palette_provider: Some(provider),
                    palette: external.clone(),
                    groups: vec![crate::editor_symbols::Group {
                        provider,
                        model: external.clone()
                    }],
                    error: None,
                })
            )
            .unwrap()
    );
    let palette = symbols.palette(Some(&project), Some(&snapshot));
    assert_ne!(palette.generation, native_generation);
    assert_eq!(palette.entries.unwrap().len(), 1);
    assert!(
        symbols
            .position(&project, &snapshot, native_generation, 0)
            .is_none()
    );
    assert!(Arc::ptr_eq(
        &symbols.sticky(&project, &snapshot).unwrap(),
        &native_sticky
    ));
    let mut folding = crate::editor_folding::State::default();
    folding.install_builtin(&snapshot, native.folds);
    let request = folding
        .observe(
            &project,
            snapshot.clone(),
            providers.clone(),
            std::time::Instant::now(),
        )
        .unwrap();
    assert!(folding.accept(
        &request,
        &snapshot,
        providers,
        Ok(crate::editor_folding::Response {
            groups: vec![crate::editor_folding::Group {
                provider,
                ranges: Some(vec![taide_native_editor::syntax_folding::SyntaxFoldRange {
                    region: taide_native_editor::folding::FoldRegion {
                        start_line: 0,
                        end_line: 5
                    },
                    kind: Some("imports".into()),
                }])
            }]
        })
    ));
    let merged = folding.model(&project, &snapshot).unwrap();
    assert_eq!(merged.regions().len(), 2);
    assert_eq!(merged.regions()[0].end_line, 5);
    assert_eq!(merged.kind(0), Some("imports"));
    folding.retain_builtin(&HashSet::new());
    symbols.retain_builtin(&HashSet::new());
    assert_eq!(
        folding.model(&project, &snapshot).unwrap().regions().len(),
        1
    );
    assert_eq!(
        symbols
            .palette(Some(&project), Some(&snapshot))
            .entries
            .unwrap()
            .len(),
        1
    );
}

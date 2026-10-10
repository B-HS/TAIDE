use super::*;
use crate::editor_completion::{Context, Request, State};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{Edit, UndoGroup};
use taide_native_editor::documentation::Content;
use taide_native_editor::store::{EditorStore, Transaction};
use taide_native_editor::view::{ScrollPosition, Selection, SelectionSet, ViewId, ViewKey};
use tokio::sync::Notify;

const DEADLINE: Duration = Duration::from_secs(30);
const READY_DEADLINE: Duration = Duration::from_secs(10);

fn view(store: &mut EditorStore, document: DocumentId) -> ViewId {
    let view = store
        .attach_view(
            ViewKey {
                window: "synthetic".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    let text = store
        .documents()
        .snapshot(document)
        .unwrap()
        .rope
        .to_string();
    let head = text.find("method").unwrap() + "method".len();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: head, head }],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    view
}

async fn next(bridge: &mut LspBridge, signal: &Notify) -> Reply {
    tokio::time::timeout(READY_DEADLINE, async {
        loop {
            if let Some(reply) = bridge.poll() {
                return reply;
            }
            signal.notified().await;
        }
    })
    .await
    .expect("completion reply deadline")
}

async fn running(
    bridge: &mut LspBridge,
    signal: &Notify,
    fixture: &document_symbol_tests::Fixture,
) {
    let ready = tokio::time::timeout(READY_DEADLINE, async {
        loop {
            while let Some(reply) = bridge.poll() {
                if let Reply::Failed { error, .. } = reply {
                    panic!("completion initialization: {error}");
                }
            }
            if bridge.states.borrow().len() == 1
                && bridge
                    .states
                    .borrow()
                    .iter()
                    .all(|state| state.snapshot.phase == Phase::Running)
            {
                return;
            }
            signal.notified().await;
        }
    })
    .await;
    assert!(
        ready.is_ok(),
        "completion readiness: {:?}; stderr={}",
        bridge
            .states
            .borrow()
            .iter()
            .map(|state| (&state.name, &state.snapshot))
            .collect::<Vec<_>>(),
        std::fs::read_to_string(fixture.directory.join("stderr.log")).unwrap_or_default()
    );
}

fn begin(
    bridge: &LspBridge,
    state: &mut State,
    store: &EditorStore,
    project: &ProjectId,
    view: ViewId,
) -> Request {
    let snapshot = store
        .documents()
        .snapshot(store.views().get(view).unwrap().document)
        .unwrap();
    let text = snapshot.rope.to_string();
    let start = text.find("method").unwrap();
    let request = state
        .begin(
            store,
            Context {
                project: project.clone(),
                source: view,
                owner: view,
                word: start..start + "method".len(),
                viewport: eframe::egui::ViewportId::ROOT,
                automatic: false,
            },
            bridge.completion_providers(project, &snapshot),
        )
        .unwrap();
    bridge.completion(request.clone()).unwrap();
    request
}

async fn complete(
    bridge: &mut LspBridge,
    signal: &Notify,
    state: &mut State,
    store: &EditorStore,
    request: &Request,
) {
    loop {
        match next(bridge, signal).await {
            Reply::Completion {
                request: current,
                result,
            } if current.token == request.token => {
                let providers = bridge.completion_providers(&current.project, &current.snapshot);
                assert!(state.accept(store, &current, providers, result).unwrap());
                return;
            }
            Reply::Failed { error, .. } => panic!("completion failure: {error}"),
            _ => {}
        }
    }
}

async fn diagnostic(bridge: &mut LspBridge, signal: &Notify, message: &str) {
    loop {
        if let Reply::Diagnostics { diagnostics, .. } = next(bridge, signal).await
            && diagnostics
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message == message)
        {
            return;
        }
    }
}

#[tokio::test]
async fn 실제_child_completion의_list_array_스니펫_utf16_문서종류와_모르는kind를_소비한다() {
    tokio::time::timeout(DEADLINE, async {
        for mode in ["--native-completion", "--native-completion-array"] {
            let (fixture, project, mut store, id, bin) = document_symbol_tests::fixture(mode);
            let source = view(&mut store, id);
            let signal = Arc::new(Notify::new());
            let repaint = signal.clone();
            let mut bridge = LspBridge::connect(
                fixture.services.clone(),
                bin,
                Arc::new(move || repaint.notify_one()),
            )
            .unwrap();
            bridge
                .sync(project.clone(), store.documents().snapshot(id).unwrap())
                .unwrap();
            running(&mut bridge, &signal, &fixture).await;
            let snapshot = store.documents().snapshot(id).unwrap();
            let providers = bridge.completion_providers(&project, &snapshot);
            assert_eq!(providers.len(), 1);
            let options = bridge.completion_options(&project, &snapshot);
            assert_eq!(options.len(), 1);
            let provider = *providers.iter().next().unwrap();
            assert_eq!(
                options[&provider][0].trigger_characters.as_deref(),
                Some([".".to_owned(), ":".to_owned()].as_slice())
            );
            assert!(
                bridge
                    .completion_providers(&ProjectId::new(), &snapshot)
                    .is_empty()
            );
            let mut state = State::default();
            let request = begin(&bridge, &mut state, &store, &project, source);
            assert_eq!(request.position, lsp_types::Position::new(1, 10));
            complete(&mut bridge, &signal, &mut state, &store, &request).await;
            let groups = state.groups(&store, source).unwrap();
            assert_eq!(groups.len(), 1);
            let group = &groups[&0];
            assert_eq!(group.provider, provider);
            assert_eq!(
                group.candidates.is_incomplete,
                mode == "--native-completion"
            );
            assert_eq!(group.candidates.items.len(), 2);
            let item = &group.candidates.items[0];
            assert_eq!(item.item.label, "method");
            assert_eq!(
                item.insert,
                "class\n  \u{1f600}".len().."class\n  \u{1f600}method".len()
            );
            assert_eq!(item.text(), "method(${1|a,b|})$0");
            assert!(item.is_snippet());
            assert_eq!(item.item.data, Some("opaque".into()));
            assert_eq!(
                item.documentation(),
                Some(Content::Markdown("**method**".into()))
            );
            assert_eq!(
                group.candidates.items[1].item.kind,
                Some(lsp_types::CompletionItemKind::TEXT)
            );
            assert_eq!(
                group.candidates.items[1].documentation(),
                Some(Content::Plain("plain".into()))
            );
            state.clear();
            bridge.disconnect().await.unwrap();
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn 실제_child의_빈_null_오류_형식오류_미지원은_completion후보를_남기지_않는다() {
    tokio::time::timeout(DEADLINE, async {
        for mode in [
            "--native-completion-empty",
            "--native-completion-null",
            "--native-completion-error",
            "--native-completion-bad",
            "--native-completion-unsupported",
        ] {
            let (fixture, project, mut store, id, bin) = document_symbol_tests::fixture(mode);
            let source = view(&mut store, id);
            let signal = Arc::new(Notify::new());
            let repaint = signal.clone();
            let mut bridge = LspBridge::connect(
                fixture.services.clone(),
                bin,
                Arc::new(move || repaint.notify_one()),
            )
            .unwrap();
            bridge
                .sync(project.clone(), store.documents().snapshot(id).unwrap())
                .unwrap();
            running(&mut bridge, &signal, &fixture).await;
            let mut state = State::default();
            let request = begin(&bridge, &mut state, &store, &project, source);
            complete(&mut bridge, &signal, &mut state, &store, &request).await;
            assert!(
                state
                    .groups(&store, source)
                    .unwrap()
                    .values()
                    .all(|group| group.candidates.items.is_empty()),
                "{mode}"
            );
            assert!(!state.pending(&store, source));
            state.clear();
            bridge.disconnect().await.unwrap();
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn 실제_completion_편집취소는_child_cancel과_늦은응답거절_종료task회수를_보존한다() {
    tokio::time::timeout(DEADLINE, async {
        let (fixture, project, mut store, id, bin) =
            document_symbol_tests::fixture("--native-completion-wait");
        let source = view(&mut store, id);
        let signal = Arc::new(Notify::new());
        let repaint = signal.clone();
        let mut bridge = LspBridge::connect(
            fixture.services.clone(),
            bin,
            Arc::new(move || repaint.notify_one()),
        )
        .unwrap();
        bridge
            .sync(project.clone(), store.documents().snapshot(id).unwrap())
            .unwrap();
        running(&mut bridge, &signal, &fixture).await;
        let mut state = State::default();
        let request = begin(&bridge, &mut state, &store, &project, source);
        diagnostic(&mut bridge, &signal, "synthetic completion held").await;
        let byte = store.views().get(source).unwrap().selection.selections[0].head;
        store
            .apply(
                id,
                Transaction {
                    revision: 0,
                    edits: vec![Edit {
                        bytes: byte..byte,
                        text: "s".into(),
                    }],
                    group: UndoGroup(0),
                    origin: Some(source),
                    selection_after: None,
                },
            )
            .unwrap();
        state.reconcile(
            &store,
            &HashSet::from([project]),
            |_| true,
            |project, document| bridge.completion_providers(project, document),
        );
        assert!(request.is_cancelled());
        diagnostic(&mut bridge, &signal, "synthetic completion cancelled").await;
        assert!(state.groups(&store, source).is_none());
        bridge.disconnect().await.unwrap();
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}

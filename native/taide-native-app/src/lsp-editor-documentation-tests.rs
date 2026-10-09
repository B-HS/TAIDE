use super::*;
use crate::editor_documentation::{Context, Kind, Payload, Request, Response, State};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{Edit, UndoGroup};
use taide_native_editor::store::{EditorStore, Transaction};
use taide_native_editor::view::{ViewId, ViewKey};
use tokio::sync::Notify;

const DEADLINE: Duration = Duration::from_secs(15);
const READY_DEADLINE: Duration = Duration::from_secs(10);

fn view(store: &mut EditorStore, document: DocumentId) -> ViewId {
    store
        .attach_view(
            ViewKey {
                window: "synthetic".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap()
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
    .expect("documentation reply deadline")
}

async fn running(
    bridge: &mut LspBridge,
    signal: &Notify,
    fixture: &document_symbol_tests::Fixture,
    count: usize,
) {
    let ready = tokio::time::timeout(READY_DEADLINE, async {
        loop {
            while let Some(reply) = bridge.poll() {
                if let Reply::Failed { error, .. } = reply {
                    panic!("{error}");
                }
            }
            if bridge.states.borrow().len() == count
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
        "documentation readiness: {:?}; stderr={}",
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
    kind: Kind,
) -> Request {
    let snapshot = store
        .documents()
        .snapshot(store.views().get(view).unwrap().document)
        .unwrap();
    let text = snapshot.rope.to_string();
    let byte = text.find("method").unwrap();
    let request = state
        .begin(
            store,
            Context {
                project: project.clone(),
                source: view,
                owner: view,
                kind,
                byte,
                fallback: byte..byte + "method".len(),
                viewport: eframe::egui::ViewportId::ROOT,
                keyboard: false,
            },
            bridge.documentation_providers(project, &snapshot, kind),
        )
        .unwrap();
    bridge.documentation(request.clone()).unwrap();
    request
}

async fn complete(
    bridge: &mut LspBridge,
    signal: &Notify,
    state: &mut State,
    store: &EditorStore,
    request: &Request,
) -> usize {
    let mut partial = 0;
    loop {
        match next(bridge, signal).await {
            Reply::Documentation {
                request: current,
                result,
            } => {
                assert_eq!(current.token, request.token);
                let complete = match &result {
                    Ok(Response::Hover { complete, .. }) => *complete,
                    Ok(Response::Signature(_)) | Err(_) => true,
                };
                if !complete {
                    partial += 1;
                }
                let providers = bridge.documentation_providers(
                    &current.project,
                    &current.snapshot,
                    current.kind,
                );
                assert!(state.accept(store, &current, providers, result).unwrap());
                if complete {
                    return partial;
                }
            }
            Reply::Failed { error, .. } => panic!("documentation failure: {error}"),
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
async fn 실제_child의_hover_부분응답과_signature_utf16_문서종류_trigger를_소비한다() {
    tokio::time::timeout(DEADLINE, async {
        let (fixture, project, mut store, id, bin) = document_symbol_tests::fixture("--native-documentation");
        let source = view(&mut store, id);
        let signal = Arc::new(Notify::new());
        let repaint = signal.clone();
        let mut bridge = LspBridge::connect(fixture.services.clone(), bin, Arc::new(move || repaint.notify_one())).unwrap();
        bridge.sync(project.clone(), store.documents().snapshot(id).unwrap()).unwrap();
        running(&mut bridge, &signal, &fixture, 1).await;
        let snapshot = store.documents().snapshot(id).unwrap();
        let triggers = bridge.signature_triggers(&project, &snapshot);
        assert!(triggers.matches("(", false));
        assert!(triggers.matches(",", false));
        assert!(triggers.matches(")", true));
        assert!(!triggers.matches(")", false));
        let mut state = State::default();
        let hover = begin(&bridge, &mut state, &store, &project, source, Kind::Hover);
        assert_eq!(hover.position, lsp_types::Position::new(1, 4));
        assert_eq!(complete(&mut bridge, &signal, &mut state, &store, &hover).await, 1);
        let Some(Payload::Hover(parts)) = state.payload(&store, source, Kind::Hover) else { panic!("expected hover") };
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].range, hover.fallback);
        assert_eq!(parts[0].documents.len(), 2);
        assert!(matches!(&parts[0].documents[1].blocks[0], taide_native_editor::documentation::Block::Code { language, .. } if language == "rust"));
        let signature = begin(&bridge, &mut state, &store, &project, source, Kind::Signature);
        complete(&mut bridge, &signal, &mut state, &store, &signature).await;
        let Some(Payload::Signature(signature)) = state.payload(&store, source, Kind::Signature) else { panic!("expected signature") };
        assert_eq!(signature.model.parameter_range(), Some(2..6));
        assert_eq!(signature.model.value().signatures.len(), 2);
        assert!(signature.parameter_documentation().is_some());
        assert!(state.cycle(&store, source, true, true));
        bridge.disconnect().await.unwrap();
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    }).await.unwrap();
}

#[tokio::test]
async fn 실제_빈_null_오류_형식오류와_미지원은_호버와_서명에_격리된다() {
    tokio::time::timeout(DEADLINE, async {
        for mode in [
            "--native-documentation-empty",
            "--native-documentation-null",
            "--native-documentation-error",
            "--native-documentation-bad",
            "--native-documentation-unsupported",
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
            running(&mut bridge, &signal, &fixture, 1).await;
            let mut state = State::default();
            for kind in [Kind::Hover, Kind::Signature] {
                let request = begin(&bridge, &mut state, &store, &project, source, kind);
                assert_eq!(
                    complete(&mut bridge, &signal, &mut state, &store, &request).await,
                    0
                );
                assert!(!state.pending(&store, source, kind));
                match state.payload(&store, source, kind) {
                    Some(Payload::Hover(parts)) => assert!(parts.is_empty()),
                    None => assert_eq!(kind, Kind::Signature),
                    _ => panic!("unexpected documentation"),
                }
            }
            bridge.disconnect().await.unwrap();
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn 실제_보류_요청은_다른_mirror를_막지않고_편집과_뷰닫힘에_cancel을_전송한다() {
    tokio::time::timeout(DEADLINE, async {
        let (fixture, project, mut store, id, bin) =
            document_symbol_tests::fixture("--native-documentation-wait");
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
        running(&mut bridge, &signal, &fixture, 1).await;
        let mut state = State::default();
        let hover = begin(&bridge, &mut state, &store, &project, source, Kind::Hover);
        diagnostic(&mut bridge, &signal, "synthetic documentation held").await;
        let root = fixture.services.state.projects.read()[&project]
            .root
            .clone();
        let path = std::path::Path::new(&root).join("other.rs");
        std::fs::write(&path, "other mirror").unwrap();
        let file = taide_file::service::open_file(&path, &[], false).unwrap();
        let other = store.open_file(path, file).unwrap();
        bridge
            .sync(project.clone(), store.documents().snapshot(other).unwrap())
            .unwrap();
        loop {
            if let Reply::Synced { document, .. } = next(&mut bridge, &signal).await
                && document == other
            {
                break;
            }
        }
        assert!(!hover.is_cancelled());
        store
            .apply(
                id,
                Transaction {
                    revision: 0,
                    edits: vec![Edit {
                        bytes: 0..0,
                        text: "edited ".into(),
                    }],
                    group: UndoGroup(0),
                    origin: Some(source),
                    selection_after: None,
                },
            )
            .unwrap();
        state.reconcile(
            &store,
            &HashSet::from([project.clone()]),
            |_| true,
            |project, snapshot, kind| bridge.documentation_providers(project, snapshot, kind),
        );
        assert!(hover.is_cancelled());
        diagnostic(&mut bridge, &signal, "synthetic documentation cancelled").await;
        bridge
            .sync(project.clone(), store.documents().snapshot(id).unwrap())
            .unwrap();
        loop {
            if let Reply::Synced {
                document, revision, ..
            } = next(&mut bridge, &signal).await
                && document == id
                && revision == 1
            {
                break;
            }
        }
        let signature = begin(
            &bridge,
            &mut state,
            &store,
            &project,
            source,
            Kind::Signature,
        );
        diagnostic(&mut bridge, &signal, "synthetic documentation held").await;
        store.detach_view(source).unwrap();
        state.reconcile(
            &store,
            &HashSet::from([project]),
            |_| true,
            |project, snapshot, kind| bridge.documentation_providers(project, snapshot, kind),
        );
        assert!(signature.is_cancelled());
        diagnostic(&mut bridge, &signal, "synthetic documentation cancelled").await;
        bridge.disconnect().await.unwrap();
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn 실제_복수provider는_호버_우선순위와_첫서명_빈서명_부분오류_프로젝트격리를_보존한다() {
    use std::os::unix::fs::PermissionsExt;
    const EXECUTABLE_MODE: u32 = 0o700;
    tokio::time::timeout(DEADLINE, async {
        for mode in [
            "--native-documentation-alternate",
            "--native-documentation-error",
            "--native-documentation-null",
            "--native-documentation-empty",
        ] {
            let (fixture, project, mut store, id, bin) =
                document_symbol_tests::fixture("--native-documentation");
            let DocumentKey::File(path) = &store.documents().snapshot(id).unwrap().key else {
                panic!("file fixture")
            };
            let path = path.parent().unwrap().join("source.py");
            std::fs::write(&path, "class\n  \u{1f600}method\nend").unwrap();
            let file = taide_file::service::open_file(&path, &[], false).unwrap();
            let id = store.open_file(path, file).unwrap();
            let source = view(&mut store, id);
            let snapshot = store.documents().snapshot(id).unwrap();
            let mock = std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("examples/native-lsp-mock")
                .to_str()
                .unwrap()
                .replace('\'', "'\\''");
            for (binary, mode) in [
                ("basedpyright-langserver", "--native-documentation"),
                ("ruff", mode),
            ] {
                let path = std::path::PathBuf::from(&bin).join(binary);
                std::fs::write(&path, format!("#!/bin/sh\nexec '{mock}' {mode}\n")).unwrap();
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(EXECUTABLE_MODE))
                    .unwrap();
            }
            let other = ProjectId::new();
            let mut entry = fixture.services.state.projects.read()[&project].clone();
            entry.id = other.clone();
            fixture
                .services
                .state
                .projects
                .write()
                .insert(other.clone(), entry);
            let signal = Arc::new(Notify::new());
            let repaint = signal.clone();
            let mut bridge = LspBridge::connect(
                fixture.services.clone(),
                bin,
                Arc::new(move || repaint.notify_one()),
            )
            .unwrap();
            bridge.sync(project.clone(), snapshot.clone()).unwrap();
            bridge.sync(other.clone(), snapshot.clone()).unwrap();
            running(&mut bridge, &signal, &fixture, 4).await;
            let providers = bridge.documentation_providers(&project, &snapshot, Kind::Hover);
            let foreign = bridge.documentation_providers(&other, &snapshot, Kind::Hover);
            assert_eq!(providers.len(), 2);
            assert_eq!(foreign.len(), 2);
            assert!(providers.is_disjoint(&foreign));
            let mut state = State::default();
            let hover = begin(&bridge, &mut state, &store, &project, source, Kind::Hover);
            let alternate = mode == "--native-documentation-alternate";
            assert_eq!(
                complete(&mut bridge, &signal, &mut state, &store, &hover).await,
                if alternate { 2 } else { 1 }
            );
            let Some(Payload::Hover(parts)) = state.payload(&store, source, Kind::Hover) else {
                panic!("expected hover")
            };
            assert_eq!(parts.len(), if alternate { 2 } else { 1 });
            let taide_native_editor::documentation::Block::Code { text, .. } =
                &parts[0].documents[1].blocks[0]
            else {
                panic!("expected provider code")
            };
            assert!(text.contains(if alternate { "alternate" } else { "primary" }));
            let signature = begin(
                &bridge,
                &mut state,
                &store,
                &project,
                source,
                Kind::Signature,
            );
            complete(&mut bridge, &signal, &mut state, &store, &signature).await;
            if mode == "--native-documentation-empty" {
                assert!(state.payload(&store, source, Kind::Signature).is_none());
            } else {
                let Some(Payload::Signature(signature)) =
                    state.payload(&store, source, Kind::Signature)
                else {
                    panic!("expected signature")
                };
                assert!(signature.model.active().label.starts_with(if alternate {
                    "alternate("
                } else {
                    "f("
                }));
                assert_eq!(
                    signature.model.parameter_range(),
                    Some(if alternate { 10..14 } else { 2..6 })
                );
            }
            bridge.disconnect().await.unwrap();
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn 실제_재시작은_이전_호버를_만료하고_새_mirror와_provider_세대만_적용한다() {
    tokio::time::timeout(DEADLINE, async {
        let (fixture, project, mut store, id, bin) =
            document_symbol_tests::fixture("--native-documentation-crash");
        let source = view(&mut store, id);
        store
            .apply(
                id,
                Transaction {
                    revision: 0,
                    edits: vec![Edit {
                        bytes: 0..0,
                        text: "crash ".into(),
                    }],
                    group: UndoGroup(0),
                    origin: Some(source),
                    selection_after: None,
                },
            )
            .unwrap();
        let snapshot = store.documents().snapshot(id).unwrap();
        let signal = Arc::new(Notify::new());
        let repaint = signal.clone();
        let mut bridge = LspBridge::connect(
            fixture.services.clone(),
            bin,
            Arc::new(move || repaint.notify_one()),
        )
        .unwrap();
        bridge.sync(project.clone(), snapshot.clone()).unwrap();
        running(&mut bridge, &signal, &fixture, 1).await;
        let before = bridge.documentation_providers(&project, &snapshot, Kind::Hover);
        let mut state = State::default();
        let old = begin(&bridge, &mut state, &store, &project, source, Kind::Hover);
        loop {
            while let Some(reply) = bridge.poll() {
                if let Reply::Failed { error, .. } = reply {
                    panic!("{error}");
                }
            }
            let providers = bridge.documentation_providers(&project, &snapshot, Kind::Hover);
            if !providers.is_empty() && providers != before {
                break;
            }
            signal.notified().await;
        }
        store
            .apply(
                id,
                Transaction {
                    revision: 1,
                    edits: vec![Edit {
                        bytes: 0.."crash ".len(),
                        text: "fresh ".into(),
                    }],
                    group: UndoGroup(1),
                    origin: Some(source),
                    selection_after: None,
                },
            )
            .unwrap();
        state.reconcile(
            &store,
            &HashSet::from([project.clone()]),
            |_| true,
            |project, snapshot, kind| bridge.documentation_providers(project, snapshot, kind),
        );
        assert!(old.is_cancelled());
        let snapshot = store.documents().snapshot(id).unwrap();
        bridge.sync(project.clone(), snapshot.clone()).unwrap();
        loop {
            if let Reply::Synced {
                document, revision, ..
            } = next(&mut bridge, &signal).await
                && document == id
                && revision == snapshot.revision
            {
                break;
            }
        }
        let current = bridge.documentation_providers(&project, &snapshot, Kind::Hover);
        assert!(current.iter().all(|provider| {
            before
                .iter()
                .any(|old| old.owner == provider.owner && old.generation < provider.generation)
        }));
        let request = begin(&bridge, &mut state, &store, &project, source, Kind::Hover);
        assert_ne!(request.token, old.token);
        assert_eq!(
            complete(&mut bridge, &signal, &mut state, &store, &request).await,
            1
        );
        bridge.disconnect().await.unwrap();
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}

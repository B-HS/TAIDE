use super::*;
use crate::editor_locations::State;
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{Edit, UndoGroup};
use taide_native_editor::store::{EditorStore, Transaction};
use taide_native_editor::symbol_locations::{Kind, Mode};
use taide_native_editor::view::{ViewId, ViewKey};
use tokio::sync::Notify;

const DEADLINE: Duration = Duration::from_secs(15);
const READY_DEADLINE: Duration = Duration::from_secs(10);
const EXECUTABLE_MODE: u32 = 0o700;

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
    .unwrap_or_else(|_| {
        panic!(
            "location reply: {:?}",
            bridge
                .states
                .borrow()
                .iter()
                .map(|state| (&state.name, &state.snapshot))
                .collect::<Vec<_>>()
        )
    })
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
                    panic!("{error}")
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
        "location readiness: {:?}; stderr={}",
        bridge
            .states
            .borrow()
            .iter()
            .map(|state| (&state.name, &state.snapshot))
            .collect::<Vec<_>>(),
        std::fs::read_to_string(fixture.directory.join("stderr.log")).unwrap_or_default()
    );
}

async fn diagnostic(bridge: &mut LspBridge, signal: &Notify, message: &str) {
    loop {
        if let Reply::Diagnostics { diagnostics, .. } = next(bridge, signal).await
            && diagnostics
                .diagnostics
                .iter()
                .any(|entry| entry.message == message)
        {
            return;
        }
    }
}

fn replace(store: &mut EditorStore, id: DocumentId, prefix: &str) -> DocumentSnapshot {
    let snapshot = store.documents().snapshot(id).unwrap();
    store
        .apply(
            id,
            Transaction {
                revision: snapshot.revision,
                edits: vec![Edit {
                    bytes: 0..snapshot.rope.len_bytes(),
                    text: format!("{prefix}\n  \u{1f600}method\nend"),
                }],
                group: UndoGroup(snapshot.revision),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
    store.documents().snapshot(id).unwrap()
}

async fn request(
    bridge: &mut LspBridge,
    signal: &Notify,
    state: &mut State,
    store: &EditorStore,
    project: &ProjectId,
    view: ViewId,
    kind: Kind,
    mode: Mode,
) -> (
    crate::editor_locations::Request,
    crate::editor_locations::Response,
) {
    let snapshot = store
        .documents()
        .snapshot(store.views().get(view).unwrap().document)
        .unwrap();
    let request = state
        .begin(
            project.clone(),
            store,
            view,
            kind,
            mode,
            bridge.location_providers(project, &snapshot, kind),
            Some(lsp_types::Position::new(1, 4)),
        )
        .unwrap();
    bridge.symbol_locations(request.clone()).unwrap();
    loop {
        if let Reply::SymbolLocations {
            request: current,
            result,
        } = next(bridge, signal).await
        {
            assert_eq!(current.token, request.token);
            return (current, result.unwrap());
        }
    }
}

#[tokio::test]
async fn 실제_child의_5종_위치는_scalar_array_link_참조축약과_명시peek를_보존한다() {
    tokio::time::timeout(DEADLINE, async {
        let (fixture, project, mut store, id, bin) =
            document_symbol_tests::fixture("--native-locations");
        let view = view(&mut store, id);
        let signal = Arc::new(Notify::new());
        let repaint = signal.clone();
        let mut bridge = LspBridge::connect(
            fixture.services.clone(),
            bin,
            Arc::new(move || repaint.notify_one()),
        )
        .unwrap();
        let snapshot = store.documents().snapshot(id).unwrap();
        bridge.sync(project.clone(), snapshot.clone()).unwrap();
        running(&mut bridge, &signal, &fixture, 1).await;
        let mut state = State::default();
        for (kind, mode, count) in [
            (Kind::Definition, Mode::GoTo, 1),
            (Kind::Declaration, Mode::GoTo, 1),
            (Kind::TypeDefinition, Mode::Peek, 1),
            (Kind::Implementation, Mode::GoTo, 2),
            (Kind::References, Mode::GoTo, 1),
            (Kind::References, Mode::Peek, 2),
        ] {
            let providers = bridge.location_providers(&project, &snapshot, kind);
            assert_eq!(providers.len(), 1);
            let (request, response) = request(
                &mut bridge,
                &signal,
                &mut state,
                &store,
                &project,
                view,
                kind,
                mode,
            )
            .await;
            assert_eq!(response.groups.len(), 1);
            let model = state
                .accept(&request, &store, providers, Ok(response))
                .unwrap()
                .unwrap();
            assert_eq!(model.targets().len(), count, "{kind:?} {mode:?}");
            if kind == Kind::TypeDefinition {
                assert_eq!(model.targets()[0].range.end, lsp_types::Position::new(2, 3));
                assert_eq!(
                    model.targets()[0].selection.start,
                    lsp_types::Position::new(1, 4)
                );
                assert_eq!(model.targets()[0].origin.unwrap().start, request.position);
            }
            if kind == Kind::References && mode == Mode::GoTo {
                assert_eq!(model.targets()[0].selection.start.line, 1);
            }
        }
        bridge.disconnect().await.unwrap();
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn 실제_child의_빈_null_오류_형식오류_미지원은_요청별로_격리된다() {
    tokio::time::timeout(DEADLINE, async {
        for mode in [
            "--native-locations-empty",
            "--native-locations-null",
            "--native-locations-error",
            "--native-locations-bad",
            "--native-document",
        ] {
            let (fixture, project, mut store, id, bin) = document_symbol_tests::fixture(mode);
            let view = view(&mut store, id);
            let signal = Arc::new(Notify::new());
            let repaint = signal.clone();
            let mut bridge = LspBridge::connect(
                fixture.services.clone(),
                bin,
                Arc::new(move || repaint.notify_one()),
            )
            .unwrap();
            let snapshot = store.documents().snapshot(id).unwrap();
            bridge.sync(project.clone(), snapshot.clone()).unwrap();
            running(&mut bridge, &signal, &fixture, 1).await;
            let providers = bridge.location_providers(&project, &snapshot, Kind::Definition);
            assert_eq!(providers.is_empty(), mode == "--native-document");
            let mut state = State::default();
            let (request, response) = request(
                &mut bridge,
                &signal,
                &mut state,
                &store,
                &project,
                view,
                Kind::Definition,
                Mode::Peek,
            )
            .await;
            assert_eq!(
                response.groups.len(),
                usize::from(matches!(
                    mode,
                    "--native-locations-empty" | "--native-locations-null"
                )),
                "{mode}"
            );
            let model = state
                .accept(&request, &store, providers, Ok(response))
                .unwrap()
                .unwrap();
            assert!(model.targets().is_empty());
            bridge.disconnect().await.unwrap();
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn 실제_보류_요청은_다른문서를_막지않고_편집과_뷰닫힘에_cancel을_전송한다() {
    tokio::time::timeout(DEADLINE, async {
        let (fixture, project, mut store, id, bin) =
            document_symbol_tests::fixture("--native-locations-wait");
        let source = view(&mut store, id);
        let held = replace(&mut store, id, "hold");
        let signal = Arc::new(Notify::new());
        let repaint = signal.clone();
        let mut bridge = LspBridge::connect(
            fixture.services.clone(),
            bin,
            Arc::new(move || repaint.notify_one()),
        )
        .unwrap();
        bridge.sync(project.clone(), held.clone()).unwrap();
        running(&mut bridge, &signal, &fixture, 1).await;
        let mut state = State::default();
        let old = state
            .begin(
                project.clone(),
                &store,
                source,
                Kind::Definition,
                Mode::Peek,
                bridge.location_providers(&project, &held, Kind::Definition),
                None,
            )
            .unwrap();
        bridge.symbol_locations(old.clone()).unwrap();
        diagnostic(&mut bridge, &signal, "synthetic location held").await;
        let DocumentKey::File(path) = &held.key else {
            panic!("file fixture")
        };
        let path = path.parent().unwrap().join("other.rs");
        std::fs::write(&path, "class\n  \u{1f600}method\nend").unwrap();
        let file = taide_file::service::open_file(&path, &[], false).unwrap();
        let other = store.open_file(path, file).unwrap();
        let other_view = view(&mut store, other);
        let other_snapshot = store.documents().snapshot(other).unwrap();
        bridge
            .sync(project.clone(), other_snapshot.clone())
            .unwrap();
        while bridge
            .location_providers(&project, &other_snapshot, Kind::Definition)
            .is_empty()
        {
            signal.notified().await;
        }
        let (other_request, response) = request(
            &mut bridge,
            &signal,
            &mut state,
            &store,
            &project,
            other_view,
            Kind::Definition,
            Mode::GoTo,
        )
        .await;
        assert_eq!(response.groups[0].targets.len(), 1);
        assert!(
            state
                .accept(
                    &other_request,
                    &store,
                    bridge.location_providers(&project, &other_snapshot, Kind::Definition),
                    Ok(response)
                )
                .unwrap()
                .is_some()
        );
        let current = replace(&mut store, id, "fresh");
        state.reconcile(
            &store,
            &HashSet::from([project.clone()]),
            |project, snapshot, kind| bridge.location_providers(project, snapshot, kind),
        );
        assert!(old.is_cancelled());
        diagnostic(&mut bridge, &signal, "synthetic location cancelled").await;
        bridge.sync(project.clone(), current).unwrap();
        let (fresh, response) = request(
            &mut bridge,
            &signal,
            &mut state,
            &store,
            &project,
            source,
            Kind::References,
            Mode::GoTo,
        )
        .await;
        assert_ne!(old.token, fresh.token);
        assert_eq!(response.groups[0].targets.len(), 1);
        let held = replace(&mut store, id, "hold");
        bridge.sync(project.clone(), held.clone()).unwrap();
        let closing = state
            .begin(
                project.clone(),
                &store,
                source,
                Kind::Definition,
                Mode::Peek,
                bridge.location_providers(&project, &held, Kind::Definition),
                None,
            )
            .unwrap();
        bridge.symbol_locations(closing.clone()).unwrap();
        diagnostic(&mut bridge, &signal, "synthetic location held").await;
        store.detach_view(source).unwrap();
        state.reconcile(
            &store,
            &HashSet::from([project]),
            |project, snapshot, kind| bridge.location_providers(project, snapshot, kind),
        );
        assert!(closing.is_cancelled());
        diagnostic(&mut bridge, &signal, "synthetic location cancelled").await;
        bridge.disconnect().await.unwrap();
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn 실제_다중provider는_등록우선순위_부분오류와_프로젝트_owner를_보존한다() {
    use std::os::unix::fs::PermissionsExt;
    tokio::time::timeout(DEADLINE, async {
        for partial_error in [false, true] {
            let (fixture, project, mut store, id, bin) =
                document_symbol_tests::fixture("--native-locations");
            let rust = store.documents().snapshot(id).unwrap();
            let DocumentKey::File(path) = &rust.key else {
                panic!("file fixture")
            };
            let python = path.parent().unwrap().join("source.py");
            std::fs::write(&python, "class\n  \u{1f600}method\nend").unwrap();
            let file = taide_file::service::open_file(&python, &[], false).unwrap();
            let id = store.open_file(python, file).unwrap();
            let snapshot = store.documents().snapshot(id).unwrap();
            assert_eq!(snapshot.metadata.language_id, "python");
            let view = view(&mut store, id);
            let mock = std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("examples/native-lsp-mock");
            let mock = mock.to_str().unwrap().replace('\'', "'\\''");
            let mode = if partial_error {
                "--native-locations-error"
            } else {
                "--native-locations-alternate"
            };
            for (binary, mode) in [
                ("basedpyright-langserver", "--native-locations"),
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
            let providers = bridge.location_providers(&project, &snapshot, Kind::Definition);
            let foreign = bridge.location_providers(&other, &snapshot, Kind::Definition);
            assert_eq!(providers.len(), 2);
            assert_eq!(foreign.len(), 2);
            assert!(providers.is_disjoint(&foreign));
            let mut state = State::default();
            let (request, response) = request(
                &mut bridge,
                &signal,
                &mut state,
                &store,
                &project,
                view,
                Kind::Definition,
                Mode::GoTo,
            )
            .await;
            assert_eq!(response.groups.len(), if partial_error { 1 } else { 2 });
            assert!(
                response
                    .groups
                    .iter()
                    .all(|group| providers.contains(&group.provider))
            );
            let model = state
                .accept(&request, &store, providers, Ok(response))
                .unwrap()
                .unwrap();
            let first = &model.targets()[model.first().unwrap()];
            assert_eq!(first.selection.start.line, u32::from(partial_error));
            assert_eq!(model.targets().len(), if partial_error { 1 } else { 2 });
            bridge.disconnect().await.unwrap();
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn 실제_서버_재시작은_이전위치를_만료하고_새_mirror의_응답만_적용한다() {
    tokio::time::timeout(DEADLINE, async {
        let (fixture, project, mut store, id, bin) =
            document_symbol_tests::fixture("--native-locations-crash");
        let view = view(&mut store, id);
        let snapshot = replace(&mut store, id, "crash");
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
        let before = bridge.location_providers(&project, &snapshot, Kind::Definition);
        let mut state = State::default();
        let old = state
            .begin(
                project.clone(),
                &store,
                view,
                Kind::Definition,
                Mode::GoTo,
                before.clone(),
                None,
            )
            .unwrap();
        bridge.symbol_locations(old.clone()).unwrap();
        loop {
            while let Some(reply) = bridge.poll() {
                if let Reply::Failed { error, .. } = reply {
                    panic!("{error}")
                }
            }
            let providers = bridge.location_providers(&project, &snapshot, Kind::Definition);
            if !providers.is_empty() && providers != before {
                break;
            }
            signal.notified().await;
        }
        let current = replace(&mut store, id, "fresh");
        state.reconcile(
            &store,
            &HashSet::from([project.clone()]),
            |project, snapshot, kind| bridge.location_providers(project, snapshot, kind),
        );
        assert!(old.is_cancelled());
        bridge.sync(project.clone(), current.clone()).unwrap();
        let (request, response) = request(
            &mut bridge,
            &signal,
            &mut state,
            &store,
            &project,
            view,
            Kind::Definition,
            Mode::GoTo,
        )
        .await;
        assert_ne!(request.token, old.token);
        assert!(response.groups.iter().all(|group| {
            before.iter().any(|old| {
                old.owner == group.provider.owner && old.generation < group.provider.generation
            })
        }));
        let model = state
            .accept(
                &request,
                &store,
                bridge.location_providers(&project, &current, Kind::Definition),
                Ok(response),
            )
            .unwrap()
            .unwrap();
        assert_eq!(model.targets().len(), 1);
        bridge.disconnect().await.unwrap();
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}

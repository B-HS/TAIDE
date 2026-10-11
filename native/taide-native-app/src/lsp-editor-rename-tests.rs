use super::*;
use crate::editor_rename::{Response, State};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{Selection, SelectionSet, ViewKey};
use tokio::sync::Notify;

const DEADLINE: Duration = Duration::from_secs(10);
const SERVER_ERROR_CODE: i64 = -32603;

fn rename_view(store: &mut EditorStore, document: DocumentId) -> taide_native_editor::view::ViewId {
    let view = store
        .attach_view(
            ViewKey {
                window: "synthetic-rename".into(),
                pane: taide_model::ids::PaneId::new(),
                tab: taide_model::ids::TabId::new(),
            },
            document,
        )
        .unwrap();
    let head = store
        .documents()
        .snapshot(document)
        .unwrap()
        .rope
        .to_string()
        .find("method")
        .unwrap()
        + "me".len();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: head, head }],
            },
            Default::default(),
            Vec::new(),
        )
        .unwrap();
    view
}

async fn providers(
    bridge: &mut LspBridge,
    signal: &Notify,
    project: &ProjectId,
    snapshot: &DocumentSnapshot,
) -> HashSet<crate::editor_symbols::ProviderIdentity> {
    loop {
        while let Some(reply) = bridge.poll() {
            if let Reply::Failed { error, .. } = reply {
                panic!("{error}");
            }
        }
        let providers = bridge.rename_providers(project, snapshot);
        if !providers.is_empty() {
            return providers;
        }
        signal.notified().await;
    }
}

async fn rename_diagnostic(bridge: &mut LspBridge, signal: &Notify, message: &str) {
    loop {
        while let Some(reply) = bridge.poll() {
            match reply {
                Reply::Failed { error, .. } => panic!("{error}"),
                Reply::Diagnostics { diagnostics, .. }
                    if diagnostics
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.message == message) =>
                {
                    return;
                }
                _ => {}
            }
        }
        signal.notified().await;
    }
}

#[tokio::test]
async fn 실제_child는_준비_범위_기본값_새_이름과_프로토콜_버전을_현재_문서에_보존한다() {
    for mode in [
        "--native-rename",
        "--native-rename-range",
        "--native-rename-no-prepare",
        "--native-rename-default",
        "--native-rename-empty",
    ] {
        tokio::time::timeout(DEADLINE, async {
            let (fixture, project, mut store, document, bin) = document_symbol_tests::fixture(mode);
            let view = store
                .attach_view(
                    ViewKey {
                        window: "synthetic".into(),
                        pane: taide_model::ids::PaneId::new(),
                        tab: taide_model::ids::TabId::new(),
                    },
                    document,
                )
                .unwrap();
            let before = store.documents().snapshot(document).unwrap();
            let text = before.rope.to_string();
            let start = text.find("method").unwrap();
            let head = start + "me".len();
            store
                .set_view_state(
                    view,
                    SelectionSet {
                        primary: 0,
                        selections: vec![Selection { anchor: head, head }],
                    },
                    Default::default(),
                    Vec::new(),
                )
                .unwrap();
            let signal = Arc::new(Notify::new());
            let repaint = signal.clone();
            let mut bridge = LspBridge::connect(
                fixture.services.clone(),
                bin,
                Arc::new(move || repaint.notify_one()),
            )
            .unwrap();
            bridge.sync(project.clone(), before.clone()).unwrap();
            let available = providers(&mut bridge, &signal, &project, &before).await;
            let mut state = State::default();
            let prepare = state
                .begin(&store, project.clone(), view, view, available.clone())
                .unwrap()
                .unwrap();
            assert_eq!(prepare.position, lsp_types::Position::new(1, 6));
            let response = bridge
                .rename_editor(prepare.clone())
                .unwrap()
                .await
                .unwrap();
            state
                .accept(&store, &prepare, available.clone(), response)
                .unwrap();
            let (_, ready) = state.prepared().unwrap();
            let expected_name = match mode {
                "--native-rename-range"
                | "--native-rename-no-prepare"
                | "--native-rename-default" => "\u{1f600}method",
                _ => "method",
            };
            let expected_start = match mode {
                "--native-rename-no-prepare" | "--native-rename-default" => {
                    text.find('\u{1f600}').unwrap()
                }
                _ => start,
            };
            assert_eq!(ready.name, expected_name, "mode={mode}");
            assert_eq!(ready.range, expected_start..start + "method".len());
            let rename = state.rename(&store, "이름\u{1f600}".into()).unwrap();
            let response = bridge.rename_editor(rename.clone()).unwrap().await.unwrap();
            let edits = state
                .accept(&store, &rename, available, response)
                .unwrap()
                .unwrap();
            let DocumentKey::File(path) = &before.key else {
                unreachable!()
            };
            assert!(edits.roots.iter().any(|root| path.starts_with(root)));
            assert!(
                edits
                    .documents
                    .iter()
                    .any(|document| document.snapshot.id == before.id
                        && document.snapshot.revision == before.revision
                        && document.revision == Some(0))
            );
            let (completion, _) = oneshot::channel();
            let event = WorkspaceEditEvent {
                edit: edits.edit,
                documents: edits.documents,
                completion,
            };
            let outcome = crate::lsp_workspace::apply_open(&mut store, &event);
            assert!(outcome.failure.is_none());
            let after = store.documents().snapshot(document).unwrap();
            if mode == "--native-rename-empty" {
                assert!(outcome.changed.is_empty());
                assert_eq!(after.rope, before.rope);
            } else {
                assert_eq!(outcome.changed, vec![document]);
                assert_eq!(
                    after.rope.to_string(),
                    text.replace("method", "이름\u{1f600}")
                );
                assert!(after.dirty);
                assert!(store.undo(document).unwrap());
                assert_eq!(
                    store.documents().snapshot(document).unwrap().rope,
                    before.rope
                );
            }
            assert_eq!(std::fs::read_to_string(path).unwrap(), text);
            state.finish(&rename);
            bridge.disconnect().await.unwrap();
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        })
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn 실제_child의_준비_null_잘못된_utf16과_오류는_입력이나_편집을_허용하지_않는다() {
    for mode in [
        "--native-rename-null",
        "--native-rename-bad",
        "--native-rename-error",
    ] {
        tokio::time::timeout(DEADLINE, async {
            let (fixture, project, mut store, document, bin) = document_symbol_tests::fixture(mode);
            let view = store.attach_view(ViewKey { window: "synthetic".into(), pane: taide_model::ids::PaneId::new(), tab: taide_model::ids::TabId::new() }, document).unwrap();
            let before = store.documents().snapshot(document).unwrap();
            let head = before.rope.to_string().find("method").unwrap();
            store.set_view_state(view, SelectionSet { primary: 0, selections: vec![Selection { anchor: head, head }] }, Default::default(), Vec::new()).unwrap();
            let signal = Arc::new(Notify::new());
            let repaint = signal.clone();
            let mut bridge = LspBridge::connect(fixture.services.clone(), bin, Arc::new(move || repaint.notify_one())).unwrap();
            bridge.sync(project.clone(), before.clone()).unwrap();
            let available = providers(&mut bridge, &signal, &project, &before).await;
            let mut state = State::default();
            let request = state.begin(&store, project, view, view, available.clone()).unwrap().unwrap();
            let response = bridge.rename_editor(request.clone()).unwrap().await.unwrap();
            let expected = match mode {
                "--native-rename-null" => { assert!(matches!(&response, Ok(Response::Unavailable))); Failure::UnsupportedCapability }
                "--native-rename-bad" => Failure::MalformedResponse,
                "--native-rename-error" => Failure::ServerError(SERVER_ERROR_CODE),
                _ => unreachable!(),
            };
            assert!(matches!(state.accept(&store, &request, available, response), Err(error) if error == expected));
            assert!(request.is_cancelled());
            assert!(state.prepared().is_none());
            assert_eq!(store.documents().snapshot(document).unwrap().rope, before.rope);
            bridge.disconnect().await.unwrap();
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        }).await.unwrap();
    }
}

#[tokio::test]
async fn rename_boundary_두_공급자의_준비_대체는_선택한_공급자와_프로젝트를_보존한다() {
    use std::os::unix::fs::PermissionsExt;
    const EXECUTABLE_MODE: u32 = 0o700;
    const SESSION_COUNT: usize = 4;
    for (base_mode, mode, expected_id) in [
        ("--native-rename", "--native-rename", Some("ruff")),
        (
            "--native-rename",
            "--native-rename-null",
            Some("basedPyright"),
        ),
        (
            "--native-rename-no-prepare",
            "--native-rename-null",
            Some("basedPyright"),
        ),
        (
            "--native-rename",
            "--native-rename-no-prepare",
            Some("ruff"),
        ),
        ("--native-rename", "--native-rename-empty", Some("ruff")),
        (
            "--native-rename",
            "--native-rename-unsupported",
            Some("basedPyright"),
        ),
        ("--native-rename", "--native-rename-error", None),
        ("--native-rename", "--native-rename-bad", None),
    ] {
        tokio::time::timeout(DEADLINE, async {
            let (fixture, project, mut store, rust_document, bin) = document_symbol_tests::fixture("--native-rename");
            let rust = store.documents().snapshot(rust_document).unwrap();
            let DocumentKey::File(path) = &rust.key else { unreachable!() };
            let python = path.parent().unwrap().join("rename.py");
            let text = rust.rope.to_string();
            std::fs::write(&python, &text).unwrap();
            let file = taide_file::service::open_file(&python, &[], false).unwrap();
            let document = store.open_file(python.clone(), file).unwrap();
            let view = rename_view(&mut store, document);
            let before = store.documents().snapshot(document).unwrap();
            assert_eq!(before.metadata.language_id, "python");
            let mock = std::env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("examples/native-lsp-mock");
            let escaped = mock.to_str().unwrap().replace('\'', "'\\''");
            for (binary, selected_mode) in [("basedpyright-langserver", base_mode), ("ruff", mode)] {
                let path = std::path::PathBuf::from(&bin).join(binary);
                std::fs::write(&path, format!("#!/bin/sh\nexec '{escaped}' {selected_mode}\n")).unwrap();
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(EXECUTABLE_MODE)).unwrap();
            }
            let other = ProjectId::new();
            let mut foreign_project = fixture.services.state.projects.read()[&project].clone();
            foreign_project.id = other.clone();
            fixture.services.state.projects.write().insert(other.clone(), foreign_project);
            let signal = Arc::new(Notify::new());
            let repaint = signal.clone();
            let mut bridge = LspBridge::connect(fixture.services.clone(), bin, Arc::new(move || repaint.notify_one())).unwrap();
            bridge.sync(project.clone(), before.clone()).unwrap();
            bridge.sync(other.clone(), before.clone()).unwrap();
            loop {
                while let Some(reply) = bridge.poll() {
                    if let Reply::Failed { error, .. } = reply { panic!("{mode}: {error}"); }
                }
                if bridge.states.borrow().len() == SESSION_COUNT && bridge.states.borrow().iter().all(|state| state.snapshot.phase == Phase::Running) { break; }
                signal.notified().await;
            }
            let available = bridge.rename_providers(&project, &before);
            let foreign = bridge.rename_providers(&other, &before);
            let expected_count = if mode == "--native-rename-unsupported" { 1 } else { 2 };
            assert_eq!(available.len(), expected_count, "{mode}");
            assert_eq!(foreign.len(), expected_count, "{mode}");
            assert!(available.is_disjoint(&foreign));
            let mut state = State::default();
            let prepare = state.begin(&store, project.clone(), view, view, available.clone()).unwrap().unwrap();
            let response = bridge.rename_editor(prepare.clone()).unwrap().await.unwrap();
            if let Some(expected_id) = expected_id {
                state.accept(&store, &prepare, available.clone(), response).unwrap();
                let (_, ready) = state.prepared().unwrap();
                let expected_name = taide_lsp::manifest::servers().iter().find(|spec| spec.id.0 == expected_id).unwrap().name.clone();
                let expected_owner = bridge.states.borrow().iter().find(|entry| entry.project == project && entry.name == expected_name).unwrap().owner;
                assert_eq!(ready.provider.owner, expected_owner, "base={base_mode} top={mode}");
                assert!(available.contains(&ready.provider));
                assert!(!foreign.contains(&ready.provider));
                let rename = state.rename(&store, "renamed".into()).unwrap();
                let response = bridge.rename_editor(rename.clone()).unwrap().await.unwrap();
                let edits = state.accept(&store, &rename, available, response).unwrap().unwrap();
                assert_eq!(edits.provider.owner, expected_owner);
                let (completion, _) = oneshot::channel();
                let outcome = crate::lsp_workspace::apply_open(&mut store, &WorkspaceEditEvent { edit: edits.edit, documents: edits.documents, completion });
                assert!(outcome.failure.is_none());
                if mode == "--native-rename-empty" {
                    assert!(outcome.changed.is_empty());
                    assert_eq!(store.documents().snapshot(document).unwrap().rope, before.rope);
                } else {
                    assert_eq!(outcome.changed, [document]);
                    assert_eq!(store.documents().snapshot(document).unwrap().rope.to_string(), text.replace("method", "renamed"));
                    assert!(store.undo(document).unwrap());
                    assert_eq!(store.documents().snapshot(document).unwrap().rope, before.rope);
                }
                state.finish(&rename);
            } else {
                let expected = if mode == "--native-rename-error" { Failure::ServerError(SERVER_ERROR_CODE) } else { Failure::MalformedResponse };
                assert!(matches!(state.accept(&store, &prepare, available, response), Err(error) if error == expected));
                assert!(state.prepared().is_none());
                assert!(prepare.is_cancelled());
            }
            assert_eq!(std::fs::read_to_string(&python).unwrap(), text);
            bridge.disconnect().await.unwrap();
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        }).await.unwrap();
    }
}

#[tokio::test]
async fn rename_boundary_준비와_편집_대기는_취소를_전달하고_늦은_응답과_종료를_보존한다() {
    for mode in ["--native-rename-wait", "--native-rename-wait-edits"] {
        let mut phase = "fixture";
        let result = tokio::time::timeout(DEADLINE, async {
            let (fixture, project, mut store, document, bin) = document_symbol_tests::fixture(mode);
            let before = store.documents().snapshot(document).unwrap();
            let DocumentKey::File(path) = &before.key else {
                unreachable!()
            };
            let disk = std::fs::read_to_string(path).unwrap();
            let view = rename_view(&mut store, document);
            let signal = Arc::new(Notify::new());
            let repaint = signal.clone();
            let mut bridge = LspBridge::connect(
                fixture.services.clone(),
                bin,
                Arc::new(move || repaint.notify_one()),
            )
            .unwrap();
            bridge.sync(project.clone(), before.clone()).unwrap();
            phase = "providers";
            let available = providers(&mut bridge, &signal, &project, &before).await;
            let mut state = State::default();
            let prepare = state
                .begin(&store, project.clone(), view, view, available.clone())
                .unwrap()
                .unwrap();
            let request = if mode == "--native-rename-wait" {
                prepare
            } else {
                phase = "prepare";
                let response = bridge
                    .rename_editor(prepare.clone())
                    .unwrap()
                    .await
                    .unwrap();
                state
                    .accept(&store, &prepare, available.clone(), response)
                    .unwrap();
                state.rename(&store, "renamed".into()).unwrap()
            };
            let receive = bridge.rename_editor(request.clone()).unwrap();
            phase = "held diagnostic";
            rename_diagnostic(&mut bridge, &signal, "synthetic rename held").await;
            if mode == "--native-rename-wait" {
                state.clear();
            } else {
                store.detach_view(view).unwrap();
                state.reconcile(&store, |_| available.clone());
            }
            assert!(request.is_cancelled());
            phase = "cancel result";
            let response = receive.await.unwrap();
            assert!(matches!(response, Err(Failure::Cancelled)), "{mode}");
            assert!(
                state
                    .accept(&store, &request, available, response)
                    .unwrap()
                    .is_none()
            );
            phase = "cancel diagnostic";
            rename_diagnostic(&mut bridge, &signal, "synthetic rename cancelled").await;
            assert!(state.prepared().is_none());
            let after = store.documents().snapshot(document).unwrap();
            assert_eq!(after.rope, before.rope);
            assert_eq!(after.revision, before.revision);
            assert_eq!(after.dirty, before.dirty);
            assert_eq!(std::fs::read_to_string(path).unwrap(), disk);
            phase = "disconnect";
            bridge.disconnect().await.unwrap();
            phase = "task shutdown";
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        })
        .await;
        assert!(result.is_ok(), "{mode}: {phase}");
    }
}

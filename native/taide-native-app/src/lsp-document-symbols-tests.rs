use super::*;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use taide_model::{app_event::AppEvent, paths::AppPaths, project::Project};
use taide_native_editor::document::{Edit, UndoGroup};
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::Notify;

const DEADLINE: Duration = Duration::from_secs(10);
const BYTE_LIMIT: usize = 4096;
const EXECUTABLE_MODE: u32 = 0o700;

struct Sink;
impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

pub(super) struct Fixture {
    pub(super) directory: PathBuf,
    pub(super) services: Arc<AppServices>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.lsp.shutdown();
        self.services.tasks.stop_all();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

pub(super) fn fixture(mode: &str) -> (Fixture, ProjectId, EditorStore, DocumentId, OsString) {
    assert!(matches!(
        mode,
        "--native-symbols"
            | "--native-symbols-flat"
            | "--native-symbols-wait"
            | "--native-symbols-bad"
            | "--native-document"
            | "--native-workspace-symbols"
            | "--native-workspace-symbols-nested"
            | "--native-workspace-symbols-wait"
            | "--native-workspace-symbols-crash"
            | "--native-workspace-symbols-error"
            | "--native-folding"
            | "--native-folding-empty"
            | "--native-folding-null"
            | "--native-folding-error"
            | "--native-folding-wait"
            | "--native-folding-crash"
            | "--native-locations"
            | "--native-locations-empty"
            | "--native-locations-null"
            | "--native-locations-error"
            | "--native-locations-bad"
            | "--native-locations-wait"
            | "--native-locations-crash"
    ));
    let directory = std::env::temp_dir().join(format!("taide-native-symbols-{}", ProjectId::new()));
    let root = directory.join("root");
    let bin = directory.join("bin");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&bin).unwrap();
    let mock = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/native-lsp-mock");
    assert!(mock.is_file());
    let escaped = mock.to_str().unwrap().replace('\'', "'\\''");
    let executable = bin.join("rust-analyzer");
    let stderr = directory
        .join("stderr.log")
        .to_str()
        .unwrap()
        .replace('\'', "'\\''");
    std::fs::write(
        &executable,
        format!("#!/bin/sh\nexec '{escaped}' {mode} 2> '{stderr}'\n"),
    )
    .unwrap();
    std::fs::set_permissions(
        &executable,
        std::fs::Permissions::from_mode(EXECUTABLE_MODE),
    )
    .unwrap();
    let root = root.canonicalize().unwrap();
    let project = ProjectId::new();
    let state = AppState::new(AppPaths::new(directory.join("data")));
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project.clone(),
            root: root.to_str().unwrap().into(),
            name: "synthetic symbols".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let fixture = Fixture {
        directory,
        services: crate::bootstrap::services(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            Arc::new(Sink),
        ),
    };
    let path = root.join("bound(a),.rs");
    std::fs::write(&path, "class\n  \u{1f600}method\nend").unwrap();
    let file = taide_file::service::open_file(&path, &[], false).unwrap();
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 2,
        max_views: 2,
        max_undo_groups: 1,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store.open_file(path, file).unwrap();
    (fixture, project, store, document, bin.into_os_string())
}

async fn next(bridge: &mut LspBridge, signal: &Notify) -> Reply {
    loop {
        if let Some(reply) = bridge.poll() {
            return reply;
        }
        signal.notified().await;
    }
}

#[tokio::test]
async fn 실제_child의_계층_flat_오류_미지원은_파일_root와_최신_mirror를_보존한다() {
    tokio::time::timeout(DEADLINE, async {
        for mode in [
            "--native-symbols",
            "--native-symbols-flat",
            "--native-symbols-bad",
            "--native-document",
        ] {
            let (fixture, project, mut store, id, bin) = fixture(mode);
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
            let mut state = crate::editor_symbols::State::default();
            let request = state
                .observe(
                    &project,
                    snapshot.clone(),
                    HashSet::new(),
                    std::time::Instant::now(),
                )
                .unwrap();
            bridge.document_symbols(request.clone()).unwrap();
            loop {
                match next(&mut bridge, &signal).await {
                    Reply::DocumentSymbols { request, result } => {
                        if mode == "--native-symbols-bad" {
                            assert!(matches!(
                                result.as_ref().unwrap().error,
                                Some(Failure::MalformedResponse)
                            ));
                        }
                        let applied = state.accept(
                            &request,
                            &snapshot,
                            bridge.symbol_providers(&project, id),
                            result,
                        );
                        if mode == "--native-symbols-bad" {
                            assert_eq!(applied, Err(Failure::MalformedResponse));
                        } else {
                            assert!(applied.unwrap());
                        }
                        break;
                    }
                    Reply::Failed { error, .. } => panic!("{error}"),
                    _ => {}
                }
            }
            assert!(!state.palette(Some(&project), Some(&snapshot)).is_pending);
            let model = state.model(&project, &snapshot).unwrap();
            match mode {
                "--native-symbols" => {
                    assert_eq!(model.symbols()[0].name, "Class:0");
                    assert_eq!(model.symbols()[1].container_label, "Class:0");
                    let target = state
                        .position(&project, &snapshot, request.generation, 1)
                        .unwrap();
                    assert_eq!((target.line, target.column), (2.0, 5.0));
                    assert!(state.sticky(&project, &snapshot).is_some());
                }
                "--native-symbols-flat" => assert_eq!(model.symbols()[0].name, "flat:0"),
                _ => assert!(model.symbols().is_empty()),
            }
            if mode == "--native-symbols" {
                store
                    .apply(
                        id,
                        Transaction {
                            revision: snapshot.revision,
                            edits: vec![Edit {
                                bytes: 0..5,
                                text: "CLASS".into(),
                            }],
                            group: UndoGroup(0),
                            origin: None,
                            selection_after: None,
                        },
                    )
                    .unwrap();
                let current = store.documents().snapshot(id).unwrap();
                bridge.sync(project.clone(), current.clone()).unwrap();
                let now = std::time::Instant::now();
                assert!(
                    state
                        .observe(
                            &project,
                            current.clone(),
                            bridge.symbol_providers(&project, id),
                            now
                        )
                        .is_none()
                );
                let request = state
                    .observe(
                        &project,
                        current.clone(),
                        bridge.symbol_providers(&project, id),
                        now + Duration::from_millis(400),
                    )
                    .unwrap();
                bridge.document_symbols(request).unwrap();
                loop {
                    if let Reply::DocumentSymbols { request, result } =
                        next(&mut bridge, &signal).await
                    {
                        state
                            .accept(
                                &request,
                                &current,
                                bridge.symbol_providers(&project, id),
                                result,
                            )
                            .unwrap();
                        assert_eq!(
                            state.model(&project, &current).unwrap().symbols()[0].name,
                            "Class:1"
                        );
                        assert!(
                            state.model(&project, &current).unwrap().symbols()[0]
                                .detail
                                .starts_with("CLASS")
                        );
                        break;
                    }
                }
            }
            bridge.disconnect().await.unwrap();
            fixture.services.tasks.shutdown().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn 응답을_보류한_child에서도_문서_동기화는_진행되고_취소된_회신은_버린다() {
    tokio::time::timeout(DEADLINE, async {
        let (fixture, project, mut store, id, bin) = fixture("--native-symbols-wait");
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
        let mut state = crate::editor_symbols::State::default();
        let request = state
            .observe(
                &project,
                snapshot.clone(),
                HashSet::new(),
                std::time::Instant::now(),
            )
            .unwrap();
        bridge.document_symbols(request.clone()).unwrap();
        loop {
            match next(&mut bridge, &signal).await {
                Reply::Diagnostics { diagnostics, .. }
                    if diagnostics
                        .diagnostics
                        .iter()
                        .any(|item| item.message == "synthetic symbol held") =>
                {
                    break;
                }
                Reply::Failed { error, .. } => panic!("{error}"),
                _ => {}
            }
        }
        let DocumentKey::File(path) = &snapshot.key else {
            panic!("file expected");
        };
        let other_path = path.parent().unwrap().join("other.rs");
        std::fs::write(&other_path, "class\n  method\nend").unwrap();
        let other_file = taide_file::service::open_file(&other_path, &[], false).unwrap();
        let other = store.open_file(other_path, other_file).unwrap();
        let current = store.documents().snapshot(other).unwrap();
        bridge.sync(project.clone(), current.clone()).unwrap();
        loop {
            if let Reply::Synced {
                document, revision, ..
            } = next(&mut bridge, &signal).await
            {
                if document == current.id && revision == current.revision {
                    break;
                }
            }
        }
        state.retain(&HashSet::new());
        assert!(request.is_cancelled());
        loop {
            match next(&mut bridge, &signal).await {
                Reply::Diagnostics { diagnostics, .. }
                    if diagnostics
                        .diagnostics
                        .iter()
                        .any(|item| item.message == "synthetic symbol cancelled") =>
                {
                    break;
                }
                Reply::DocumentSymbols { .. } => panic!("cancelled result reached the UI"),
                _ => {}
            }
        }
        assert!(state.model(&project, &current).is_none());
        bridge.disconnect().await.unwrap();
        fixture.services.tasks.shutdown().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn 같은_파일을_공유하는_두_프로젝트는_독립_mirror와_심볼_owner를_유지한다() {
    tokio::time::timeout(DEADLINE, async {
        let (fixture, first_project, store, id, bin) = fixture("--native-symbols");
        let second_project = ProjectId::new();
        let mut shared = fixture.services.state.projects.read()[&first_project].clone();
        shared.id = second_project.clone();
        fixture
            .services
            .state
            .projects
            .write()
            .insert(second_project.clone(), shared);
        let signal = Arc::new(Notify::new());
        let repaint = signal.clone();
        let mut bridge = LspBridge::connect(
            fixture.services.clone(),
            bin,
            Arc::new(move || repaint.notify_one()),
        )
        .unwrap();
        let snapshot = store.documents().snapshot(id).unwrap();
        let mut state = crate::editor_symbols::State::default();
        let mut requests = Vec::new();
        for project in [&first_project, &second_project] {
            bridge.sync(project.clone(), snapshot.clone()).unwrap();
            let request = state
                .observe(
                    project,
                    snapshot.clone(),
                    HashSet::new(),
                    std::time::Instant::now(),
                )
                .unwrap();
            bridge.document_symbols(request.clone()).unwrap();
            requests.push(request);
        }
        let mut received = HashSet::new();
        while received.len() < requests.len() {
            match next(&mut bridge, &signal).await {
                Reply::DocumentSymbols { request, result } => {
                    let project = request.project.clone();
                    assert!(
                        state
                            .accept(
                                &request,
                                &snapshot,
                                bridge.symbol_providers(&project, id),
                                result
                            )
                            .unwrap()
                    );
                    received.insert(project);
                }
                Reply::Failed { error, .. } => panic!("{error}"),
                _ => {}
            }
        }
        let first = bridge.symbol_providers(&first_project, id);
        let second = bridge.symbol_providers(&second_project, id);
        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1);
        assert!(first.is_disjoint(&second));
        assert!(state.model(&first_project, &snapshot).is_some());
        assert!(state.model(&second_project, &snapshot).is_some());
        let retained = HashSet::from([(first_project.clone(), id)]);
        bridge.retain_bindings(&retained).unwrap();
        state.retain(&retained);
        while !bridge.symbol_providers(&second_project, id).is_empty() {
            signal.notified().await;
        }
        assert_eq!(bridge.symbol_providers(&first_project, id), first);
        assert!(
            state
                .position(&first_project, &snapshot, requests[0].generation, 1)
                .is_some()
        );
        assert!(
            state
                .position(&second_project, &snapshot, requests[1].generation, 1)
                .is_none()
        );
        bridge.disconnect().await.unwrap();
        fixture.services.tasks.shutdown().await;
    })
    .await
    .unwrap();
}

use super::*;
use crate::workspace_symbols::State;
use tokio::sync::Notify;

const DEADLINE: Duration = Duration::from_secs(10);
const DEBOUNCE: Duration = Duration::from_millis(200);
const READY_DEADLINE: Duration = Duration::from_secs(5);

async fn running(
    bridge: &mut LspBridge,
    signal: &Notify,
    fixture: &document_symbol_tests::Fixture,
    expected: usize,
) {
    let ready = tokio::time::timeout(READY_DEADLINE, async {
        loop {
            while let Some(reply) = bridge.poll() {
                if let Reply::Failed { error, .. } = reply {
                    panic!("{error}");
                }
            }
            if bridge.states.borrow().len() == expected
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
        "workspace readiness: {:?}; stderr={}",
        bridge
            .states
            .borrow()
            .iter()
            .map(|state| (&state.name, state.snapshot.phase, &state.snapshot.failure))
            .collect::<Vec<_>>(),
        std::fs::read_to_string(fixture.directory.join("stderr.log")).unwrap_or_default()
    );
}

async fn next(bridge: &mut LspBridge, signal: &Notify) -> Reply {
    let reply = tokio::time::timeout(READY_DEADLINE, async {
        loop {
            if let Some(reply) = bridge.poll() {
                return reply;
            }
            signal.notified().await;
        }
    })
    .await;
    reply.unwrap_or_else(|_| {
        panic!(
            "workspace reply: {:?}",
            bridge
                .states
                .borrow()
                .iter()
                .map(|state| (&state.name, state.snapshot.phase, &state.snapshot.failure))
                .collect::<Vec<_>>()
        )
    })
}

#[tokio::test]
async fn workspace_실제_child의_flat_nested_미지원_오류는_typed_파일_좌표와_순서를_보존한다() {
    tokio::time::timeout(DEADLINE, async {
        for mode in [
            "--native-workspace-symbols",
            "--native-workspace-symbols-nested",
            "--native-workspace-symbols-error",
            "--native-document",
        ] {
            let (fixture, project, store, id, bin) = document_symbol_tests::fixture(mode);
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
            let providers = bridge.workspace_providers(&project);
            assert_eq!(providers.len(), 1);
            let mut state = State::default();
            let now = std::time::Instant::now();
            state.observe(Some(&project), Some(" query "), providers.clone(), now);
            let request = state
                .observe(Some(&project), Some(" query "), providers, now + DEBOUNCE)
                .unwrap();
            bridge.workspace_symbols(request).unwrap();
            loop {
                if let Reply::WorkspaceSymbols { request, result } =
                    next(&mut bridge, &signal).await
                {
                    assert!(state.accept(&request, bridge.workspace_providers(&project), result));
                    break;
                }
            }
            let index = state.index(Some(&project));
            assert!(!index.is_pending);
            let symbols = index.entries.unwrap();
            if mode == "--native-document" || mode == "--native-workspace-symbols-error" {
                assert!(symbols.is_empty());
            } else {
                assert_eq!(symbols.len(), 2);
                assert_eq!(symbols[1].name, "Workspace:query");
                assert_eq!((symbols[0].line, symbols[0].column), (2, 5));
                assert_eq!(symbols[0].container_name, "Container");
                assert!(symbols[0].path.ends_with("bound(a),.rs"));
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
async fn workspace_실제_보류_요청은_검색_변경시_cancel되고_다른_문서_동기화를_막지_않는다() {
    tokio::time::timeout(DEADLINE, async {
        let (fixture, project, mut store, id, bin) =
            document_symbol_tests::fixture("--native-workspace-symbols-wait");
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
        let now = std::time::Instant::now();
        state.observe(
            Some(&project),
            Some("hold"),
            bridge.workspace_providers(&project),
            now,
        );
        let held = state
            .observe(
                Some(&project),
                Some("hold"),
                bridge.workspace_providers(&project),
                now + DEBOUNCE,
            )
            .unwrap();
        bridge.workspace_symbols(held.clone()).unwrap();
        loop {
            if let Reply::Diagnostics { diagnostics, .. } = next(&mut bridge, &signal).await
                && diagnostics
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message == "synthetic workspace held")
            {
                break;
            }
        }
        let DocumentKey::File(path) = &snapshot.key else {
            panic!("expected file fixture")
        };
        let other = path.parent().unwrap().join("other.rs");
        std::fs::write(&other, "other\nline").unwrap();
        let opened = taide_file::service::open_file(&other, &[], false).unwrap();
        let other = store.open_file(other, opened).unwrap();
        bridge
            .sync(project.clone(), store.documents().snapshot(other).unwrap())
            .unwrap();
        loop {
            if bridge
                .states
                .borrow()
                .iter()
                .any(|state| state.open_documents.contains(&other))
            {
                break;
            }
            while bridge.poll().is_some() {}
            signal.notified().await;
        }
        state.observe(
            Some(&project),
            Some("fresh"),
            bridge.workspace_providers(&project),
            now + DEBOUNCE,
        );
        assert!(held.is_cancelled());
        let current = state
            .observe(
                Some(&project),
                Some("fresh"),
                bridge.workspace_providers(&project),
                now + DEBOUNCE * 2,
            )
            .unwrap();
        bridge.workspace_symbols(current).unwrap();
        let mut cancelled = false;
        let mut applied = false;
        while !cancelled || !applied {
            match next(&mut bridge, &signal).await {
                Reply::Diagnostics { diagnostics, .. } => {
                    cancelled |= diagnostics
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.message == "synthetic workspace cancelled");
                }
                Reply::WorkspaceSymbols { request, result } => {
                    assert_eq!(request.query, "fresh");
                    applied = state.accept(&request, bridge.workspace_providers(&project), result);
                }
                _ => {}
            }
        }
        assert_eq!(
            state.index(Some(&project)).entries.unwrap()[1].name,
            "Workspace:fresh"
        );
        bridge.disconnect().await.unwrap();
        fixture.services.tasks.shutdown().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn workspace_실제_여러_root는_등록순서와_부분오류_및_프로젝트_격리를_보존한다() {
    use std::os::unix::fs::PermissionsExt;
    const SESSION_COUNT: usize = 3;
    const EXECUTABLE_MODE: u32 = 0o700;
    tokio::time::timeout(DEADLINE, async {
        for partial_error in [false, true] {
            let (fixture, project, mut store, id, bin) = document_symbol_tests::fixture("--native-workspace-symbols");
            let snapshot = store.documents().snapshot(id).unwrap();
            let DocumentKey::File(path) = &snapshot.key else { panic!("file fixture") };
            let root = path.parent().unwrap();
            std::fs::write(root.join("Cargo.toml"), "[package]\nname='synthetic-first'\nversion='0.0.0'\n").unwrap();
            let second = root.join("second");
            std::fs::create_dir_all(&second).unwrap();
            std::fs::write(second.join("Cargo.toml"), "[package]\nname='synthetic-second'\nversion='0.0.0'\n").unwrap();
            let second_file = second.join("second.rs");
            std::fs::write(&second_file, "class\n  method\nend").unwrap();
            let opened = taide_file::service::open_file(&second_file, &[], false).unwrap();
            let second_id = store.open_file(second_file.clone(), opened).unwrap();
            let mock = std::env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("examples/native-lsp-mock");
            let mock = mock.to_str().unwrap().replace('\'', "'\\''");
            let second_mode = if partial_error { "--native-workspace-symbols-error" } else { "--native-workspace-symbols" };
            let executable = std::path::PathBuf::from(&bin).join("rust-analyzer");
            std::fs::write(&executable, format!("#!/bin/sh\ncase \"$PWD\" in\n*/second) exec '{mock}' {second_mode} ;;\n*) exec '{mock}' --native-workspace-symbols ;;\nesac\n")).unwrap();
            std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(EXECUTABLE_MODE)).unwrap();
            let signal = Arc::new(Notify::new());
            let repaint = signal.clone();
            let mut bridge = LspBridge::connect(fixture.services.clone(), bin, Arc::new(move || repaint.notify_one())).unwrap();
            bridge.sync(project.clone(), snapshot.clone()).unwrap();
            running(&mut bridge, &signal, &fixture, 1).await;
            bridge.sync(project.clone(), store.documents().snapshot(second_id).unwrap()).unwrap();
            running(&mut bridge, &signal, &fixture, SESSION_COUNT - 1).await;
            let other_project = ProjectId::new();
            let mut other = fixture.services.state.projects.read()[&project].clone();
            other.id = other_project.clone();
            fixture.services.state.projects.write().insert(other_project.clone(), other);
            bridge.sync(other_project.clone(), snapshot.clone()).unwrap();
            running(&mut bridge, &signal, &fixture, SESSION_COUNT).await;
            let providers = bridge.workspace_providers(&project);
            assert_eq!(providers.len(), SESSION_COUNT - 1);
            assert_eq!(bridge.workspace_providers(&other_project).len(), 1);
            let mut state = State::default();
            let now = std::time::Instant::now();
            state.observe(Some(&project), Some("multi"), providers.clone(), now);
            let request = state.observe(Some(&project), Some("multi"), providers.clone(), now + DEBOUNCE).unwrap();
            bridge.workspace_symbols(request).unwrap();
            loop {
                if let Reply::WorkspaceSymbols { request, result } = next(&mut bridge, &signal).await {
                    let response = result.unwrap();
                    assert_eq!(response.groups.len(), if partial_error { 1 } else { SESSION_COUNT - 1 });
                    assert_eq!(response.groups[0].symbols[0].path, path.to_str().unwrap());
                    if !partial_error { assert_eq!(response.groups[1].symbols[0].path, second_file.to_str().unwrap()); }
                    assert!(state.accept(&request, bridge.workspace_providers(&project), Ok(response)));
                    assert_eq!(state.index(Some(&project)).entries.unwrap().len(), if partial_error { 2 } else { SESSION_COUNT + 1 });
                    break;
                }
            }
            bridge.disconnect().await.unwrap();
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        }
    }).await.unwrap();
}

#[tokio::test]
async fn workspace_실제_재시작은_이전_검색을_만료하고_새_세대의_replay_응답만_적용한다() {
    tokio::time::timeout(DEADLINE, async {
        let (fixture, project, store, id, bin) =
            document_symbol_tests::fixture("--native-workspace-symbols-crash");
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
        let before = bridge.workspace_providers(&project);
        let mut state = State::default();
        let now = std::time::Instant::now();
        state.observe(Some(&project), Some("restart"), before.clone(), now);
        let old = state
            .observe(
                Some(&project),
                Some("restart"),
                before.clone(),
                now + DEBOUNCE,
            )
            .unwrap();
        bridge.workspace_symbols(old.clone()).unwrap();
        let mut late = None;
        loop {
            while let Some(reply) = bridge.poll() {
                match reply {
                    Reply::WorkspaceSymbols { request, result } => {
                        assert_eq!(request.generation, old.generation);
                        late = Some(result);
                    }
                    Reply::Failed { error, .. } => panic!("{error}"),
                    _ => {}
                }
            }
            let providers = bridge.workspace_providers(&project);
            if providers != before
                && bridge
                    .states
                    .borrow()
                    .iter()
                    .all(|entry| entry.snapshot.phase == Phase::Running)
            {
                assert_eq!(providers.len(), 1);
                assert!(
                    providers.iter().all(|identity| before
                        .iter()
                        .any(|old| old.owner == identity.owner
                            && old.generation < identity.generation))
                );
                break;
            }
            signal.notified().await;
        }
        let providers = bridge.workspace_providers(&project);
        state.observe(
            Some(&project),
            Some("fresh"),
            providers.clone(),
            now + DEBOUNCE,
        );
        assert!(old.is_cancelled());
        if let Some(result) = late {
            assert!(!state.accept(&old, providers.clone(), result));
        }
        let current = state
            .observe(Some(&project), Some("fresh"), providers, now + DEBOUNCE * 2)
            .unwrap();
        bridge.workspace_symbols(current).unwrap();
        loop {
            if let Reply::WorkspaceSymbols { request, result } = next(&mut bridge, &signal).await {
                if request.query == "restart" {
                    assert!(!state.accept(&request, bridge.workspace_providers(&project), result));
                    continue;
                }
                assert!(state.accept(&request, bridge.workspace_providers(&project), result));
                break;
            }
        }
        assert_eq!(
            state.index(Some(&project)).entries.unwrap()[1].name,
            "Workspace:fresh"
        );
        bridge.disconnect().await.unwrap();
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}

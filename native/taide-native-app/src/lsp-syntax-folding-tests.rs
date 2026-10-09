use super::*;
use crate::editor_folding::State;
use tokio::sync::Notify;

const DEADLINE: Duration = Duration::from_secs(10);
const DEBOUNCE: Duration = Duration::from_millis(200);

async fn next(bridge: &mut LspBridge, signal: &Notify) -> Reply {
    loop {
        if let Some(reply) = bridge.poll() {
            return reply;
        }
        signal.notified().await;
    }
}

async fn running(bridge: &mut LspBridge, signal: &Notify) {
    loop {
        while let Some(reply) = bridge.poll() {
            if let Reply::Failed { error, .. } = reply {
                panic!("{error}")
            }
        }
        if !bridge.states.borrow().is_empty()
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
}

fn replace(
    store: &mut taide_native_editor::store::EditorStore,
    id: DocumentId,
    prefix: &str,
) -> DocumentSnapshot {
    let snapshot = store.documents().snapshot(id).unwrap();
    store
        .apply(
            id,
            taide_native_editor::store::Transaction {
                revision: snapshot.revision,
                edits: vec![taide_native_editor::document::Edit {
                    bytes: 0..snapshot.rope.len_bytes(),
                    text: format!("{prefix}\n  body\nend"),
                }],
                group: taide_native_editor::document::UndoGroup(snapshot.revision),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
    store.documents().snapshot(id).unwrap()
}

#[tokio::test]
async fn 구문_접기_실제_child는_사용자종류와_빈_null_미지원_오류를_구분한다() {
    tokio::time::timeout(DEADLINE, async {
        for mode in [
            "--native-folding",
            "--native-folding-empty",
            "--native-folding-null",
            "--native-folding-error",
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
            let current = store.documents().snapshot(id).unwrap();
            bridge.sync(project.clone(), current.clone()).unwrap();
            running(&mut bridge, &signal).await;
            let mut state = State::default();
            let providers = bridge.symbol_providers(&project, id);
            let request = state
                .observe(
                    &project,
                    current.clone(),
                    providers.clone(),
                    std::time::Instant::now(),
                )
                .unwrap();
            bridge.syntax_folding(request).unwrap();
            loop {
                if let Reply::SyntaxFolding { request, result } = next(&mut bridge, &signal).await {
                    assert!(state.accept(&request, &current, providers.clone(), result));
                    break;
                }
            }
            let model = state.model(&project, &current);
            match mode {
                "--native-folding" => {
                    let model = model.unwrap();
                    assert_eq!(model.regions().len(), 2);
                    assert_eq!(model.regions()[0].end_line, 2);
                    assert_eq!(model.kind(0), Some("imports"));
                    assert_eq!(model.kind(1), Some("custom"));
                }
                "--native-folding-empty" => assert!(model.unwrap().regions().is_empty()),
                _ => assert!(model.is_none()),
            }
            assert!(state.model(&ProjectId::new(), &current).is_none());
            bridge.disconnect().await.unwrap();
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn 구문_접기_보류_중에도_다른_문서는_동기화되고_편집과_닫힘은_요청을_취소한다() {
    tokio::time::timeout(DEADLINE, async {
        let (fixture, project, mut store, id, bin) =
            document_symbol_tests::fixture("--native-folding-wait");
        let current = replace(&mut store, id, "hold");
        let signal = Arc::new(Notify::new());
        let repaint = signal.clone();
        let mut bridge = LspBridge::connect(
            fixture.services.clone(),
            bin,
            Arc::new(move || repaint.notify_one()),
        )
        .unwrap();
        bridge.sync(project.clone(), current.clone()).unwrap();
        running(&mut bridge, &signal).await;
        let mut state = State::default();
        let old = state
            .observe(
                &project,
                current.clone(),
                bridge.symbol_providers(&project, id),
                std::time::Instant::now(),
            )
            .unwrap();
        bridge.syntax_folding(old.clone()).unwrap();
        loop {
            if let Reply::Diagnostics { diagnostics, .. } = next(&mut bridge, &signal).await
                && diagnostics
                    .diagnostics
                    .iter()
                    .any(|item| item.message == "synthetic folding held")
            {
                break;
            }
        }
        let DocumentKey::File(path) = &current.key else {
            panic!("file expected")
        };
        let other_path = path.parent().unwrap().join("other.rs");
        std::fs::write(&other_path, "class\n  body\nend").unwrap();
        let other_file = taide_file::service::open_file(&other_path, &[], false).unwrap();
        let other = store.open_file(other_path, other_file).unwrap();
        let other_snapshot = store.documents().snapshot(other).unwrap();
        bridge
            .sync(project.clone(), other_snapshot.clone())
            .unwrap();
        while bridge.symbol_providers(&project, other).is_empty() {
            signal.notified().await;
        }
        let second = state
            .observe(
                &project,
                other_snapshot.clone(),
                bridge.symbol_providers(&project, other),
                std::time::Instant::now(),
            )
            .unwrap();
        bridge.syntax_folding(second).unwrap();
        loop {
            if let Reply::SyntaxFolding { request, result } = next(&mut bridge, &signal).await
                && request.snapshot.id == other
            {
                assert_eq!(result.as_ref().unwrap().groups.len(), 1);
                assert!(state.accept(
                    &request,
                    &other_snapshot,
                    bridge.symbol_providers(&project, other),
                    result
                ));
                break;
            }
        }
        let current = replace(&mut store, id, "fresh");
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
        assert!(old.is_cancelled());
        let request = state
            .observe(
                &project,
                current.clone(),
                bridge.symbol_providers(&project, id),
                now + DEBOUNCE,
            )
            .unwrap();
        bridge.syntax_folding(request.clone()).unwrap();
        loop {
            if let Reply::SyntaxFolding { request, result } = next(&mut bridge, &signal).await
                && request.snapshot.id == id
            {
                assert!(state.accept(
                    &request,
                    &current,
                    bridge.symbol_providers(&project, id),
                    result
                ));
                break;
            }
        }
        assert_eq!(
            state.model(&project, &current).unwrap().regions()[0].end_line,
            2
        );
        assert!(!state.accept(
            &old,
            &current,
            bridge.symbol_providers(&project, id),
            Ok(crate::editor_folding::Response { groups: vec![] })
        ));
        let held = replace(&mut store, id, "hold");
        bridge.sync(project.clone(), held.clone()).unwrap();
        let now = std::time::Instant::now();
        state.observe(
            &project,
            held.clone(),
            bridge.symbol_providers(&project, id),
            now,
        );
        let closing = state
            .observe(
                &project,
                held.clone(),
                bridge.symbol_providers(&project, id),
                now + DEBOUNCE,
            )
            .unwrap();
        bridge.syntax_folding(closing.clone()).unwrap();
        loop {
            if let Reply::Diagnostics { diagnostics, .. } = next(&mut bridge, &signal).await
                && diagnostics
                    .diagnostics
                    .iter()
                    .any(|item| item.message == "synthetic folding held")
            {
                break;
            }
        }
        state.retain(&HashSet::new());
        assert!(closing.is_cancelled());
        assert!(state.model(&project, &held).is_none());
        bridge.retain_bindings(&HashSet::new()).unwrap();
        bridge.disconnect().await.unwrap();
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn 구문_접기_실제_서버_재시작은_새_세대의_mirror만_적용한다() {
    tokio::time::timeout(DEADLINE, async {
        let (fixture, project, mut store, id, bin) =
            document_symbol_tests::fixture("--native-folding-crash");
        let current = replace(&mut store, id, "crash");
        let signal = Arc::new(Notify::new());
        let repaint = signal.clone();
        let mut bridge = LspBridge::connect(
            fixture.services.clone(),
            bin,
            Arc::new(move || repaint.notify_one()),
        )
        .unwrap();
        bridge.sync(project.clone(), current.clone()).unwrap();
        running(&mut bridge, &signal).await;
        let before = bridge.symbol_providers(&project, id);
        let mut state = State::default();
        let old = state
            .observe(&project, current, before.clone(), std::time::Instant::now())
            .unwrap();
        bridge.syntax_folding(old.clone()).unwrap();
        loop {
            while let Some(reply) = bridge.poll() {
                if let Reply::Failed { error, .. } = reply {
                    panic!("{error}")
                }
            }
            let providers = bridge.symbol_providers(&project, id);
            if !providers.is_empty() && providers != before {
                assert!(providers.iter().all(|current| {
                    before.iter().any(|old| {
                        old.owner == current.owner && old.generation < current.generation
                    })
                }));
                break;
            }
            signal.notified().await;
        }
        let current = replace(&mut store, id, "fresh");
        bridge.sync(project.clone(), current.clone()).unwrap();
        let providers = bridge.symbol_providers(&project, id);
        let now = std::time::Instant::now();
        assert!(
            state
                .observe(&project, current.clone(), providers.clone(), now)
                .is_none()
        );
        assert!(old.is_cancelled());
        let request = state
            .observe(&project, current.clone(), providers.clone(), now + DEBOUNCE)
            .unwrap();
        bridge.syntax_folding(request).unwrap();
        loop {
            if let Reply::SyntaxFolding { request, result } = next(&mut bridge, &signal).await {
                if request.generation == old.generation {
                    continue;
                }
                assert!(state.accept(&request, &current, providers.clone(), result));
                break;
            }
        }
        assert_eq!(
            state.model(&project, &current).unwrap().kind(0),
            Some("imports")
        );
        bridge.disconnect().await.unwrap();
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn 구문_접기_실제_child는_같은_문서의_다른_프로젝트_owner를_섞지_않는다() {
    tokio::time::timeout(DEADLINE, async {
        let (fixture, project, store, id, bin) = document_symbol_tests::fixture("--native-folding");
        let other = ProjectId::new();
        let mut entry = fixture.services.state.projects.read()[&project].clone();
        entry.id = other.clone();
        fixture
            .services
            .state
            .projects
            .write()
            .insert(other.clone(), entry);
        let current = store.documents().snapshot(id).unwrap();
        let signal = Arc::new(Notify::new());
        let repaint = signal.clone();
        let mut bridge = LspBridge::connect(
            fixture.services.clone(),
            bin,
            Arc::new(move || repaint.notify_one()),
        )
        .unwrap();
        bridge.sync(project.clone(), current.clone()).unwrap();
        bridge.sync(other.clone(), current.clone()).unwrap();
        loop {
            running(&mut bridge, &signal).await;
            if bridge.states.borrow().len() == 2 {
                break;
            }
            signal.notified().await;
        }
        let providers = bridge.symbol_providers(&project, id);
        let other_providers = bridge.symbol_providers(&other, id);
        assert_eq!(providers.len(), 1);
        assert_eq!(other_providers.len(), 1);
        assert!(providers.is_disjoint(&other_providers));
        let mut state = State::default();
        let request = state
            .observe(
                &project,
                current.clone(),
                providers.clone(),
                std::time::Instant::now(),
            )
            .unwrap();
        bridge.syntax_folding(request).unwrap();
        loop {
            if let Reply::SyntaxFolding { request, result } = next(&mut bridge, &signal).await {
                let response = result.unwrap();
                assert_eq!(response.groups.len(), 1);
                assert!(providers.contains(&response.groups[0].provider));
                assert!(state.accept(&request, &current, providers.clone(), Ok(response)));
                break;
            }
        }
        assert!(state.model(&other, &current).is_none());
        bridge.disconnect().await.unwrap();
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}

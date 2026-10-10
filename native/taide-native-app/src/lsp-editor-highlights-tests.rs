use super::*;
use crate::editor_highlights::{Context, State};
use std::time::Instant;
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::view::{Selection, SelectionSet, ViewKey};
use tokio::sync::Notify;

const DEADLINE: Duration = Duration::from_secs(10);
const DEBOUNCE: Duration = Duration::from_millis(50);
const CARET_BYTE: usize = 13;
const READY_DEADLINE: Duration = Duration::from_secs(5);

#[tokio::test]
async fn highlight_tier_실제_child의_남은_공급자는_크기_등급_차단과_정상_복원을_우회하지_못한다() {
    tokio::time::timeout(DEADLINE, async {
        let (fixture, project, mut store, id, bin) =
            document_symbol_tests::fixture("--native-highlights");
        let original = store.documents().snapshot(id).unwrap();
        let DocumentKey::File(path) = &original.key else {
            panic!("file fixture");
        };
        let normal_file = taide_file::service::open_file(path, &[], false).unwrap();
        let view = store
            .attach_view(
                ViewKey {
                    window: "synthetic".into(),
                    pane: PaneId::new(),
                    tab: TabId::new(),
                },
                id,
            )
            .unwrap();
        store
            .set_view_state(
                view,
                SelectionSet {
                    primary: 0,
                    selections: vec![Selection {
                        anchor: CARET_BYTE,
                        head: CARET_BYTE,
                    }],
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
        bridge.sync(project.clone(), original.clone()).unwrap();
        running(&mut bridge, &signal, &fixture, 1).await;
        let context = || Context {
            project: project.clone(),
            source: view,
            owner: view,
            viewport: eframe::egui::ViewportId::ROOT,
        };
        let mut state = State::default();
        for tier in [FileSizeTier::Large, FileSizeTier::ReadOnly] {
            let snapshot = store.documents().snapshot(id).unwrap();
            let providers = bridge.highlight_providers(&project, &snapshot);
            assert!(!providers.is_empty());
            let now = Instant::now();
            assert!(
                state
                    .observe(&store, context(), providers.clone(), now)
                    .unwrap()
                    .is_none()
            );
            let old = state
                .observe(&store, context(), providers.clone(), now + DEBOUNCE)
                .unwrap()
                .unwrap();
            bridge.highlights(old.clone()).unwrap();
            loop {
                if let Reply::Highlights { request, result } = next(&mut bridge, &signal).await {
                    assert!(state.accept(&store, &request, providers.clone(), result));
                    break;
                }
            }
            assert!(state.has_highlights(&store, eframe::egui::ViewportId::ROOT, view));
            let mut restricted_file = normal_file.clone();
            restricted_file.tier = tier;
            restricted_file.read_only = tier == FileSizeTier::ReadOnly;
            store.observe_file(id, path, restricted_file).unwrap();
            let restricted = store.documents().snapshot(id).unwrap();
            assert_eq!(restricted.revision, snapshot.revision);
            assert!(
                bridge.highlight_providers(&project, &restricted).is_empty(),
                "{tier:?}: old child is still connected"
            );
            state.reconcile(
                &store,
                |_| true,
                |project, snapshot| bridge.highlight_providers(project, snapshot),
            );
            assert!(old.is_cancelled());
            assert!(!state.has_highlights(&store, eframe::egui::ViewportId::ROOT, view));
            assert!(!state.accept(
                &store,
                &old,
                HashSet::new(),
                Ok(crate::editor_highlights::Response {
                    provider: None,
                    highlights: Vec::new()
                })
            ));
            bridge.sync(project.clone(), restricted).unwrap();
            loop {
                while bridge.poll().is_some() {}
                if bridge
                    .states
                    .borrow()
                    .iter()
                    .all(|entry| !entry.open_documents.contains(&id))
                {
                    break;
                }
                signal.notified().await;
            }
            let mut restored_file = normal_file.clone();
            restored_file.read_only = true;
            store.observe_file(id, path, restored_file).unwrap();
            let restored = store.documents().snapshot(id).unwrap();
            assert_eq!(restored.metadata.tier, FileSizeTier::Normal);
            assert!(restored.metadata.read_only);
            bridge.sync(project.clone(), restored.clone()).unwrap();
            loop {
                while bridge.poll().is_some() {}
                if !bridge.highlight_providers(&project, &restored).is_empty() {
                    break;
                }
                signal.notified().await;
            }
            assert!(!state.accept(
                &store,
                &old,
                bridge.highlight_providers(&project, &restored),
                Ok(crate::editor_highlights::Response {
                    provider: None,
                    highlights: Vec::new()
                })
            ));
        }
        let snapshot = store.documents().snapshot(id).unwrap();
        let providers = bridge.highlight_providers(&project, &snapshot);
        let now = Instant::now();
        assert!(
            state
                .observe(&store, context(), providers.clone(), now)
                .unwrap()
                .is_none()
        );
        let restored = state
            .observe(&store, context(), providers.clone(), now + DEBOUNCE)
            .unwrap()
            .unwrap();
        bridge.highlights(restored).unwrap();
        loop {
            if let Reply::Highlights { request, result } = next(&mut bridge, &signal).await {
                assert!(state.accept(&store, &request, providers.clone(), result));
                break;
            }
        }
        assert!(state.has_highlights(&store, eframe::egui::ViewportId::ROOT, view));
        bridge.disconnect().await.unwrap();
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn multihighlight_실제_공급자_우선순위_빈_null_오류_대체와_프로젝트_소유를_보존한다() {
    use std::os::unix::fs::PermissionsExt;
    const EXECUTABLE_MODE: u32 = 0o700;
    const PROVIDER_COUNT: usize = 2;
    const PROJECT_COUNT: usize = 2;
    for (mode, expected_id, expected_highlights) in [
        ("--native-highlights-alternate", "ruff", 1),
        ("--native-highlights-empty", "ruff", 0),
        ("--native-highlights-null", "ruff", 0),
        ("--native-highlights-error", "basedPyright", 3),
        ("--native-highlights-bad", "basedPyright", 3),
        ("--native-highlights-unsupported", "basedPyright", 3),
    ] {
        tokio::time::timeout(DEADLINE, async {
            let (fixture, project, mut store, id, bin) =
                document_symbol_tests::fixture("--native-highlights");
            let rust = store.documents().snapshot(id).unwrap();
            let DocumentKey::File(path) = &rust.key else {
                panic!("file fixture");
            };
            let python = path.parent().unwrap().join("source.py");
            std::fs::write(&python, "class\n  \u{1f600}method\nend").unwrap();
            let file = taide_file::service::open_file(&python, &[], false).unwrap();
            let id = store.open_file(python, file).unwrap();
            let snapshot = store.documents().snapshot(id).unwrap();
            assert_eq!(snapshot.metadata.language_id, "python");
            let view = store
                .attach_view(
                    ViewKey {
                        window: "synthetic".into(),
                        pane: PaneId::new(),
                        tab: TabId::new(),
                    },
                    id,
                )
                .unwrap();
            let mock = std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("examples/native-lsp-mock");
            let escaped = mock.to_str().unwrap().replace('\'', "'\\''");
            for (binary, mode) in [
                ("basedpyright-langserver", "--native-highlights"),
                ("ruff", mode),
            ] {
                let path = std::path::PathBuf::from(&bin).join(binary);
                std::fs::write(&path, format!("#!/bin/sh\nexec '{escaped}' {mode}\n")).unwrap();
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
            running(
                &mut bridge,
                &signal,
                &fixture,
                PROVIDER_COUNT * PROJECT_COUNT,
            )
            .await;
            let providers = bridge.highlight_providers(&project, &snapshot);
            let foreign = bridge.highlight_providers(&other, &snapshot);
            let expected_count = if mode == "--native-highlights-unsupported" {
                PROVIDER_COUNT - 1
            } else {
                PROVIDER_COUNT
            };
            assert_eq!(providers.len(), expected_count, "{mode}");
            assert_eq!(foreign.len(), expected_count);
            assert!(providers.is_disjoint(&foreign));
            let expected_name = taide_lsp::manifest::servers()
                .iter()
                .find(|spec| spec.id.0 == expected_id)
                .unwrap()
                .name
                .clone();
            let expected_owner = bridge
                .states
                .borrow()
                .iter()
                .find(|entry| entry.project == project && entry.name == expected_name)
                .unwrap()
                .owner;
            let mut state = State::default();
            let context = || Context {
                project: project.clone(),
                owner: view,
                source: view,
                viewport: eframe::egui::ViewportId::ROOT,
            };
            let now = Instant::now();
            state
                .observe(&store, context(), providers.clone(), now)
                .unwrap();
            let request = state
                .observe(&store, context(), providers.clone(), now + DEBOUNCE)
                .unwrap()
                .unwrap();
            bridge.highlights(request.clone()).unwrap();
            loop {
                if let Reply::Highlights {
                    request: returned,
                    result,
                } = next(&mut bridge, &signal).await
                {
                    let response = result.unwrap();
                    assert_eq!(response.provider.unwrap().owner, expected_owner, "{mode}");
                    assert_eq!(response.highlights.len(), expected_highlights, "{mode}");
                    assert!(state.accept(&store, &returned, providers.clone(), Ok(response)));
                    break;
                }
            }
            bridge.disconnect().await.unwrap();
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        })
        .await
        .unwrap();
    }
}

async fn next(bridge: &mut LspBridge, signal: &Notify) -> Reply {
    loop {
        if let Some(reply) = bridge.poll() {
            return reply;
        }
        signal.notified().await;
    }
}

async fn running(
    bridge: &mut LspBridge,
    signal: &Notify,
    fixture: &document_symbol_tests::Fixture,
    expected_count: usize,
) {
    let ready = tokio::time::timeout(READY_DEADLINE, async {
        loop {
            while let Some(reply) = bridge.poll() {
                if let Reply::Failed { error, .. } = reply {
                    panic!("{error}");
                }
            }
            let ready = {
                let states = bridge.states.borrow();
                states.len() == expected_count
                    && states
                        .iter()
                        .all(|state| state.snapshot.phase == Phase::Running)
            };
            if ready {
                return;
            }
            signal.notified().await;
        }
    })
    .await;
    assert!(
        ready.is_ok(),
        "highlight readiness: {:?}; stderr={}",
        bridge
            .states
            .borrow()
            .iter()
            .map(|state| (
                &state.name,
                state.snapshot.phase,
                state.open_documents.len()
            ))
            .collect::<Vec<_>>(),
        std::fs::read_to_string(fixture.directory.join("stderr.log")).unwrap_or_default()
    );
}

#[tokio::test]
async fn 실제_child와_문서_mirror의_utf16_하이라이트는_정상_빈_null_오류_미지원_응답을_구분한다() {
    let mut phase = String::new();
    let result = tokio::time::timeout(DEADLINE, async {
        for mode in [
            "--native-highlights",
            "--native-highlights-empty",
            "--native-highlights-null",
            "--native-highlights-error",
            "--native-highlights-bad",
            "--native-highlights-unsupported",
        ] {
            let (fixture, project, mut store, id, bin) = document_symbol_tests::fixture(mode);
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
            phase = format!("{mode}:initialize");
            running(&mut bridge, &signal, &fixture, 1).await;
            let view = store
                .attach_view(
                    ViewKey {
                        window: "synthetic".into(),
                        pane: PaneId::new(),
                        tab: TabId::new(),
                    },
                    id,
                )
                .unwrap();
            store
                .set_view_state(
                    view,
                    SelectionSet {
                        primary: 0,
                        selections: vec![Selection {
                            anchor: CARET_BYTE,
                            head: CARET_BYTE,
                        }],
                    },
                    Default::default(),
                    Vec::new(),
                )
                .unwrap();
            let providers = bridge.highlight_providers(&project, &snapshot);
            let context = || Context {
                project: project.clone(),
                source: view,
                owner: view,
                viewport: eframe::egui::ViewportId::ROOT,
            };
            let mut state = State::default();
            let now = Instant::now();
            assert!(
                state
                    .observe(&store, context(), providers.clone(), now)
                    .unwrap()
                    .is_none()
            );
            if mode == "--native-highlights-unsupported" {
                assert!(providers.is_empty());
                assert!(
                    state
                        .observe(&store, context(), providers, now + DEBOUNCE)
                        .unwrap()
                        .is_none()
                );
            } else {
                assert!(!providers.is_empty());
                let request = state
                    .observe(&store, context(), providers.clone(), now + DEBOUNCE)
                    .unwrap()
                    .unwrap();
                assert_eq!(request.position, lsp_types::Position::new(1, 5));
                phase = format!("{mode}:highlights");
                bridge.highlights(request.clone()).unwrap();
                loop {
                    if let Reply::Highlights {
                        request: returned,
                        result,
                    } = next(&mut bridge, &signal).await
                    {
                        assert_eq!(returned.token, request.token);
                        let result = result.unwrap();
                        if mode == "--native-highlights" {
                            assert_eq!(result.highlights.len(), 3);
                            assert_eq!(result.highlights[0].kind, None);
                            assert_eq!(
                                result.highlights[1].kind,
                                Some(lsp_types::DocumentHighlightKind::READ)
                            );
                            assert_eq!(
                                result.highlights[2].kind,
                                Some(lsp_types::DocumentHighlightKind::WRITE)
                            );
                            assert!(
                                result
                                    .provider
                                    .is_some_and(|provider| providers.contains(&provider))
                            );
                        } else {
                            assert!(result.highlights.is_empty());
                        }
                        assert!(state.accept(&store, &returned, providers.clone(), Ok(result)));
                        break;
                    }
                }
                assert!(
                    state
                        .observe(&store, context(), providers, now + DEBOUNCE)
                        .unwrap()
                        .is_none()
                );
            }
            phase = format!("{mode}:disconnect");
            bridge.disconnect().await.unwrap();
        }
    })
    .await;
    assert!(result.is_ok(), "phase={phase}");
}

#[tokio::test]
async fn 실제_child의_대기_요청은_뷰_회수로_취소되고_정상_종료된다() {
    let mut phase = "initialize";
    let result = tokio::time::timeout(DEADLINE, async {
        let (fixture, project, mut store, id, bin) =
            document_symbol_tests::fixture("--native-highlights-wait");
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
        let view = store
            .attach_view(
                ViewKey {
                    window: "synthetic".into(),
                    pane: PaneId::new(),
                    tab: TabId::new(),
                },
                id,
            )
            .unwrap();
        let providers = bridge.highlight_providers(&project, &snapshot);
        let context = || Context {
            project: project.clone(),
            source: view,
            owner: view,
            viewport: eframe::egui::ViewportId::ROOT,
        };
        let now = Instant::now();
        let mut state = State::default();
        state
            .observe(&store, context(), providers.clone(), now)
            .unwrap();
        let request = state
            .observe(&store, context(), providers.clone(), now + DEBOUNCE)
            .unwrap()
            .unwrap();
        phase = "held request";
        bridge.highlights(request.clone()).unwrap();
        loop {
            if let Reply::Diagnostics { diagnostics, .. } = next(&mut bridge, &signal).await
                && diagnostics
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message == "synthetic highlights held")
            {
                break;
            }
        }
        store.detach_view(view).unwrap();
        state.reconcile(&store, |_| true, |_, _| providers.clone());
        assert!(request.is_cancelled());
        phase = "disconnect";
        bridge.disconnect().await.unwrap();
    })
    .await;
    assert!(result.is_ok(), "phase={phase}");
}

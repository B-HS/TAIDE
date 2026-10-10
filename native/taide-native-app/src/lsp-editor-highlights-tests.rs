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
                !states.is_empty()
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
            running(&mut bridge, &signal, &fixture).await;
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
        running(&mut bridge, &signal, &fixture).await;
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

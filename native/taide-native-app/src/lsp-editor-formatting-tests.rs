use super::*;
use crate::editor_formatting::State;
use taide_native_editor::formatting::Command;
use taide_native_editor::indent::{IndentConfiguration, IndentOptions, IndentationChange};
use taide_native_editor::view::{Selection, SelectionSet, ViewKey};
use tokio::sync::Notify;

const DEADLINE: Duration = Duration::from_secs(10);
const EDIT_SIZE: u32 = 2;
const DISPLAY_SIZE: u32 = 8;

async fn next(bridge: &mut LspBridge, signal: &Notify) -> Reply {
    loop {
        if let Some(reply) = bridge.poll() {
            return reply;
        }
        signal.notified().await;
    }
}

async fn providers(
    bridge: &mut LspBridge,
    signal: &Notify,
    project: &ProjectId,
    snapshot: &DocumentSnapshot,
    command: Command,
) -> HashSet<crate::editor_symbols::ProviderIdentity> {
    loop {
        while let Some(reply) = bridge.poll() {
            if let Reply::Failed { error, .. } = reply {
                panic!("{error}");
            }
        }
        let providers = bridge.formatting_providers(project, snapshot, command);
        if !providers.is_empty() {
            return providers;
        }
        signal.notified().await;
    }
}

#[tokio::test]
async fn 실제_child의_문서_선택과_범위_대체_겹침은_최신_utf16과_편집_폭으로_완료한다() {
    for (mode, command) in [
        ("--native-format-options", Command::Document),
        ("--native-format-options", Command::Selection),
        ("--native-format-range-only", Command::Document),
        ("--native-format-range-overlap", Command::Selection),
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
            let configuration = IndentConfiguration {
                defaults: IndentOptions {
                    tab_size: EDIT_SIZE,
                    insert_spaces: false,
                },
                detect_indentation: false,
            };
            store
                .set_indentation(
                    document,
                    configuration,
                    IndentationChange::UseTabs(EDIT_SIZE),
                )
                .unwrap();
            store
                .set_indentation(
                    document,
                    configuration,
                    IndentationChange::DisplaySize(DISPLAY_SIZE),
                )
                .unwrap();
            let before = store.documents().snapshot(document).unwrap();
            let text = before.rope.to_string();
            let start = text.find("method").unwrap();
            let selections = if mode == "--native-format-range-overlap" {
                vec![
                    Selection {
                        anchor: 0,
                        head: "class".len(),
                    },
                    Selection {
                        anchor: start,
                        head: start + "method".len(),
                    },
                ]
            } else {
                vec![Selection {
                    anchor: start,
                    head: start + "method".len(),
                }]
            };
            store
                .set_view_state(
                    view,
                    SelectionSet {
                        primary: 0,
                        selections,
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
            let providers = providers(&mut bridge, &signal, &project, &before, command).await;
            let mut state = State::default();
            let request = state
                .begin(
                    &store,
                    project.clone(),
                    view,
                    view,
                    command,
                    configuration.defaults,
                    providers.clone(),
                )
                .unwrap()
                .unwrap();
            bridge.format_editor(request).unwrap();
            loop {
                match next(&mut bridge, &signal).await {
                    Reply::EditorFormatted { request, result } => {
                        assert!(
                            state
                                .accept(&mut store, &request, providers, result)
                                .unwrap(),
                            "{mode}"
                        );
                        break;
                    }
                    Reply::Failed { error, .. } => panic!("{mode}: {error}"),
                    _ => {}
                }
            }
            let expected = if command == Command::Selection && mode == "--native-format-options" {
                text.replacen("method", "range:2:false:method", 1)
            } else if mode == "--native-format-options" {
                format!("formatted:2:false:{text}")
            } else {
                format!("range:2:false:{text}")
            };
            assert_eq!(
                store
                    .documents()
                    .snapshot(document)
                    .unwrap()
                    .rope
                    .to_string(),
                expected,
                "{mode}"
            );
            assert!(store.undo(document).unwrap());
            assert_eq!(
                store.documents().snapshot(document).unwrap().rope,
                before.rope
            );
            bridge.disconnect().await.unwrap();
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        })
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn 실제_child의_대기_포맷은_커서_변경에서_취소되고_늦은_응답은_적용되지_않는다() {
    let mut phase = "initialize";
    let result = tokio::time::timeout(DEADLINE, async {
        let (fixture, project, mut store, document, bin) =
            document_symbol_tests::fixture("--native-format-wait");
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
        let signal = Arc::new(Notify::new());
        let repaint = signal.clone();
        let mut bridge = LspBridge::connect(
            fixture.services.clone(),
            bin,
            Arc::new(move || repaint.notify_one()),
        )
        .unwrap();
        bridge.sync(project.clone(), before.clone()).unwrap();
        let providers = providers(&mut bridge, &signal, &project, &before, Command::Document).await;
        let mut state = State::default();
        let request = state
            .begin(
                &store,
                project,
                view,
                view,
                Command::Document,
                IndentOptions {
                    tab_size: EDIT_SIZE,
                    insert_spaces: true,
                },
                providers.clone(),
            )
            .unwrap()
            .unwrap();
        bridge.format_editor(request.clone()).unwrap();
        for expected in ["synthetic format held", "synthetic format cancelled"] {
            phase = expected;
            loop {
                match next(&mut bridge, &signal).await {
                    Reply::Diagnostics { diagnostics, .. }
                        if diagnostics
                            .diagnostics
                            .iter()
                            .any(|diagnostic| diagnostic.message == expected) =>
                    {
                        break;
                    }
                    Reply::EditorFormatted { .. } => {
                        panic!("cancelled format must not reach the application")
                    }
                    Reply::Failed { error, .. } => panic!("{error}"),
                    _ => {}
                }
            }
            store
                .set_view_state(
                    view,
                    SelectionSet {
                        primary: 0,
                        selections: vec![Selection { anchor: 1, head: 1 }],
                    },
                    Default::default(),
                    Vec::new(),
                )
                .unwrap();
            state.reconcile(&store, |_| providers.clone());
        }
        assert!(request.is_cancelled());
        assert_eq!(
            store.documents().snapshot(document).unwrap().rope,
            before.rope
        );
        phase = "disconnect";
        bridge.disconnect().await.unwrap();
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await;
    assert!(result.is_ok(), "phase={phase}");
}

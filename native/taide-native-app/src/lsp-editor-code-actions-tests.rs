use super::*;
use crate::editor_code_actions::{Invocation, State};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{Selection, SelectionSet, ViewId, ViewKey};
use taide_native_ui::editor_code_actions::Command as ActionCommand;
use tokio::sync::Notify;

const DEADLINE: Duration = Duration::from_secs(10);

fn source(store: &mut EditorStore, document: DocumentId) -> ViewId {
    let view = store
        .attach_view(
            ViewKey {
                window: "synthetic-code-actions".into(),
                pane: taide_model::ids::PaneId::new(),
                tab: taide_model::ids::TabId::new(),
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
    let start = text.find("method").unwrap();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: start + "method".len(),
                    head: start,
                }],
            },
            Default::default(),
            Vec::new(),
        )
        .unwrap();
    view
}

async fn ready(
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
        let providers = bridge.code_action_providers(project, snapshot, ActionCommand::QuickFix);
        if !providers.is_empty()
            && bridge
                .code_action_diagnostics(project, snapshot.id)
                .values()
                .any(|revision| *revision != 0)
        {
            return providers;
        }
        signal.notified().await;
    }
}

#[tokio::test]
async fn 실제_child의_종류_진단_원본_data와_선택_시_resolve는_문서_변경_없이_보존된다() {
    tokio::time::timeout(DEADLINE, async {
        let (fixture, project, mut store, document, bin) =
            document_symbol_tests::fixture("--native-code-actions");
        let view = source(&mut store, document);
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
        let providers = ready(&mut bridge, &signal, &project, &before).await;
        for (command, titles) in [
            (
                ActionCommand::QuickFix,
                vec![
                    "Resolved fix",
                    "Direct fix",
                    "Disabled fix",
                    "Extract",
                    "Bare command",
                    "Client references",
                ],
            ),
            (ActionCommand::Refactor, vec!["Extract"]),
            (ActionCommand::Source, vec!["Imports", "Fix all"]),
            (ActionCommand::OrganizeImports, vec!["Imports"]),
            (ActionCommand::FixAll, vec!["Fix all"]),
            (ActionCommand::AutoFix, vec!["Resolved fix"]),
        ] {
            let mut state = State::default();
            let request = state
                .begin(
                    &store,
                    project.clone(),
                    view,
                    view,
                    providers.clone(),
                    Invocation {
                        command,
                        automatic: false,
                    },
                )
                .unwrap()
                .unwrap();
            assert_eq!(
                request.range,
                lsp_types::Range::new(
                    lsp_types::Position::new(1, 4),
                    lsp_types::Position::new(1, 10)
                )
            );
            let raw = bridge
                .code_actions(request.clone())
                .unwrap()
                .await
                .unwrap()
                .unwrap();
            assert_eq!(raw.len(), 8);
            let lsp_types::CodeActionOrCommand::CodeAction(first) = &raw[0].raw else {
                panic!("typed action");
            };
            assert_eq!(first.data.as_ref().unwrap()["payload"], "preserved");
            assert_eq!(
                first.diagnostics.as_ref().unwrap()[0]
                    .data
                    .as_ref()
                    .unwrap()["payload"],
                "diagnostic retained"
            );
            let automatic = state
                .accept(&store, &request, providers.clone(), Ok(raw))
                .unwrap();
            let chosen = if let Some(chosen) = automatic {
                assert!(command.auto_apply_single());
                chosen
            } else {
                let (_, listed, _) = state.listed(&store, view).unwrap();
                assert_eq!(
                    listed
                        .iter()
                        .map(|action| action.title.as_str())
                        .collect::<Vec<_>>(),
                    titles
                );
                state
                    .select(&store, view, request.token.as_u128(), 0, false)
                    .unwrap()
            };
            assert_eq!(chosen.candidate.presentation().title, titles[0]);
            let resolved = bridge
                .resolve_code_action(chosen)
                .unwrap()
                .await
                .unwrap()
                .unwrap();
            assert!(resolved.edit.is_some());
            assert_eq!(
                store.documents().snapshot(document).unwrap().rope,
                before.rope
            );
        }
        bridge.disconnect().await.unwrap();
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn 실제_child의_null_오류_잘못된_응답과_resolve_오류는_빈_목록과_원본으로_대체된다() {
    for mode in [
        "--native-code-actions-null",
        "--native-code-actions-error",
        "--native-code-actions-bad",
        "--native-code-actions-resolve-error",
    ] {
        tokio::time::timeout(DEADLINE, async {
            let (fixture, project, mut store, document, bin) = document_symbol_tests::fixture(mode);
            let view = source(&mut store, document);
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
            let providers = ready(&mut bridge, &signal, &project, &before).await;
            let mut state = State::default();
            let request = state
                .begin(
                    &store,
                    project,
                    view,
                    view,
                    providers.clone(),
                    Invocation {
                        command: ActionCommand::QuickFix,
                        automatic: false,
                    },
                )
                .unwrap()
                .unwrap();
            let raw = bridge
                .code_actions(request.clone())
                .unwrap()
                .await
                .unwrap()
                .unwrap();
            if mode == "--native-code-actions-resolve-error" {
                state.accept(&store, &request, providers, Ok(raw)).unwrap();
                let chosen = state
                    .select(&store, view, request.token.as_u128(), 0, false)
                    .unwrap();
                let resolved = bridge
                    .resolve_code_action(chosen)
                    .unwrap()
                    .await
                    .unwrap()
                    .unwrap();
                assert!(resolved.edit.is_none());
                assert!(resolved.command.is_none());
            } else {
                assert!(raw.is_empty(), "{mode}");
            }
            assert_eq!(
                store.documents().snapshot(document).unwrap().revision,
                before.revision
            );
            bridge.disconnect().await.unwrap();
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        })
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn 실제_child의_대기_요청은_취소되고_늦은_응답을_선택하지_않는다() {
    tokio::time::timeout(DEADLINE, async {
        let (fixture, project, mut store, document, bin) =
            document_symbol_tests::fixture("--native-code-actions-wait");
        let view = source(&mut store, document);
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
        let providers = ready(&mut bridge, &signal, &project, &before).await;
        let mut state = State::default();
        let request = state
            .begin(
                &store,
                project,
                view,
                view,
                providers.clone(),
                Invocation {
                    command: ActionCommand::QuickFix,
                    automatic: false,
                },
            )
            .unwrap()
            .unwrap();
        let response = bridge.code_actions(request.clone()).unwrap();
        loop {
            let mut held = false;
            while let Some(reply) = bridge.poll() {
                if let Reply::Diagnostics { diagnostics, .. } = reply {
                    held |= diagnostics
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.message == "synthetic action held");
                }
            }
            if held {
                break;
            }
            signal.notified().await;
        }
        state.clear();
        assert!(matches!(response.await.unwrap(), Err(Failure::Cancelled)));
        assert!(
            state
                .accept(&store, &request, providers, Ok(Vec::new()))
                .unwrap()
                .is_none()
        );
        assert!(state.listed(&store, view).is_none());
        bridge.disconnect().await.unwrap();
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}

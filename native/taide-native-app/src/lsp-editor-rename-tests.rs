use super::*;
use crate::editor_rename::{Response, State};
use taide_native_editor::view::{Selection, SelectionSet, ViewKey};
use tokio::sync::Notify;

const DEADLINE: Duration = Duration::from_secs(10);
const SERVER_ERROR_CODE: i64 = -32603;

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

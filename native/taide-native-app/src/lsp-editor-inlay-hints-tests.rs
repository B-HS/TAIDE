use super::*;
use crate::editor_inlay_hints::{Context, Request, State};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::Instant;
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{ViewId, ViewKey};
use tokio::sync::Notify;

const DEADLINE: Duration = Duration::from_secs(10);
const DEBOUNCE: Duration = Duration::from_millis(25);
const EXECUTABLE_MODE: u32 = 0o700;

fn view(store: &mut EditorStore, document: DocumentId) -> ViewId {
    store
        .attach_view(
            ViewKey {
                window: "inlay-child".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap()
}

fn context(project: &ProjectId, view: ViewId) -> Context {
    Context {
        project: project.clone(),
        owner: view,
        source: view,
        viewport: eframe::egui::ViewportId::ROOT,
        lines: 0..1,
    }
}

async fn ready(
    bridge: &mut LspBridge,
    signal: &Notify,
    project: &ProjectId,
    snapshot: &DocumentSnapshot,
    count: usize,
) -> HashSet<crate::editor_symbols::ProviderIdentity> {
    loop {
        while let Some(reply) = bridge.poll() {
            if let Reply::Failed { error, .. } = reply {
                panic!("{error}");
            }
        }
        let providers = bridge.inlay_hint_providers(project, snapshot);
        if providers.len() == count && !providers.is_empty() {
            return providers;
        }
        signal.notified().await;
    }
}

async fn response(
    bridge: &mut LspBridge,
    signal: &Notify,
    request: &Request,
) -> Result<crate::editor_inlay_hints::Response, Failure> {
    loop {
        while let Some(reply) = bridge.poll() {
            match reply {
                Reply::Failed { error, .. } => panic!("{error}"),
                Reply::InlayHints {
                    request: returned,
                    result,
                } if returned.token == request.token => return result,
                _ => {}
            }
        }
        signal.notified().await;
    }
}

#[tokio::test]
async fn 실제_child는_다중_공급자의_빈_null_오류와_잘못된_응답을_격리하고_프로젝트를_섞지_않는다() {
    for mode in [
        "--native-inlay-hints",
        "--native-inlay-hints-empty",
        "--native-inlay-hints-null",
        "--native-inlay-hints-error",
        "--native-inlay-hints-bad",
        "--native-inlay-hints-unsupported",
    ] {
        tokio::time::timeout(DEADLINE, async {
            let (fixture, project, mut store, original, bin) = document_symbol_tests::fixture(mode);
            let original = store.documents().snapshot(original).unwrap();
            let DocumentKey::File(path) = &original.key else {
                panic!("file fixture");
            };
            let python = path.parent().unwrap().join("hints.py");
            std::fs::write(&python, original.rope.to_string()).unwrap();
            let file = taide_file::service::open_file(&python, &[], false).unwrap();
            let document = store.open_file(python, file).unwrap();
            let source = view(&mut store, document);
            let snapshot = store.documents().snapshot(document).unwrap();
            let mock = std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("examples/native-lsp-mock");
            let escaped = mock.to_str().unwrap().replace('\'', "'\\''");
            for (binary, mode) in [
                ("basedpyright-langserver", "--native-inlay-hints"),
                ("ruff", mode),
            ] {
                let path = PathBuf::from(&bin).join(binary);
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
            let count = if mode.ends_with("unsupported") { 1 } else { 2 };
            let providers = ready(&mut bridge, &signal, &project, &snapshot, count).await;
            let other_providers = ready(&mut bridge, &signal, &other, &snapshot, count).await;
            assert!(providers.is_disjoint(&other_providers));
            let mut state = State::default();
            let request = state
                .observe(
                    &store,
                    context(&project, source),
                    providers.clone(),
                    Instant::now(),
                )
                .unwrap()
                .unwrap();
            bridge.inlay_hints(request.clone()).unwrap();
            let result = response(&mut bridge, &signal, &request).await.unwrap();
            assert!(
                result
                    .groups
                    .iter()
                    .all(|(provider, _)| providers.contains(provider))
            );
            assert_eq!(
                result.groups.len(),
                if mode.ends_with("error") || mode.ends_with("bad") || mode.ends_with("unsupported")
                {
                    1
                } else {
                    2
                }
            );
            assert!(state.accept(&store, &request, providers.clone(), Ok(result)));
            let style = taide_native_ui::editor_inlay_hints::Style {
                foreground: eframe::egui::Color32::WHITE,
                background: eframe::egui::Color32::TRANSPARENT,
            };
            let colors = taide_native_ui::editor_inlay_hints::Colors {
                default: style,
                type_hint: style,
                parameter: style,
            };
            assert_eq!(
                state
                    .display(&store, eframe::egui::ViewportId::ROOT, source, colors)
                    .unwrap()
                    .items
                    .len(),
                3
            );
            assert_eq!(
                store.documents().snapshot(document).unwrap().revision,
                snapshot.revision
            );
            assert_eq!(
                store
                    .documents()
                    .snapshot(document)
                    .unwrap()
                    .rope
                    .to_string(),
                snapshot.rope.to_string()
            );
            state.clear();
            assert!(request.is_cancelled());
            bridge.disconnect().await.unwrap();
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        })
        .await
        .unwrap_or_else(|_| panic!("{mode}: deadline"));
    }
}

#[tokio::test]
async fn 실제_child의_보류_요청_취소와_서버_재시작은_이전_힌트를_거절한다() {
    for mode in ["--native-inlay-hints", "--native-inlay-hints-wait"] {
        tokio::time::timeout(DEADLINE, async {
            let (fixture, project, mut store, document, bin) = document_symbol_tests::fixture(mode);
            let source = view(&mut store, document);
            let snapshot = store.documents().snapshot(document).unwrap();
            let signal = Arc::new(Notify::new());
            let repaint = signal.clone();
            let mut bridge = LspBridge::connect(
                fixture.services.clone(),
                bin.clone(),
                Arc::new(move || repaint.notify_one()),
            )
            .unwrap();
            bridge.sync(project.clone(), snapshot.clone()).unwrap();
            let providers = ready(&mut bridge, &signal, &project, &snapshot, 1).await;
            let mut state = State::default();
            let request = state
                .observe(
                    &store,
                    context(&project, source),
                    providers.clone(),
                    Instant::now(),
                )
                .unwrap()
                .unwrap();
            bridge.inlay_hints(request.clone()).unwrap();
            if mode.ends_with("wait") {
                for phase in ["held", "cancelled"] {
                    loop {
                        if let Some(Reply::Diagnostics { diagnostics, .. }) = bridge.poll() {
                            if diagnostics
                                .diagnostics
                                .iter()
                                .any(|item| item.message == format!("synthetic inlay {phase}"))
                            {
                                break;
                            }
                        } else {
                            signal.notified().await;
                        }
                    }
                    if phase == "held" {
                        state.clear();
                    }
                }
                assert!(request.is_cancelled());
                assert!(!state.accept(
                    &store,
                    &request,
                    providers,
                    Ok(crate::editor_inlay_hints::Response { groups: Vec::new() })
                ));
            } else {
                let result = response(&mut bridge, &signal, &request).await;
                assert!(state.accept(&store, &request, providers.clone(), result));
                let (completion, returned) = oneshot::channel();
                bridge
                    .submit(Command::Bindings {
                        document,
                        completion,
                    })
                    .unwrap();
                let sessions = returned.await.unwrap();
                let config = plans(&fixture.services, project.clone(), &snapshot, bin)
                    .await
                    .unwrap()
                    .remove(0)
                    .config;
                sessions
                    .values()
                    .next()
                    .unwrap()
                    .client
                    .restart(config)
                    .await
                    .unwrap();
                let changed = loop {
                    let current = bridge.inlay_hint_providers(&project, &snapshot);
                    if !current.is_empty() && current != providers {
                        break current;
                    }
                    while bridge.poll().is_some() {}
                    signal.notified().await;
                };
                state.reconcile(
                    &store,
                    |_| true,
                    |project, snapshot| bridge.inlay_hint_providers(project, snapshot),
                );
                assert!(request.is_cancelled());
                let now = Instant::now();
                state
                    .observe(&store, context(&project, source), changed.clone(), now)
                    .unwrap();
                let fresh = state
                    .observe(
                        &store,
                        context(&project, source),
                        changed.clone(),
                        now + DEBOUNCE,
                    )
                    .unwrap()
                    .unwrap();
                bridge.inlay_hints(fresh.clone()).unwrap();
                let result = response(&mut bridge, &signal, &fresh).await;
                let result = result.unwrap();
                assert_eq!(result.groups.len(), 1);
                assert_eq!(result.groups[0].1.len(), 3);
                assert!(state.accept(&store, &fresh, changed.clone(), Ok(result)));
                assert!(!state.accept(
                    &store,
                    &request,
                    changed,
                    Ok(crate::editor_inlay_hints::Response { groups: Vec::new() })
                ));
                assert_eq!(
                    store.documents().snapshot(document).unwrap().revision,
                    snapshot.revision
                );
                state.clear();
            }
            bridge.disconnect().await.unwrap();
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        })
        .await
        .unwrap_or_else(|_| panic!("{mode}: deadline"));
    }
}

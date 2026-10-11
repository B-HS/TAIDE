use super::*;
use crate::editor_code_lenses::{Context, Request, Response, State};
use crate::editor_symbols::ProviderIdentity;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::Instant;
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{ViewId, ViewKey};
use tokio::sync::Notify;

const DEADLINE: Duration = Duration::from_secs(10);
const DEBOUNCE: Duration = Duration::from_millis(250);
const EXECUTABLE_MODE: u32 = 0o700;
const TEXT: &str = "class\n  \u{1f600}method\nend";

fn view(store: &mut EditorStore, document: DocumentId) -> ViewId {
    store
        .attach_view(
            ViewKey {
                window: "lens-child".into(),
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
    }
}

fn request(
    state: &mut State,
    store: &EditorStore,
    project: &ProjectId,
    source: ViewId,
    providers: HashSet<ProviderIdentity>,
) -> Request {
    let now = Instant::now();
    assert!(
        state
            .observe(
                store,
                context(project, source),
                providers.clone(),
                true,
                now
            )
            .unwrap()
            .is_none()
    );
    state
        .observe(
            store,
            context(project, source),
            providers,
            true,
            now + DEBOUNCE,
        )
        .unwrap()
        .unwrap()
}

async fn ready(
    bridge: &mut LspBridge,
    signal: &Notify,
    project: &ProjectId,
    snapshot: &DocumentSnapshot,
    count: usize,
) -> HashSet<ProviderIdentity> {
    loop {
        while let Some(reply) = bridge.poll() {
            if let Reply::Failed { error, .. } = reply {
                panic!("{error}");
            }
        }
        let providers = bridge.code_lens_providers(project, snapshot);
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
) -> Result<Response, Failure> {
    loop {
        while let Some(reply) = bridge.poll() {
            match reply {
                Reply::Failed { error, .. } => panic!("{error}"),
                Reply::CodeLenses {
                    request: returned,
                    result,
                } if returned.token == request.token => return result,
                _ => {}
            }
        }
        signal.notified().await;
    }
}

async fn diagnostic(bridge: &mut LspBridge, signal: &Notify, message: &str) {
    loop {
        while let Some(reply) = bridge.poll() {
            match reply {
                Reply::Failed { error, .. } => panic!("{error}"),
                Reply::Diagnostics { diagnostics, .. }
                    if diagnostics
                        .diagnostics
                        .iter()
                        .any(|item| item.message == message) =>
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
async fn 실제_child는_전체문서_다중공급자_빈_null_오류와_프로젝트_소유를_격리한다() {
    for mode in [
        "--native-code-lenses",
        "--native-code-lenses-empty",
        "--native-code-lenses-null",
        "--native-code-lenses-error",
        "--native-code-lenses-bad",
        "--native-code-lenses-unsupported",
    ] {
        tokio::time::timeout(DEADLINE, async {
            let (fixture, project, mut store, original, bin) = document_symbol_tests::fixture(mode);
            let original = store.documents().snapshot(original).unwrap();
            let DocumentKey::File(path) = &original.key else {
                panic!("file fixture");
            };
            let path = path.parent().unwrap().join("lenses.py");
            std::fs::write(&path, TEXT).unwrap();
            let document = store
                .open_file(
                    path.clone(),
                    taide_file::service::open_file(&path, &[], false).unwrap(),
                )
                .unwrap();
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
                ("basedpyright-langserver", "--native-code-lenses"),
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
            let request = request(&mut state, &store, &project, source, providers.clone());
            bridge.code_lenses(request.clone()).unwrap();
            let result = response(&mut bridge, &signal, &request).await.unwrap();
            assert!(
                result
                    .groups
                    .iter()
                    .all(|(provider, _, _)| providers.contains(provider))
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
            assert!(state.accept(&store, &request, providers, Ok(result), Instant::now()));
            let colors = taide_native_ui::editor_code_lenses::Colors {
                foreground: eframe::egui::Color32::WHITE,
                hover: eframe::egui::Color32::BLUE,
            };
            let items = state
                .display(&store, eframe::egui::ViewportId::ROOT, source, colors)
                .unwrap()
                .items;
            assert_eq!(
                items.len(),
                if mode == "--native-code-lenses" { 6 } else { 3 }
            );
            assert_eq!(
                items.iter().filter(|item| item.title.is_some()).count(),
                if mode == "--native-code-lenses" { 2 } else { 1 }
            );
            assert_eq!(
                store
                    .documents()
                    .snapshot(document)
                    .unwrap()
                    .rope
                    .to_string(),
                TEXT
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
async fn 실제_child의_provide_resolve_취소와_재시작은_늦은_응답을_거절한다() {
    for mode in [
        "--native-code-lenses-wait",
        "--native-code-lenses-resolve-wait",
        "--native-code-lenses",
    ] {
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
            let request = request(&mut state, &store, &project, source, providers.clone());
            bridge.code_lenses(request.clone()).unwrap();
            if mode.ends_with("wait") {
                if mode.ends_with("resolve-wait") {
                    let result = response(&mut bridge, &signal, &request).await;
                    let now = Instant::now();
                    assert!(state.accept(&store, &request, providers.clone(), result, now));
                    let resolve = state
                        .resolve_visible(
                            &store,
                            eframe::egui::ViewportId::ROOT,
                            source,
                            &[1],
                            now + DEBOUNCE,
                        )
                        .pop()
                        .unwrap();
                    bridge.resolve_code_lens(resolve).unwrap();
                    diagnostic(&mut bridge, &signal, "synthetic lens resolve held").await;
                } else {
                    diagnostic(&mut bridge, &signal, "synthetic lens held").await;
                }
                state.clear();
                diagnostic(&mut bridge, &signal, "synthetic lens cancelled").await;
                assert!(request.is_cancelled());
            } else {
                let result = response(&mut bridge, &signal, &request).await;
                assert!(state.accept(&store, &request, providers.clone(), result, Instant::now()));
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
                    let current = bridge.code_lens_providers(&project, &snapshot);
                    if !current.is_empty() && current != providers {
                        break current;
                    }
                    while bridge.poll().is_some() {}
                    signal.notified().await;
                };
                state.reconcile(
                    &store,
                    |_| true,
                    |project, snapshot| bridge.code_lens_providers(project, snapshot),
                );
                assert!(request.is_cancelled());
                let fresh = self::request(&mut state, &store, &project, source, changed.clone());
                bridge.code_lenses(fresh.clone()).unwrap();
                let result = response(&mut bridge, &signal, &fresh).await;
                assert!(state.accept(&store, &fresh, changed.clone(), result, Instant::now()));
                assert!(!state.accept(
                    &store,
                    &request,
                    changed,
                    Ok(Response { groups: Vec::new() }),
                    Instant::now()
                ));
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

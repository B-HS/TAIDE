use super::*;
use crate::editor_selection_ranges::{Context, Request, Response, State};
use crate::editor_symbols::ProviderIdentity;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::language_configuration::UntokenizedLines;
use taide_native_editor::line_commands::LineCommandContext;
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{Selection, SelectionSet, ViewId, ViewKey};
use tokio::sync::Notify;

const DEADLINE: Duration = Duration::from_secs(10);
const EXECUTABLE_MODE: u32 = 0o700;
const TEXT: &str = "class\n  \u{1f600}method\nend";
const RULES: LineCommandContext<'static> = LineCommandContext {
    indent: IndentOptions {
        tab_size: 4,
        insert_spaces: true,
    },
    language: None,
    syntax: &UntokenizedLines,
    compare: None,
    transforms: None,
    word_rules: None,
};

fn view(store: &mut EditorStore, document: DocumentId) -> ViewId {
    let source = store
        .attach_view(
            ViewKey {
                window: "selection-child".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    let head = TEXT.find("method").unwrap() + 2;
    store
        .set_view_state(
            source,
            SelectionSet {
                primary: 0,
                selections: vec![
                    Selection { anchor: head, head },
                    Selection { anchor: 2, head: 2 },
                ],
            },
            Default::default(),
            Vec::new(),
        )
        .unwrap();
    source
}

fn context(project: &ProjectId, source: ViewId) -> Context {
    Context {
        project: Some(project.clone()),
        owner: source,
        source,
        viewport: eframe::egui::ViewportId::ROOT,
    }
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
        let providers = bridge.selection_range_providers(project, snapshot);
        if providers.len() == count
            && !bridge
                .ready_feature_providers(project, snapshot, "textDocument/hover")
                .is_empty()
        {
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
                Reply::SelectionRanges {
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
async fn 실제_child의_utf16_다중위치_nullable_오류와_미지원은_로컬_선택을_보존한다() {
    for mode in [
        "--native-selection-ranges",
        "--native-selection-ranges-empty",
        "--native-selection-ranges-null",
        "--native-selection-ranges-partial-null",
        "--native-selection-ranges-error",
        "--native-selection-ranges-bad",
        "--native-selection-ranges-bad-length",
        "--native-selection-ranges-unsupported",
    ] {
        tokio::time::timeout(DEADLINE, async {
            let (fixture, project, mut store, document, bin) = document_symbol_tests::fixture(mode);
            let source = view(&mut store, document);
            let snapshot = store.documents().snapshot(document).unwrap();
            let signal = Arc::new(Notify::new());
            let repaint = signal.clone();
            let mut bridge = LspBridge::connect(
                fixture.services.clone(),
                bin,
                Arc::new(move || repaint.notify_one()),
            )
            .unwrap();
            bridge.sync(project.clone(), snapshot.clone()).unwrap();
            let count = usize::from(!mode.ends_with("unsupported"));
            let providers = ready(&mut bridge, &signal, &project, &snapshot, count).await;
            let mut state = State::default();
            state
                .queue(&store, context(&project, source), true)
                .unwrap();
            let requests = state
                .before(
                    &mut store,
                    context(&project, source),
                    providers.clone(),
                    RULES,
                )
                .unwrap();
            if count == 0 {
                assert!(requests.is_empty());
            } else {
                let request = &requests[0];
                bridge.selection_ranges(request.clone()).unwrap();
                let response = response(&mut bridge, &signal, request).await.unwrap();
                if mode.ends_with("error") || mode.ends_with("bad") {
                    assert!(response.groups.is_empty());
                } else {
                    assert_eq!(response.groups.len(), 1);
                    assert!(providers.contains(&response.groups[0].0));
                    if mode.ends_with("partial-null") {
                        let provided = response.groups[0].1.as_ref().unwrap();
                        assert!(provided[0].is_some());
                        assert!(provided[1].is_none());
                    }
                }
                assert!(state.accept(&store, request, providers.clone(), Ok(response)));
                assert!(
                    state
                        .before(
                            &mut store,
                            context(&project, source),
                            providers.clone(),
                            RULES
                        )
                        .unwrap()
                        .is_empty()
                );
                state.clear(&mut store);
                assert!(request.is_cancelled());
            }
            let selected = &store.views().get(source).unwrap().selection.selections;
            let start = TEXT.find('\u{1f600}').unwrap();
            assert_eq!(
                selected[0],
                Selection {
                    anchor: start,
                    head: start + "\u{1f600}method".len()
                }
            );
            assert_eq!(
                selected[1],
                Selection {
                    anchor: 0,
                    head: "class".len()
                }
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
                TEXT
            );
            bridge.disconnect().await.unwrap();
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        })
        .await
        .unwrap_or_else(|_| panic!("{mode}: deadline"));
    }
}

#[tokio::test]
async fn 실제_child의_취소와_재시작은_늦은_응답과_선택_캐시를_회수한다() {
    for mode in [
        "--native-selection-ranges-wait",
        "--native-selection-ranges",
    ] {
        tokio::time::timeout(DEADLINE, async {
            let (fixture, project, mut store, document, bin) = document_symbol_tests::fixture(mode);
            let source = view(&mut store, document);
            let snapshot = store.documents().snapshot(document).unwrap();
            let original = store.views().get(source).unwrap().selection.clone();
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
            state
                .queue(&store, context(&project, source), true)
                .unwrap();
            let request = state
                .before(
                    &mut store,
                    context(&project, source),
                    providers.clone(),
                    RULES,
                )
                .unwrap()
                .pop()
                .unwrap();
            bridge.selection_ranges(request.clone()).unwrap();
            if mode.ends_with("wait") {
                diagnostic(&mut bridge, &signal, "synthetic selection waiting").await;
                state.clear(&mut store);
                diagnostic(&mut bridge, &signal, "synthetic selection cancelled").await;
                assert_eq!(store.views().get(source).unwrap().selection, original);
            } else {
                let result = response(&mut bridge, &signal, &request).await;
                assert!(state.accept(&store, &request, providers.clone(), result));
                state
                    .before(
                        &mut store,
                        context(&project, source),
                        providers.clone(),
                        RULES,
                    )
                    .unwrap();
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
                let replacement = ready(&mut bridge, &signal, &project, &snapshot, 1).await;
                assert_ne!(replacement, providers);
                state.reconcile(&mut store, |_| true, |_, _| replacement.clone());
                assert!(!taide_native_editor::cursor_commands::has_smart_selection(
                    &store, source
                ));
            }
            assert!(request.is_cancelled());
            assert!(!state.accept(
                &store,
                &request,
                providers,
                Ok(Response { groups: Vec::new() })
            ));
            bridge.disconnect().await.unwrap();
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        })
        .await
        .unwrap_or_else(|_| panic!("{mode}: deadline"));
    }
}

#[tokio::test]
async fn 실제_복수_child와_두_프로젝트는_선택_범위의_공급과_소유를_격리한다() {
    for mode in [
        "--native-selection-ranges",
        "--native-selection-ranges-null",
        "--native-selection-ranges-error",
        "--native-selection-ranges-unsupported",
    ] {
        tokio::time::timeout(DEADLINE, async {
            let (fixture, project, mut store, original, bin) = document_symbol_tests::fixture(mode);
            let original = store.documents().snapshot(original).unwrap();
            let DocumentKey::File(path) = original.key else {
                panic!("file fixture");
            };
            let path = path.parent().unwrap().join("selection.py");
            std::fs::write(&path, TEXT).unwrap();
            let document = store
                .open_file(
                    path.clone(),
                    taide_file::service::open_file(&path, &[], false).unwrap(),
                )
                .unwrap();
            let source = view(&mut store, document);
            let other_source = view(&mut store, document);
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
                ("basedpyright-langserver", "--native-selection-ranges"),
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
            for (project, source, providers) in [
                (&project, source, &providers),
                (&other, other_source, &other_providers),
            ] {
                state.queue(&store, context(project, source), true).unwrap();
                let request = state
                    .before(
                        &mut store,
                        context(project, source),
                        providers.clone(),
                        RULES,
                    )
                    .unwrap()
                    .pop()
                    .unwrap();
                bridge.selection_ranges(request.clone()).unwrap();
                let result = response(&mut bridge, &signal, &request).await.unwrap();
                assert_eq!(
                    result.groups.len(),
                    if mode.ends_with("error") || mode.ends_with("unsupported") {
                        1
                    } else {
                        2
                    }
                );
                assert!(
                    result
                        .groups
                        .iter()
                        .all(|(provider, _)| providers.contains(provider))
                );
                assert!(state.accept(&store, &request, providers.clone(), Ok(result)));
                state
                    .before(
                        &mut store,
                        context(project, source),
                        providers.clone(),
                        RULES,
                    )
                    .unwrap();
                assert!(taide_native_editor::cursor_commands::has_smart_selection(
                    &store, source
                ));
            }
            state.clear(&mut store);
            assert_eq!(
                store
                    .documents()
                    .snapshot(document)
                    .unwrap()
                    .rope
                    .to_string(),
                TEXT
            );
            assert_eq!(
                store.documents().snapshot(document).unwrap().revision,
                snapshot.revision
            );
            bridge.disconnect().await.unwrap();
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        })
        .await
        .unwrap_or_else(|_| panic!("{mode}: deadline"));
    }
}

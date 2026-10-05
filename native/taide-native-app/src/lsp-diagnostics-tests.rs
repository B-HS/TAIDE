use super::*;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use taide_model::{app_event::AppEvent, paths::AppPaths, project::Project};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::Notify;

const DEADLINE: Duration = Duration::from_secs(5);
const BYTE_LIMIT: usize = 1024;
const MODEL_LIMIT: usize = 2;
const EXECUTABLE_MODE: u32 = 0o700;
const UNBOUND_URI: &str = "file:///synthetic/native-unbound%28a%29%2C.rs";
const UNBOUND_PATH: &str = "/synthetic/native-unbound(a),.rs";
const LIFETIME_DEADLINE: Duration = Duration::from_secs(12);
const EXPECTED_GRACE: Duration = Duration::from_secs(5);
const PROVIDER_COUNT: usize = 2;
const EXPECTED_REINITIALIZE_DELAY: Duration = Duration::from_secs(4);
const EXPECTED_REINITIALIZE_ATTEMPTS: usize = 3;

struct Sink;

impl EventSink for Sink {
    fn publish(&self, _event: AppEvent) {}
}

struct Fixture {
    directory: PathBuf,
    services: Arc<AppServices>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.lsp.shutdown();
        self.services.tasks.stop_all();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn empty_file() -> taide_model::file::OpenedFile {
    taide_model::file::OpenedFile {
        path: UNBOUND_PATH.into(),
        content: String::new(),
        language_id: "rust".into(),
        byte_size: 0,
        line_count: 1,
        tier: FileSizeTier::Normal,
        read_only: false,
        encoding_lossy: false,
        modified_ms: 0.0,
        editor_config: Default::default(),
    }
}

fn fixture(mode: &str, binaries: &[&str]) -> (Fixture, ProjectId, PathBuf, PathBuf) {
    assert!(matches!(
        mode,
        "--native-raw-diagnostics" | "--native-idle-diagnostics" | "--native-workspace-roots"
    ));
    let directory = std::env::temp_dir().join(format!("taide-native-raw-{}", ProjectId::new()));
    let root = directory.join("root");
    let bin = directory.join("bin");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&bin).unwrap();
    let mock = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/native-lsp-mock");
    assert!(mock.is_file());
    let escaped = mock.to_str().unwrap().replace('\'', "'\\''");
    for binary in binaries {
        assert!(matches!(
            *binary,
            "rust-analyzer" | "basedpyright-langserver" | "ruff"
        ));
        let executable = bin.join(binary);
        std::fs::write(&executable, format!("#!/bin/sh\nexec '{escaped}' {mode}\n")).unwrap();
        std::fs::set_permissions(
            &executable,
            std::fs::Permissions::from_mode(EXECUTABLE_MODE),
        )
        .unwrap();
    }
    let project = ProjectId::new();
    let root = root.canonicalize().unwrap();
    let state = AppState::new(AppPaths::new(directory.join("data")));
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project.clone(),
            root: root.to_str().unwrap().into(),
            name: "synthetic raw".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let fixture = Fixture {
        directory,
        services: crate::bootstrap::services(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            Arc::new(Sink),
        ),
    };
    (fixture, project, root, bin)
}

#[tokio::test]
async fn 실제_child의_미바인딩_raw와_동등_uri_marker는_세션_회수와_재개를_따른다() {
    tokio::time::timeout(DEADLINE, async {
        let (fixture, project, root, bin) = fixture("--native-raw-diagnostics", &["rust-analyzer"]);
        let ready = Arc::new(Notify::new());
        let signal = ready.clone();
        let mut bridge = LspBridge::connect(
            fixture.services.clone(),
            bin.into_os_string(),
            Arc::new(move || signal.notify_one()),
        )
        .unwrap();
        let path = root.join("bound(a),.rs");
        std::fs::write(&path, b"fn main() {}\n").unwrap();
        let file = taide_file::service::open_file(&path, &[], false).unwrap();
        let mut editor = EditorStore::new(EditorLimits {
            max_documents: MODEL_LIMIT,
            max_views: 1,
            max_undo_groups: 1,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        editor.track_document_disposals();
        let document = editor.open_file(path, file).unwrap();
        let model = editor.documents().snapshot(document).unwrap();
        let other_path = root.join("other.rs");
        std::fs::write(&other_path, b"fn other() {}\n").unwrap();
        let other_file = taide_file::service::open_file(&other_path, &[], false).unwrap();
        let other = editor.open_file(other_path, other_file).unwrap();
        let other_model = editor.documents().snapshot(other).unwrap();
        let mut previous_owner = None;
        for dispose_model in [false, true] {
            bridge
                .retain_projects(&HashSet::from([project.clone()]))
                .unwrap();
            bridge.sync(project.clone(), model.clone()).unwrap();
            let mut synced = false;
            let mut diagnosed = false;
            while !synced || !diagnosed {
                if let Some(reply) = bridge.poll() {
                    match reply {
                        Reply::Synced { .. } => synced = true,
                        Reply::Diagnostics { diagnostics, .. } => {
                            assert_eq!(diagnostics.diagnostics.len(), 1);
                            diagnosed = true;
                        }
                        Reply::Failed { error, .. } => {
                            panic!("raw diagnostics fixture failed: {error}")
                        }
                        _ => panic!("unexpected raw diagnostics fixture reply"),
                    }
                } else {
                    ready.notified().await;
                }
            }
            let selected = bindings(&bridge.commands, document).await.unwrap();
            assert_eq!(selected.len(), 1);
            let session = selected.values().next().unwrap();
            assert_ne!(Some(session.owner), previous_owner);
            previous_owner = Some(session.owner);
            let diagnostics = session.raw_diagnostics.get(UNBOUND_URI);
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(
                diagnostics[0].code,
                Some(lsp_types::NumberOrString::Number(42))
            );
            assert_eq!(diagnostics[0].source.as_deref(), Some("synthetic raw"));
            assert_eq!(
                diagnostics[0].data,
                Some(json!({"fix":"synthetic retained data"}))
            );
            assert_eq!(session.documents.len(), 1);
            let uri = &session.documents[&document].uri;
            assert_eq!(session.raw_diagnostics.get(uri).len(), 1);
            let states = bridge.diagnostic_bindings().unwrap();
            assert_eq!(states[&session.owner], HashSet::from([document]));
            drop(selected);
            bridge.sync(project.clone(), other_model.clone()).unwrap();
            let mut synced = false;
            let mut diagnosed = false;
            while !synced || !diagnosed {
                if let Some(reply) = bridge.poll() {
                    match reply {
                        Reply::Synced { .. } => synced = true,
                        Reply::Diagnostics { .. } => diagnosed = true,
                        Reply::Failed { error, .. } => panic!("second raw model failed: {error}"),
                        _ => panic!("unexpected second raw model reply"),
                    }
                } else {
                    ready.notified().await;
                }
            }
            if dispose_model {
                editor.release_document(document).unwrap();
                bridge.flush_model_disposals(&mut editor).unwrap();
                assert!(editor.pending_document_disposals().is_empty());
            } else {
                bridge.retain(&HashSet::from([other])).unwrap();
            }
            assert!(
                bindings(&bridge.commands, document)
                    .await
                    .unwrap()
                    .is_empty()
            );
            let remaining = bindings(&bridge.commands, other).await.unwrap();
            let session = remaining.values().next().unwrap();
            assert_eq!(Some(session.owner), previous_owner);
            assert_eq!(session.documents.len(), 1);
            let owners = bridge.diagnostic_bindings().unwrap();
            assert_eq!(owners[&session.owner].contains(&document), !dispose_model);
            let original_uri = taide_lsp::service::workspace_folder_uri(match &model.key {
                DocumentKey::File(path) => path.to_str().unwrap(),
                _ => panic!("synthetic model must be a file"),
            });
            assert_eq!(
                session.raw_diagnostics.get(&original_uri).len(),
                usize::from(!dispose_model)
            );
            assert_eq!(session.raw_diagnostics.get(UNBOUND_URI).len(), 1);
            assert_eq!(
                session
                    .raw_diagnostics
                    .get(&session.documents[&other].uri)
                    .len(),
                1
            );
            drop(remaining);
            if dispose_model {
                let mut file = taide_file::service::open_file(
                    match &other_model.key {
                        DocumentKey::File(path) => path,
                        _ => panic!("synthetic other model must be a file"),
                    },
                    &[],
                    false,
                )
                .unwrap();
                file.path = UNBOUND_PATH.into();
                let transient = editor.open_file(UNBOUND_PATH.into(), file).unwrap();
                editor.release_document(transient).unwrap();
                bridge.flush_model_disposals(&mut editor).unwrap();
                let remaining = bindings(&bridge.commands, other).await.unwrap();
                let session = remaining.values().next().unwrap();
                assert_eq!(Some(session.owner), previous_owner);
                assert!(session.raw_diagnostics.get(UNBOUND_URI).is_empty());
                assert_eq!(session.documents.len(), 1);
                assert_eq!(
                    session
                        .raw_diagnostics
                        .get(&session.documents[&other].uri)
                        .len(),
                    1
                );
            }
            bridge.retain(&HashSet::new()).unwrap();
            assert!(bindings(&bridge.commands, other).await.unwrap().is_empty());
            assert_eq!(bridge.summary(&project).unwrap().total, 1);
            bridge.retain_projects(&HashSet::new()).unwrap();
            assert!(bindings(&bridge.commands, other).await.unwrap().is_empty());
            assert!(bridge.diagnostic_bindings().unwrap().is_empty());
        }
        bridge.disconnect().await.unwrap();
        fixture.services.lsp.shutdown();
        fixture.services.lsp.wait_for_idle().await;
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}

#[test]
fn 모델_폐기_전송은_포화와_닫힌_큐에서_기록을_보존하고_성공만_확인한다() {
    let (commands, mut receiver) = mpsc::channel(1);
    let (_sender, replies) = mpsc::channel(1);
    let (stop, _stopping) = watch::channel(false);
    let (_publisher, states) = watch::channel(Vec::new());
    let mut bridge = LspBridge {
        commands,
        replies,
        stop,
        worker: None,
        synced: HashMap::new(),
        models: HashMap::new(),
        projects: None,
        states,
    };
    let mut editor = EditorStore::new(EditorLimits {
        max_documents: MODEL_LIMIT,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    editor.track_document_disposals();
    let tab = taide_model::ids::TabId::new();
    let document = editor.restore_untitled(tab.clone(), None).unwrap();
    let saved = editor.save_snapshot(document).unwrap();
    editor
        .convert_untitled_save(saved, UNBOUND_PATH.into(), empty_file(), tab)
        .unwrap();
    editor.release_document(document).unwrap();
    let pending = editor.pending_document_disposals().to_vec();
    bridge.submit(Command::Close(document)).unwrap();
    assert!(bridge.flush_model_disposals(&mut editor).is_err());
    assert_eq!(editor.pending_document_disposals(), pending);
    assert!(matches!(receiver.try_recv(), Ok(Command::Close(id)) if id == document));
    bridge.flush_model_disposals(&mut editor).unwrap();
    assert!(editor.pending_document_disposals().is_empty());
    let Ok(Command::DisposeModels(uris)) = receiver.try_recv() else {
        panic!("model disposal must be queued exactly once");
    };
    assert_eq!(
        uris,
        vec![taide_lsp::service::workspace_folder_uri(UNBOUND_PATH)]
    );
    bridge.flush_model_disposals(&mut editor).unwrap();
    assert!(matches!(
        receiver.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    let tab = taide_model::ids::TabId::new();
    let untitled = editor.restore_untitled(tab, None).unwrap();
    editor.discard_document(untitled, 0).unwrap();
    bridge.flush_model_disposals(&mut editor).unwrap();
    assert!(editor.pending_document_disposals().is_empty());
    let document = editor.open_file(UNBOUND_PATH.into(), empty_file()).unwrap();
    bridge.reconcile_models(&mut editor).unwrap();
    assert_eq!(bridge.models.len(), 1);
    bridge.reconcile_models(&mut editor).unwrap();
    let Ok(Command::Models { changed, removed }) = receiver.try_recv() else {
        panic!("model catalog must publish one changed snapshot");
    };
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].id, document);
    assert!(removed.is_empty());
    editor.release_document(document).unwrap();
    assert!(bridge.reconcile_models(&mut editor).is_err());
    assert!(matches!(receiver.try_recv(), Ok(Command::DisposeModels(_))));
    assert!(editor.pending_document_disposals().is_empty());
    assert_eq!(bridge.models.len(), 1);
    bridge.reconcile_models(&mut editor).unwrap();
    let Ok(Command::Models { changed, removed }) = receiver.try_recv() else {
        panic!("model catalog must retry its removal after queue saturation");
    };
    assert!(changed.is_empty());
    assert_eq!(removed, HashSet::from([document]));
    assert!(bridge.models.is_empty());
    let document = editor.open_file(UNBOUND_PATH.into(), empty_file()).unwrap();
    editor.release_document(document).unwrap();
    let pending = editor.pending_document_disposals().to_vec();
    drop(receiver);
    assert!(bridge.flush_model_disposals(&mut editor).is_err());
    assert_eq!(editor.pending_document_disposals(), pending);
}

async fn receive_diagnostics(
    bridge: &mut LspBridge,
    ready: &Notify,
    editor: &EditorStore,
    markers: &mut crate::diagnostics::Store,
    mut expected: HashSet<DocumentId>,
) -> crate::diagnostics::Owner {
    let mut observed = None;
    while !expected.is_empty() {
        match bridge.poll() {
            Some(Reply::Diagnostics {
                owner,
                document,
                revision,
                diagnostics,
            }) => {
                let model = editor.documents().snapshot(document).unwrap();
                assert_eq!(model.revision, revision);
                if let Some(bindings) = bridge.diagnostic_bindings() {
                    markers.reconcile(bindings);
                }
                markers.publish(owner, &model, diagnostics.diagnostics);
                if let Some(previous) = observed {
                    assert_eq!(owner, previous);
                }
                observed = Some(owner);
                expected.remove(&document);
            }
            Some(Reply::Synced { .. }) => {}
            Some(Reply::Failed { error, .. }) => panic!("idle diagnostics failed: {error}"),
            Some(_) => panic!("unexpected idle diagnostics reply"),
            None => ready.notified().await,
        }
    }
    observed.unwrap()
}

#[tokio::test]
async fn 실제_child는_비활성_모델_marker와_5초_유예_재사용_만료를_보존한다() {
    tokio::time::timeout(LIFETIME_DEADLINE, async {
        let (fixture, project, root, bin) =
            fixture("--native-idle-diagnostics", &["rust-analyzer"]);
        let ready = Arc::new(Notify::new());
        let signal = ready.clone();
        let mut bridge = LspBridge::connect(
            fixture.services.clone(),
            bin.into_os_string(),
            Arc::new(move || signal.notify_one()),
        )
        .unwrap();
        let path = root.join("idle.rs");
        std::fs::write(&path, b"fn main() {}\n").unwrap();
        let file = taide_file::service::open_file(&path, &[], false).unwrap();
        let mut editor = EditorStore::new(EditorLimits {
            max_documents: MODEL_LIMIT,
            max_views: 1,
            max_undo_groups: 1,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        editor.track_document_disposals();
        let document = editor.open_file(path, file).unwrap();
        let unbound = editor.open_file(UNBOUND_PATH.into(), empty_file()).unwrap();
        let model = editor.documents().snapshot(document).unwrap();
        let documents = HashSet::from([document, unbound]);
        let mut markers = crate::diagnostics::Store::default();
        markers.retain_documents(documents.clone());
        bridge.reconcile_models(&mut editor).unwrap();
        bridge
            .retain_projects(&HashSet::from([project.clone()]))
            .unwrap();
        bridge.sync(project.clone(), model.clone()).unwrap();
        let owner = receive_diagnostics(
            &mut bridge,
            &ready,
            &editor,
            &mut markers,
            documents.clone(),
        )
        .await;
        assert_eq!(markers.counts()[crate::diagnostics::ERROR], MODEL_LIMIT);
        let selected = bindings(&bridge.commands, document).await.unwrap();
        let session = selected.values().next().unwrap();
        let pid = session.client.snapshot().pid;
        assert!(pid.is_some());
        assert_eq!(session.owner, owner);
        drop(selected);
        let view = editor
            .attach_view(
                taide_native_editor::view::ViewKey {
                    window: "synthetic".into(),
                    pane: taide_model::ids::PaneId::new(),
                    tab: taide_model::ids::TabId::new(),
                },
                document,
            )
            .unwrap();
        taide_native_editor::editing::replace_selections(&mut editor, view, "live:", None).unwrap();
        editor.detach_view(view).unwrap();
        let current = editor.documents().snapshot(document).unwrap();
        assert_ne!(current.revision, model.revision);
        bridge.reconcile_models(&mut editor).unwrap();
        bridge.saved(document).unwrap();
        loop {
            match bridge.poll() {
                Some(Reply::Diagnostics {
                    owner: incoming_owner,
                    document: incoming_document,
                    revision,
                    diagnostics,
                }) => {
                    assert_eq!(incoming_owner, owner);
                    assert_eq!(incoming_document, document);
                    assert_eq!(
                        diagnostics.diagnostics[0].message,
                        "synthetic stale diagnostic"
                    );
                    assert_eq!(revision, model.revision);
                    assert_ne!(revision, current.revision);
                    break;
                }
                Some(Reply::Synced { .. }) => {}
                Some(Reply::Failed { error, .. }) => {
                    panic!("stale diagnostics fixture failed: {error}")
                }
                Some(_) => panic!("unexpected stale diagnostics reply"),
                None => ready.notified().await,
            }
        }
        assert_eq!(markers.counts()[crate::diagnostics::ERROR], MODEL_LIMIT);
        bridge.sync(project.clone(), current.clone()).unwrap();
        assert_eq!(
            receive_diagnostics(
                &mut bridge,
                &ready,
                &editor,
                &mut markers,
                HashSet::from([document])
            )
            .await,
            owner
        );
        let model = current;
        bridge.retain(&HashSet::new()).unwrap();
        assert_eq!(
            receive_diagnostics(
                &mut bridge,
                &ready,
                &editor,
                &mut markers,
                HashSet::from([document])
            )
            .await,
            owner
        );
        assert!(
            bindings(&bridge.commands, document)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(bridge.summary(&project).unwrap().total, 1);
        assert_eq!(markers.counts()[crate::diagnostics::ERROR], MODEL_LIMIT);
        assert!(
            markers
                .batches()
                .flat_map(|batch| batch.diagnostics.iter())
                .any(|diagnostic| { diagnostic.message == "synthetic inactive diagnostic" })
        );
        bridge.sync(project.clone(), model).unwrap();
        assert_eq!(
            receive_diagnostics(&mut bridge, &ready, &editor, &mut markers, documents).await,
            owner
        );
        let selected = bindings(&bridge.commands, document).await.unwrap();
        let session = selected.values().next().unwrap();
        assert_eq!(session.owner, owner);
        assert_eq!(session.client.snapshot().pid, pid);
        assert!(session.idle.deadline().is_none());
        drop(selected);
        let released_at = tokio::time::Instant::now();
        bridge.retain(&HashSet::new()).unwrap();
        assert_eq!(
            receive_diagnostics(
                &mut bridge,
                &ready,
                &editor,
                &mut markers,
                HashSet::from([document])
            )
            .await,
            owner
        );
        assert_eq!(markers.counts()[crate::diagnostics::ERROR], MODEL_LIMIT);
        while bridge.summary(&project).is_some() {
            ready.notified().await;
        }
        assert!(released_at.elapsed() >= EXPECTED_GRACE);
        markers.reconcile(bridge.diagnostic_bindings().unwrap());
        assert_eq!(markers.counts()[crate::diagnostics::ERROR], 0);
        assert!(
            bindings(&bridge.commands, document)
                .await
                .unwrap()
                .is_empty()
        );
        bridge.disconnect().await.unwrap();
        fixture.services.lsp.shutdown();
        fixture.services.lsp.wait_for_idle().await;
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}

fn reconcile_provider_markers(
    sessions: &HashMap<SessionKey, Session>,
    markers: &mut crate::diagnostics::Store,
) {
    let (states, receiver) = watch::channel(Vec::new());
    let repaint: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    publish_status(sessions, &states, &repaint);
    markers.reconcile(
        receiver
            .borrow()
            .iter()
            .map(|state| (state.owner, state.documents.clone()))
            .collect(),
    );
}

async fn receive_provider_diagnostics(
    sessions: &mut HashMap<SessionKey, Session>,
    models: &HashMap<DocumentId, DocumentSnapshot>,
    commands: &mut mpsc::Receiver<Command>,
    services: &Arc<AppServices>,
    markers: &mut crate::diagnostics::Store,
    mut expected: HashSet<(crate::diagnostics::Owner, DocumentId)>,
) {
    let (replies, _receiver) = mpsc::channel(COMMAND_CAPACITY);
    let repaint: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    while !expected.is_empty() {
        let Some(command) = commands.recv().await else {
            panic!("provider notice channel must remain open");
        };
        match command {
            Command::Notice {
                key,
                source,
                notice: incoming,
            } => {
                assert!(source.same_channel(&sessions[&key].client.subscribe()));
                let reply = notice(
                    sessions, models, key, incoming, services, &replies, &repaint,
                )
                .await;
                if let Some(Reply::Diagnostics {
                    owner,
                    document,
                    revision,
                    diagnostics,
                }) = reply
                {
                    assert_eq!(revision, models[&document].revision);
                    reconcile_provider_markers(sessions, markers);
                    markers.publish(owner, &models[&document], diagnostics.diagnostics);
                    expected.remove(&(owner, document));
                }
            }
            Command::StateChanged { .. } => {}
            _ => panic!("unexpected provider notice command"),
        }
    }
}

#[tokio::test]
async fn 공유_provider의_두_root는_같은_child와_owner로_합류한다() {
    tokio::time::timeout(DEADLINE, async {
        let (fixture, project, root, bin) =
            fixture("--native-workspace-roots", &["basedpyright-langserver"]);
        let mut editor = EditorStore::new(EditorLimits {
            max_documents: MODEL_LIMIT,
            max_views: 1,
            max_undo_groups: 1,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let roots = [root.join("first(a),"), root.join("second(b),")];
        let mut ids = Vec::new();
        for workspace in &roots {
            std::fs::create_dir(workspace).unwrap();
            std::fs::write(
                workspace.join("pyproject.toml"),
                b"[project]\nname='synthetic'\n",
            )
            .unwrap();
            let path = workspace.join("source.py");
            std::fs::write(&path, b"pass\n").unwrap();
            let file = taide_file::service::open_file(&path, &[], false).unwrap();
            ids.push(editor.open_file(path, file).unwrap());
        }
        let models = editor
            .documents()
            .snapshots()
            .map(|model| (model.id, model))
            .collect::<HashMap<_, _>>();
        let (commands, mut notices) = mpsc::channel(COMMAND_CAPACITY);
        let mut sessions = HashMap::new();
        for id in &ids {
            sync_document(
                &fixture.services,
                &mut sessions,
                &commands,
                project.clone(),
                models[id].clone(),
                bin.as_os_str().to_owned(),
            )
            .await
            .unwrap();
        }
        assert_eq!(sessions.len(), 1);
        let expected = sessions
            .values()
            .flat_map(|session| ids.iter().map(|id| (session.owner, *id)))
            .collect();
        let mut markers = crate::diagnostics::Store::default();
        markers.retain_documents(models.keys().copied().collect());
        receive_provider_diagnostics(
            &mut sessions,
            &models,
            &mut notices,
            &fixture.services,
            &mut markers,
            expected,
        )
        .await;
        assert_eq!(markers.counts()[crate::diagnostics::ERROR], MODEL_LIMIT);
        let session = sessions.values().next().unwrap();
        let owner = session.owner;
        let pid = session.client.snapshot().pid.unwrap();
        let client = session.client.clone();
        let uri = session.documents[&ids[1]].uri.clone();
        let expected_roots = roots
            .iter()
            .map(|root| {
                json!({
                    "uri":taide_lsp::service::workspace_folder_uri(root.to_str().unwrap()),
                    "name":root.file_name().unwrap().to_str().unwrap()
                })
            })
            .collect::<Vec<_>>();
        let params = json!({"textDocument":{"uri":uri},"position":{"line":0,"character":0}});
        let reply = client
            .request(
                "textDocument/hover".into(),
                params.clone(),
                Some((uri.clone(), 0)),
            )
            .await
            .unwrap();
        assert_eq!(reply["experimental"]["initializeCount"], 1);
        assert_eq!(
            reply["experimental"]["workspaceFolders"],
            json!(expected_roots)
        );
        assert_eq!(
            reply["experimental"]["workspaceReply"],
            json!(expected_roots)
        );
        sync_document(
            &fixture.services,
            &mut sessions,
            &commands,
            project.clone(),
            models[&ids[1]].clone(),
            bin.as_os_str().to_owned(),
        )
        .await
        .unwrap();
        assert_eq!(sessions.len(), 1);
        let plan = plans(
            &fixture.services,
            project,
            &models[&ids[0]],
            bin.as_os_str().to_owned(),
        )
        .await
        .unwrap()
        .pop()
        .unwrap();
        let generation = client.snapshot().generation;
        client.restart(plan.config).await.unwrap();
        let mut restarted = client.clone();
        restarted.wait_for_phase(Phase::Running).await.unwrap();
        let reply = client
            .request("textDocument/hover".into(), params, Some((uri, 0)))
            .await
            .unwrap();
        assert_eq!(
            reply["experimental"]["workspaceFolders"],
            json!(expected_roots)
        );
        assert_eq!(reply["experimental"]["initializeCount"], 1);
        assert_eq!(client.snapshot().generation, generation + 1);
        assert_ne!(client.snapshot().pid, Some(pid));
        assert_eq!(sessions.values().next().unwrap().owner, owner);
        close_document(&mut sessions, ids[0], None, None).await;
        let session = sessions.values().next().unwrap();
        assert!(session.documents.contains_key(&ids[1]));
        assert!(session.idle.deadline().is_none());
        client.stop().await.unwrap();
        fixture.services.lsp.shutdown();
        fixture.services.lsp.wait_for_idle().await;
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn root_비공유_spec과_다른_project는_child와_owner를_분리한다() {
    tokio::time::timeout(DEADLINE, async {
        for (binary, filename, manifest, should_split_project) in [
            ("rust-analyzer", "source.rs", "Cargo.toml", false),
            (
                "basedpyright-langserver",
                "source.py",
                "pyproject.toml",
                true,
            ),
        ] {
            let (fixture, project, root, bin) = fixture("--native-workspace-roots", &[binary]);
            let mut other_project = fixture
                .services
                .state
                .projects
                .read()
                .get(&project)
                .unwrap()
                .clone();
            other_project.id = ProjectId::new();
            let other_id = other_project.id.clone();
            fixture
                .services
                .state
                .projects
                .write()
                .insert(other_id.clone(), other_project);
            let mut editor = EditorStore::new(EditorLimits {
                max_documents: MODEL_LIMIT,
                max_views: 1,
                max_undo_groups: 1,
                max_document_bytes: BYTE_LIMIT,
            })
            .unwrap();
            let mut ids = Vec::new();
            for name in ["first", "second"] {
                let workspace = root.join(name);
                std::fs::create_dir(&workspace).unwrap();
                std::fs::write(workspace.join(manifest), b"[package]\nname='synthetic'\n").unwrap();
                let path = workspace.join(filename);
                std::fs::write(&path, b"synthetic\n").unwrap();
                let file = taide_file::service::open_file(&path, &[], false).unwrap();
                ids.push(editor.open_file(path, file).unwrap());
            }
            let (commands, _receiver) = mpsc::channel(COMMAND_CAPACITY);
            let mut sessions = HashMap::new();
            for (index, id) in ids.iter().enumerate() {
                let selected = if should_split_project && index > 0 {
                    other_id.clone()
                } else {
                    project.clone()
                };
                sync_document(
                    &fixture.services,
                    &mut sessions,
                    &commands,
                    selected,
                    editor.documents().snapshot(*id).unwrap(),
                    bin.as_os_str().to_owned(),
                )
                .await
                .unwrap();
            }
            assert_eq!(sessions.len(), MODEL_LIMIT);
            let mut owners = HashSet::new();
            let mut pids = HashSet::new();
            for session in sessions.values() {
                let mut client = session.client.clone();
                client.wait_for_phase(Phase::Running).await.unwrap();
                assert_eq!(session.roots.len(), 1);
                assert_eq!(session.documents.len(), 1);
                owners.insert(session.owner);
                pids.insert(client.snapshot().pid.unwrap());
                client.stop().await.unwrap();
            }
            assert_eq!(owners.len(), MODEL_LIMIT);
            assert_eq!(pids.len(), MODEL_LIMIT);
            fixture.services.lsp.shutdown();
            fixture.services.lsp.wait_for_idle().await;
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn 재초기화는_같은_child_generation에서_세번째_initialize까지_재시도한다() {
    tokio::time::timeout(LIFETIME_DEADLINE, async {
        let (fixture, project, root, bin) = fixture("--native-raw-diagnostics", &["rust-analyzer"]);
        let path = root.join("source.rs");
        std::fs::write(&path, b"synthetic\n").unwrap();
        let mut editor = EditorStore::new(EditorLimits {
            max_documents: 1,
            max_views: 1,
            max_undo_groups: 1,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let model = editor
            .open_file(
                path.clone(),
                taide_file::service::open_file(&path, &[], false).unwrap(),
            )
            .unwrap();
        let snapshot = editor.documents().snapshot(model).unwrap();
        let (commands, _receiver) = mpsc::channel(COMMAND_CAPACITY);
        let mut sessions = HashMap::new();
        sync_document(
            &fixture.services,
            &mut sessions,
            &commands,
            project.clone(),
            snapshot.clone(),
            bin.as_os_str().to_owned(),
        )
        .await
        .unwrap();
        let session = sessions.values().next().unwrap();
        let mut client = session.client.clone();
        client.wait_for_phase(Phase::Running).await.unwrap();
        let mock = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples/native-lsp-mock");
        let escaped = mock.to_str().unwrap().replace('\'', "'\\''");
        std::fs::write(
            bin.join("rust-analyzer"),
            format!("#!/bin/sh\nexec '{escaped}' --native-reinitialize\n"),
        )
        .unwrap();
        let plan = plans(
            &fixture.services,
            project,
            &snapshot,
            bin.as_os_str().to_owned(),
        )
        .await
        .unwrap()
        .pop()
        .unwrap();
        client.restart(plan.config).await.unwrap();
        let state = client.snapshot();
        let started = tokio::time::Instant::now();
        let running = client.wait_for_phase(Phase::Running).await.unwrap();
        assert_eq!(running.generation, state.generation);
        assert_eq!(running.pid, state.pid);
        assert!(started.elapsed() >= EXPECTED_REINITIALIZE_DELAY);
        let uri = session.documents[&model].uri.clone();
        let response = client
            .request(
                "textDocument/hover".into(),
                json!({"textDocument":{"uri":uri},"position":{"line":0,"character":0}}),
                Some((uri, 0)),
            )
            .await
            .unwrap();
        assert_eq!(
            response["experimental"]["initializeCount"],
            EXPECTED_REINITIALIZE_ATTEMPTS
        );
        assert_eq!(response["contents"]["value"], snapshot.rope.to_string());
        client.stop().await.unwrap();
        fixture.services.lsp.shutdown();
        fixture.services.lsp.wait_for_idle().await;
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn 재초기화_소진은_worker_owner를_회수하고_다음_열기에_재연결한다() {
    tokio::time::timeout(LIFETIME_DEADLINE, async {
        let (fixture, project, root, bin) = fixture("--native-raw-diagnostics", &["rust-analyzer"]);
        let ready = Arc::new(Notify::new());
        let signal = ready.clone();
        let mut bridge = LspBridge::connect(
            fixture.services.clone(),
            bin.as_os_str().to_owned(),
            Arc::new(move || signal.notify_one()),
        )
        .unwrap();
        let path = root.join("source.rs");
        std::fs::write(&path, b"synthetic\n").unwrap();
        let mut editor = EditorStore::new(EditorLimits {
            max_documents: 1,
            max_views: 1,
            max_undo_groups: 1,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let document = editor
            .open_file(
                path.clone(),
                taide_file::service::open_file(&path, &[], false).unwrap(),
            )
            .unwrap();
        let model = editor.documents().snapshot(document).unwrap();
        bridge
            .retain_projects(&HashSet::from([project.clone()]))
            .unwrap();
        bridge.sync(project.clone(), model.clone()).unwrap();
        let mut markers = crate::diagnostics::Store::default();
        markers.retain_documents(HashSet::from([document]));
        let owner = receive_diagnostics(
            &mut bridge,
            &ready,
            &editor,
            &mut markers,
            HashSet::from([document]),
        )
        .await;
        let selected = bindings(&bridge.commands, document).await.unwrap();
        let session = selected.values().next().unwrap();
        let client = session.client.clone();
        assert_eq!(markers.counts()[crate::diagnostics::ERROR], 1);
        let mock = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples/native-lsp-mock");
        let escaped = mock.to_str().unwrap().replace('\'', "'\\''");
        let executable = bin.join("rust-analyzer");
        std::fs::write(
            &executable,
            format!("#!/bin/sh\nexec '{escaped}' --native-reinitialize-exhausted\n"),
        )
        .unwrap();
        let plan = plans(
            &fixture.services,
            project.clone(),
            &model,
            bin.as_os_str().to_owned(),
        )
        .await
        .unwrap()
        .pop()
        .unwrap();
        drop(selected);
        client.restart(plan.config).await.unwrap();
        let generation = client.snapshot().generation;
        let started = tokio::time::Instant::now();
        loop {
            match bridge.poll() {
                Some(Reply::Failed {
                    document: None,
                    error,
                }) => {
                    assert_eq!(
                        error.to_string(),
                        failure(Failure::ReinitializeExhausted).to_string()
                    );
                    break;
                }
                Some(Reply::Synced { .. } | Reply::Diagnostics { .. }) => {}
                Some(_) => panic!("unexpected exhausted initialization reply"),
                None => ready.notified().await,
            }
        }
        assert!(started.elapsed() >= EXPECTED_REINITIALIZE_DELAY);
        assert_eq!(client.snapshot().generation, generation);
        assert_eq!(client.snapshot().phase, Phase::Stopped);
        assert!(client.snapshot().pid.is_none());
        assert!(bridge.summary(&project).is_none());
        assert!(
            bindings(&bridge.commands, document)
                .await
                .unwrap()
                .is_empty()
        );
        markers.reconcile(bridge.diagnostic_bindings().unwrap());
        assert_eq!(markers.counts()[crate::diagnostics::ERROR], 0);
        bridge.sync(project.clone(), model.clone()).unwrap();
        assert!(
            bindings(&bridge.commands, document)
                .await
                .unwrap()
                .is_empty()
        );
        std::fs::write(
            &executable,
            format!("#!/bin/sh\nexec '{escaped}' --native-raw-diagnostics\n"),
        )
        .unwrap();
        bridge.retain(&HashSet::new()).unwrap();
        bridge.sync(project.clone(), model).unwrap();
        let next_owner = receive_diagnostics(
            &mut bridge,
            &ready,
            &editor,
            &mut markers,
            HashSet::from([document]),
        )
        .await;
        assert_ne!(next_owner, owner);
        let selected = bindings(&bridge.commands, document).await.unwrap();
        let session = selected.values().next().unwrap();
        assert_eq!(session.owner, next_owner);
        assert_eq!(session.client.snapshot().generation, 0);
        assert!(session.client.snapshot().pid.is_some());
        assert_eq!(markers.counts()[crate::diagnostics::ERROR], 1);
        drop(selected);
        bridge.disconnect().await.unwrap();
        fixture.services.lsp.shutdown();
        fixture.services.lsp.wait_for_idle().await;
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}

async fn provider_disposal_case(should_unbind: bool) {
    let (fixture, project, root, bin) = fixture(
        "--native-raw-diagnostics",
        &["basedpyright-langserver", "ruff"],
    );
    let mut editor = EditorStore::new(EditorLimits {
        max_documents: MODEL_LIMIT,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    editor.track_document_disposals();
    let paths = [root.join("source(a),.py"), root.join("other.py")];
    let mut ids = Vec::new();
    for path in &paths {
        std::fs::write(path, b"pass\n").unwrap();
        let file = taide_file::service::open_file(path, &[], false).unwrap();
        assert_eq!(file.language_id, "python");
        ids.push(editor.open_file(path.clone(), file).unwrap());
    }
    let source_id = ids[0];
    let other_id = ids[1];
    let mut models = editor
        .documents()
        .snapshots()
        .map(|model| (model.id, model))
        .collect::<HashMap<_, _>>();
    let (commands, mut notices) = mpsc::channel(COMMAND_CAPACITY);
    let mut sessions = HashMap::new();
    for id in &ids {
        let states = sync_document(
            &fixture.services,
            &mut sessions,
            &commands,
            project.clone(),
            models[id].clone(),
            bin.as_os_str().to_owned(),
        )
        .await
        .unwrap();
        assert_eq!(states.len(), PROVIDER_COUNT);
    }
    assert_eq!(sessions.len(), PROVIDER_COUNT);
    let mut markers = crate::diagnostics::Store::default();
    markers.retain_documents(ids.iter().copied().collect());
    let expected = sessions
        .values()
        .flat_map(|session| ids.iter().map(|id| (session.owner, *id)))
        .collect();
    receive_provider_diagnostics(
        &mut sessions,
        &models,
        &mut notices,
        &fixture.services,
        &mut markers,
        expected,
    )
    .await;
    assert_eq!(
        markers.counts()[crate::diagnostics::ERROR],
        PROVIDER_COUNT * MODEL_LIMIT
    );
    let identities = sessions
        .iter()
        .map(|(key, session)| {
            (
                key.clone(),
                (session.owner, session.client.snapshot().pid.unwrap()),
            )
        })
        .collect::<HashMap<_, _>>();
    assert_eq!(
        identities
            .values()
            .map(|(_, pid)| *pid)
            .collect::<HashSet<_>>()
            .len(),
        PROVIDER_COUNT
    );
    let mut keys = sessions.keys().cloned().collect::<Vec<_>>();
    keys.sort_by(|left, right| left.server.cmp(&right.server));
    let old = models[&source_id].clone();
    let old_uri = taide_lsp::service::workspace_folder_uri(paths[0].to_str().unwrap());
    let other_uri = taide_lsp::service::workspace_folder_uri(paths[1].to_str().unwrap());
    let next = root.join("retargeted(b),.py");
    std::fs::rename(&paths[0], &next).unwrap();
    let file = taide_file::service::open_file(&next, &[], false).unwrap();
    editor
        .retarget_file(
            &old,
            next.clone(),
            taide_native_editor::document::DocumentMetadata::from_opened(&file),
        )
        .unwrap();
    assert_eq!(
        editor.pending_document_disposals(),
        std::slice::from_ref(&old.key)
    );
    let current = editor.documents().snapshot(source_id).unwrap();
    let new_uri = taide_lsp::service::workspace_folder_uri(next.to_str().unwrap());
    models.insert(source_id, current.clone());
    sync_session_document(sessions.get_mut(&keys[0]).unwrap(), &current, &new_uri)
        .await
        .unwrap();
    let new_owner = sessions[&keys[0]].owner;
    receive_provider_diagnostics(
        &mut sessions,
        &models,
        &mut notices,
        &fixture.services,
        &mut markers,
        HashSet::from([(new_owner, source_id)]),
    )
    .await;
    if should_unbind {
        close_document(
            &mut sessions,
            source_id,
            Some(&HashSet::from([keys[0].clone()])),
            None,
        )
        .await;
    }
    assert_eq!(sessions[&keys[1]].raw_diagnostics.get(&old_uri).len(), 1);
    dispose_models(&mut sessions, &mut models, vec![old_uri.clone()]).await;
    reconcile_provider_markers(&sessions, &mut markers);
    assert_eq!(
        markers.counts()[crate::diagnostics::ERROR],
        PROVIDER_COUNT + 1
    );
    assert!(sessions[&keys[0]].documents.contains_key(&source_id));
    assert_eq!(sessions[&keys[0]].documents[&source_id].uri, new_uri);
    assert!(!sessions[&keys[1]].documents.contains_key(&source_id));
    assert_eq!(sessions[&keys[0]].raw_diagnostics.get(&new_uri).len(), 1);
    for (key, session) in &sessions {
        assert!(session.raw_diagnostics.get(&old_uri).is_empty());
        assert_eq!(session.raw_diagnostics.get(&other_uri).len(), 1);
        assert_eq!(session.raw_diagnostics.get(UNBOUND_URI).len(), 1);
        assert!(session.documents.contains_key(&other_id));
        assert_eq!(session.owner, identities[key].0);
        assert_eq!(session.client.snapshot().pid, Some(identities[key].1));
        assert!(session.idle.deadline().is_none());
    }
    assert_eq!(models[&source_id].key, DocumentKey::File(next));
    for session in sessions.into_values() {
        session.client.stop().await.unwrap();
    }
    fixture.services.lsp.shutdown();
    fixture.services.lsp.wait_for_idle().await;
    fixture.services.tasks.shutdown().await;
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 실제_두_provider의_이전_uri_폐기는_새_uri와_다른_문서_owner를_보존한다() {
    tokio::time::timeout(DEADLINE, provider_disposal_case(true))
        .await
        .unwrap();
}

#[tokio::test]
async fn 실제_두_provider의_활성_이전_uri_폐기는_부분_전환_문서를_닫지_않는다() {
    tokio::time::timeout(DEADLINE, provider_disposal_case(false))
        .await
        .unwrap();
}

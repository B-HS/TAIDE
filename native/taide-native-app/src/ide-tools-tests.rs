use std::path::PathBuf;
use std::sync::Mutex;

use taide_ide::protocol::{encode, parse_incoming};
use taide_model::ide::{IdeDiagnostic, IdeDiagnosticSeverity, IdeSelectionInput};
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_runtime::{AppState, EventSink, TaskSupervisor, ide_actions, layout_actions};

use super::*;

const DEADLINE: Duration = Duration::from_secs(5);
const TOOL_COUNT: usize = 12;
const RESPONSE_ID: u32 = 42;
const VERSION: &str = "synthetic-version";
#[cfg(unix)]
const TERMINAL_COLUMNS: u16 = 80;
#[cfg(unix)]
const TERMINAL_ROWS: u16 = 24;
#[cfg(unix)]
const TERMINAL_HISTORY: usize = 128;
#[cfg(unix)]
const QUEUE_BYTES: usize = 256 * 1024;
#[cfg(unix)]
const QUEUE_COUNT: usize = 64;
#[cfg(unix)]
const QUEUE_VISITS: usize = 4096;
#[cfg(unix)]
const CELL_WIDTH: u16 = 8;
#[cfg(unix)]
const CELL_HEIGHT: u16 = 16;

#[cfg(unix)]
#[tokio::test]
async fn production_layout_actions는_파일_preview_diff_pending과_같은_hub의_pty_close를_연결한다() {
    tokio::time::timeout(DEADLINE, async {
        let fixture = Fixture::new();
        let hub = Arc::new(
            crate::terminal_host::Hub::new(
                fixture.services.clone(),
                crate::terminal_host::Limits {
                    sessions: 1,
                    core: Default::default(),
                    frames: crate::terminal_frames::Limits {
                        bytes: QUEUE_BYTES,
                        count: QUEUE_COUNT,
                        visits: QUEUE_VISITS,
                    },
                    writer: crate::terminal_writer::Limits {
                        bytes: QUEUE_BYTES,
                        count: QUEUE_COUNT,
                    },
                },
            )
            .unwrap(),
        );
        let actions = LayoutActions::new(hub.clone());
        let opened = dispatch(
            fixture.services.clone(),
            &actions,
            "openFile",
            &json!({"filePath": fixture.file, "preview": true, "makeFrontmost": false}),
        )
        .await
        .unwrap_or_else(|error| panic!("{}: {}", error.code, error.message));
        assert_eq!(decoded(opened)["success"], true);
        let first = find_file_tab(&fixture.services.state.layouts.read(), &fixture.file).unwrap();
        assert!(first.preview);
        fixture.services.state.settings.write().enable_preview_tabs = false;
        let second_file = PathBuf::from(&fixture.root).join("second.cs");
        std::fs::write(&second_file, "synthetic second").unwrap();
        (actions.open_file_tab)(
            fixture.services.clone(),
            fixture.project.clone(),
            second_file.to_str().unwrap().into(),
            "second.cs".into(),
            true,
        )
        .await
        .unwrap();
        let second =
            find_file_tab(&fixture.services.state.layouts.read(), second_file.to_str().unwrap())
                .unwrap();
        assert!(!second.preview);
        assert_eq!(
            (actions.close_tab)(fixture.services.clone(), first.id.clone())
                .await
                .unwrap()
                .id,
            first.id,
        );
        assert!(find_file_tab(&fixture.services.state.layouts.read(), &fixture.file).is_none());
        let request_id = uuid::Uuid::new_v4().to_string();
        let (responder, response) = oneshot::channel();
        fixture.services.ide.insert_pending_diff(
            request_id.clone(),
            taide_ide::store::PendingDiff {
                project_id: fixture.project.clone(),
                new_path: PathBuf::from(&fixture.file),
                responder,
            },
        );
        layout_actions::layout_open_tab(
            fixture.services.events.as_ref(),
            &fixture.services.state,
            fixture.project.clone(),
            TabKind::ClaudeDiff {
                request_id: request_id.clone(),
                path: fixture.file.clone(),
            },
            "synthetic production diff".into(),
            None,
            false,
        )
        .await
        .unwrap();
        dispatch(
            fixture.services.clone(),
            &actions,
            "close_tab",
            &json!({"tab_name":"synthetic production diff"}),
        )
        .await
        .unwrap_or_else(|error| panic!("{}: {}", error.code, error.message));
        assert_eq!(response.await.unwrap(), (IdeDiffOutcome::TabClosed, None));
        assert!(fixture.services.ide.take_pending_diff(&request_id).is_none());
        let id = hub
            .spawn(
                taide_model::terminal::PtySpawnOptions {
                    project_id: fixture.project.clone(),
                    cwd: fixture.root.clone(),
                    shell: Some("/bin/cat".into()),
                    cols: TERMINAL_COLUMNS,
                    rows: TERMINAL_ROWS,
                    scrollback_bytes: None,
                },
                TERMINAL_HISTORY,
                async { Vec::new() },
                crate::terminal_dispatch::EffectPorts {
                    command_colors: Default::default(),
                    updated: Arc::new(|| {}),
                    color: Arc::new(|_| Ok(taide_native_terminal::Rgb { r: 0, g: 0, b: 0 })),
                    geometry: Arc::new(|| {
                        Ok(taide_native_terminal::WindowSize {
                            num_cols: TERMINAL_COLUMNS,
                            num_lines: TERMINAL_ROWS,
                            cell_width: CELL_WIDTH,
                            cell_height: CELL_HEIGHT,
                        })
                    }),
                    event: Arc::new(|_| Ok(())),
                    stream: Arc::new(|_| Ok(())),
                },
            )
            .await
            .unwrap();
        let session = hub.get(&id).unwrap();
        assert!(!session.is_finished());
        layout_actions::layout_open_tab(
            fixture.services.events.as_ref(),
            &fixture.services.state,
            fixture.project.clone(),
            TabKind::Terminal {
                session_id: id.clone(),
                cwd: None,
            },
            "synthetic production cat".into(),
            None,
            false,
        )
        .await
        .unwrap();
        dispatch(
            fixture.services.clone(),
            &actions,
            "close_tab",
            &json!({"tab_name":"synthetic production cat"}),
        )
        .await
        .unwrap_or_else(|error| panic!("{}: {}", error.code, error.message));
        assert!(hub.get(&id).is_none());
        assert!(fixture.services.terminal.sessions_for_project(&fixture.project).is_empty());
        fixture.services.terminal.wait_for_idle().await.unwrap();
        fixture.finish().await;
        assert!(session.is_finished());
        assert!(fixture.sink.0.lock().unwrap().iter().any(|event| matches!(
            event,
            AppEvent::IdeCloseTabRequested { tab_name, .. } if tab_name == "synthetic production cat"
        )));
    })
    .await
    .unwrap();
}

#[derive(Default)]
struct Sink(Mutex<Vec<AppEvent>>);

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

struct Fixture {
    directory: PathBuf,
    root: String,
    file: String,
    project: ProjectId,
    services: Arc<AppServices>,
    sink: Arc<Sink>,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-ide-tools-{}", ProjectId::new()));
        let root_path = directory.join("project");
        std::fs::create_dir_all(&root_path).unwrap();
        let directory = directory.canonicalize().unwrap();
        let root_path = directory.join("project");
        let file = root_path.join("synthetic.cs").to_str().unwrap().to_string();
        std::fs::write(&file, "synthetic initial content").unwrap();
        let root = root_path.to_str().unwrap().to_string();
        let project = ProjectId::new();
        let state = AppState::new(AppPaths::new(directory.join("data")));
        state.settings.write().enable_preview_tabs = true;
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.clone(),
                name: "synthetic IDE".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        state
            .layouts
            .write()
            .insert(project.clone(), layout_service::default_layout());
        let sink = Arc::new(Sink::default());
        let services = crate::bootstrap::services(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            sink.clone(),
        );
        Self {
            directory,
            root,
            file,
            project,
            services,
            sink,
        }
    }

    async fn call(&self, name: &str, arguments: Value) -> Value {
        match dispatch(self.services.clone(), &actions(), name, &arguments).await {
            Ok(result) => result,
            Err(error) => panic!("{}: {}", error.code, error.message),
        }
    }

    async fn next_event(&self, predicate: impl Fn(&AppEvent) -> bool) -> AppEvent {
        loop {
            {
                let mut events = self.sink.0.lock().unwrap();
                if let Some(index) = events.iter().position(&predicate) {
                    return events.remove(index);
                }
            }
            tokio::task::yield_now().await;
        }
    }

    async fn finish(&self) {
        self.services.tasks.shutdown().await;
        assert_eq!(self.services.tasks.tracked_count(), 0);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.tasks.stop_all();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

fn actions() -> LayoutActions {
    LayoutActions {
        open_file_tab: |services, project, path, title, preview| {
            Box::pin(async move {
                layout_actions::layout_open_tab(
                    services.events.as_ref(),
                    &services.state,
                    project,
                    TabKind::File { path },
                    title,
                    None,
                    preview,
                )
                .await
                .map(|_| ())
            })
        },
        close_tab: Arc::new(|services, tab| {
            Box::pin(async move {
                layout_actions::close_tab_and_finish(
                    services.events.as_ref(),
                    &services.state,
                    &tab,
                    |tab| services.ide.reconcile_closed_tab(tab),
                )
                .await
                .map(|(_, closed, _)| closed.tab)
            })
        }),
    }
}

fn decoded(result: Value) -> Value {
    serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap()
}

#[tokio::test]
async fn native_ide_tools는_실제_layout과_경로_gate_선택_진단_조회_응답을_보존한다() {
    tokio::time::timeout(DEADLINE, async {
        let fixture = Fixture::new();
        for path in [
            PathBuf::from(&fixture.root).join("missing.cs"),
            fixture.directory.join("outside.cs"),
        ] {
            let error = dispatch(
                fixture.services.clone(),
                &actions(),
                "openFile",
                &json!({"filePath": path}),
            )
            .await
            .err()
            .unwrap();
            assert_eq!(error.code, RPC_INVALID_PARAMS);
            assert!(
                find_file_tab(
                    &fixture.services.state.layouts.read(),
                    path.to_str().unwrap()
                )
                .is_none()
            );
        }
        assert!(fixture.sink.0.lock().unwrap().is_empty());
        let opened = decoded(
            fixture
                .call(
                    "openFile",
                    json!({"filePath": fixture.file, "preview": true, "makeFrontmost": false}),
                )
                .await,
        );
        assert_eq!(
            opened,
            json!({"success": true, "filePath": fixture.file, "languageId": "csharp"})
        );
        let tab = find_file_tab(&fixture.services.state.layouts.read(), &fixture.file).unwrap();
        assert!(tab.preview);
        layout_actions::layout_set_dirty(
            fixture.services.events.as_ref(),
            &fixture.services.state,
            tab.id,
            true,
        )
        .await
        .unwrap();
        assert_eq!(
            decoded(
                fixture
                    .call("checkDocumentDirty", json!({"filePath": fixture.file}))
                    .await
            )["isDirty"],
            true
        );
        let editors = decoded(fixture.call("getOpenEditors", json!({})).await);
        assert_eq!(editors["tabs"].as_array().unwrap().len(), 1);
        assert_eq!(editors["tabs"][0]["languageId"], "csharp");
        assert_eq!(editors["tabs"][0]["isDirty"], true);
        let workspaces = decoded(fixture.call("getWorkspaceFolders", json!({})).await);
        assert_eq!(workspaces["rootPath"], fixture.root);
        assert_eq!(workspaces["folders"].as_array().unwrap().len(), 1);
        assert_eq!(
            decoded(fixture.call("getCurrentSelection", json!({})).await)["success"],
            false
        );

        let input = IdeSelectionInput {
            owner: "main".into(),
            project_id: fixture.project.clone(),
            path: fixture.file.clone(),
            text: "synthetic selection".into(),
            start_line: 0,
            start_character: 0,
            end_line: 0,
            end_character: 1,
            is_empty: false,
        };
        ide_actions::ide_set_selection(&fixture.services.ide, input.clone())
            .await
            .unwrap();
        let current = fixture.call("getCurrentSelection", json!({})).await;
        let mut remote = input;
        remote.owner = taide_model::remote::REMOTE_OWNER_LABEL.into();
        remote.text = "must not replace desktop selection".into();
        ide_actions::ide_set_selection(&fixture.services.ide, remote)
            .await
            .unwrap();
        assert_eq!(
            fixture.call("getCurrentSelection", json!({})).await,
            current
        );
        ide_actions::ide_clear_selection(&fixture.services.ide, "main".into())
            .await
            .unwrap();
        assert_eq!(
            decoded(fixture.call("getLatestSelection", json!({})).await)["text"],
            "synthetic selection"
        );
        let diagnostic_error = dispatch(
            fixture.services.clone(),
            &actions(),
            "getDiagnostics",
            &json!({}),
        )
        .await
        .err()
        .unwrap();
        assert_eq!(diagnostic_error.code, RPC_DIAGNOSTICS_NOT_READY);
        let diagnostic = IdeDiagnostic {
            path: fixture.file.clone(),
            severity: IdeDiagnosticSeverity::Warning,
            start_line: 0,
            start_character: 0,
            end_line: 0,
            end_character: 1,
            message: "synthetic diagnostic".into(),
            source: None,
        };
        ide_actions::ide_publish_diagnostics(
            &fixture.services.ide,
            fixture.project.clone(),
            vec![diagnostic],
        )
        .await
        .unwrap();
        let diagnostics = decoded(
            fixture
                .call(
                    "getDiagnostics",
                    json!({"uri": format!("file://{}", fixture.file)}),
                )
                .await,
        );
        assert_eq!(diagnostics[0]["diagnostics"][0]["severity"], "Warning");
        assert_eq!(
            fixture
                .call("close_tab", json!({"tab_name": "synthetic.cs"}))
                .await,
            text_content("TAB_CLOSED")
        );
        assert!(find_file_tab(&fixture.services.state.layouts.read(), &fixture.file).is_none());
        assert_eq!(
            fixture.call("closeAllDiffTabs", json!({})).await,
            text_content("CLOSED_0_DIFF_TABS")
        );
        fixture.finish().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn native_ide_tools는_원본_rpc_응답과_필수_layout_port_호출경계를_보존한다() {
    tokio::time::timeout(DEADLINE, async {
        let fixture = Fixture::new();
        let actions = actions();
        let call = |method: &str, params: Value| JsonRpcIncoming {
            id: Some(json!(RESPONSE_ID)),
            method: method.into(),
            params,
        };
        let initialized = handle_incoming(
            fixture.services.clone(),
            &actions,
            call("initialize", json!({})),
            VERSION,
        )
        .await
        .unwrap();
        assert_eq!(
            initialized.result.unwrap()["serverInfo"]["version"],
            VERSION
        );
        let listed = handle_incoming(
            fixture.services.clone(),
            &actions,
            call("tools/list", json!({})),
            VERSION,
        )
        .await
        .unwrap();
        assert_eq!(
            listed.result.unwrap()["tools"].as_array().unwrap().len(),
            TOOL_COUNT
        );
        let ping = handle_incoming(
            fixture.services.clone(),
            &actions,
            call("ping", json!({})),
            VERSION,
        )
        .await
        .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&encode(&ping)).unwrap(),
            json!({"jsonrpc": "2.0", "id": RESPONSE_ID, "result": {}})
        );
        for (method, params, code) in [
            ("tools/call", json!({}), RPC_INVALID_PARAMS),
            (
                "tools/call",
                json!({"name": "openFile"}),
                RPC_INVALID_PARAMS,
            ),
            (
                "tools/call",
                json!({"name": "executeCode"}),
                RPC_UNSUPPORTED,
            ),
            (
                "tools/call",
                json!({"name": "unknown"}),
                RPC_METHOD_NOT_FOUND,
            ),
            ("unknown", json!({}), RPC_METHOD_NOT_FOUND),
        ] {
            let response = handle_incoming(
                fixture.services.clone(),
                &actions,
                call(method, params),
                VERSION,
            )
            .await
            .unwrap();
            assert_eq!(response.id, json!(RESPONSE_ID));
            assert_eq!(response.error.unwrap().code, code);
        }
        let notification =
            parse_incoming(r#"{"method":"tools/call","params":{"name":"openFile"}}"#).unwrap();
        assert!(
            handle_incoming(fixture.services.clone(), &actions, notification, VERSION)
                .await
                .is_none()
        );
        assert!(fixture.sink.0.lock().unwrap().is_empty());
        fixture.services.state.begin_shutdown();
        let error = dispatch(
            fixture.services.clone(),
            &actions,
            "getWorkspaceFolders",
            &json!({}),
        )
        .await
        .err()
        .unwrap();
        assert_eq!(error.code, RPC_UNSUPPORTED);
        fixture.finish().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn native_ide_tools는_실제_diff_save_responder와_취소후_pending_owner를_회수한다() {
    tokio::time::timeout(DEADLINE, async {
        let fixture = Fixture::new();
        let actions = actions();
        let outside = fixture.directory.join("outside.cs");
        assert_eq!(fixture.call("openDiff", json!({"new_file_path": outside})).await, text_content(diff_outcome_text(IdeDiffOutcome::Rejected)));
        assert!(fixture.sink.0.lock().unwrap().is_empty());
        let args = json!({"new_file_path": fixture.file, "new_file_contents": "synthetic proposed", "tab_name": "synthetic diff"});
        let mut proposed = Box::pin(dispatch(fixture.services.clone(), &actions, "openDiff", &args));
        let requested = tokio::select! {
            response = &mut proposed => panic!("must wait for diff response: {}", response.is_ok()),
            event = fixture.next_event(|event| matches!(event, AppEvent::IdeDiffRequested { .. })) => event,
        };
        let AppEvent::IdeDiffRequested { request_id, project_id, new_contents, .. } = requested else {
            panic!("diff event expected");
        };
        assert_eq!(project_id, fixture.project);
        assert_eq!(new_contents, "synthetic proposed");
        assert_eq!(fixture.services.tasks.tracked_count(), 1);
        ide_actions::ide_resolve_diff(
            &fixture.services.state, &fixture.services.ide_save_file, &fixture.services.ide,
            request_id, IdeDiffOutcome::Saved, Some(new_contents),
        ).await.unwrap();
        let result = match proposed.await { Ok(value) => value, Err(error) => panic!("{}", error.message) };
        assert_eq!(result, text_content(diff_outcome_text(IdeDiffOutcome::Saved)));
        assert_eq!(std::fs::read_to_string(&fixture.file).unwrap(), "synthetic proposed");

        let mut cancelled = Box::pin(dispatch(fixture.services.clone(), &actions, "openDiff", &args));
        let event = tokio::select! {
            response = &mut cancelled => panic!("must wait for cancellation: {}", response.is_ok()),
            event = fixture.next_event(|event| matches!(event, AppEvent::IdeDiffRequested { .. })) => event,
        };
        let AppEvent::IdeDiffRequested { request_id, .. } = event else { panic!("diff event expected") };
        drop(cancelled);
        assert!(fixture.services.ide.take_pending_diff(&request_id).is_none());
        assert_eq!(fixture.services.tasks.tracked_count(), 0);

        fixture.call("openFile", json!({"filePath": fixture.file})).await;
        let save_args = json!({"filePath": fixture.file});
        let mut saved = Box::pin(dispatch(fixture.services.clone(), &actions, "saveDocument", &save_args));
        let event = tokio::select! {
            response = &mut saved => panic!("must wait for save response: {}", response.is_ok()),
            event = fixture.next_event(|event| matches!(event, AppEvent::IdeSaveRequested { .. })) => event,
        };
        let AppEvent::IdeSaveRequested { request_id, path, .. } = event else { panic!("save event expected") };
        taide_runtime::file_actions::file_save(&fixture.services.state, &fixture.services.tasks, path, "synthetic actual save".into()).await.unwrap();
        ide_actions::ide_resolve_save(&fixture.services.ide, request_id, true).await.unwrap();
        let result = match saved.await { Ok(value) => value, Err(error) => panic!("{}", error.message) };
        assert_eq!(decoded(result)["saved"], true);
        assert_eq!(std::fs::read_to_string(&fixture.file).unwrap(), "synthetic actual save");
        let mut cancelled = Box::pin(dispatch(fixture.services.clone(), &actions, "saveDocument", &save_args));
        let event = tokio::select! {
            response = &mut cancelled => panic!("must wait for cancellation: {}", response.is_ok()),
            event = fixture.next_event(|event| matches!(event, AppEvent::IdeSaveRequested { .. })) => event,
        };
        let AppEvent::IdeSaveRequested { request_id, .. } = event else { panic!("save event expected") };
        drop(cancelled);
        assert!(fixture.services.ide.take_pending_save(&request_id).is_none());
        fixture.finish().await;
    }).await.unwrap();
}

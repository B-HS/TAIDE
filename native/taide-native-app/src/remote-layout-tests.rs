use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Mutex;

use serde_json::json;
use taide_ide::store::PendingDiff;
use taide_model::app_event::AppEvent;
use taide_model::ide::IdeDiffOutcome;
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::layout::{PaneNode, ProjectLayout, Tab};
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_remote::command_policy::REMOTE_ALLOWED_COMMANDS;
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::oneshot;

use super::*;
use crate::remote_gateway;
use crate::remote_ws::ChannelFactory;

const TERMINAL_ID: &str = "synthetic-remote-terminal";
const VIEW_STATE: &str = "synthetic opaque view state";
const SPLIT_SIZE: f32 = 40.0;
const REMAINING_SIZE: f32 = 60.0;
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
const DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

#[derive(Default)]
struct Sink(Mutex<Vec<AppEvent>>);

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

struct Fixture {
    directory: PathBuf,
    root: PathBuf,
    project: ProjectId,
    pane: PaneId,
    services: Arc<AppServices>,
    sink: Arc<Sink>,
    discarded: Arc<Mutex<Vec<String>>>,
    backend: Dispatch,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-remote-layout-{}", ProjectId::new()));
        std::fs::create_dir_all(directory.join("project")).unwrap();
        let directory = directory.canonicalize().unwrap();
        let root = directory.join("project");
        let project = ProjectId::new();
        let state = AppState::new(AppPaths::new(directory.join("data")));
        state.settings.write().enable_preview_tabs = true;
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.to_str().unwrap().into(),
                name: "synthetic remote layout".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        let layout = taide_layout::service::default_layout();
        let pane = layout.focused_pane.clone();
        state.layouts.write().insert(project.clone(), layout);
        let sink = Arc::new(Sink::default());
        let services = crate::bootstrap::services(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            sink.clone(),
        );
        let discarded = Arc::new(Mutex::new(Vec::new()));
        let recorded = discarded.clone();
        let observed_services = services.clone();
        let observed_project = project.clone();
        let ports = Ports {
            discard_terminal: Arc::new(move |id| {
                let layouts = observed_services.state.layouts.read();
                assert!(!crate::tabs::tabs_in(&layouts[&observed_project].root).iter()
                .any(|tab| matches!(&tab.kind, TabKind::Terminal { session_id, .. } if session_id == id)));
                recorded.lock().unwrap().push(id.into());
            }),
        };
        let remaining = Dispatch {
            json: Arc::new(|_, name, args, _| {
                Box::pin(async move {
                    assert_eq!(name, "settings_get");
                    Ok(args.to_string())
                })
            }),
            raw: Arc::new(|_, name, _| {
                Box::pin(async move {
                    assert_eq!(name, "file_read_raw");
                    Ok(vec![0])
                })
            }),
        };
        Self {
            directory,
            root,
            project,
            pane,
            services,
            sink,
            discarded,
            backend: remote_gateway::with_policy(extend_backend(ports, remaining)),
        }
    }

    async fn call(&self, name: &str, args: Value) -> Result<Value, Value> {
        let channels: ChannelFactory = Arc::new(|_| Box::new(|_| panic!("layout has no channels")));
        let text = (self.backend.json)(self.services.clone(), name.into(), args, channels).await?;
        Ok(serde_json::from_str(&text).unwrap())
    }

    async fn ok(&self, name: &str, args: Value) -> Value {
        self.call(name, args)
            .await
            .unwrap_or_else(|error| panic!("{name}: {error}"))
    }

    async fn layout(&self, name: &str, args: Value) -> ProjectLayout {
        serde_json::from_value(self.ok(name, args).await).unwrap()
    }

    fn current(&self) -> ProjectLayout {
        self.services.state.layouts.read()[&self.project].clone()
    }

    async fn open(&self, kind: Value, title: &str) -> TabId {
        let layout = self
            .layout(
                "layout_open_tab",
                json!({"projectId":self.project,
            "kind":kind,"title":title,"preview":false}),
            )
            .await;
        taide_layout::service::find_tab_by_title(&layout.root, title).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.tasks.stop_all();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[cfg(unix)]
#[tokio::test]
async fn 실제_native_hub_포트의_원격_terminal_닫기는_pty와_actor를_회수한다() {
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
    let id = hub
        .spawn(
            taide_model::terminal::PtySpawnOptions {
                project_id: fixture.project.clone(),
                cwd: fixture.root.to_str().unwrap().into(),
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
    assert!(
        fixture
            .services
            .terminal
            .sessions_for_project(&fixture.project)
            .iter()
            .any(|entry| entry.id == id)
    );
    let tab = fixture
        .open(
            json!({"kind":"terminal","sessionId":id}),
            "active synthetic cat",
        )
        .await;
    let remaining = Dispatch {
        json: Arc::new(|_, name, _, _| Box::pin(async move { panic!("unexpected JSON: {name}") })),
        raw: Arc::new(|_, name, _| Box::pin(async move { panic!("unexpected raw: {name}") })),
    };
    let backend = remote_gateway::with_policy(extend_backend(Ports::new(hub.clone()), remaining));
    let channels: ChannelFactory = Arc::new(|_| Box::new(|_| panic!("close has no channels")));
    (backend.json)(
        fixture.services.clone(),
        "layout_close_tab".into(),
        json!({"tabId":tab}),
        channels,
    )
    .await
    .unwrap();
    assert!(hub.get(&id).is_none());
    assert!(
        fixture
            .services
            .terminal
            .sessions_for_project(&fixture.project)
            .is_empty()
    );
    tokio::time::timeout(DEADLINE, fixture.services.terminal.wait_for_idle())
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(DEADLINE, fixture.services.tasks.shutdown())
        .await
        .unwrap();
    assert!(session.is_finished());
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
}

fn tab(layout: &ProjectLayout, id: &TabId) -> Tab {
    crate::tabs::tabs_in(&layout.root)
        .into_iter()
        .find(|tab| &tab.id == id)
        .unwrap()
        .clone()
}

#[tokio::test]
async fn catalog_실제_arm과_원본_입력_파일_gate_remaining을_보존한다() {
    let declared: BTreeSet<_> = COMMANDS.iter().copied().collect();
    assert_eq!(declared.len(), COMMANDS.len());
    assert!(
        declared
            .iter()
            .all(|name| REMOTE_ALLOWED_COMMANDS.contains(name))
    );
    assert!(
        declared
            .iter()
            .all(|name| !crate::remote_ide::COMMANDS.contains(name)
                && !crate::remote_preferences::COMMANDS.contains(name)
                && !crate::remote_files::JSON_COMMANDS.contains(name))
    );
    let pattern = regex::Regex::new(r#"(?m)^        "([a-z_]+)" =>"#).unwrap();
    let arms: BTreeSet<_> = pattern
        .captures_iter(include_str!("remote-layout.rs"))
        .map(|capture| capture.get(1).unwrap().as_str())
        .collect();
    assert_eq!(declared, arms);
    let fixture = Fixture::new();
    let initial = fixture.current();
    assert_eq!(
        fixture
            .layout("layout_get", json!({"projectId":fixture.project}))
            .await,
        initial
    );
    let outside = fixture.directory.join("outside.txt");
    std::fs::write(&outside, "synthetic outside").unwrap();
    for path in [outside, fixture.root.join("missing.txt")] {
        assert!(
            fixture
                .call(
                    "layout_open_tab",
                    json!({"projectId":fixture.project,
            "kind":{"kind":"file","path":path},"title":"invalid","preview":false})
                )
                .await
                .is_err()
        );
    }
    assert_eq!(fixture.current(), initial);
    assert!(fixture.sink.0.lock().unwrap().is_empty());
    assert_eq!(
        fixture
            .call(
                "layout_open_tab",
                json!({"projectId":fixture.project,
        "kind":{"kind":"settings"},"title":"invalid","preview":"true"})
            )
            .await
            .unwrap_err()["code"],
        "InvalidArgument"
    );
    assert_eq!(
        fixture
            .call("layout_move_tab_to_window", json!({}))
            .await
            .unwrap_err()["message"]["kind"],
        "Forbidden"
    );
    assert_eq!(
        fixture.ok("settings_get", json!({"owner":"main"})).await["owner"],
        "remote"
    );
    assert_eq!(
        (fixture.backend.raw)(
            fixture.services.clone(),
            "file_read_raw".into(),
            Value::Null
        )
        .await
        .unwrap(),
        vec![0]
    );
    fixture.services.tasks.shutdown().await;
}

#[tokio::test]
async fn 실제_tab_상태_untitled_닫기_reopen과_ide_terminal_정리를_보존한다() {
    let fixture = Fixture::new();
    let file = fixture.root.join("synthetic.txt");
    std::fs::write(&file, "synthetic file").unwrap();
    let id = fixture
        .open(json!({"kind":"file","path":file}), "synthetic.txt")
        .await;
    fixture
        .ok("layout_set_preview", json!({"tabId":id,"preview":true}))
        .await;
    assert!(tab(&fixture.current(), &id).preview);
    fixture
        .ok("layout_set_dirty", json!({"tabId":id,"dirty":true}))
        .await;
    assert!(tab(&fixture.current(), &id).dirty);
    fixture
        .ok(
            "layout_set_view_state",
            json!({"tabId":id,"viewState":VIEW_STATE}),
        )
        .await;
    assert_eq!(
        tab(&fixture.current(), &id).view_state.as_deref(),
        Some(VIEW_STATE)
    );
    fixture.ok("layout_activate_tab", json!({"tabId":id})).await;
    fixture
        .ok("layout_pin_tab", json!({"tabId":id,"pinned":true}))
        .await;
    assert!(tab(&fixture.current(), &id).pinned);
    assert!(!tab(&fixture.current(), &id).preview);
    fixture
        .ok("layout_pin_tab", json!({"tabId":id,"pinned":false}))
        .await;
    fixture.ok("layout_close_tab", json!({"tabId":id})).await;
    assert!(taide_layout::service::find_tab(&fixture.current().root, &id).is_none());
    assert_eq!(fixture.current().closed_tabs[0].tab.id, id);
    fixture
        .ok("layout_reopen_closed", json!({"projectId":fixture.project}))
        .await;
    assert_eq!(
        tab(&fixture.current(), &id).view_state.as_deref(),
        Some(VIEW_STATE)
    );
    let untitled = fixture
        .layout("layout_open_untitled", json!({"projectId":fixture.project}))
        .await;
    let new_id = crate::tabs::tabs_in(&untitled.root)
        .into_iter()
        .find(|candidate| matches!(candidate.kind, TabKind::Untitled { .. }))
        .unwrap()
        .id
        .clone();
    let converted = fixture.root.join("converted.txt");
    fixture
        .ok(
            "layout_convert_untitled",
            json!({"tabId":new_id,"path":converted}),
        )
        .await;
    assert_eq!(
        tab(&fixture.current(), &new_id).kind,
        TabKind::File {
            path: converted.to_str().unwrap().into()
        }
    );
    assert!(!converted.exists());

    let (sender, response) = oneshot::channel();
    fixture.services.ide.insert_pending_diff(
        "synthetic-diff".into(),
        PendingDiff {
            project_id: fixture.project.clone(),
            new_path: file.clone(),
            responder: sender,
        },
    );
    let diff = fixture
        .open(
            json!({"kind":"claudeDiff","requestId":"synthetic-diff","path":file}),
            "diff",
        )
        .await;
    fixture.ok("layout_close_tab", json!({"tabId":diff})).await;
    assert_eq!(response.await.unwrap(), (IdeDiffOutcome::TabClosed, None));
    let terminal = fixture
        .open(
            json!({"kind":"terminal","sessionId":"old-synthetic"}),
            "terminal",
        )
        .await;
    fixture
        .ok(
            "layout_set_terminal_session",
            json!({"tabId":terminal,"sessionId":TERMINAL_ID}),
        )
        .await;
    fixture
        .ok("layout_close_tab", json!({"tabId":terminal}))
        .await;
    assert_eq!(*fixture.discarded.lock().unwrap(), [TERMINAL_ID]);
    assert!(
        fixture
            .services
            .state
            .dirty_layouts
            .read()
            .contains(&fixture.project)
    );
    let revisions: Vec<_> = fixture
        .sink
        .0
        .lock()
        .unwrap()
        .iter()
        .map(|event| {
            let AppEvent::LayoutChanged {
                project_id,
                revision,
            } = event
            else {
                panic!("unexpected event")
            };
            assert_eq!(project_id, &fixture.project);
            *revision
        })
        .collect();
    assert!(revisions.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(revisions.last(), Some(&fixture.current().revision));
    fixture.services.tasks.shutdown().await;
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 실제_split_move_resize_focus_shell과_path_change는_원본_단일_변경을_보존한다() {
    let fixture = Fixture::new();
    let a = fixture.open(json!({"kind":"settings"}), "first").await;
    let b = fixture
        .open(
            json!({"kind":"terminal","sessionId":"synthetic-split"}),
            "second",
        )
        .await;
    fixture
        .ok(
            "layout_split",
            json!({"paneId":fixture.pane,"edge":"right","tabId":b}),
        )
        .await;
    let split = fixture.current();
    let b_pane = taide_layout::service::find_tab(&split.root, &b)
        .unwrap()
        .0
        .clone();
    let PaneNode::Split { id: split_id, .. } = &split.root else {
        panic!("split expected")
    };
    fixture
        .ok(
            "layout_resize",
            json!({"paneId":split_id,"sizes":[SPLIT_SIZE,REMAINING_SIZE]}),
        )
        .await;
    fixture
        .ok("layout_focus_pane", json!({"paneId":b_pane}))
        .await;
    assert_eq!(fixture.current().focused_pane, b_pane);
    fixture
        .ok(
            "layout_move_tab",
            json!({"tabId":a,"paneId":b_pane,"index":0}),
        )
        .await;
    assert_eq!(
        taide_layout::service::find_tab(&fixture.current().root, &a)
            .unwrap()
            .0,
        &b_pane
    );
    let before = fixture.current().revision;
    fixture
        .ok(
            "layout_open_tab_in_split",
            json!({"request":{"projectId":fixture.project,
        "targetPane":b_pane,"edge":"bottom","kind":{"kind":"settings"},"title":"split-new"}}),
        )
        .await;
    assert_eq!(fixture.current().revision, before + 1);
    let unchanged = fixture.current();
    assert!(
        fixture
            .call(
                "layout_open_tab_in_split",
                json!({"request":{"projectId":fixture.project,
        "targetPane":b_pane,"edge":"center","kind":{"kind":"settings"},"title":"invalid"}})
            )
            .await
            .is_err()
    );
    assert_eq!(fixture.current(), unchanged);
    fixture
        .ok(
            "layout_set_shell_view",
            json!({"projectId":fixture.project,"patch":{"sidebarCollapsed":true}}),
        )
        .await;
    assert!(fixture.current().shell_view.sidebar_collapsed);

    let original = fixture.root.join("original.txt");
    let renamed = fixture.root.join("renamed.txt");
    std::fs::write(&original, "synthetic rename").unwrap();
    let id = fixture
        .open(json!({"kind":"file","path":original}), "original.txt")
        .await;
    std::fs::rename(&original, &renamed).unwrap();
    let result = fixture
        .ok(
            "layout_apply_path_change",
            json!({"projectId":fixture.project,
        "change":{"kind":"renamed","from":original,"to":renamed}}),
        )
        .await;
    assert_eq!(result["moved"].as_array().unwrap().len(), 1);
    assert_eq!(
        tab(&fixture.current(), &id).kind,
        TabKind::File {
            path: renamed.to_str().unwrap().into()
        }
    );
    let before = fixture.current();
    let events_before = fixture.sink.0.lock().unwrap().len();
    let result = fixture
        .ok(
            "layout_apply_path_change",
            json!({"projectId":fixture.project,
        "change":{"kind":"deleted","path":fixture.root.join("no-open-tab")}}),
        )
        .await;
    assert!(result["closedPaths"].as_array().unwrap().is_empty());
    assert_eq!(fixture.current(), before);
    assert_eq!(fixture.sink.0.lock().unwrap().len(), events_before);
    assert_eq!(
        fixture
            .call(
                "layout_apply_path_change",
                json!({"projectId":fixture.project,
        "change":{"kind":"deleted","path":"/"}})
            )
            .await
            .unwrap_err()["message"]["kind"],
        "Forbidden"
    );
    let result = fixture
        .ok(
            "layout_apply_path_change",
            json!({"projectId":fixture.project,
        "change":{"kind":"deleted","path":renamed}}),
        )
        .await;
    assert_eq!(result["closedPaths"], json!([renamed]));
    assert!(taide_layout::service::find_tab(&fixture.current().root, &id).is_none());
    fixture.services.tasks.shutdown().await;
}

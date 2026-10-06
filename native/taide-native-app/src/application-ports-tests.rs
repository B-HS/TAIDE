use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::{Value, json};
use taide_model::app_event::AppEvent;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::settings::Settings;
use taide_runtime::{AppState, EventSink, TaskSupervisor};

use super::*;
use crate::terminal_host::Limits;

const QUEUE_BYTES: usize = 256 * 1024;
const QUEUE_COUNT: usize = 64;
const QUEUE_VISITS: usize = 4096;
const HISTORY: usize = 128;
const FONT: u32 = 19;
const TERMINAL_FONT: u32 = 18;
const DEADLINE: Duration = Duration::from_secs(5);
const ASSET: &[u8] = b"synthetic remote assets";

#[derive(Default)]
struct Sink(Mutex<Vec<AppEvent>>);

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

struct Directory(PathBuf);

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn terminal_limits() -> Limits {
    Limits {
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
    }
}

#[cfg(unix)]
#[tokio::test]
async fn agent_poll은_foreground_pid와_probe로_상태를_발행하고_재등록없이_종료에서_회수한다() {
    use taide_agent::constants::AGENT_NAME_CLAUDE;
    use taide_model::project::Project;
    use taide_model::terminal::PtySpawnOptions;
    use taide_native_terminal::{Rgb, WindowSize};

    const COLUMNS: u16 = 80;
    const ROWS: u16 = 24;
    const EVENT_POLL: Duration = Duration::from_millis(10);
    let directory = Directory(
        std::env::temp_dir().join(format!("taide-native-agent-poll-{}", ProjectId::new())),
    );
    std::fs::create_dir_all(&directory.0).unwrap();
    let root = directory.0.canonicalize().unwrap();
    let state = AppState::new(AppPaths::new(root.join("data")));
    {
        let mut settings = state.settings.write();
        settings.ide_integration_enabled = false;
        settings.agent_hooks_enabled = false;
        settings.remote_access_enabled = false;
    }
    let project = ProjectId::new();
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project.clone(),
            root: root.to_str().unwrap().into(),
            name: "synthetic agent poll".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let sink = Arc::new(Sink::default());
    let assembly = crate::bootstrap::assemble(
        state,
        TaskSupervisor::new(tokio::runtime::Handle::current()),
        sink.clone(),
    );
    let services = &assembly.services;
    let tabs =
        Arc::new(crate::terminal_tabs::Tabs::new(services.clone(), terminal_limits()).unwrap());
    let hub = tabs.hub().clone();
    let ports = Ports::new(
        services,
        assembly.git_events.clone(),
        crate::remote_terminal::Ports {
            terminals: hub.clone(),
            environment: Arc::new(|_| panic!("remote spawn is outside this poll test")),
            history: Arc::new(|_, _| HISTORY),
            effects: Arc::new(|_, _| panic!("remote effects are outside this poll test")),
        },
        Arc::new(|_| None),
        AppInfo {
            name: "synthetic TAIDE".into(),
            version: "synthetic".into(),
            platform: "synthetic".into(),
            arch: "synthetic".into(),
        },
    );
    let id = hub
        .spawn(
            PtySpawnOptions {
                project_id: project.clone(),
                cwd: root.to_str().unwrap().into(),
                shell: Some("/bin/cat".into()),
                cols: COLUMNS,
                rows: ROWS,
                scrollback_bytes: None,
            },
            HISTORY,
            async { Vec::new() },
            crate::terminal_dispatch::EffectPorts {
                command_colors: Default::default(),
                updated: Arc::new(|| {}),
                color: Arc::new(|_| Ok(Rgb { r: 0, g: 0, b: 0 })),
                geometry: Arc::new(|| {
                    Ok(WindowSize {
                        num_cols: COLUMNS,
                        num_lines: ROWS,
                        cell_width: 1,
                        cell_height: 1,
                    })
                }),
                event: Arc::new(|_| Ok(())),
                stream: Arc::new(|_| Ok(())),
            },
        )
        .await
        .unwrap();
    let foreground = services.terminal.foreground_pids(&project);
    let [(session, pid)] = foreground.as_slice() else {
        panic!("synthetic terminal has no single foreground process")
    };
    assert_eq!(session, &id);
    services
        .agents
        .remember_process_names([(*pid, Some(AGENT_NAME_CLAUDE))].into_iter().collect());
    assert!(services.agents.agents_for(&project).is_empty());
    ports.start(services.clone()).await;
    super::start_agent_poll(services.clone());
    let detected = tokio::time::timeout(DEADLINE, async {
        loop {
            let published = sink.0.lock().unwrap().iter().find_map(|event| match event {
                AppEvent::AgentStateChanged { project_id, agents } if project_id == &project => {
                    Some(agents.clone())
                }
                _ => None,
            });
            if let Some(agents) = published {
                return agents;
            }
            tokio::time::sleep(EVENT_POLL).await;
        }
    })
    .await
    .unwrap();
    let [agent] = detected.as_slice() else {
        panic!("agent poll did not publish exactly one detected agent")
    };
    assert_eq!(agent.session_id, id);
    assert_eq!(agent.name, AGENT_NAME_CLAUDE);
    assert_eq!(agent.pid, *pid);
    assert_eq!(services.agents.agents_for(&project).len(), 1);
    let registered = services.tasks.tracked_count();
    super::start_agent_poll(services.clone());
    assert_eq!(services.tasks.tracked_count(), registered);
    tokio::time::timeout(DEADLINE, hub.close(&id))
        .await
        .unwrap()
        .unwrap();
    ports.stop(services);
    services.terminal.shutdown();
    tokio::time::timeout(DEADLINE, services.terminal.wait_for_idle())
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(DEADLINE, services.tasks.shutdown())
        .await
        .unwrap();
    assert_eq!(services.tasks.tracked_count(), 0);
}

#[tokio::test]
async fn production_graph는_실제_dispatch_persist_event와_lazy_ports_소유권을_연결한다() {
    let directory = Directory(std::env::temp_dir().join(format!(
        "taide-native-application-ports-{}",
        ProjectId::new()
    )));
    std::fs::create_dir_all(&directory.0).unwrap();
    let state = AppState::new(AppPaths::new(directory.0.clone()));
    {
        let mut settings = state.settings.write();
        settings.ide_integration_enabled = false;
        settings.agent_hooks_enabled = false;
        settings.remote_access_enabled = false;
    }
    let sink = Arc::new(Sink::default());
    let assembly = crate::bootstrap::assemble(
        state,
        TaskSupervisor::new(tokio::runtime::Handle::current()),
        sink.clone(),
    );
    let services = &assembly.services;
    let tabs =
        Arc::new(crate::terminal_tabs::Tabs::new(services.clone(), terminal_limits()).unwrap());
    let hub = tabs.hub().clone();
    let environment: crate::terminal_environment::Environment =
        Arc::new(|_| panic!("terminal spawn is outside this graph test"));
    let asset_reads = Arc::new(AtomicUsize::new(0));
    let reads = asset_reads.clone();
    let info = AppInfo {
        name: "synthetic TAIDE".into(),
        version: "synthetic".into(),
        platform: "synthetic".into(),
        arch: "synthetic".into(),
    };
    let expected_info = serde_json::to_value(&info).unwrap();
    let ports = Ports::new(
        services,
        assembly.git_events.clone(),
        crate::remote_terminal::Ports {
            terminals: hub.clone(),
            environment: environment.clone(),
            history: Arc::new(|_, _| HISTORY),
            effects: Arc::new(|_, _| panic!("terminal effects are outside this graph test")),
        },
        Arc::new(move |path| {
            reads.fetch_add(1, Ordering::Relaxed);
            (path == "index.html").then(|| crate::remote_serving::Asset {
                mime: "text/html".into(),
                bytes: ASSET.to_vec(),
            })
        }),
        info,
    );
    assert_eq!(asset_reads.load(Ordering::Relaxed), 0);
    assert_eq!(services.tasks.tracked_count(), 0);
    ports.start(services.clone()).await;
    assert!(!services.ide.is_running());
    assert!(!services.remote.is_running());
    assert!(services.agent_hooks.server_info().is_none());
    assert_eq!(services.tasks.tracked_count(), 0);
    let channels: crate::remote_ws::ChannelFactory =
        Arc::new(|_| panic!("non-channel commands must not allocate a channel in this graph test"));
    let actual: Value = serde_json::from_str(
        &(ports.dispatch.json)(
            services.clone(),
            "app_get_info".into(),
            json!({}),
            channels.clone(),
        )
        .await
        .unwrap(),
    )
    .unwrap();
    assert_eq!(actual, expected_info);
    let mut events = services.remote.subscribe_events();
    let saved: Settings = serde_json::from_str(
        &(ports.dispatch.json)(
            services.clone(),
            "settings_update".into(),
            json!({"patch":{"editorFontSize":FONT}}),
            channels.clone(),
        )
        .await
        .unwrap(),
    )
    .unwrap();
    assert_eq!(saved.editor_font_size, FONT);
    assert_eq!(saved, *services.state.settings.read());
    assert_eq!(
        saved,
        serde_json::from_slice::<Settings>(
            &std::fs::read(services.state.paths.settings_file()).unwrap()
        )
        .unwrap()
    );
    let event: Value = serde_json::from_str(&events.try_recv().unwrap()).unwrap();
    assert_eq!(event["event"], "settings:changed");
    let payload: Value = serde_json::from_str(event["payload"].as_str().unwrap()).unwrap();
    assert_eq!(payload["settings"], serde_json::to_value(saved).unwrap());
    assert!(matches!(
        sink.0.lock().unwrap().as_slice(),
        [AppEvent::SettingsChanged { .. }]
    ));
    let ready = Arc::new(tokio::sync::Notify::new());
    let signal = ready.clone();
    let mut host = crate::host::HostBridge::connect_with_application_ports(
        services.clone(),
        Arc::new(move || signal.notify_one()),
        crate::host::Terminals {
            tabs: tabs.clone(),
            environment,
        },
        &ports,
    )
    .unwrap();
    host.submit(crate::host::HostCommand::SetTerminalFontSize(TERMINAL_FONT))
        .unwrap();
    tokio::time::timeout(DEADLINE, ready.notified())
        .await
        .unwrap();
    assert!(host.poll().is_none());
    assert_eq!(
        services.state.settings.read().terminal_font_size,
        TERMINAL_FONT
    );
    assert!(!services.ide.is_running());
    assert!(!services.remote.is_running());
    assert!(services.agent_hooks.server_info().is_none());
    assert_eq!(asset_reads.load(Ordering::Relaxed), 0);
    let event: Value = serde_json::from_str(&events.try_recv().unwrap()).unwrap();
    assert_eq!(event["event"], "settings:changed");
    let payload: Value = serde_json::from_str(event["payload"].as_str().unwrap()).unwrap();
    assert_eq!(payload["settings"]["terminalFontSize"], TERMINAL_FONT);
    tokio::time::timeout(DEADLINE, host.disconnect())
        .await
        .unwrap()
        .unwrap();
    assert!(
        (ports.dispatch.json)(services.clone(), "app_exit".into(), json!({}), channels)
            .await
            .is_err()
    );
    assert_eq!((ports.remote.assets)("index.html").unwrap().bytes, ASSET);
    assert_eq!(asset_reads.load(Ordering::Relaxed), 1);
    ports.stop(services);
    services.tasks.shutdown().await;
    assert_eq!(services.tasks.tracked_count(), 0);
    let remote = Arc::downgrade(&ports.remote);
    let ide = Arc::downgrade(&ports.ide);
    let terminal = Arc::downgrade(&hub);
    let owner = Arc::downgrade(services);
    let dispatch = ports.dispatch.clone();
    drop(ports);
    assert!(remote.upgrade().is_none());
    assert!(ide.upgrade().is_some());
    drop(dispatch);
    assert!(ide.upgrade().is_none());
    drop(tabs);
    drop(hub);
    assert!(terminal.upgrade().is_none());
    drop(assembly);
    assert!(owner.upgrade().is_none());
}

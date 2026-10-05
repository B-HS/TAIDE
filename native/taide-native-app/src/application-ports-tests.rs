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
    let tabs = Arc::new(
        crate::terminal_tabs::Tabs::new(
            services.clone(),
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
            },
        )
        .unwrap(),
    );
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

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::{Value, json};
use taide_infra::secret::{SecretAccount, SecretStore, SecretStoreState};
use taide_model::agent::CliInstallStatus;
use taide_model::app::AppInfo;
use taide_model::app_event::AppEvent;
use taide_model::error::AppResult;
use taide_model::font::FontFamily;
use taide_model::ids::ProjectId;
use taide_model::layout::ProjectLayout;
use taide_model::paths::AppPaths;
use taide_model::settings::Settings;
use taide_model::system::SystemUsage;
use taide_model::terminal::PtySpawnOptions;
use taide_remote::command_policy::{REMOTE_ALLOWED_COMMANDS, REMOTE_DENIED_COMMANDS};
use taide_runtime::{
    AppState, EventSink, IdeSaveFile, PlatformServicesState, RemoteDispatchLimiter, TaskSupervisor,
    save_file_within_open_projects,
};
use taide_system::service::ProcessRecord;
use tokio::sync::Notify;

use super::*;
use crate::projects::NativeProjects;
use crate::remote_ws::{ChannelFactory, ResponseBody};
use crate::terminal_dispatch::ObservePorts;
use crate::terminal_host::{Hub, Limits};

const DEADLINE: Duration = Duration::from_secs(5);
const REMOTE_CONCURRENT: usize = 128;
const QUEUE_BYTES: usize = 256 * 1024;
const QUEUE_COUNT: usize = 64;
const QUEUE_VISITS: usize = 4096;
const HISTORY: usize = 128;
const COLUMNS: u16 = 80;
const ROWS: u16 = 24;
const FONT: u32 = 19;
const SYNC_FONT: u32 = 21;
const TEXT: &str = "synthetic dispatcher needle\n";
const GIST: &str = "synthetic-gist";
const DATE: &str = "2026-10-04T00:00:00Z";

struct Secrets(String);

impl SecretStore for Secrets {
    fn set(&self, _: SecretAccount, _: &str) -> AppResult<()> {
        panic!("dispatcher must not mutate credentials");
    }
    fn get(&self, account: SecretAccount) -> AppResult<Option<String>> {
        Ok((account == SecretAccount::GithubSync).then(|| self.0.clone()))
    }
    fn delete(&self, _: SecretAccount) -> AppResult<()> {
        panic!("dispatcher must not mutate credentials");
    }
}

struct Gist;

impl SyncGistPort for Gist {
    async fn discover_sync_gist(
        &self,
        _: &str,
        _: Option<&str>,
    ) -> AppResult<Option<(String, String)>> {
        panic!("discovery is outside this integration");
    }
    async fn create_gist(&self, _: &str, _: &str) -> AppResult<(String, String)> {
        panic!("upload is outside this integration");
    }
    async fn update_gist(&self, _: &str, _: &str, _: &str) -> AppResult<String> {
        panic!("upload is outside this integration");
    }
    async fn fetch_gist(&self, _: &str, id: &str) -> AppResult<(String, String)> {
        assert_eq!(id, GIST);
        Ok((
            DATE.into(),
            json!({"schemaVersion":1,"updatedAt":DATE,
            "settings":{"editorFontSize":SYNC_FONT}})
            .to_string(),
        ))
    }
}

struct Usage;

impl remote_utilities::UsageProvider for Usage {
    fn collect_app_usage(&self) -> AppResult<SystemUsage> {
        panic!("OS sampling is outside this integration");
    }
    fn refresh_process_records(&self) -> Vec<ProcessRecord> {
        panic!("OS sampling is outside this integration");
    }
}

#[derive(Default)]
struct Sink(Mutex<Vec<AppEvent>>);

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

#[derive(Default)]
struct Channels {
    bodies: Mutex<Vec<(String, ResponseBody)>>,
    changed: Notify,
}

impl Channels {
    fn factory(self: &Arc<Self>) -> ChannelFactory {
        let channels = self.clone();
        Arc::new(move |id| {
            let channels = channels.clone();
            Box::new(move |body| {
                channels.bodies.lock().unwrap().push((id.clone(), body));
                channels.changed.notify_waiters();
                Ok(())
            })
        })
    }

    async fn raw_contains(&self, id: &str, text: &str) {
        tokio::time::timeout(DEADLINE, async {
            loop {
                let changed = self.changed.notified();
                tokio::pin!(changed);
                changed.as_mut().enable();
                let bytes: Vec<_> = self
                    .bodies
                    .lock()
                    .unwrap()
                    .iter()
                    .filter_map(|(channel, body)| {
                        if channel != id {
                            return None;
                        }
                        match body {
                            ResponseBody::Raw(bytes) => Some(bytes.as_slice()),
                            _ => None,
                        }
                    })
                    .flatten()
                    .copied()
                    .collect();
                if String::from_utf8_lossy(&bytes).contains(text) {
                    return;
                }
                changed.await;
            }
        })
        .await
        .unwrap();
    }
}

struct Fixture {
    directory: PathBuf,
    root: PathBuf,
    services: Arc<AppServices>,
    hub: Arc<Hub>,
    sink: Arc<Sink>,
    channels: Arc<Channels>,
    factory_calls: Arc<AtomicUsize>,
    reconcile_calls: Arc<AtomicUsize>,
    backend: Dispatch,
}

impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "taide-native-remote-dispatch-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(directory.join("project")).unwrap();
        let directory = directory.canonicalize().unwrap();
        let root = directory.join("project");
        let state = AppState::new(AppPaths::new(directory.join("data")));
        {
            let mut settings = state.settings.write();
            settings.agent_hooks_enabled = false;
            settings.ide_integration_enabled = false;
            settings.sync_gist_id = Some(GIST.into());
        }
        let sink = Arc::new(Sink::default());
        let services = Arc::new(AppServices::new(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            RemoteDispatchLimiter::new(REMOTE_CONCURRENT),
            PlatformServicesState::new(Arc::new(crate::bootstrap::NativePlatform)),
            SecretStoreState(Arc::new(Secrets(uuid::Uuid::new_v4().to_string()))),
            IdeSaveFile(save_file_within_open_projects),
            sink.clone(),
        ));
        let hub = Arc::new(
            Hub::new(
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
        let reconcile_calls = Arc::new(AtomicUsize::new(0));
        let recorded = reconcile_calls.clone();
        let expected = services.clone();
        let preferences = remote_preferences::Ports {
            info: AppInfo {
                name: "TAIDE".into(),
                version: "synthetic".into(),
                platform: "synthetic".into(),
                arch: "synthetic".into(),
            },
            reconcile: Arc::new(move |services, _, updated| {
                assert!(Arc::ptr_eq(&services, &expected));
                let recorded = recorded.clone();
                Box::pin(async move {
                    assert_eq!(*services.state.settings.read(), updated);
                    let saved: Settings = serde_json::from_slice(
                        &std::fs::read(services.state.paths.settings_file()).unwrap(),
                    )
                    .unwrap();
                    assert_eq!(saved, updated);
                    recorded.fetch_add(1, Ordering::AcqRel);
                })
            }),
        };
        let ports = Ports {
            preferences,
            agents: remote_agents::Ports {
                foreground_pids: Arc::new(|_, _| panic!("probe is outside this integration")),
                probe: Arc::new(|_, _| panic!("probe is outside this integration")),
                cli_status: Arc::new(|| CliInstallStatus {
                    installed: false,
                    resolved_path: None,
                    dangling: false,
                    target_path: "synthetic-cli".into(),
                    editor_env_hint: "synthetic-editor".into(),
                }),
                resolve_home: Arc::new(|| panic!("home is outside this integration")),
                project_emitter: Arc::new(|_| panic!("hook startup is outside this integration")),
                cli_target: "synthetic-cli".into(),
            },
            git: remote_git::Ports {
                install_status_invalidation: Arc::new(|_| {
                    panic!("Git observer is outside this integration")
                }),
            },
            lsp: remote_lsp::Ports {
                create_process: Arc::new(|_, _, _, _, _| {
                    panic!("LSP spawn is outside this integration")
                }),
                path_var: Arc::new(|| panic!("OS PATH is outside this integration")),
            },
            terminal: remote_terminal::Ports {
                terminals: hub.clone(),
                environment: Arc::new(|_| Box::pin(async { Vec::new() })),
                history: Arc::new(|_, _| HISTORY),
                effects: Arc::new(|_, _| {
                    Ok(ObservePorts {
                        command_colors: Default::default(),
                        updated: Arc::new(|| {}),
                        event: Arc::new(|_| Ok(())),
                        stream: Arc::new(|_| Ok(())),
                    })
                }),
            },
            utilities: remote_utilities::Ports {
                fonts: Arc::new(|| {
                    vec![FontFamily {
                        name: "Synthetic dispatcher".into(),
                        monospaced: true,
                    }]
                }),
                usage: Arc::new(Usage),
                root_pid: Arc::new(|| panic!("OS PID is outside this integration")),
                cpu_count: Arc::new(|| panic!("OS CPU is outside this integration")),
                label_providers: Vec::new(),
            },
            create_gist_client: Arc::new(|| Gist),
        };
        let factory_calls = Arc::new(AtomicUsize::new(0));
        let recorded = factory_calls.clone();
        let expected = services.clone();
        let backend = create_dispatch(ports, move |services: Arc<AppServices>| {
            assert!(Arc::ptr_eq(&services, &expected));
            recorded.fetch_add(1, Ordering::AcqRel);
            NativeProjects::new(services)
        });
        Self {
            directory,
            root,
            services,
            hub,
            sink,
            channels: Arc::default(),
            factory_calls,
            reconcile_calls,
            backend,
        }
    }

    async fn call(&self, name: &str, args: Value) -> Result<Value, Value> {
        let text = tokio::time::timeout(
            DEADLINE,
            (self.backend.json)(
                self.services.clone(),
                name.into(),
                args,
                self.channels.factory(),
            ),
        )
        .await
        .unwrap()?;
        Ok(serde_json::from_str(&text).unwrap())
    }

    async fn open(&self) -> ProjectId {
        let result = self
            .call("project_open", json!({"path":self.root}))
            .await
            .unwrap();
        serde_json::from_value(result["project"]["id"].clone()).unwrap()
    }

    async fn finish(&self) {
        let projects: Vec<_> = self
            .services
            .state
            .projects
            .read()
            .keys()
            .cloned()
            .collect();
        for project in projects {
            self.call("project_close", json!({"projectId":project}))
                .await
                .unwrap();
        }
        tokio::time::timeout(DEADLINE, self.services.terminal.wait_for_idle())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(DEADLINE, self.services.tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(self.services.tasks.tracked_count(), 0);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.terminal.kill_all();
        self.services.tasks.stop_all();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[tokio::test]
async fn catalog177과_전체_chain의_입력_routing_default_deny_모드를_검증한다() {
    let catalogs = [
        remote_preferences::COMMANDS,
        remote_ide::COMMANDS,
        remote_files::JSON_COMMANDS,
        remote_layout::COMMANDS,
        remote_projects::COMMANDS,
        remote_search::COMMANDS,
        remote_plugins::COMMANDS,
        remote_git::COMMANDS,
        remote_agents::COMMANDS,
        remote_lsp::COMMANDS,
        remote_terminal::COMMANDS,
        remote_utilities::COMMANDS,
        remote_ai::COMMANDS,
        remote_sync::COMMANDS,
    ];
    let declared: Vec<_> = catalogs
        .into_iter()
        .flatten()
        .copied()
        .chain(["file_read_raw"])
        .collect();
    let unique: BTreeSet<_> = declared.iter().copied().collect();
    assert_eq!(declared.len(), REMOTE_ALLOWED_COMMANDS.len());
    assert_eq!(declared.len(), unique.len());
    assert_eq!(unique, REMOTE_ALLOWED_COMMANDS.iter().copied().collect());
    let fixture = Fixture::new();
    for name in [
        "file_open",
        "layout_get",
        "project_get",
        "search_run",
        "plugin_read_grammar",
        "git_init",
        "agent_list",
        "lsp_sessions",
        "pty_spawn",
        "detect_tasks",
        "ai_inline_complete",
        "sync_download",
    ] {
        assert_eq!(
            fixture.call(name, json!({})).await.unwrap_err()["code"],
            "InvalidArgument",
            "{name}"
        );
    }
    assert_eq!(
        fixture.call("app_get_info", json!({})).await.unwrap()["version"],
        "synthetic"
    );
    assert!(fixture.call("ide_get_status", json!({})).await.is_ok());
    for name in REMOTE_DENIED_COMMANDS
        .iter()
        .map(|(name, _)| *name)
        .chain(["unclassified_command", "file_read_raw"])
    {
        assert_eq!(
            fixture.call(name, json!({})).await.unwrap_err()["message"]["kind"],
            "Forbidden",
            "{name}"
        );
    }
    let error = (fixture.backend.raw)(fixture.services.clone(), "project_list".into(), json!({}))
        .await
        .unwrap_err();
    assert_eq!(error["message"]["kind"], "Forbidden");
    assert_eq!(fixture.factory_calls.load(Ordering::Acquire), 0);
    assert_eq!(fixture.reconcile_calls.load(Ordering::Acquire), 0);
    fixture.finish().await;
}

#[tokio::test]
async fn 실제_project_file_raw_search_channel과_설정_sync는_동일_services와_reconcile을_사용한다() {
    let _os_watch_registration = NativeProjects::exclusive_os_watch_registration().await;
    let fixture = Fixture::new();
    let project = fixture.open().await;
    assert_eq!(fixture.factory_calls.load(Ordering::Acquire), 1);
    let path = fixture.root.join("synthetic.txt");
    fixture
        .call("file_create", json!({"path":path,"isDir":false}))
        .await
        .unwrap();
    fixture
        .call("file_save", json!({"path":path,"content":TEXT}))
        .await
        .unwrap();
    let bytes = (fixture.backend.raw)(
        fixture.services.clone(),
        "file_read_raw".into(),
        json!({"path":path}),
    )
    .await
    .unwrap();
    assert_eq!(bytes, TEXT.as_bytes());
    fixture
        .call(
            "search_run",
            json!({"projectId":project,"owner":"main","sessionId":"synthetic-search",
        "query":{"text":"needle"},"onMatch":"__CHANNEL__:search"}),
        )
        .await
        .unwrap();
    assert!(
        fixture
            .channels
            .bodies
            .lock()
            .unwrap()
            .iter()
            .any(|(id, body)| {
                id == "search"
                    && matches!(body, ResponseBody::Json(text) if text.contains("needle"))
            })
    );
    assert!(
        fixture
            .call("plugin_list", json!({}))
            .await
            .unwrap()
            .is_array()
    );
    assert_eq!(
        fixture.call("agent_cli_status", json!({})).await.unwrap()["targetPath"],
        "synthetic-cli"
    );
    assert_eq!(
        fixture.call("font_list", json!({})).await.unwrap()[0]["name"],
        "Synthetic dispatcher"
    );
    fixture
        .call(
            "settings_update",
            json!({"patch":{"editorFontSize":FONT,"remotePasswordOnlyLogin":true}}),
        )
        .await
        .unwrap();
    assert_eq!(
        fixture.services.state.settings.read().editor_font_size,
        FONT
    );
    assert!(
        !fixture
            .services
            .state
            .settings
            .read()
            .remote_password_only_login
    );
    fixture
        .call("sync_download", json!({"force":true}))
        .await
        .unwrap();
    assert_eq!(
        fixture.services.state.settings.read().editor_font_size,
        SYNC_FONT
    );
    assert_eq!(fixture.reconcile_calls.load(Ordering::Acquire), 2);
    {
        let events = fixture.sink.0.lock().unwrap();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, AppEvent::SettingsChanged { .. }))
                .count(),
            2
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, AppEvent::SyncStateChanged { .. }))
                .count(),
            1
        );
    }
    fixture.finish().await;
}

#[cfg(unix)]
#[tokio::test]
async fn 실제_pty_spawn_attach와_layout_close는_공유_hub_core_child를_회수한다() {
    let _os_watch_registration = NativeProjects::exclusive_os_watch_registration().await;
    let fixture = Fixture::new();
    let project = fixture.open().await;
    let opts = PtySpawnOptions {
        project_id: project.clone(),
        cwd: fixture.root.to_str().unwrap().into(),
        shell: Some("/bin/cat".into()),
        cols: COLUMNS,
        rows: ROWS,
        scrollback_bytes: None,
    };
    let result = fixture
        .call("pty_spawn", json!({"opts":opts,"onData":"initial"}))
        .await
        .unwrap();
    let id = result.as_str().unwrap().to_owned();
    let session = fixture.hub.get(&id).unwrap();
    fixture
        .call(
            "pty_attach",
            json!({"sessionId":id,"onData":"__CHANNEL__:terminal"}),
        )
        .await
        .unwrap();
    fixture
        .call("pty_write", json!({"sessionId":id,"data":TEXT}))
        .await
        .unwrap();
    fixture
        .channels
        .raw_contains("terminal", "synthetic dispatcher needle")
        .await;
    let layout = fixture
        .call(
            "layout_open_tab",
            json!({"projectId":project,
        "kind":{"kind":"terminal","sessionId":id,"cwd":fixture.root},
        "title":"Synthetic","target":null,"preview":false}),
        )
        .await
        .unwrap();
    let layout: ProjectLayout = serde_json::from_value(layout).unwrap();
    let tab = taide_layout::service::find_tab_by_title(&layout.root, "Synthetic").unwrap();
    fixture
        .call("layout_close_tab", json!({"tabId":tab}))
        .await
        .unwrap();
    assert!(fixture.hub.get(&id).is_none());
    tokio::time::timeout(DEADLINE, session.wait_dispatch())
        .await
        .unwrap()
        .unwrap();
    assert!(session.is_finished());
    drop(session);
    fixture.finish().await;
}

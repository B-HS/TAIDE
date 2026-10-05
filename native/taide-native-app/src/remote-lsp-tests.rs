use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::json;
use taide_infra::lsp_proc::{self, LspProcConfig};
use taide_lsp::manifest;
use taide_lsp::session::LspMessageSubscribers;
use taide_lsp::store::LspSessionEntry;
use taide_model::app_event::AppEvent;
use taide_model::ids::ProjectId;
use taide_model::lsp::LspSessionStatus;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_remote::command_policy::{self, REMOTE_ALLOWED_COMMANDS};
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::mpsc;

use super::*;
use crate::remote_gateway;
use crate::remote_ws::ResponseBody;

const DEADLINE: Duration = Duration::from_secs(6);
const RESPONSE_CAPACITY: usize = 16;
const SERVER: &str = "gopls";
const SESSION: &str = "synthetic-session";
const OTHER_OWNER: &str = "main";
const FIRST_GENERATION: u32 = 1;

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
    bin: PathBuf,
    project: ProjectId,
    services: Arc<AppServices>,
    sink: Arc<Sink>,
    processes: Arc<AtomicUsize>,
    exits: Arc<AtomicUsize>,
    backend: Dispatch,
}

impl Fixture {
    fn new(real_process: bool) -> Self {
        let project = ProjectId::new();
        let directory = std::env::temp_dir().join(format!("taide-native-remote-lsp-{project}"));
        std::fs::create_dir_all(directory.join("project")).unwrap();
        std::fs::create_dir_all(directory.join("bin")).unwrap();
        let directory = directory.canonicalize().unwrap();
        let root = directory.join("project");
        let bin = directory.join("bin");
        let state = AppState::new(AppPaths::new(directory.join("data")));
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.to_str().unwrap().into(),
                name: "synthetic LSP".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        let sink = Arc::new(Sink::default());
        let services = crate::bootstrap::services(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            sink.clone(),
        );
        let processes = Arc::new(AtomicUsize::new(0));
        let exits = Arc::new(AtomicUsize::new(0));
        let process_count = processes.clone();
        let exit_count = exits.clone();
        let path = bin.as_os_str().to_os_string();
        let ports = Ports {
            create_process: Arc::new(move |services, id, epoch, spec, root| {
                process_count.fetch_add(1, Ordering::AcqRel);
                assert_eq!(spec.id.as_str(), SERVER);
                assert!(
                    services
                        .lsp
                        .get(&id)
                        .unwrap()
                        .lifecycle
                        .is_active_process_epoch(epoch)
                );
                if !real_process {
                    return Err(AppError::Internal("synthetic process rejection".into()));
                }
                let mock = std::env::current_exe()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .join("examples/native-lsp-mock");
                assert!(
                    mock.is_file(),
                    "existing native-lsp-mock example is required"
                );
                let owner = Arc::downgrade(&services);
                let exited = exit_count.clone();
                lsp_proc::spawn(
                    LspProcConfig {
                        command: mock.to_str().unwrap().into(),
                        args: Vec::new(),
                        cwd: PathBuf::from(root),
                    },
                    move |message| {
                        if let Some(services) = owner.upgrade()
                            && let Some(entry) = services.lsp.get(&id)
                            && entry.lifecycle.is_active_process_epoch(epoch)
                        {
                            entry.subscribers.broadcast(&message);
                        }
                    },
                    move |_, _| {
                        exited.fetch_add(1, Ordering::AcqRel);
                    },
                )
                .map(Arc::new)
            }),
            path_var: Arc::new(move || path.clone()),
        };
        let remaining = Dispatch {
            json: Arc::new(|_, name, args, channels| {
                Box::pin(async move {
                    assert_eq!(name, "layout_get");
                    channels("remaining".into())(ResponseBody::Json("null".into())).unwrap();
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
            bin,
            project,
            services,
            sink,
            processes,
            exits,
            backend: remote_gateway::with_policy(extend_backend(ports, remaining)),
        }
    }

    fn request(&self, root: &std::path::Path) -> Value {
        json!({"projectId":self.project,"serverId":SERVER,"root":root,"owner":OTHER_OWNER})
    }

    async fn call(
        &self,
        name: &str,
        args: Value,
        channels: ChannelFactory,
    ) -> Result<Value, Value> {
        let text = (self.backend.json)(self.services.clone(), name.into(), args, channels).await?;
        Ok(serde_json::from_str(&text).unwrap())
    }

    async fn ok(&self, name: &str, args: Value) -> Value {
        self.call(name, args, Arc::new(|_| panic!("unexpected LSP channel")))
            .await
            .unwrap_or_else(|error| panic!("{name}: {error}"))
    }

    fn seed(&self, owner: &str) -> Arc<LspSessionEntry> {
        let subscribers = LspMessageSubscribers::new();
        subscribers.insert(owner.into(), |_| true);
        let entry = Arc::new(LspSessionEntry::new(
            self.project.clone(),
            manifest::find_spec(SERVER).unwrap(),
            self.root.to_str().unwrap().into(),
            subscribers,
        ));
        entry.lifecycle.set_status(LspSessionStatus::Running, None);
        self.services.lsp.insert(SESSION.into(), entry.clone());
        entry
    }

    async fn finish(&self) {
        self.services.lsp.shutdown();
        tokio::time::timeout(DEADLINE, self.services.lsp.wait_for_idle())
            .await
            .unwrap();
        tokio::time::timeout(DEADLINE, self.services.tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(self.services.tasks.tracked_count(), 0);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.lsp.shutdown();
        self.services.tasks.stop_all();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

fn channel(id: &'static str, sender: mpsc::Sender<String>) -> ChannelFactory {
    Arc::new(move |actual| {
        assert_eq!(actual, id);
        let sender = sender.clone();
        Box::new(move |body| {
            let ResponseBody::Json(body) = body else {
                panic!("LSP channel must send JSON strings")
            };
            sender
                .try_send(serde_json::from_str::<String>(&body).unwrap())
                .map_err(|error| AppError::Internal(error.to_string()))
        })
    })
}

#[tokio::test]
async fn 목록10_actual_arm_입력_channel과_거절시_factory_비실행을_보존한다() {
    let declared: BTreeSet<_> = COMMANDS.iter().copied().collect();
    assert_eq!(declared.len(), COMMANDS.len());
    assert!(
        declared
            .iter()
            .all(|name| REMOTE_ALLOWED_COMMANDS.contains(name))
    );
    let pattern = regex::Regex::new(r#"(?m)^        "([a-z_]+)" =>"#).unwrap();
    let arms: BTreeSet<_> = pattern
        .captures_iter(include_str!("remote-lsp.rs"))
        .map(|capture| capture.get(1).unwrap().as_str())
        .collect();
    assert_eq!(declared, arms);
    for previous in [
        crate::remote_agents::COMMANDS,
        crate::remote_git::COMMANDS,
        crate::remote_preferences::COMMANDS,
        crate::remote_ide::COMMANDS,
        crate::remote_files::JSON_COMMANDS,
        crate::remote_projects::COMMANDS,
        crate::remote_layout::COMMANDS,
        crate::remote_search::COMMANDS,
        crate::remote_plugins::COMMANDS,
    ] {
        assert!(declared.iter().all(|name| !previous.contains(name)));
    }
    let fixture = Fixture::new(false);
    let no_channel: ChannelFactory = Arc::new(|_| panic!("unexpected channel factory"));
    let error = fixture
        .call(
            "lsp_spawn",
            json!({"request":fixture.request(&fixture.root)}),
            no_channel.clone(),
        )
        .await
        .unwrap_err();
    assert_eq!(error["code"], "Localized");
    assert_eq!(error["message"]["kind"], "InvalidArgument");
    assert_eq!(error["message"]["key"], "error.remote.channelArgRequired");
    assert_eq!(
        fixture
            .call(
                "lsp_spawn",
                json!({"request":false,"onMessage":"1"}),
                no_channel.clone()
            )
            .await
            .unwrap_err()["code"],
        "InvalidArgument"
    );
    let channels: ChannelFactory = Arc::new(|id| {
        assert_eq!(id, "1");
        Box::new(|_| panic!("rejected spawn must not publish a message"))
    });
    let mut request = fixture.request(&fixture.root);
    request["projectId"] = json!(ProjectId::new());
    assert_eq!(
        fixture
            .call(
                "lsp_spawn",
                json!({"request":request,"onMessage":"__CHANNEL__:1"}),
                channels.clone()
            )
            .await
            .unwrap_err()["code"],
        "NotFound"
    );
    request["projectId"] = json!(fixture.project);
    request["serverId"] = json!("synthetic-unknown");
    assert_eq!(
        fixture
            .call(
                "lsp_spawn",
                json!({"request":request,"onMessage":"1"}),
                channels.clone()
            )
            .await
            .unwrap_err()["code"],
        "InvalidArgument"
    );
    assert_eq!(fixture.processes.load(Ordering::Acquire), 0);
    assert_eq!(
        fixture
            .call("lsp_install", json!({}), no_channel.clone())
            .await
            .unwrap_err(),
        error_value(command_policy::admit("lsp_install").unwrap_err())
    );
    assert_eq!(
        fixture
            .call(
                "lsp_spawn",
                json!({"request":fixture.request(&fixture.root),"onMessage":"1"}),
                channels.clone()
            )
            .await
            .unwrap_err()["code"],
        "Internal"
    );
    assert_eq!(fixture.processes.load(Ordering::Acquire), 1);
    assert!(
        fixture
            .services
            .lsp
            .sessions_for_project(&fixture.project)
            .is_empty()
    );
    assert!(fixture.sink.0.lock().unwrap().is_empty());
    let missing = json!({"sessionId":"synthetic-missing","owner":OTHER_OWNER,"root":null,"generation":FIRST_GENERATION,"message":"{}"});
    for name in [
        "lsp_send",
        "lsp_stop",
        "lsp_restart",
        "lsp_confirm_reinitialize",
        "lsp_report_reinitialize_failure",
    ] {
        assert_eq!(
            fixture
                .call(name, missing.clone(), no_channel.clone())
                .await
                .unwrap_err()["code"],
            "NotFound"
        );
    }
    let remaining: ChannelFactory = Arc::new(|id| {
        assert_eq!(id, "remaining");
        Box::new(|body| {
            assert!(matches!(body, ResponseBody::Json(_)));
            Ok(())
        })
    });
    assert_eq!(
        fixture
            .call("layout_get", json!({"owner":OTHER_OWNER}), remaining)
            .await
            .unwrap()["owner"],
        "remote"
    );
    assert_eq!(
        (fixture.backend.raw)(fixture.services.clone(), "file_read_raw".into(), json!({}))
            .await
            .unwrap(),
        vec![0]
    );
    fixture.finish().await;
    assert_eq!(
        fixture
            .call(
                "lsp_spawn",
                json!({"request":fixture.request(&fixture.root),"onMessage":"1"}),
                channels
            )
            .await
            .unwrap_err()["code"],
        "Forbidden"
    );
    assert_eq!(fixture.processes.load(Ordering::Acquire), 1);
}

#[tokio::test]
async fn 실제_reuse_owner_roots_generation_cancel과_조회_원본수명을_보존한다() {
    let fixture = Fixture::new(false);
    let entry = fixture.seed("remote");
    let second = fixture.root.join("second");
    std::fs::create_dir_all(&second).unwrap();
    let (sender, mut receiver) = mpsc::channel(RESPONSE_CAPACITY);
    assert_eq!(
        fixture
            .call(
                "lsp_spawn",
                json!({"request":fixture.request(&second),"onMessage":"__CHANNEL__:2"}),
                channel("2", sender)
            )
            .await
            .unwrap(),
        SESSION
    );
    assert_eq!(fixture.processes.load(Ordering::Acquire), 0);
    assert_eq!(entry.roots.paths().len(), 2);
    assert!(entry.subscribers.contains("remote"));
    assert!(!entry.subscribers.contains(OTHER_OWNER));
    entry.subscribers.broadcast("{\"synthetic\":true}");
    assert_eq!(receiver.recv().await.unwrap(), "{\"synthetic\":true}");
    let remote = fixture
        .ok("lsp_sessions", json!({"projectId":fixture.project}))
        .await;
    assert_eq!(remote[0]["sessionId"], SESSION);
    assert_eq!(remote[0]["status"], "running");
    assert_eq!(
        fixture
            .ok("lsp_sessions", json!({"projectId":ProjectId::new()}))
            .await,
        json!([])
    );
    let generation = entry
        .lifecycle
        .auto_respawned("synthetic reinitialize".into())
        .generation;
    fixture
        .ok(
            "lsp_confirm_reinitialize",
            json!({"sessionId":SESSION,"generation":generation+1}),
        )
        .await;
    assert_eq!(entry.lifecycle.snapshot().status, LspSessionStatus::Crashed);
    fixture
        .ok(
            "lsp_report_reinitialize_failure",
            json!({"sessionId":SESSION,"generation":generation}),
        )
        .await;
    assert!(entry.lifecycle.snapshot().last_error.is_some());
    fixture
        .ok(
            "lsp_confirm_reinitialize",
            json!({"sessionId":SESSION,"generation":generation}),
        )
        .await;
    assert_eq!(entry.lifecycle.snapshot().status, LspSessionStatus::Running);
    assert!(entry.lifecycle.snapshot().last_error.is_none());
    let install = fixture.services.lsp_install.begin(&SERVER.into()).unwrap();
    fixture
        .ok("lsp_install_cancel", json!({"serverId":SERVER}))
        .await;
    assert!(install.cancellation_token().load(Ordering::Acquire));
    assert!(fixture.services.lsp_install.begin(&SERVER.into()).is_none());
    drop(install);
    assert!(fixture.services.lsp_install.begin(&SERVER.into()).is_some());
    std::fs::write(fixture.root.join("go.mod"), "module synthetic\n").unwrap();
    let file = second.join("synthetic.go");
    std::fs::write(&file, "package synthetic\n").unwrap();
    assert_eq!(
        fixture
            .ok(
                "lsp_resolve_root",
                json!({"serverId":SERVER,"filePath":file})
            )
            .await,
        json!(fixture.root)
    );
    fixture
        .ok(
            "lsp_stop",
            json!({"sessionId":SESSION,"root":second,"owner":OTHER_OWNER}),
        )
        .await;
    assert_eq!(
        entry.roots.paths(),
        vec![fixture.root.to_str().unwrap().to_string()]
    );
    assert!(entry.subscribers.contains("remote"));
    assert!(fixture.services.lsp.contains(SESSION));
    drop(receiver);
    entry.subscribers.broadcast("synthetic dropped recipient");
    assert!(!entry.subscribers.contains("remote"));
    fixture
        .ok(
            "lsp_stop",
            json!({"sessionId":SESSION,"root":null,"owner":OTHER_OWNER}),
        )
        .await;
    assert!(!fixture.services.lsp.contains(SESSION));
    assert_eq!(entry.lifecycle.snapshot().status, LspSessionStatus::Stopped);
    fixture.finish().await;
    assert_eq!(
        fixture
            .ok("lsp_sessions", json!({"projectId":fixture.project}))
            .await,
        json!([])
    );
    fixture
        .ok("lsp_install_cancel", json!({"serverId":SERVER}))
        .await;
}

#[tokio::test]
async fn 실제_합성프로세스_spawn_channel_send_restart_stop와_완료회수를_보존한다() {
    let fixture = Fixture::new(true);
    let (sender, mut receiver) = mpsc::channel(RESPONSE_CAPACITY);
    let id = fixture
        .call(
            "lsp_spawn",
            json!({"request":fixture.request(&fixture.root),"onMessage":"3"}),
            channel("3", sender),
        )
        .await
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    assert!(id.starts_with("lsp-"));
    let entry = fixture.services.lsp.get(&id).unwrap();
    let first = entry.proc.lock().clone().unwrap();
    assert!(first.pid().is_some());
    assert!(entry.subscribers.contains("remote"));
    fixture.ok("lsp_send", json!({"sessionId":id,"message":json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}).to_string()})).await;
    let response = tokio::time::timeout(DEADLINE, receiver.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(serde_json::from_str::<Value>(&response).unwrap()["id"], 1);
    fixture.ok("lsp_restart", json!({"sessionId":id})).await;
    tokio::time::timeout(DEADLINE, first.wait_for_completion())
        .await
        .unwrap();
    assert!(first.is_exited());
    assert!(first.is_finished());
    let second = entry.proc.lock().clone().unwrap();
    assert!(!Arc::ptr_eq(&first, &second));
    assert_eq!(fixture.processes.load(Ordering::Acquire), 2);
    assert_eq!(fixture.exits.load(Ordering::Acquire), 1);
    assert_eq!(entry.lifecycle.snapshot().status, LspSessionStatus::Running);
    fixture.ok("lsp_send", json!({"sessionId":id,"message":json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}).to_string()})).await;
    assert!(
        tokio::time::timeout(DEADLINE, receiver.recv())
            .await
            .unwrap()
            .is_some()
    );
    fixture
        .ok(
            "lsp_stop",
            json!({"sessionId":id,"root":null,"owner":OTHER_OWNER}),
        )
        .await;
    tokio::time::timeout(DEADLINE, second.wait_for_completion())
        .await
        .unwrap();
    assert!(second.is_exited());
    assert!(second.is_finished());
    assert_eq!(fixture.exits.load(Ordering::Acquire), 2);
    assert!(!fixture.services.lsp.contains(&id));
    assert_eq!(
        fixture
            .sink
            .0
            .lock()
            .unwrap()
            .iter()
            .map(|event| match event {
                AppEvent::LspSessionStatusChanged { status, .. } => *status,
                _ => panic!("unexpected LSP event"),
            })
            .collect::<Vec<_>>(),
        vec![
            LspSessionStatus::Running,
            LspSessionStatus::Stopped,
            LspSessionStatus::Starting,
            LspSessionStatus::Running,
            LspSessionStatus::Stopped
        ]
    );
    fixture.finish().await;
}

#[tokio::test]
async fn 실제_detect는_주입_path의_합성파일과_manifest_옵션을_직렬화한다() {
    let fixture = Fixture::new(false);
    let specs = manifest::servers();
    for spec in &specs {
        let bin = std::path::Path::new(spec.command.bin());
        assert!(
            bin.components()
                .all(|component| matches!(component, std::path::Component::Normal(_)))
        );
        let path = fixture.bin.join(bin);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"synthetic detection file, never executed").unwrap();
    }
    let detected = fixture.ok("lsp_detect_servers", json!({})).await;
    let rows = detected.as_array().unwrap();
    assert_eq!(rows.len(), specs.len());
    let gopls = rows.iter().find(|row| row["id"] == SERVER).unwrap();
    assert_eq!(gopls["available"], true);
    assert_eq!(gopls["resolvedPath"], json!(fixture.bin.join(SERVER)));
    assert_eq!(
        gopls["initializationOptions"],
        json!(manifest::find_spec(SERVER).unwrap().initialization_options)
    );
    assert_eq!(fixture.processes.load(Ordering::Acquire), 0);
    assert!(fixture.sink.0.lock().unwrap().is_empty());
    fixture.finish().await;
}

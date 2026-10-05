use std::collections::BTreeSet;
use std::future::{Future, poll_fn};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};
use std::task::Poll;
use std::time::Duration;

use serde_json::json;
use taide_model::app_event::AppEvent;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_model::task::{Task, TaskSource};
use taide_remote::command_policy::REMOTE_ALLOWED_COMMANDS;
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::Notify;

use super::*;
use crate::remote_gateway;
use crate::remote_ws::ChannelFactory;

const COMMAND_COUNT: usize = 4;
const ROOT_PID: u32 = 4000;
const CHILD_PID: u32 = ROOT_PID + 1;
const GRANDCHILD_PID: u32 = CHILD_PID + 1;
const UNRELATED_PID: u32 = GRANDCHILD_PID + 1;
const CPU_COUNT: usize = 4;
const CPU_RAW: f32 = 200.0;
const CPU_EXPECTED: f64 = 50.0;
const MEMORY: u64 = 1024;
const TASK_COUNT: usize = 9;
const DEADLINE: Duration = Duration::from_secs(3);

struct Sink;

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        panic!("utility action unexpectedly published {event:?}");
    }
}

#[derive(Clone)]
struct Gate(Arc<(Mutex<bool>, Condvar)>);

impl Gate {
    fn new(open: bool) -> Self {
        Self(Arc::new((Mutex::new(open), Condvar::new())))
    }

    fn wait(&self) {
        let (lock, changed) = self.0.as_ref();
        let mut open = lock.lock().unwrap();
        while !*open {
            open = changed.wait(open).unwrap();
        }
    }

    fn open(&self) {
        let (lock, changed) = self.0.as_ref();
        *lock.lock().unwrap() = true;
        changed.notify_all();
    }
}

struct Release(Gate);

impl Drop for Release {
    fn drop(&mut self) {
        self.0.open();
    }
}

struct Samples {
    gate: Gate,
    started: Notify,
    calls: AtomicUsize,
    trace: Mutex<Vec<&'static str>>,
}

impl Samples {
    fn read(&self, name: &'static str) {
        self.trace.lock().unwrap().push(name);
        self.calls.fetch_add(1, Ordering::AcqRel);
        self.started.notify_one();
        self.gate.wait();
    }
}

impl UsageProvider for Samples {
    fn collect_app_usage(&self) -> AppResult<SystemUsage> {
        self.read("app");
        Ok(SystemUsage {
            cpu_percent: None,
            memory_bytes: MEMORY as f64,
        })
    }

    fn refresh_process_records(&self) -> Vec<ProcessRecord> {
        self.read("records");
        vec![
            record(ROOT_PID, None, "synthetic-app", MEMORY, false),
            record(CHILD_PID, Some(ROOT_PID), "bash", MEMORY * 4, true),
            record(GRANDCHILD_PID, Some(CHILD_PID), "gopls", MEMORY * 2, true),
            record(UNRELATED_PID, None, "unrelated", MEMORY * 8, true),
        ]
    }
}

fn record(
    pid: u32,
    parent_pid: Option<u32>,
    name: &str,
    memory: u64,
    sampled: bool,
) -> ProcessRecord {
    ProcessRecord {
        pid,
        parent_pid,
        name: name.into(),
        cpu_usage: CPU_RAW,
        memory,
        has_previous_cpu_sample: sampled,
    }
}

struct Fixture {
    directory: PathBuf,
    root: PathBuf,
    project: ProjectId,
    services: Arc<AppServices>,
    samples: Arc<Samples>,
}

impl Fixture {
    fn new(blocked: bool) -> Self {
        let project = ProjectId::new();
        let directory =
            std::env::temp_dir().join(format!("taide-native-remote-utilities-{project}"));
        std::fs::create_dir_all(directory.join("project")).unwrap();
        let directory = directory.canonicalize().unwrap();
        let root = directory.join("project");
        let state = AppState::new(AppPaths::new(directory.join("data")));
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.to_str().unwrap().into(),
                name: "synthetic utility".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        let services = crate::bootstrap::services(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            Arc::new(Sink),
        );
        let samples = Arc::new(Samples {
            gate: Gate::new(!blocked),
            started: Notify::new(),
            calls: AtomicUsize::new(0),
            trace: Mutex::new(Vec::new()),
        });
        Self {
            directory,
            root,
            project,
            services,
            samples,
        }
    }

    fn ports(&self) -> Ports {
        let fonts = self.samples.clone();
        let root = self.samples.clone();
        let cpu = self.samples.clone();
        let providers = [
            (SystemUsageProcessKind::Terminal, "terminal"),
            (SystemUsageProcessKind::Agent, "agent"),
            (SystemUsageProcessKind::Lsp, "lsp"),
        ]
        .into_iter()
        .map(|(kind, name)| {
            let samples = self.samples.clone();
            let provider: LabelProvider = Arc::new(move |_| {
                samples.trace.lock().unwrap().push(name);
                HashMap::from([(CHILD_PID, (kind, name.into()))])
            });
            provider
        })
        .collect();
        Ports {
            fonts: Arc::new(move || {
                fonts.read("fonts");
                vec![
                    FontFamily {
                        name: "Synthetic Mono".into(),
                        monospaced: true,
                    },
                    FontFamily {
                        name: "Synthetic Serif".into(),
                        monospaced: false,
                    },
                ]
            }),
            usage: self.samples.clone(),
            root_pid: Arc::new(move || {
                root.trace.lock().unwrap().push("root");
                Ok(ROOT_PID)
            }),
            cpu_count: Arc::new(move || {
                cpu.trace.lock().unwrap().push("cpu");
                CPU_COUNT
            }),
            label_providers: providers,
        }
    }

    fn backend(&self, ports: Ports) -> Dispatch {
        remote_gateway::with_policy(extend_backend(
            ports,
            Dispatch {
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
            },
        ))
    }

    async fn call(&self, backend: &Dispatch, name: &str, args: Value) -> Result<Value, Value> {
        let channels: ChannelFactory = Arc::new(|_| panic!("utilities do not create channels"));
        let result = (backend.json)(self.services.clone(), name.into(), args, channels).await?;
        Ok(serde_json::from_str(&result).unwrap())
    }

    async fn finish(&self) {
        self.samples.gate.open();
        tokio::time::timeout(DEADLINE, self.services.tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(self.services.tasks.tracked_count(), 0);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.samples.gate.open();
        self.services.tasks.stop_all();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[tokio::test]
async fn catalog_정책_remaining과_실제_합성_task_발견은_명령을_실행하지_않는다() {
    let declared: BTreeSet<_> = COMMANDS.iter().copied().collect();
    assert_eq!(declared.len(), COMMAND_COUNT);
    assert!(
        declared
            .iter()
            .all(|name| REMOTE_ALLOWED_COMMANDS.contains(name))
    );
    let previous: BTreeSet<_> = [
        crate::remote_preferences::COMMANDS,
        crate::remote_ide::COMMANDS,
        crate::remote_files::JSON_COMMANDS,
        crate::remote_layout::COMMANDS,
        crate::remote_projects::COMMANDS,
        crate::remote_search::COMMANDS,
        crate::remote_plugins::COMMANDS,
        crate::remote_git::COMMANDS,
        crate::remote_agents::COMMANDS,
        crate::remote_lsp::COMMANDS,
        crate::remote_terminal::COMMANDS,
    ]
    .into_iter()
    .flatten()
    .copied()
    .collect();
    assert!(declared.is_disjoint(&previous));
    let pattern = regex::Regex::new(r#"(?m)^        "([a-z_]+)" =>"#).unwrap();
    let arms: BTreeSet<_> = pattern
        .captures_iter(include_str!("remote-utilities.rs"))
        .map(|capture| capture.get(1).unwrap().as_str())
        .collect();
    assert_eq!(declared, arms);
    let fixture = Fixture::new(false);
    let backend = fixture.backend(fixture.ports());
    std::fs::write(
        fixture.root.join("package.json"),
        r#"{"scripts":{"build":"touch must-not-exist","test":"exit 91"}}"#,
    )
    .unwrap();
    std::fs::write(fixture.root.join("bun.lock"), "").unwrap();
    std::fs::write(
        fixture.root.join("GNUmakefile"),
        "serve:\n\ttouch must-not-exist\ncheck:\n\texit 91\n",
    )
    .unwrap();
    std::fs::write(
        fixture.root.join("Makefile"),
        "ignored:\n\ttouch must-not-exist\n",
    )
    .unwrap();
    std::fs::write(
        fixture.root.join("Cargo.toml"),
        "[package]\nname='synthetic'\n",
    )
    .unwrap();
    let tasks: Vec<Task> = serde_json::from_value(
        fixture
            .call(
                &backend,
                "detect_tasks",
                json!({"projectId":fixture.project}),
            )
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(tasks.len(), TASK_COUNT);
    assert_eq!(tasks[0].command, "bun run 'build'");
    assert_eq!(tasks[1].command, "bun run 'test'");
    assert_eq!(tasks[0].source, TaskSource::Npm);
    assert_eq!(tasks[2].command, "make 'serve'");
    assert_eq!(tasks[3].command, "make 'check'");
    assert_eq!(tasks[2].source, TaskSource::Make);
    assert_eq!(tasks.last().unwrap().command, "cargo clippy");
    assert!(
        tasks
            .iter()
            .all(|task| task.cwd == fixture.root.to_str().unwrap())
    );
    assert!(!fixture.root.join("must-not-exist").exists());
    assert_eq!(
        fixture
            .call(&backend, "detect_tasks", json!({}))
            .await
            .unwrap_err()["code"],
        "InvalidArgument"
    );
    assert_eq!(
        fixture
            .call(
                &backend,
                "detect_tasks",
                json!({"projectId":ProjectId::new()})
            )
            .await
            .unwrap_err()["code"],
        "NotFound"
    );
    assert_eq!(
        fixture
            .call(&backend, "settings_get", json!({"owner":"main"}))
            .await
            .unwrap()["owner"],
        "remote"
    );
    assert_eq!(
        (backend.raw)(
            fixture.services.clone(),
            "file_read_raw".into(),
            Value::Null
        )
        .await
        .unwrap(),
        vec![0]
    );
    assert_eq!(
        fixture
            .call(&backend, "system_open_path", json!({"path":fixture.root}))
            .await
            .unwrap_err()["message"]["kind"],
        "Forbidden"
    );
    assert_eq!(fixture.samples.calls.load(Ordering::Acquire), 0);
    fixture.finish().await;
}

#[tokio::test]
async fn font와_usage_wire_첫_sample_null_분류_라벨_순서와_shutdown_정책을_보존한다() {
    let fixture = Fixture::new(false);
    let backend = fixture.backend(fixture.ports());
    fixture.services.state.begin_shutdown();
    let fonts = fixture
        .call(&backend, "font_list", json!({}))
        .await
        .unwrap();
    assert_eq!(
        fonts,
        json!([
            {"name":"Synthetic Mono","monospaced":true},
            {"name":"Synthetic Serif","monospaced":false},
        ])
    );
    let app = fixture
        .call(&backend, "system_usage_get", json!({}))
        .await
        .unwrap();
    assert_eq!(app, json!({"cpuPercent":null,"memoryBytes":MEMORY as f64}));
    fixture.samples.trace.lock().unwrap().clear();
    let rows = fixture
        .call(&backend, "system_usage_breakdown", json!({}))
        .await
        .unwrap();
    assert_eq!(
        *fixture.samples.trace.lock().unwrap(),
        ["root", "terminal", "agent", "lsp", "cpu", "records"]
    );
    assert_eq!(rows.as_array().unwrap().len(), 3);
    assert_eq!(
        rows[0],
        json!({"pid":CHILD_PID,"kind":"lsp","label":"lsp",
        "cpuPercent":CPU_EXPECTED,"memoryBytes":(MEMORY * 4) as f64})
    );
    assert_eq!(rows[1]["kind"], "lsp");
    assert_eq!(rows[1]["label"], "gopls");
    assert_eq!(rows[2]["kind"], "app");
    assert_eq!(rows[2]["label"], "TAIDE");
    assert_eq!(rows[2]["cpuPercent"], Value::Null);
    assert!(
        !rows
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["pid"] == UNRELATED_PID)
    );
    fixture.services.tasks.stop_all();
    let calls = fixture.samples.calls.load(Ordering::Acquire);
    for name in ["font_list", "system_usage_get", "system_usage_breakdown"] {
        assert_eq!(
            fixture.call(&backend, name, json!({})).await.unwrap_err()["code"],
            "Forbidden"
        );
    }
    assert_eq!(fixture.samples.calls.load(Ordering::Acquire), calls);
    fixture.finish().await;
}

#[tokio::test]
async fn font와_usage의_취소한_waiter_뒤에도_시작된_blocking_worker를_회수한다() {
    for name in ["font_list", "system_usage_get", "system_usage_breakdown"] {
        let fixture = Fixture::new(true);
        let _release = Release(fixture.samples.gate.clone());
        let backend = fixture.backend(fixture.ports());
        let mut request = Box::pin(fixture.call(&backend, name, json!({})));
        tokio::time::timeout(DEADLINE, async {
            tokio::select! {
                result = &mut request => panic!("blocking sample must wait: {result:?}"),
                () = fixture.samples.started.notified() => {}
            }
        })
        .await
        .unwrap();
        drop(request);
        assert_eq!(fixture.services.tasks.tracked_count(), 1);
        let mut shutdown = Box::pin(fixture.services.tasks.shutdown());
        assert!(poll_fn(|context| Poll::Ready(shutdown.as_mut().poll(context).is_pending())).await);
        assert_eq!(fixture.services.tasks.tracked_count(), 1);
        fixture.samples.gate.open();
        tokio::time::timeout(DEADLINE, shutdown).await.unwrap();
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
        assert_eq!(fixture.samples.calls.load(Ordering::Acquire), 1);
    }
}

#[tokio::test]
async fn pid_실패와_sample_panic_오류를_보존하고_미사용_os_provider를_실행하지_않는다() {
    let fixture = Fixture::new(false);
    let mut ports = fixture.ports();
    ports.root_pid = Arc::new(|| Err(AppError::Internal("synthetic pid failure".into())));
    let backend = fixture.backend(ports);
    let error = fixture
        .call(&backend, "system_usage_breakdown", json!({}))
        .await
        .unwrap_err();
    assert_eq!(error["code"], "Internal");
    assert_eq!(error["message"], "synthetic pid failure");
    assert!(fixture.samples.trace.lock().unwrap().is_empty());
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
    let mut ports = fixture.ports();
    ports.fonts = Arc::new(|| panic!("synthetic scan panic"));
    let backend = fixture.backend(ports);
    assert_eq!(
        fixture
            .call(&backend, "font_list", json!({}))
            .await
            .unwrap_err()["code"],
        "Internal"
    );
    let production = Ports::new(&fixture.services, domain_label_providers());
    assert_eq!(production.label_providers.len(), 3);
    for provider in &production.label_providers {
        assert!(provider(&fixture.services).is_empty());
    }
    assert_eq!(fixture.samples.calls.load(Ordering::Acquire), 0);
    fixture.finish().await;
}

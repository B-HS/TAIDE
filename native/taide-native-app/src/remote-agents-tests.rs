use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::json;
use taide_agent::constants::{AGENT_NAME_CLAUDE, KNOWN_AGENT_NAMES, WAIT_MARKER_PREFIX};
use taide_agent::{hook_files, service};
use taide_model::agent::AgentActivity;
use taide_model::app_event::AppEvent;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_remote::command_policy::{self, REMOTE_ALLOWED_COMMANDS, RemoteDenialPolicy};
use taide_runtime::{AppState, EventSink, TaskSupervisor, agent_probe};

use super::*;
use crate::remote_gateway;
use crate::remote_ws::{ChannelFactory, ResponseBody};

const DEADLINE: Duration = Duration::from_secs(5);
const PID: u32 = 222;
const SESSION: &str = "synthetic-agent-session";
const OWN_CONTENT: &str = "synthetic marker";

#[tokio::test]
async fn production_port는_빈foreground와_probe_설정gate를_실제로_사용한다() {
    let fixture = Fixture::new();
    let ports = Ports::new();
    assert_eq!(ports.cli_target, crate::agent_hooks::TAIDE_CLI_TARGET_PATH);
    let pids = (ports.foreground_pids)(&fixture.services, &fixture.project);
    assert!(pids.is_empty());
    assert!(
        (ports.probe)(fixture.services.clone(), pids)
            .await
            .unwrap()
            .is_empty()
    );
    let listed = dispatch(
        fixture.services.clone(),
        &ports,
        "agent_list",
        &json!({"projectId":fixture.project}),
    )
    .await
    .unwrap();
    let value: Value = serde_json::from_str(&listed).unwrap();
    assert!(value["agents"].as_array().unwrap().is_empty());
    assert!(!fixture.services.state.settings.read().agent_hooks_enabled);
    assert!(
        dispatch(
            fixture.services.clone(),
            &ports,
            "agent_hooks_install",
            &json!({"projectId":fixture.project,"agentName":AGENT_NAME_CLAUDE})
        )
        .await
        .is_err()
    );
    assert!(fixture.services.agent_hooks.server_info().is_none());
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
    fixture.services.tasks.shutdown().await;
}

struct ProbeGate(Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>);

impl ProbeGate {
    fn release(&self) {
        *self.0.0.lock().unwrap() = true;
        self.0.1.notify_all();
    }
}

impl Drop for ProbeGate {
    fn drop(&mut self) {
        self.release();
    }
}

struct Sink;

impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {
        panic!("agent request commands do not emit events");
    }
}

struct Fixture {
    directory: PathBuf,
    root: PathBuf,
    home: PathBuf,
    marker: PathBuf,
    project: ProjectId,
    services: Arc<AppServices>,
    backend: Dispatch,
    foregrounds: Arc<AtomicUsize>,
    probes: Arc<AtomicUsize>,
    homes: Arc<AtomicUsize>,
    emitters: Arc<AtomicUsize>,
}

impl Fixture {
    fn new() -> Self {
        let project = ProjectId::new();
        let directory = std::env::temp_dir().join(format!("taide-native-remote-agent-{project}"));
        let root = directory.join("project");
        let home = directory.join("home");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        let root = root.canonicalize().unwrap();
        let home = home.canonicalize().unwrap();
        let marker = std::env::temp_dir().join(format!("{WAIT_MARKER_PREFIX}{project}"));
        let state = AppState::new(AppPaths::new(directory.join("data")));
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.to_str().unwrap().into(),
                name: "synthetic agent".into(),
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
        let foregrounds = Arc::new(AtomicUsize::new(0));
        let probes = Arc::new(AtomicUsize::new(0));
        let homes = Arc::new(AtomicUsize::new(0));
        let emitters = Arc::new(AtomicUsize::new(0));
        let foreground_count = foregrounds.clone();
        let probe_count = probes.clone();
        let home_count = homes.clone();
        let emitter_count = emitters.clone();
        let synthetic_home = home.clone();
        let cli = directory
            .join("synthetic-cli")
            .to_str()
            .unwrap()
            .to_string();
        let cli_target = cli.clone();
        let ports = Ports {
            foreground_pids: Arc::new(move |services, project| {
                assert!(services.state.projects.read().contains_key(project));
                foreground_count.fetch_add(1, Ordering::AcqRel);
                vec![(SESSION.into(), PID)]
            }),
            probe: Arc::new(move |services, pids| {
                let count = probe_count.clone();
                Box::pin(async move {
                    #[cfg(unix)]
                    {
                        agent_probe::probe_process_names(
                            &services.tasks,
                            &services.agents,
                            pids,
                            move |unresolved| {
                                count.fetch_add(1, Ordering::AcqRel);
                                assert_eq!(unresolved, vec![PID]);
                                [(
                                    PID,
                                    service::detect_agent_name("claude", "claude --synthetic"),
                                )]
                                .into_iter()
                                .collect()
                            },
                        )
                        .await
                    }
                    #[cfg(not(unix))]
                    {
                        agent_probe::probe_process_tree(&services.tasks, pids, move |pids| {
                            count.fetch_add(1, Ordering::AcqRel);
                            pids.into_iter()
                                .map(|(session_id, pid)| DetectedAgentProbe {
                                    session_id,
                                    pid,
                                    name: AGENT_NAME_CLAUDE,
                                })
                                .collect()
                        })
                        .await
                    }
                })
            }),
            cli_status: Arc::new(move || {
                service::build_cli_install_status(&cli, false, None, false)
            }),
            resolve_home: Arc::new(move || {
                home_count.fetch_add(1, Ordering::AcqRel);
                Some(synthetic_home.to_str().unwrap().into())
            }),
            project_emitter: Arc::new(move |_| {
                emitter_count.fetch_add(1, Ordering::AcqRel);
                Box::pin(async { HookEmitter::TerminalSequence })
            }),
            cli_target,
        };
        let remaining = Dispatch {
            json: Arc::new(|_, name, args, channels| {
                Box::pin(async move {
                    assert_eq!(name, "layout_get");
                    channels("1".into())(ResponseBody::Json("null".into())).unwrap();
                    Ok(args.to_string())
                })
            }),
            raw: Arc::new(|_, name, args| {
                Box::pin(async move {
                    assert_eq!(name, "file_read_raw");
                    assert_eq!(args["owner"], "remote");
                    Ok(vec![0])
                })
            }),
        };
        Self {
            directory,
            root,
            home,
            marker,
            project,
            services,
            backend: remote_gateway::with_policy(extend_backend(ports, remaining)),
            foregrounds,
            probes,
            homes,
            emitters,
        }
    }

    async fn call(&self, name: &str, mut args: Value) -> Result<Value, Value> {
        if args.is_object() && args.get("projectId").is_none() {
            args["projectId"] = json!(self.project);
        }
        let channels: ChannelFactory =
            Arc::new(|_| Box::new(|_| panic!("agent command used channel")));
        let text = (self.backend.json)(self.services.clone(), name.into(), args, channels).await?;
        Ok(serde_json::from_str(&text).unwrap())
    }

    async fn ok(&self, name: &str, args: Value) -> Value {
        self.call(name, args)
            .await
            .unwrap_or_else(|error| panic!("{name}: {error}"))
    }

    async fn finish(&self) {
        tokio::time::timeout(DEADLINE, self.services.tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(self.services.tasks.tracked_count(), 0);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.tasks.stop_all();
        if self.marker.exists() {
            std::fs::remove_file(&self.marker).unwrap();
        }
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[tokio::test]
async fn 목록6_actual_arm_정책_입력_remaining과_shutdown_원본오류를_보존한다() {
    let declared: BTreeSet<_> = COMMANDS.iter().copied().collect();
    assert_eq!(declared.len(), COMMANDS.len());
    assert_eq!(declared.len(), 6);
    assert!(
        declared
            .iter()
            .all(|name| REMOTE_ALLOWED_COMMANDS.contains(name))
    );
    let pattern = regex::Regex::new(r#"(?m)^        "([a-z_]+)" =>"#).unwrap();
    let arms: BTreeSet<_> = pattern
        .captures_iter(include_str!("remote-agents.rs"))
        .map(|capture| capture.get(1).unwrap().as_str())
        .collect();
    assert_eq!(declared, arms);
    for previous in [
        crate::remote_git::COMMANDS,
        crate::remote_preferences::COMMANDS,
        crate::remote_ide::COMMANDS,
        crate::remote_files::JSON_COMMANDS,
        crate::remote_layout::COMMANDS,
        crate::remote_projects::COMMANDS,
        crate::remote_search::COMMANDS,
        crate::remote_plugins::COMMANDS,
    ] {
        assert!(declared.iter().all(|name| !previous.contains(name)));
    }
    let fixture = Fixture::new();
    for name in [
        "agent_cli_install",
        "agent_cli_uninstall",
        "agent_pending_external_opens",
        "synthetic_unknown",
    ] {
        assert_eq!(
            fixture.call(name, json!({})).await.unwrap_err(),
            error_value(command_policy::admit(name).unwrap_err())
        );
    }
    assert_eq!(
        fixture
            .call("agent_list", json!({"projectId":false}))
            .await
            .unwrap_err()["code"],
        "InvalidArgument"
    );
    assert_eq!(
        fixture
            .call("agent_list", json!({"projectId":ProjectId::new()}))
            .await
            .unwrap_err()["code"],
        "NotFound"
    );
    assert_eq!(fixture.foregrounds.load(Ordering::Acquire), 0);
    assert_eq!(fixture.probes.load(Ordering::Acquire), 0);
    let channel_count = Arc::new(AtomicUsize::new(0));
    let count = channel_count.clone();
    let channels: ChannelFactory = Arc::new(move |id| {
        assert_eq!(id, "1");
        let count = count.clone();
        Box::new(move |body| {
            assert!(matches!(body, ResponseBody::Json(_)));
            count.fetch_add(1, Ordering::AcqRel);
            Ok(())
        })
    });
    let result = (fixture.backend.json)(
        fixture.services.clone(),
        "layout_get".into(),
        json!({"owner":"main"}),
        channels,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&result).unwrap()["owner"],
        "remote"
    );
    assert_eq!(channel_count.load(Ordering::Acquire), 1);
    assert_eq!(
        (fixture.backend.raw)(
            fixture.services.clone(),
            "file_read_raw".into(),
            json!({"owner":"main"})
        )
        .await
        .unwrap(),
        vec![0]
    );
    fixture.finish().await;
    assert_eq!(
        fixture.call("agent_list", json!({})).await.unwrap_err()["code"],
        "Forbidden"
    );
    assert_eq!(
        fixture
            .call("agent_release_marker", json!({"marker":fixture.marker}))
            .await
            .unwrap_err()["code"],
        "Forbidden"
    );
    for name in [
        "agent_hooks_status",
        "agent_hooks_install",
        "agent_hooks_uninstall",
    ] {
        assert_eq!(
            fixture
                .call(name, json!({"agentName":AGENT_NAME_CLAUDE}))
                .await
                .unwrap_err()["code"],
            "Internal"
        );
    }
    assert_eq!(
        fixture.ok("agent_cli_status", json!({})).await["installed"],
        false
    );
    assert_eq!(fixture.emitters.load(Ordering::Acquire), 0);
    assert_eq!(fixture.homes.load(Ordering::Acquire), 0);
}

#[tokio::test]
async fn 실제_probe_cache_활동과_marker_검증_회수를_보존한다() {
    let fixture = Fixture::new();
    let first = fixture.ok("agent_list", json!({})).await;
    assert_eq!(first["projectId"], json!(fixture.project));
    assert_eq!(first["agents"][0]["sessionId"], SESSION);
    assert_eq!(first["agents"][0]["name"], AGENT_NAME_CLAUDE);
    assert_eq!(first["agents"][0]["pid"], PID);
    assert_eq!(first["agents"][0]["activity"], "unknown");
    fixture.services.agents.record_input(SESSION);
    assert_eq!(
        fixture.ok("agent_list", json!({})).await["agents"][0]["activity"],
        "unknown"
    );
    fixture.services.agents.record_scan_parts_at(
        SESSION,
        std::iter::empty(),
        "Do you want to proceed?",
        "",
        std::time::Instant::now(),
    );
    fixture.services.agent_hooks.set_project_override(
        fixture.project.clone(),
        AGENT_NAME_CLAUDE.into(),
        AgentActivity::Working,
    );
    assert_eq!(
        fixture.ok("agent_list", json!({})).await["agents"][0]["activity"],
        "awaitingInput"
    );
    assert_eq!(
        fixture.ok("agent_list", json!({})).await["agents"][0]["blockedReason"],
        "dialog"
    );
    #[cfg(unix)]
    assert_eq!(fixture.probes.load(Ordering::Acquire), 1);
    assert_eq!(fixture.foregrounds.load(Ordering::Acquire), 4);
    let marker = fixture.marker.to_str().unwrap();
    std::fs::write(&fixture.marker, OWN_CONTENT).unwrap();
    fixture.services.agents.register_wait_marker(marker.into());
    fixture
        .ok("agent_release_marker", json!({"marker":marker}))
        .await;
    assert!(!fixture.marker.exists());
    assert!(fixture.services.agents.take_all_markers().is_empty());
    fixture
        .ok("agent_release_marker", json!({"marker":marker}))
        .await;
    let invalid = fixture.root.join("not-a-marker");
    std::fs::write(&invalid, OWN_CONTENT).unwrap();
    assert_eq!(
        fixture
            .call("agent_release_marker", json!({"marker":invalid}))
            .await
            .unwrap_err()["code"],
        "InvalidArgument"
    );
    assert_eq!(std::fs::read_to_string(invalid).unwrap(), OWN_CONTENT);
    fixture.finish().await;
}

#[tokio::test]
async fn 실제_프로젝트_hook_install과_합성_home_status_uninstall_소유권을_보존한다() {
    let fixture = Fixture::new();
    let root = fixture.root.to_str().unwrap();
    let foreign = json!({"synthetic":true,"hooks":{"Stop":[{"hooks":[{"type":"command","command":"synthetic-foreign-command"}]}]}});
    hook_files::write_settings_local(root, &foreign).unwrap();
    assert_eq!(
        fixture
            .ok("agent_hooks_status", json!({"agentName":AGENT_NAME_CLAUDE}))
            .await["installed"],
        false
    );
    fixture.services.state.settings.write().agent_hooks_enabled = false;
    assert_eq!(
        fixture
            .call(
                "agent_hooks_install",
                json!({"agentName":AGENT_NAME_CLAUDE})
            )
            .await
            .unwrap_err()["code"],
        "InvalidArgument"
    );
    assert_eq!(fixture.emitters.load(Ordering::Acquire), 0);
    fixture.services.state.settings.write().agent_hooks_enabled = true;
    assert_eq!(
        fixture
            .ok(
                "agent_hooks_install",
                json!({"agentName":AGENT_NAME_CLAUDE})
            )
            .await["installed"],
        true
    );
    let installed = hook_files::read_settings_local(root).unwrap();
    assert!(service::agent_hook_entries_match(
        AGENT_NAME_CLAUDE,
        &installed,
        HookEmitter::TerminalSequence
    ));
    assert_eq!(installed["synthetic"], true);
    assert_eq!(fixture.emitters.load(Ordering::Acquire), 1);
    fixture
        .ok(
            "agent_hooks_uninstall",
            json!({"agentName":AGENT_NAME_CLAUDE}),
        )
        .await;
    assert_eq!(hook_files::read_settings_local(root).unwrap(), foreign);
    assert_eq!(fixture.homes.load(Ordering::Acquire), 0);
    for agent in KNOWN_AGENT_NAMES
        .iter()
        .copied()
        .filter(|name| *name != AGENT_NAME_CLAUDE)
    {
        assert_eq!(
            fixture
                .call("agent_hooks_install", json!({"agentName":agent}))
                .await
                .unwrap_err(),
            error_value(
                RemoteDenialPolicy::DesktopCliInterception.denial_error("agent_hooks_install")
            )
        );
        let path = service::user_level_hooks_path(agent, fixture.home.to_str()).unwrap();
        let is_owned_file =
            service::hook_install_shape(agent).unwrap() == service::HookInstallShape::OwnedFile;
        if is_owned_file {
            hook_files::write_owned_hook_file(
                &path,
                &service::build_owned_hook_file_source(agent).unwrap(),
            )
            .unwrap();
        } else {
            let value = if service::requires_taide_cli(agent) {
                let url = service::build_hook_url(
                    &taide_agent::store::HooksServerInfo {
                        port: 1,
                        token: "synthetic-unserved-hook".into(),
                    },
                    agent,
                );
                let command = service::build_command_hook_shell_command("synthetic-cli", &url);
                service::inject_taide_command_hook_entries(
                    foreign.clone(),
                    service::managed_hook_events_for(agent),
                    &command,
                    service::user_level_hook_command_timeout(agent),
                )
            } else {
                service::inject_taide_agent_hook_entries(
                    agent,
                    foreign.clone(),
                    HookEmitter::DevTty,
                )
            };
            hook_files::write_user_level_hooks(&path, &value).unwrap();
        }
        let status = fixture
            .ok("agent_hooks_status", json!({"agentName":agent}))
            .await;
        assert_eq!(status["installed"], true);
        assert_eq!(status["scope"], "user");
        fixture
            .ok("agent_hooks_uninstall", json!({"agentName":agent}))
            .await;
        if is_owned_file {
            assert!(!path.exists());
            hook_files::write_owned_hook_file(&path, OWN_CONTENT).unwrap();
            fixture
                .ok("agent_hooks_uninstall", json!({"agentName":agent}))
                .await;
            assert_eq!(std::fs::read_to_string(&path).unwrap(), OWN_CONTENT);
        } else {
            assert_eq!(hook_files::read_user_level_hooks(&path).unwrap(), foreign);
        }
        assert_eq!(
            fixture
                .ok("agent_hooks_status", json!({"agentName":agent}))
                .await["installed"],
            false
        );
    }
    assert_eq!(fixture.emitters.load(Ordering::Acquire), 1);
    assert_eq!(
        fixture
            .call(
                "agent_hooks_status",
                json!({"agentName":"synthetic-unknown"})
            )
            .await
            .unwrap_err()["code"],
        "InvalidArgument"
    );
    let path = hook_files::settings_local_path(root);
    std::fs::write(&path, "invalid-json").unwrap();
    assert!(
        fixture
            .call(
                "agent_hooks_install",
                json!({"agentName":AGENT_NAME_CLAUDE})
            )
            .await
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), "invalid-json");
    fixture.finish().await;
}

#[tokio::test]
async fn 요청취소와_감독종료가_실행중인_실제_probe_worker를_조기회수하지_않는다() {
    let fixture = Fixture::new();
    let gate = ProbeGate(Arc::new((
        std::sync::Mutex::new(false),
        std::sync::Condvar::new(),
    )));
    let started = Arc::new(tokio::sync::Notify::new());
    let finished = Arc::new(AtomicUsize::new(0));
    let worker_gate = gate.0.clone();
    let worker_started = started.clone();
    let worker_finished = finished.clone();
    let ports = Ports {
        foreground_pids: Arc::new(|_, _| vec![(SESSION.into(), PID)]),
        probe: Arc::new(move |services, pids| {
            let gate = worker_gate.clone();
            let started = worker_started.clone();
            let finished = worker_finished.clone();
            Box::pin(async move {
                agent_probe::probe_process_tree(&services.tasks, pids, move |pids| {
                    started.notify_one();
                    let unlocked = gate.0.lock().unwrap();
                    let (unlocked, timeout) = gate
                        .1
                        .wait_timeout_while(unlocked, DEADLINE, |unlocked| !*unlocked)
                        .unwrap();
                    assert!(*unlocked && !timeout.timed_out());
                    finished.fetch_add(1, Ordering::AcqRel);
                    pids.into_iter()
                        .map(|(session_id, pid)| DetectedAgentProbe {
                            session_id,
                            pid,
                            name: AGENT_NAME_CLAUDE,
                        })
                        .collect()
                })
                .await
            })
        }),
        cli_status: Arc::new(|| panic!("unexpected CLI probe")),
        resolve_home: Arc::new(|| panic!("unexpected home access")),
        project_emitter: Arc::new(|_| panic!("unexpected emitter probe")),
        cli_target: fixture
            .directory
            .join("synthetic-cli")
            .to_str()
            .unwrap()
            .into(),
    };
    let backend = remote_gateway::with_policy(extend_backend(
        ports,
        Dispatch {
            json: Arc::new(|_, _, _, _| panic!("unexpected remaining JSON")),
            raw: Arc::new(|_, _, _| panic!("unexpected remaining raw")),
        },
    ));
    let services = fixture.services.clone();
    let project = fixture.project.clone();
    let waiter = tokio::spawn(async move {
        let channels: ChannelFactory = Arc::new(|_| panic!("unexpected agent channel"));
        (backend.json)(
            services,
            "agent_list".into(),
            json!({"projectId":project}),
            channels,
        )
        .await
    });
    tokio::time::timeout(DEADLINE, started.notified())
        .await
        .unwrap();
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    fixture.services.tasks.stop_all();
    assert_eq!(finished.load(Ordering::Acquire), 0);
    assert!(fixture.services.tasks.tracked_count() > 0);
    let services = fixture.services.clone();
    let shutdown = tokio::spawn(async move { services.tasks.shutdown().await });
    tokio::task::yield_now().await;
    assert!(!shutdown.is_finished());
    gate.release();
    tokio::time::timeout(DEADLINE, shutdown)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(finished.load(Ordering::Acquire), 1);
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
    assert!(
        fixture
            .services
            .agents
            .agents_for(&fixture.project)
            .is_empty()
    );
}

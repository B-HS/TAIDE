use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use taide_agent::constants::{AGENT_NAME_CLAUDE, AGENT_NAME_CODEX, AGENT_NAME_GEMINI, GEMINI_MANAGED_HOOK_EVENTS};
use taide_agent::store::HooksServerInfo;
use taide_agent::{hook_files, service};
use taide_lsp::install::LspInstallStore;
use taide_lsp::store::LspStore;
use taide_model::error::AppError;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::{Project, ProjectDisplay};
use taide_runtime::agent_hook_reconcile::{
    apply_agent_hooks_toggle, reconcile_installed_hooks, uninstall_hooks_from_open_projects, AgentHookReconcilePorts,
};
use taide_runtime::{AppState, ExitDrain, TaskSupervisor};
use taide_terminal::store::TerminalStore;
use tokio::sync::oneshot;
use uuid::Uuid;

const FIXTURE_TIMEOUT_MS: u64 = 2_000;
const PENDING_PROBE_MS: u64 = 20;
const FIXTURE_PORT: u16 = 12345;
const CLI_PATH: &str = "/fixture/taide";

struct Fixture {
    base: PathBuf,
    state: AppState,
    root: String,
    home: String,
}

impl Fixture {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!("taide-hook-reconcile-{}", Uuid::new_v4()));
        let state = AppState::new(AppPaths::new(base.join("data")));
        state.settings.write().agent_hooks_enabled = true;
        let root = base.join("project").to_string_lossy().into_owned();
        let home = base.join("home").to_string_lossy().into_owned();
        let project = Project {
            id: ProjectId::new(),
            root: root.clone(),
            name: "fixture".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: ProjectDisplay::default(),
        };
        state.projects.write().insert(project.id.clone(), project);
        Self { base, state, root, home }
    }

    fn install_project(&self) -> serde_json::Value {
        let value = service::inject_taide_agent_hook_entries(
            AGENT_NAME_CLAUDE,
            serde_json::json!({"user": "snapshot"}),
            service::HookEmitter::DevTty,
        );
        hook_files::write_settings_local(&self.root, &value).unwrap();
        value
    }

    fn install_http(&self, agent: &str) -> PathBuf {
        let path = service::user_level_hooks_path(agent, Some(&self.home)).unwrap();
        let value = service::inject_taide_command_hook_entries(
            serde_json::json!({"user": "keep"}),
            service::managed_hook_events_for(agent),
            "taide hook --url http://127.0.0.1:1/claude/hook?token=fixture&taide=1",
            service::user_level_hook_command_timeout(agent),
        );
        hook_files::write_user_level_hooks(&path, &value).unwrap();
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.base.exists() {
            std::fs::remove_dir_all(&self.base).expect("자기 UUID fixture만 정리");
        }
    }
}

fn server() -> HooksServerInfo {
    HooksServerInfo {
        port: FIXTURE_PORT,
        token: "fixture-token".to_string(),
    }
}
fn deadline() -> Duration {
    Duration::from_millis(FIXTURE_TIMEOUT_MS)
}

#[tokio::test]
async fn disabled_동일_toggle_닫힌_감독자는_lazy_port와_파일을_건드리지_않는다() {
    let fixture = Fixture::new();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    fixture.state.settings.write().agent_hooks_enabled = false;
    reconcile_installed_hooks(
        &fixture.state,
        &tasks,
        AgentHookReconcilePorts::new(
            || panic!("disabled home"),
            || async { panic!("disabled emitter") },
            || async { panic!("disabled server") },
            CLI_PATH,
        ),
    )
    .await;
    apply_agent_hooks_toggle(
        &fixture.state,
        &tasks,
        true,
        true,
        AgentHookReconcilePorts::new(
            || panic!("unchanged home"),
            || async { panic!("unchanged emitter") },
            || async { panic!("unchanged server") },
            CLI_PATH,
        ),
        || panic!("unchanged stop"),
    )
    .await;
    fixture.state.settings.write().agent_hooks_enabled = true;
    tasks.stop_all();
    uninstall_hooks_from_open_projects(&fixture.state, &tasks, || panic!("closed uninstall home")).await;
    reconcile_installed_hooks(
        &fixture.state,
        &tasks,
        AgentHookReconcilePorts::new(
            || panic!("closed home"),
            || async { panic!("closed emitter") },
            || async { panic!("closed server") },
            CLI_PATH,
        ),
    )
    .await;
    assert!(!fixture.base.exists());
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 설치한_프로젝트가_없으면_emitter는_건너뛰고_home과_server_순서를_유지한다() {
    let fixture = Fixture::new();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let read_home = AtomicBool::new(false);
    reconcile_installed_hooks(
        &fixture.state,
        &tasks,
        AgentHookReconcilePorts::new(
            || {
                read_home.store(true, Ordering::SeqCst);
                Some(fixture.home.clone())
            },
            || async { panic!("uninstalled emitter") },
            || async {
                assert!(read_home.load(Ordering::SeqCst));
                Err(AppError::Internal("fixture server failed".to_string()))
            },
            CLI_PATH,
        ),
    )
    .await;
    assert!(!fixture.base.exists());
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 프로젝트와_인밴드_파일은_server_전에_갱신되고_http는_새_server_답을_사용한다() {
    let fixture = Fixture::new();
    fixture.install_project();
    let codex = fixture.install_http(AGENT_NAME_CODEX);
    let gemini = fixture.install_http(AGENT_NAME_GEMINI);
    let gemini_before = std::fs::read(&gemini).unwrap();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    reconcile_installed_hooks(
        &fixture.state,
        &tasks,
        AgentHookReconcilePorts::new(
            || Some(fixture.home.clone()),
            || async {
                assert!(fixture.state.projects.try_write().is_some());
                service::HookEmitter::TerminalSequence
            },
            || async {
                assert!(service::agent_hook_entries_match(
                    AGENT_NAME_CLAUDE,
                    &hook_files::read_settings_local(&fixture.root).unwrap(),
                    service::HookEmitter::TerminalSequence
                ));
                assert!(service::agent_hook_entries_match(
                    AGENT_NAME_CODEX,
                    &hook_files::read_user_level_hooks(&codex).unwrap(),
                    service::USER_LEVEL_IN_BAND_EMITTER
                ));
                assert_eq!(std::fs::read(&gemini).unwrap(), gemini_before);
                Ok(server())
            },
            CLI_PATH,
        ),
    )
    .await;
    let after = hook_files::read_user_level_hooks(&gemini).unwrap();
    let command = service::build_command_hook_shell_command(CLI_PATH, &service::build_hook_url(&server(), AGENT_NAME_GEMINI));
    assert!(service::has_command_hook_entries_for_command(
        &after,
        GEMINI_MANAGED_HOOK_EVENTS,
        &command
    ));
    assert_eq!(after["user"], "keep");
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn server_실패는_앞선_프로젝트와_인밴드_갱신을_되돌리지_않고_http_원본은_유지한다() {
    let fixture = Fixture::new();
    fixture.install_project();
    let codex = fixture.install_http(AGENT_NAME_CODEX);
    let gemini = fixture.install_http(AGENT_NAME_GEMINI);
    let before = std::fs::read(&gemini).unwrap();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    reconcile_installed_hooks(
        &fixture.state,
        &tasks,
        AgentHookReconcilePorts::new(
            || Some(fixture.home.clone()),
            || async { service::HookEmitter::TerminalSequence },
            || async { Err(AppError::Internal("fixture server failure".to_string())) },
            CLI_PATH,
        ),
    )
    .await;
    assert!(service::agent_hook_entries_match(
        AGENT_NAME_CLAUDE,
        &hook_files::read_settings_local(&fixture.root).unwrap(),
        service::HookEmitter::TerminalSequence
    ));
    assert!(service::agent_hook_entries_match(
        AGENT_NAME_CODEX,
        &hook_files::read_user_level_hooks(&codex).unwrap(),
        service::USER_LEVEL_IN_BAND_EMITTER
    ));
    assert_eq!(std::fs::read(&gemini).unwrap(), before);
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn emitter_await_전_snapshot_쓰기와_기존_사용자_행_보존_정책을_유지한다() {
    let fixture = Fixture::new();
    fixture.install_project();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    reconcile_installed_hooks(
        &fixture.state,
        &tasks,
        AgentHookReconcilePorts::new(
            || Some(fixture.home.clone()),
            || async {
                hook_files::write_settings_local(&fixture.root, &serde_json::json!({"user": "newer", "late": true})).unwrap();
                service::HookEmitter::TerminalSequence
            },
            || async { Err(AppError::Internal("fixture server failure".to_string())) },
            CLI_PATH,
        ),
    )
    .await;
    let after = hook_files::read_settings_local(&fixture.root).unwrap();
    assert_eq!(after["user"], "snapshot");
    assert!(after.get("late").is_none(), "원래 reconcile은 await 전 snapshot을 쓴다");
    assert!(service::agent_hook_entries_match(
        AGENT_NAME_CLAUDE,
        &after,
        service::HookEmitter::TerminalSequence
    ));
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 토글_해제는_프로젝트와_사용자_소유_훅을_지운_뒤에만_server를_중지한다() {
    let fixture = Fixture::new();
    fixture.install_project();
    let codex = fixture.install_http(AGENT_NAME_CODEX);
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let stopped = AtomicBool::new(false);
    apply_agent_hooks_toggle(
        &fixture.state,
        &tasks,
        true,
        false,
        AgentHookReconcilePorts::new(
            || {
                assert!(!service::has_taide_agent_hook_entries(
                    AGENT_NAME_CLAUDE,
                    &hook_files::read_settings_local(&fixture.root).unwrap()
                ));
                Some(fixture.home.clone())
            },
            || async { panic!("disable emitter") },
            || async { panic!("disable server start") },
            CLI_PATH,
        ),
        || {
            assert!(!service::has_taide_marker_anywhere(
                &hook_files::read_user_level_hooks(&codex).unwrap()
            ));
            assert_eq!(tasks.tracked_count(), 1);
            stopped.store(true, Ordering::SeqCst);
        },
    )
    .await;
    assert!(stopped.load(Ordering::SeqCst));
    assert_eq!(hook_files::read_user_level_hooks(&codex).unwrap()["user"], "keep");
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 정상_root는_emitter_await의_reconcile_owner를_기다리고_취소는_파일을_보존한다() {
    let fixture = Fixture::new();
    fixture.install_project();
    let before = std::fs::read(hook_files::settings_local_path(&fixture.root)).unwrap();
    let runtime = tokio::runtime::Handle::current();
    let tasks = TaskSupervisor::new(runtime.clone());
    let request_tasks = tasks.clone();
    let state = fixture.state.clone();
    let home = fixture.home.clone();
    let (started, started_rx) = oneshot::channel();
    let request = tokio::spawn(async move {
        reconcile_installed_hooks(
            &state,
            &request_tasks,
            AgentHookReconcilePorts::new(
                || Some(home),
                || async move {
                    started.send(()).ok();
                    std::future::pending().await
                },
                || async { panic!("cancelled server") },
                CLI_PATH,
            ),
        )
        .await;
    });
    tokio::time::timeout(deadline(), started_rx).await.unwrap().unwrap();
    assert_eq!(tasks.tracked_count(), 1);
    let mut drain = ExitDrain::default();
    let (ready, mut ready_rx) = oneshot::channel();
    assert!(drain.begin(
        &runtime,
        tasks.clone(),
        LspInstallStore::new(),
        LspStore::new(),
        TerminalStore::new(),
        move || {
            ready.send(()).ok();
        }
    ));
    assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), &mut ready_rx)
        .await
        .is_err());
    assert!(!drain.is_ready());
    request.abort();
    assert!(request.await.is_err_and(|error| error.is_cancelled()));
    tokio::time::timeout(deadline(), ready_rx).await.unwrap().unwrap();
    assert!(drain.is_ready());
    assert_eq!(std::fs::read(hook_files::settings_local_path(&fixture.root)).unwrap(), before);
    assert_eq!(tasks.tracked_count(), 0);
}

#[test]
fn native는_동일_state_supervisor와_lazy_port를_주입하고_transport는_유지한다() {
    let native = include_str!("../../../src-tauri/src/domain/agent/hooks.rs");
    for name in [
        "apply_agent_hooks_toggle",
        "reconcile_installed_hooks",
        "uninstall_hooks_from_open_projects",
    ] {
        assert!(native.contains(&format!("agent_hook_reconcile::{name}(")));
    }
    assert!(native.contains("pub use taide_runtime::agent_hook_reconcile::remove_taide_hooks_from_roots;"));
    assert!(native.contains("|| commands::resolve_claude_hook_emitter(&tasks)"));
    assert!(native.contains("|| ensure_hooks_server_started(app)"));
    assert!(native.contains("|| stop_hooks_server(app)"));
    assert!(native.contains("async fn handle_connection("));
    assert!(native.contains("TcpListener::bind(\"127.0.0.1:0\")"));
}

#[tokio::test]
async fn server_await_취소_전_root는_owner를_기다리고_이미_쓴_인밴드_파일은_유지한다() {
    let fixture = Fixture::new();
    let codex = fixture.install_http(AGENT_NAME_CODEX);
    let gemini = fixture.install_http(AGENT_NAME_GEMINI);
    let before = std::fs::read(&gemini).unwrap();
    let runtime = tokio::runtime::Handle::current();
    let tasks = TaskSupervisor::new(runtime.clone());
    let request_tasks = tasks.clone();
    let state = fixture.state.clone();
    let home = fixture.home.clone();
    let (started, started_rx) = oneshot::channel();
    let request = tokio::spawn(async move {
        reconcile_installed_hooks(
            &state,
            &request_tasks,
            AgentHookReconcilePorts::new(
                || Some(home),
                || async { panic!("uninstalled emitter") },
                || async move {
                    started.send(()).ok();
                    std::future::pending().await
                },
                CLI_PATH,
            ),
        )
        .await;
    });
    tokio::time::timeout(deadline(), started_rx).await.unwrap().unwrap();
    assert_eq!(tasks.tracked_count(), 1);
    assert!(service::agent_hook_entries_match(
        AGENT_NAME_CODEX,
        &hook_files::read_user_level_hooks(&codex).unwrap(),
        service::USER_LEVEL_IN_BAND_EMITTER,
    ));
    let mut drain = ExitDrain::default();
    let (ready, mut ready_rx) = oneshot::channel();
    assert!(drain.begin(
        &runtime,
        tasks.clone(),
        LspInstallStore::new(),
        LspStore::new(),
        TerminalStore::new(),
        move || {
            ready.send(()).ok();
        },
    ));
    assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), &mut ready_rx)
        .await
        .is_err());
    assert!(!drain.is_ready());
    request.abort();
    assert!(request.await.is_err_and(|error| error.is_cancelled()));
    tokio::time::timeout(deadline(), ready_rx).await.unwrap().unwrap();
    assert!(drain.is_ready());
    assert_eq!(std::fs::read(&gemini).unwrap(), before);
    assert_eq!(tasks.tracked_count(), 0);
}

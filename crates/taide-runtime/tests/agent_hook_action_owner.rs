use std::path::PathBuf;
use std::time::Duration;

use taide_agent::constants::{AGENT_NAME_CLAUDE, AGENT_NAME_GEMINI};
use taide_agent::store::HooksServerInfo;
use taide_agent::{hook_files, service};
use taide_lsp::install::LspInstallStore;
use taide_lsp::store::LspStore;
use taide_model::error::AppErrorKind;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::{Project, ProjectDisplay};
use taide_runtime::agent_hook_actions::{agent_hooks_install, agent_hooks_status, agent_hooks_uninstall, AgentHookInstallPorts};
use taide_runtime::{AppState, ExitDrain, TaskSupervisor};
use taide_terminal::store::TerminalStore;
use tokio::sync::oneshot;
use uuid::Uuid;

const FIXTURE_PORT: u16 = 12345;
const FIXTURE_TIMEOUT_MS: u64 = 2_000;
const PENDING_OWNER_MS: u64 = 20;
const CLI_PATH: &str = "/fixture/taide";

struct Fixture {
    base: PathBuf,
    state: AppState,
    tasks: TaskSupervisor,
    project_id: ProjectId,
    home: String,
}

impl Fixture {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!("taide-hook-action-owner-{}", Uuid::new_v4()));
        let state = AppState::new(AppPaths::new(base.join("data")));
        state.settings.write().agent_hooks_enabled = true;
        let project = Project {
            id: ProjectId::new(),
            root: base.join("project").to_string_lossy().into_owned(),
            name: "fixture".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: ProjectDisplay::default(),
        };
        let project_id = project.id.clone();
        state.projects.write().insert(project_id.clone(), project);
        let home = base.join("home").to_string_lossy().into_owned();
        Self {
            base,
            state,
            tasks: TaskSupervisor::new(tokio::runtime::Handle::current()),
            project_id,
            home,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.base.exists() {
            std::fs::remove_dir_all(&self.base).expect("자기 UUID fixture만 정리");
        }
    }
}

fn deadline() -> Duration {
    Duration::from_millis(FIXTURE_TIMEOUT_MS)
}

fn server() -> HooksServerInfo {
    HooksServerInfo {
        port: FIXTURE_PORT,
        token: "fixture-token".to_string(),
    }
}

#[tokio::test]
async fn 닫힌_감독자는_status_install_uninstall의_모든_port와_파일을_건드리지_않는다() {
    let fixture = Fixture::new();
    fixture.tasks.stop_all();
    let status = agent_hooks_status(
        &fixture.state,
        &fixture.tasks,
        fixture.project_id.clone(),
        AGENT_NAME_CLAUDE.to_string(),
        || panic!("closed status home"),
    )
    .await;
    let install = agent_hooks_install(
        &fixture.state,
        &fixture.tasks,
        fixture.project_id.clone(),
        AGENT_NAME_CLAUDE.to_string(),
        AgentHookInstallPorts::new(
            || panic!("closed install home"),
            || async { panic!("closed emitter") },
            || panic!("closed cli"),
            || async { panic!("closed server") },
            CLI_PATH,
        ),
    )
    .await;
    let uninstall = agent_hooks_uninstall(
        &fixture.state,
        &fixture.tasks,
        fixture.project_id.clone(),
        AGENT_NAME_CLAUDE.to_string(),
        || panic!("closed uninstall home"),
    )
    .await;
    for result in [status, install, uninstall] {
        assert_eq!(result.unwrap_err().kind(), AppErrorKind::Internal);
    }
    assert_eq!(fixture.tasks.tracked_count(), 0);
    assert!(!fixture.base.exists());
}

#[tokio::test]
async fn emitter_대기_중_caller_취소는_파일을_쓰지_않고_정상_root가_owner를_기다린다() {
    let fixture = Fixture::new();
    let root = fixture.state.projects.read().get(&fixture.project_id).unwrap().root.clone();
    let state = fixture.state.clone();
    let tasks = fixture.tasks.clone();
    let project_id = fixture.project_id.clone();
    let runtime = tokio::runtime::Handle::current();
    let (started, started_rx) = oneshot::channel();
    let request = tokio::spawn(async move {
        agent_hooks_install(
            &state,
            &tasks,
            project_id,
            AGENT_NAME_CLAUDE.to_string(),
            AgentHookInstallPorts::new(
                || panic!("project scope home"),
                || async move {
                    started.send(()).ok();
                    std::future::pending().await
                },
                || panic!("project scope cli"),
                || async { panic!("project scope server") },
                CLI_PATH,
            ),
        )
        .await
    });
    tokio::time::timeout(deadline(), started_rx).await.unwrap().unwrap();
    assert_eq!(fixture.tasks.tracked_count(), 1);
    let mut drain = ExitDrain::default();
    let (ready, mut ready_rx) = oneshot::channel();
    assert!(drain.begin(
        &runtime,
        fixture.tasks.clone(),
        LspInstallStore::new(),
        LspStore::new(),
        TerminalStore::new(),
        move || {
            ready.send(()).ok();
        },
    ));
    assert!(tokio::time::timeout(Duration::from_millis(PENDING_OWNER_MS), &mut ready_rx)
        .await
        .is_err());
    assert!(!drain.is_ready());
    request.abort();
    assert!(request.await.is_err_and(|error| error.is_cancelled()));
    tokio::time::timeout(deadline(), ready_rx).await.unwrap().unwrap();
    assert!(drain.is_ready());
    assert!(!hook_files::settings_local_path(&root).exists());
    assert_eq!(fixture.tasks.tracked_count(), 0);
}

#[tokio::test]
async fn server_대기_뒤에도_http_파일_쓰기와_status_반환까지_owner를_보유한다() {
    let fixture = Fixture::new();
    let path = service::user_level_hooks_path(AGENT_NAME_GEMINI, Some(&fixture.home)).unwrap();
    let value = serde_json::json!({"user": "snapshot"});
    hook_files::write_user_level_hooks(&path, &value).unwrap();
    let state = fixture.state.clone();
    let tasks = fixture.tasks.clone();
    let home = fixture.home.clone();
    let runtime = tokio::runtime::Handle::current();
    let (started, started_rx) = oneshot::channel();
    let (release, released) = oneshot::channel();
    let request = tokio::spawn(async move {
        agent_hooks_install(
            &state,
            &tasks,
            ProjectId::new(),
            AGENT_NAME_GEMINI.to_string(),
            AgentHookInstallPorts::new(
                || Some(home),
                || async { panic!("user scope emitter") },
                || true,
                || async move {
                    started.send(()).ok();
                    released.await.unwrap();
                    Ok(server())
                },
                CLI_PATH,
            ),
        )
        .await
    });
    tokio::time::timeout(deadline(), started_rx).await.unwrap().unwrap();
    assert_eq!(fixture.tasks.tracked_count(), 1);
    let mut drain = ExitDrain::default();
    let (ready, mut ready_rx) = oneshot::channel();
    assert!(drain.begin(
        &runtime,
        fixture.tasks.clone(),
        LspInstallStore::new(),
        LspStore::new(),
        TerminalStore::new(),
        move || {
            ready.send(()).ok();
        },
    ));
    assert!(tokio::time::timeout(Duration::from_millis(PENDING_OWNER_MS), &mut ready_rx)
        .await
        .is_err());
    assert_eq!(hook_files::read_user_level_hooks(&path).unwrap(), value);
    release.send(()).unwrap();
    let status = tokio::time::timeout(deadline(), request).await.unwrap().unwrap().unwrap();
    assert!(status.installed);
    tokio::time::timeout(deadline(), ready_rx).await.unwrap().unwrap();
    assert!(drain.is_ready());
    let after = hook_files::read_user_level_hooks(&path).unwrap();
    let command = service::build_command_hook_shell_command(CLI_PATH, &service::build_hook_url(&server(), AGENT_NAME_GEMINI));
    assert!(service::has_command_hook_entries_for_command(
        &after,
        service::managed_hook_events_for(AGENT_NAME_GEMINI),
        &command
    ));
    assert_eq!(after["user"], "snapshot");
    assert_eq!(fixture.tasks.tracked_count(), 0);
}

#[test]
fn native는_공개_입력_반환을_유지하고_같은_감독자를_세_action에_주입한다() {
    let native = include_str!("../../../src-tauri/src/domain/agent/commands.rs");
    for name in ["agent_hooks_status", "agent_hooks_install", "agent_hooks_uninstall"] {
        let action = native
            .split(&format!("pub async fn {name}("))
            .nth(1)
            .unwrap()
            .split("\n}")
            .next()
            .unwrap();
        assert!(action.contains(&format!("agent_hook_actions::{name}(")));
        assert!(action.contains("&tasks"));
    }
    let runtime = include_str!("../src/agent_hook_actions.rs");
    assert_eq!(runtime.matches(".begin_operation(\"agent-hooks-").count(), 3);
    let bindings = include_str!("../../../src/shared/api/bindings.ts");
    for name in ["agentHooksStatus", "agentHooksInstall", "agentHooksUninstall"] {
        assert!(bindings.contains(&format!(
            "{name}: (projectId: ProjectId, agentName: string) => typedError<AgentHooksStatus, AppError>"
        )));
    }
}

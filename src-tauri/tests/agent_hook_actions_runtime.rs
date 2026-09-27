use std::cell::{Cell, RefCell};
use std::future::{ready, Ready};
use std::path::PathBuf;

use futures_util::FutureExt;
use taide_agent::constants::{AGENT_NAME_CLAUDE, AGENT_NAME_CODEX, AGENT_NAME_GEMINI, AGENT_NAME_OPENCODE, AGENT_NAME_PI};
use taide_agent::store::HooksServerInfo;
use taide_agent::{hook_files, service};
use taide_model::agent::HookInstallScope;
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::{Project, ProjectDisplay};
use taide_runtime::agent_hook_actions::AgentHookInstallPorts;
use taide_runtime::{agent_hook_actions, AppState, TaskSupervisor};
use uuid::Uuid;

const FIXTURE_CLI_PATH: &str = "/fixture/bin/taide";
const FIXTURE_PORT: u16 = 12345;
#[cfg(unix)]
const EXISTING_FILE_MODE: u32 = 0o640;
#[cfg(unix)]
const FILE_MODE_MASK: u32 = 0o777;

struct Fixture {
    state: AppState,
    tasks: TaskSupervisor,
    project_id: ProjectId,
    root: String,
    home: String,
}

impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!("taide-agent-hook-action-{}", Uuid::new_v4()));
        let state = AppState::new(AppPaths::new(directory.clone()));
        state.settings.write().agent_hooks_enabled = true;
        let project_id = ProjectId::new();
        let root = directory.join("project").to_string_lossy().into_owned();
        let home = directory.join("home").to_string_lossy().into_owned();
        state.projects.write().insert(
            project_id.clone(),
            Project {
                id: project_id.clone(),
                root: root.clone(),
                name: "fixture".to_string(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: ProjectDisplay::default(),
            },
        );
        Self {
            state,
            tasks: TaskSupervisor::new(tokio::runtime::Handle::current()),
            project_id,
            root,
            home,
        }
    }

    fn user_path(&self, name: &str) -> PathBuf {
        service::user_level_hooks_path(name, Some(&self.home)).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.state.paths.data_dir.exists() {
            std::fs::remove_dir_all(&self.state.paths.data_dir).expect("자기 UUID fixture만 정리");
        }
    }
}

fn no_home() -> Option<String> {
    panic!("이 scope/gate는 home을 조회하지 않는다")
}

fn no_emitter() -> Ready<service::HookEmitter> {
    panic!("이 scope/gate는 emitter를 조회하지 않는다")
}

fn no_cli() -> bool {
    panic!("이 shape/gate는 CLI를 조회하지 않는다")
}

fn no_server() -> Ready<AppResult<HooksServerInfo>> {
    panic!("이 shape/gate는 서버를 시작하지 않는다")
}

fn user_rows() -> serde_json::Value {
    serde_json::json!({
        "fixture": "preserved",
        "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "fixture-user-command" }] }] },
    })
}

#[cfg(unix)]
fn file_mode(path: &std::path::Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;

    std::fs::metadata(path).unwrap().permissions().mode() & FILE_MODE_MASK
}

#[tokio::test]
async fn project_scope의_닫힌_프로젝트는_home_port를_호출하지_않는다() {
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-agent-hook-action-{}", Uuid::new_v4())),
    ));
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let error = agent_hook_actions::agent_hooks_status(&state, &tasks, ProjectId::new(), AGENT_NAME_CLAUDE.to_string(), || {
        panic!("project scope는 home을 조회하지 않는다")
    })
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::NotFound);
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn unknown_agent와_disabled_gate는_project와_native_port보다_먼저다() {
    let fixture = Fixture::new();
    fixture.state.settings.write().agent_hooks_enabled = false;
    for name in ["fixture-unknown", AGENT_NAME_CLAUDE] {
        let error = agent_hook_actions::agent_hooks_install(
            &fixture.state,
            &fixture.tasks,
            ProjectId::new(),
            name.to_string(),
            AgentHookInstallPorts::new(no_home, no_emitter, no_cli, no_server, FIXTURE_CLI_PATH),
        )
        .await
        .unwrap_err();
        assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
        let expected = if name == AGENT_NAME_CLAUDE {
            "invalid argument: agent hooks are disabled in settings"
        } else {
            "invalid argument: unknown agent name: fixture-unknown"
        };
        assert_eq!(error.to_string(), expected);
    }
    let error = agent_hook_actions::agent_hooks_status(
        &fixture.state,
        &fixture.tasks,
        fixture.project_id.clone(),
        "fixture-unknown".to_string(),
        no_home,
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
    let error = agent_hook_actions::agent_hooks_uninstall(
        &fixture.state,
        &fixture.tasks,
        fixture.project_id.clone(),
        "fixture-unknown".to_string(),
        no_home,
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
    assert!(!fixture.state.paths.data_dir.exists());
}

#[tokio::test]
async fn project_install은_emitter_완료_뒤_최신_json을_읽고_기존_사용자_row를_보존한다() {
    let fixture = Fixture::new();
    let emitter_called = Cell::new(false);
    let (release, released) = tokio::sync::oneshot::channel();
    let mut action = Box::pin(agent_hook_actions::agent_hooks_install(
        &fixture.state,
        &fixture.tasks,
        fixture.project_id.clone(),
        AGENT_NAME_CLAUDE.to_string(),
        AgentHookInstallPorts::new(
            no_home,
            || {
                emitter_called.set(true);
                assert!(fixture.state.begin_mutation().now_or_never().is_some());
                async {
                    released.await.unwrap();
                    service::HookEmitter::TerminalSequence
                }
            },
            no_cli,
            no_server,
            FIXTURE_CLI_PATH,
        ),
    ));
    assert!(action.as_mut().now_or_never().is_none());
    assert!(emitter_called.get());
    assert!(!fixture.state.paths.data_dir.exists());
    let existing = user_rows();
    hook_files::write_settings_local(&fixture.root, &existing).unwrap();
    release.send(()).unwrap();
    let status = action.await.unwrap();
    assert_eq!(status.scope, HookInstallScope::Project);
    assert!(status.installed);
    assert!(!status.requires_taide_cli);
    let installed = hook_files::read_settings_local(&fixture.root).unwrap();
    let expected = service::inject_taide_agent_hook_entries(AGENT_NAME_CLAUDE, existing.clone(), service::HookEmitter::TerminalSequence);
    assert_eq!(installed, expected);
    let status = agent_hook_actions::agent_hooks_status(
        &fixture.state,
        &fixture.tasks,
        fixture.project_id.clone(),
        AGENT_NAME_CLAUDE.to_string(),
        no_home,
    )
    .await
    .unwrap();
    assert!(status.installed);
    fixture.state.settings.write().agent_hooks_enabled = false;
    let status = agent_hook_actions::agent_hooks_uninstall(
        &fixture.state,
        &fixture.tasks,
        fixture.project_id.clone(),
        AGENT_NAME_CLAUDE.to_string(),
        no_home,
    )
    .await
    .unwrap();
    assert!(!status.installed);
    assert_eq!(hook_files::read_settings_local(&fixture.root).unwrap(), existing);
}

#[tokio::test]
async fn project_missing_해제는_디렉터리를_만들지_않고_invalid_json은_emitter_뒤에도_보존된다() {
    let fixture = Fixture::new();
    let status = agent_hook_actions::agent_hooks_status(
        &fixture.state,
        &fixture.tasks,
        fixture.project_id.clone(),
        AGENT_NAME_CLAUDE.to_string(),
        no_home,
    )
    .await
    .unwrap();
    assert!(!status.installed);
    agent_hook_actions::agent_hooks_uninstall(
        &fixture.state,
        &fixture.tasks,
        fixture.project_id.clone(),
        AGENT_NAME_CLAUDE.to_string(),
        no_home,
    )
    .await
    .unwrap();
    assert!(!fixture.state.paths.data_dir.exists());
    let path = hook_files::settings_local_path(&fixture.root);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "{ fixture invalid").unwrap();
    let called = Cell::new(false);
    let error = agent_hook_actions::agent_hooks_install(
        &fixture.state,
        &fixture.tasks,
        fixture.project_id.clone(),
        AGENT_NAME_CLAUDE.to_string(),
        AgentHookInstallPorts::new(
            no_home,
            || {
                called.set(true);
                ready(service::HookEmitter::DevTty)
            },
            no_cli,
            no_server,
            FIXTURE_CLI_PATH,
        ),
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::Internal);
    assert!(called.get());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ fixture invalid");
}

#[tokio::test]
async fn user_in_band_json은_project_cli_emitter_server_없이_멱등_설치_해제한다() {
    let fixture = Fixture::new();
    let missing = agent_hook_actions::agent_hooks_status(
        &fixture.state,
        &fixture.tasks,
        ProjectId::new(),
        AGENT_NAME_CODEX.to_string(),
        || Some(fixture.home.clone()),
    )
    .await
    .unwrap();
    assert_eq!(missing.scope, HookInstallScope::User);
    assert!(!missing.installed);
    assert!(!missing.requires_taide_cli);
    assert!(!fixture.state.paths.data_dir.exists());
    let path = fixture.user_path(AGENT_NAME_CODEX);
    let existing = user_rows();
    hook_files::write_user_level_hooks(&path, &existing).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(EXISTING_FILE_MODE)).unwrap();
    }
    let expected = service::inject_taide_agent_hook_entries(AGENT_NAME_CODEX, existing.clone(), service::USER_LEVEL_IN_BAND_EMITTER);
    for _ in [false, true] {
        let status = agent_hook_actions::agent_hooks_install(
            &fixture.state,
            &fixture.tasks,
            ProjectId::new(),
            AGENT_NAME_CODEX.to_string(),
            AgentHookInstallPorts::new(|| Some(fixture.home.clone()), no_emitter, no_cli, no_server, FIXTURE_CLI_PATH),
        )
        .await
        .unwrap();
        assert!(status.installed);
        assert_eq!(hook_files::read_user_level_hooks(&path).unwrap(), expected);
        #[cfg(unix)]
        assert_eq!(file_mode(&path), EXISTING_FILE_MODE);
    }
    let installed = agent_hook_actions::agent_hooks_status(
        &fixture.state,
        &fixture.tasks,
        ProjectId::new(),
        AGENT_NAME_CODEX.to_string(),
        || Some(fixture.home.clone()),
    )
    .await
    .unwrap();
    assert!(installed.installed);
    let status = agent_hook_actions::agent_hooks_uninstall(
        &fixture.state,
        &fixture.tasks,
        ProjectId::new(),
        AGENT_NAME_CODEX.to_string(),
        || Some(fixture.home.clone()),
    )
    .await
    .unwrap();
    assert!(!status.installed);
    assert_eq!(hook_files::read_user_level_hooks(&path).unwrap(), existing);
}

#[tokio::test]
async fn owned_file은_비소유_파일을_보존하고_생성한_파일만_해제한다() {
    for name in [AGENT_NAME_OPENCODE, AGENT_NAME_PI] {
        let fixture = Fixture::new();
        let path = fixture.user_path(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "fixture user plugin").unwrap();
        let error = agent_hook_actions::agent_hooks_install(
            &fixture.state,
            &fixture.tasks,
            ProjectId::new(),
            name.to_string(),
            AgentHookInstallPorts::new(|| Some(fixture.home.clone()), no_emitter, no_cli, no_server, FIXTURE_CLI_PATH),
        )
        .await
        .unwrap_err();
        assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
        let status = agent_hook_actions::agent_hooks_status(&fixture.state, &fixture.tasks, ProjectId::new(), name.to_string(), || {
            Some(fixture.home.clone())
        })
        .await
        .unwrap();
        assert!(!status.installed);
        agent_hook_actions::agent_hooks_uninstall(&fixture.state, &fixture.tasks, ProjectId::new(), name.to_string(), || {
            Some(fixture.home.clone())
        })
        .await
        .unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "fixture user plugin");
        std::fs::remove_file(&path).unwrap();
        let status = agent_hook_actions::agent_hooks_install(
            &fixture.state,
            &fixture.tasks,
            ProjectId::new(),
            name.to_string(),
            AgentHookInstallPorts::new(|| Some(fixture.home.clone()), no_emitter, no_cli, no_server, FIXTURE_CLI_PATH),
        )
        .await
        .unwrap();
        assert!(status.installed);
        assert!(!status.requires_taide_cli);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            service::build_owned_hook_file_source(name).unwrap()
        );
        let status = agent_hook_actions::agent_hooks_status(&fixture.state, &fixture.tasks, ProjectId::new(), name.to_string(), || {
            Some(fixture.home.clone())
        })
        .await
        .unwrap();
        assert!(status.installed);
        agent_hook_actions::agent_hooks_uninstall(&fixture.state, &fixture.tasks, ProjectId::new(), name.to_string(), || {
            Some(fixture.home.clone())
        })
        .await
        .unwrap();
        assert!(!path.exists());
        agent_hook_actions::agent_hooks_uninstall(&fixture.state, &fixture.tasks, ProjectId::new(), name.to_string(), || {
            Some(fixture.home.clone())
        })
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn user_home_누락과_invalid_json은_cli_server_호출_전에_거절한다() {
    let fixture = Fixture::new();
    let error = agent_hook_actions::agent_hooks_status(
        &fixture.state,
        &fixture.tasks,
        ProjectId::new(),
        AGENT_NAME_CODEX.to_string(),
        || None,
    )
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), "operation failed: home directory not found");
    assert!(!fixture.state.paths.data_dir.exists());
    let path = fixture.user_path(AGENT_NAME_GEMINI);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "{ fixture invalid").unwrap();
    let error = agent_hook_actions::agent_hooks_install(
        &fixture.state,
        &fixture.tasks,
        ProjectId::new(),
        AGENT_NAME_GEMINI.to_string(),
        AgentHookInstallPorts::new(|| Some(fixture.home.clone()), no_emitter, no_cli, no_server, FIXTURE_CLI_PATH),
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::Internal);
    let error = agent_hook_actions::agent_hooks_status(
        &fixture.state,
        &fixture.tasks,
        ProjectId::new(),
        AGENT_NAME_GEMINI.to_string(),
        || Some(fixture.home.clone()),
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::Internal);
    let error = agent_hook_actions::agent_hooks_uninstall(
        &fixture.state,
        &fixture.tasks,
        ProjectId::new(),
        AGENT_NAME_GEMINI.to_string(),
        || Some(fixture.home.clone()),
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::Internal);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ fixture invalid");
}

#[tokio::test]
async fn http_cli_gate와_server_실패는_기존_json을_변경하지_않는다() {
    let fixture = Fixture::new();
    let path = fixture.user_path(AGENT_NAME_GEMINI);
    let existing = user_rows();
    hook_files::write_user_level_hooks(&path, &existing).unwrap();
    let original = std::fs::read(&path).unwrap();
    let error = agent_hook_actions::agent_hooks_install(
        &fixture.state,
        &fixture.tasks,
        ProjectId::new(),
        AGENT_NAME_GEMINI.to_string(),
        AgentHookInstallPorts::new(|| Some(fixture.home.clone()), no_emitter, || false, no_server, FIXTURE_CLI_PATH),
    )
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), "invalid argument: taide CLI is not installed yet");
    assert_eq!(std::fs::read(&path).unwrap(), original);
    let error = agent_hook_actions::agent_hooks_install(
        &fixture.state,
        &fixture.tasks,
        ProjectId::new(),
        AGENT_NAME_GEMINI.to_string(),
        AgentHookInstallPorts::new(
            || Some(fixture.home.clone()),
            no_emitter,
            || true,
            || async { Err(AppError::Internal("fixture server failure".to_string())) },
            FIXTURE_CLI_PATH,
        ),
    )
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), "operation failed: fixture server failure");
    assert_eq!(std::fs::read(&path).unwrap(), original);
}

#[tokio::test]
async fn http_설치는_home_cli_server_순서와_기존_url_command_timeout을_유지한다() {
    let fixture = Fixture::new();
    let path = fixture.user_path(AGENT_NAME_GEMINI);
    let existing = user_rows();
    hook_files::write_user_level_hooks(&path, &existing).unwrap();
    let server = HooksServerInfo {
        port: FIXTURE_PORT,
        token: "synthetic-not-a-credential".to_string(),
    };
    let stages = RefCell::new(Vec::new());
    let status = agent_hook_actions::agent_hooks_install(
        &fixture.state,
        &fixture.tasks,
        ProjectId::new(),
        AGENT_NAME_GEMINI.to_string(),
        AgentHookInstallPorts::new(
            || {
                stages.borrow_mut().push("home");
                Some(fixture.home.clone())
            },
            no_emitter,
            || {
                stages.borrow_mut().push("cli");
                true
            },
            || {
                stages.borrow_mut().push("server");
                assert!(fixture.state.begin_mutation().now_or_never().is_some());
                ready(Ok(server.clone()))
            },
            FIXTURE_CLI_PATH,
        ),
    )
    .await
    .unwrap();
    assert_eq!(*stages.borrow(), ["home", "cli", "server"]);
    assert!(status.installed);
    assert!(status.requires_taide_cli);
    assert_eq!(status.scope, HookInstallScope::User);
    let url = service::build_hook_url(&server, AGENT_NAME_GEMINI);
    assert_eq!(
        url,
        format!("http://127.0.0.1:{FIXTURE_PORT}/claude/hook?token=synthetic-not-a-credential&agent=gemini&taide=1")
    );
    let command = service::build_command_hook_shell_command(FIXTURE_CLI_PATH, &url);
    let expected = service::inject_taide_command_hook_entries(
        existing.clone(),
        service::managed_hook_events_for(AGENT_NAME_GEMINI),
        &command,
        service::user_level_hook_command_timeout(AGENT_NAME_GEMINI),
    );
    assert_eq!(hook_files::read_user_level_hooks(&path).unwrap(), expected);
    let installed = agent_hook_actions::agent_hooks_status(
        &fixture.state,
        &fixture.tasks,
        ProjectId::new(),
        AGENT_NAME_GEMINI.to_string(),
        || Some(fixture.home.clone()),
    )
    .await
    .unwrap();
    assert!(installed.installed);
    assert!(installed.requires_taide_cli);
    let status = agent_hook_actions::agent_hooks_uninstall(
        &fixture.state,
        &fixture.tasks,
        ProjectId::new(),
        AGENT_NAME_GEMINI.to_string(),
        || Some(fixture.home.clone()),
    )
    .await
    .unwrap();
    assert!(!status.installed);
    assert_eq!(hook_files::read_user_level_hooks(&path).unwrap(), existing);
}

#[test]
fn 세_command의_native_port는_기존_provider를_주입하고_file_url_helper는_공유한다() {
    let source = include_str!("../src/domain/agent/commands.rs");
    for name in ["agent_hooks_status", "agent_hooks_install", "agent_hooks_uninstall"] {
        let action = source
            .split_once(&format!("pub async fn {name}("))
            .unwrap()
            .1
            .split_once("\n}")
            .unwrap()
            .0;
        assert!(action.contains(&format!("agent_hook_actions::{name}(")));
        assert!(!action.contains("read_settings_local("));
        assert!(!action.contains("write_settings_local("));
    }
    let install = source
        .split_once("pub async fn agent_hooks_install(")
        .unwrap()
        .1
        .split_once("\n}")
        .unwrap()
        .0;
    assert!(install.contains("home::home_dir_env"));
    assert!(install.contains("resolve_claude_hook_emitter"));
    assert!(install.contains("|| resolve_cli_install_status().installed"));
    assert!(install.contains("|| hooks::ensure_hooks_server_started(&app)"));
    assert!(install.contains("TAIDE_CLI_TARGET_PATH"));
    let reconcile = include_str!("../../crates/taide-runtime/src/agent_hook_reconcile.rs");
    for helper in [
        "read_owned_hook_file",
        "read_settings_local",
        "read_user_level_hooks",
        "remove_owned_hook_file",
        "write_owned_hook_file",
        "write_settings_local",
        "write_user_level_hooks",
    ] {
        assert!(reconcile.contains(&format!("hook_files::{helper}(")));
    }
    let hooks = include_str!("../src/domain/agent/hooks.rs");
    assert!(hooks.contains("pub use taide_agent::service::build_hook_url;"));
    assert!(hooks.contains("spawn_transient_handle(\"agent-hooks-accept\""));
}

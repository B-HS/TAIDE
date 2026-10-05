use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use taide_runtime::{agent_actions, agent_hook_actions, agent_probe, TaskSupervisor};
use tauri::{Manager, State};

pub use taide_agent::store::{AgentHooksStore, AgentStore, HooksServerInfo};
pub use taide_runtime::agent_actions::{build_detected_agents, cleanup_all_wait_markers, resolve_state};

use super::hooks;
use super::service;
use super::types::{
    AgentHooksStatus, CliInstallStatus, ExternalOpenRequest, ProjectAgents, AGENT_NAME_CLAUDE, AGENT_PROTOCOL_VERSION,
    AGENT_PROTOCOL_VERSION_ENV_NAME, APP_VERSION_ENV_NAME, CLAUDE_VERSION_TIMEOUT_SECONDS,
};
use crate::error::{AppError, AppResult};
use crate::ids::ProjectId;
use crate::infra::home;
use crate::infra::terminal_scan::ScanOutcome;
use crate::platform::event_sink::TauriEventSink;
use crate::state::AppState;

#[cfg(unix)]
pub(super) const TAIDE_CLI_TARGET_PATH: &str = "/usr/local/bin/taide";
#[cfg(windows)]
pub(super) const TAIDE_CLI_TARGET_PATH: &str = "C:/Program Files/TAIDE/bin/taide.exe";

pub struct AgentForegroundPids(pub fn(&tauri::AppHandle, &ProjectId) -> Vec<(String, u32)>);

#[cfg(windows)]
pub use taide_runtime::agent_host::detect_agents_for_pids;
pub use taide_runtime::agent_host::detect_agents_for_pids_blocking;
#[cfg(all(test, unix))]
use taide_runtime::agent_host::{resolve_agent_names, resolve_process_infos};

/// Feeds one scanned pty chunk into the agent signals, if that session runs an agent. Wired from
/// the terminal domain through the assembly-owned `PtySessionObservers` (lib.rs) rather than
/// called across domains (architecture.md §2).
pub(crate) fn record_session_scan(app: &tauri::AppHandle, session_id: &str, outcome: &ScanOutcome) {
    let Some(agents) = app.try_state::<AgentStore>() else {
        return;
    };
    agents.record_scan(session_id, outcome);
}

/// Same wiring, for bytes going the other way: whoever is typing into this pty is watching it.
pub(crate) fn record_session_input(app: &tauri::AppHandle, session_id: &str) {
    let Some(agents) = app.try_state::<AgentStore>() else {
        return;
    };
    agents.record_input(session_id);
}

/// The protocol handshake every spawned terminal inherits: the hook commands TAIDE writes into
/// `settings.local.json` emit their event only when `TAIDE_AGENT_PROTOCOL_VERSION` is set, so the
/// same project opened in another terminal stays silent (contract §1.4). Registered by `lib.rs`'s
/// assembly on `terminal::commands::PtySpawnEnvProvider`, after [`editor_terminal_env`].
pub fn agent_protocol_env() -> Vec<(String, String)> {
    vec![
        (AGENT_PROTOCOL_VERSION_ENV_NAME.to_string(), AGENT_PROTOCOL_VERSION.to_string()),
        (APP_VERSION_ENV_NAME.to_string(), env!("CARGO_PKG_VERSION").to_string()),
    ]
}

/// Which emitter the installed Claude hooks use, decided once per app run.
///
/// The probe is one `claude --version` on the blocking pool with a deadline, because the answer is
/// only needed when hooks are installed or reconciled and a `claude` that hangs (or is not on
/// `PATH`) must not hold that up: every failure mode resolves to `DevTty`, which works on every
/// version. Cached in a `OnceLock` so reconciling ten projects forks once, not ten times.
pub(super) async fn resolve_claude_hook_emitter(tasks: &TaskSupervisor) -> service::HookEmitter {
    static EMITTER: OnceLock<service::HookEmitter> = OnceLock::new();

    agent_probe::resolve_claude_hook_emitter(tasks, &EMITTER, Duration::from_secs(CLAUDE_VERSION_TIMEOUT_SECONDS), || {
        std::process::Command::new(AGENT_NAME_CLAUDE)
            .arg(CLAUDE_VERSION_FLAG)
            .output()
            .map(|output| output.stdout)
    })
    .await
}

const CLAUDE_VERSION_FLAG: &str = "--version";

fn resolve_cli_install_status() -> CliInstallStatus {
    taide_runtime::agent_host::cli_install_status(Path::new(TAIDE_CLI_TARGET_PATH))
}

/// The `taide` CLI to point an injected `EDITOR` at: the installed `/usr/local/bin/taide` symlink
/// when it resolves to our own CLI sidecar (same ownership check `agent_cli_uninstall` applies, so
/// a same-named binary TAIDE did not install is never made every terminal's editor), otherwise the
/// CLI sidecar shipped inside the running app bundle (so ctrl+g works even before the user installs
/// the shell command). `None` for a dev build that has neither.
fn resolve_editor_cli_path() -> Option<String> {
    let status = resolve_cli_install_status();
    let is_owned_symlink = status
        .resolved_path
        .as_deref()
        .is_some_and(|resolved| service::is_cli_symlink_owned(Path::new(resolved)));
    if status.installed && !status.dangling && is_owned_symlink {
        return Some(status.target_path);
    }
    let sidecar = service::resolve_cli_install_target(&std::env::current_exe().ok()?)?;
    sidecar.exists().then(|| sidecar.to_string_lossy().to_string())
}

/// The `EDITOR`/`VISUAL` environment entries every newly spawned terminal inherits, so Claude
/// Code's ctrl+g opens its temp file in this running TAIDE and waits for the tab to close instead
/// of falling back to `vi`. Registered by `lib.rs`'s assembly on
/// `terminal::commands::PtySpawnEnvProvider` next to `ide::store::claude_terminal_env`; an
/// unresolvable CLI path yields an empty vec, never a spawn failure
/// (`docs/features/agent-integration.md` §2.3).
pub fn editor_terminal_env() -> Vec<(String, String)> {
    service::build_editor_env_entries(resolve_editor_cli_path().as_deref())
}

/// Resolves the CLI sidecar symlink target from the running app's own executable path, rejecting
/// unbundled dev builds. Shared by install (needs the target) and uninstall (only needs the gate).
#[cfg(target_os = "macos")]
fn cli_install_target() -> AppResult<PathBuf> {
    let exe = std::env::current_exe()?;
    service::resolve_cli_install_target(&exe).ok_or_else(|| {
        AppError::InvalidArgument("taide CLI shell command is only available in the installed TAIDE.app (not a dev build)".to_string())
    })
}

#[cfg(target_os = "macos")]
fn run_cli_osascript(script: &str) -> AppResult<()> {
    let args = service::build_osascript_args(script);
    let output = std::process::Command::new("osascript").args(&args).output()?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if service::is_osascript_user_cancelled(output.status.code(), &stderr) {
        return Ok(());
    }
    Err(AppError::Internal(format!("osascript failed: {stderr}")))
}

#[tauri::command]
#[specta::specta]
pub async fn agent_list(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    foreground_pids: State<'_, AgentForegroundPids>,
    agents: State<'_, AgentStore>,
    agent_hooks: State<'_, AgentHooksStore>,
    project_id: ProjectId,
) -> AppResult<ProjectAgents> {
    let tasks = app.state::<TaskSupervisor>();
    agent_actions::agent_list(
        &state,
        &agents,
        &agent_hooks,
        &tasks,
        || (foreground_pids.0)(&app, &project_id),
        |pids| detect_agents_for_pids_blocking(&tasks, &agents, pids),
        project_id.clone(),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn agent_release_marker(
    state: State<'_, AppState>,
    agents: State<'_, AgentStore>,
    tasks: State<'_, TaskSupervisor>,
    marker: String,
) -> AppResult<()> {
    agent_actions::agent_release_marker(&state, &agents, &tasks, marker).await
}

#[tauri::command]
#[specta::specta]
pub async fn agent_cli_status() -> AppResult<CliInstallStatus> {
    Ok(resolve_cli_install_status())
}

#[cfg(target_os = "macos")]
#[tauri::command]
#[specta::specta]
pub async fn agent_cli_install() -> AppResult<CliInstallStatus> {
    let target = cli_install_target()?;
    let source = Path::new(TAIDE_CLI_TARGET_PATH);

    let already_linked = std::fs::read_link(source).is_ok_and(|existing| existing == target);
    if !already_linked {
        run_cli_osascript(&service::build_cli_install_apple_script(&target, source))?;
    }

    Ok(resolve_cli_install_status())
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
#[specta::specta]
pub async fn agent_cli_install() -> AppResult<CliInstallStatus> {
    Err(AppError::InvalidArgument(
        "taide CLI shell command install is only supported on macOS".to_string(),
    ))
}

#[cfg(target_os = "macos")]
#[tauri::command]
#[specta::specta]
pub async fn agent_cli_uninstall() -> AppResult<CliInstallStatus> {
    cli_install_target()?;
    let source = Path::new(TAIDE_CLI_TARGET_PATH);

    match std::fs::symlink_metadata(source) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(resolve_cli_install_status()),
        Err(error) => return Err(AppError::from(error)),
    }

    let owned = std::fs::read_link(source).is_ok_and(|link_target| service::is_cli_symlink_owned(&link_target));
    if !owned {
        return Err(AppError::InvalidArgument(
            "the existing 'taide' command is not managed by TAIDE and was not removed".to_string(),
        ));
    }

    match std::fs::remove_file(source) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            run_cli_osascript(&service::build_cli_uninstall_apple_script(source))?;
        }
        Err(error) => return Err(AppError::from(error)),
    }

    Ok(resolve_cli_install_status())
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
#[specta::specta]
pub async fn agent_cli_uninstall() -> AppResult<CliInstallStatus> {
    Err(AppError::InvalidArgument(
        "taide CLI shell command uninstall is only supported on macOS".to_string(),
    ))
}

#[tauri::command]
#[specta::specta]
pub async fn agent_pending_external_opens(agents: State<'_, AgentStore>) -> AppResult<Vec<ExternalOpenRequest>> {
    agent_actions::agent_pending_external_opens(&agents).await
}

#[tauri::command]
#[specta::specta]
pub async fn agent_hooks_status(
    state: State<'_, AppState>,
    tasks: State<'_, TaskSupervisor>,
    project_id: ProjectId,
    agent_name: String,
) -> AppResult<AgentHooksStatus> {
    agent_hook_actions::agent_hooks_status(&state, &tasks, project_id, agent_name, home::home_dir_env).await
}

#[tauri::command]
#[specta::specta]
pub async fn agent_hooks_install(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    agent_name: String,
) -> AppResult<AgentHooksStatus> {
    let tasks = app.state::<TaskSupervisor>();
    agent_hook_actions::agent_hooks_install(
        &state,
        &tasks,
        project_id,
        agent_name,
        agent_hook_actions::AgentHookInstallPorts::new(
            home::home_dir_env,
            || resolve_claude_hook_emitter(&tasks),
            || resolve_cli_install_status().installed,
            || hooks::ensure_hooks_server_started(&app),
            TAIDE_CLI_TARGET_PATH,
        ),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn agent_hooks_uninstall(
    state: State<'_, AppState>,
    tasks: State<'_, TaskSupervisor>,
    project_id: ProjectId,
    agent_name: String,
) -> AppResult<AgentHooksStatus> {
    agent_hook_actions::agent_hooks_uninstall(&state, &tasks, project_id, agent_name, home::home_dir_env).await
}

pub(crate) async fn poll_agents(app: &tauri::AppHandle) {
    let tasks = app.state::<TaskSupervisor>();
    let state = app.state::<AppState>();
    let foreground_pids = app.state::<AgentForegroundPids>();
    let agents = app.state::<AgentStore>();
    let agent_hooks = app.state::<AgentHooksStore>();

    agent_actions::poll_agents(
        &state,
        &agents,
        &agent_hooks,
        &TauriEventSink(app),
        &tasks,
        |project_id| (foreground_pids.0)(app, project_id),
        |pids| detect_agents_for_pids_blocking(&tasks, &agents, pids),
    )
    .await;
}

/// Queues one CLI-originated open request — the cold-start argv below or a
/// `tauri-plugin-single-instance` relay (`lib.rs`), the only two places a path ever enters
/// `AppState::authorize_cli_opened_path`'s allowlist: the wait marker is registered for exit-time
/// cleanup, the path is let past the open-project boundary (Claude Code's Ctrl+G temp file lives
/// under the OS tmpdir, outside every root), and the request is queued for the frontend to drain
/// (`agent_pending_external_opens`). Emitting `AgentExternalOpen` stays with the single-instance
/// caller; a cold start has no subscribed frontend yet and relies on the boot-time drain alone.
pub(crate) fn queue_external_open(app_handle: &tauri::AppHandle, request: ExternalOpenRequest) {
    let agent_store = app_handle.state::<AgentStore>();
    if let Some(marker) = request.wait_marker.clone() {
        agent_store.register_wait_marker(marker);
    }
    app_handle.state::<AppState>().authorize_cli_opened_path(Path::new(&request.path));
    agent_store.push_pending_external_open(request);
}

/// Handles a cold-start `taide <file>` invocation. `tauri-plugin-single-instance` only forwards
/// argv to a *second* launch's callback, so a cold start that spawns the app fresh never reaches
/// it; this queues the request into `AgentStore` instead so the frontend can drain it on boot.
pub(crate) fn queue_cold_start_external_open(app_handle: &tauri::AppHandle) {
    let argv: Vec<String> = std::env::args().collect();
    let Some(request) = service::parse_cli_payload(&argv) else {
        return;
    };

    queue_external_open(app_handle, request);
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use taide_agent::hook_files::{read_settings_local, settings_local_path, write_settings_local, NEW_HOOKS_FILE_MODE};
    use uuid::Uuid;

    use super::*;

    fn temp_project_root(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("taide-agent-hooks-file-safety-{name}-{}", Uuid::new_v4()))
            .to_string_lossy()
            .to_string()
    }

    #[cfg(unix)]
    #[test]
    fn 프로세스_조회는_한_번의_ps_호출로_살아있는_pid를_해석한다() {
        let own = std::process::id();

        let infos = resolve_process_infos(&[own, own]);

        let info = infos.get(&own).expect("테스트 프로세스 자신의 pid 는 항상 살아 있다");
        assert!(!info.comm.is_empty());
        assert!(!info.cmdline.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn 조회할_pid가_없으면_ps를_호출하지_않는다() {
        assert!(resolve_process_infos(&[]).is_empty());
        assert!(resolve_agent_names(&[]).is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn 에이전트가_아닌_pid도_이름_캐시에_남겨_다음_틱에_다시_묻지_않는다() {
        let own = std::process::id();
        let store = AgentStore::new();
        let pids = vec![("session-1".to_string(), own)];

        assert_eq!(store.unresolved_pids(&pids), vec![own]);
        store.remember_process_names(resolve_agent_names(&[own]));

        assert!(store.unresolved_pids(&pids).is_empty(), "에이전트가 아니라는 답도 캐시된 답이다");
        assert!(store.probes_for(pids).is_empty(), "테스트 러너는 에이전트가 아니다");

        store.retain_process_names(&HashSet::new());
        assert_eq!(
            store.unresolved_pids(&[("session-1".to_string(), own)]),
            vec![own],
            "전경 pid 집합을 벗어난 항목은 축출된다"
        );
    }

    #[test]
    fn 비_json_파일은_읽기_실패로_원본을_보존한다() {
        let root = temp_project_root("invalid-json-preserved");
        let path = settings_local_path(&root);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"{ not json").unwrap();

        let result = read_settings_local(&root);

        assert!(result.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"{ not json");

        std::fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn 기존_파일의_권한을_보존해서_재작성한다() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_project_root("preserve-mode");
        let path = settings_local_path(&root);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"{}").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        write_settings_local(&root, &serde_json::json!({ "hooks": {} })).unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o644);

        std::fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn 신규_파일은_0600_권한으로_생성된다() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_project_root("new-file-mode");
        let path = settings_local_path(&root);

        write_settings_local(&root, &serde_json::json!({ "hooks": {} })).unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, NEW_HOOKS_FILE_MODE);

        std::fs::remove_dir_all(&root).ok();
    }
}

use std::future::Future;
use std::path::Path;

use taide_agent::constants::AGENT_NAME_CLAUDE;
use taide_agent::service::{build_hook_url, HookDelivery, HookInstallShape};
use taide_agent::store::HooksServerInfo;
use taide_agent::{hook_files, service};
use taide_model::error::AppResult;

use crate::{AppState, TaskSupervisor};

/// Supplies lazy host capabilities for reconciliation without exposing AppHandle to policy.
pub struct AgentHookReconcilePorts<'a, H, E, S> {
    resolve_home: H,
    resolve_project_emitter: E,
    start_server: S,
    cli_target_path: &'a str,
}

impl<'a, H, E, S> AgentHookReconcilePorts<'a, H, E, S> {
    /// Defers host work until the existing settings and installed-file gates have passed.
    pub fn new(resolve_home: H, resolve_project_emitter: E, start_server: S, cli_target_path: &'a str) -> Self {
        Self {
            resolve_home,
            resolve_project_emitter,
            start_server,
            cli_target_path,
        }
    }
}

/// Owns the existing toggle routing through reconciliation or uninstall followed by server stop.
pub async fn apply_agent_hooks_toggle<H, E, S, EF, SF>(
    state: &AppState,
    tasks: &TaskSupervisor,
    was_enabled: bool,
    enabled: bool,
    ports: AgentHookReconcilePorts<'_, H, E, S>,
    stop_server: impl FnOnce(),
) where
    H: FnOnce() -> Option<String>,
    E: FnOnce() -> EF,
    S: FnOnce() -> SF,
    EF: Future<Output = service::HookEmitter>,
    SF: Future<Output = AppResult<HooksServerInfo>>,
{
    if was_enabled == enabled {
        return;
    }
    let Some(_operation) = tasks.begin_operation("agent-hooks-toggle") else {
        return;
    };
    if enabled {
        reconcile_enabled_hooks(state, ports).await;
    } else {
        uninstall_open_project_hooks(state, ports.resolve_home).await;
        stop_server();
    }
}

/// Tracks an admitted reconcile through its final file application and host server response.
pub async fn reconcile_installed_hooks<H, E, S, EF, SF>(
    state: &AppState,
    tasks: &TaskSupervisor,
    ports: AgentHookReconcilePorts<'_, H, E, S>,
) where
    H: FnOnce() -> Option<String>,
    E: FnOnce() -> EF,
    S: FnOnce() -> SF,
    EF: Future<Output = service::HookEmitter>,
    SF: Future<Output = AppResult<HooksServerInfo>>,
{
    let Some(_operation) = tasks.begin_operation("agent-hooks-reconcile") else {
        return;
    };
    reconcile_enabled_hooks(state, ports).await;
}

async fn reconcile_enabled_hooks<H, E, S, EF, SF>(state: &AppState, ports: AgentHookReconcilePorts<'_, H, E, S>)
where
    H: FnOnce() -> Option<String>,
    E: FnOnce() -> EF,
    S: FnOnce() -> SF,
    EF: Future<Output = service::HookEmitter>,
    SF: Future<Output = AppResult<HooksServerInfo>>,
{
    let is_enabled = state.settings.read().agent_hooks_enabled;
    if !is_enabled {
        return;
    }

    let home = (ports.resolve_home)();
    reconcile_claude_project_hooks(state, ports.resolve_project_emitter).await;
    reconcile_in_band_user_level_hooks(home.as_deref());

    let Ok(server) = (ports.start_server)().await else {
        return;
    };
    reconcile_http_user_level_hooks(&server, home.as_deref(), ports.cli_target_path);
}

async fn reconcile_claude_project_hooks<Fut: Future<Output = service::HookEmitter>>(
    state: &AppState,
    resolve_project_emitter: impl FnOnce() -> Fut,
) {
    let roots: Vec<String> = {
        let guard = state.projects.read();
        guard.values().map(|project| project.root.clone()).collect()
    };

    let installed: Vec<(String, serde_json::Value)> = roots
        .into_iter()
        .filter_map(|root| hook_files::read_settings_local(&root).ok().map(|value| (root, value)))
        .filter(|(_, value)| service::has_taide_agent_hook_entries(AGENT_NAME_CLAUDE, value))
        .collect();
    if installed.is_empty() {
        return;
    }

    let emitter = resolve_project_emitter().await;
    for (root, value) in installed {
        if service::agent_hook_entries_match(AGENT_NAME_CLAUDE, &value, emitter) {
            continue;
        }
        let updated = service::inject_taide_agent_hook_entries(AGENT_NAME_CLAUDE, value, emitter);
        if let Err(error) = hook_files::write_settings_local(&root, &updated) {
            log::warn!("claude hooks 갱신 실패 ({root}): {error}");
        }
    }
}

fn reconcile_in_band_user_level_hooks(home_env: Option<&str>) {
    for spec in service::user_level_hook_agents() {
        let Some(hooks) = spec.hooks.filter(|hooks| hooks.delivery == HookDelivery::InBandTty) else {
            continue;
        };
        let Ok(path) = service::user_level_hooks_path(spec.name, home_env) else {
            continue;
        };
        match hooks.shape {
            HookInstallShape::JsonEntries => reconcile_in_band_hook_entries(spec.name, &path),
            HookInstallShape::OwnedFile => reconcile_owned_hook_file(spec.name, &path),
        }
    }
}

fn reconcile_in_band_hook_entries(agent_name: &str, path: &Path) {
    let Ok(value) = hook_files::read_user_level_hooks(path) else {
        return;
    };
    if !service::has_taide_marker_anywhere(&value) {
        return;
    }
    if service::agent_hook_entries_match(agent_name, &value, service::USER_LEVEL_IN_BAND_EMITTER) {
        return;
    }

    let updated = service::inject_taide_agent_hook_entries(agent_name, value, service::USER_LEVEL_IN_BAND_EMITTER);
    if let Err(error) = hook_files::write_user_level_hooks(path, &updated) {
        log::warn!("hooks 갱신 실패 (사용자 레벨, {agent_name}): {error}");
    }
}

fn reconcile_owned_hook_file(agent_name: &str, path: &Path) {
    let (Some(expected), Ok(Some(existing))) = (
        service::build_owned_hook_file_source(agent_name),
        hook_files::read_owned_hook_file(path),
    ) else {
        return;
    };
    if !service::is_owned_hook_file(&existing) || existing == expected {
        return;
    }

    if let Err(error) = hook_files::write_owned_hook_file(path, &expected) {
        log::warn!("플러그인 갱신 실패 (사용자 레벨, {agent_name}): {error}");
    }
}

fn reconcile_http_user_level_hooks(server: &HooksServerInfo, home_env: Option<&str>, cli_target_path: &str) {
    for spec in service::user_level_hook_agents() {
        if !spec.hooks.is_some_and(|hooks| hooks.delivery == HookDelivery::Http) {
            continue;
        }
        let agent_name = spec.name;
        let Ok(path) = service::user_level_hooks_path(agent_name, home_env) else {
            continue;
        };
        let Ok(value) = hook_files::read_user_level_hooks(&path) else {
            continue;
        };
        if !service::has_taide_marker_anywhere(&value) {
            continue;
        }

        let hook_url = build_hook_url(server, agent_name);
        let command = service::build_command_hook_shell_command(cli_target_path, &hook_url);
        let events = service::managed_hook_events_for(agent_name);
        if service::has_command_hook_entries_for_command(&value, events, &command) {
            continue;
        }

        let timeout = service::user_level_hook_command_timeout(agent_name);
        let updated = service::inject_taide_command_hook_entries(value, events, &command, timeout);
        if let Err(error) = hook_files::write_user_level_hooks(&path, &updated) {
            log::warn!("hooks URL 갱신 실패 (사용자 레벨, {agent_name}): {error}");
        }
    }
}

/// Owns open-project and user-level hook cleanup without invoking the host server transport.
pub async fn uninstall_hooks_from_open_projects(state: &AppState, tasks: &TaskSupervisor, resolve_home: impl FnOnce() -> Option<String>) {
    let Some(_operation) = tasks.begin_operation("agent-hooks-uninstall") else {
        return;
    };
    uninstall_open_project_hooks(state, resolve_home).await;
}

async fn uninstall_open_project_hooks(state: &AppState, resolve_home: impl FnOnce() -> Option<String>) {
    let roots: Vec<String> = {
        let guard = state.projects.read();
        guard.values().map(|project| project.root.clone()).collect()
    };
    remove_taide_hooks_from_roots(&roots);
    remove_taide_hooks_from_user_level_files(resolve_home().as_deref());
}

fn remove_taide_hooks_from_user_level_files(home_env: Option<&str>) {
    for spec in service::user_level_hook_agents() {
        let Some(hooks) = spec.hooks else {
            continue;
        };
        let agent_name = spec.name;
        let Ok(path) = service::user_level_hooks_path(agent_name, home_env) else {
            continue;
        };

        match hooks.shape {
            HookInstallShape::JsonEntries => {
                let Ok(value) = hook_files::read_user_level_hooks(&path) else {
                    log::warn!("hooks 제거 실패 (사용자 레벨, {agent_name}): 읽기 실패");
                    continue;
                };
                if !service::has_taide_marker_anywhere(&value) {
                    continue;
                }
                let updated = service::remove_taide_agent_hook_entries(agent_name, value);
                if let Err(error) = hook_files::write_user_level_hooks(&path, &updated) {
                    log::warn!("hooks 제거 실패 (사용자 레벨, {agent_name}): {error}");
                }
            }
            HookInstallShape::OwnedFile => {
                if let Err(error) = hook_files::remove_owned_hook_file(&path) {
                    log::warn!("플러그인 제거 실패 (사용자 레벨, {agent_name}): {error}");
                }
            }
        }
    }
}

/// Removes only TAIDE-owned project hook entries while preserving unrelated content.
pub fn remove_taide_hooks_from_roots(roots: &[String]) {
    for root in roots {
        let Ok(value) = hook_files::read_settings_local(root) else {
            continue;
        };
        if !service::has_taide_agent_hook_entries(AGENT_NAME_CLAUDE, &value) {
            continue;
        }
        let updated = service::remove_taide_agent_hook_entries(AGENT_NAME_CLAUDE, value);
        if let Err(error) = hook_files::write_settings_local(root, &updated) {
            log::warn!("hooks 제거 실패 ({root}): {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taide_agent::constants::{
        AGENT_NAME_CODEX, AGENT_NAME_GEMINI, AGENT_NAME_OPENCODE, AGENT_NAME_PI, CODEX_MANAGED_HOOK_EVENTS, GEMINI_MANAGED_HOOK_EVENTS,
    };

    #[cfg(unix)]
    const TEST_CLI_TARGET_PATH: &str = "/usr/local/bin/taide";
    #[cfg(windows)]
    const TEST_CLI_TARGET_PATH: &str = "C:/Program Files/TAIDE/bin/taide.exe";

    fn make_project_root(name: &str) -> String {
        let dir = std::env::temp_dir().join(format!("taide-hooks-uninstall-test-{name}-{}", uuid::Uuid::new_v4()));
        dir.to_string_lossy().to_string()
    }

    #[test]
    fn 훅_비활성화시_taide_항목만_제거하고_사용자_항목은_보존한다() {
        let root = make_project_root("keep-user-hooks");

        let existing = serde_json::json!({
            "hooks": {
                "Stop": [{ "hooks": [{ "type": "command", "command": "echo done" }] }]
            }
        });
        let injected = service::inject_taide_agent_hook_entries(AGENT_NAME_CLAUDE, existing, service::HookEmitter::DevTty);
        hook_files::write_settings_local(&root, &injected).expect("write settings");

        remove_taide_hooks_from_roots(std::slice::from_ref(&root));

        let after = hook_files::read_settings_local(&root).expect("read settings");
        assert!(!service::has_taide_agent_hook_entries(AGENT_NAME_CLAUDE, &after));
        let stop_entries = after["hooks"]["Stop"].as_array().expect("stop entries");
        assert_eq!(stop_entries.len(), 1);
        assert_eq!(stop_entries[0]["hooks"][0]["command"], "echo done");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn taide_항목이_없는_프로젝트는_건드리지_않는다() {
        let root = make_project_root("no-taide-hooks");

        let existing = serde_json::json!({
            "hooks": {
                "Stop": [{ "hooks": [{ "type": "command", "command": "echo done" }] }]
            }
        });
        hook_files::write_settings_local(&root, &existing).expect("write settings");

        remove_taide_hooks_from_roots(std::slice::from_ref(&root));

        let after = hook_files::read_settings_local(&root).expect("read settings");
        assert_eq!(after, existing);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 구버전_http_설치가_남은_프로젝트도_제거_대상이다() {
        let root = make_project_root("legacy-http-install");
        let legacy = serde_json::json!({
            "hooks": {
                "UserPromptSubmit": [{ "hooks": [{ "type": "http", "url": "http://127.0.0.1:9999/claude/hook?token=abc&taide=1" }] }],
                "Stop": [
                    { "hooks": [{ "type": "command", "command": "echo done" }] },
                    { "hooks": [{ "type": "http", "url": "http://127.0.0.1:9999/claude/hook?token=abc&taide=1" }] }
                ]
            }
        });
        hook_files::write_settings_local(&root, &legacy).expect("write settings");

        remove_taide_hooks_from_roots(std::slice::from_ref(&root));

        let after = hook_files::read_settings_local(&root).expect("read settings");
        assert!(!service::has_taide_agent_hook_entries(AGENT_NAME_CLAUDE, &after));
        assert!(after["hooks"].get("UserPromptSubmit").is_none());
        assert_eq!(after["hooks"]["Stop"].as_array().expect("stop entries").len(), 1);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 존재하지_않는_프로젝트가_섞여도_나머지는_계속_처리한다() {
        let missing_root = make_project_root("missing-parent-remains-absent");
        let real_root = make_project_root("real-project");

        let injected = service::inject_taide_agent_hook_entries(AGENT_NAME_CLAUDE, serde_json::json!({}), service::HookEmitter::DevTty);
        hook_files::write_settings_local(&real_root, &injected).expect("write settings");

        remove_taide_hooks_from_roots(&[missing_root.clone(), real_root.clone()]);

        let after = hook_files::read_settings_local(&real_root).expect("read settings");
        assert!(!service::has_taide_agent_hook_entries(AGENT_NAME_CLAUDE, &after));

        std::fs::remove_dir_all(&real_root).ok();
    }

    fn make_fake_home(name: &str) -> String {
        let dir = std::env::temp_dir().join(format!("taide-hooks-user-level-test-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create fake home");
        dir.to_string_lossy().to_string()
    }

    #[test]
    fn 훅_비활성화시_codex_gemini_사용자_레벨_taide_항목만_제거하고_사용자_항목은_보존한다() {
        let home = make_fake_home("keep-user-hooks");

        let codex_path = service::user_level_hooks_path(AGENT_NAME_CODEX, Some(&home)).unwrap();
        let codex_existing = serde_json::json!({
            "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "echo codex-user-hook" }] }] }
        });
        let codex_injected = service::inject_taide_command_hook_entries(
            codex_existing,
            CODEX_MANAGED_HOOK_EVENTS,
            "taide hook --url http://127.0.0.1:9999/claude/hook?token=abc&agent=codex&taide=1",
            5,
        );
        hook_files::write_user_level_hooks(&codex_path, &codex_injected).expect("write codex hooks");

        let gemini_path = service::user_level_hooks_path(AGENT_NAME_GEMINI, Some(&home)).unwrap();
        let gemini_injected = service::inject_taide_command_hook_entries(
            serde_json::json!({}),
            GEMINI_MANAGED_HOOK_EVENTS,
            "taide hook --url http://127.0.0.1:9999/claude/hook?token=abc&agent=gemini&taide=1",
            5_000,
        );
        hook_files::write_user_level_hooks(&gemini_path, &gemini_injected).expect("write gemini settings");

        remove_taide_hooks_from_user_level_files(Some(&home));

        let codex_after = hook_files::read_user_level_hooks(&codex_path).expect("read codex hooks");
        assert!(!service::has_taide_marker_anywhere(&codex_after));
        let stop_entries = codex_after["hooks"]["Stop"].as_array().expect("stop entries");
        assert_eq!(stop_entries.len(), 1);
        assert_eq!(stop_entries[0]["hooks"][0]["command"], "echo codex-user-hook");

        let gemini_after = hook_files::read_user_level_hooks(&gemini_path).expect("read gemini settings");
        assert!(gemini_after.get("hooks").is_none());

        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn taide_항목이_없는_사용자_레벨_파일은_건드리지_않는다() {
        let home = make_fake_home("no-taide-hooks");
        let codex_path = service::user_level_hooks_path(AGENT_NAME_CODEX, Some(&home)).unwrap();
        let existing = serde_json::json!({ "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "echo done" }] }] } });
        hook_files::write_user_level_hooks(&codex_path, &existing).expect("write codex hooks");

        remove_taide_hooks_from_user_level_files(Some(&home));

        let after = hook_files::read_user_level_hooks(&codex_path).expect("read codex hooks");
        assert_eq!(after, existing);

        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn 사용자_레벨_파일이_존재하지_않아도_패닉없이_넘어간다() {
        let home = make_fake_home("missing-files");
        remove_taide_hooks_from_user_level_files(Some(&home));
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn 재부팅으로_포트가_바뀌면_사용자_레벨_command_hook을_새_url로_재주입한다() {
        let home = make_fake_home("stale-port-heals");
        let gemini_path = service::user_level_hooks_path(AGENT_NAME_GEMINI, Some(&home)).unwrap();
        let stale_command = service::build_command_hook_shell_command(
            TEST_CLI_TARGET_PATH,
            "http://127.0.0.1:9999/claude/hook?token=old&agent=gemini&taide=1",
        );
        let existing = service::inject_taide_command_hook_entries(serde_json::json!({}), GEMINI_MANAGED_HOOK_EVENTS, &stale_command, 5_000);
        hook_files::write_user_level_hooks(&gemini_path, &existing).expect("write gemini settings");

        let server = HooksServerInfo {
            port: 10000,
            token: "new-token".to_string(),
        };
        reconcile_http_user_level_hooks(&server, Some(&home), TEST_CLI_TARGET_PATH);

        let after = hook_files::read_user_level_hooks(&gemini_path).expect("read gemini settings");
        let fresh_command = service::build_command_hook_shell_command(TEST_CLI_TARGET_PATH, &build_hook_url(&server, AGENT_NAME_GEMINI));
        assert!(service::has_command_hook_entries_for_command(
            &after,
            GEMINI_MANAGED_HOOK_EVENTS,
            &fresh_command
        ));

        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn 이미_최신_url이면_재주입하지_않고_그대로_둔다() {
        let home = make_fake_home("already-fresh");
        let gemini_path = service::user_level_hooks_path(AGENT_NAME_GEMINI, Some(&home)).unwrap();
        let server = HooksServerInfo {
            port: 20000,
            token: "current-token".to_string(),
        };
        let fresh_command = service::build_command_hook_shell_command(TEST_CLI_TARGET_PATH, &build_hook_url(&server, AGENT_NAME_GEMINI));
        let existing = service::inject_taide_command_hook_entries(serde_json::json!({}), GEMINI_MANAGED_HOOK_EVENTS, &fresh_command, 5_000);
        hook_files::write_user_level_hooks(&gemini_path, &existing).expect("write gemini settings");

        reconcile_http_user_level_hooks(&server, Some(&home), TEST_CLI_TARGET_PATH);

        let after = hook_files::read_user_level_hooks(&gemini_path).expect("read gemini settings");
        assert_eq!(after, existing);

        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn codex의_구버전_http_설치는_재조정에서_인밴드_명령으로_바뀐다() {
        let home = make_fake_home("codex-http-to-in-band");
        let codex_path = service::user_level_hooks_path(AGENT_NAME_CODEX, Some(&home)).unwrap();
        let stale_command = service::build_command_hook_shell_command(
            TEST_CLI_TARGET_PATH,
            "http://127.0.0.1:9999/claude/hook?token=old&agent=codex&taide=1",
        );
        let existing = service::inject_taide_command_hook_entries(serde_json::json!({}), CODEX_MANAGED_HOOK_EVENTS, &stale_command, 5);
        hook_files::write_user_level_hooks(&codex_path, &existing).expect("write codex hooks");

        reconcile_in_band_user_level_hooks(Some(&home));

        let after = hook_files::read_user_level_hooks(&codex_path).expect("read codex hooks");
        assert!(service::agent_hook_entries_match(
            AGENT_NAME_CODEX,
            &after,
            service::USER_LEVEL_IN_BAND_EMITTER
        ));
        assert!(
            !serde_json::to_string(&after).unwrap().contains("hook --url"),
            "인밴드로 옮긴 뒤에는 HTTP shim 명령이 남지 않는다"
        );

        reconcile_in_band_user_level_hooks(Some(&home));
        let twice = hook_files::read_user_level_hooks(&codex_path).expect("read codex hooks");
        assert_eq!(twice, after, "재조정은 멱등이다");

        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn taide_항목이_없는_codex_파일은_인밴드_재조정이_건드리지_않는다() {
        let home = make_fake_home("codex-untouched");
        let codex_path = service::user_level_hooks_path(AGENT_NAME_CODEX, Some(&home)).unwrap();
        let existing = serde_json::json!({ "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "echo mine" }] }] } });
        hook_files::write_user_level_hooks(&codex_path, &existing).expect("write codex hooks");

        reconcile_in_band_user_level_hooks(Some(&home));

        assert_eq!(hook_files::read_user_level_hooks(&codex_path).expect("read codex hooks"), existing);

        std::fs::remove_dir_all(&home).ok();
    }

    fn install_owned_file(agent_name: &str, home: &str) -> std::path::PathBuf {
        let path = service::user_level_hooks_path(agent_name, Some(home)).unwrap();
        let source = service::build_owned_hook_file_source(agent_name).expect("plugin source");
        hook_files::write_owned_hook_file(&path, &source).expect("write plugin");
        path
    }

    #[test]
    fn 소유한_플러그인_파일만_설치되고_토글_해제시_제거된다() {
        let home = make_fake_home("owned-file-install-remove");

        for agent_name in [AGENT_NAME_OPENCODE, AGENT_NAME_PI] {
            let path = install_owned_file(agent_name, &home);
            let written = std::fs::read_to_string(&path).expect("read plugin");
            assert!(service::is_owned_hook_file(&written));
            assert_eq!(written, service::build_owned_hook_file_source(agent_name).unwrap());
        }

        remove_taide_hooks_from_user_level_files(Some(&home));

        for agent_name in [AGENT_NAME_OPENCODE, AGENT_NAME_PI] {
            let path = service::user_level_hooks_path(agent_name, Some(&home)).unwrap();
            assert!(!path.exists(), "{agent_name} 플러그인이 남아 있다");
        }

        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn 남의_플러그인_파일은_재조정도_제거도_건드리지_않는다() {
        let home = make_fake_home("owned-file-third-party");
        let path = service::user_level_hooks_path(AGENT_NAME_OPENCODE, Some(&home)).unwrap();
        let foreign = "export const Mine = async () => ({})\n";
        hook_files::write_owned_hook_file(&path, foreign).expect("write third-party plugin");

        reconcile_in_band_user_level_hooks(Some(&home));
        assert_eq!(std::fs::read_to_string(&path).expect("read plugin"), foreign);

        remove_taide_hooks_from_user_level_files(Some(&home));
        assert_eq!(std::fs::read_to_string(&path).expect("read plugin"), foreign);

        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn 옛_버전의_소유_플러그인은_재조정에서_현재_소스로_갱신된다() {
        let home = make_fake_home("owned-file-heals");
        let path = service::user_level_hooks_path(AGENT_NAME_OPENCODE, Some(&home)).unwrap();
        let stale = format!(
            "// {} (v0)\nexport const TaideAgent = async () => ({{}})\n",
            service::OWNED_HOOK_FILE_MARKER
        );
        hook_files::write_owned_hook_file(&path, &stale).expect("write stale plugin");

        reconcile_in_band_user_level_hooks(Some(&home));

        let after = std::fs::read_to_string(&path).expect("read plugin");
        assert_eq!(after, service::build_owned_hook_file_source(AGENT_NAME_OPENCODE).unwrap());

        reconcile_in_band_user_level_hooks(Some(&home));
        assert_eq!(std::fs::read_to_string(&path).expect("read plugin"), after, "재조정은 멱등이다");

        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn 설치한_적_없는_플러그인_경로는_재조정이_만들지_않는다() {
        let home = make_fake_home("owned-file-absent");

        reconcile_in_band_user_level_hooks(Some(&home));

        for agent_name in [AGENT_NAME_OPENCODE, AGENT_NAME_PI] {
            let path = service::user_level_hooks_path(agent_name, Some(&home)).unwrap();
            assert!(!path.exists(), "설치는 사용자의 결정이다: {agent_name}");
        }

        std::fs::remove_dir_all(&home).ok();
    }
}

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::types::{
    AgentActivity, CliInstallStatus, DetectedAgent, ExternalOpenRequest, HookInstallScope, ACTIVITY_IDLE_QUIET_MS,
    ACTIVITY_WORKING_HOLD_MS, AGENT_NAME_CLAUDE, AGENT_NAME_CODEX, AGENT_NAME_GEMINI, CODEX_HOOK_COMMAND_TIMEOUT_SECONDS,
    CODEX_MANAGED_HOOK_EVENTS, GEMINI_HOOK_COMMAND_TIMEOUT_MS, GEMINI_MANAGED_HOOK_EVENTS, HOOKS_HTTP_TIMEOUT_SECONDS, HOOKS_URL_MARKER,
    HOOK_EVENT_AFTER_AGENT, HOOK_EVENT_BEFORE_AGENT, HOOK_EVENT_NOTIFICATION, HOOK_EVENT_PERMISSION_REQUEST, HOOK_EVENT_POST_TOOL_USE,
    HOOK_EVENT_STOP, HOOK_EVENT_USER_PROMPT_SUBMIT, HOOK_HANDLER_TYPE_COMMAND, HOOK_HANDLER_TYPE_HTTP, KNOWN_AGENT_NAMES,
    MANAGED_HOOK_EVENTS, WAIT_MARKER_PREFIX,
};
use crate::error::{AppError, AppResult};
use crate::ids::ProjectId;

const LINUX_COMM_MAX_LEN: usize = 15;
const NODE_RUNTIME_NAMES: &[&str] = &["node", "bun", "deno"];
const EDITOR_ENV_HINT: &str = "export EDITOR=\"taide --wait\"";
const CLI_WAIT_MARKER_FLAG: &str = "--wait-marker";

/// Filename of the bundled CLI sidecar binary (also the symlink ownership marker).
pub const CLI_SIDECAR_BIN_NAME: &str = "taide-cli";
const MACOS_BUNDLE_MACOS_DIR_NAME: &str = "MacOS";
const MACOS_BUNDLE_CONTENTS_DIR_NAME: &str = "Contents";
const MACOS_BUNDLE_EXTENSION: &str = "app";
const OSASCRIPT_ADMIN_PRIVILEGES_SUFFIX: &str = "with administrator privileges";
const OSASCRIPT_ARG_FLAG: &str = "-e";
const OSASCRIPT_CANCELLED_EXIT_CODE: i32 = 1;
const OSASCRIPT_CANCELLED_STDERR_MARKER: &str = "-128";

fn process_basename(raw: &str) -> &str {
    raw.rsplit(['/', '\\']).next().unwrap_or(raw)
}

fn match_agent_name_in(base: &str, known: &[&'static str]) -> Option<&'static str> {
    known
        .iter()
        .copied()
        .find(|&name| base == name || (base.len() >= LINUX_COMM_MAX_LEN && name.starts_with(base)))
}

pub fn detect_agent_name(comm: &str, cmdline: &str) -> Option<&'static str> {
    let comm_base = process_basename(comm.trim());

    if let Some(name) = match_agent_name_in(comm_base, KNOWN_AGENT_NAMES) {
        return Some(name);
    }

    if !NODE_RUNTIME_NAMES.contains(&comm_base) {
        return None;
    }

    cmdline
        .split_whitespace()
        .map(process_basename)
        .find_map(|arg| match_agent_name_in(arg, KNOWN_AGENT_NAMES))
}

pub fn validate_wait_marker_path(marker: &str, temp_dir: &Path) -> AppResult<PathBuf> {
    let candidate = PathBuf::from(marker);

    if !candidate.is_absolute() {
        return Err(AppError::InvalidArgument(format!("wait marker path must be absolute: {marker}")));
    }

    let file_name = candidate
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| AppError::InvalidArgument(format!("invalid wait marker file name: {marker}")))?;

    if !file_name.starts_with(WAIT_MARKER_PREFIX) {
        return Err(AppError::InvalidArgument(format!(
            "wait marker file name must start with {WAIT_MARKER_PREFIX}: {marker}"
        )));
    }

    let parent = candidate
        .parent()
        .ok_or_else(|| AppError::InvalidArgument(format!("invalid wait marker path: {marker}")))?;

    if parent != temp_dir {
        return Err(AppError::InvalidArgument(format!(
            "wait marker path must be under temp dir: {marker}"
        )));
    }

    Ok(candidate)
}

pub fn build_cli_install_status(target_path: &str, installed: bool, resolved_path: Option<String>, dangling: bool) -> CliInstallStatus {
    CliInstallStatus {
        installed,
        resolved_path,
        dangling,
        target_path: target_path.to_string(),
        editor_env_hint: EDITOR_ENV_HINT.to_string(),
    }
}

/// True when `exe_path` sits under a macOS `.app` bundle's `Contents/MacOS/` directory
/// (e.g. `/Applications/TAIDE.app/Contents/MacOS/TAIDE`). Used to reject CLI shell-command
/// install/uninstall from unbundled dev builds, since the CLI sidecar only exists in the bundle.
pub fn is_running_from_macos_app_bundle(exe_path: &Path) -> bool {
    let Some(macos_dir) = exe_path.parent() else { return false };
    if macos_dir.file_name().and_then(|name| name.to_str()) != Some(MACOS_BUNDLE_MACOS_DIR_NAME) {
        return false;
    }
    let Some(contents_dir) = macos_dir.parent() else { return false };
    if contents_dir.file_name().and_then(|name| name.to_str()) != Some(MACOS_BUNDLE_CONTENTS_DIR_NAME) {
        return false;
    }
    let Some(app_dir) = contents_dir.parent() else { return false };
    app_dir.extension().and_then(|extension| extension.to_str()) == Some(MACOS_BUNDLE_EXTENSION)
}

/// Resolves the CLI sidecar path (`.../Contents/MacOS/taide-cli`) to symlink to, derived from the
/// currently running app's own executable path. Returns `None` for unbundled dev builds.
pub fn resolve_cli_install_target(exe_path: &Path) -> Option<PathBuf> {
    if !is_running_from_macos_app_bundle(exe_path) {
        return None;
    }
    exe_path.parent().map(|macos_dir| macos_dir.join(CLI_SIDECAR_BIN_NAME))
}

/// Escapes a value so it can be embedded inside an AppleScript double-quoted string literal.
pub fn escape_applescript_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Wraps a value in single quotes for safe embedding as one POSIX shell argument.
pub fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Builds the `do shell script ... with administrator privileges` AppleScript source that creates
/// (or replaces) the `taide` symlink. Passed to `osascript -e` as a single argv element (no shell
/// involved), so only the AppleScript string escaping and the inner shell single-quoting matter.
pub fn build_cli_install_apple_script(target: &Path, source: &Path) -> String {
    let shell_command = format!(
        "mkdir -p /usr/local/bin && ln -sf {} {}",
        shell_single_quote(&target.to_string_lossy()),
        shell_single_quote(&source.to_string_lossy()),
    );
    format!(
        "do shell script \"{}\" {OSASCRIPT_ADMIN_PRIVILEGES_SUFFIX}",
        escape_applescript_string(&shell_command)
    )
}

/// Builds the `do shell script ... with administrator privileges` AppleScript source that removes
/// the `taide` symlink (used only when a plain `remove_file` fails with `PermissionDenied`).
pub fn build_cli_uninstall_apple_script(source: &Path) -> String {
    let shell_command = format!("rm {}", shell_single_quote(&source.to_string_lossy()));
    format!(
        "do shell script \"{}\" {OSASCRIPT_ADMIN_PRIVILEGES_SUFFIX}",
        escape_applescript_string(&shell_command)
    )
}

/// Builds the `osascript` argv (excluding the program name itself) for a given AppleScript source.
/// `-e` is passed as its own argv element so the script never goes through a shell.
pub fn build_osascript_args(script: &str) -> Vec<String> {
    vec![OSASCRIPT_ARG_FLAG.to_string(), script.to_string()]
}

/// True when an `osascript` failure represents the user dismissing the administrator-privileges
/// prompt (exit code 1, stderr containing the `-128` "User canceled" marker) rather than a real
/// error. Callers should treat this as a quiet no-op, not a failure.
pub fn is_osascript_user_cancelled(exit_code: Option<i32>, stderr: &str) -> bool {
    exit_code == Some(OSASCRIPT_CANCELLED_EXIT_CODE) && stderr.contains(OSASCRIPT_CANCELLED_STDERR_MARKER)
}

/// True when a symlink's target file name is the CLI sidecar binary, i.e. the symlink was created
/// by TAIDE's own install flow. Used to refuse uninstalling a symlink/file we don't own.
pub fn is_cli_symlink_owned(link_target: &Path) -> bool {
    link_target.file_name().and_then(|name| name.to_str()) == Some(CLI_SIDECAR_BIN_NAME)
}

pub fn parse_cli_payload(argv: &[String]) -> Option<ExternalOpenRequest> {
    let mut wait_marker: Option<String> = None;
    let mut path: Option<String> = None;

    let mut iter = argv.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == CLI_WAIT_MARKER_FLAG {
            wait_marker = iter.next().cloned();
        } else if path.is_none() {
            path = Some(arg.clone());
        }
    }

    path.map(|path| ExternalOpenRequest { path, wait_marker })
}

pub fn agents_changed(previous: &[DetectedAgent], current: &[DetectedAgent]) -> bool {
    if previous.len() != current.len() {
        return true;
    }

    let mut previous_sorted = previous.to_vec();
    let mut current_sorted = current.to_vec();
    previous_sorted.sort_by(|a, b| a.session_id.cmp(&b.session_id));
    current_sorted.sort_by(|a, b| a.session_id.cmp(&b.session_id));

    previous_sorted != current_sorted
}

#[derive(Debug, Clone)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub name: String,
    pub cmdline: String,
}

pub fn find_descendant_agent(processes: &[ProcessSnapshot], root_pid: u32) -> Option<(u32, &'static str)> {
    let mut frontier = vec![root_pid];
    let mut visited: HashSet<u32> = HashSet::new();

    while let Some(pid) = frontier.pop() {
        if !visited.insert(pid) {
            continue;
        }

        for process in processes.iter().filter(|process| process.parent_pid == Some(pid)) {
            if let Some(name) = detect_agent_name(&process.name, &process.cmdline) {
                return Some((process.pid, name));
            }
            frontier.push(process.pid);
        }
    }

    None
}

const WINDOWS_EXE_SUFFIX: &str = ".exe";

pub fn strip_windows_exe_suffix(name: &str) -> &str {
    if name.len() > WINDOWS_EXE_SUFFIX.len() && name[name.len() - WINDOWS_EXE_SUFFIX.len()..].eq_ignore_ascii_case(WINDOWS_EXE_SUFFIX) {
        &name[..name.len() - WINDOWS_EXE_SUFFIX.len()]
    } else {
        name
    }
}

#[derive(Debug, Clone)]
pub struct DetectedAgentProbe {
    pub session_id: String,
    pub name: String,
    pub pid: u32,
    pub state: Option<char>,
}

#[derive(Debug, Deserialize)]
pub struct HookPayload {
    pub hook_event_name: String,
    #[serde(default)]
    pub cwd: String,
}

pub fn is_probe_active(state: Option<char>) -> bool {
    matches!(state, Some('R'))
}

pub fn classify_activity(previous: AgentActivity, ms_since_active: Option<u64>) -> AgentActivity {
    match ms_since_active {
        None => AgentActivity::Unknown,
        Some(elapsed) if elapsed < ACTIVITY_WORKING_HOLD_MS => AgentActivity::Working,
        Some(elapsed) if elapsed >= ACTIVITY_IDLE_QUIET_MS => AgentActivity::Idle,
        Some(_) => previous,
    }
}

pub use crate::infra::crypto::constant_time_eq;

fn is_cwd_within_root(cwd: &str, root: &str) -> bool {
    let cwd = cwd.trim_end_matches('/');
    let root = root.trim_end_matches('/');
    cwd == root || cwd.starts_with(&format!("{root}/"))
}

pub fn match_project_by_cwd<'a>(cwd: &str, projects: &'a [(ProjectId, String)]) -> Option<&'a ProjectId> {
    if cwd.is_empty() {
        return None;
    }
    projects
        .iter()
        .filter(|(_, root)| is_cwd_within_root(cwd, root))
        .max_by_key(|(_, root)| root.len())
        .map(|(id, _)| id)
}

pub fn map_hook_event_to_activity(agent_name: &str, event_name: &str) -> Option<AgentActivity> {
    match agent_name {
        AGENT_NAME_CLAUDE => match event_name {
            HOOK_EVENT_USER_PROMPT_SUBMIT => Some(AgentActivity::Working),
            HOOK_EVENT_NOTIFICATION => Some(AgentActivity::AwaitingInput),
            HOOK_EVENT_STOP => Some(AgentActivity::Idle),
            _ => None,
        },
        AGENT_NAME_CODEX => match event_name {
            HOOK_EVENT_USER_PROMPT_SUBMIT => Some(AgentActivity::Working),
            HOOK_EVENT_PERMISSION_REQUEST => Some(AgentActivity::AwaitingInput),
            HOOK_EVENT_POST_TOOL_USE => Some(AgentActivity::Working),
            HOOK_EVENT_STOP => Some(AgentActivity::Idle),
            _ => None,
        },
        AGENT_NAME_GEMINI => match event_name {
            HOOK_EVENT_BEFORE_AGENT => Some(AgentActivity::Working),
            HOOK_EVENT_NOTIFICATION => Some(AgentActivity::AwaitingInput),
            HOOK_EVENT_AFTER_AGENT => Some(AgentActivity::Idle),
            _ => None,
        },
        _ => None,
    }
}

pub fn is_hook_managed_agent(name: &str) -> bool {
    KNOWN_AGENT_NAMES.contains(&name)
}

pub fn hook_scope_for_agent(agent_name: &str) -> AppResult<HookInstallScope> {
    match agent_name {
        AGENT_NAME_CLAUDE => Ok(HookInstallScope::Project),
        AGENT_NAME_CODEX | AGENT_NAME_GEMINI => Ok(HookInstallScope::User),
        other => Err(AppError::InvalidArgument(format!("unknown agent name: {other}"))),
    }
}

pub fn build_command_hook_shell_command(taide_cli_path: &str, hook_url: &str) -> String {
    format!("\"{taide_cli_path}\" hook --url \"{hook_url}\"")
}

pub fn managed_hook_events_for(agent_name: &str) -> &'static [&'static str] {
    match agent_name {
        AGENT_NAME_CLAUDE => MANAGED_HOOK_EVENTS,
        AGENT_NAME_CODEX => CODEX_MANAGED_HOOK_EVENTS,
        AGENT_NAME_GEMINI => GEMINI_MANAGED_HOOK_EVENTS,
        _ => &[],
    }
}

pub fn user_level_hook_command_timeout(agent_name: &str) -> u64 {
    match agent_name {
        AGENT_NAME_CODEX => CODEX_HOOK_COMMAND_TIMEOUT_SECONDS,
        AGENT_NAME_GEMINI => GEMINI_HOOK_COMMAND_TIMEOUT_MS,
        _ => HOOKS_HTTP_TIMEOUT_SECONDS,
    }
}

const CODEX_HOME_RELATIVE_PATH: &str = ".codex/hooks.json";
const GEMINI_HOME_RELATIVE_PATH: &str = ".gemini/settings.json";

pub fn user_level_hooks_path(agent_name: &str, home_env: Option<&str>) -> AppResult<PathBuf> {
    let relative = match agent_name {
        AGENT_NAME_CODEX => CODEX_HOME_RELATIVE_PATH,
        AGENT_NAME_GEMINI => GEMINI_HOME_RELATIVE_PATH,
        other => return Err(AppError::InvalidArgument(format!("agent has no user-level hooks path: {other}"))),
    };
    let home = home_env
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Internal("home directory not found".to_string()))?;
    Ok(PathBuf::from(home).join(relative))
}

fn json_value_contains_marker(value: &serde_json::Value, marker: &str) -> bool {
    match value {
        serde_json::Value::String(text) => text.contains(marker),
        serde_json::Value::Array(items) => items.iter().any(|item| json_value_contains_marker(item, marker)),
        serde_json::Value::Object(map) => map.values().any(|item| json_value_contains_marker(item, marker)),
        _ => false,
    }
}

pub fn has_taide_marker_anywhere(root: &serde_json::Value) -> bool {
    json_value_contains_marker(root, HOOKS_URL_MARKER)
}

fn hook_handler_value_contains(hook: &serde_json::Value, needle: &str) -> bool {
    hook.get("url")
        .or_else(|| hook.get("command"))
        .and_then(|value| value.as_str())
        .is_some_and(|value| value.contains(needle))
}

fn is_taide_managed_entry(entry: &serde_json::Value) -> bool {
    entry
        .get("hooks")
        .and_then(|hooks| hooks.as_array())
        .map(|hooks| hooks.iter().any(|hook| hook_handler_value_contains(hook, HOOKS_URL_MARKER)))
        .unwrap_or(false)
}

fn remove_taide_managed_entries(mut root: serde_json::Value, events: &[&str]) -> serde_json::Value {
    if let Some(hooks_obj) = root.get_mut("hooks").and_then(|value| value.as_object_mut()) {
        for event in events {
            if let Some(entries) = hooks_obj.get_mut(*event).and_then(|value| value.as_array_mut()) {
                entries.retain(|entry| !is_taide_managed_entry(entry));
            }
        }
        hooks_obj.retain(|_, value| !value.as_array().is_some_and(|entries| entries.is_empty()));
    }
    if root
        .get("hooks")
        .and_then(|value| value.as_object())
        .is_some_and(|hooks_obj| hooks_obj.is_empty())
    {
        if let Some(obj) = root.as_object_mut() {
            obj.remove("hooks");
        }
    }
    root
}

fn inject_taide_managed_entries(root: serde_json::Value, events: &[&str], hook_handler: serde_json::Value) -> serde_json::Value {
    let root = remove_taide_managed_entries(root, events);
    let mut root = if root.is_object() { root } else { serde_json::json!({}) };

    if let Some(root_obj) = root.as_object_mut() {
        let hooks_slot = root_obj.entry("hooks".to_string()).or_insert_with(|| serde_json::json!({}));
        if !hooks_slot.is_object() {
            *hooks_slot = serde_json::json!({});
        }

        if let Some(hooks_obj) = hooks_slot.as_object_mut() {
            for event in events {
                let event_slot = hooks_obj.entry((*event).to_string()).or_insert_with(|| serde_json::json!([]));
                if !event_slot.is_array() {
                    *event_slot = serde_json::json!([]);
                }
                if let Some(entries) = event_slot.as_array_mut() {
                    entries.push(serde_json::json!({ "hooks": [hook_handler.clone()] }));
                }
            }
        }
    }

    root
}

pub fn remove_taide_hook_entries(root: serde_json::Value) -> serde_json::Value {
    remove_taide_managed_entries(root, MANAGED_HOOK_EVENTS)
}

pub fn inject_taide_hook_entries(root: serde_json::Value, hook_url: &str) -> serde_json::Value {
    inject_taide_managed_entries(
        root,
        MANAGED_HOOK_EVENTS,
        serde_json::json!({ "type": HOOK_HANDLER_TYPE_HTTP, "url": hook_url, "timeout": HOOKS_HTTP_TIMEOUT_SECONDS }),
    )
}

pub fn remove_taide_command_hook_entries(root: serde_json::Value, events: &[&str]) -> serde_json::Value {
    remove_taide_managed_entries(root, events)
}

pub fn inject_taide_command_hook_entries(root: serde_json::Value, events: &[&str], command: &str, timeout: u64) -> serde_json::Value {
    inject_taide_managed_entries(
        root,
        events,
        serde_json::json!({ "type": HOOK_HANDLER_TYPE_COMMAND, "command": command, "timeout": timeout }),
    )
}

/// Shared by [`has_hook_entries_for_url`] (the HTTP hook family — always checks the fixed
/// [`MANAGED_HOOK_EVENTS`], which is never empty) and [`has_command_hook_entries_for_command`]
/// (the shell-command family — checks a caller-supplied `events` slice, which *can* be empty for
/// an unmanaged agent, hence the explicit guard here rather than relying on `[].iter().all(..)`
/// vacuously returning `true`). Both need the identical "every one of `events` has a TAIDE-managed
/// entry whose handler matches" walk, differing only in which hook field (`url`/`command`) the
/// per-entry predicate inspects.
fn has_managed_entries_matching(root: &serde_json::Value, events: &[&str], matches_handler: impl Fn(&serde_json::Value) -> bool) -> bool {
    if events.is_empty() {
        return false;
    }
    let Some(hooks_obj) = root.get("hooks").and_then(|value| value.as_object()) else {
        return false;
    };
    events.iter().all(|event| {
        hooks_obj
            .get(*event)
            .and_then(|value| value.as_array())
            .is_some_and(|entries| entries.iter().filter(|entry| is_taide_managed_entry(entry)).any(&matches_handler))
    })
}

pub fn has_hook_entries_for_url(root: &serde_json::Value, hook_url: &str) -> bool {
    has_managed_entries_matching(root, MANAGED_HOOK_EVENTS, |entry| entry_has_url(entry, hook_url))
}

fn entry_has_url(entry: &serde_json::Value, hook_url: &str) -> bool {
    entry.get("hooks").and_then(|hooks| hooks.as_array()).is_some_and(|hooks| {
        hooks
            .iter()
            .any(|hook| hook.get("url").and_then(|url| url.as_str()) == Some(hook_url))
    })
}

pub fn has_command_hook_entries_for_command(root: &serde_json::Value, events: &[&str], command: &str) -> bool {
    has_managed_entries_matching(root, events, |entry| entry_has_command(entry, command))
}

fn entry_has_command(entry: &serde_json::Value, command: &str) -> bool {
    entry.get("hooks").and_then(|hooks| hooks.as_array()).is_some_and(|hooks| {
        hooks
            .iter()
            .any(|hook| hook.get("command").and_then(|value| value.as_str()) == Some(command))
    })
}

pub fn has_taide_hook_entries(root: &serde_json::Value) -> bool {
    root.get("hooks")
        .and_then(|value| value.as_object())
        .map(|hooks_obj| {
            MANAGED_HOOK_EVENTS.iter().any(|event| {
                hooks_obj
                    .get(*event)
                    .and_then(|value| value.as_array())
                    .is_some_and(|entries| entries.iter().any(is_taide_managed_entry))
            })
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 프로세스명이_직접_claude면_감지한다() {
        assert_eq!(detect_agent_name("claude", "claude"), Some("claude"));
    }

    #[test]
    fn node_런타임_위에서_실행된_claude_바이너리를_감지한다() {
        let cmdline = "/usr/local/bin/node /Users/dev/.claude/local/claude";
        assert_eq!(detect_agent_name("node", cmdline), Some("claude"));
    }

    #[test]
    fn 무관한_프로세스는_감지하지_않는다() {
        assert_eq!(detect_agent_name("bash", "bash"), None);
        assert_eq!(detect_agent_name("node", "node server.js"), None);
    }

    #[test]
    fn comm이_리눅스_15자에서_잘려도_고유하게_매칭되면_감지한다() {
        let known: &[&str] = &["gemini-experimental"];
        let truncated = &"gemini-experimental"[..LINUX_COMM_MAX_LEN];
        assert_eq!(match_agent_name_in(truncated, known), Some("gemini-experimental"));
    }

    #[test]
    fn 짧은_접두사는_잘림으로_간주하지_않는다() {
        let known: &[&str] = &["claude"];
        assert_eq!(match_agent_name_in("cla", known), None);
    }

    #[test]
    fn 정상_마커_경로는_통과한다() {
        let temp = Path::new("/tmp");
        let result = validate_wait_marker_path("/tmp/taide-wait-abc123", temp);
        assert_eq!(result.unwrap(), PathBuf::from("/tmp/taide-wait-abc123"));
    }

    #[test]
    fn 접두사가_없는_파일명은_거부한다() {
        let temp = Path::new("/tmp");
        assert!(validate_wait_marker_path("/tmp/other-file", temp).is_err());
    }

    #[test]
    fn temp_dir_밖의_경로는_거부한다() {
        let temp = Path::new("/tmp");
        assert!(validate_wait_marker_path("/etc/taide-wait-abc123", temp).is_err());
    }

    #[test]
    fn 상대경로는_거부한다() {
        let temp = Path::new("/tmp");
        assert!(validate_wait_marker_path("taide-wait-abc123", temp).is_err());
    }

    #[test]
    fn 상위_디렉토리_순회_시도는_거부한다() {
        let temp = Path::new("/tmp");
        assert!(validate_wait_marker_path("/tmp/../etc/taide-wait-abc123", temp).is_err());
    }

    #[test]
    fn cli_설치_상태_문자열을_구성한다() {
        let status = build_cli_install_status("/usr/local/bin/taide", true, Some("/usr/local/bin/taide".to_string()), false);
        assert!(status.installed);
        assert!(!status.dangling);
        assert_eq!(status.editor_env_hint, EDITOR_ENV_HINT);
    }

    #[test]
    fn dangling_상태도_설치_상태_문자열에_반영된다() {
        let status = build_cli_install_status("/usr/local/bin/taide", true, None, true);
        assert!(status.installed);
        assert!(status.dangling);
        assert!(status.resolved_path.is_none());
    }

    #[test]
    fn 앱_번들_경로는_macos_번들_실행으로_판정한다() {
        assert!(is_running_from_macos_app_bundle(Path::new(
            "/Applications/TAIDE.app/Contents/MacOS/TAIDE"
        )));
    }

    #[test]
    fn 이동된_앱_번들_경로도_macos_번들_실행으로_판정한다() {
        assert!(is_running_from_macos_app_bundle(Path::new(
            "/Users/dev/Applications/TAIDE.app/Contents/MacOS/TAIDE"
        )));
    }

    #[test]
    fn dev_빌드_경로는_macos_번들_실행이_아니다() {
        assert!(!is_running_from_macos_app_bundle(Path::new(
            "/Users/dev/TAIDE/src-tauri/target/debug/taide"
        )));
    }

    #[test]
    fn 번들_구조가_비슷해도_확장자가_app이_아니면_거부한다() {
        assert!(!is_running_from_macos_app_bundle(Path::new(
            "/Applications/TAIDE.notapp/Contents/MacOS/TAIDE"
        )));
    }

    #[test]
    fn 번들_실행_파일_기준으로_사이드카_설치_대상_경로를_만든다() {
        let target = resolve_cli_install_target(Path::new("/Applications/TAIDE.app/Contents/MacOS/TAIDE")).unwrap();
        assert_eq!(target, PathBuf::from("/Applications/TAIDE.app/Contents/MacOS/taide-cli"));
    }

    #[test]
    fn dev_빌드에서는_설치_대상_경로가_없다() {
        assert!(resolve_cli_install_target(Path::new("/Users/dev/TAIDE/src-tauri/target/debug/taide")).is_none());
    }

    #[test]
    fn applescript_문자열은_백슬래시와_큰따옴표를_이스케이프한다() {
        assert_eq!(escape_applescript_string(r#"say "hi" \ bye"#), r#"say \"hi\" \\ bye"#);
    }

    #[test]
    fn 셸_단일_인용은_내부_작은따옴표를_안전하게_이스케이프한다() {
        assert_eq!(shell_single_quote("/usr/local/bin/taide"), "'/usr/local/bin/taide'");
        assert_eq!(shell_single_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn 설치_스크립트는_mkdir와_ln_sf를_관리자_권한으로_실행한다() {
        let script = build_cli_install_apple_script(
            Path::new("/Applications/TAIDE.app/Contents/MacOS/taide-cli"),
            Path::new("/usr/local/bin/taide"),
        );
        assert_eq!(
            script,
            "do shell script \"mkdir -p /usr/local/bin && ln -sf '/Applications/TAIDE.app/Contents/MacOS/taide-cli' '/usr/local/bin/taide'\" with administrator privileges"
        );
    }

    #[test]
    fn 제거_스크립트는_rm을_관리자_권한으로_실행한다() {
        let script = build_cli_uninstall_apple_script(Path::new("/usr/local/bin/taide"));
        assert_eq!(
            script,
            "do shell script \"rm '/usr/local/bin/taide'\" with administrator privileges"
        );
    }

    #[test]
    fn osascript_인자는_e_플래그와_스크립트_두_요소뿐이다() {
        let args = build_osascript_args("do shell script \"echo hi\"");
        assert_eq!(args, vec!["-e".to_string(), "do shell script \"echo hi\"".to_string()]);
    }

    #[test]
    fn 사용자_취소는_exit_1과_128_마커로_판정한다() {
        assert!(is_osascript_user_cancelled(Some(1), "execution error: User canceled. (-128)"));
        assert!(!is_osascript_user_cancelled(Some(0), "execution error: User canceled. (-128)"));
        assert!(!is_osascript_user_cancelled(Some(1), "execution error: something else"));
        assert!(!is_osascript_user_cancelled(None, "execution error: User canceled. (-128)"));
    }

    #[test]
    fn 심링크_소유_판정은_파일명이_taide_cli일_때만_참이다() {
        assert!(is_cli_symlink_owned(Path::new("/Applications/TAIDE.app/Contents/MacOS/taide-cli")));
        assert!(!is_cli_symlink_owned(Path::new("/opt/homebrew/bin/some-other-tool")));
    }

    #[test]
    fn 순서만_다르고_내용이_같으면_변경으로_보지_않는다() {
        let a = DetectedAgent {
            session_id: "term-1".to_string(),
            name: "claude".to_string(),
            pid: 1,
            activity: AgentActivity::Unknown,
        };
        let b = DetectedAgent {
            session_id: "term-2".to_string(),
            name: "codex".to_string(),
            pid: 2,
            activity: AgentActivity::Unknown,
        };
        assert!(!agents_changed(&[a.clone(), b.clone()], &[b, a]));
    }

    #[test]
    fn 내용이_다르면_변경으로_본다() {
        let a = DetectedAgent {
            session_id: "term-1".to_string(),
            name: "claude".to_string(),
            pid: 1,
            activity: AgentActivity::Unknown,
        };
        assert!(agents_changed(&[], &[a]));
    }

    #[test]
    fn wait_마커가_포함된_argv를_파싱한다() {
        let argv = vec![
            "/usr/local/bin/taide".to_string(),
            "/tmp/edit.md".to_string(),
            "--wait-marker".to_string(),
            "/tmp/taide-wait-abc123".to_string(),
        ];
        let request = parse_cli_payload(&argv).unwrap();
        assert_eq!(request.path, "/tmp/edit.md");
        assert_eq!(request.wait_marker.as_deref(), Some("/tmp/taide-wait-abc123"));
    }

    #[test]
    fn wait_마커_없이_argv를_파싱한다() {
        let argv = vec!["/usr/local/bin/taide".to_string(), "/tmp/edit.md".to_string()];
        let request = parse_cli_payload(&argv).unwrap();
        assert_eq!(request.path, "/tmp/edit.md");
        assert!(request.wait_marker.is_none());
    }

    #[test]
    fn 파일이_없으면_none을_반환한다() {
        let argv = vec!["/usr/local/bin/taide".to_string()];
        assert!(parse_cli_payload(&argv).is_none());
    }

    fn snapshot(pid: u32, parent_pid: Option<u32>, name: &str, cmdline: &str) -> ProcessSnapshot {
        ProcessSnapshot {
            pid,
            parent_pid,
            name: name.to_string(),
            cmdline: cmdline.to_string(),
        }
    }

    #[test]
    fn 셸의_직계_자식이_에이전트면_감지한다() {
        let processes = vec![snapshot(200, Some(100), "claude", "claude")];
        assert_eq!(find_descendant_agent(&processes, 100), Some((200, "claude")));
    }

    #[test]
    fn 손자_프로세스까지_내려가서_감지한다() {
        let processes = vec![
            snapshot(200, Some(100), "node", "node"),
            snapshot(300, Some(200), "node", "C:\\node.exe C:\\Users\\dev\\.claude\\local\\claude"),
        ];
        assert_eq!(find_descendant_agent(&processes, 100), Some((300, "claude")));
    }

    #[test]
    fn 에이전트가_없으면_none을_반환한다() {
        let processes = vec![
            snapshot(200, Some(100), "powershell", "powershell"),
            snapshot(300, Some(200), "node", "node server.js"),
        ];
        assert_eq!(find_descendant_agent(&processes, 100), None);
    }

    #[test]
    fn 다른_트리에_속한_프로세스는_무시한다() {
        let processes = vec![
            snapshot(200, Some(999), "claude", "claude"),
            snapshot(300, Some(100), "powershell", "powershell"),
        ];
        assert_eq!(find_descendant_agent(&processes, 100), None);
    }

    #[test]
    fn 순환_참조가_있어도_무한루프에_빠지지_않는다() {
        let processes = vec![
            snapshot(200, Some(100), "powershell", "powershell"),
            snapshot(100, Some(200), "powershell", "powershell"),
        ];
        assert_eq!(find_descendant_agent(&processes, 100), None);
    }

    #[test]
    fn windows_exe_확장자를_제거한다() {
        assert_eq!(strip_windows_exe_suffix("claude.exe"), "claude");
        assert_eq!(strip_windows_exe_suffix("claude.EXE"), "claude");
        assert_eq!(strip_windows_exe_suffix("claude"), "claude");
        assert_eq!(strip_windows_exe_suffix(".exe"), ".exe");
    }

    #[test]
    fn r_상태만_활동으로_본다() {
        assert!(is_probe_active(Some('R')));
        assert!(!is_probe_active(Some('S')));
        assert!(!is_probe_active(None));
    }

    #[test]
    fn 활동_직후에는_working_을_유지한다() {
        assert_eq!(classify_activity(AgentActivity::Idle, Some(0)), AgentActivity::Working);
        assert_eq!(
            classify_activity(AgentActivity::Idle, Some(ACTIVITY_WORKING_HOLD_MS - 1)),
            AgentActivity::Working
        );
    }

    #[test]
    fn 충분히_조용하면_idle_로_전이한다() {
        assert_eq!(
            classify_activity(AgentActivity::Working, Some(ACTIVITY_IDLE_QUIET_MS)),
            AgentActivity::Idle
        );
    }

    #[test]
    fn 히스테리시스_구간에서는_직전_상태를_유지한다() {
        let mid = (ACTIVITY_WORKING_HOLD_MS + ACTIVITY_IDLE_QUIET_MS) / 2;
        assert_eq!(classify_activity(AgentActivity::Working, Some(mid)), AgentActivity::Working);
        assert_eq!(classify_activity(AgentActivity::Idle, Some(mid)), AgentActivity::Idle);
    }

    #[test]
    fn 신호가_없으면_unknown_이다() {
        assert_eq!(classify_activity(AgentActivity::Working, None), AgentActivity::Unknown);
    }

    #[test]
    fn 상수시간_비교는_같은_바이트열에서_참이다() {
        assert!(constant_time_eq(b"token-value", b"token-value"));
    }

    #[test]
    fn 상수시간_비교는_다른_바이트열에서_거짓이다() {
        assert!(!constant_time_eq(b"token-value", b"token-other"));
        assert!(!constant_time_eq(b"short", b"longer-token"));
    }

    #[test]
    fn cwd가_프로젝트_루트와_정확히_같으면_매칭한다() {
        let projects = vec![(ProjectId::from("prj-1".to_string()), "/Users/dev/app".to_string())];
        assert_eq!(match_project_by_cwd("/Users/dev/app", &projects), Some(&projects[0].0));
    }

    #[test]
    fn cwd가_프로젝트_루트의_하위_경로면_매칭한다() {
        let projects = vec![(ProjectId::from("prj-1".to_string()), "/Users/dev/app".to_string())];
        assert_eq!(match_project_by_cwd("/Users/dev/app/src", &projects), Some(&projects[0].0));
    }

    #[test]
    fn 중첩된_프로젝트에서는_가장_긴_루트가_이긴다() {
        let projects = vec![
            (ProjectId::from("outer".to_string()), "/Users/dev".to_string()),
            (ProjectId::from("inner".to_string()), "/Users/dev/app".to_string()),
        ];
        assert_eq!(match_project_by_cwd("/Users/dev/app/src", &projects), Some(&projects[1].0));
    }

    #[test]
    fn 어느_루트에도_속하지_않으면_none이다() {
        let projects = vec![(ProjectId::from("prj-1".to_string()), "/Users/dev/app".to_string())];
        assert_eq!(match_project_by_cwd("/tmp/scratch", &projects), None);
    }

    #[test]
    fn claude_hook_이벤트를_활동으로_매핑한다() {
        assert_eq!(
            map_hook_event_to_activity("claude", "UserPromptSubmit"),
            Some(AgentActivity::Working)
        );
        assert_eq!(
            map_hook_event_to_activity("claude", "Notification"),
            Some(AgentActivity::AwaitingInput)
        );
        assert_eq!(map_hook_event_to_activity("claude", "Stop"), Some(AgentActivity::Idle));
        assert_eq!(map_hook_event_to_activity("claude", "PreToolUse"), None);
    }

    #[test]
    fn codex_hook_이벤트를_활동으로_매핑한다() {
        assert_eq!(
            map_hook_event_to_activity("codex", "UserPromptSubmit"),
            Some(AgentActivity::Working)
        );
        assert_eq!(
            map_hook_event_to_activity("codex", "PermissionRequest"),
            Some(AgentActivity::AwaitingInput)
        );
        assert_eq!(map_hook_event_to_activity("codex", "PostToolUse"), Some(AgentActivity::Working));
        assert_eq!(map_hook_event_to_activity("codex", "Stop"), Some(AgentActivity::Idle));
        assert_eq!(map_hook_event_to_activity("codex", "Notification"), None);
    }

    #[test]
    fn gemini_hook_이벤트를_활동으로_매핑한다() {
        assert_eq!(map_hook_event_to_activity("gemini", "BeforeAgent"), Some(AgentActivity::Working));
        assert_eq!(
            map_hook_event_to_activity("gemini", "Notification"),
            Some(AgentActivity::AwaitingInput)
        );
        assert_eq!(map_hook_event_to_activity("gemini", "AfterAgent"), Some(AgentActivity::Idle));
    }

    #[test]
    fn 알려지지_않은_에이전트는_hook_이벤트를_매핑하지_않는다() {
        assert_eq!(map_hook_event_to_activity("unknown", "Stop"), None);
    }

    #[test]
    fn 빈_설정에_hook을_주입하면_관리대상_이벤트_3종이_생긴다() {
        let injected = inject_taide_hook_entries(serde_json::json!({}), "http://127.0.0.1:9999/claude/hook?token=abc&taide=1");
        assert!(has_taide_hook_entries(&injected));
        for event in MANAGED_HOOK_EVENTS {
            assert_eq!(injected["hooks"][event].as_array().unwrap().len(), 1);
        }
    }

    #[test]
    fn 기존_사용자_hook은_보존하고_taide_항목만_주입한다() {
        let existing = serde_json::json!({
            "hooks": {
                "Stop": [{ "hooks": [{ "type": "command", "command": "echo done" }] }]
            }
        });
        let injected = inject_taide_hook_entries(existing, "http://127.0.0.1:9999/claude/hook?token=abc&taide=1");
        let stop_entries = injected["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop_entries.len(), 2);
    }

    #[test]
    fn 두_번_주입해도_taide_항목이_중복되지_않는다() {
        let once = inject_taide_hook_entries(serde_json::json!({}), "http://127.0.0.1:9999/claude/hook?token=abc&taide=1");
        let twice = inject_taide_hook_entries(once, "http://127.0.0.1:10000/claude/hook?token=xyz&taide=1");
        for event in MANAGED_HOOK_EVENTS {
            assert_eq!(twice["hooks"][event].as_array().unwrap().len(), 1);
        }
    }

    #[test]
    fn hooks_값이_객체가_아니어도_패닉없이_주입한다() {
        let broken = serde_json::json!({ "hooks": "disabled" });
        let injected = inject_taide_hook_entries(broken, "http://127.0.0.1:9999/claude/hook?token=abc&taide=1");
        assert!(has_taide_hook_entries(&injected));
    }

    #[test]
    fn hook_이벤트_값이_배열이_아니어도_패닉없이_주입한다() {
        let broken = serde_json::json!({ "hooks": { "Stop": {}, "Notification": 3 } });
        let injected = inject_taide_hook_entries(broken, "http://127.0.0.1:9999/claude/hook?token=abc&taide=1");
        for event in MANAGED_HOOK_EVENTS {
            assert_eq!(injected["hooks"][event].as_array().unwrap().len(), 1);
        }
    }

    #[test]
    fn 기록된_hook_url이_현재_서버와_다르면_재주입_대상이다() {
        let url = "http://127.0.0.1:9999/claude/hook?token=abc&taide=1";
        let injected = inject_taide_hook_entries(serde_json::json!({}), url);
        assert!(has_hook_entries_for_url(&injected, url));
        assert!(!has_hook_entries_for_url(
            &injected,
            "http://127.0.0.1:10000/claude/hook?token=xyz&taide=1"
        ));
    }

    #[test]
    fn 기록된_command_hook이_현재_커맨드와_다르면_재주입_대상이다() {
        let injected = inject_taide_command_hook_entries(serde_json::json!({}), CODEX_MANAGED_HOOK_EVENTS, &codex_command(), 5);
        assert!(has_command_hook_entries_for_command(
            &injected,
            CODEX_MANAGED_HOOK_EVENTS,
            &codex_command()
        ));
        let stale_command = build_command_hook_shell_command(
            "/usr/local/bin/taide",
            "http://127.0.0.1:10000/claude/hook?token=xyz&agent=codex&taide=1",
        );
        assert!(!has_command_hook_entries_for_command(
            &injected,
            CODEX_MANAGED_HOOK_EVENTS,
            &stale_command
        ));
    }

    #[test]
    fn hook_override는_알려진_에이전트_3종에만_적용된다() {
        assert!(is_hook_managed_agent("claude"));
        assert!(is_hook_managed_agent("codex"));
        assert!(is_hook_managed_agent("gemini"));
        assert!(!is_hook_managed_agent("bash"));
    }

    #[test]
    fn claude는_프로젝트_스코프이고_codex_gemini는_사용자_스코프다() {
        assert_eq!(hook_scope_for_agent("claude").unwrap(), HookInstallScope::Project);
        assert_eq!(hook_scope_for_agent("codex").unwrap(), HookInstallScope::User);
        assert_eq!(hook_scope_for_agent("gemini").unwrap(), HookInstallScope::User);
        assert!(hook_scope_for_agent("bash").is_err());
    }

    #[test]
    fn 사용자_레벨_hooks_경로를_홈_디렉토리_기준으로_구성한다() {
        assert_eq!(
            user_level_hooks_path("codex", Some("/Users/dev")).unwrap(),
            PathBuf::from("/Users/dev/.codex/hooks.json")
        );
        assert_eq!(
            user_level_hooks_path("gemini", Some("/Users/dev")).unwrap(),
            PathBuf::from("/Users/dev/.gemini/settings.json")
        );
        assert!(user_level_hooks_path("claude", Some("/Users/dev")).is_err());
        assert!(user_level_hooks_path("codex", None).is_err());
    }

    #[test]
    fn 문서_어디에_있어도_taide_마커를_찾는다() {
        let nested = serde_json::json!({
            "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "taide-cli hook --url http://127.0.0.1:9999/claude/hook?token=abc&agent=codex&taide=1" }] }] }
        });
        assert!(has_taide_marker_anywhere(&nested));
        assert!(!has_taide_marker_anywhere(&serde_json::json!({ "hooks": {} })));
    }

    #[test]
    fn taide_hook을_제거하면_사용자_hook만_남는다() {
        let existing = serde_json::json!({
            "hooks": {
                "Stop": [
                    { "hooks": [{ "type": "command", "command": "echo done" }] },
                    { "hooks": [{ "type": "http", "url": "http://127.0.0.1:9999/claude/hook?token=abc&taide=1" }] }
                ]
            }
        });
        let removed = remove_taide_hook_entries(existing);
        let stop_entries = removed["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop_entries.len(), 1);
        assert!(!has_taide_hook_entries(&removed));
    }

    #[test]
    fn taide_hook만_있었다면_제거_후_hooks_키_자체가_사라진다() {
        let injected = inject_taide_hook_entries(serde_json::json!({}), "http://127.0.0.1:9999/claude/hook?token=abc&taide=1");
        let removed = remove_taide_hook_entries(injected);
        assert!(removed.get("hooks").is_none());
    }

    #[test]
    fn 에이전트별_관리대상_이벤트_목록이_다르다() {
        assert_eq!(managed_hook_events_for("claude"), MANAGED_HOOK_EVENTS);
        assert_eq!(managed_hook_events_for("codex"), CODEX_MANAGED_HOOK_EVENTS);
        assert_eq!(managed_hook_events_for("gemini"), GEMINI_MANAGED_HOOK_EVENTS);
        assert!(managed_hook_events_for("bash").is_empty());
    }

    #[test]
    fn 에이전트별_command_hook_timeout_단위가_다르다() {
        assert_eq!(user_level_hook_command_timeout("codex"), CODEX_HOOK_COMMAND_TIMEOUT_SECONDS);
        assert_eq!(user_level_hook_command_timeout("gemini"), GEMINI_HOOK_COMMAND_TIMEOUT_MS);
        assert_eq!(
            user_level_hook_command_timeout("gemini"),
            CODEX_HOOK_COMMAND_TIMEOUT_SECONDS * 1_000
        );
    }

    #[test]
    fn 쉘_커맨드는_바이너리_경로와_url을_각각_큰따옴표로_감싼다() {
        let command = build_command_hook_shell_command(
            "/usr/local/bin/taide",
            "http://127.0.0.1:9999/claude/hook?token=abc&agent=codex&taide=1",
        );
        assert_eq!(
            command,
            "\"/usr/local/bin/taide\" hook --url \"http://127.0.0.1:9999/claude/hook?token=abc&agent=codex&taide=1\""
        );
    }

    fn codex_hook_url() -> &'static str {
        "http://127.0.0.1:9999/claude/hook?token=abc&agent=codex&taide=1"
    }

    fn codex_command() -> String {
        build_command_hook_shell_command("/usr/local/bin/taide", codex_hook_url())
    }

    #[test]
    fn codex_빈_설정에_command_hook을_주입하면_관리대상_이벤트_4종이_생긴다() {
        let injected = inject_taide_command_hook_entries(
            serde_json::json!({}),
            CODEX_MANAGED_HOOK_EVENTS,
            &codex_command(),
            CODEX_HOOK_COMMAND_TIMEOUT_SECONDS,
        );
        for event in CODEX_MANAGED_HOOK_EVENTS {
            let entries = injected["hooks"][event].as_array().expect("event entries");
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0]["hooks"][0]["type"], HOOK_HANDLER_TYPE_COMMAND);
            assert_eq!(entries[0]["hooks"][0]["command"], codex_command());
        }
    }

    #[test]
    fn codex_기존_사용자_hook은_보존하고_taide_command_항목만_주입한다() {
        let existing = serde_json::json!({
            "hooks": {
                "Stop": [{ "hooks": [{ "type": "command", "command": "echo done" }] }]
            }
        });
        let injected = inject_taide_command_hook_entries(existing, CODEX_MANAGED_HOOK_EVENTS, &codex_command(), 5);
        let stop_entries = injected["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop_entries.len(), 2);
        assert_eq!(stop_entries[0]["hooks"][0]["command"], "echo done");
    }

    #[test]
    fn codex_두_번_주입해도_taide_command_항목이_중복되지_않는다() {
        let once = inject_taide_command_hook_entries(serde_json::json!({}), CODEX_MANAGED_HOOK_EVENTS, &codex_command(), 5);
        let other_command = build_command_hook_shell_command(
            "/usr/local/bin/taide",
            "http://127.0.0.1:10000/claude/hook?token=xyz&agent=codex&taide=1",
        );
        let twice = inject_taide_command_hook_entries(once, CODEX_MANAGED_HOOK_EVENTS, &other_command, 5);
        for event in CODEX_MANAGED_HOOK_EVENTS {
            let entries = twice["hooks"][event].as_array().unwrap();
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0]["hooks"][0]["command"], other_command);
        }
    }

    #[test]
    fn codex_taide_command_hook을_제거하면_사용자_hook만_남고_제거_대상이_없으면_hooks_키가_사라진다() {
        let injected = inject_taide_command_hook_entries(serde_json::json!({}), CODEX_MANAGED_HOOK_EVENTS, &codex_command(), 5);
        let removed = remove_taide_command_hook_entries(injected, CODEX_MANAGED_HOOK_EVENTS);
        assert!(removed.get("hooks").is_none());

        let existing = serde_json::json!({
            "hooks": {
                "Stop": [
                    { "hooks": [{ "type": "command", "command": "echo done" }] },
                    { "hooks": [{ "type": "command", "command": codex_command() }] }
                ]
            }
        });
        let injected_with_user_hook = inject_taide_command_hook_entries(existing, CODEX_MANAGED_HOOK_EVENTS, &codex_command(), 5);
        let removed = remove_taide_command_hook_entries(injected_with_user_hook, CODEX_MANAGED_HOOK_EVENTS);
        let stop_entries = removed["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop_entries.len(), 1);
        assert_eq!(stop_entries[0]["hooks"][0]["command"], "echo done");
    }

    fn gemini_hook_url() -> &'static str {
        "http://127.0.0.1:9999/claude/hook?token=abc&agent=gemini&taide=1"
    }

    fn gemini_command() -> String {
        build_command_hook_shell_command("/usr/local/bin/taide", gemini_hook_url())
    }

    #[test]
    fn gemini_빈_설정에_command_hook을_주입하면_관리대상_이벤트_3종이_생긴다() {
        let injected = inject_taide_command_hook_entries(
            serde_json::json!({}),
            GEMINI_MANAGED_HOOK_EVENTS,
            &gemini_command(),
            GEMINI_HOOK_COMMAND_TIMEOUT_MS,
        );
        for event in GEMINI_MANAGED_HOOK_EVENTS {
            let entries = injected["hooks"][event].as_array().expect("event entries");
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0]["hooks"][0]["command"], gemini_command());
            assert_eq!(entries[0]["hooks"][0]["timeout"], GEMINI_HOOK_COMMAND_TIMEOUT_MS);
        }
    }

    #[test]
    fn gemini_두_번_주입해도_taide_command_항목이_중복되지_않고_다른_에이전트_이벤트와_섞이지_않는다() {
        let with_codex = inject_taide_command_hook_entries(serde_json::json!({}), CODEX_MANAGED_HOOK_EVENTS, &codex_command(), 5);
        let with_both = inject_taide_command_hook_entries(with_codex, GEMINI_MANAGED_HOOK_EVENTS, &gemini_command(), 5_000);

        assert_eq!(with_both["hooks"][HOOK_EVENT_STOP].as_array().unwrap().len(), 1);
        assert!(with_both["hooks"][HOOK_EVENT_BEFORE_AGENT].as_array().unwrap()[0]["hooks"][0]["command"] == gemini_command());
        assert!(with_both["hooks"][HOOK_EVENT_PERMISSION_REQUEST].as_array().unwrap()[0]["hooks"][0]["command"] == codex_command());
    }

    #[test]
    fn gemini_taide_command_hook을_제거해도_codex_항목은_그대로다() {
        let with_codex = inject_taide_command_hook_entries(serde_json::json!({}), CODEX_MANAGED_HOOK_EVENTS, &codex_command(), 5);
        let with_both = inject_taide_command_hook_entries(with_codex, GEMINI_MANAGED_HOOK_EVENTS, &gemini_command(), 5_000);

        let gemini_removed = remove_taide_command_hook_entries(with_both, GEMINI_MANAGED_HOOK_EVENTS);
        assert!(gemini_removed["hooks"].get(HOOK_EVENT_BEFORE_AGENT).is_none());
        assert_eq!(
            gemini_removed["hooks"][HOOK_EVENT_STOP].as_array().unwrap()[0]["hooks"][0]["command"],
            codex_command()
        );
    }
}

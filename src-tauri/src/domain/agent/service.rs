use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::types::{
    AgentActivity, CliInstallStatus, DetectedAgent, ExternalOpenRequest, ACTIVITY_IDLE_QUIET_MS, ACTIVITY_WORKING_HOLD_MS,
    HOOKS_HTTP_TIMEOUT_SECONDS, HOOKS_URL_MARKER, HOOK_EVENT_NOTIFICATION, HOOK_EVENT_STOP, HOOK_EVENT_USER_PROMPT_SUBMIT,
    KNOWN_AGENT_NAMES, MANAGED_HOOK_EVENTS, WAIT_MARKER_PREFIX,
};
use crate::error::{AppError, AppResult};
use crate::ids::ProjectId;

const LINUX_COMM_MAX_LEN: usize = 15;
const NODE_RUNTIME_NAMES: &[&str] = &["node", "bun", "deno"];
const EDITOR_ENV_HINT: &str = "export EDITOR=\"taide --wait\"";
const CLI_WAIT_MARKER_FLAG: &str = "--wait-marker";

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

pub fn build_cli_install_status(target_path: &str, installed: bool, resolved_path: Option<String>) -> CliInstallStatus {
    CliInstallStatus {
        installed,
        resolved_path,
        target_path: target_path.to_string(),
        editor_env_hint: EDITOR_ENV_HINT.to_string(),
    }
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

/// 프로세스가 지금 이 순간 CPU 위에서 실행 중인지("R" 상태)만을 활동 신호로 본다.
/// sleeping 상태는 "API 응답 대기"·"사용자 입력 대기" 둘 다에서 관측되어 구분할 수 없으므로
/// 활동 없음으로 취급한다(risk: agent-status-badge.md risks §2).
pub fn is_probe_active(state: Option<char>) -> bool {
    matches!(state, Some('R'))
}

/// 활동 신호가 마지막으로 관측된 시점(ms)을 바탕으로 idle/working 을 판정한다.
/// 순수 함수로 두어 실제 타이머 없이 테스트 가능하게 한다.
pub fn classify_activity(previous: AgentActivity, ms_since_active: Option<u64>) -> AgentActivity {
    match ms_since_active {
        None => AgentActivity::Unknown,
        Some(elapsed) if elapsed < ACTIVITY_WORKING_HOLD_MS => AgentActivity::Working,
        Some(elapsed) if elapsed >= ACTIVITY_IDLE_QUIET_MS => AgentActivity::Idle,
        Some(_) => previous,
    }
}

pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn is_cwd_within_root(cwd: &str, root: &str) -> bool {
    let cwd = cwd.trim_end_matches('/');
    let root = root.trim_end_matches('/');
    cwd == root || cwd.starts_with(&format!("{root}/"))
}

/// cwd 가 속한 프로젝트를 루트 prefix 매칭으로 찾는다. 중첩 프로젝트에서는 가장 긴(가장 구체적인)
/// 루트를 우선한다.
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

pub fn map_hook_event_to_activity(event_name: &str) -> Option<AgentActivity> {
    match event_name {
        HOOK_EVENT_USER_PROMPT_SUBMIT => Some(AgentActivity::Working),
        HOOK_EVENT_NOTIFICATION => Some(AgentActivity::AwaitingInput),
        HOOK_EVENT_STOP => Some(AgentActivity::Idle),
        _ => None,
    }
}

fn is_taide_managed_entry(entry: &serde_json::Value) -> bool {
    entry
        .get("hooks")
        .and_then(|hooks| hooks.as_array())
        .map(|hooks| {
            hooks.iter().any(|hook| {
                hook.get("url")
                    .and_then(|url| url.as_str())
                    .is_some_and(|url| url.contains(HOOKS_URL_MARKER))
            })
        })
        .unwrap_or(false)
}

/// TAIDE 가 주입한 hook 항목만 골라 제거한다. 사용자가 직접 추가한 다른 hook 은 그대로 둔다.
pub fn remove_taide_hook_entries(mut root: serde_json::Value) -> serde_json::Value {
    if let Some(hooks_obj) = root.get_mut("hooks").and_then(|value| value.as_object_mut()) {
        for event in MANAGED_HOOK_EVENTS {
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

/// 기존 설정을 보존한 채(병합) TAIDE hook 항목을 주입한다. 이미 주입된 항목은 먼저 제거해 멱등하게 만든다.
pub fn inject_taide_hook_entries(root: serde_json::Value, hook_url: &str) -> serde_json::Value {
    let root = remove_taide_hook_entries(root);
    let mut root = if root.is_object() { root } else { serde_json::json!({}) };

    let hooks_obj = root
        .as_object_mut()
        .expect("root normalized to object above")
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .expect("hooks value normalized to object");

    for event in MANAGED_HOOK_EVENTS {
        let entries = hooks_obj
            .entry(event.to_string())
            .or_insert_with(|| serde_json::json!([]))
            .as_array_mut()
            .expect("hook event value normalized to array");
        entries.push(serde_json::json!({
            "hooks": [{ "type": "http", "url": hook_url, "timeout": HOOKS_HTTP_TIMEOUT_SECONDS }]
        }));
    }

    root
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
        let status = build_cli_install_status("/usr/local/bin/taide", true, Some("/usr/local/bin/taide".to_string()));
        assert!(status.installed);
        assert_eq!(status.editor_env_hint, EDITOR_ENV_HINT);
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
    fn hook_이벤트를_활동으로_매핑한다() {
        assert_eq!(map_hook_event_to_activity("UserPromptSubmit"), Some(AgentActivity::Working));
        assert_eq!(map_hook_event_to_activity("Notification"), Some(AgentActivity::AwaitingInput));
        assert_eq!(map_hook_event_to_activity("Stop"), Some(AgentActivity::Idle));
        assert_eq!(map_hook_event_to_activity("PreToolUse"), None);
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
}

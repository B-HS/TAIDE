use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::types::{CliInstallStatus, DetectedAgent, ExternalOpenRequest, KNOWN_AGENT_NAMES, WAIT_MARKER_PREFIX};
use crate::error::{AppError, AppResult};

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
        };
        let b = DetectedAgent {
            session_id: "term-2".to_string(),
            name: "codex".to_string(),
            pid: 2,
        };
        assert!(!agents_changed(&[a.clone(), b.clone()], &[b, a]));
    }

    #[test]
    fn 내용이_다르면_변경으로_본다() {
        let a = DetectedAgent {
            session_id: "term-1".to_string(),
            name: "claude".to_string(),
            pid: 1,
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
}

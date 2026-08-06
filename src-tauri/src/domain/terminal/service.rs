use std::path::{Path, PathBuf};

use super::types::ShellProfile;
use crate::error::{AppError, AppResult};

pub fn ring_buffer_append(buffer: &mut Vec<u8>, incoming: &[u8], capacity: usize) {
    buffer.extend_from_slice(incoming);
    if buffer.len() > capacity {
        let overflow = buffer.len() - capacity;
        buffer.drain(0..overflow);
    }
}

pub fn parse_shell_list(content: &str) -> Vec<String> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect()
}

fn shell_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
const HOMEBREW_CANDIDATE_SHELLS: &[&str] = &[
    "/opt/homebrew/bin/fish",
    "/opt/homebrew/bin/zsh",
    "/opt/homebrew/bin/nu",
    "/usr/local/bin/fish",
    "/usr/local/bin/zsh",
    "/usr/local/bin/nu",
];

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub fn list_shell_profiles() -> Vec<ShellProfile> {
    let mut seen = std::collections::BTreeSet::new();
    let mut profiles = Vec::new();

    if let Ok(content) = std::fs::read_to_string("/etc/shells") {
        for path in parse_shell_list(&content) {
            if seen.insert(path.clone()) {
                profiles.push(ShellProfile {
                    id: path.clone(),
                    name: shell_name(&path),
                    path,
                    args: Vec::new(),
                });
            }
        }
    }

    #[cfg(target_os = "macos")]
    for candidate in HOMEBREW_CANDIDATE_SHELLS {
        if seen.contains(*candidate) {
            continue;
        }
        if Path::new(candidate).exists() {
            seen.insert((*candidate).to_string());
            profiles.push(ShellProfile {
                id: (*candidate).to_string(),
                name: shell_name(candidate),
                path: (*candidate).to_string(),
                args: Vec::new(),
            });
        }
    }

    profiles
}

#[cfg(target_os = "windows")]
pub fn list_shell_profiles() -> Vec<ShellProfile> {
    let comspec = std::env::var("ComSpec").unwrap_or_else(|_| "C:\\Windows\\System32\\cmd.exe".to_string());

    vec![ShellProfile {
        id: "cmd".to_string(),
        name: "Command Prompt".to_string(),
        path: comspec,
        args: Vec::new(),
    }]
}

pub fn parse_terminal_path(input: &str) -> (String, Option<u32>, Option<u32>) {
    let trimmed = input.trim();
    let segments: Vec<&str> = trimmed.split(':').collect();

    let mut path_end = segments.len();
    let mut numbers: Vec<u32> = Vec::new();

    while path_end > 1 && numbers.len() < 2 {
        match segments[path_end - 1].parse::<u32>() {
            Ok(value) => {
                numbers.push(value);
                path_end -= 1;
            }
            Err(_) => break,
        }
    }

    let path = segments[..path_end].join(":");
    numbers.reverse();

    (path, numbers.first().copied(), numbers.get(1).copied())
}

fn expand_home(path: &str) -> String {
    if let Some(rest) = path.strip_prefix('~') {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}{rest}");
        }
    }
    path.to_string()
}

pub fn resolve_terminal_path(raw: &str, cwd: &str) -> AppResult<String> {
    let (path_part, _line, _col) = parse_terminal_path(raw);
    let expanded = expand_home(&path_part);
    let candidate = PathBuf::from(&expanded);

    let resolved = if candidate.is_absolute() {
        candidate
    } else {
        PathBuf::from(cwd).join(candidate)
    };

    let canonical = resolved
        .canonicalize()
        .map_err(|_| AppError::NotFound(format!("path not found: {raw}")))?;

    Ok(canonical.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 용량을_초과하면_앞부분을_잘라낸다() {
        let mut buffer = vec![1u8, 2, 3, 4, 5];
        ring_buffer_append(&mut buffer, &[6, 7], 5);
        assert_eq!(buffer, vec![3, 4, 5, 6, 7]);
    }

    #[test]
    fn 용량_이내면_전부_보존한다() {
        let mut buffer = vec![1u8, 2];
        ring_buffer_append(&mut buffer, &[3, 4], 10);
        assert_eq!(buffer, vec![1, 2, 3, 4]);
    }

    #[test]
    fn 주석과_빈줄은_셸_목록에서_제외된다() {
        let content = "/bin/bash\n# comment\n\n  /bin/zsh  \n";
        assert_eq!(parse_shell_list(content), vec!["/bin/bash".to_string(), "/bin/zsh".to_string()]);
    }

    #[test]
    fn 경로_라인_컬럼을_전부_해석한다() {
        assert_eq!(
            parse_terminal_path("src/main.rs:42:10"),
            ("src/main.rs".to_string(), Some(42), Some(10))
        );
    }

    #[test]
    fn 라인만_있으면_컬럼은_없다() {
        assert_eq!(parse_terminal_path("src/main.rs:42"), ("src/main.rs".to_string(), Some(42), None));
    }

    #[test]
    fn 콜론이_없으면_경로_그대로다() {
        assert_eq!(parse_terminal_path("src/main.rs"), ("src/main.rs".to_string(), None, None));
    }

    #[test]
    fn 윈도우_드라이브_콜론은_경로에_보존된다() {
        assert_eq!(
            parse_terminal_path("C:\\foo\\bar.rs:5"),
            ("C:\\foo\\bar.rs".to_string(), Some(5), None)
        );
    }

    #[test]
    fn 존재하지_않는_경로는_찾을_수_없음_에러다() {
        let result = resolve_terminal_path("does/not/exist.rs", "/tmp");
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use super::types::ShellProfile;
use crate::error::{AppError, AppResult};

/// A pty session's recent output, capped at `capacity` bytes with the oldest bytes evicted first —
/// what `pty_attach` replays so a subscriber joining late still sees the session's scrollback.
///
/// Backed by a `VecDeque` rather than the `Vec` + `drain(0..overflow)` this used to be: once the
/// buffer sat at its `DEFAULT_SCROLLBACK_BYTES` (2MB) cap, dropping the overflow shifted every
/// retained byte down, so a busy session paid a ~2MB `memmove` for *each* 64KB read the pty
/// delivered (§2 M-5). A `VecDeque` drops the same bytes by advancing its head, making the cost
/// proportional to the incoming chunk instead of to the retained scrollback.
pub struct ScrollbackRing {
    buffer: VecDeque<u8>,
    capacity: usize,
}

impl ScrollbackRing {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::new(),
            capacity,
        }
    }

    /// Appends `incoming`, evicting whatever no longer fits. A chunk at least as large as the whole
    /// capacity is truncated to its tail *before* being stored rather than appended and then
    /// evicted, so the buffer never transiently grows past its cap.
    pub fn append(&mut self, incoming: &[u8]) {
        if incoming.len() >= self.capacity {
            self.buffer.clear();
            self.buffer.extend(&incoming[incoming.len() - self.capacity..]);
            return;
        }

        self.buffer.extend(incoming);
        let overflow = self.buffer.len().saturating_sub(self.capacity);
        if overflow > 0 {
            self.buffer.drain(..overflow);
        }
    }

    /// The retained bytes in order, as the ring's two halves — a replay must send both, front
    /// first. The second half is empty until the buffer wraps, so an attach before the cap is
    /// reached still replays as a single chunk. Splitting there is safe for the consumer: xterm's
    /// parser preserves its state across `write` boundaries, exactly as it already must for the
    /// reader thread's own output batching.
    pub fn as_slices(&self) -> (&[u8], &[u8]) {
        self.buffer.as_slices()
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

    fn replayed(ring: &ScrollbackRing) -> Vec<u8> {
        let (front, back) = ring.as_slices();
        [front, back].concat()
    }

    #[test]
    fn 용량을_초과하면_앞부분을_잘라낸다() {
        let mut ring = ScrollbackRing::new(5);
        ring.append(&[1, 2, 3, 4, 5]);
        ring.append(&[6, 7]);
        assert_eq!(replayed(&ring), vec![3, 4, 5, 6, 7]);
    }

    #[test]
    fn 용량_이내면_전부_보존한다() {
        let mut ring = ScrollbackRing::new(10);
        ring.append(&[1, 2]);
        ring.append(&[3, 4]);
        assert_eq!(replayed(&ring), vec![1, 2, 3, 4]);
    }

    #[test]
    fn 용량보다_큰_청크는_꼬리만_남긴다() {
        let mut ring = ScrollbackRing::new(3);
        ring.append(&[9, 9]);
        ring.append(&[1, 2, 3, 4, 5]);
        assert_eq!(replayed(&ring), vec![3, 4, 5]);
    }

    #[test]
    fn 되감긴_링도_두_조각을_이어_붙이면_순서가_보존된다() {
        const CAPACITY: usize = 64;
        let mut ring = ScrollbackRing::new(CAPACITY);
        for round in 0..16u8 {
            ring.append(&[round; 13]);
        }

        let written: Vec<u8> = (0..16u8).flat_map(|round| vec![round; 13]).collect();
        assert_eq!(replayed(&ring), written[written.len() - CAPACITY..]);
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

    #[test]
    fn 앞뒤_공백만_있는_줄과_인라인_해시는_셸_목록에서_다르게_다뤄진다() {
        let content = "   \n\t\n /bin/dash \n/bin/sh # not a comment\n";

        assert_eq!(
            parse_shell_list(content),
            vec!["/bin/dash".to_string(), "/bin/sh # not a comment".to_string()],
            "주석 판정은 줄 맨 앞의 # 뿐이라 경로 뒤의 #는 경로의 일부로 남습니다"
        );
    }

    /// Three or more trailing numbers: only the last two are read as line/column, and the rest stay
    /// in the path — a `path:1:2:3` pasted from a stack trace must not silently drop `:1`.
    #[test]
    fn 숫자_꼬리는_최대_둘까지만_행과_열로_읽는다() {
        assert_eq!(parse_terminal_path("a:1:2:3"), ("a:1".to_string(), Some(2), Some(3)));
    }

    /// The `path_end > 1` guard: an input that is nothing but a number is a path, not a bare line
    /// number, so the first segment is never consumed.
    #[test]
    fn 숫자만_있는_입력은_경로로_남는다() {
        assert_eq!(parse_terminal_path("42"), ("42".to_string(), None, None));
    }

    #[test]
    fn 앞뒤_공백은_경로_해석_전에_잘린다() {
        assert_eq!(parse_terminal_path("  src/main.rs:7  "), ("src/main.rs".to_string(), Some(7), None));
    }

    #[test]
    fn 음수나_소수_꼬리는_행_열로_읽히지_않는다() {
        assert_eq!(parse_terminal_path("a:-1"), ("a:-1".to_string(), None, None));
        assert_eq!(parse_terminal_path("a:1.5"), ("a:1.5".to_string(), None, None));
    }

    fn resolve_fixture(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("taide-terminal-resolve-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src").join("main.rs"), b"fn main() {}").unwrap();
        dir
    }

    #[test]
    fn 상대_경로는_cwd_를_기준으로_해석되고_행_열_꼬리는_버려진다() {
        let dir = resolve_fixture("relative");
        let expected = std::fs::canonicalize(dir.join("src").join("main.rs")).unwrap();

        let resolved = resolve_terminal_path("src/main.rs:12:3", &dir.to_string_lossy()).expect("cwd 기준으로 찾아야 합니다");

        assert_eq!(resolved, expected.to_string_lossy());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 절대_경로는_cwd_와_무관하게_해석된다() {
        let dir = resolve_fixture("absolute");
        let absolute = dir.join("src").join("main.rs");
        let expected = std::fs::canonicalize(&absolute).unwrap();

        let resolved = resolve_terminal_path(&absolute.to_string_lossy(), "/nonexistent-cwd").expect("절대 경로는 cwd 를 보지 않습니다");

        assert_eq!(resolved, expected.to_string_lossy());

        std::fs::remove_dir_all(&dir).ok();
    }
}

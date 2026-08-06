use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use uuid::Uuid;

const WAIT_MARKER_PREFIX: &str = "taide-wait-";
const DEFAULT_TIMEOUT_SECS: u64 = 1_800;
const POLL_INTERVAL_MS: u64 = 300;
const APP_PATH_ENV_VAR: &str = "TAIDE_APP_PATH";
const MACOS_APP_BUNDLE_PATH: &str = "/Applications/TAIDE.app/Contents/MacOS/TAIDE";
const DEV_APP_RELATIVE_PATH: &str = "target/debug/taide";
const WAIT_MARKER_FLAG: &str = "--wait-marker";

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedArgs {
    wait: bool,
    timeout_secs: u64,
    files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ArgsError {
    NoFiles,
    MissingTimeoutValue,
    InvalidTimeoutValue(String),
}

impl std::fmt::Display for ArgsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgsError::NoFiles => write!(f, "no file arguments given"),
            ArgsError::MissingTimeoutValue => write!(f, "--timeout requires a value"),
            ArgsError::InvalidTimeoutValue(value) => write!(f, "invalid --timeout value: {value}"),
        }
    }
}

fn parse_args(args: &[String]) -> Result<ParsedArgs, ArgsError> {
    let mut wait = false;
    let mut timeout_secs = DEFAULT_TIMEOUT_SECS;
    let mut files = Vec::new();

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--wait" | "-w" => wait = true,
            "--timeout" => {
                let value = iter.next().ok_or(ArgsError::MissingTimeoutValue)?;
                timeout_secs = value
                    .parse::<u64>()
                    .map_err(|_| ArgsError::InvalidTimeoutValue(value.clone()))?;
            }
            other => files.push(other.to_string()),
        }
    }

    if files.is_empty() {
        return Err(ArgsError::NoFiles);
    }

    Ok(ParsedArgs {
        wait,
        timeout_secs,
        files,
    })
}

fn normalize_absolute(file: &str) -> std::io::Result<PathBuf> {
    let path = Path::new(file);
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    Ok(env::current_dir()?.join(path))
}

fn build_wait_marker_path(temp_dir: &Path, id: Uuid) -> PathBuf {
    temp_dir.join(format!("{WAIT_MARKER_PREFIX}{id}"))
}

fn resolve_app_path() -> Option<PathBuf> {
    if let Ok(value) = env::var(APP_PATH_ENV_VAR) {
        let path = PathBuf::from(value);
        if path.exists() {
            return Some(path);
        }
    }

    let macos_bundle = PathBuf::from(MACOS_APP_BUNDLE_PATH);
    if macos_bundle.exists() {
        return Some(macos_bundle);
    }

    if let Ok(exe) = env::current_exe() {
        let mut dir = exe.parent().map(Path::to_path_buf);
        while let Some(current) = dir {
            let candidate = current.join(DEV_APP_RELATIVE_PATH);
            if candidate.exists() {
                return Some(candidate);
            }
            dir = current.parent().map(Path::to_path_buf);
        }
    }

    None
}

fn spawn_app(
    app_path: &Path,
    files: &[PathBuf],
    wait_marker: Option<&Path>,
) -> std::io::Result<()> {
    let mut command = Command::new(app_path);
    command.args(files.iter().map(|path| path.as_os_str()));
    if let Some(marker) = wait_marker {
        command.arg(WAIT_MARKER_FLAG).arg(marker.as_os_str());
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command.spawn()?;
    Ok(())
}

fn block_until_marker_removed(marker: &Path, timeout: Duration) -> bool {
    let started = Instant::now();
    while marker.exists() {
        if started.elapsed() >= timeout {
            return false;
        }
        sleep(Duration::from_millis(POLL_INTERVAL_MS));
    }
    true
}

fn run() -> i32 {
    let raw_args: Vec<String> = env::args().skip(1).collect();

    let parsed = match parse_args(&raw_args) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("taide: {error}");
            eprintln!("usage: taide [--wait|-w] [--timeout <secs>] <file> [<file>...]");
            return 1;
        }
    };

    let mut absolute_files = Vec::with_capacity(parsed.files.len());
    for file in &parsed.files {
        match normalize_absolute(file) {
            Ok(path) => absolute_files.push(path),
            Err(error) => {
                eprintln!("taide: failed to resolve path {file}: {error}");
                return 1;
            }
        }
    }

    let Some(app_path) = resolve_app_path() else {
        eprintln!(
            "taide: could not locate the TAIDE application (set {APP_PATH_ENV_VAR} to override)"
        );
        return 1;
    };

    let marker_path = if parsed.wait {
        Some(build_wait_marker_path(&env::temp_dir(), Uuid::new_v4()))
    } else {
        None
    };

    if let Some(marker) = &marker_path {
        if let Err(error) = fs::write(marker, b"") {
            eprintln!(
                "taide: failed to create wait marker {}: {error}",
                marker.display()
            );
            return 1;
        }
    }

    if let Err(error) = spawn_app(&app_path, &absolute_files, marker_path.as_deref()) {
        eprintln!(
            "taide: failed to launch TAIDE at {}: {error}",
            app_path.display()
        );
        if let Some(marker) = &marker_path {
            let _ = fs::remove_file(marker);
        }
        return 1;
    }

    if let Some(marker) = &marker_path {
        let timeout = Duration::from_secs(parsed.timeout_secs);
        if !block_until_marker_removed(marker, timeout) {
            eprintln!(
                "taide: timed out after {}s waiting for the editor to close",
                parsed.timeout_secs
            );
            let _ = fs::remove_file(marker);
            return 1;
        }
    }

    0
}

fn main() {
    std::process::exit(run());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 파일_하나만_주면_wait_없이_파싱된다() {
        let args = vec!["file.txt".to_string()];
        let parsed = parse_args(&args).unwrap();
        assert!(!parsed.wait);
        assert_eq!(parsed.files, vec!["file.txt".to_string()]);
        assert_eq!(parsed.timeout_secs, DEFAULT_TIMEOUT_SECS);
    }

    #[test]
    fn wait_플래그와_다중_파일을_파싱한다() {
        let args = vec![
            "--wait".to_string(),
            "a.txt".to_string(),
            "b.txt".to_string(),
        ];
        let parsed = parse_args(&args).unwrap();
        assert!(parsed.wait);
        assert_eq!(parsed.files, vec!["a.txt".to_string(), "b.txt".to_string()]);
    }

    #[test]
    fn 짧은_wait_플래그도_인식한다() {
        let args = vec!["-w".to_string(), "a.txt".to_string()];
        let parsed = parse_args(&args).unwrap();
        assert!(parsed.wait);
    }

    #[test]
    fn timeout_옵션을_파싱한다() {
        let args = vec![
            "--wait".to_string(),
            "--timeout".to_string(),
            "60".to_string(),
            "a.txt".to_string(),
        ];
        let parsed = parse_args(&args).unwrap();
        assert_eq!(parsed.timeout_secs, 60);
    }

    #[test]
    fn 파일이_없으면_에러를_반환한다() {
        let args = vec!["--wait".to_string()];
        assert_eq!(parse_args(&args), Err(ArgsError::NoFiles));
    }

    #[test]
    fn timeout_값이_없으면_에러를_반환한다() {
        let args = vec!["--timeout".to_string()];
        assert_eq!(parse_args(&args), Err(ArgsError::MissingTimeoutValue));
    }

    #[test]
    fn timeout_값이_숫자가_아니면_에러를_반환한다() {
        let args = vec![
            "--timeout".to_string(),
            "abc".to_string(),
            "a.txt".to_string(),
        ];
        assert_eq!(
            parse_args(&args),
            Err(ArgsError::InvalidTimeoutValue("abc".to_string()))
        );
    }

    #[test]
    fn 마커_경로는_temp_dir_아래에_접두사를_붙여_생성된다() {
        let id = Uuid::nil();
        let marker = build_wait_marker_path(Path::new("/tmp"), id);
        assert_eq!(
            marker,
            PathBuf::from(format!(
                "/tmp/{WAIT_MARKER_PREFIX}00000000-0000-0000-0000-000000000000"
            ))
        );
    }

    #[test]
    fn 상대경로는_현재_디렉토리_기준_절대경로로_바뀐다() {
        let resolved = normalize_absolute("relative.txt").unwrap();
        assert!(resolved.is_absolute());
        assert!(resolved.ends_with("relative.txt"));
    }
}

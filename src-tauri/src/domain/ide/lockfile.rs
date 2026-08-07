use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::types::{IDE_NAME, IDE_TRANSPORT};
use crate::error::{AppError, AppResult};
use crate::infra::persist;

const CLAUDE_CONFIG_DIR_ENV: &str = "CLAUDE_CONFIG_DIR";
const HOME_ENV: &str = "HOME";
const USERPROFILE_ENV: &str = "USERPROFILE";
const IDE_SUBDIR: &str = "ide";
const CLAUDE_DIR: &str = ".claude";

#[cfg(unix)]
const LOCKFILE_DIR_MODE: u32 = 0o700;
#[cfg(all(unix, test))]
const LOCKFILE_FILE_MODE: u32 = 0o600;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdeLockfileContent {
    pub pid: u32,
    pub workspace_folders: Vec<String>,
    pub ide_name: String,
    pub transport: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub running_in_windows: bool,
    pub auth_token: String,
}

pub fn build_lockfile_content(pid: u32, workspace_folders: Vec<String>, auth_token: String) -> IdeLockfileContent {
    IdeLockfileContent {
        pid,
        workspace_folders,
        ide_name: IDE_NAME.to_string(),
        transport: IDE_TRANSPORT.to_string(),
        running_in_windows: cfg!(windows),
        auth_token,
    }
}

pub fn resolve_lockfile_dir(config_dir_env: Option<&str>, home_env: Option<&str>) -> AppResult<PathBuf> {
    if let Some(config_dir) = config_dir_env.filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(config_dir).join(IDE_SUBDIR));
    }

    let home = home_env
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Internal("home directory not found".to_string()))?;
    Ok(PathBuf::from(home).join(CLAUDE_DIR).join(IDE_SUBDIR))
}

pub fn lockfile_dir() -> AppResult<PathBuf> {
    let config_dir_env = std::env::var(CLAUDE_CONFIG_DIR_ENV).ok();
    let home_env = std::env::var(HOME_ENV).or_else(|_| std::env::var(USERPROFILE_ENV)).ok();
    resolve_lockfile_dir(config_dir_env.as_deref(), home_env.as_deref())
}

pub fn lockfile_path(dir: &Path, port: u32) -> PathBuf {
    dir.join(format!("{port}.lock"))
}

#[cfg(unix)]
fn create_private_dir(dir: &Path) -> AppResult<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

    if dir.exists() {
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(LOCKFILE_DIR_MODE))?;
        return Ok(());
    }
    std::fs::DirBuilder::new().recursive(true).mode(LOCKFILE_DIR_MODE).create(dir)?;
    Ok(())
}

#[cfg(not(unix))]
fn create_private_dir(dir: &Path) -> AppResult<()> {
    std::fs::create_dir_all(dir)?;
    Ok(())
}

pub fn write_lockfile_atomic(dir: &Path, port: u32, content: &IdeLockfileContent) -> AppResult<()> {
    create_private_dir(dir)?;

    let json = serde_json::to_string_pretty(content)?;
    persist::write_private_atomic(&lockfile_path(dir, port), json.as_bytes())
}

pub fn remove_lockfile(dir: &Path, port: u32) -> AppResult<()> {
    match std::fs::remove_file(lockfile_path(dir, port)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::from(error)),
    }
}

fn is_pid_alive(pid: u32) -> bool {
    let mut system = sysinfo::System::new();
    system.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(pid)]),
        true,
        sysinfo::ProcessRefreshKind::nothing(),
    );
    system.process(sysinfo::Pid::from_u32(pid)).is_some()
}

/// Removes lockfiles left behind by TAIDE processes that no longer exist.
///
/// Only files that parse as `IdeLockfileContent`, name TAIDE as the owner, and
/// reference a dead pid (never the caller's own pid) are removed. Unparseable
/// files and lockfiles owned by other IDEs are always left untouched so that
/// another editor's Claude Code integration is never disturbed.
pub fn cleanup_stale_lockfiles(dir: &Path, current_pid: u32) -> AppResult<u32> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(AppError::from(error)),
    };

    let mut removed = 0u32;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("lock") {
            continue;
        }

        let Ok(raw) = std::fs::read_to_string(&path) else { continue };
        let Ok(content) = serde_json::from_str::<IdeLockfileContent>(&raw) else {
            continue;
        };
        if content.ide_name != IDE_NAME || content.pid == current_pid || is_pid_alive(content.pid) {
            continue;
        }

        match std::fs::remove_file(&path) {
            Ok(()) => {
                removed += 1;
                log::info!("정지된 IDE lockfile 정리: pid={}", content.pid);
            }
            Err(error) => log::warn!("정지된 IDE lockfile 삭제 실패: {error}"),
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dir_env이_있으면_그_하위_ide를_쓴다() {
        let dir = resolve_lockfile_dir(Some("/custom/config"), Some("/home/user")).unwrap();
        assert_eq!(dir, PathBuf::from("/custom/config/ide"));
    }

    #[test]
    fn config_dir_env이_없으면_홈의_claude_ide를_쓴다() {
        let dir = resolve_lockfile_dir(None, Some("/home/user")).unwrap();
        assert_eq!(dir, PathBuf::from("/home/user/.claude/ide"));
    }

    #[test]
    fn 둘_다_없으면_에러를_반환한다() {
        assert!(resolve_lockfile_dir(None, None).is_err());
    }

    #[test]
    fn 빈_문자열_config_dir는_무시하고_홈으로_폴백한다() {
        let dir = resolve_lockfile_dir(Some(""), Some("/home/user")).unwrap();
        assert_eq!(dir, PathBuf::from("/home/user/.claude/ide"));
    }

    #[test]
    fn 락파일_경로는_포트_번호_파일명이다() {
        let dir = PathBuf::from("/home/user/.claude/ide");
        assert_eq!(lockfile_path(&dir, 54321), PathBuf::from("/home/user/.claude/ide/54321.lock"));
    }

    #[test]
    fn 쓰고_읽으면_내용이_동일하다() {
        let tmp = std::env::temp_dir().join(format!("taide-ide-lock-test-{}", uuid::Uuid::new_v4()));
        let content = build_lockfile_content(1234, vec!["/repo".to_string()], "a".repeat(32));

        write_lockfile_atomic(&tmp, 55123, &content).unwrap();
        let raw = std::fs::read_to_string(lockfile_path(&tmp, 55123)).unwrap();
        let parsed: IdeLockfileContent = serde_json::from_str(&raw).unwrap();

        assert_eq!(parsed, content);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn 이미_존재하는_락파일도_원자적으로_교체된다() {
        let tmp = std::env::temp_dir().join(format!("taide-ide-lock-test-{}", uuid::Uuid::new_v4()));
        let first = build_lockfile_content(1, vec!["/a".to_string()], "a".repeat(32));
        let second = build_lockfile_content(2, vec!["/b".to_string()], "b".repeat(32));

        write_lockfile_atomic(&tmp, 55124, &first).unwrap();
        write_lockfile_atomic(&tmp, 55124, &second).unwrap();

        let raw = std::fs::read_to_string(lockfile_path(&tmp, 55124)).unwrap();
        let parsed: IdeLockfileContent = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed, second);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn 삭제는_없는_파일에도_성공한다() {
        let tmp = std::env::temp_dir().join(format!("taide-ide-lock-test-{}", uuid::Uuid::new_v4()));
        assert!(remove_lockfile(&tmp, 1).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn 유닉스에서_퍼미션이_0600_0700이다() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = std::env::temp_dir().join(format!("taide-ide-lock-test-{}", uuid::Uuid::new_v4()));
        let content = build_lockfile_content(1, vec!["/a".to_string()], "a".repeat(32));
        write_lockfile_atomic(&tmp, 55125, &content).unwrap();

        let dir_mode = std::fs::metadata(&tmp).unwrap().permissions().mode() & 0o777;
        let file_mode = std::fs::metadata(lockfile_path(&tmp, 55125)).unwrap().permissions().mode() & 0o777;
        assert_eq!(dir_mode, LOCKFILE_DIR_MODE);
        assert_eq!(file_mode, LOCKFILE_FILE_MODE);
        std::fs::remove_dir_all(&tmp).ok();
    }

    fn stale_cleanup_test_dir() -> PathBuf {
        std::env::temp_dir().join(format!("taide-ide-lock-stale-test-{}", uuid::Uuid::new_v4()))
    }

    fn spawn_and_reap_dead_pid() -> u32 {
        let mut child = std::process::Command::new("true")
            .spawn()
            .expect("spawn 'true' for a disposable pid");
        let pid = child.id();
        child.wait().expect("reap the disposable process");
        pid
    }

    #[test]
    fn 죽은_taide_lock은_정리된다() {
        let tmp = stale_cleanup_test_dir();
        let dead_pid = spawn_and_reap_dead_pid();
        let content = build_lockfile_content(dead_pid, vec!["/repo".to_string()], "a".repeat(32));
        write_lockfile_atomic(&tmp, 60001, &content).unwrap();

        let removed = cleanup_stale_lockfiles(&tmp, std::process::id()).unwrap();

        assert_eq!(removed, 1);
        assert!(!lockfile_path(&tmp, 60001).exists());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn 살아있는_pid의_lock은_보존된다() {
        let tmp = stale_cleanup_test_dir();
        let alive_pid = std::process::id();
        let content = build_lockfile_content(alive_pid, vec!["/repo".to_string()], "a".repeat(32));
        write_lockfile_atomic(&tmp, 60002, &content).unwrap();

        let removed = cleanup_stale_lockfiles(&tmp, alive_pid.wrapping_add(1)).unwrap();

        assert_eq!(removed, 0);
        assert!(lockfile_path(&tmp, 60002).exists());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn 타_ide의_lock은_pid가_죽었어도_보존된다() {
        let tmp = stale_cleanup_test_dir();
        let dead_pid = spawn_and_reap_dead_pid();
        let mut content = build_lockfile_content(dead_pid, vec!["/repo".to_string()], "a".repeat(32));
        content.ide_name = "OtherIDE".to_string();
        write_lockfile_atomic(&tmp, 60003, &content).unwrap();

        let removed = cleanup_stale_lockfiles(&tmp, std::process::id()).unwrap();

        assert_eq!(removed, 0);
        assert!(lockfile_path(&tmp, 60003).exists());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn 깨진_json_lock은_보존된다() {
        let tmp = stale_cleanup_test_dir();
        create_private_dir(&tmp).unwrap();
        std::fs::write(tmp.join("60004.lock"), "not json").unwrap();

        let removed = cleanup_stale_lockfiles(&tmp, std::process::id()).unwrap();

        assert_eq!(removed, 0);
        assert!(tmp.join("60004.lock").exists());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn 디렉터리가_없으면_0을_반환한다() {
        let tmp = stale_cleanup_test_dir();
        assert_eq!(cleanup_stale_lockfiles(&tmp, std::process::id()).unwrap(), 0);
    }
}

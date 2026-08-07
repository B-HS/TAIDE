use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::types::{IDE_NAME, IDE_TRANSPORT};
use crate::error::{AppError, AppResult};

const CLAUDE_CONFIG_DIR_ENV: &str = "CLAUDE_CONFIG_DIR";
const HOME_ENV: &str = "HOME";
const USERPROFILE_ENV: &str = "USERPROFILE";
const IDE_SUBDIR: &str = "ide";
const CLAUDE_DIR: &str = ".claude";

#[cfg(unix)]
const LOCKFILE_DIR_MODE: u32 = 0o700;
#[cfg(unix)]
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

fn tmp_lockfile_path(dir: &Path, port: u32) -> PathBuf {
    dir.join(format!(".{port}.lock.tmp"))
}

#[cfg(unix)]
fn set_permissions(path: &Path, mode: u32) -> AppResult<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_permissions(_path: &Path, _mode: u32) -> AppResult<()> {
    Ok(())
}

pub fn write_lockfile_atomic(dir: &Path, port: u32, content: &IdeLockfileContent) -> AppResult<()> {
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    set_permissions(dir, LOCKFILE_DIR_MODE)?;

    let json = serde_json::to_string_pretty(content)?;
    let tmp_path = tmp_lockfile_path(dir, port);
    std::fs::write(&tmp_path, json)?;
    #[cfg(unix)]
    set_permissions(&tmp_path, LOCKFILE_FILE_MODE)?;

    std::fs::rename(&tmp_path, lockfile_path(dir, port))?;
    Ok(())
}

pub fn remove_lockfile(dir: &Path, port: u32) -> AppResult<()> {
    match std::fs::remove_file(lockfile_path(dir, port)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::from(error)),
    }
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
}

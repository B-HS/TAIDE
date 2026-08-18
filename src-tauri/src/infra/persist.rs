use std::io::Write;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::{AppError, AppResult};

const PRIVATE_FILE_MODE: u32 = 0o600;

pub fn write_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::InvalidArgument(format!("no parent directory: {}", path.display())))?;
    std::fs::create_dir_all(parent)?;

    let temp_path = temp_sibling(path);
    let mut file = std::fs::File::create(&temp_path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);

    std::fs::rename(&temp_path, path)?;
    Ok(())
}

pub fn write_atomic_with_mode(path: &Path, bytes: &[u8], mode: u32) -> AppResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::InvalidArgument(format!("no parent directory: {}", path.display())))?;
    std::fs::create_dir_all(parent)?;

    let temp_path = temp_sibling(path);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(mode);
    }
    #[cfg(not(unix))]
    {
        let _ = mode;
    }

    let result = (|| -> AppResult<()> {
        let mut file = options.open(&temp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp_path, path).map_err(AppError::from)
    })();

    if result.is_err() {
        std::fs::remove_file(&temp_path).ok();
    }
    result
}

/// Writes a file that may contain secrets (auth tokens).
/// The temporary file is created with owner-only permissions from the start,
/// so the content is never readable through a default-umask window.
pub fn write_private_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    write_atomic_with_mode(path, bytes, PRIVATE_FILE_MODE)
}

/// Rewrites an existing file atomically while preserving whatever permission bits it already had —
/// `write_atomic`'s create-new-temp-then-rename otherwise loses them, because the fresh temp file
/// only ever gets the process umask's default mode. Without this, the first time the app rewrites a
/// user's executable script (`chmod +x deploy.sh`) via `file_save` or a project-wide
/// `search_replace`, the rewrite silently clears its execute bit. Falls back to `write_atomic`'s
/// default-mode behavior when the target doesn't exist yet (nothing to preserve — a brand new file)
/// or on non-Unix targets, where permission bits don't carry the same meaning.
pub fn write_atomic_preserving_mode(path: &Path, bytes: &[u8]) -> AppResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(path) {
            let mode = metadata.permissions().mode() & 0o777;
            return write_atomic_with_mode(path, bytes, mode);
        }
    }
    write_atomic(path, bytes)
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> AppResult<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    write_atomic(path, &bytes)
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> AppResult<Option<T>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn temp_sibling(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.{}.tmp", uuid::Uuid::new_v4()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct Sample {
        value: u32,
    }

    #[test]
    fn write_json_후_read_json_이_왕복한다() {
        let dir = std::env::temp_dir().join(format!("taide-persist-{}", uuid::Uuid::new_v4()));
        let path = dir.join("nested").join("sample.json");

        write_json(&path, &Sample { value: 7 }).expect("write");
        let loaded: Option<Sample> = read_json(&path).expect("read");

        assert_eq!(loaded, Some(Sample { value: 7 }));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn write_atomic_with_mode_은_지정한_모드로_기록한다() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("taide-mode-{}", uuid::Uuid::new_v4()));
        let path = dir.join("file.json");

        write_atomic_with_mode(&path, b"{}", 0o640).expect("write");

        let mode = std::fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o640);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn write_private_atomic_은_소유자_전용_권한으로_기록한다() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("taide-private-{}", uuid::Uuid::new_v4()));
        let path = dir.join("secret.json");

        write_private_atomic(&path, b"{}").expect("write");

        let mode = std::fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, PRIVATE_FILE_MODE);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_atomic_with_mode_은_실패시_임시파일을_남기지_않는다() {
        let dir = std::env::temp_dir().join(format!("taide-mode-fail-{}", uuid::Uuid::new_v4()));
        let path = dir.join("target");
        std::fs::create_dir_all(&path).expect("setup dir");

        let result = write_atomic_with_mode(&path, b"{}", 0o600);

        assert!(result.is_err());
        let leftovers = std::fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn write_atomic_preserving_mode_은_기존_실행_비트를_유지한다() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("taide-preserve-mode-{}", uuid::Uuid::new_v4()));
        let path = dir.join("deploy.sh");
        std::fs::create_dir_all(&dir).expect("setup dir");
        std::fs::write(&path, b"#!/bin/sh\necho old\n").expect("initial write");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod +x");

        write_atomic_preserving_mode(&path, b"#!/bin/sh\necho new\n").expect("rewrite");

        let mode = std::fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o755, "재작성 후에도 실행 비트가 남아있어야 한다");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "#!/bin/sh\necho new\n");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_atomic_preserving_mode_은_새_파일이면_기본_모드로_기록한다() {
        let dir = std::env::temp_dir().join(format!("taide-preserve-mode-new-{}", uuid::Uuid::new_v4()));
        let path = dir.join("fresh.txt");

        let result = write_atomic_preserving_mode(&path, b"hello");

        assert!(result.is_ok());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 없는_파일은_none_을_반환한다() {
        let path = std::env::temp_dir().join(format!("taide-missing-{}.json", uuid::Uuid::new_v4()));
        let loaded: Option<Sample> = read_json(&path).expect("read");
        assert_eq!(loaded, None);
    }

    #[test]
    fn write_atomic_은_임시파일을_남기지_않는다() {
        let dir = std::env::temp_dir().join(format!("taide-atomic-{}", uuid::Uuid::new_v4()));
        let path = dir.join("state.json");

        write_atomic(&path, b"{}").expect("write");

        let leftovers = std::fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();

        assert_eq!(leftovers, 0);
        std::fs::remove_dir_all(&dir).ok();
    }
}

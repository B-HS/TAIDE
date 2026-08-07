use std::io::Write;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::{AppError, AppResult};

#[cfg(unix)]
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

/// Writes a file that may contain secrets (auth tokens).
/// The temporary file is created with owner-only permissions from the start,
/// so the content is never readable through a default-umask window.
pub fn write_private_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
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
        options.mode(PRIVATE_FILE_MODE);
    }

    let mut file = options.open(&temp_path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);

    std::fs::rename(&temp_path, path)?;
    Ok(())
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

fn temp_sibling(path: &Path) -> PathBuf {
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

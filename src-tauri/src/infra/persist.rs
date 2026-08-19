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

/// Recognizes a path shaped exactly like one [`temp_sibling`] would have produced. The filesystem
/// watcher (`infra::watcher::group_relevant_changes`) uses this to drop `write_atomic`'s
/// create-then-rename intermediate file from `FsChange` groups before they ever reach
/// `infra::self_write::resolve_from_app` — without it, that untracked temp path rides along in the
/// same watcher-debounced group as the final path `SelfWriteTracker` actually marked, and the
/// group's "every path must match" rule falls back to `from_app: false` for what was really the
/// app's own write (X1#10, `docs/acknowledge/2026-08-19-xa-wiring-cleanup-contract.md` §4's
/// follow-up finding). Matches the precise `.{name}.{uuid}.tmp` shape — a leading dot, a trailing
/// `.tmp`, and a valid UUID as the final dot-separated segment — rather than a loose
/// `starts_with('.') && ends_with(".tmp")` check, so an unrelated user dotfile that happens to end
/// in `.tmp` is never silently hidden from the watcher.
pub fn is_temp_sibling(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    let Some(without_tmp_suffix) = file_name.strip_suffix(".tmp") else {
        return false;
    };
    let Some(without_leading_dot) = without_tmp_suffix.strip_prefix('.') else {
        return false;
    };

    without_leading_dot
        .rsplit_once('.')
        .is_some_and(|(_, candidate_uuid)| uuid::Uuid::parse_str(candidate_uuid).is_ok())
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
    fn is_temp_sibling_은_temp_sibling이_만든_경로를_인식한다() {
        assert!(is_temp_sibling(&temp_sibling(Path::new("/repo/src/main.rs"))));
        assert!(is_temp_sibling(&temp_sibling(Path::new("/repo/Makefile"))));
    }

    #[test]
    fn is_temp_sibling_은_uuid_형식이_아니면_tmp로_끝나도_인식하지_않는다() {
        assert!(!is_temp_sibling(Path::new("/repo/.notes.tmp")));
        assert!(!is_temp_sibling(Path::new("/repo/main.rs")));
        assert!(!is_temp_sibling(Path::new("/repo/.main.rs.not-a-uuid.tmp")));
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

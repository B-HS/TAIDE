use std::path::{Path, PathBuf};

use taide_model::error::{AppError, AppResult};

use crate::service;

/// Resolves the existing project-scoped hook settings path.
pub fn settings_local_path(root: &str) -> PathBuf {
    Path::new(root).join(".claude").join("settings.local.json")
}

/// Restricts newly created JSON hook files to their owner.
pub const NEW_HOOKS_FILE_MODE: u32 = 0o600;

fn read_json_file_rejecting_invalid(path: &Path) -> AppResult<serde_json::Value> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(serde_json::json!({})),
        Err(error) => Err(AppError::from(error)),
    }
}

#[cfg(unix)]
fn existing_file_mode(path: &Path) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).ok().map(|metadata| metadata.permissions().mode() & 0o777)
}

fn write_hooks_file_preserving_mode(path: &Path, value: &serde_json::Value) -> AppResult<()> {
    let text = serde_json::to_string_pretty(value)?;

    #[cfg(unix)]
    let mode = existing_file_mode(path).unwrap_or(NEW_HOOKS_FILE_MODE);
    #[cfg(not(unix))]
    let mode = NEW_HOOKS_FILE_MODE;

    taide_infra::persist::write_atomic_with_mode(path, text.as_bytes(), mode)
}

/// Reads project hook JSON without overwriting invalid existing content.
pub fn read_settings_local(root: &str) -> AppResult<serde_json::Value> {
    read_json_file_rejecting_invalid(&settings_local_path(root))
}

/// Writes project hook JSON atomically while preserving its existing mode.
pub fn write_settings_local(root: &str, value: &serde_json::Value) -> AppResult<()> {
    write_hooks_file_preserving_mode(&settings_local_path(root), value)
}

/// Reads user hook JSON, treating only missing files as an empty document.
pub fn read_user_level_hooks(path: &Path) -> AppResult<serde_json::Value> {
    read_json_file_rejecting_invalid(path)
}

/// Writes user hook JSON atomically while preserving its existing mode.
pub fn write_user_level_hooks(path: &Path, value: &serde_json::Value) -> AppResult<()> {
    write_hooks_file_preserving_mode(path, value)
}

/// Reads a file TAIDE owns whole, `None` when it is not there. Unlike the JSON installs there is
/// nothing to parse — ownership is decided by the first line (`service::is_owned_hook_file`), and a
/// file failing that check belongs to somebody else whatever is in it.
pub fn read_owned_hook_file(path: &Path) -> AppResult<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(AppError::from(error)),
    }
}

/// Writes a plugin/extension file, creating the directory when the agent has not made one yet.
/// Deliberately *not* `write_private_atomic`: this file carries no server token (that is the point
/// of the in-band install) and sits among the user's own plugins, so narrowing it to `0600` would
/// make TAIDE's the odd one out in a directory the agent reads.
pub fn write_owned_hook_file(path: &Path, source: &str) -> AppResult<()> {
    taide_infra::persist::write_atomic_preserving_mode(path, source.as_bytes())
}

/// Deletes a file TAIDE owns. A missing file and a file owned by somebody else are both a quiet
/// no-op — uninstall must never remove a plugin TAIDE did not write.
pub fn remove_owned_hook_file(path: &Path) -> AppResult<()> {
    let Some(existing) = read_owned_hook_file(path)? else {
        return Ok(());
    };
    if !service::is_owned_hook_file(&existing) {
        return Ok(());
    }
    std::fs::remove_file(path).map_err(AppError::from)
}

use tauri::State;

use taide_runtime::app_actions;

use super::types::{AppFileTarget, PerfSnapshot};
use super::{service, types::AppInfo};
use crate::domain::settings::types::Settings;
use crate::error::AppResult;
use crate::infra::perf;
use crate::settings_port::SettingsApplyPort;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn app_get_info() -> AppResult<AppInfo> {
    Ok(service::app_info())
}

/// Reads the process-wide instrumentation registry (`infra::perf`). Every number is zero unless
/// the `TAIDE_PERF` gate is on — `enabled` in the reply says which, so an all-zero snapshot is
/// never ambiguous. Lives in `app` because the registry is app-wide, not any one domain's, and it
/// is the same domain `app_get_info` already uses for process-level metadata.
///
/// Remote-denied: the registry is a single process-global accumulator shared with the desktop
/// user's own measurement session (`RemoteDenialPolicy::DesktopProcessDiagnostics`).
#[tauri::command]
#[specta::specta]
pub async fn perf_snapshot() -> AppResult<PerfSnapshot> {
    Ok(service::perf_snapshot(perf::global()))
}

/// Zeroes every accumulated instrumentation number, starting a fresh measurement window. Leaves
/// the `TAIDE_PERF` gate and the installed command-name table alone.
#[tauri::command]
#[specta::specta]
pub async fn perf_reset() -> AppResult<()> {
    perf::global().reset();
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn app_file_read(state: State<'_, AppState>, target: AppFileTarget) -> AppResult<String> {
    app_actions::app_file_read(&state, target).await
}

/// Writes an `AppFileTarget`'s content. `Settings` runs the exact same
/// parse→sanitize→apply→broadcast pipeline as `settings_update`/`sync_download` through
/// `SettingsApplyPort`, which calls `settings::commands::apply_and_broadcast`. A hand-edited
/// `settings.json` reaches every window/remote session the same way a patch-based update does;
/// invalid JSON is rejected and the
/// on-disk file is left untouched. `Prompt` is a plain validate-then-write with no cross-window
/// broadcast, since prompt templates are only read lazily at the moment an AI request builds its
/// prompt (`ai::prompt::load_*`), not cached in `AppState`.
#[tauri::command]
#[specta::specta]
pub async fn app_file_write(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    apply_settings: State<'_, SettingsApplyPort>,
    target: AppFileTarget,
    content: String,
) -> AppResult<()> {
    app_actions::app_file_write(&state, target, content, |parsed| (apply_settings.0)(&app, &state, parsed)).await
}

/// Applies an already-parsed `Settings` value through the same
/// apply→persist→broadcast pipeline `settings_update`/the `Settings` branch above use
/// (through `SettingsApplyPort`), taking the mutation guard itself. Exists as its own
/// entry point — distinct from `app_file_write` — so the remote dispatch table can route a
/// `Settings`-target `app_file_write` through a value it has already stripped of the remote-gated
/// fields before it ever reaches `apply_and_broadcast`. The gated field set is not repeated here —
/// see `taide_remote::policy::strip_remote_gated_settings` for the canonical list.
pub async fn apply_settings_file(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    apply_settings: State<'_, SettingsApplyPort>,
    settings: Settings,
) -> AppResult<()> {
    app_actions::apply_settings_file(&state, settings, |next| (apply_settings.0)(&app, &state, next)).await
}

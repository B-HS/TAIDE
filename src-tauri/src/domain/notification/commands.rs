use tauri::{AppHandle, Manager, State};
use tauri_plugin_notification::NotificationExt;

use super::service;
use super::types::{NotificationCategory, NotificationDelivery};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Sends one completion event to the OS notification center, gated by
/// [`service::decide_delivery`].
///
/// The gate lives here rather than in the frontend because "no TAIDE window has focus" is an
/// app-wide fact: every window is a separate JS realm, so a window asking only about itself would
/// notify while the user is looking at another TAIDE window. `webview_windows()` covers the main
/// and every auxiliary editor window; `is_focused()` failing is read as "not focused" so a window
/// that is being torn down cannot suppress a notification.
///
/// `title`/`body` arrive already translated — Rust owns *whether* a notification is sent, the
/// frontend owns *what it says* (it has the `t()` catalog and the event's data). Neither string is
/// interpreted here.
///
/// A [`NotificationDelivery::Delivered`] return means the notification was handed to
/// `tauri-plugin-notification`, **not** that macOS displayed it: the plugin's desktop backend
/// spawns delivery and drops the result, and its permission query is a `Granted` stub, so the app
/// cannot observe "the user has notifications turned off for TAIDE". `Suppressed` is the only
/// outcome that is fully knowable, and it names which switch stopped the notification so the
/// settings screen's test button can explain a silent result.
#[tauri::command]
#[specta::specta]
pub async fn notification_notify(
    app: AppHandle,
    state: State<'_, AppState>,
    category: NotificationCategory,
    title: String,
    body: String,
) -> AppResult<NotificationDelivery> {
    let settings = state.settings.read().clone();
    let any_window_focused = app.webview_windows().values().any(|window| window.is_focused().unwrap_or(false));

    let decision = service::decide_delivery(&settings, category, any_window_focused);
    if decision == NotificationDelivery::Delivered {
        app.notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map_err(|error| AppError::Internal(error.to_string()))?;
    }
    Ok(decision)
}

/// Opens System Settings on its Notifications pane, so a user whose notifications are silently
/// blocked has one click to the switch that unblocks them. Detection is impossible (see
/// [`notification_notify`]), so this is offered unconditionally from the settings screen instead of
/// being triggered by a denied-permission signal that never arrives.
///
/// macOS only, and deliberately not routed through `system_open_external_url`: that command's
/// `validate_external_url` accepts `http(s)://` alone and must stay that narrow, so this opens the
/// single hardcoded [`crate::constants::MACOS_NOTIFICATION_SETTINGS_URL`] instead of accepting a
/// URL argument at all.
#[tauri::command]
#[specta::specta]
pub async fn notification_open_system_settings() -> AppResult<()> {
    #[cfg(target_os = "macos")]
    {
        tauri_plugin_opener::open_url(crate::constants::MACOS_NOTIFICATION_SETTINGS_URL, None::<&str>)
            .map_err(|error| AppError::Internal(error.to_string()))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(AppError::localized(
            crate::error::AppErrorKind::InvalidArgument,
            "error.notification.settingsUnsupported",
            "opening the notification settings is only supported on macOS",
        ))
    }
}

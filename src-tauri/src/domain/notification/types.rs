use serde::{Deserialize, Serialize};
use specta::Type;

/// The closed set of "a piece of work finished" events allowed to reach the OS notification
/// center. Deliberately an enum rather than a free-form string tag: the settings gate
/// ([`super::service::decide_delivery`]) matches on it exhaustively, so adding a category without
/// also adding its per-category toggle stops compiling instead of silently notifying with no way
/// to turn it off. The user decision that fixes this list to completion events only — no mirroring
/// of the app's ~150 in-app toasts — is
/// `docs/acknowledge/2026-09-04-usability-batch4-user-decisions.md` §결정 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum NotificationCategory {
    AgentCompleted,
    TaskCompleted,
    GitRemote,
    SearchReplace,
    LspInstall,
    Error,
}

/// Why a notification never reached the OS. Returned to the caller rather than logged and dropped
/// so the settings screen's "send a test notification" button can say *which* switch swallowed it
/// — with `tauri-plugin-notification`'s desktop backend reporting neither permission state nor
/// delivery failure, a silent no-op is otherwise indistinguishable from macOS having notifications
/// turned off for the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum NotificationSuppressionReason {
    /// `Settings::notifications_enabled` is off — the master switch.
    NotificationsDisabled,
    /// The category's own `notify_*` switch is off.
    CategoryDisabled,
    /// `Settings::notifications_only_when_unfocused` is on and some TAIDE window has focus.
    WindowFocused,
}

/// The outcome of one `notification_notify` call — the pure gate's decision
/// ([`super::service::decide_delivery`]) and the command's return value are the same type on
/// purpose, since nothing observable happens between them: `Delivered` means the notification was
/// handed to the plugin, **not** that macOS displayed it. The desktop backend spawns the actual
/// delivery and discards its result (`tauri-plugin-notification` 2.4.0 `src/desktop.rs`'s
/// `let _ = notification.show()`), and its `permission_state()` is a `Granted` stub, so "the user
/// has notifications turned off for TAIDE" is not observable from inside the app at all — see
/// `docs/features/settings-ui.md` for the always-visible escape hatch that replaces detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", tag = "outcome", content = "reason")]
pub enum NotificationDelivery {
    Delivered,
    Suppressed(NotificationSuppressionReason),
}

pub const IGNORED_DIR_NAMES: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    ".turbo",
    ".venv",
    "venv",
    "__pycache__",
    ".cache",
    ".idea",
    ".DS_Store",
];

pub const WATCH_DEBOUNCE_MS: u64 = 300;

pub const HOT_EXIT_FLUSH_TIMEOUT_MS: u64 = 2_500;

/// The label of the single window that existed before Wave I's multi-window support. Kept as the
/// target for menu Quit (`domain::window::commands::handle_menu_event`) and single-instance
/// focus-forwarding (`lib.rs`) — those two flows are intentionally main-window-only, unlike the
/// general `editor-*` auxiliary windows `domain::window` issues at runtime
/// (`docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md` §2.9).
pub const MAIN_WINDOW_LABEL: &str = "main";

/// The one URL [`crate::domain::notification::commands::notification_open_system_settings`] hands
/// to the OS opener — System Settings' Notifications pane, addressed by the settings extension's
/// bundle identifier (`/System/Library/ExtensionKit/Extensions/NotificationsSettings.appex` declares
/// `allowsXAppleSystemPreferencesURLScheme = true` for exactly this). It is a hardcoded constant
/// rather than an argument because `infra::external_url::validate_external_url` deliberately
/// whitelists `http(s)://` only — widening that guard to let a caller pass an
/// `x-apple.systempreferences:` URL would turn the app's "open a link" primitive into a generic
/// "open anything the OS shell understands" one. The extension's bundle id is OS-version
/// dependent, and `/usr/bin/open` spawns detached, so a version where this id no longer resolves
/// fails silently — which is why the settings screen shows the manual path as text next to the
/// button (`docs/features/settings-ui.md`).
pub const MACOS_NOTIFICATION_SETTINGS_URL: &str = "x-apple.systempreferences:com.apple.Notifications-Settings.extension";

pub const LARGE_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const LARGE_FILE_LINES: usize = 50_000;
pub const READ_ONLY_FILE_BYTES: u64 = 20 * 1024 * 1024;
pub const REFUSED_FILE_BYTES: u64 = 50 * 1024 * 1024;

const _: () = assert!(LARGE_FILE_BYTES < READ_ONLY_FILE_BYTES);
const _: () = assert!(READ_ONLY_FILE_BYTES < REFUSED_FILE_BYTES);

pub fn is_ignored_dir(name: &str) -> bool {
    IGNORED_DIR_NAMES.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 무시_목록은_대표_디렉토리를_포함한다() {
        assert!(is_ignored_dir(".git"));
        assert!(is_ignored_dir("node_modules"));
        assert!(!is_ignored_dir("src"));
    }
}

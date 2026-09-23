pub use taide_model::window::*;

/// Every auxiliary editor window's OS-level Tauri label starts with this prefix, followed by a
/// Rust-assigned integer (`editor-1`, `editor-2`, ...). The prefix (not the whole label) is what
/// `capabilities/main.json`'s `editor-*` glob and `tauri-plugin-window-state`'s label-normalizing
/// hook match against — see `service::is_auxiliary_label`.
pub const AUXILIARY_WINDOW_LABEL_PREFIX: &str = "editor-";

/// Single `tauri-plugin-window-state` cache key every auxiliary window is normalized onto
/// (`service::normalize_window_state_label`), so short-lived, ever-incrementing `editor-<n>`
/// labels don't accumulate unboundedly in `.window-state.json`. Deliberately has no trailing
/// digit, so it can never collide with a real `editor-<n>` label.
pub const AUXILIARY_WINDOW_STATE_KEY: &str = "editor";

pub const AUXILIARY_WINDOW_DEFAULT_WIDTH: f64 = 1_000.0;
pub const AUXILIARY_WINDOW_DEFAULT_HEIGHT: f64 = 700.0;
pub const AUXILIARY_WINDOW_MIN_WIDTH: f64 = 640.0;
pub const AUXILIARY_WINDOW_MIN_HEIGHT: f64 = 420.0;

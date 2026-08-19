use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ids::ProjectId;

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

/// Result of `commands::open_auxiliary_window` (reached via `layout_move_tab_to_window`'s
/// `newAuxiliary` path or boot-time restoration — the standalone `window_open_auxiliary` command was
/// removed as a duplicate IPC surface, X1#13). `label` is the Rust-assigned Tauri window label
/// (`editor-<n>`) the frontend needs to address this specific OS window (e.g. to correlate it
/// against `getAllWebviewWindows()`); `project_id`/`window_slot` echo the request back so the
/// caller doesn't have to thread its own copies through the async round-trip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AuxiliaryWindowInfo {
    pub label: String,
    pub project_id: ProjectId,
    pub window_slot: u32,
}

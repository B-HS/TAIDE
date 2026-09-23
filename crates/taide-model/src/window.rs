use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ids::ProjectId;

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

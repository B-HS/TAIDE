use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ids::ProjectId;

/// Which dirty editor models one flush handshake asks the frontend to write to the hot-exit mirror
/// before the backend does something that would otherwise lose them.
///
/// The handshake itself (`crate::events::HotExitFlushRequested` out,
/// `domain::file::commands::file_flush_complete` back, with a timeout fallback) started life as an
/// app-exit-only mechanism. Three separate teardowns destroy a webview or drop a project's state
/// while the mirror debounce (`HOT_EXIT_MIRROR_DEBOUNCE_MS`) may still be holding the last half
/// second of typing, and only one of them used to run it — so the scope travels with the request
/// and with every confirmation, and one window confirming a project close can never be mistaken
/// for the same window confirming the app's exit.
///
/// Serialized externally-tagged, which is what gives the frontend the union it reads:
/// `"all" | { window: label } | { project: projectId }`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FlushScope {
    /// Every window, every project — the app is exiting.
    All,
    /// One window's own models, because that OS window is about to be destroyed. Keyed by the
    /// Tauri label (`editor-<n>`) rather than by project so two auxiliary windows closing at the
    /// same time each wait for their own confirmation.
    Window(String),
    /// Every window's models belonging to one project, because that project is being closed and
    /// its `file_mirror_dirty` writes stop being accepted the moment it leaves `AppState`.
    Project(ProjectId),
}

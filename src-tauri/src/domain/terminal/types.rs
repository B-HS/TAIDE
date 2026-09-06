use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ids::ProjectId;

pub const DEFAULT_SCROLLBACK_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ShellProfile {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PtySpawnOptions {
    pub project_id: ProjectId,
    pub cwd: String,
    #[serde(default)]
    pub shell: Option<String>,
    pub cols: u16,
    pub rows: u16,
}

/// What one `pty_attach` handed back: the subscription id `pty_detach` consumes, plus how many
/// bytes that attach replayed before any live output could arrive.
///
/// `replay_bytes` exists so the renderer can tell replayed scrollback from live output on a stream
/// that carries both. It counts them against a budget instead of the write backlog that drives flow
/// control, which otherwise saw up to a full scrollback land at once and paused a healthy child
/// process on every terminal tab switch. It is a `u32` because `specta-typescript` refuses to export
/// BigInt-style types; one replay is bounded by [`DEFAULT_SCROLLBACK_BYTES`] plus a four-byte
/// preamble, three orders of magnitude below that ceiling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PtyAttachResult {
    pub subscription_id: u32,
    pub replay_bytes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSession {
    pub id: String,
    pub project_id: ProjectId,
    pub cwd: String,
    pub shell: String,
    pub running: bool,
}

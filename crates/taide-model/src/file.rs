use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ids::TabId;

pub const LARGE_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const LARGE_FILE_LINES: usize = 50_000;
pub const READ_ONLY_FILE_BYTES: u64 = 20 * 1024 * 1024;
pub const REFUSED_FILE_BYTES: u64 = 50 * 1024 * 1024;

const _: () = assert!(LARGE_FILE_BYTES < READ_ONLY_FILE_BYTES);
const _: () = assert!(READ_ONLY_FILE_BYTES < REFUSED_FILE_BYTES);

/// A restorable hot-exit mirror, resolved against the file's *current* disk
/// state at list time. `conflict` is `true` when the disk was modified after
/// the mirror's `disk_modified_ms` baseline was captured, meaning applying
/// the mirror as-is would silently discard an external change.
///
/// `source_missing` marks the other resolution: the file the draft belongs to
/// is gone from disk (deleted outside the app — `rm`, a `git checkout`, a build
/// script). Such a mirror is still listed, because being unlistable is exactly
/// what used to make the draft unreachable, but neither `conflict` nor
/// `disk_modified_ms` can be answered against a file that is not there, so both
/// come back `false`/`None` and the frontend offers "save as" instead of a
/// restore.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MirrorEntry {
    pub path: String,
    pub content: String,
    pub saved_at_ms: f64,
    pub disk_modified_ms: Option<f64>,
    pub conflict: bool,
    pub source_missing: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UntitledMirrorEntry {
    pub tab_id: TabId,
    pub content: String,
    pub saved_at_ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FileSizeTier {
    Normal,
    Large,
    ReadOnly,
    Refused,
}

/// `.editorconfig`'s `indent_style`, the only two values it defines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum EditorConfigIndentStyle {
    Tab,
    Space,
}

/// The core `.editorconfig` properties resolved for one file, as they came out of the config chain —
/// `None` means "no `.editorconfig` in this file's ancestry declared it", which is also what every
/// field holds when the `editor_config_enabled` setting is off or the file is
/// [`FileSizeTier::Refused`].
///
/// Deliberately the *raw* properties rather than an editor-ready shape: the frontend owns the
/// mapping onto monaco's `tabSize`/`insertSpaces` (`src/shared/lib/editorconfig.ts`), since only it
/// knows what the user's own settings are to fall back to for a property the config left alone. The
/// one interpretation applied here is `indent_size = tab`, which is a property-level alias for
/// `tab_width` and has nothing to do with editor settings
/// (`domain::file::editorconfig::interpret`).
///
/// `charset`/`end_of_line` are not carried — see this module's `editorconfig` sibling and the d-53
/// contract §5.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EditorConfigOptions {
    pub indent_style: Option<EditorConfigIndentStyle>,
    pub indent_size: Option<u32>,
    pub tab_width: Option<u32>,
    pub insert_final_newline: Option<bool>,
    pub trim_trailing_whitespace: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OpenedFile {
    pub path: String,
    pub content: String,
    pub language_id: String,
    pub byte_size: u32,
    pub line_count: u32,
    pub tier: FileSizeTier,
    pub read_only: bool,
    /// `content` came back from a lossy UTF-8 decode — the file holds bytes that are not valid
    /// UTF-8 (EUC-KR, Latin-1, …) and every one of them is now a U+FFFD replacement character that
    /// cannot be turned back into the original byte. Independent of [`FileSizeTier`], which only
    /// grades size: such a file is forced `read_only` at any tier so the lossy text can never be
    /// written back over the original (`domain::file::service::decode_utf8_lossy`), and the flag is
    /// what lets the editor say *why* it is read-only instead of blaming the file's size.
    pub encoding_lossy: bool,
    pub modified_ms: f64,
    /// The `.editorconfig` properties in force for this file, resolved at open time by
    /// `domain::file::editorconfig::resolve_for_file`. Rides on the open response rather
    /// than a command of its own so the editor has them in the same render that first shows the
    /// content — a second round trip would mount the model with the wrong indentation and correct
    /// it a tick later. Empty (every field `None`) when the `editor_config_enabled` setting is off,
    /// which is its default.
    pub editor_config: EditorConfigOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FsChangeKind {
    Created,
    Modified,
    Removed,
    Renamed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FsChange {
    pub kind: FsChangeKind,
    pub paths: Vec<String>,
    pub from_app: bool,
}

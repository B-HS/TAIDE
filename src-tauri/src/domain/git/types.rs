use serde::{Deserialize, Serialize};
use specta::Type;

pub const GRAPH_LANE_COLOR_COUNT: u32 = 12;
pub const LOG_PAGE_SIZE: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum GitChangeKind {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    TypeChange,
    Conflicted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StatusRow {
    pub path: String,
    #[serde(default)]
    pub orig_path: Option<String>,
    #[serde(default)]
    pub staged: Option<GitChangeKind>,
    #[serde(default)]
    pub unstaged: Option<GitChangeKind>,
    pub is_conflicted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub rows: Vec<StatusRow>,
    pub branch: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub has_remote: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GitRemote {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GitBranch {
    pub name: String,
    pub is_head: bool,
    pub is_remote: bool,
    pub upstream: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GitStashEntry {
    pub index: u32,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum HunkKind {
    Added,
    Modified,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GutterHunk {
    pub kind: HunkKind,
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BlameLine {
    pub line: u32,
    pub commit_id: String,
    pub author: String,
    pub time_unix: f64,
    pub summary: String,
    pub is_uncommitted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: String,
    pub parents: Vec<String>,
    pub summary: String,
    pub author: String,
    pub time_unix: f64,
    pub refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DiffMode {
    WorkdirVsIndex,
    IndexVsHead,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DiffSides {
    pub original: String,
    pub modified: String,
    pub language_id: String,
}

/// Unified diff text of staged changes (HEAD vs index), for AI commit message generation
/// (`ai_commit_message`) — see [`crate::domain::git::service::diff_staged_text`] for how
/// `truncated`/`skipped_files` get populated. `used_fallback` is `true` when the index had no
/// staged deltas against HEAD and `diff_text` was built from the working tree instead (Wave H
/// unstaged fallback, `docs/acknowledge/2026-08-16-wave-h-keymap-contract.md` §3.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StagedDiffText {
    pub diff_text: String,
    pub truncated: bool,
    pub skipped_files: Vec<String>,
    pub used_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CommitOptions {
    #[serde(default)]
    pub amend: bool,
    #[serde(default)]
    pub stage_all: bool,
}

/// The three sides of an unresolved merge conflict, keyed by index stage
/// (1 = ancestor/base, 2 = ours, 3 = theirs), plus the file's current
/// on-disk content. A side is `None` when that stage has no entry — e.g.
/// the ancestor is absent for an add/add conflict, or "ours"/"theirs" is
/// absent for a delete/modify conflict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ConflictSides {
    #[serde(default)]
    pub base: Option<String>,
    #[serde(default)]
    pub ours: Option<String>,
    #[serde(default)]
    pub theirs: Option<String>,
    pub workdir: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CommitFile {
    pub path: String,
    #[serde(default)]
    pub orig_path: Option<String>,
    pub kind: GitChangeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TagInfo {
    pub name: String,
    pub target: String,
    #[serde(default)]
    pub message: Option<String>,
    pub annotated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TagCreateOptions {
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub annotated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RevertOutcome {
    pub conflicted: bool,
    /// Paths left with unresolved conflict markers, so the caller can route the user straight to
    /// them (e.g. open the first one) instead of leaving conflict resolution to be discovered via
    /// the next status refresh. Always empty when `conflicted` is `false`.
    pub conflicted_paths: Vec<String>,
}

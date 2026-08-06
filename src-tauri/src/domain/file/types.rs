use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FileSizeTier {
    Normal,
    Large,
    ReadOnly,
    Refused,
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
    pub modified_ms: f64,
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

use std::ops::Range;
use std::path::PathBuf;

use ropey::Rope;
use taide_model::app::AppFileTarget;
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::TabId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DocumentId(pub(crate) u64);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DocumentKey {
    File(PathBuf),
    Untitled(TabId),
    AppFile(AppFileTarget),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UndoGroup(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorError {
    NotFound,
    InvalidBoundary,
    Overlap,
    StaleRevision,
    RevisionOverflow,
    ReadOnly,
    Refused,
    Capacity,
    AttachedViews,
    UnsavedChanges,
    StaleSave,
    InvalidIdentity,
}

#[derive(Debug, Clone)]
pub struct Edit {
    pub bytes: Range<usize>,
    pub text: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LineEnding {
    #[default]
    Lf,
    CrLf,
}

impl LineEnding {
    pub fn from_content(content: &str) -> Self {
        let mut bytes = content.bytes().peekable();
        let mut carriage_returns = 0usize;
        let mut line_feeds = 0usize;
        while let Some(byte) = bytes.next() {
            if byte == b'\r' {
                carriage_returns += 1;
                if bytes.peek() == Some(&b'\n') {
                    bytes.next();
                }
            } else if byte == b'\n' {
                line_feeds += 1;
            }
        }
        if carriage_returns > line_feeds {
            Self::CrLf
        } else {
            Self::Lf
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DocumentMetadata {
    pub language_id: String,
    pub tier: FileSizeTier,
    pub read_only: bool,
    pub lossy: bool,
    pub editor_config: EditorConfigOptions,
    pub disk_modified_ms: Option<f64>,
    pub line_ending: LineEnding,
}

impl DocumentMetadata {
    pub fn from_opened(file: &OpenedFile) -> Self {
        Self {
            language_id: file.language_id.clone(),
            tier: file.tier,
            read_only: file.read_only
                || file.encoding_lossy
                || matches!(file.tier, FileSizeTier::ReadOnly | FileSizeTier::Refused),
            lossy: file.encoding_lossy,
            editor_config: file.editor_config,
            disk_modified_ms: Some(file.modified_ms),
            line_ending: LineEnding::from_content(&file.content),
        }
    }
}

#[derive(Clone)]
pub struct DocumentSnapshot {
    pub id: DocumentId,
    pub key: DocumentKey,
    pub revision: u64,
    pub rope: Rope,
    pub metadata: DocumentMetadata,
    pub indent_options: Option<crate::indent::IndentOptions>,
    pub dirty: bool,
}

#[derive(Clone)]
pub struct DiskSnapshot {
    pub rope: Rope,
    pub metadata: DocumentMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskChoice {
    ViewDisk,
    KeepMine,
}

pub(crate) fn byte_to_char(rope: &Rope, byte: usize) -> Result<usize, EditorError> {
    let scalar = rope
        .try_byte_to_char(byte)
        .map_err(|_| EditorError::InvalidBoundary)?;
    if rope.char_to_byte(scalar) != byte {
        return Err(EditorError::InvalidBoundary);
    }
    Ok(scalar)
}

pub(crate) fn apply_edits(
    rope: &Rope,
    edits: &[Edit],
    byte_limit: usize,
) -> Result<(Rope, Vec<Edit>), EditorError> {
    let mut ordered = edits.to_vec();
    ordered.sort_by_key(|edit| edit.bytes.start);
    let mut size = rope.len_bytes();
    for edit in &ordered {
        if edit.bytes.start > edit.bytes.end {
            return Err(EditorError::InvalidBoundary);
        }
        byte_to_char(rope, edit.bytes.start)?;
        byte_to_char(rope, edit.bytes.end)?;
        size = size
            .checked_sub(edit.bytes.end - edit.bytes.start)
            .and_then(|size| size.checked_add(edit.text.len()))
            .ok_or(EditorError::Capacity)?;
    }
    if size > byte_limit {
        return Err(EditorError::Capacity);
    }
    let mut merged = Vec::<Edit>::new();
    for edit in ordered {
        if let Some(previous) = merged.last_mut() {
            if previous.bytes.end > edit.bytes.start {
                return Err(EditorError::Overlap);
            }
            if previous.bytes.start == edit.bytes.start {
                if previous.bytes.end != previous.bytes.start {
                    return Err(EditorError::Overlap);
                }
                previous.bytes.end = edit.bytes.end;
                previous.text.push_str(&edit.text);
                continue;
            }
        }
        merged.push(edit);
    }
    let mut after = rope.clone();
    for edit in merged.iter().rev() {
        let start = after.byte_to_char(edit.bytes.start);
        let end = after.byte_to_char(edit.bytes.end);
        after.remove(start..end);
        after.insert(start, &edit.text);
    }
    Ok((after, merged))
}

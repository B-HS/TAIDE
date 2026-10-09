use std::borrow::Cow;
use std::ops::Range;
use std::sync::Arc;

use crate::change_journal::ChangesSince;
use crate::decoration::{Decoration, DecorationKind, DecorationLayer, InlineStyle, Stickiness};
use crate::document::{DocumentId, DocumentKey, DocumentSnapshot, EditorError, byte_to_char};
use crate::editing::line_content_range;
use crate::language_configuration::LanguageRules;
use crate::lsp::Position;

const HINT_UTF16_WIDTH: u32 = 2;
const ANCHOR_Z_ORDER: u8 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
    Information,
    Hint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub severity: Severity,
    pub text: String,
    pub source: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marker {
    pub bytes: Range<usize>,
    pub message: Arc<Message>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerSet {
    document: DocumentId,
    key: DocumentKey,
    revision: u64,
    language: String,
    markers: Vec<Marker>,
    anchors: DecorationLayer,
}

impl MarkerSet {
    pub fn new(document: &DocumentSnapshot, mut markers: Vec<Marker>) -> Result<Self, EditorError> {
        for marker in &markers {
            if marker.bytes.start > marker.bytes.end {
                return Err(EditorError::InvalidBoundary);
            }
            byte_to_char(&document.rope, marker.bytes.start)?;
            byte_to_char(&document.rope, marker.bytes.end)?;
        }
        markers.sort_by_key(|marker| marker.bytes.start);
        let anchors = DecorationLayer::new(
            document.revision,
            ANCHOR_Z_ORDER,
            markers
                .iter()
                .map(|marker| Decoration {
                    bytes: marker.bytes.clone(),
                    kind: DecorationKind::Inline(InlineStyle::default()),
                    stickiness: Stickiness::NeverGrowsWhenTypingAtEdges,
                })
                .collect(),
        );
        Ok(Self {
            document: document.id,
            key: document.key.clone(),
            revision: document.revision,
            language: document.metadata.language_id.clone(),
            markers,
            anchors,
        })
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn markers(&self) -> &[Marker] {
        &self.markers
    }

    pub fn tracked<'a>(
        &'a self,
        document: &DocumentSnapshot,
        changes: ChangesSince<'_>,
    ) -> Option<Cow<'a, Self>> {
        if self.document != document.id
            || self.key != document.key
            || self.language != document.metadata.language_id
        {
            return None;
        }
        if self.revision == document.revision {
            return Some(Cow::Borrowed(self));
        }
        let anchors = self.anchors.tracking(changes)?;
        if anchors.revision() != document.revision {
            return None;
        }
        let mut tracked = self.clone();
        tracked.revision = document.revision;
        for (marker, anchor) in tracked.markers.iter_mut().zip(anchors.items()) {
            marker.bytes = anchor.bytes.clone();
        }
        tracked.anchors = anchors.into_owned();
        Some(Cow::Owned(tracked))
    }
}

pub fn marker_range(
    document: &DocumentSnapshot,
    range: lsp_types::Range,
    severity: Severity,
) -> Option<Range<usize>> {
    if range.start > range.end {
        return None;
    }
    let end = if severity == Severity::Hint {
        Position::new(
            range.start.line,
            range.start.character.saturating_add(HINT_UTF16_WIDTH),
        )
    } else {
        range.end
    };
    let start = marker_byte(document, range.start, false);
    Some(start..marker_byte(document, end, range.start != end).max(start))
}

pub fn display_range(
    document: &DocumentSnapshot,
    bytes: Range<usize>,
    rules: Option<&dyn LanguageRules>,
) -> Range<usize> {
    if !bytes.is_empty() {
        return bytes;
    }
    let line = document.rope.byte_to_line(bytes.start);
    let content = line_content_range(document, line);
    let last_non_whitespace = document
        .rope
        .byte_slice(content.clone())
        .to_string()
        .trim_end_matches(char::is_whitespace)
        .len();
    if bytes.end >= content.start + last_non_whitespace {
        return bytes;
    }
    crate::cursor_commands::word_range(document, bytes.start, rules).unwrap_or(bytes)
}

fn marker_byte(document: &DocumentSnapshot, position: Position, expand: bool) -> usize {
    if position.line as usize >= document.rope.len_lines() {
        return document.rope.len_bytes();
    }
    let line = (position.line as usize).min(document.rope.len_lines() - 1);
    let content = line_content_range(document, line);
    let text = document.rope.byte_slice(content.clone());
    let units = (position.character as usize).min(text.len_utf16_cu());
    let scalar = text.utf16_cu_to_char(units);
    let scalar = scalar + usize::from(expand && text.char_to_utf16_cu(scalar) != units);
    content.start + text.char_to_byte(scalar)
}

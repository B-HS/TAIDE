use std::ops::Range;

pub use lsp_types::Range as LspRange;
pub use lsp_types::{Position, TextEdit};

use crate::document::{DocumentSnapshot, Edit, EditorError, UndoGroup, byte_to_char};
use crate::editing::line_content_range;
use crate::store::{EditorStore, Transaction};
use crate::view::ViewId;

pub fn position_to_byte(
    document: &DocumentSnapshot,
    position: Position,
) -> Result<usize, EditorError> {
    let line = usize::try_from(position.line).map_err(|_| EditorError::InvalidBoundary)?;
    if line >= document.rope.len_lines() {
        return Ok(document.rope.len_bytes());
    }
    let range = line_content_range(document, line);
    let requested =
        usize::try_from(position.character).map_err(|_| EditorError::InvalidBoundary)?;
    let slice = document.rope.byte_slice(range.clone());
    let requested = requested.min(slice.len_utf16_cu());
    let scalar = slice.utf16_cu_to_char(requested);
    if slice.char_to_utf16_cu(scalar) != requested {
        return Err(EditorError::InvalidBoundary);
    }
    Ok(range.start + slice.char_to_byte(scalar))
}

pub fn byte_to_position(document: &DocumentSnapshot, byte: usize) -> Result<Position, EditorError> {
    byte_to_char(&document.rope, byte)?;
    let line = document.rope.byte_to_line(byte);
    let range = line_content_range(document, line);
    let byte = byte.min(range.end);
    let units = document
        .rope
        .char_to_utf16_cu(byte_to_char(&document.rope, byte)?)
        - document
            .rope
            .char_to_utf16_cu(document.rope.line_to_char(line));
    Ok(Position {
        line: u32::try_from(line).map_err(|_| EditorError::Capacity)?,
        character: u32::try_from(units).map_err(|_| EditorError::Capacity)?,
    })
}

pub fn range_to_bytes(
    document: &DocumentSnapshot,
    range: lsp_types::Range,
) -> Result<Range<usize>, EditorError> {
    if range.start > range.end {
        return Err(EditorError::InvalidBoundary);
    }
    Ok(position_to_byte(document, range.start)?..position_to_byte(document, range.end)?)
}

pub fn apply_text_edits(
    store: &mut EditorStore,
    requested: &DocumentSnapshot,
    view: Option<ViewId>,
    edits: Vec<TextEdit>,
) -> Result<bool, EditorError> {
    let current = store.documents().snapshot(requested.id)?;
    if current.key != requested.key {
        return Err(EditorError::InvalidIdentity);
    }
    if current.revision != requested.revision {
        return Err(EditorError::StaleRevision);
    }
    if current.metadata.read_only {
        return Err(EditorError::ReadOnly);
    }
    let edits = edits
        .into_iter()
        .map(|edit| {
            Ok(Edit {
                bytes: range_to_bytes(&current, edit.range)?,
                text: edit.new_text,
            })
        })
        .collect::<Result<Vec<_>, EditorError>>()?;
    if edits.is_empty() {
        return Ok(false);
    }
    store.break_undo_group(current.id)?;
    let revision = store.apply(
        current.id,
        Transaction {
            revision: current.revision,
            edits,
            group: UndoGroup(current.revision),
            origin: view,
            selection_after: None,
        },
    )?;
    store.break_undo_group(current.id)?;
    Ok(revision != current.revision)
}

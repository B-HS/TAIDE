use ropey::RopeSlice;
use taide_lsp::native::protocol::lsp_types::{Position, TextEdit};

use crate::{DocumentProbe, Edit, EditError};

const MAX_LSP_UINT: u32 = 2147483647;

fn content_chars(line: RopeSlice<'_>) -> usize {
    let length = line.len_chars();
    if length == 0 {
        return 0;
    }
    let last = line.char(length - 1);
    if last == '\r' {
        return length - 1;
    }
    if last != '\n' {
        return length;
    }
    if length > 1 && line.char(length - 2) == '\r' {
        return length - 2;
    }
    length - 1
}

impl DocumentProbe {
    pub fn position_to_byte(&self, revision: u64, position: Position) -> Result<usize, EditError> {
        if revision != self.revision {
            return Err(EditError::StaleRevision);
        }
        if position.line > MAX_LSP_UINT || position.character > MAX_LSP_UINT {
            return Err(EditError::InvalidBoundary);
        }
        let line_index = usize::try_from(position.line).map_err(|_| EditError::InvalidBoundary)?;
        let line = self
            .rope
            .get_line(line_index)
            .ok_or(EditError::InvalidBoundary)?;
        let length = line.char_to_utf16_cu(content_chars(line));
        let unit = usize::try_from(position.character)
            .map_err(|_| EditError::InvalidBoundary)?
            .min(length);
        let scalar = line.utf16_cu_to_char(unit);
        if line.char_to_utf16_cu(scalar) != unit {
            return Err(EditError::InvalidBoundary);
        }
        let start = self.rope.line_to_char(line_index);
        Ok(self.rope.char_to_byte(start + scalar))
    }

    pub fn byte_to_position(&self, revision: u64, byte: usize) -> Result<Position, EditError> {
        if revision != self.revision {
            return Err(EditError::StaleRevision);
        }
        let scalar = self.byte_to_scalar(byte)?;
        let line_index = self.rope.char_to_line(scalar);
        let line = self.rope.line(line_index);
        let local = scalar - self.rope.line_to_char(line_index);
        if local > content_chars(line) {
            return Err(EditError::InvalidBoundary);
        }
        let character =
            u32::try_from(line.char_to_utf16_cu(local)).map_err(|_| EditError::InvalidBoundary)?;
        let line = u32::try_from(line_index).map_err(|_| EditError::InvalidBoundary)?;
        if line > MAX_LSP_UINT || character > MAX_LSP_UINT {
            return Err(EditError::InvalidBoundary);
        }
        Ok(Position::new(line, character))
    }

    pub fn apply_lsp_edits(
        &mut self,
        revision: u64,
        edits: Vec<TextEdit>,
    ) -> Result<(), EditError> {
        if revision != self.revision {
            return Err(EditError::StaleRevision);
        }
        let mut edits = edits
            .into_iter()
            .map(|edit| {
                if edit.range.start > edit.range.end {
                    return Err(EditError::InvalidBoundary);
                }
                let start = self.position_to_byte(revision, edit.range.start)?;
                let end = self.position_to_byte(revision, edit.range.end)?;
                Ok(Edit {
                    bytes: start..end,
                    text: edit.new_text,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        edits.sort_by_key(|edit| edit.bytes.start);
        let mut merged = Vec::<Edit>::new();
        for edit in edits {
            if let Some(previous) = merged
                .last_mut()
                .filter(|previous| previous.bytes.start == edit.bytes.start)
            {
                if previous.bytes.end != previous.bytes.start {
                    return Err(EditError::Overlap);
                }
                previous.bytes.end = edit.bytes.end;
                previous.text.push_str(&edit.text);
                continue;
            }
            merged.push(edit);
        }
        self.apply(revision, merged)
    }
}

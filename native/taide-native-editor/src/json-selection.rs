use std::ops::Range;

use crate::document::EditorError;
use crate::json_parser::{self, Kind};
use crate::json_scanner::{self, Kind as TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionRange {
    pub bytes: Range<usize>,
    pub utf16: Range<usize>,
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub parser: json_parser::Limits,
    pub max_positions: usize,
    pub max_ranges: usize,
}

struct Offsets {
    unit_bytes: Vec<usize>,
}

impl Offsets {
    fn new(input: &str) -> Self {
        let mut unit_bytes = vec![0];
        for (byte, character) in input.char_indices() {
            if character.len_utf16() > 1 {
                unit_bytes.push(byte);
            }
            unit_bytes.push(byte + character.len_utf8());
        }
        Self { unit_bytes }
    }

    fn units_at(&self, byte: usize) -> Result<usize, EditorError> {
        let unit = self.unit_bytes.partition_point(|&current| current < byte);
        if self.unit_bytes.get(unit) != Some(&byte) {
            return Err(EditorError::InvalidBoundary);
        }
        Ok(unit)
    }

    fn range(&self, mut utf16: Range<usize>) -> Result<SelectionRange, EditorError> {
        if utf16.start > utf16.end || utf16.end >= self.unit_bytes.len() {
            return Err(EditorError::InvalidBoundary);
        }
        if utf16.start > 0 && self.unit_bytes[utf16.start] == self.unit_bytes[utf16.start - 1] {
            utf16.start -= 1;
        }
        if utf16.end > 0 && self.unit_bytes[utf16.end] == self.unit_bytes[utf16.end - 1] {
            utf16.end += 1;
        }
        Ok(SelectionRange {
            bytes: self.unit_bytes[utf16.start]..self.unit_bytes[utf16.end],
            utf16,
        })
    }
}

pub fn selection_ranges(
    input: &str,
    positions: &[usize],
    limits: Limits,
) -> Result<Vec<Vec<SelectionRange>>, EditorError> {
    if positions.len() > limits.max_positions {
        return Err(EditorError::Capacity);
    }
    if positions.iter().any(|&byte| !input.is_char_boundary(byte)) {
        return Err(EditorError::InvalidBoundary);
    }
    let document = json_parser::parse(input, false, limits.parser)?;
    let tokens = json_scanner::scan(input, true, limits.parser.scanner)?;
    let offsets = Offsets::new(input);
    let mut result = Vec::new();
    let mut range_count = 0usize;
    for &position in positions {
        let offset = offsets.units_at(position)?;
        let mut current = document.node_at_byte(position, true);
        let has_node = current.is_some();
        let mut ranges = Vec::new();
        let mut push = |utf16| {
            if range_count >= limits.max_ranges {
                return Err(EditorError::Capacity);
            }
            ranges.push(offsets.range(utf16)?);
            range_count += 1;
            Ok(())
        };
        while let Some(index) = current {
            let node = &document.nodes[index];
            if matches!(node.kind, Kind::String | Kind::Object | Kind::Array) {
                let inner = node.utf16.start + 1..node.utf16.end.saturating_sub(1);
                if inner.start < inner.end && inner.start <= offset && offset <= inner.end {
                    push(inner)?;
                }
            }
            push(node.utf16.clone())?;
            if node.kind == Kind::Property
                || node
                    .parent
                    .is_some_and(|parent| document.nodes[parent].kind == Kind::Array)
            {
                let next = tokens.partition_point(|token| token.utf16.start < node.utf16.end);
                if let Some(token) = tokens
                    .get(next)
                    .filter(|token| token.kind == TokenKind::Comma)
                {
                    push(node.utf16.start..token.utf16.end)?;
                }
            }
            current = node.parent;
        }
        if !has_node {
            push(offset..offset)?;
        }
        result.push(ranges);
    }
    Ok(result)
}

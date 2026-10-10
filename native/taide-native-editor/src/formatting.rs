use crate::document::{DocumentSnapshot, EditorError};
use crate::editing::{line_content_range, ordered};
use crate::lsp::{LspRange, TextEdit, byte_to_position, range_to_bytes};
use crate::view::SelectionSet;

const DIFF_UNIT_LIMIT: usize = 100_000;
const DIFF_WORK_LIMIT: usize = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Document,
    Selection,
}

impl Command {
    pub fn from_action(action: &str) -> Option<Self> {
        match action {
            "editor.action.formatDocument" => Some(Self::Document),
            "editor.action.formatSelection" => Some(Self::Selection),
            _ => None,
        }
    }
}

pub fn selection_ranges(
    document: &DocumentSnapshot,
    selections: &SelectionSet,
) -> Result<Vec<LspRange>, EditorError> {
    let ranges = selections
        .selections
        .iter()
        .map(|selection| {
            let bytes = ordered(selection);
            let bytes = if bytes.is_empty() {
                line_content_range(
                    document,
                    document
                        .rope
                        .try_byte_to_line(bytes.start)
                        .map_err(|_| EditorError::InvalidBoundary)?,
                )
            } else {
                bytes
            };
            Ok(LspRange {
                start: byte_to_position(document, bytes.start)?,
                end: byte_to_position(document, bytes.end)?,
            })
        })
        .collect::<Result<Vec<_>, EditorError>>()?;
    Ok(merge_ranges(ranges))
}

pub fn merge_ranges(mut ranges: Vec<LspRange>) -> Vec<LspRange> {
    ranges.sort_by_key(|range| (range.start, range.end));
    let mut merged: Vec<LspRange> = Vec::new();
    for range in ranges {
        if let Some(last) = merged.last_mut().filter(|last| range.start <= last.end) {
            last.end = last.end.max(range.end);
        } else {
            merged.push(range);
        }
    }
    merged
}

pub fn minimal_edits(
    document: &DocumentSnapshot,
    edits: Vec<TextEdit>,
) -> Result<Vec<TextEdit>, EditorError> {
    let mut bytes = edits
        .into_iter()
        .map(|edit| Ok((range_to_bytes(document, edit.range)?, edit.new_text)))
        .collect::<Result<Vec<_>, EditorError>>()?;
    bytes.sort_by_key(|(range, _)| (range.start, range.end));
    let mut merged: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    for (range, text) in bytes {
        if let Some((previous, content)) = merged.last_mut() {
            if range.start < previous.end {
                return Err(EditorError::Overlap);
            }
            if previous.end == range.start {
                previous.end = range.end;
                content.push_str(&text);
                continue;
            }
        }
        merged.push((range, text));
    }
    let mut minimal = Vec::new();
    for (range, text) in merged {
        let original = document.rope.byte_slice(range.clone()).to_string();
        let text = text
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .replace('\n', document.metadata.line_ending.as_str());
        if original == text {
            continue;
        }
        if original
            .encode_utf16()
            .count()
            .max(text.encode_utf16().count())
            > DIFF_UNIT_LIMIT
        {
            minimal.push(TextEdit::new(
                LspRange::new(
                    byte_to_position(document, range.start)?,
                    byte_to_position(document, range.end)?,
                ),
                text,
            ));
            continue;
        }
        let original_units = units(&original);
        let modified_units = units(&text);
        let matches = matching_units(&original_units.0, &modified_units.0);
        let Some(matches) = matches else {
            minimal.push(TextEdit::new(
                LspRange::new(
                    byte_to_position(document, range.start)?,
                    byte_to_position(document, range.end)?,
                ),
                text,
            ));
            continue;
        };
        let mut old = 0;
        let mut new = 0;
        for (left, right) in matches.into_iter().chain(std::iter::once((
            original_units.0.len(),
            modified_units.0.len(),
        ))) {
            if old < left || new < right {
                minimal.push(TextEdit::new(
                    LspRange::new(
                        byte_to_position(document, range.start + original_units.1[old])?,
                        byte_to_position(document, range.start + original_units.1[left])?,
                    ),
                    text[modified_units.1[new]..modified_units.1[right]].to_owned(),
                ));
            }
            old = left + 1;
            new = right + 1;
        }
    }
    Ok(minimal)
}

fn units(text: &str) -> (Vec<&str>, Vec<usize>) {
    let mut result = Vec::new();
    let mut offsets = Vec::new();
    let mut characters = text.char_indices().peekable();
    while let Some((byte, character)) = characters.next() {
        offsets.push(byte);
        let length =
            if character == '\r' && characters.peek().is_some_and(|(_, next)| *next == '\n') {
                characters.next();
                "\r\n".len()
            } else {
                character.len_utf8()
            };
        result.push(&text[byte..byte + length]);
    }
    offsets.push(text.len());
    (result, offsets)
}

fn frontier(row: &[usize], diagonal: isize) -> usize {
    let radius = (row.len() - 1) / 2;
    usize::try_from(diagonal + isize::try_from(radius).unwrap())
        .ok()
        .and_then(|index| row.get(index))
        .copied()
        .unwrap_or(0)
}

fn matching_units(left: &[&str], right: &[&str]) -> Option<Vec<(usize, usize)>> {
    let mut history = Vec::new();
    let mut previous = Vec::new();
    let mut work = 0usize;
    for distance in 0..=left.len() + right.len() {
        let signed = isize::try_from(distance).ok()?;
        let mut row = vec![0; distance.checked_mul(2)?.checked_add(1)?];
        for diagonal in (-signed..=signed).step_by(2) {
            work += 1;
            let mut x = if distance == 0 {
                0
            } else if diagonal == -signed
                || (diagonal != signed
                    && frontier(&previous, diagonal - 1) < frontier(&previous, diagonal + 1))
            {
                frontier(&previous, diagonal + 1)
            } else {
                frontier(&previous, diagonal - 1) + 1
            };
            let mut y = usize::try_from(isize::try_from(x).ok()? - diagonal).ok()?;
            while x < left.len() && y < right.len() && left[x] == right[y] {
                x += 1;
                y += 1;
                work += 1;
            }
            if work > DIFF_WORK_LIMIT {
                return None;
            }
            row[usize::try_from(diagonal + signed).ok()?] = x;
            if x == left.len() && y == right.len() {
                history.push(row);
                return Some(backtrack(&history, x, y));
            }
        }
        history.push(row.clone());
        previous = row;
    }
    None
}

fn backtrack(history: &[Vec<usize>], mut x: usize, mut y: usize) -> Vec<(usize, usize)> {
    let mut matches = Vec::new();
    for distance in (1..history.len()).rev() {
        let signed = isize::try_from(distance).unwrap();
        let diagonal = isize::try_from(x).unwrap() - isize::try_from(y).unwrap();
        let previous = &history[distance - 1];
        let previous_diagonal = if diagonal == -signed
            || (diagonal != signed
                && frontier(previous, diagonal - 1) < frontier(previous, diagonal + 1))
        {
            diagonal + 1
        } else {
            diagonal - 1
        };
        let previous_x = frontier(previous, previous_diagonal);
        let previous_y =
            usize::try_from(isize::try_from(previous_x).unwrap() - previous_diagonal).unwrap();
        while x > previous_x && y > previous_y {
            x -= 1;
            y -= 1;
            matches.push((x, y));
        }
        x = previous_x;
        y = previous_y;
    }
    while x > 0 && y > 0 {
        x -= 1;
        y -= 1;
        matches.push((x, y));
    }
    matches.reverse();
    matches
}

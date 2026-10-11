use std::collections::BTreeMap;

use crate::completion_preview::Part;
use crate::completion_preview_view::ViewData;
use crate::document::{DocumentSnapshot, EditorError, byte_to_char};
use crate::editing::line_content_range;

pub const DEFAULT_MAXIMUM_LENGTH: usize = 43;
const WHITESPACE: char = '\u{00a0}';
const PADDING: char = '\u{200a}';
const ELLIPSIS: char = '\u{2026}';

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Type,
    Parameter,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Hint {
    pub byte: usize,
    pub label: String,
    pub kind: Option<Kind>,
    pub tooltip: Option<String>,
    pub padding_left: bool,
    pub padding_right: bool,
}

pub(crate) fn views(
    document: &DocumentSnapshot,
    hints: &[Hint],
    maximum_length: usize,
) -> Result<Vec<(usize, ViewData)>, EditorError> {
    let mut ordered = hints.iter().enumerate().collect::<Vec<_>>();
    ordered.sort_by_key(|(_, hint)| hint.byte);
    let mut line_lengths = BTreeMap::<usize, usize>::new();
    let mut result = Vec::new();
    for (index, hint) in ordered {
        byte_to_char(&document.rope, hint.byte)?;
        let line = document.rope.byte_to_line(hint.byte);
        let source = line_content_range(document, line);
        if hint.byte > source.end || hint.label.is_empty() || hint.label.contains(['\n', '\r']) {
            return Err(EditorError::InvalidBoundary);
        }
        let length = line_lengths.entry(line).or_default();
        if maximum_length > 0 && *length >= maximum_length {
            continue;
        }
        let units = hint.label.encode_utf16().count();
        let remaining = maximum_length.saturating_sub(*length);
        *length = length.saturating_add(units);
        let truncated = maximum_length > 0 && units > remaining;
        let limit = if truncated {
            remaining.saturating_sub(1)
        } else {
            units
        };
        let mut rendered = String::new();
        if hint.padding_left {
            rendered.push(PADDING);
        }
        let mut consumed = 0;
        for character in hint.label.chars() {
            if consumed + character.len_utf16() > limit {
                break;
            }
            consumed += character.len_utf16();
            rendered.push(match character {
                ' ' | '\t' => WHITESPACE,
                other => other,
            });
        }
        if truncated {
            rendered.push(ELLIPSIS);
        }
        if hint.padding_right {
            rendered.push(PADDING);
        }
        result.push((
            index,
            ViewData {
                line,
                inline: vec![Part {
                    byte: hint.byte,
                    text: rendered,
                    is_preview: false,
                }],
                additional: Vec::new(),
                hidden_source: None,
            },
        ));
    }
    Ok(result)
}

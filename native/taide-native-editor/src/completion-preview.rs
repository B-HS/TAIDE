use std::ops::Range;

use crate::document::{DocumentSnapshot, EditorError, byte_to_char};
use crate::editing::line_content_range;

const MAX_DIFF_UNITS: usize = 5000;
const DIRECTIONS: usize = 2;
const OPEN_PARENTHESIS: u16 = b'(' as u16;
const CLOSE_PARENTHESIS: u16 = b')' as u16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Prefix,
    Subword,
    SubwordSmart,
}

#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub mode: Mode,
    pub preview_suffix_utf16: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub byte: usize,
    pub text: String,
    pub is_preview: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhostText {
    pub line: usize,
    pub parts: Vec<Part>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Token {
    code: u16,
    bracket: Option<(usize, usize)>,
}

pub fn compute(
    document: &DocumentSnapshot,
    bytes: Range<usize>,
    text: &str,
    cursor: usize,
    options: Options,
) -> Result<Option<GhostText>, EditorError> {
    if bytes.start > bytes.end {
        return Err(EditorError::InvalidBoundary);
    }
    byte_to_char(&document.rope, bytes.start)?;
    byte_to_char(&document.rope, bytes.end)?;
    byte_to_char(&document.rope, cursor)?;
    for byte in [bytes.start, bytes.end] {
        let line = document.rope.byte_to_line(byte);
        if !line_content_range(document, line).contains(&byte)
            && byte != line_content_range(document, line).end
        {
            return Err(EditorError::InvalidBoundary);
        }
    }
    let replacement = text.replace("\r\n", "\n");
    let source = document.rope.byte_slice(bytes.clone()).to_string();
    let (prefix_source, prefix_replacement) = common_prefix(&source, &replacement);
    let mut start = bytes.start + prefix_source;
    let line = document.rope.byte_to_line(start);
    if line != document.rope.byte_to_line(bytes.end) {
        return Ok(None);
    }
    let content = document
        .rope
        .byte_slice(line_content_range(document, line))
        .to_string();
    let line_start = document.rope.line_to_byte(line);
    let indentation_end = content
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    let mut replacement = &replacement[prefix_replacement..];
    if start - line_start <= indentation_end {
        let replaced_indentation = &content[start - line_start..indentation_end];
        start = (start + replaced_indentation.len()).min(bytes.end);
        replacement = if replacement.starts_with(replaced_indentation) {
            &replacement[replaced_indentation.len()..]
        } else {
            replacement.trim_start_matches([' ', '\t'])
        };
    }
    let original = source[start - bytes.start..]
        .encode_utf16()
        .take(MAX_DIFF_UNITS + 1)
        .collect::<Vec<_>>();
    let modified = replacement
        .encode_utf16()
        .take(MAX_DIFF_UNITS + 1)
        .collect::<Vec<_>>();
    if original.len() > MAX_DIFF_UNITS || modified.len() > MAX_DIFF_UNITS {
        return Ok(None);
    }
    let matches = insertion_matches(&tokens(&original, true), &tokens(&modified, true))
        .or_else(|| insertion_matches(&tokens(&original, false), &tokens(&modified, false)));
    let Some(matches) = matches else {
        return Ok(None);
    };
    let mut insertions = Vec::new();
    let mut previous = 0;
    for (position, matched) in matches.into_iter().enumerate() {
        if previous < matched {
            insertions.push((position, previous..matched));
        }
        previous = matched + 1;
    }
    if previous < modified.len() {
        insertions.push((original.len(), previous..modified.len()));
    }
    if options.mode == Mode::Prefix
        && (insertions.len() > 1
            || insertions
                .first()
                .is_some_and(|(position, _)| *position != original.len()))
    {
        return Ok(None);
    }
    let start_units = document
        .rope
        .char_to_utf16_cu(byte_to_char(&document.rope, start)?);
    let preview_start = modified.len().saturating_sub(options.preview_suffix_utf16);
    let mut parts = Vec::new();
    for (position, range) in insertions {
        let units = start_units
            .checked_add(position)
            .ok_or(EditorError::Capacity)?;
        let character = document
            .rope
            .try_utf16_cu_to_char(units)
            .map_err(|_| EditorError::InvalidBoundary)?;
        if document.rope.char_to_utf16_cu(character) != units {
            return Ok(None);
        }
        let byte = document.rope.char_to_byte(character);
        if options.mode == Mode::SubwordSmart
            && document.rope.byte_to_line(cursor) == line
            && byte < cursor
        {
            return Ok(None);
        }
        let split = preview_start.clamp(range.start, range.end);
        for (range, is_preview) in [(range.start..split, false), (split..range.end, true)] {
            if range.is_empty() {
                continue;
            }
            let Ok(text) = String::from_utf16(&modified[range]) else {
                return Ok(None);
            };
            parts.push(Part {
                byte,
                text,
                is_preview,
            });
        }
    }
    Ok(Some(GhostText { line, parts }))
}

fn common_prefix(source: &str, replacement: &str) -> (usize, usize) {
    let mut original = source.char_indices().peekable();
    let mut modified = replacement.char_indices();
    let mut ends = (0, 0);
    while let Some((offset, character)) = original.next() {
        let mut original_end = offset + character.len_utf8();
        let character = if character == '\r' {
            if original
                .peek()
                .is_some_and(|(_, character)| *character == '\n')
            {
                let (offset, character) = original.next().unwrap();
                original_end = offset + character.len_utf8();
            }
            '\n'
        } else {
            character
        };
        let Some((offset, modified)) = modified.next() else {
            break;
        };
        if character != modified {
            break;
        }
        ends = (original_end, offset + modified.len_utf8());
    }
    ends
}

fn tokens(units: &[u16], smart: bool) -> Vec<Token> {
    let mut level = 0usize;
    let mut group = 0usize;
    units
        .iter()
        .map(|code| {
            let bracket = if smart && *code == OPEN_PARENTHESIS {
                let bracket = Some((group, level));
                level += 1;
                bracket
            } else if smart && *code == CLOSE_PARENTHESIS {
                level = level.saturating_sub(1);
                let bracket = Some((group, level));
                if level == 0 {
                    group += 1;
                }
                bracket
            } else {
                None
            };
            Token {
                code: *code,
                bracket,
            }
        })
        .collect()
}

fn insertion_matches(original: &[Token], modified: &[Token]) -> Option<Vec<usize>> {
    let mut cursor = 0;
    for token in original {
        while cursor < modified.len() && modified[cursor] != *token {
            cursor += 1;
        }
        if cursor == modified.len() {
            return None;
        }
        cursor += 1;
    }
    let mut matches = vec![0; original.len()];
    let mut pending = vec![(0..original.len(), 0..modified.len())];
    while let Some((mut source, mut target)) = pending.pop() {
        while source.start < source.end
            && target.start < target.end
            && original[source.start] == modified[target.start]
        {
            matches[source.start] = target.start;
            source.start += 1;
            target.start += 1;
        }
        while source.end > source.start
            && target.end > target.start
            && original[source.end - 1] == modified[target.end - 1]
        {
            source.end -= 1;
            target.end -= 1;
            matches[source.end] = target.end;
        }
        if source.is_empty() {
            continue;
        }
        let differences = target.len().checked_sub(source.len())?;
        if differences == 0 {
            return None;
        }
        let mut forward = source.start;
        let mut next = target.start;
        let mut forward_before_last = forward;
        for _ in 0..differences.div_ceil(DIRECTIONS) {
            forward_before_last = forward;
            next += 1;
            while forward < source.end && next < target.end && original[forward] == modified[next] {
                matches[forward] = next;
                forward += 1;
                next += 1;
            }
        }
        let mut reverse = source.end;
        let mut previous = target.end;
        let mut reverse_before_last = reverse;
        let mut reverse_matches = Vec::new();
        for _ in 0..differences / DIRECTIONS {
            reverse_before_last = reverse;
            previous -= 1;
            while reverse > source.start
                && previous > target.start
                && original[reverse - 1] == modified[previous - 1]
            {
                reverse -= 1;
                previous -= 1;
                reverse_matches.push((reverse, previous));
            }
        }
        let odd = differences % DIRECTIONS != 0;
        let crossed = if odd {
            forward_before_last > reverse
        } else {
            reverse_before_last < forward
        };
        if crossed {
            let (middle_source, middle_target) = if odd {
                (forward, next)
            } else {
                (reverse, previous)
            };
            if middle_target <= target.start || middle_target >= target.end {
                return None;
            }
            pending.push((middle_source..source.end, middle_target..target.end));
            pending.push((source.start..middle_source, target.start..middle_target));
        } else {
            for (position, matched) in reverse_matches {
                if position >= forward {
                    matches[position] = matched;
                }
            }
        }
    }
    if matches.iter().enumerate().any(|(index, matched)| {
        original[index] != modified[*matched] || (index > 0 && matches[index - 1] >= *matched)
    }) {
        return None;
    }
    Some(matches)
}

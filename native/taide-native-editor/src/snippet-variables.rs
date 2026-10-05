use crate::document::{DocumentSnapshot, EditorError, byte_to_char};
use crate::editing::line_content_range;
use crate::snippet_expansion::VariableContext;
use crate::view::{Selection, SelectionSet};

pub struct OvertypedText<'a> {
    pub value: &'a str,
    pub multiline: bool,
}

pub struct SelectionVariables<'a> {
    document: &'a DocumentSnapshot,
    selection: Selection,
    cursor_index: usize,
    max_bytes: usize,
}

impl<'a> SelectionVariables<'a> {
    pub fn new(
        document: &'a DocumentSnapshot,
        selections: &SelectionSet,
        cursor_index: usize,
        max_bytes: usize,
    ) -> Result<Self, EditorError> {
        selections.validate(&document.rope)?;
        let selection = *selections
            .selections
            .get(cursor_index)
            .ok_or(EditorError::InvalidBoundary)?;
        for byte in [selection.anchor, selection.head] {
            let line = document.rope.byte_to_line(byte);
            if byte > line_content_range(document, line).end {
                return Err(EditorError::InvalidBoundary);
            }
        }
        Ok(Self {
            document,
            selection,
            cursor_index,
            max_bytes,
        })
    }

    pub fn resolve(
        &self,
        context: VariableContext<'_>,
        overtyped: Option<OvertypedText<'_>>,
        mut word_at_position: impl FnMut(
            &DocumentSnapshot,
            usize,
        ) -> Result<Option<String>, EditorError>,
    ) -> Result<Option<String>, EditorError> {
        let rope = &self.document.rope;
        let position_line = rope.byte_to_line(self.selection.head);
        let value = match context.name {
            "SELECTION" | "TM_SELECTED_TEXT" => {
                let start = self.selection.anchor.min(self.selection.head);
                let end = self.selection.anchor.max(self.selection.head);
                let start_line = rope.byte_to_line(start);
                let mut multiline = start_line != rope.byte_to_line(end);
                if end - start > self.max_bytes {
                    return Err(EditorError::Capacity);
                }
                let mut value = rope.byte_slice(start..end).to_string();
                if value.is_empty()
                    && let Some(overtyped) = overtyped
                {
                    if overtyped.value.len() > self.max_bytes {
                        return Err(EditorError::Capacity);
                    }
                    value = overtyped.value.into();
                    multiline = overtyped.multiline;
                    if value.is_empty() {
                        return Ok(Some(value));
                    }
                }
                if value.is_empty() {
                    return Ok(None);
                }
                if multiline {
                    let line_start = rope.line_to_byte(start_line);
                    byte_to_char(rope, start)?;
                    let line_leading = rope
                        .byte_slice(line_start..start)
                        .chars()
                        .take_while(|character| matches!(character, ' ' | '\t'))
                        .take(self.max_bytes.saturating_add(1))
                        .collect::<String>();
                    if line_leading.len() > self.max_bytes {
                        return Err(EditorError::Capacity);
                    }
                    let leading = match context.preceding_text_line {
                        Some(line) => {
                            let length = line
                                .bytes()
                                .take_while(|byte| matches!(byte, b' ' | b'\t'))
                                .count();
                            &line[..length]
                        }
                        None => line_leading.as_str(),
                    };
                    if leading.len() > self.max_bytes {
                        return Err(EditorError::Capacity);
                    }
                    let common = leading
                        .bytes()
                        .zip(line_leading.bytes())
                        .take_while(|(left, right)| left == right)
                        .count();
                    let extra = &leading[common..];
                    let mut adjusted = String::new();
                    let mut cursor = 0;
                    while cursor < value.len() {
                        let start = cursor;
                        while cursor < value.len()
                            && !matches!(value.as_bytes()[cursor], b'\r' | b'\n')
                        {
                            cursor += 1;
                        }
                        if adjusted.len().saturating_add(cursor - start) > self.max_bytes {
                            return Err(EditorError::Capacity);
                        }
                        adjusted.push_str(&value[start..cursor]);
                        if cursor == value.len() {
                            break;
                        }
                        let newline = cursor;
                        let carriage_return = value.as_bytes()[cursor] == b'\r';
                        cursor += 1;
                        if carriage_return && value.as_bytes().get(cursor) == Some(&b'\n') {
                            cursor += 1;
                        }
                        let length = adjusted
                            .len()
                            .checked_add(cursor - newline)
                            .and_then(|length| length.checked_add(extra.len()))
                            .ok_or(EditorError::Capacity)?;
                        if length > self.max_bytes {
                            return Err(EditorError::Capacity);
                        }
                        adjusted.push_str(&value[newline..cursor]);
                        adjusted.push_str(extra);
                    }
                    value = adjusted;
                }
                Some(value)
            }
            "TM_CURRENT_LINE" => {
                let range = line_content_range(self.document, position_line);
                if range.len() > self.max_bytes {
                    return Err(EditorError::Capacity);
                }
                Some(rope.byte_slice(range).to_string())
            }
            "TM_CURRENT_WORD" => word_at_position(self.document, self.selection.head)?
                .filter(|value| !value.is_empty()),
            "TM_LINE_INDEX" => Some(position_line.to_string()),
            "TM_LINE_NUMBER" => Some((position_line + 1).to_string()),
            "CURSOR_INDEX" => Some(self.cursor_index.to_string()),
            "CURSOR_NUMBER" => Some((self.cursor_index + 1).to_string()),
            _ => None,
        };
        if value
            .as_ref()
            .is_some_and(|value| value.len() > self.max_bytes)
        {
            return Err(EditorError::Capacity);
        }
        Ok(value)
    }
}

pub fn clipboard_value(
    text: Option<&str>,
    cursor_index: usize,
    cursor_count: usize,
    spread: bool,
    max_bytes: usize,
) -> Result<Option<String>, EditorError> {
    if cursor_count == 0 || cursor_index >= cursor_count {
        return Err(EditorError::InvalidBoundary);
    }
    let Some(text) = text.filter(|text| !text.is_empty()) else {
        return Ok(None);
    };
    if text.len() > max_bytes {
        return Err(EditorError::Capacity);
    }
    if !spread {
        return Ok(Some(text.into()));
    }
    let mut lines = Vec::new();
    let mut start = 0;
    let mut cursor = 0;
    loop {
        while cursor < text.len() && !matches!(text.as_bytes()[cursor], b'\r' | b'\n') {
            cursor += 1;
        }
        let line = &text[start..cursor];
        if !line.chars().all(|character| {
            matches!(character, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
        }) {
            lines.push(line);
        }
        if cursor == text.len() {
            break;
        }
        let carriage_return = text.as_bytes()[cursor] == b'\r';
        cursor += 1;
        if carriage_return && text.as_bytes().get(cursor) == Some(&b'\n') {
            cursor += 1;
        }
        start = cursor;
    }
    if lines.len() == cursor_count {
        return Ok(Some(lines[cursor_index].into()));
    }
    Ok(Some(text.into()))
}

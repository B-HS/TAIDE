use std::ops::Range;

use crate::completion_preview::{GhostText, Part};
use crate::document::{DocumentSnapshot, EditorError, byte_to_char};
use crate::editing::line_content_range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRun {
    pub bytes: Range<usize>,
    pub source: Option<Range<usize>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdditionalLine {
    pub text: String,
    pub runs: Vec<TextRun>,
}

impl AdditionalLine {
    fn append(&mut self, text: &str, source: Option<Range<usize>>) {
        if text.is_empty() {
            return;
        }
        let start = self.text.len();
        self.text.push_str(text);
        self.runs.push(TextRun {
            bytes: start..self.text.len(),
            source,
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewData {
    pub line: usize,
    pub inline: Vec<Part>,
    pub additional: Vec<AdditionalLine>,
    pub hidden_source: Option<Range<usize>>,
}

impl ViewData {
    fn append_ghost_lines(&mut self, mut lines: impl Iterator<Item = impl AsRef<str>>) {
        let Some(first) = lines.next() else {
            return;
        };
        if let Some(last) = self.additional.last_mut() {
            last.append(first.as_ref(), None);
        } else {
            let mut line = AdditionalLine::default();
            line.append(first.as_ref(), None);
            self.additional.push(line);
        }
        for text in lines {
            let mut line = AdditionalLine::default();
            line.append(text.as_ref(), None);
            self.additional.push(line);
        }
    }

    pub fn new(document: &DocumentSnapshot, ghost: &GhostText) -> Result<Self, EditorError> {
        if ghost.line >= document.rope.len_lines() {
            return Err(EditorError::InvalidBoundary);
        }
        let source = line_content_range(document, ghost.line);
        let mut previous = source.start;
        for part in &ghost.parts {
            if part.byte < previous || !source.contains(&part.byte) && part.byte != source.end {
                return Err(EditorError::InvalidBoundary);
            }
            byte_to_char(&document.rope, part.byte)?;
            previous = part.byte;
        }
        let mut view = Self {
            line: ghost.line,
            inline: Vec::new(),
            additional: Vec::new(),
            hidden_source: None,
        };
        let mut previous = source.start;
        for part in &ghost.parts {
            let normalized = part.text.replace("\r\n", "\n").replace('\r', "\n");
            let mut lines = normalized.split('\n');
            if view.hidden_source.is_none() {
                view.inline.push(Part {
                    byte: part.byte,
                    text: lines.next().unwrap_or_default().into(),
                    is_preview: part.is_preview,
                });
                let mut lines = lines.peekable();
                let has_additional = lines.peek().is_some();
                view.append_ghost_lines(lines);
                if has_additional && part.byte < source.end {
                    view.hidden_source = Some(part.byte..source.end);
                }
            } else {
                let existing = document.rope.byte_slice(previous..part.byte).to_string();
                let last = view
                    .additional
                    .last_mut()
                    .ok_or(EditorError::InvalidBoundary)?;
                last.append(&existing, Some(previous..part.byte));
                view.append_ghost_lines(lines);
            }
            previous = part.byte;
        }
        if view.hidden_source.is_some() {
            let suffix = document.rope.byte_slice(previous..source.end).to_string();
            view.additional
                .last_mut()
                .ok_or(EditorError::InvalidBoundary)?
                .append(&suffix, Some(previous..source.end));
        }
        Ok(view)
    }
}

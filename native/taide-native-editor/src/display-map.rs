use std::borrow::Cow;
use std::ops::Range;

use ropey::{Rope, RopeSlice};

use crate::document::DocumentSnapshot;
use crate::editing::line_content_range;
use crate::line_breaks::{LineBreakData, create_line_breaks};
pub use crate::line_breaks::{WrapSettings, WrappingIndent};

const UTF8_CONTINUATION_MASK: u8 = 0xC0;
const UTF8_CONTINUATION_TAG: u8 = 0x80;
const FIRST_HIDEABLE_LINE: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowSegment {
    pub line: usize,
    pub bytes: Range<usize>,
    pub is_continuation: bool,
    pub indent_columns: u32,
    pub ends_folded: bool,
}

pub fn merged_line_ranges(
    ranges: impl IntoIterator<Item = Range<usize>>,
    line_count: usize,
) -> Vec<Range<usize>> {
    let mut sorted: Vec<Range<usize>> = ranges
        .into_iter()
        .map(|range| range.start.max(FIRST_HIDEABLE_LINE)..range.end.min(line_count))
        .filter(|range| range.start < range.end)
        .collect();
    sorted.sort_by_key(|range| range.start);
    let mut merged: Vec<Range<usize>> = Vec::with_capacity(sorted.len());
    for range in sorted {
        match merged.last_mut() {
            Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
            _ => merged.push(range),
        }
    }
    merged
}

#[derive(Debug, Clone, Default, PartialEq)]
struct HiddenLines {
    ranges: Vec<Range<usize>>,
    shown_before: Vec<usize>,
}

impl HiddenLines {
    fn new(ranges: Vec<Range<usize>>) -> Self {
        let mut hidden = 0;
        let shown_before = ranges
            .iter()
            .map(|range| {
                let shown = range.start - hidden;
                hidden += range.len();
                shown
            })
            .collect();
        Self {
            ranges,
            shown_before,
        }
    }

    fn count(&self) -> usize {
        self.ranges
            .last()
            .zip(self.shown_before.last())
            .map_or(0, |(range, shown)| range.end - shown)
    }

    fn containing(&self, line: usize) -> Option<&Range<usize>> {
        let preceding = self.ranges.partition_point(|range| range.start <= line);
        self.ranges[..preceding]
            .last()
            .filter(|range| line < range.end)
    }

    fn shown_lines_before(&self, line: usize) -> usize {
        let preceding = self.ranges.partition_point(|range| range.start < line);
        match preceding.checked_sub(1) {
            Some(last) => self.shown_before[last] + line - line.min(self.ranges[last].end),
            None => line,
        }
    }

    fn shown_line(&self, index: usize) -> usize {
        let preceding = self.shown_before.partition_point(|shown| *shown <= index);
        match preceding.checked_sub(1) {
            Some(last) => index + self.ranges[last].end - self.shown_before[last],
            None => index,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct WrappedLines {
    settings: WrapSettings,
    rope: Rope,
    breaks: Vec<Option<Box<LineBreakData>>>,
    first_rows: Vec<usize>,
}

impl WrappedLines {
    fn new(rope: &Rope, settings: WrapSettings) -> Self {
        let mut wrapped = Self {
            settings,
            rope: rope.clone(),
            breaks: rope
                .lines()
                .map(|line| line_breaks(&settings, line))
                .collect(),
            first_rows: vec![0],
        };
        wrapped.number_rows_from(0, &HiddenLines::default());
        wrapped
    }

    fn number_rows_from(&mut self, line: usize, hidden: &HiddenLines) {
        self.first_rows.truncate(line + 1);
        let mut next = self.first_rows[line];
        for (index, data) in self.breaks.iter().enumerate().skip(line) {
            if hidden.containing(index).is_none() {
                next += data.as_ref().map_or(1, |data| data.break_offsets.len());
            }
            self.first_rows.push(next);
        }
    }

    fn row_count(&self) -> usize {
        self.first_rows[self.breaks.len()]
    }

    fn locate(&self, row: usize) -> (usize, usize) {
        let row = row.min(self.row_count().saturating_sub(1));
        let line = self
            .first_rows
            .partition_point(|first| *first <= row)
            .saturating_sub(1);
        (line, row - self.first_rows[line])
    }

    fn update(&mut self, rope: &Rope) -> usize {
        let shorter = self.rope.len_bytes().min(rope.len_bytes());
        let mut prefix = shared_bytes(self.rope.chunks(), rope.chunks(), shorter, false);
        while prefix > 0 && is_continuation_byte(&self.rope, prefix) {
            prefix -= 1;
        }
        let mut suffix = shared_bytes(
            self.rope.chunks_at_byte(self.rope.len_bytes()).0.reversed(),
            rope.chunks_at_byte(rope.len_bytes()).0.reversed(),
            shorter - prefix,
            true,
        );
        while suffix > 0 && is_continuation_byte(&self.rope, self.rope.len_bytes() - suffix) {
            suffix -= 1;
        }
        let first = self
            .rope
            .byte_to_line(prefix)
            .min(rope.byte_to_line(prefix));
        let unchanged_after =
            |rope: &Rope| rope.len_lines() - 1 - rope.byte_to_line(rope.len_bytes() - suffix);
        let trailing = unchanged_after(&self.rope).min(unchanged_after(rope));
        let replaced = first..self.rope.len_lines() - trailing;
        let replacement: Vec<_> = rope
            .lines_at(first)
            .take(rope.len_lines() - trailing - first)
            .map(|line| line_breaks(&self.settings, line))
            .collect();
        self.breaks.splice(replaced, replacement);
        self.rope = rope.clone();
        first
    }
}

fn line_breaks(settings: &WrapSettings, line: RopeSlice<'_>) -> Option<Box<LineBreakData>> {
    let text: Cow<'_, str> = line.into();
    let text = text.strip_suffix('\n').unwrap_or(&text);
    let text = text.strip_suffix('\r').unwrap_or(text);
    create_line_breaks(settings, text).map(Box::new)
}

fn is_continuation_byte(rope: &Rope, byte: usize) -> bool {
    byte < rope.len_bytes() && rope.byte(byte) & UTF8_CONTINUATION_MASK == UTF8_CONTINUATION_TAG
}

fn shared_bytes<'a>(
    mut old: impl Iterator<Item = &'a str>,
    mut new: impl Iterator<Item = &'a str>,
    limit: usize,
    backward: bool,
) -> usize {
    let (mut left, mut right): (&[u8], &[u8]) = (&[], &[]);
    let mut shared = 0;
    while shared < limit {
        while left.is_empty() {
            match old.next() {
                Some(chunk) => left = chunk.as_bytes(),
                None => return shared,
            }
        }
        while right.is_empty() {
            match new.next() {
                Some(chunk) => right = chunk.as_bytes(),
                None => return shared,
            }
        }
        let length = left.len().min(right.len()).min(limit - shared);
        let (left_rest, left_part, right_rest, right_part) = if backward {
            let (left_rest, left_part) = left.split_at(left.len() - length);
            let (right_rest, right_part) = right.split_at(right.len() - length);
            (left_rest, left_part, right_rest, right_part)
        } else {
            let (left_part, left_rest) = left.split_at(length);
            let (right_part, right_rest) = right.split_at(length);
            (left_rest, left_part, right_rest, right_part)
        };
        if left_part != right_part {
            let pairs = left_part.iter().zip(right_part);
            return shared
                + if backward {
                    pairs
                        .rev()
                        .take_while(|(left, right)| left == right)
                        .count()
                } else {
                    pairs.take_while(|(left, right)| left == right).count()
                };
        }
        shared += length;
        (left, right) = (left_rest, right_rest);
    }
    shared
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayMap {
    line_count: usize,
    revision: u64,
    wrapped: Option<WrappedLines>,
    hidden: HiddenLines,
}

impl DisplayMap {
    pub fn identity(line_count: usize, revision: u64) -> Self {
        Self {
            line_count,
            revision,
            wrapped: None,
            hidden: HiddenLines::default(),
        }
    }

    pub fn build(document: &DocumentSnapshot, wrap: Option<WrapSettings>) -> Self {
        Self {
            line_count: document.rope.len_lines(),
            revision: document.revision,
            wrapped: wrap.map(|settings| WrappedLines::new(&document.rope, settings)),
            hidden: HiddenLines::default(),
        }
    }

    pub fn refresh(&mut self, document: &DocumentSnapshot) {
        if self.revision == document.revision {
            return;
        }
        let previous = std::mem::take(&mut self.hidden);
        if let Some(wrapped) = &mut self.wrapped {
            let changed = wrapped.update(&document.rope);
            let renumbered = previous
                .ranges
                .first()
                .map_or(changed, |range| changed.min(range.start));
            wrapped.number_rows_from(renumbered, &self.hidden);
        }
        self.line_count = document.rope.len_lines();
        self.revision = document.revision;
    }

    pub fn set_hidden_lines(&mut self, hidden: &[Range<usize>]) -> bool {
        let ranges = merged_line_ranges(hidden.iter().cloned(), self.line_count);
        if ranges == self.hidden.ranges {
            return false;
        }
        let renumbered = [self.hidden.ranges.first(), ranges.first()]
            .into_iter()
            .flatten()
            .map(|range| range.start)
            .min()
            .unwrap_or(0);
        self.hidden = HiddenLines::new(ranges);
        if let Some(wrapped) = &mut self.wrapped {
            wrapped.number_rows_from(renumbered, &self.hidden);
        }
        true
    }

    pub fn hidden_lines(&self) -> &[Range<usize>] {
        &self.hidden.ranges
    }

    pub fn hidden_lines_at(&self, line: usize) -> Option<Range<usize>> {
        self.hidden.containing(line).cloned()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn wrap_settings(&self) -> Option<&WrapSettings> {
        self.wrapped.as_ref().map(|wrapped| &wrapped.settings)
    }

    pub fn row_count(&self) -> usize {
        self.wrapped.as_ref().map_or(
            self.line_count - self.hidden.count(),
            WrappedLines::row_count,
        )
    }

    pub fn rows_of_line(&self, line: usize) -> Range<usize> {
        let line = line.min(self.line_count.saturating_sub(1));
        match &self.wrapped {
            Some(wrapped) => wrapped.first_rows[line]..wrapped.first_rows[line + 1],
            None => {
                let first = self.hidden.shown_lines_before(line);
                first..first + usize::from(self.hidden.containing(line).is_none())
            }
        }
    }

    pub fn row_start_column(&self, row: usize) -> f64 {
        let Some(wrapped) = &self.wrapped else {
            return 0.0;
        };
        let (line, index) = wrapped.locate(row);
        wrapped.breaks[line]
            .as_ref()
            .zip(index.checked_sub(1))
            .map_or(0.0, |(data, previous)| {
                data.break_offsets_visible_column[previous]
            })
    }

    pub fn segment(&self, document: &DocumentSnapshot, row: usize) -> RowSegment {
        let Some(wrapped) = &self.wrapped else {
            let line = self
                .hidden
                .shown_line(row.min(self.row_count().saturating_sub(1)));
            return RowSegment {
                line,
                bytes: line_content_range(document, line),
                is_continuation: false,
                indent_columns: 0,
                ends_folded: self.hidden.containing(line + 1).is_some(),
            };
        };
        let (line, index) = wrapped.locate(row);
        let content = line_content_range(document, line);
        let heads_hidden_lines = self.hidden.containing(line + 1).is_some();
        let Some(data) = &wrapped.breaks[line] else {
            return RowSegment {
                line,
                bytes: content,
                is_continuation: false,
                indent_columns: 0,
                ends_folded: heads_hidden_lines,
            };
        };
        let is_continuation = index > 0;
        let start = index
            .checked_sub(1)
            .map_or(0, |previous| data.break_offsets[previous]);
        let boundary = |offset: usize| (content.start + offset).min(content.end);
        RowSegment {
            line,
            bytes: boundary(start)..boundary(data.break_offsets[index]),
            is_continuation,
            indent_columns: if is_continuation {
                data.wrapped_text_indent_length
            } else {
                0
            },
            ends_folded: heads_hidden_lines && index + 1 == data.break_offsets.len(),
        }
    }

    pub fn row_of_byte(&self, document: &DocumentSnapshot, byte: usize) -> usize {
        let line = document
            .rope
            .byte_to_line(byte.min(document.rope.len_bytes()));
        if let Some(hidden) = self.hidden.containing(line) {
            return self.rows_of_line(hidden.start - 1).end - 1;
        }
        let Some(wrapped) = &self.wrapped else {
            return self.hidden.shown_lines_before(line);
        };
        let line = line.min(self.line_count.saturating_sub(1));
        let first = wrapped.first_rows[line];
        let Some(data) = &wrapped.breaks[line] else {
            return first;
        };
        let offset = byte.saturating_sub(document.rope.line_to_byte(line));
        first
            + data
                .break_offsets
                .partition_point(|end| *end <= offset)
                .min(data.break_offsets.len() - 1)
    }

    pub fn row_of_head(&self, document: &DocumentSnapshot, byte: usize, at_row_end: bool) -> usize {
        let row = self.row_of_byte(document, byte);
        if !at_row_end || row == 0 {
            return row;
        }
        let segment = self.segment(document, row);
        if segment.is_continuation && segment.bytes.start == byte {
            row - 1
        } else {
            row
        }
    }
}

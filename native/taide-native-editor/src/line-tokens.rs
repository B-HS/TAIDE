use std::ops::Range;

use crate::change_journal::{ChangeSet, ChangeSpan};
use crate::syntax::TokenKind;

const SPAN_FIELDS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenStyle {
    pub foreground: [u8; 4],
    pub is_italic: bool,
    pub is_bold: bool,
    pub is_underlined: bool,
    pub is_struck_through: bool,
    pub kind: TokenKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenStyleTable {
    default_style: TokenStyle,
    styles: Vec<TokenStyle>,
}

impl TokenStyleTable {
    pub fn new(default_style: TokenStyle, styles: Vec<TokenStyle>) -> Self {
        Self {
            default_style,
            styles,
        }
    }

    pub fn len(&self) -> usize {
        self.styles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.styles.is_empty()
    }

    pub fn default_style(&self) -> TokenStyle {
        self.default_style
    }

    pub fn style(&self, style_id: u32) -> TokenStyle {
        usize::try_from(style_id)
            .ok()
            .and_then(|index| self.styles.get(index))
            .copied()
            .unwrap_or(self.default_style)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct InvalidLines {
    ranges: Vec<Range<usize>>,
}

impl InvalidLines {
    fn first(&self) -> Option<usize> {
        self.ranges.first().map(|range| range.start)
    }

    fn contains(&self, line: usize) -> bool {
        self.ranges.iter().any(|range| range.contains(&line))
    }

    fn delete(&mut self, line: usize) {
        let Some(index) = self.ranges.iter().position(|range| range.contains(&line)) else {
            return;
        };
        let range = self.ranges[index].clone();
        let before = range.start..line;
        let after = line + 1..range.end;
        self.ranges.splice(
            index..=index,
            [before, after].into_iter().filter(|part| !part.is_empty()),
        );
    }

    fn add_range(&mut self, range: Range<usize>) {
        if range.is_empty() {
            return;
        }
        let first = self
            .ranges
            .iter()
            .position(|existing| existing.end >= range.start)
            .unwrap_or(self.ranges.len());
        let after = self.ranges[first..]
            .iter()
            .position(|existing| existing.start > range.end)
            .map_or(self.ranges.len(), |offset| first + offset);
        if first == after {
            self.ranges.insert(first, range);
            return;
        }
        let merged =
            range.start.min(self.ranges[first].start)..range.end.max(self.ranges[after - 1].end);
        self.ranges.splice(first..after, [merged]);
    }

    fn add_range_and_resize(&mut self, range: Range<usize>, new_length: usize) {
        let first = self
            .ranges
            .iter()
            .position(|existing| range.start <= existing.end)
            .unwrap_or(self.ranges.len());
        let after = self.ranges[first..]
            .iter()
            .position(|existing| range.end < existing.start)
            .map_or(self.ranges.len(), |offset| first + offset);
        let old_length = range.end - range.start;
        for existing in &mut self.ranges[after..] {
            *existing =
                existing.start + new_length - old_length..existing.end + new_length - old_length;
        }
        let resized = if first == after {
            range.start..range.start + new_length
        } else {
            range.start.min(self.ranges[first].start)
                ..range.end.max(self.ranges[after - 1].end) + new_length - old_length
        };
        self.ranges.splice(
            first..after,
            [resized].into_iter().filter(|part| !part.is_empty()),
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineTokens {
    lines: Vec<Vec<u32>>,
    invalid: InvalidLines,
}

impl LineTokens {
    pub fn new(line_count: usize) -> Self {
        let mut invalid = InvalidLines::default();
        invalid.add_range(0..line_count);
        Self {
            lines: vec![Vec::new(); line_count],
            invalid,
        }
    }

    pub fn reset(&mut self, line_count: usize) {
        *self = Self::new(line_count);
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn spans(&self, line: usize) -> &[u32] {
        self.lines.get(line).map_or(&[], Vec::as_slice)
    }

    pub fn invalid_ranges(&self) -> &[Range<usize>] {
        &self.invalid.ranges
    }

    pub fn first_invalid_line(&self) -> Option<usize> {
        self.invalid.first()
    }

    pub fn is_valid(&self, line: usize) -> bool {
        line < self.lines.len() && !self.invalid.contains(line)
    }

    pub fn has_accurate_tokens(&self, line: usize) -> bool {
        line < self.lines.len() && self.invalid.first().is_none_or(|first| line < first)
    }

    pub fn invalidate(&mut self, lines: Range<usize>) {
        self.invalid
            .add_range(lines.start..lines.end.min(self.lines.len()));
    }

    pub fn set_line(&mut self, line: usize, spans: Vec<u32>, has_end_state_changed: bool) {
        let Some(stored) = self.lines.get_mut(line) else {
            return;
        };
        *stored = spans;
        self.invalid.delete(line);
        if has_end_state_changed && line + 1 < self.lines.len() {
            self.invalid.add_range(line + 1..line + 2);
        }
    }

    pub fn apply(&mut self, changes: &ChangeSet) -> bool {
        let is_applied = self.lines.len() == changes.line_count_before
            && changes.spans.iter().rev().all(|span| self.apply_span(span))
            && self.lines.len() == changes.line_count_after;
        if !is_applied {
            self.reset(changes.line_count_after);
        }
        is_applied
    }

    fn apply_span(&mut self, span: &ChangeSpan) -> bool {
        let first = span.start.line;
        let old_last = span.old_end.line;
        if old_last < first || span.new_end.line < first || old_last >= self.lines.len() {
            return false;
        }
        if first == old_last {
            let kept = delete_range(
                std::mem::take(&mut self.lines[first]),
                span.start.column,
                span.old_end.column,
            );
            self.lines[first] = kept;
        } else {
            let head = delete_ending(std::mem::take(&mut self.lines[first]), span.start.column);
            let tail = delete_range(
                std::mem::take(&mut self.lines[old_last]),
                0,
                span.old_end.column,
            );
            self.lines[first] = append(head, &tail, span.start.column);
            self.lines.drain(first + 1..=old_last);
        }
        let inserted_lines = span.new_end.line - first;
        if inserted_lines == 0 {
            let inserted_bytes = span.new_end.column.saturating_sub(span.start.column);
            insert(&mut self.lines[first], span.start.column, inserted_bytes);
        } else {
            let head = delete_ending(std::mem::take(&mut self.lines[first]), span.start.column);
            self.lines[first] = head;
            self.lines.splice(
                first + 1..first + 1,
                std::iter::repeat_n(Vec::new(), inserted_lines),
            );
        }
        self.invalid
            .add_range_and_resize(first..old_last + 1, inserted_lines + 1);
        true
    }
}

fn byte_offset(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn delete_range(mut spans: Vec<u32>, from: usize, to: usize) -> Vec<u32> {
    if spans.is_empty() || from >= to {
        return spans;
    }
    let from = byte_offset(from);
    let to = byte_offset(to);
    let removed = to - from;
    let mut kept = 0;
    for index in (0..spans.len()).step_by(SPAN_FIELDS) {
        let start = spans[index];
        let style = spans[index + 1];
        let moved = if start <= from {
            start
        } else if start < to {
            from
        } else {
            start - removed
        };
        if kept >= SPAN_FIELDS && spans[kept - SPAN_FIELDS] == moved {
            kept -= SPAN_FIELDS;
        }
        spans[kept] = moved;
        spans[kept + 1] = style;
        kept += SPAN_FIELDS;
    }
    spans.truncate(kept);
    spans
}

fn delete_ending(mut spans: Vec<u32>, from: usize) -> Vec<u32> {
    let from = byte_offset(from);
    let kept = spans
        .as_chunks::<SPAN_FIELDS>()
        .0
        .iter()
        .take_while(|[start, _]| *start < from)
        .count();
    spans.truncate(kept * SPAN_FIELDS);
    spans
}

fn append(mut head: Vec<u32>, tail: &[u32], offset: usize) -> Vec<u32> {
    let offset = byte_offset(offset);
    head.extend(
        tail.as_chunks::<SPAN_FIELDS>()
            .0
            .iter()
            .flat_map(|[start, style]| [start.saturating_add(offset), *style]),
    );
    head
}

fn insert(spans: &mut [u32], at: usize, inserted_bytes: usize) {
    if inserted_bytes == 0 {
        return;
    }
    let at = byte_offset(at);
    let inserted_bytes = byte_offset(inserted_bytes);
    for [start, _] in spans.as_chunks_mut::<SPAN_FIELDS>().0 {
        if *start != 0 && *start >= at {
            *start = start.saturating_add(inserted_bytes);
        }
    }
}

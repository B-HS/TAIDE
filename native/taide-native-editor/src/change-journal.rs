use std::collections::VecDeque;
use std::collections::vec_deque;

use ropey::Rope;

use crate::document::{Edit, byte_to_char};

pub const MAX_JOURNAL_ENTRIES: usize = 64;
pub const MAX_JOURNAL_SPANS: usize = 1024;
pub const MAX_CHANGE_SET_SPANS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinePoint {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeSpan {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_end_byte: usize,
    pub start_utf16: usize,
    pub old_end_utf16: usize,
    pub new_end_utf16: usize,
    pub start: LinePoint,
    pub old_end: LinePoint,
    pub new_end: LinePoint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeSet {
    pub revision_before: u64,
    pub revision_after: u64,
    pub line_count_before: usize,
    pub line_count_after: usize,
    pub spans: Vec<ChangeSpan>,
}

impl ChangeSet {
    pub fn first_changed_line(&self) -> Option<usize> {
        self.spans.first().map(|span| span.start.line)
    }
}

pub type Changes<'a> = vec_deque::Iter<'a, ChangeSet>;

pub enum ChangesSince<'a> {
    Tracked(Changes<'a>),
    Lagged,
}

#[derive(Debug, Default)]
pub(crate) struct ChangeJournal {
    entries: VecDeque<ChangeSet>,
    span_count: usize,
}

impl ChangeJournal {
    pub(crate) fn record_edits(
        &mut self,
        revisions: (u64, u64),
        before: &Rope,
        after: &Rope,
        edits: &[Edit],
    ) {
        let spans = edit_spans(before, after, edits)
            .unwrap_or_else(|| difference_span(before, after).into_iter().collect());
        self.push(revisions, before, after, spans);
    }

    pub(crate) fn record_replacement(
        &mut self,
        revisions: (u64, u64),
        before: &Rope,
        after: &Rope,
    ) {
        let spans = difference_span(before, after).into_iter().collect();
        self.push(revisions, before, after, spans);
    }

    pub(crate) fn since(&self, revision: u64, current_revision: u64) -> ChangesSince<'_> {
        if revision == current_revision {
            return ChangesSince::Tracked(self.entries.range(self.entries.len()..));
        }
        if self
            .entries
            .back()
            .is_none_or(|last| last.revision_after != current_revision)
        {
            return ChangesSince::Lagged;
        }
        match self
            .entries
            .iter()
            .position(|entry| entry.revision_before == revision)
        {
            Some(index) => ChangesSince::Tracked(self.entries.range(index..)),
            None => ChangesSince::Lagged,
        }
    }

    fn push(
        &mut self,
        (revision_before, revision_after): (u64, u64),
        before: &Rope,
        after: &Rope,
        spans: Vec<ChangeSpan>,
    ) {
        if self
            .entries
            .back()
            .is_some_and(|last| last.revision_after != revision_before)
        {
            self.entries.clear();
            self.span_count = 0;
        }
        self.span_count += spans.len();
        self.entries.push_back(ChangeSet {
            revision_before,
            revision_after,
            line_count_before: before.len_lines(),
            line_count_after: after.len_lines(),
            spans,
        });
        while self.entries.len() > MAX_JOURNAL_ENTRIES
            || (self.span_count > MAX_JOURNAL_SPANS && self.entries.len() > 1)
        {
            let Some(dropped) = self.entries.pop_front() else {
                break;
            };
            self.span_count -= dropped.spans.len();
        }
    }
}

fn point(rope: &Rope, byte: usize) -> LinePoint {
    let line = rope.byte_to_line(byte);
    LinePoint {
        line,
        column: byte - rope.line_to_byte(line),
    }
}

fn edit_spans(before: &Rope, after: &Rope, edits: &[Edit]) -> Option<Vec<ChangeSpan>> {
    if edits.len() > MAX_CHANGE_SET_SPANS {
        return None;
    }
    let mut spans = Vec::with_capacity(edits.len());
    let mut inserted_bytes = 0usize;
    let mut removed_bytes = 0usize;
    let mut inserted_lines = 0usize;
    let mut removed_lines = 0usize;
    for edit in edits {
        let start = point(before, edit.bytes.start);
        let old_end = point(before, edit.bytes.end);
        let after_start = (edit.bytes.start + inserted_bytes).checked_sub(removed_bytes)?;
        let after_end = after_start + edit.text.len();
        let after_start_line = after.byte_to_line(after_start);
        let after_end_line = after.byte_to_line(after_end);
        if after_start_line + removed_lines != start.line + inserted_lines {
            return None;
        }
        let span_lines = after_end_line - after_start_line;
        let new_end = LinePoint {
            line: start.line + span_lines,
            column: if span_lines == 0 {
                start.column + edit.text.len()
            } else {
                after_end - after.line_to_byte(after_end_line)
            },
        };
        inserted_bytes += edit.text.len();
        removed_bytes += edit.bytes.end - edit.bytes.start;
        inserted_lines += span_lines;
        removed_lines += old_end.line - start.line;
        spans.push(ChangeSpan {
            start_byte: edit.bytes.start,
            old_end_byte: edit.bytes.end,
            new_end_byte: edit.bytes.start + edit.text.len(),
            start_utf16: before.char_to_utf16_cu(before.byte_to_char(edit.bytes.start)),
            old_end_utf16: before.char_to_utf16_cu(before.byte_to_char(edit.bytes.end)),
            new_end_utf16: before.char_to_utf16_cu(before.byte_to_char(edit.bytes.start))
                + edit.text.encode_utf16().count(),
            start,
            old_end,
            new_end,
        });
    }
    (after.len_lines() + removed_lines == before.len_lines() + inserted_lines).then_some(spans)
}

fn difference_span(before: &Rope, after: &Rope) -> Option<ChangeSpan> {
    let before_len = before.len_bytes();
    let after_len = after.len_bytes();
    let mut prefix = common_prefix_len(before, after);
    while byte_to_char(before, prefix).is_err() {
        prefix -= 1;
    }
    if prefix == before_len && prefix == after_len {
        return None;
    }
    let mut suffix = common_suffix_len(before, after, before_len.min(after_len) - prefix);
    while byte_to_char(before, before_len - suffix).is_err() {
        suffix -= 1;
    }
    let old_end_byte = before_len - suffix;
    let new_end_byte = after_len - suffix;
    let start_line = before.byte_to_line(prefix).min(after.byte_to_line(prefix));
    Some(ChangeSpan {
        start_byte: prefix,
        old_end_byte,
        new_end_byte,
        start_utf16: before.char_to_utf16_cu(before.byte_to_char(prefix)),
        old_end_utf16: before.char_to_utf16_cu(before.byte_to_char(old_end_byte)),
        new_end_utf16: after.char_to_utf16_cu(after.byte_to_char(new_end_byte)),
        start: LinePoint {
            line: start_line,
            column: prefix - before.line_to_byte(start_line),
        },
        old_end: point(before, old_end_byte),
        new_end: point(after, new_end_byte),
    })
}

fn matching_prefix(left: &[u8], right: &[u8]) -> usize {
    left.iter()
        .zip(right)
        .take_while(|(left, right)| left == right)
        .count()
}

fn matching_suffix(left: &[u8], right: &[u8]) -> usize {
    left.iter()
        .rev()
        .zip(right.iter().rev())
        .take_while(|(left, right)| left == right)
        .count()
}

fn common_prefix_len(before: &Rope, after: &Rope) -> usize {
    let mut before_chunks = before.chunks();
    let mut after_chunks = after.chunks();
    let mut left: &[u8] = &[];
    let mut right: &[u8] = &[];
    let mut matched = 0;
    loop {
        if left.is_empty() {
            match before_chunks.next() {
                Some(chunk) => left = chunk.as_bytes(),
                None => return matched,
            }
        }
        if right.is_empty() {
            match after_chunks.next() {
                Some(chunk) => right = chunk.as_bytes(),
                None => return matched,
            }
        }
        let shared = left.len().min(right.len());
        if left[..shared] != right[..shared] {
            return matched + matching_prefix(left, right);
        }
        matched += shared;
        left = &left[shared..];
        right = &right[shared..];
    }
}

fn common_suffix_len(before: &Rope, after: &Rope, limit: usize) -> usize {
    let mut before_chunks = before.chunks_at_byte(before.len_bytes()).0.reversed();
    let mut after_chunks = after.chunks_at_byte(after.len_bytes()).0.reversed();
    let mut left: &[u8] = &[];
    let mut right: &[u8] = &[];
    let mut matched = 0;
    while matched < limit {
        if left.is_empty() {
            match before_chunks.next() {
                Some(chunk) => left = chunk.as_bytes(),
                None => break,
            }
        }
        if right.is_empty() {
            match after_chunks.next() {
                Some(chunk) => right = chunk.as_bytes(),
                None => break,
            }
        }
        let shared = left.len().min(right.len()).min(limit - matched);
        let left_tail = &left[left.len() - shared..];
        let right_tail = &right[right.len() - shared..];
        if left_tail != right_tail {
            return matched + matching_suffix(left_tail, right_tail);
        }
        matched += shared;
        left = &left[..left.len() - shared];
        right = &right[..right.len() - shared];
    }
    matched
}

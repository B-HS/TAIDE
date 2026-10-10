use std::ops::Range;

use ropey::Rope;
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete, UnicodeSegmentation};

use crate::display_map::{DisplayMap, RowSegment};
use crate::document::{DocumentSnapshot, Edit, EditorError, LineEnding, UndoGroup, byte_to_char};
use crate::indent::{IndentOptions, ModelIndentOptions};
use crate::store::{EditorStore, Transaction};
use crate::view::{
    EditOperation, EditRun, GoalColumns, Selection, SelectionSet, ViewId, ViewState, WrapAffinities,
};

pub(crate) const WORD_SEPARATORS: &str = "`~!@#$%^&*()-=+[{]}\\|;:'\",.<>/?";
const WIDE_CHARACTER_COLUMNS: usize = 2;
const WIDE_CHARACTER_RANGES: [(u32, u32); 16] = [
    (0x2E80, 0xD7AF),
    (0xF900, 0xFAFF),
    (0xFF01, 0xFF5E),
    (0xFFE0, 0xFFE6),
    (0x1F1E6, 0x1F1FF),
    (8986, 8987),
    (9200, 9200),
    (9203, 9203),
    (9728, 10175),
    (11088, 11088),
    (11093, 11093),
    (127744, 128591),
    (128640, 128764),
    (128992, 129008),
    (129280, 129535),
    (129648, 129782),
];

#[derive(Clone, Copy)]
pub enum Motion {
    Left,
    Right,
    WordLeft,
    WordRight,
    Vertical { lines: isize, tab_size: u32 },
    LineStart,
    LineEnd,
    DocumentStart,
    DocumentEnd,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardText {
    pub text: String,
    pub from_empty_selection: bool,
    pub multicursor: Option<Vec<String>>,
}

pub(crate) struct LineText {
    pub(crate) start: usize,
    pub(crate) text: String,
}

struct LineCharacters {
    start: usize,
    characters: Vec<char>,
    offsets: Vec<usize>,
}

impl LineCharacters {
    fn index_of(&self, byte: usize) -> usize {
        self.offsets
            .partition_point(|offset| self.start + offset < byte)
            .min(self.characters.len())
    }

    fn byte_of(&self, index: usize) -> usize {
        self.start + self.offsets[index.min(self.characters.len())]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CharacterClass {
    Regular,
    Whitespace,
    WordSeparator,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WordKind {
    Regular,
    Separator,
}

#[derive(Clone, Copy)]
struct Word {
    start: usize,
    end: usize,
    kind: WordKind,
    next_class: CharacterClass,
}

pub(crate) struct Step {
    pub(crate) operation: EditOperation,
    pub(crate) stop_before: bool,
    pub(crate) stop_after: bool,
}

pub(crate) const SEPARATE_STEP: Step = Step {
    operation: EditOperation::Other,
    stop_before: true,
    stop_after: true,
};

#[derive(Clone, Copy)]
pub(crate) enum Mark {
    AfterEdit(usize),
    InEdit { edit: usize, bytes: usize },
    BeforeEditEnd { edit: usize, bytes: usize },
    Tracked { offset: usize, sticks: bool },
    Anchored { offset: usize, line_start: usize },
}

pub(crate) struct Plan {
    edits: Vec<Edit>,
    pub(crate) marks: Vec<(Mark, Mark)>,
    primary: usize,
}

impl Plan {
    pub(crate) fn new(selection: &SelectionSet) -> Self {
        Self {
            edits: Vec::new(),
            marks: selection
                .selections
                .iter()
                .map(|selection| {
                    (
                        Mark::Tracked {
                            offset: selection.anchor,
                            sticks: false,
                        },
                        Mark::Tracked {
                            offset: selection.head,
                            sticks: false,
                        },
                    )
                })
                .collect(),
            primary: selection.primary,
        }
    }

    pub(crate) fn edit(&mut self, bytes: Range<usize>, text: String) -> Option<usize> {
        if bytes.is_empty() && text.is_empty() {
            return None;
        }
        if let Some(index) = self
            .edits
            .iter()
            .position(|edit| edit.bytes == bytes && edit.text == text)
        {
            return Some(index);
        }
        if self.edits.iter().any(|edit| {
            edit.bytes.start == bytes.start
                || (edit.bytes.start < bytes.end && bytes.start < edit.bytes.end)
        }) {
            return None;
        }
        self.edits.push(Edit { bytes, text });
        Some(self.edits.len() - 1)
    }

    pub(crate) fn replace(&mut self, index: usize, bytes: Range<usize>, text: String) {
        if let Some(edit) = self.edit(bytes, text) {
            self.marks[index] = (Mark::AfterEdit(edit), Mark::AfterEdit(edit));
        }
    }

    pub(crate) fn has_edits(&self) -> bool {
        !self.edits.is_empty()
    }

    pub(crate) fn transaction(
        self,
        document: &DocumentSnapshot,
        view: ViewId,
    ) -> Option<Transaction> {
        if self.edits.is_empty() {
            return None;
        }
        let selections = {
            let mut order: Vec<usize> = (0..self.edits.len()).collect();
            order.sort_by_key(|index| self.edits[*index].bytes.start);
            let ordered: Vec<&Edit> = order.iter().map(|index| &self.edits[*index]).collect();
            let mut insertion_ends = vec![0; self.edits.len()];
            let mut removed = 0;
            let mut inserted = 0;
            for index in order {
                let edit = &self.edits[index];
                inserted += edit.text.len();
                insertion_ends[index] = edit.bytes.start - removed + inserted;
                removed += edit.bytes.len();
            }
            let resolve = |mark: Mark| match mark {
                Mark::AfterEdit(index) => insertion_ends[index],
                Mark::InEdit { edit, bytes } => {
                    insertion_ends[edit] - self.edits[edit].text.len() + bytes
                }
                Mark::BeforeEditEnd { edit, bytes } => insertion_ends[edit] - bytes,
                Mark::Tracked { offset, sticks } => tracked_offset(&ordered, offset, sticks),
                Mark::Anchored { offset, line_start } => tracked_offset(&ordered, offset, false)
                    .min(tracked_offset(&ordered, line_start, true) + offset - line_start),
            };
            self.marks
                .iter()
                .map(|(anchor, head)| Selection {
                    anchor: resolve(*anchor),
                    head: resolve(*head),
                })
                .collect()
        };
        Some(Transaction {
            revision: document.revision,
            group: UndoGroup(document.revision),
            origin: Some(view),
            selection_after: Some(SelectionSet {
                primary: self.primary,
                selections,
            }),
            edits: self.edits,
        })
    }
}

pub(crate) fn tracked_offset(edits: &[&Edit], offset: usize, sticks: bool) -> usize {
    let mut removed_before = 0;
    let mut inserted_before = 0;
    for edit in edits {
        let (start, end) = (edit.bytes.start, edit.bytes.end);
        if offset < start {
            break;
        }
        let removed = end - start;
        let inserted = edit.text.len();
        let common = removed.min(inserted);
        let stays = if offset == start {
            removed > 0 || sticks
        } else if offset < start + common {
            true
        } else {
            offset == start + common && (removed > inserted || sticks)
        };
        if stays {
            break;
        }
        if offset < end || (offset == end && sticks) {
            return start - removed_before + inserted_before + inserted;
        }
        removed_before += removed;
        inserted_before += inserted;
    }
    offset - removed_before + inserted_before
}

pub(crate) fn view_document(
    store: &EditorStore,
    view: ViewId,
) -> Result<(ViewState, DocumentSnapshot), EditorError> {
    let current = store
        .views()
        .get(view)
        .ok_or(EditorError::NotFound)?
        .clone();
    let document = store.documents().snapshot(current.document)?;
    Ok((current, document))
}

pub(crate) fn ordered(selection: &Selection) -> Range<usize> {
    selection.anchor.min(selection.head)..selection.anchor.max(selection.head)
}

pub(crate) fn line_text(document: &DocumentSnapshot, line: usize) -> LineText {
    let range = line_content_range(document, line);
    LineText {
        start: range.start,
        text: document.rope.byte_slice(range).to_string(),
    }
}

fn line_characters(document: &DocumentSnapshot, line: usize) -> LineCharacters {
    let content = line_text(document, line);
    let mut offsets: Vec<usize> = content
        .text
        .char_indices()
        .map(|(offset, _)| offset)
        .collect();
    offsets.push(content.text.len());
    LineCharacters {
        start: content.start,
        characters: content.text.chars().collect(),
        offsets,
    }
}

pub(crate) fn leading_whitespace(text: &str) -> &str {
    let length = text
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    &text[..length]
}

fn spans_lines(document: &DocumentSnapshot, range: &Range<usize>) -> bool {
    document.rope.byte_to_line(range.start) != document.rope.byte_to_line(range.end)
}

pub(crate) fn tab_width(indent: impl Into<ModelIndentOptions>) -> usize {
    let indent = indent.into();
    (indent.tab_size as usize).max(1)
}

pub(crate) fn indent_step(indent: ModelIndentOptions) -> usize {
    if indent.insert_spaces {
        return indent.indent_size.max(1) as usize;
    }
    tab_width(indent)
}

pub(crate) fn normalization_options(indent: ModelIndentOptions) -> ModelIndentOptions {
    ModelIndentOptions {
        tab_size: indent.indent_size,
        ..indent
    }
}

pub(crate) fn next_tab_stop(column: usize, size: usize) -> usize {
    column + size - column % size
}

pub(crate) fn previous_tab_stop(column: usize, size: usize) -> usize {
    column.checked_sub(1).map_or(0, |last| last - last % size)
}

fn next_visible_column(character: char, column: usize, tab_size: usize) -> usize {
    if character == '\t' {
        return next_tab_stop(column, tab_size);
    }
    let code = u32::from(character);
    let wide = WIDE_CHARACTER_RANGES
        .iter()
        .any(|(first, last)| (*first..=*last).contains(&code));
    column + if wide { WIDE_CHARACTER_COLUMNS } else { 1 }
}

pub(crate) fn visible_column(text: &str, tab_size: usize) -> usize {
    text.graphemes(true)
        .filter_map(|grapheme| grapheme.chars().next())
        .fold(0, |column, character| {
            next_visible_column(character, column, tab_size)
        })
}

pub(crate) fn offset_at_visible_column(text: &str, visible: usize, tab_size: usize) -> usize {
    if visible == 0 {
        return 0;
    }
    let mut before_visible = 0;
    let mut before_offset = 0;
    for (offset, grapheme) in text.grapheme_indices(true) {
        let Some(character) = grapheme.chars().next() else {
            continue;
        };
        let after_visible = next_visible_column(character, before_visible, tab_size);
        let after_offset = offset + grapheme.len();
        if after_visible >= visible {
            return if after_visible - visible < visible - before_visible {
                after_offset
            } else {
                before_offset
            };
        }
        before_visible = after_visible;
        before_offset = after_offset;
    }
    text.len()
}

pub(crate) fn indentation(columns: usize, indent: impl Into<ModelIndentOptions>) -> String {
    let indent = indent.into();
    if indent.insert_spaces {
        return " ".repeat(columns);
    }
    let size = tab_width(indent);
    format!(
        "{}{}",
        "\t".repeat(columns / size),
        " ".repeat(columns % size)
    )
}

pub fn normalize_line_breaks(text: &str, line_break: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\r' => {
                if characters.peek() == Some(&'\n') {
                    characters.next();
                }
                normalized.push_str(line_break);
            }
            '\n' => normalized.push_str(line_break),
            other => normalized.push(other),
        }
    }
    normalized
}

pub(crate) fn character_class(character: char) -> CharacterClass {
    if matches!(character, ' ' | '\t') {
        CharacterClass::Whitespace
    } else if WORD_SEPARATORS.contains(character) {
        CharacterClass::WordSeparator
    } else {
        CharacterClass::Regular
    }
}

fn ends_word(kind: WordKind, class: CharacterClass) -> bool {
    match class {
        CharacterClass::Whitespace => true,
        CharacterClass::WordSeparator => kind == WordKind::Regular,
        CharacterClass::Regular => kind == WordKind::Separator,
    }
}

fn word_end(characters: &[char], kind: WordKind, start: usize) -> usize {
    (start..characters.len())
        .find(|index| ends_word(kind, character_class(characters[*index])))
        .unwrap_or(characters.len())
}

fn word_start(characters: &[char], kind: WordKind, last: usize) -> usize {
    (0..=last)
        .rev()
        .find(|index| ends_word(kind, character_class(characters[*index])))
        .map_or(0, |index| index + 1)
}

fn previous_word(characters: &[char], cursor: usize) -> Option<Word> {
    let mut kind = None;
    for index in (0..cursor.min(characters.len())).rev() {
        let class = character_class(characters[index]);
        let boundary = match class {
            CharacterClass::Regular => kind == Some(WordKind::Separator),
            CharacterClass::WordSeparator => kind == Some(WordKind::Regular),
            CharacterClass::Whitespace => kind.is_some(),
        };
        if boundary && let Some(kind) = kind {
            return Some(Word {
                start: index + 1,
                end: word_end(characters, kind, index + 1),
                kind,
                next_class: class,
            });
        }
        kind = match class {
            CharacterClass::Regular => Some(WordKind::Regular),
            CharacterClass::WordSeparator => Some(WordKind::Separator),
            CharacterClass::Whitespace => kind,
        };
    }
    kind.map(|kind| Word {
        start: 0,
        end: word_end(characters, kind, 0),
        kind,
        next_class: CharacterClass::Whitespace,
    })
}

fn next_word(characters: &[char], cursor: usize) -> Option<Word> {
    let mut kind = None;
    for index in cursor..characters.len() {
        let class = character_class(characters[index]);
        let boundary = match class {
            CharacterClass::Regular => kind == Some(WordKind::Separator),
            CharacterClass::WordSeparator => kind == Some(WordKind::Regular),
            CharacterClass::Whitespace => kind.is_some(),
        };
        if boundary && let Some(kind) = kind {
            return Some(Word {
                start: word_start(characters, kind, index - 1),
                end: index,
                kind,
                next_class: class,
            });
        }
        kind = match class {
            CharacterClass::Regular => Some(WordKind::Regular),
            CharacterClass::WordSeparator => Some(WordKind::Separator),
            CharacterClass::Whitespace => kind,
        };
    }
    kind.map(|kind| Word {
        start: word_start(characters, kind, characters.len() - 1),
        end: characters.len(),
        kind,
        next_class: CharacterClass::Whitespace,
    })
}

fn is_lone_separator_before_word(word: &Word) -> bool {
    word.kind == WordKind::Separator
        && word.end - word.start == 1
        && word.next_class == CharacterClass::Regular
}

pub fn mouse_word_range(document: &DocumentSnapshot, byte: usize, extending: bool) -> Range<usize> {
    let line = line_characters(document, document.rope.byte_to_line(byte));
    let cursor = line.index_of(byte);
    let previous = previous_word(&line.characters, cursor);
    let next = next_word(&line.characters, cursor);
    let candidate = if extending {
        [previous, next]
            .into_iter()
            .flatten()
            .find(|word| word.kind == WordKind::Regular && word.start < cursor && cursor < word.end)
    } else {
        [previous, next].into_iter().flatten().find(|word| {
            word.start <= cursor
                && (cursor < word.end || word.kind == WordKind::Regular && cursor == word.end)
        })
    };
    if let Some(word) = candidate {
        return line.byte_of(word.start)..line.byte_of(word.end);
    }
    if extending {
        return byte..byte;
    }
    line.byte_of(previous.map_or(0, |word| word.end))
        ..line.byte_of(next.map_or(line.characters.len(), |word| word.start))
}

fn word_left_target(document: &DocumentSnapshot, head: usize, has_multiple_cursors: bool) -> usize {
    let mut line = document.rope.byte_to_line(head);
    let mut content = line_characters(document, line);
    let mut column = content.index_of(head);
    if column == 0 && line > 0 {
        line -= 1;
        content = line_characters(document, line);
        column = content.characters.len();
    }
    let mut word = previous_word(&content.characters, column);
    if !has_multiple_cursors
        && let Some(found) = word
        && is_lone_separator_before_word(&found)
    {
        word = previous_word(&content.characters, found.start);
    }
    content.byte_of(word.map_or(0, |word| word.start))
}

fn word_right_target(document: &DocumentSnapshot, head: usize) -> usize {
    let mut line = document.rope.byte_to_line(head);
    let mut content = line_characters(document, line);
    let mut column = content.index_of(head);
    if column == content.characters.len() && line + 1 < document.rope.len_lines() {
        line += 1;
        content = line_characters(document, line);
        column = 0;
    }
    let mut word = next_word(&content.characters, column);
    if let Some(found) = word
        && is_lone_separator_before_word(&found)
    {
        word = next_word(&content.characters, found.end);
    }
    content.byte_of(word.map_or(content.characters.len(), |word| word.end))
}

fn word_delete_left_range(document: &DocumentSnapshot, head: usize) -> Range<usize> {
    let line = document.rope.byte_to_line(head);
    let content = line_characters(document, line);
    let column = content.index_of(head);
    if line == 0 && column == 0 {
        return head..head;
    }
    let whitespace_start = content.characters[..column]
        .iter()
        .rposition(|character| !matches!(character, ' ' | '\t'))
        .map_or(0, |index| index + 1);
    if column > whitespace_start + 1 {
        return content.byte_of(whitespace_start)..head;
    }
    match previous_word(&content.characters, column) {
        Some(word) => content.byte_of(word.start)..head,
        None if column > 0 => content.start..head,
        None => line_content_range(document, line - 1).end..head,
    }
}

fn word_delete_right_range(document: &DocumentSnapshot, head: usize) -> Range<usize> {
    let line = document.rope.byte_to_line(head);
    let last_line = document.rope.len_lines() - 1;
    let content = line_characters(document, line);
    let column = content.index_of(head);
    let length = content.characters.len();
    if line == last_line && column == length {
        return head..head;
    }
    let first_non_whitespace = content.characters[column..]
        .iter()
        .position(|character| !matches!(character, ' ' | '\t'))
        .map_or(length, |index| column + index);
    if column + 1 < first_non_whitespace {
        return head..content.byte_of(first_non_whitespace);
    }
    match next_word(&content.characters, column) {
        Some(word) => head..content.byte_of(word.end),
        None if column < length || line == last_line => head..content.byte_of(length),
        None => {
            let next = line_characters(document, line + 1);
            let target =
                next_word(&next.characters, 0).map_or(next.characters.len(), |word| word.start);
            head..next.byte_of(target)
        }
    }
}

pub(crate) fn word_delete_inside_range(document: &DocumentSnapshot, head: usize) -> Range<usize> {
    let line = document.rope.byte_to_line(head);
    let content = line_characters(document, line);
    let length = content.characters.len();
    if length == 0 {
        if line > 0 {
            return line_content_range(document, line - 1).end..head;
        }
        if line + 1 < document.rope.len_lines() {
            return head..document.rope.line_to_byte(line + 1);
        }
        return head..head;
    }
    let column = content.index_of(head);
    let is_blank = |index: usize| matches!(content.characters[index], ' ' | '\t');
    let left = column.saturating_sub(1);
    let right = column.min(length - 1);
    if is_blank(left) && is_blank(right) {
        let start = (0..left)
            .rev()
            .find(|index| !is_blank(*index))
            .map_or(0, |index| index + 1);
        let end = (right + 1..length)
            .find(|index| !is_blank(*index))
            .unwrap_or(length);
        return content.byte_of(start)..content.byte_of(end);
    }
    let around_caret = |start: usize, end: usize| {
        content.byte_of(start.min(column))..content.byte_of(end.max(column))
    };
    let touches = |word: &Word| word.start <= column && column <= word.end;
    let with_adjacent_blanks = |word: &Word| {
        let end = (word.end..length)
            .find(|index| !is_blank(*index))
            .unwrap_or(length);
        let start = if end > word.end {
            word.start
        } else {
            (0..word.start)
                .rev()
                .find(|index| !is_blank(*index))
                .map_or(0, |index| index + 1)
        };
        around_caret(start, end)
    };
    let previous = previous_word(&content.characters, column);
    if let Some(word) = previous.as_ref().filter(|word| touches(word)) {
        return with_adjacent_blanks(word);
    }
    let next = next_word(&content.characters, column);
    if let Some(word) = next.as_ref().filter(|word| touches(word)) {
        return with_adjacent_blanks(word);
    }
    match (previous, next) {
        (Some(previous), Some(next)) => around_caret(previous.end, next.start),
        (Some(word), None) | (None, Some(word)) => around_caret(word.start, word.end),
        (None, None) => around_caret(0, length),
    }
}

fn line_start_target(document: &DocumentSnapshot, head: usize) -> usize {
    let content = line_text(document, document.rope.byte_to_line(head));
    let indentation = leading_whitespace(&content.text).len();
    let first_non_blank = if indentation == content.text.len() {
        0
    } else {
        indentation
    };
    if head - content.start == first_non_blank {
        content.start
    } else {
        content.start + first_non_blank
    }
}

fn shown_offset(
    document: &DocumentSnapshot,
    display: &DisplayMap,
    offset: usize,
    forward: bool,
) -> usize {
    let Some(hidden) = display.hidden_lines_at(document.rope.byte_to_line(offset)) else {
        return offset;
    };
    if forward && hidden.end < document.rope.len_lines() {
        document.rope.line_to_byte(hidden.end)
    } else {
        line_content_range(document, hidden.start - 1).end
    }
}

fn row_start_target(
    document: &DocumentSnapshot,
    display: &DisplayMap,
    head: usize,
    head_row: usize,
) -> usize {
    let row = display.segment(document, head_row);
    if !row.is_continuation {
        return line_start_target(document, head);
    }
    let first_non_blank = document
        .rope
        .byte_slice(row.bytes.clone())
        .bytes()
        .position(|byte| !matches!(byte, b' ' | b'\t'));
    if first_non_blank == Some(head.saturating_sub(row.bytes.start)) {
        line_start_target(document, head)
    } else {
        row.bytes.start + first_non_blank.unwrap_or(0)
    }
}

fn row_end_target(
    document: &DocumentSnapshot,
    display: &DisplayMap,
    head: usize,
    head_row: usize,
) -> (usize, bool) {
    let row = display.segment(document, head_row);
    let line_end = line_content_range(document, row.line).end;
    if head == row.bytes.end || row.bytes.end == line_end {
        (line_end, false)
    } else {
        (row.bytes.end, true)
    }
}

fn row_content(document: &DocumentSnapshot, row: &RowSegment) -> String {
    let mut content = " ".repeat(row.indent_columns as usize);
    content.extend(document.rope.byte_slice(row.bytes.clone()).chunks());
    content
}

fn vertical_target(
    document: &DocumentSnapshot,
    display: &DisplayMap,
    (from, from_row): (usize, usize),
    rows: isize,
    leftover: isize,
    tab_size: usize,
) -> (usize, isize, bool) {
    let last_row = display.row_count() - 1;
    let row = display.segment(document, from_row);
    let content = row_content(document, &row);
    let column =
        row.indent_columns as usize + from.saturating_sub(row.bytes.start).min(row.bytes.len());
    let current = visible_column(&content[..column], tab_size) as isize + leftover;
    let requested = from_row as isize + rows;
    let was_at_edge = if rows < 0 {
        from_row == 0 && column == 0
    } else {
        from_row == last_row && column == content.len()
    };
    let target_row = requested.clamp(0, last_row as isize) as usize;
    let target = display.segment(document, target_row);
    let target_content = row_content(document, &target);
    let target_indent = target.indent_columns as usize;
    let target_column = if requested < 0 {
        target_indent
    } else if requested > last_row as isize {
        target_content.len()
    } else {
        offset_at_visible_column(&target_content, current.max(0) as usize, tab_size)
            .max(target_indent)
    };
    let remaining = if was_at_edge {
        0
    } else {
        current - visible_column(&target_content[..target_column], tab_size) as isize
    };
    let byte = target.bytes.start + target_column - target_indent;
    (
        byte,
        remaining,
        display.row_of_byte(document, byte) != target_row,
    )
}

fn is_typing(operation: EditOperation) -> bool {
    matches!(
        operation,
        EditOperation::TypingOther
            | EditOperation::TypingFirstSpace
            | EditOperation::TypingConsecutiveSpace
    )
}

fn undo_class(operation: EditOperation) -> EditOperation {
    if operation == EditOperation::TypingConsecutiveSpace {
        EditOperation::TypingFirstSpace
    } else {
        operation
    }
}

fn pushes_undo_stop_between(previous: EditOperation, next: EditOperation) -> bool {
    if is_typing(previous) && !is_typing(next) {
        return true;
    }
    if previous == EditOperation::TypingFirstSpace {
        return false;
    }
    undo_class(previous) != undo_class(next)
}

pub(crate) fn previous_operation(
    current: &ViewState,
    document: &DocumentSnapshot,
) -> EditOperation {
    current
        .edit_run
        .as_ref()
        .filter(|run| run.revision == document.revision)
        .map_or(EditOperation::Other, |run| run.operation)
}

pub(crate) fn typing_step(previous: EditOperation, operation: EditOperation) -> Step {
    Step {
        operation,
        stop_before: pushes_undo_stop_between(previous, operation),
        stop_after: false,
    }
}

pub(crate) fn apply_step(
    store: &mut EditorStore,
    view: ViewId,
    transaction: Option<Transaction>,
    step: Step,
) -> Result<bool, EditorError> {
    let (current, document) = view_document(store, view)?;
    let continued = current
        .edit_run
        .as_ref()
        .filter(|run| {
            !step.stop_before
                && run.revision == document.revision
                && run.selection == current.selection
        })
        .map(|run| run.group);
    let group = continued.unwrap_or(UndoGroup(document.revision));
    if continued.is_none() {
        store.break_undo_group(document.id)?;
    }
    let revision = match transaction {
        Some(transaction) => store.apply(
            document.id,
            Transaction {
                group,
                ..transaction
            },
        )?,
        None => document.revision,
    };
    if step.stop_after {
        store.break_undo_group(document.id)?;
    }
    let selection = store
        .views()
        .get(view)
        .ok_or(EditorError::NotFound)?
        .selection
        .clone();
    store.set_edit_run(
        view,
        Some(EditRun {
            operation: step.operation,
            group,
            revision,
            selection,
        }),
    )?;
    Ok(revision != document.revision)
}

pub fn grapheme_boundary(
    document: &DocumentSnapshot,
    offset: usize,
    forward: bool,
) -> Result<usize, EditorError> {
    let rope = &document.rope;
    byte_to_char(rope, offset)?;
    let mut cursor = GraphemeCursor::new(offset, rope.len_bytes(), true);
    let probe = if forward {
        offset
    } else {
        offset.saturating_sub(1)
    };
    let (mut chunk, mut start, _, _) = rope.chunk_at_byte(probe);
    loop {
        let result = if forward {
            cursor.next_boundary(chunk, start)
        } else {
            cursor.prev_boundary(chunk, start)
        };
        match result {
            Ok(boundary) => return Ok(boundary.unwrap_or(offset)),
            Err(GraphemeIncomplete::NextChunk) => {
                (chunk, start, _, _) = rope.chunk_at_byte(start + chunk.len());
            }
            Err(GraphemeIncomplete::PrevChunk) => {
                (chunk, start, _, _) = rope.chunk_at_byte(start.saturating_sub(1));
            }
            Err(GraphemeIncomplete::PreContext(end)) => {
                let (context, context_start, _, _) = rope.chunk_at_byte(end.saturating_sub(1));
                cursor.provide_context(&context[..end - context_start], context_start);
            }
            Err(GraphemeIncomplete::InvalidOffset) => return Err(EditorError::InvalidBoundary),
        }
    }
}

pub fn line_content_range(document: &DocumentSnapshot, line: usize) -> Range<usize> {
    rope_line_content_range(&document.rope, line)
}

pub(crate) fn rope_line_content_range(rope: &Rope, line: usize) -> Range<usize> {
    let line = line.min(rope.len_lines() - 1);
    let start = rope.line_to_byte(line);
    let start_char = rope.line_to_char(line);
    let mut end_char = start_char + rope.line(line).len_chars();
    if end_char > start_char && rope.char(end_char - 1) == '\n' {
        end_char -= 1;
    }
    if end_char > start_char && rope.char(end_char - 1) == '\r' {
        end_char -= 1;
    }
    start..rope.char_to_byte(end_char)
}

pub fn reveal_position(
    store: &mut EditorStore,
    view: ViewId,
    line: f64,
    column: f64,
) -> Result<usize, EditorError> {
    let current = store
        .views()
        .get(view)
        .ok_or(EditorError::NotFound)?
        .clone();
    let document = store.documents().snapshot(current.document)?;
    let requested_line = if line.is_nan() { 1.0 } else { line.floor() };
    let target_line = ((requested_line - 1.0).max(0.0) as usize).min(document.rope.len_lines() - 1);
    let range = line_content_range(&document, target_line);
    let byte = if requested_line < 1.0 {
        0
    } else if requested_line > document.rope.len_lines() as f64 {
        range.end
    } else {
        let requested_column = if column.is_nan() { 1.0 } else { column.floor() };
        let slice = document.rope.byte_slice(range.clone());
        let units = ((requested_column - 1.0).max(0.0) as usize).min(slice.len_utf16_cu());
        range.start + slice.char_to_byte(slice.utf16_cu_to_char(units))
    };
    store.set_composition(view, None)?;
    store.set_view_state(
        view,
        SelectionSet {
            primary: 0,
            selections: vec![Selection {
                anchor: byte,
                head: byte,
            }],
        },
        current.scroll,
        current.folds,
    )?;
    Ok(byte)
}

pub fn move_selection(
    store: &mut EditorStore,
    view: ViewId,
    motion: Motion,
    extend: bool,
) -> Result<(), EditorError> {
    move_selection_across(store, view, motion, extend, None)
}

pub fn move_selection_displayed(
    store: &mut EditorStore,
    view: ViewId,
    motion: Motion,
    extend: bool,
    display: &DisplayMap,
) -> Result<(), EditorError> {
    move_selection_across(store, view, motion, extend, Some(display))
}

fn move_selection_across(
    store: &mut EditorStore,
    view: ViewId,
    motion: Motion,
    extend: bool,
    display: Option<&DisplayMap>,
) -> Result<(), EditorError> {
    let (current, document) = view_document(store, view)?;
    let identity = DisplayMap::identity(document.rope.len_lines(), document.revision);
    let display = display
        .filter(|display| display.revision() == document.revision)
        .unwrap_or(&identity);
    let has_multiple_cursors = current.selection.selections.len() > 1;
    let goal = current.goal_columns.as_ref().filter(|goal| {
        goal.revision == document.revision
            && goal.leftover_visible_columns.len() == current.selection.selections.len()
    });
    let mut leftover_visible_columns = Vec::new();
    let mut heads_at_row_end = Vec::new();
    let selections = current
        .selection
        .selections
        .iter()
        .enumerate()
        .map(|(index, selection)| {
            let head = shown_offset(&document, display, selection.head, false);
            let range = ordered(selection);
            let collapses = !extend && !range.is_empty();
            let head_at_row_end = current.head_at_row_end(index, document.revision);
            let head_row = display.row_of_head(&document, head, head_at_row_end);
            let (target, at_row_end) = match motion {
                Motion::Left if collapses => {
                    let start = shown_offset(&document, display, range.start, false);
                    (start, start == head && head_at_row_end)
                }
                Motion::Right if collapses => {
                    let end = shown_offset(&document, display, range.end, false);
                    (end, end == head && head_at_row_end)
                }
                Motion::Left => (
                    shown_offset(
                        &document,
                        display,
                        grapheme_boundary(&document, head, false)?,
                        false,
                    ),
                    false,
                ),
                Motion::Right => {
                    let target = shown_offset(
                        &document,
                        display,
                        grapheme_boundary(&document, head, true)?,
                        true,
                    );
                    let row = display.row_of_byte(&document, head);
                    (
                        target,
                        target == display.segment(&document, row).bytes.end
                            && display.row_of_byte(&document, target) != row,
                    )
                }
                Motion::WordLeft => (
                    word_left_target(&document, head, has_multiple_cursors),
                    false,
                ),
                Motion::WordRight => (word_right_target(&document, head), false),
                Motion::LineStart => (row_start_target(&document, display, head, head_row), false),
                Motion::LineEnd => row_end_target(&document, display, head, head_row),
                Motion::DocumentStart => (0, false),
                Motion::DocumentEnd => (document.rope.len_bytes(), false),
                Motion::Vertical { lines, tab_size } => {
                    let from = match (collapses, lines < 0) {
                        (true, true) => range.start,
                        (true, false) => range.end,
                        (false, _) => head,
                    };
                    let from_row = if from == head {
                        head_row
                    } else {
                        display.row_of_byte(&document, from)
                    };
                    let (target, remaining, at_row_end) = vertical_target(
                        &document,
                        display,
                        (from, from_row),
                        lines,
                        goal.map_or(0, |goal| goal.leftover_visible_columns[index]),
                        (tab_size as usize).max(1),
                    );
                    leftover_visible_columns.push(remaining);
                    (target, at_row_end)
                }
            };
            heads_at_row_end.push(at_row_end);
            Ok(Selection {
                anchor: if extend { selection.anchor } else { target },
                head: target,
            })
        })
        .collect::<Result<Vec<_>, EditorError>>()?;
    store.break_undo_group(document.id)?;
    store.set_composition(view, None)?;
    store.set_view_state(
        view,
        SelectionSet {
            primary: current.selection.primary,
            selections,
        },
        current.scroll,
        current.folds,
    )?;
    store.set_goal_columns(
        view,
        matches!(motion, Motion::Vertical { .. }).then_some(GoalColumns {
            revision: document.revision,
            leftover_visible_columns,
        }),
    )?;
    store.set_wrap_affinities(
        view,
        heads_at_row_end.contains(&true).then_some(WrapAffinities {
            revision: document.revision,
            heads_at_row_end,
        }),
    )
}

pub fn select_all(store: &mut EditorStore, view: ViewId) -> Result<(), EditorError> {
    let current = store
        .views()
        .get(view)
        .ok_or(EditorError::NotFound)?
        .clone();
    let document = store.documents().snapshot(current.document)?;
    store.break_undo_group(document.id)?;
    store.set_composition(view, None)?;
    store.set_view_state(
        view,
        SelectionSet {
            primary: 0,
            selections: vec![Selection {
                anchor: 0,
                head: document.rope.len_bytes(),
            }],
        },
        current.scroll,
        current.folds,
    )
}

pub fn replace_selections(
    store: &mut EditorStore,
    view: ViewId,
    text: &str,
    delete_forward: Option<bool>,
) -> Result<bool, EditorError> {
    let Some(transaction) = replacement_transaction(store, view, text, delete_forward)? else {
        return Ok(false);
    };
    let document = store
        .views()
        .get(view)
        .ok_or(EditorError::NotFound)?
        .document;
    store.apply(document, transaction)?;
    Ok(true)
}

pub fn replacement_transaction(
    store: &EditorStore,
    view: ViewId,
    text: &str,
    delete_forward: Option<bool>,
) -> Result<Option<Transaction>, EditorError> {
    let (current, document) = view_document(store, view)?;
    let ranges = current
        .selection
        .selections
        .iter()
        .map(|selection| {
            let mut range = ordered(selection);
            if range.is_empty()
                && let Some(forward) = delete_forward
            {
                let boundary = grapheme_boundary(&document, selection.head, forward)?;
                range = boundary.min(selection.head)..boundary.max(selection.head);
            }
            Ok(range)
        })
        .collect::<Result<Vec<_>, EditorError>>()?;
    Ok(ranges_transaction(
        &document,
        view,
        current.selection.primary,
        ranges,
        text,
    ))
}

pub(crate) fn ranges_transaction(
    document: &DocumentSnapshot,
    view: ViewId,
    primary: usize,
    mut ranges: Vec<Range<usize>>,
    text: &str,
) -> Option<Transaction> {
    let primary_range = ranges[primary].clone();
    ranges.sort_by_key(|range| (range.start, range.end));
    let mut merged = Vec::<Range<usize>>::new();
    for range in ranges {
        if let Some(last) = merged.last_mut()
            && (range.start < last.end || range == *last)
        {
            last.end = last.end.max(range.end);
            continue;
        }
        merged.push(range);
    }
    if text.is_empty() && merged.iter().all(Range::is_empty) {
        return None;
    }
    let primary = merged
        .iter()
        .position(|range| range == &primary_range)
        .or_else(|| {
            merged.iter().position(|range| {
                range.start <= primary_range.start && range.end >= primary_range.end
            })
        })
        .unwrap_or(0);
    let mut removed = 0;
    let mut inserted = 0;
    let selections = merged
        .iter()
        .map(|range| {
            let head = range.start - removed + inserted + text.len();
            removed += range.end - range.start;
            inserted += text.len();
            Selection { anchor: head, head }
        })
        .collect();
    Some(Transaction {
        revision: document.revision,
        group: UndoGroup(document.revision),
        origin: Some(view),
        selection_after: Some(SelectionSet {
            primary,
            selections,
        }),
        edits: merged
            .into_iter()
            .map(|bytes| Edit {
                bytes,
                text: text.into(),
            })
            .collect(),
    })
}

pub fn type_text(store: &mut EditorStore, view: ViewId, text: &str) -> Result<bool, EditorError> {
    if text.is_empty() {
        return Ok(false);
    }
    let (current, document) = view_document(store, view)?;
    let previous = previous_operation(&current, &document);
    let operation = if text != " " {
        EditOperation::TypingOther
    } else if undo_class(previous) == EditOperation::TypingFirstSpace {
        EditOperation::TypingConsecutiveSpace
    } else {
        EditOperation::TypingFirstSpace
    };
    let transaction = replacement_transaction(store, view, text, None)?;
    apply_step(store, view, transaction, typing_step(previous, operation))
}

pub fn compose_text(
    store: &mut EditorStore,
    view: ViewId,
    bytes: Range<usize>,
    text: &str,
) -> Result<bool, EditorError> {
    if bytes.is_empty() && text.is_empty() {
        return Ok(false);
    }
    let (current, document) = view_document(store, view)?;
    if bytes.start > bytes.end {
        return Err(EditorError::InvalidBoundary);
    }
    let first = byte_to_char(&document.rope, bytes.start)?;
    let last = byte_to_char(&document.rope, bytes.end)?;
    let primary = ordered(&current.selection.selections[current.selection.primary]);
    let delta_start = first as isize - document.rope.byte_to_char(primary.start) as isize;
    let delta_end = last as isize - document.rope.byte_to_char(primary.end) as isize;
    let mut plan = Plan::new(&current.selection);
    for (index, selection) in current.selection.selections.iter().enumerate() {
        let selected = ordered(selection);
        let start = document
            .rope
            .byte_to_char(selected.start)
            .saturating_add_signed(delta_start)
            .min(document.rope.len_chars());
        let end = document
            .rope
            .byte_to_char(selected.end)
            .saturating_add_signed(delta_end)
            .min(document.rope.len_chars());
        plan.replace(
            index,
            document.rope.char_to_byte(start.min(end))..document.rope.char_to_byte(start.max(end)),
            text.into(),
        );
    }
    let previous = previous_operation(&current, &document);
    apply_step(
        store,
        view,
        plan.transaction(&document, view),
        typing_step(previous, EditOperation::TypingOther),
    )
}

fn delete_ranges(
    store: &mut EditorStore,
    view: ViewId,
    operation: EditOperation,
    range_of: impl Fn(&DocumentSnapshot, &Selection) -> Result<Range<usize>, EditorError>,
) -> Result<bool, EditorError> {
    let (current, document) = view_document(store, view)?;
    let ranges = current
        .selection
        .selections
        .iter()
        .map(|selection| {
            let range = ordered(selection);
            if range.is_empty() {
                range_of(&document, selection)
            } else {
                Ok(range)
            }
        })
        .collect::<Result<Vec<_>, EditorError>>()?;
    let step = if operation == EditOperation::Other {
        SEPARATE_STEP
    } else {
        Step {
            operation,
            stop_before: previous_operation(&current, &document) != operation
                || ranges.iter().any(|range| spans_lines(&document, range)),
            stop_after: false,
        }
    };
    let transaction = ranges_transaction(&document, view, current.selection.primary, ranges, "");
    apply_step(store, view, transaction, step)
}

pub fn delete_backward(
    store: &mut EditorStore,
    view: ViewId,
    indent: IndentOptions,
) -> Result<bool, EditorError> {
    delete_ranges(
        store,
        view,
        EditOperation::DeletingLeft,
        |document, selection| {
            let indent = document.model_indentation(indent);
            let size = tab_width(indent);
            let content = line_text(document, document.rope.byte_to_line(selection.head));
            let column = (selection.head - content.start).min(content.text.len());
            if column > 0 && column <= leading_whitespace(&content.text).len() {
                let stop = previous_tab_stop(
                    visible_column(&content.text[..column], size),
                    indent.indent_size as usize,
                );
                let target = offset_at_visible_column(&content.text, stop, size).min(column);
                return Ok(content.start + target..selection.head);
            }
            Ok(grapheme_boundary(document, selection.head, false)?..selection.head)
        },
    )
}

pub fn delete_forward(store: &mut EditorStore, view: ViewId) -> Result<bool, EditorError> {
    delete_ranges(
        store,
        view,
        EditOperation::DeletingRight,
        |document, selection| Ok(selection.head..grapheme_boundary(document, selection.head, true)?),
    )
}

pub fn delete_word(
    store: &mut EditorStore,
    view: ViewId,
    forward: bool,
) -> Result<bool, EditorError> {
    delete_ranges(store, view, EditOperation::Other, |document, selection| {
        Ok(if forward {
            word_delete_right_range(document, selection.head)
        } else {
            word_delete_left_range(document, selection.head)
        })
    })
}

pub fn delete_inside_word(store: &mut EditorStore, view: ViewId) -> Result<bool, EditorError> {
    delete_ranges(store, view, EditOperation::Other, |document, selection| {
        Ok(word_delete_inside_range(document, selection.head))
    })
}

pub fn delete_to_line_start(store: &mut EditorStore, view: ViewId) -> Result<bool, EditorError> {
    let (current, document) = view_document(store, view)?;
    let ranges = current
        .selection
        .selections
        .iter()
        .map(|selection| {
            let range = ordered(selection);
            let line = document.rope.byte_to_line(range.start);
            let line_start = document.rope.line_to_byte(line);
            if !range.is_empty() || range.start > line_start {
                return line_start..range.end;
            }
            if line == 0 {
                return range;
            }
            line_content_range(&document, line - 1).end..range.end
        })
        .collect();
    let transaction = ranges_transaction(&document, view, current.selection.primary, ranges, "");
    apply_step(store, view, transaction, SEPARATE_STEP)
}

pub fn insert_line_break(
    store: &mut EditorStore,
    view: ViewId,
    indent: IndentOptions,
) -> Result<bool, EditorError> {
    let (current, document) = view_document(store, view)?;
    let indent = normalization_options(document.model_indentation(indent));
    let line_break = document.metadata.line_ending.as_str();
    let mut plan = Plan::new(&current.selection);
    for (index, selection) in current.selection.selections.iter().enumerate() {
        let range = ordered(selection);
        let content = line_text(&document, document.rope.byte_to_line(range.start));
        let leading = leading_whitespace(&content.text);
        let kept = &leading[..leading.len().min(range.start - content.start)];
        let columns = visible_column(kept, tab_width(indent));
        plan.replace(
            index,
            range,
            format!("{line_break}{}", indentation(columns, indent)),
        );
    }
    let transaction = plan.transaction(&document, view);
    apply_step(
        store,
        view,
        transaction,
        Step {
            operation: EditOperation::TypingOther,
            stop_before: true,
            stop_after: false,
        },
    )
}

pub(crate) fn shift_lines(
    plan: &mut Plan,
    document: &DocumentSnapshot,
    index: usize,
    selection: &Selection,
    indent: IndentOptions,
    outdents: bool,
) {
    shift_language_lines(plan, document, index, selection, indent, outdents, None);
}

pub(crate) fn shift_language_lines(
    plan: &mut Plan,
    document: &DocumentSnapshot,
    index: usize,
    selection: &Selection,
    indent: IndentOptions,
    outdents: bool,
    language: Option<crate::language_configuration::Language<'_>>,
) {
    let indent = document.model_indentation(indent);
    let size = tab_width(indent);
    let step = indent_step(indent);
    let range = ordered(selection);
    let start_line = document.rope.byte_to_line(range.start);
    let mut end_line = document.rope.byte_to_line(range.end);
    if end_line > start_line && range.end == document.rope.line_to_byte(end_line) {
        end_line -= 1;
    }
    let start = line_text(document, start_line);
    let moves_caret_after_edit = range.is_empty() && start.text.chars().all(char::is_whitespace);
    let ends_at_line_end = range.end == start.start + start.text.len();
    let (start_sticks, end_sticks) = if range.is_empty() {
        (ends_at_line_end, ends_at_line_end)
    } else {
        (false, true)
    };
    let mut start_mark = Mark::Tracked {
        offset: range.start,
        sticks: start_sticks,
    };
    let mut caret_edit = None;
    let mut previous_extra_spaces = 0;
    for line in start_line..=end_line {
        let content = line_text(document, line);
        let mut existing = leading_whitespace(&content.text);
        let mut extra_spaces = 0;
        if line > 0
            && visible_column(existing, size) % indent.indent_size as usize != 0
            && let Some(language) = language
            && language.syntax.tokens(document, line - 1).is_some()
        {
            extra_spaces = crate::auto_indent::extra_indent_spaces(
                document,
                language,
                line - 1,
                previous_extra_spaces,
                indent.indent_size as usize,
            )
            .unwrap_or(0);
            let trailing_spaces = existing
                .chars()
                .rev()
                .take_while(|character| *character == ' ')
                .count()
                .min(extra_spaces);
            existing = &existing[..existing.len() - trailing_spaces];
        }
        previous_extra_spaces = extra_spaces;
        if outdents && existing.is_empty() {
            continue;
        }
        if !outdents && end_line > start_line && content.text.is_empty() {
            continue;
        }
        let columns = visible_column(existing, size);
        let stop = if outdents {
            previous_tab_stop(columns, step)
        } else {
            next_tab_stop(columns, step)
        };
        let desired = indentation(stop, indent);
        if desired == existing {
            continue;
        }
        let edit = plan.edit(content.start..content.start + existing.len(), desired);
        if line == start_line && edit.is_some() {
            caret_edit = edit;
            if !range.is_empty() && range.start - content.start <= existing.len() {
                start_mark = Mark::Anchored {
                    offset: range.start,
                    line_start: content.start,
                };
            }
        }
    }
    if moves_caret_after_edit {
        if let Some(edit) = caret_edit {
            plan.marks[index] = (Mark::AfterEdit(edit), Mark::AfterEdit(edit));
        }
        return;
    }
    let end_mark = Mark::Tracked {
        offset: range.end,
        sticks: end_sticks,
    };
    plan.marks[index] = if selection.anchor <= selection.head {
        (start_mark, end_mark)
    } else {
        (end_mark, start_mark)
    };
}

fn jump_to_next_indent(prefix: &str, indent: ModelIndentOptions) -> String {
    if !indent.insert_spaces {
        return "\t".into();
    }
    let step = indent_step(indent);
    " ".repeat(step - visible_column(prefix, tab_width(indent)) % step)
}

pub fn tab(
    store: &mut EditorStore,
    view: ViewId,
    indent: IndentOptions,
) -> Result<bool, EditorError> {
    tab_with_language(store, view, indent, None)
}

pub fn tab_with_language(
    store: &mut EditorStore,
    view: ViewId,
    indent: IndentOptions,
    language: Option<crate::language_configuration::Language<'_>>,
) -> Result<bool, EditorError> {
    let (current, document) = view_document(store, view)?;
    let options = document.model_indentation(indent);
    let mut plan = Plan::new(&current.selection);
    for (index, selection) in current.selection.selections.iter().enumerate() {
        let range = ordered(selection);
        let start_line = document.rope.byte_to_line(range.start);
        let content = line_text(&document, start_line);
        let line_end = content.start + content.text.len();
        let column = (range.start - content.start).min(content.text.len());
        if range.is_empty() {
            let unit = indentation(options.indent_size as usize, normalization_options(options));
            if content.text.chars().all(char::is_whitespace) && !content.text.starts_with(&unit) {
                plan.replace(index, content.start..line_end, unit);
                continue;
            }
        }
        let covers_line = range.start == content.start && range.end == line_end;
        if range.is_empty() || (range.end <= line_end && !covers_line) {
            plan.replace(
                index,
                range,
                jump_to_next_indent(&content.text[..column], options),
            );
            continue;
        }
        shift_language_lines(
            &mut plan, &document, index, selection, indent, false, language,
        );
    }
    let transaction = plan.transaction(&document, view);
    apply_step(store, view, transaction, SEPARATE_STEP)
}

fn shift_selected_lines(
    store: &mut EditorStore,
    view: ViewId,
    indent: IndentOptions,
    outdents: bool,
) -> Result<bool, EditorError> {
    let (current, document) = view_document(store, view)?;
    let mut plan = Plan::new(&current.selection);
    for (index, selection) in current.selection.selections.iter().enumerate() {
        shift_lines(&mut plan, &document, index, selection, indent, outdents);
    }
    let transaction = plan.transaction(&document, view);
    apply_step(store, view, transaction, SEPARATE_STEP)
}

pub fn outdent(
    store: &mut EditorStore,
    view: ViewId,
    indent: IndentOptions,
) -> Result<bool, EditorError> {
    shift_selected_lines(store, view, indent, true)
}

pub fn indent_lines(
    store: &mut EditorStore,
    view: ViewId,
    indent: IndentOptions,
) -> Result<bool, EditorError> {
    shift_selected_lines(store, view, indent, false)
}

pub fn clipboard_text(
    store: &EditorStore,
    view: ViewId,
    force_crlf: bool,
) -> Result<Option<ClipboardText>, EditorError> {
    let current = store.views().get(view).ok_or(EditorError::NotFound)?;
    let document = store.documents().snapshot(current.document)?;
    let line_break = if force_crlf {
        LineEnding::CrLf
    } else {
        document.metadata.line_ending
    }
    .as_str();
    let slice = |range: Range<usize>| {
        let text = document.rope.byte_slice(range).to_string();
        if force_crlf {
            normalize_line_breaks(&text, line_break)
        } else {
            text
        }
    };
    let mut ranges: Vec<Range<usize>> = current.selection.selections.iter().map(ordered).collect();
    ranges.sort_by_key(|range| (range.start, range.end));
    let mut pieces = Vec::new();
    let mut previous_line = None;
    for range in &ranges {
        let line = document.rope.byte_to_line(range.start);
        if !range.is_empty() {
            pieces.push(slice(range.clone()));
        } else if previous_line != Some(line) {
            pieces.push(format!(
                "{}{line_break}",
                slice(line_content_range(&document, line))
            ));
        }
        previous_line = Some(line);
    }
    let text = pieces.join(line_break);
    if text.is_empty() {
        return Ok(None);
    }
    Ok(Some(ClipboardText {
        text,
        from_empty_selection: ranges.len() == 1 && ranges[0].is_empty(),
        multicursor: (pieces.len() > 1).then_some(pieces),
    }))
}

pub fn cut(store: &mut EditorStore, view: ViewId) -> Result<bool, EditorError> {
    let (current, document) = view_document(store, view)?;
    let rope = &document.rope;
    let last_line = rope.len_lines() - 1;
    let mut order: Vec<usize> = (0..current.selection.selections.len()).collect();
    order.sort_by_key(|index| {
        let range = ordered(&current.selection.selections[*index]);
        (range.start, range.end)
    });
    let mut plan = Plan::new(&current.selection);
    let mut last_cut_end_line = None;
    for index in order {
        let selection = &current.selection.selections[index];
        let mut range = ordered(selection);
        if range.is_empty() {
            let line = rope.byte_to_line(selection.head);
            let content = line_content_range(&document, line);
            range = if line < last_line {
                content.start..rope.line_to_byte(line + 1)
            } else if line > 0 && last_cut_end_line != Some(line) {
                line_content_range(&document, line - 1).end..content.end
            } else {
                content
            };
            last_cut_end_line = Some(rope.byte_to_line(range.end));
        }
        plan.replace(index, range, String::new());
    }
    let transaction = plan.transaction(&document, view);
    apply_step(store, view, transaction, SEPARATE_STEP)
}

pub fn paste(
    store: &mut EditorStore,
    view: ViewId,
    text: &str,
    source: Option<&ClipboardText>,
) -> Result<bool, EditorError> {
    let (current, document) = view_document(store, view)?;
    let line_break = document.metadata.line_ending.as_str();
    let text = normalize_line_breaks(text, line_break);
    let selections = &current.selection.selections;
    let mut pastes_on_new_line = source.is_some_and(|source| source.from_empty_selection);
    let distributed = if selections.len() == 1 {
        None
    } else if let Some(pieces) = source
        .and_then(|source| source.multicursor.as_ref())
        .filter(|pieces| pieces.len() == selections.len())
    {
        Some(
            pieces
                .iter()
                .map(|piece| normalize_line_breaks(piece, line_break))
                .collect::<Vec<_>>(),
        )
    } else if pastes_on_new_line {
        None
    } else {
        let lines: Vec<String> = text
            .strip_suffix(line_break)
            .unwrap_or(&text)
            .split(line_break)
            .map(str::to_owned)
            .collect();
        (lines.len() == selections.len()).then_some(lines)
    };
    let mut plan = Plan::new(&current.selection);
    if let Some(pieces) = distributed {
        let mut order: Vec<usize> = (0..selections.len()).collect();
        order.sort_by_key(|index| {
            let range = ordered(&selections[*index]);
            (range.start, range.end)
        });
        for (index, piece) in order.into_iter().zip(pieces) {
            plan.replace(index, ordered(&selections[index]), piece);
        }
    } else {
        for (index, selection) in selections.iter().enumerate() {
            let range = ordered(selection);
            pastes_on_new_line = pastes_on_new_line
                && range.is_empty()
                && text.ends_with('\n')
                && text.find('\n') == Some(text.len() - 1);
            if !pastes_on_new_line {
                plan.replace(index, range, text.clone());
                continue;
            }
            let line_start = document
                .rope
                .line_to_byte(document.rope.byte_to_line(selection.head));
            plan.edit(line_start..line_start, text.clone());
        }
    }
    let transaction = plan.transaction(&document, view);
    apply_step(store, view, transaction, SEPARATE_STEP)
}

pub fn selected_text(store: &EditorStore, view: ViewId) -> Result<String, EditorError> {
    let current = store.views().get(view).ok_or(EditorError::NotFound)?;
    let document = store.documents().snapshot(current.document)?;
    let mut text = String::new();
    for (index, selection) in current.selection.selections.iter().enumerate() {
        if index > 0 {
            text.push('\n');
        }
        let start = document
            .rope
            .byte_to_char(selection.anchor.min(selection.head));
        let end = document
            .rope
            .byte_to_char(selection.anchor.max(selection.head));
        for chunk in document.rope.slice(start..end).chunks() {
            text.push_str(chunk);
        }
    }
    Ok(text)
}

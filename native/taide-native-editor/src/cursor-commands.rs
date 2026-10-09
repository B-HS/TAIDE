use crate::bracket_navigation::{
    bracket_pairs, contains, enclosing_brackets, matching_brackets, next_bracket,
};
use crate::change_journal::ChangesSince;
use crate::decoration::{Decoration, DecorationKind, DecorationLayer, InlineStyle, Stickiness};
use crate::document::{DocumentSnapshot, EditorError};
use crate::editing::{
    Mark, Plan, SEPARATE_STEP, apply_step, line_content_range, line_text, offset_at_visible_column,
    visible_column,
};
use crate::language_configuration::{LanguageRules, is_js_whitespace};
use crate::line_commands::LineCommandContext;
use crate::store::EditorStore;
use crate::view::{ScrollPosition, Selection, SelectionSet, ViewId, ViewState};
use std::ops::Range;

const CURSOR_HISTORY_LIMIT: usize = 50;
const MULTICURSOR_LIMIT: usize = 10000;

#[derive(Debug, Clone, Default)]
pub(crate) struct CursorMemory {
    revision: u64,
    undo: Vec<(SelectionSet, ScrollPosition)>,
    redo: Vec<(SelectionSet, ScrollPosition)>,
    pub(crate) anchor: Option<DecorationLayer>,
    matches: Option<MatchSession>,
    smart: Option<SmartSelection>,
    vertical: Option<Vec<(Selection, usize, usize)>>,
}

#[derive(Debug, Clone)]
struct MatchSession {
    text: String,
    whole_word: bool,
    match_case: bool,
}

#[derive(Debug, Clone)]
struct SmartSelection {
    ranges: Vec<Vec<Range<usize>>>,
    indices: Vec<usize>,
}

impl CursorMemory {
    fn prepare(&mut self, revision: u64) {
        if self.revision != revision {
            self.undo.clear();
            self.redo.clear();
            self.revision = revision;
            self.matches = None;
            self.smart = None;
            self.vertical = None;
        }
    }

    pub(crate) fn selection_changed(
        &mut self,
        revision: u64,
        selection: &SelectionSet,
        scroll: &ScrollPosition,
    ) {
        self.prepare(revision);
        self.matches = None;
        self.smart = None;
        self.vertical = None;
        if self
            .undo
            .last()
            .is_none_or(|(previous, _)| previous != selection)
        {
            self.undo.push((selection.clone(), scroll.clone()));
            if self.undo.len() > CURSOR_HISTORY_LIMIT {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
    }

    pub(crate) fn content_changed(&mut self, revision: u64, changes: ChangesSince<'_>) {
        self.prepare(revision);
        self.anchor = self
            .anchor
            .as_ref()
            .and_then(|anchor| anchor.tracking(changes))
            .map(|anchor| anchor.into_owned());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorCommand {
    AddAbove,
    AddBelow,
    LineEnds,
    ToTop,
    ToBottom,
    FocusNext,
    FocusPrevious,
    AddNextMatch,
    AddPreviousMatch,
    MoveNextMatch,
    MovePreviousMatch,
    SelectMatches,
    ChangeAll,
    ExpandLine,
    Expand,
    Shrink,
    JumpToBracket,
    SelectToBracket,
    Undo,
    Redo,
    SetAnchor,
    MoveCaretLeft,
    MoveCaretRight,
}

pub fn run_cursor_command(
    store: &mut EditorStore,
    view: ViewId,
    command: CursorCommand,
    context: LineCommandContext<'_>,
) -> Result<bool, EditorError> {
    let current = store
        .views()
        .get(view)
        .ok_or(EditorError::NotFound)?
        .clone();
    let document = store.documents().snapshot(current.document)?;
    if document.metadata.read_only
        && matches!(
            command,
            CursorCommand::AddAbove
                | CursorCommand::AddBelow
                | CursorCommand::FocusNext
                | CursorCommand::FocusPrevious
                | CursorCommand::MoveCaretLeft
                | CursorCommand::MoveCaretRight
                | CursorCommand::ChangeAll
        )
    {
        return Ok(false);
    }
    if matches!(
        command,
        CursorCommand::MoveCaretLeft | CursorCommand::MoveCaretRight
    ) {
        return move_caret(
            store,
            view,
            &document,
            &current.selection,
            command == CursorCommand::MoveCaretLeft,
        );
    }
    let memory = store.cursor_memory_mut(view)?;
    memory.prepare(document.revision);
    if command == CursorCommand::SetAnchor {
        let byte = current.selection.selections[current.selection.primary].head;
        memory.anchor = Some(DecorationLayer::new(
            document.revision,
            0,
            vec![Decoration {
                bytes: byte..byte,
                kind: DecorationKind::Inline(InlineStyle::default()),
                stickiness: Stickiness::NeverGrowsWhenTypingAtEdges,
            }],
        ));
        return Ok(false);
    }
    if matches!(command, CursorCommand::Undo | CursorCommand::Redo) {
        store.take_selection_reveal(view)?;
        let memory = store.cursor_memory_mut(view)?;
        let mut memory = memory.clone();
        let next = if command == CursorCommand::Undo {
            memory.undo.pop().inspect(|_| {
                memory
                    .redo
                    .push((current.selection.clone(), current.scroll.clone()))
            })
        } else {
            memory.redo.pop().inspect(|_| {
                memory
                    .undo
                    .push((current.selection.clone(), current.scroll.clone()))
            })
        };
        if let Some((selection, scroll)) = next {
            store.set_view_state(view, selection, scroll, current.folds)?;
            *store.cursor_memory_mut(view)? = memory;
        }
        return Ok(false);
    }
    let mut memory = memory.clone();
    let is_match = matches!(
        command,
        CursorCommand::AddNextMatch
            | CursorCommand::AddPreviousMatch
            | CursorCommand::MoveNextMatch
            | CursorCommand::MovePreviousMatch
            | CursorCommand::SelectMatches
            | CursorCommand::ChangeAll
    );
    let is_vertical = matches!(command, CursorCommand::AddAbove | CursorCommand::AddBelow);
    let selection = if is_vertical {
        add_vertical_cursors(
            &document,
            &current,
            if command == CursorCommand::AddAbove {
                -1
            } else {
                1
            },
            context,
            &mut memory,
        )
    } else if is_match {
        match_selection(&document, &current.selection, command, context, &mut memory)
    } else if matches!(command, CursorCommand::Expand | CursorCommand::Shrink) {
        smart_selection(
            &document,
            &current.selection,
            command == CursorCommand::Expand,
            context,
            &mut memory,
        )
    } else {
        cursor_selection(&document, &current.selection, command, context)
    };
    if selection != current.selection {
        store.break_undo_group(document.id)?;
        store.set_composition(view, None)?;
        store.set_view_state(view, selection, current.scroll, current.folds)?;
    }
    if is_match || matches!(command, CursorCommand::Expand | CursorCommand::Shrink) {
        let state = store.cursor_memory_mut(view)?;
        if is_match {
            state.matches = memory.matches;
        } else {
            state.smart = memory.smart;
        }
    }
    if is_vertical {
        store.cursor_memory_mut(view)?.vertical = memory.vertical;
    }
    let selected = &store
        .views()
        .get(view)
        .ok_or(EditorError::NotFound)?
        .selection;
    let reveal = match command {
        CursorCommand::AddAbove => selected
            .selections
            .iter()
            .map(|selection| selection.head)
            .min()
            .map(|byte| byte..byte),
        CursorCommand::AddBelow => selected
            .selections
            .iter()
            .map(|selection| selection.head)
            .max()
            .map(|byte| byte..byte),
        CursorCommand::FocusNext
        | CursorCommand::FocusPrevious
        | CursorCommand::JumpToBracket
        | CursorCommand::SelectToBracket => {
            let primary = selected.selections[selected.primary];
            Some(primary.anchor.min(primary.head)..primary.anchor.max(primary.head))
        }
        CursorCommand::AddNextMatch
        | CursorCommand::AddPreviousMatch
        | CursorCommand::MoveNextMatch
        | CursorCommand::MovePreviousMatch => selected.selections.last().map(|selection| {
            selection.anchor.min(selection.head)..selection.anchor.max(selection.head)
        }),
        CursorCommand::ExpandLine => {
            let first = selected
                .selections
                .iter()
                .map(|selection| selection.head)
                .min()
                .unwrap();
            let last = selected
                .selections
                .iter()
                .map(|selection| selection.head)
                .max()
                .unwrap();
            Some(first..last)
        }
        _ => None,
    };
    if let Some(bytes) = reveal {
        store.request_selection_reveal(view, bytes, is_match)?;
    }
    Ok(false)
}

pub fn cursor_selection(
    document: &DocumentSnapshot,
    selection: &SelectionSet,
    command: CursorCommand,
    context: LineCommandContext<'_>,
) -> SelectionSet {
    let mut result = selection.clone();
    match command {
        CursorCommand::AddAbove | CursorCommand::AddBelow => {
            let delta = if command == CursorCommand::AddAbove {
                -1
            } else {
                1
            };
            result.selections = selection
                .selections
                .iter()
                .flat_map(|s| {
                    [
                        *s,
                        Selection {
                            anchor: translate(document, s.anchor, delta, context.indent.tab_size),
                            head: translate(document, s.head, delta, context.indent.tab_size),
                        },
                    ]
                })
                .collect();
            result.primary *= 2;
        }
        CursorCommand::LineEnds => {
            let mut selections = Vec::new();
            for s in &selection.selections {
                if s.anchor == s.head {
                    continue;
                }
                let start = document.rope.byte_to_line(s.anchor.min(s.head));
                let end = document.rope.byte_to_line(s.anchor.max(s.head));
                for line in start..end {
                    let end = line_content_range(document, line).end;
                    selections.push(Selection {
                        anchor: end,
                        head: end,
                    });
                }
                let end_byte = s.anchor.max(s.head);
                if end_byte > document.rope.line_to_byte(end) {
                    selections.push(Selection {
                        anchor: end_byte,
                        head: end_byte,
                    });
                }
            }
            if !selections.is_empty() {
                result = SelectionSet {
                    primary: 0,
                    selections,
                };
            }
        }
        CursorCommand::ToTop | CursorCommand::ToBottom => {
            let first = selection.selections[selection.primary];
            let (line, start) = position(document, first.anchor.min(first.head));
            let (_, end) = position(document, first.anchor.max(first.head));
            let lines: Vec<_> = if command == CursorCommand::ToTop {
                (0..=line).rev().collect()
            } else {
                (line..document.rope.len_lines()).collect()
            };
            result = SelectionSet {
                primary: 0,
                selections: lines
                    .into_iter()
                    .map(|line| Selection {
                        anchor: at_column(document, line, start),
                        head: at_column(document, line, end),
                    })
                    .collect(),
            };
        }
        CursorCommand::FocusNext => {
            result.selections.rotate_left(1);
            result.primary = 0;
        }
        CursorCommand::FocusPrevious => {
            result.selections.rotate_right(1);
            result.primary = 0;
        }
        CursorCommand::ExpandLine => {
            result.selections = selection
                .selections
                .iter()
                .map(|s| {
                    let first = document.rope.byte_to_line(s.anchor.min(s.head));
                    let last = document.rope.byte_to_line(s.anchor.max(s.head));
                    let head = if last + 1 < document.rope.len_lines() {
                        document.rope.line_to_byte(last + 1)
                    } else {
                        document.rope.len_bytes()
                    };
                    Selection {
                        anchor: document.rope.line_to_byte(first),
                        head,
                    }
                })
                .collect();
        }
        CursorCommand::JumpToBracket | CursorCommand::SelectToBracket => {
            let Some(language) = context.language else {
                return result;
            };
            let pairs = bracket_pairs(document, language);
            let mut selections = Vec::new();
            for s in &selection.selections {
                let byte = s.anchor.min(s.head);
                let matched = matching_brackets(&pairs, byte);
                let enclosing = enclosing_brackets(&pairs, byte);
                let next = next_bracket(document, language, byte, &pairs);
                if command == CursorCommand::JumpToBracket {
                    let target = matched
                        .map(|pair| {
                            if contains(&pair.close, byte) {
                                pair.open.start
                            } else {
                                pair.close.start
                            }
                        })
                        .or_else(|| enclosing.map(|pair| pair.close.start))
                        .or_else(|| next.as_ref().map(|range| range.start))
                        .unwrap_or(byte);
                    selections.push(Selection {
                        anchor: target,
                        head: target,
                    });
                } else if let Some(pair) = matched.or(enclosing).or_else(|| {
                    next.as_ref()
                        .and_then(|range| matching_brackets(&pairs, range.start))
                }) {
                    let (anchor, head) = if contains(&pair.close, byte) {
                        (pair.close.end, pair.open.start)
                    } else {
                        (pair.open.start, pair.close.end)
                    };
                    selections.push(Selection { anchor, head });
                }
            }
            if !selections.is_empty() {
                result = SelectionSet {
                    primary: 0,
                    selections,
                };
            }
        }
        _ => {}
    }
    result.normalized()
}

pub fn word_range(
    document: &DocumentSnapshot,
    byte: usize,
    rules: Option<&dyn LanguageRules>,
) -> Option<Range<usize>> {
    let line = document.rope.byte_to_line(byte);
    let content = line_text(document, line);
    let offset = byte.saturating_sub(content.start).min(content.text.len());
    if let Some(rules) = rules {
        return rules
            .word_range(&content.text, offset)
            .map(|range| content.start + range.start..content.start + range.end);
    }
    let mut start = None;
    for (position, character) in content
        .text
        .char_indices()
        .chain(std::iter::once((content.text.len(), ' ')))
    {
        if !is_js_whitespace(character) && !crate::editing::WORD_SEPARATORS.contains(character) {
            if start.is_none() {
                start = Some(position);
            }
        } else if let Some(first) = start.take() {
            if first <= offset && offset <= position {
                return Some(content.start + first..content.start + position);
            }
        }
    }
    None
}

fn configured_word(
    document: &DocumentSnapshot,
    byte: usize,
    context: LineCommandContext<'_>,
) -> Option<Range<usize>> {
    word_range(
        document,
        byte,
        context
            .word_rules
            .or(context.language.map(|language| language.rules)),
    )
}

fn fold_character(character: char) -> char {
    if character == 'ı' {
        return character;
    }
    let mut upper = character.to_uppercase();
    let first = upper.next().unwrap();
    if upper.next().is_some() {
        return character;
    }
    let mut lower = first.to_lowercase();
    let first = lower.next().unwrap();
    if lower.next().is_none() {
        first
    } else {
        character
    }
}

pub fn literal_match_ranges<'a>(
    document: &'a DocumentSnapshot,
    text: &str,
    match_case: bool,
    whole_word: bool,
) -> impl Iterator<Item = Range<usize>> + use<'a> {
    let needle: Vec<_> = text
        .replace("\r\n", "\n")
        .chars()
        .map(|c| if match_case { c } else { fold_character(c) })
        .collect();
    let mut prefix = vec![0; needle.len()];
    for index in 1..needle.len() {
        let mut matched = prefix[index - 1];
        while matched > 0 && needle[index] != needle[matched] {
            matched = prefix[matched - 1];
        }
        if needle[index] == needle[matched] {
            matched += 1;
        }
        prefix[index] = matched;
    }
    let mut characters = document.rope.chars().peekable();
    let mut ring = std::collections::VecDeque::new();
    let mut byte = 0;
    let mut matched = 0;
    let separator = |c: char| {
        c == '\r'
            || c == '\n'
            || c == ' '
            || c == '\t'
            || crate::editing::WORD_SEPARATORS.contains(c)
    };
    std::iter::from_fn(move || {
        if needle.is_empty() {
            return None;
        }
        while let Some(mut character) = characters.next() {
            let start = byte;
            byte += character.len_utf8();
            if character == '\r' && characters.peek() == Some(&'\n') {
                characters.next();
                byte += 1;
                character = '\n';
            }
            ring.push_back((start, character));
            if ring.len() > needle.len() + 1 {
                ring.pop_front();
            }
            let compared = if match_case {
                character
            } else {
                fold_character(character)
            };
            while matched > 0 && compared != needle[matched] {
                matched = prefix[matched - 1];
            }
            if compared == needle[matched] {
                matched += 1;
            }
            if matched != needle.len() {
                continue;
            }
            matched = 0;
            let first = ring.len() - needle.len();
            let left = first == 0 || separator(ring[first - 1].1) || separator(ring[first].1);
            let right = characters.peek().is_none_or(|c| separator(*c)) || separator(character);
            if !whole_word || left && right {
                return Some(ring[first].0..byte);
            }
        }
        None
    })
}

fn match_selection(
    document: &DocumentSnapshot,
    selection: &SelectionSet,
    command: CursorCommand,
    context: LineCommandContext<'_>,
    memory: &mut CursorMemory,
) -> SelectionSet {
    let primary = selection.selections[selection.primary];
    let first = primary.anchor.min(primary.head)..primary.anchor.max(primary.head);
    let initial = memory.matches.is_none();
    let mut current_match = None;
    if initial {
        if command == CursorCommand::AddNextMatch && selection.selections.len() > 1 {
            let texts: Vec<_> = selection
                .selections
                .iter()
                .map(|s| {
                    document
                        .rope
                        .byte_slice(s.anchor.min(s.head)..s.anchor.max(s.head))
                        .to_string()
                })
                .collect();
            if texts
                .iter()
                .skip(1)
                .any(|text| text.to_lowercase() != texts[0].to_lowercase())
            {
                return SelectionSet {
                    primary: selection.primary,
                    selections: selection
                        .selections
                        .iter()
                        .map(|s| {
                            if s.anchor != s.head {
                                return *s;
                            }
                            configured_word(document, s.head, context).map_or(*s, |range| {
                                Selection {
                                    anchor: range.start,
                                    head: range.end,
                                }
                            })
                        })
                        .collect(),
                }
                .normalized();
            }
        }
        let range = if first.is_empty() {
            let Some(word) = configured_word(document, first.start, context) else {
                return selection.clone();
            };
            current_match = Some(word.clone());
            word
        } else {
            first.clone()
        };
        let disconnected = selection.selections.len() == 1 && first.is_empty();
        memory.matches = Some(MatchSession {
            text: document
                .rope
                .byte_slice(range)
                .to_string()
                .replace("\r\n", "\n"),
            whole_word: disconnected,
            match_case: disconnected,
        });
    }
    let session = memory.matches.as_ref().unwrap();
    let matches = || {
        literal_match_ranges(
            document,
            &session.text,
            session.match_case,
            session.whole_word,
        )
    };
    if matches!(
        command,
        CursorCommand::SelectMatches | CursorCommand::ChangeAll
    ) {
        let mut ranges: Vec<_> = matches().take(MULTICURSOR_LIMIT).collect();
        if ranges.is_empty() {
            return selection.clone();
        }
        if let Some(index) = ranges
            .iter()
            .position(|range| range.start <= first.end && range.end >= first.start)
        {
            ranges.swap(0, index);
        } else if let Some(range) =
            matches().find(|range| range.start <= first.end && range.end >= first.start)
        {
            ranges[0] = range;
        }
        return SelectionSet {
            primary: 0,
            selections: ranges
                .into_iter()
                .map(|range| Selection {
                    anchor: range.start,
                    head: range.end,
                })
                .collect(),
        }
        .normalized();
    }
    let last = selection.selections.last().unwrap();
    let previous = matches!(
        command,
        CursorCommand::AddPreviousMatch | CursorCommand::MovePreviousMatch
    );
    let next = current_match.or_else(|| {
        if previous {
            matches()
                .filter(|range| range.end <= last.anchor.min(last.head))
                .last()
                .or_else(|| matches().last())
        } else {
            matches()
                .find(|range| range.start >= last.anchor.max(last.head))
                .or_else(|| matches().next())
        }
    });
    let Some(next) = next else {
        return selection.clone();
    };
    let mut result = selection.clone();
    if matches!(
        command,
        CursorCommand::MoveNextMatch | CursorCommand::MovePreviousMatch
    ) {
        result.selections.pop();
    }
    if result.selections.len() < MULTICURSOR_LIMIT {
        result.selections.push(Selection {
            anchor: next.start,
            head: next.end,
        });
    }
    result.normalized()
}

fn subword(text: &str, column: usize) -> Option<Range<usize>> {
    let units: Vec<_> = text.encode_utf16().collect();
    let upper = |c: u16| c >= u16::from(b'A') && c <= u16::from(b'Z');
    let lower = |c: u16| c >= u16::from(b'a') && c <= u16::from(b'z');
    let separator = |c: u16| c == u16::from(b'_') || c == u16::from(b'-');
    let mut start = column as isize;
    let mut last = 0;
    while start >= 0 {
        let c = units.get(start as usize).copied().unwrap_or(0);
        if start as usize != column && separator(c) || lower(c) && upper(last) {
            break;
        }
        last = c;
        start -= 1;
    }
    let start = (start + 1) as usize;
    let mut end = column;
    while let Some(c) = units.get(end).copied() {
        if upper(c) && lower(last) || separator(c) {
            break;
        }
        last = c;
        end += 1;
    }
    (start < end).then(|| byte_at_column(text, start)..byte_at_column(text, end))
}

pub fn selection_ranges(
    document: &DocumentSnapshot,
    selection: Selection,
    context: LineCommandContext<'_>,
) -> Vec<Range<usize>> {
    let byte = selection.head;
    let mut ranges = Vec::new();
    if let Some(word) = configured_word(document, byte, context) {
        let text = document.rope.byte_slice(word.clone()).to_string();
        let column = document
            .rope
            .byte_slice(word.start..byte)
            .to_string()
            .encode_utf16()
            .count();
        if let Some(part) = subword(&text, column) {
            ranges.push(word.start + part.start..word.start + part.end);
        }
        ranges.push(word);
    }
    let content = line_text(document, document.rope.byte_to_line(byte));
    if !content.text.is_empty() && content.text.chars().all(|c| c == ' ' || c == '\t') {
        ranges.push(content.start..content.start + content.text.len());
    }
    if let Some(language) = context.language {
        for pair in bracket_pairs(document, language)
            .iter()
            .filter(|pair| pair.open.end <= byte && byte <= pair.close.start)
        {
            ranges.push(pair.open.end..pair.close.start);
            ranges.push(pair.open.start..pair.close.end);
            let start_line = document.rope.byte_to_line(pair.open.start);
            if start_line == document.rope.byte_to_line(pair.close.end) {
                continue;
            }
            let first = line_text(document, start_line);
            let leading = first.text.len() - first.text.trim_start_matches([' ', '\t']).len();
            if leading < first.text.len() && first.start + leading != pair.open.start {
                ranges.push(first.start + leading..pair.close.end);
                ranges.push(first.start..pair.close.end);
            }
            if start_line > 0 {
                let above = line_text(document, start_line - 1);
                let above_leading =
                    above.text.len() - above.text.trim_start_matches([' ', '\t']).len();
                if above_leading < above.text.trim_end_matches([' ', '\t']).len()
                    && above.text[..above_leading].encode_utf16().count()
                        == first.text[..pair.open.start - first.start]
                            .encode_utf16()
                            .count()
                {
                    ranges.push(above.start + above_leading..pair.close.end);
                    ranges.push(above.start..pair.close.end);
                }
            }
        }
    }
    ranges.push(0..document.rope.len_bytes());
    ranges.sort_by_key(|range| (std::cmp::Reverse(range.start), range.end));
    let mut nested: Vec<Range<usize>> = Vec::new();
    for range in ranges {
        if nested
            .last()
            .is_none_or(|last| range != *last && range.start <= last.start && range.end >= last.end)
        {
            nested.push(range);
        }
    }
    let mut trivia = Vec::new();
    if let Some(first) = nested.first() {
        trivia.push(first.clone());
    }
    for pair in nested.windows(2) {
        let (previous, current) = (&pair[0], &pair[1]);
        let (start_line, _) = position(document, previous.start);
        let (end_line, _) = position(document, previous.end);
        if start_line != document.rope.byte_to_line(current.start)
            || end_line != document.rope.byte_to_line(current.end)
        {
            let start = line_text(document, start_line);
            let end = line_text(document, end_line);
            let trimmed = start.start + start.text.len()
                - start.text.trim_start_matches([' ', '\t']).len()
                ..end.start + end.text.trim_end_matches([' ', '\t']).len();
            let full = start.start..end.start + end.text.len();
            for range in [trimmed, full] {
                if range.start <= previous.start
                    && range.end >= previous.end
                    && current.start <= range.start
                    && current.end >= range.end
                    && range != *current
                    && range != *previous
                    && trivia.last() != Some(&range)
                {
                    trivia.push(range);
                }
            }
        }
        trivia.push(current.clone());
    }
    let selected = selection.anchor.min(selection.head)..selection.anchor.max(selection.head);
    trivia.retain(|range| range.start <= selected.start && range.end >= selected.end);
    trivia.insert(0, selected);
    trivia
}

fn smart_selection(
    document: &DocumentSnapshot,
    selection: &SelectionSet,
    forward: bool,
    context: LineCommandContext<'_>,
    memory: &mut CursorMemory,
) -> SelectionSet {
    let state = memory.smart.get_or_insert_with(|| SmartSelection {
        ranges: selection
            .selections
            .iter()
            .map(|s| selection_ranges(document, *s, context))
            .collect(),
        indices: vec![0; selection.selections.len()],
    });
    let mut selections = Vec::new();
    for (index, ranges) in state.ranges.iter().enumerate() {
        let previous = &ranges[state.indices[index]];
        let mut next = state.indices[index];
        loop {
            let requested = if forward {
                next + 1
            } else {
                next.saturating_sub(1)
            };
            if requested == next || requested >= ranges.len() {
                break;
            }
            next = requested;
            if ranges[next] != *previous {
                break;
            }
        }
        state.indices[index] = next;
        selections.push(Selection {
            anchor: ranges[next].start,
            head: ranges[next].end,
        });
    }
    SelectionSet {
        primary: selection.primary,
        selections,
    }
    .normalized()
}

fn translate(document: &DocumentSnapshot, byte: usize, delta: isize, tab_size: u32) -> usize {
    let line = document.rope.byte_to_line(byte);
    let content = line_text(document, line);
    let column = visible_column(
        &content.text[..byte.saturating_sub(content.start).min(content.text.len())],
        tab_size.max(1) as usize,
    );
    let target = line
        .saturating_add_signed(delta)
        .min(document.rope.len_lines() - 1);
    let content = line_text(document, target);
    content.start + offset_at_visible_column(&content.text, column, tab_size.max(1) as usize)
}

fn visible_at(document: &DocumentSnapshot, byte: usize, tab_size: usize) -> usize {
    let content = line_text(document, document.rope.byte_to_line(byte));
    visible_column(
        &content.text[..byte.saturating_sub(content.start).min(content.text.len())],
        tab_size,
    )
}

fn add_vertical_cursors(
    document: &DocumentSnapshot,
    current: &ViewState,
    delta: isize,
    context: LineCommandContext<'_>,
    memory: &mut CursorMemory,
) -> SelectionSet {
    let size = context.indent.tab_size.max(1) as usize;
    let mut goals = Vec::new();
    let mut selections = Vec::new();
    for (index, s) in current.selection.selections.iter().enumerate() {
        let stored = memory
            .vertical
            .as_ref()
            .and_then(|goals| goals.iter().find(|(selection, _, _)| selection == s));
        let remaining = current
            .goal_columns
            .as_ref()
            .filter(|goal| goal.revision == document.revision)
            .and_then(|goal| goal.leftover_visible_columns.get(index))
            .copied()
            .unwrap_or(0);
        let head_goal = stored.map_or(
            (visible_at(document, s.head, size) as isize + remaining).max(0) as usize,
            |(_, _, head)| *head,
        );
        let anchor_goal = stored.map_or_else(
            || {
                if s.anchor == s.head {
                    head_goal
                } else {
                    visible_at(document, s.anchor, size)
                }
            },
            |(_, anchor, _)| *anchor,
        );
        let move_to = |byte: usize, goal: usize| {
            let line = document.rope.byte_to_line(byte);
            let target = line
                .saturating_add_signed(delta)
                .min(document.rope.len_lines() - 1);
            if line == target {
                return byte;
            }
            let content = line_text(document, target);
            content.start + offset_at_visible_column(&content.text, goal, size)
        };
        let next = Selection {
            anchor: move_to(s.anchor, anchor_goal),
            head: move_to(s.head, head_goal),
        };
        selections.extend([*s, next]);
        goals.extend([(*s, anchor_goal, head_goal), (next, anchor_goal, head_goal)]);
    }
    let selection = SelectionSet {
        primary: current.selection.primary * 2,
        selections,
    }
    .normalized();
    memory.vertical = Some(
        selection
            .selections
            .iter()
            .map(|s| {
                goals
                    .iter()
                    .find(|(selection, _, _)| selection == s)
                    .copied()
                    .unwrap_or((
                        *s,
                        visible_at(document, s.anchor, size),
                        visible_at(document, s.head, size),
                    ))
            })
            .collect(),
    );
    selection
}

fn position(document: &DocumentSnapshot, byte: usize) -> (usize, usize) {
    let line = document.rope.byte_to_line(byte);
    let content = line_text(document, line);
    (
        line,
        content.text[..byte.saturating_sub(content.start).min(content.text.len())]
            .encode_utf16()
            .count(),
    )
}

fn at_column(document: &DocumentSnapshot, line: usize, column: usize) -> usize {
    let content = line_text(document, line);
    content.start + byte_at_column(&content.text, column)
}

fn byte_at_column(text: &str, column: usize) -> usize {
    let mut units = 0;
    for (byte, character) in text.char_indices() {
        if units + character.len_utf16() > column {
            return byte;
        }
        units += character.len_utf16();
    }
    text.len()
}

fn move_caret(
    store: &mut EditorStore,
    view: ViewId,
    document: &DocumentSnapshot,
    selection: &SelectionSet,
    left: bool,
) -> Result<bool, EditorError> {
    let mut plan = Plan::new(selection);
    for (index, s) in selection.selections.iter().enumerate() {
        let (start, end) = (s.anchor.min(s.head), s.anchor.max(s.head));
        let (line, start_column) = position(document, start);
        let (end_line, end_column) = position(document, end);
        if start == end || line != end_line {
            continue;
        }
        let content = line_text(document, line);
        if left && start_column == 0 || !left && end == content.start + content.text.len() {
            continue;
        }
        let (range, replacement) = if left {
            let previous = at_column(document, line, start_column - 1);
            (
                previous..end,
                format!(
                    "{}{}",
                    document.rope.byte_slice(start..end),
                    document.rope.byte_slice(previous..start)
                ),
            )
        } else {
            let next = content
                .text
                .char_indices()
                .find(|(byte, _)| content.start + *byte >= end)
                .map(|(byte, c)| content.start + byte + c.len_utf8())
                .unwrap_or(end);
            (
                start..next,
                format!(
                    "{}{}",
                    document.rope.byte_slice(end..next),
                    document.rope.byte_slice(start..end)
                ),
            )
        };
        let range_start = range.start;
        let mut updated = content.text.clone();
        updated.replace_range(
            range.start - content.start..range.end - content.start,
            &replacement,
        );
        if let Some(edit) = plan.edit(range, replacement) {
            let shift = |column: usize| {
                if left {
                    column.saturating_sub(1)
                } else {
                    column + 1
                }
            };
            let anchor = content.start + byte_at_column(&updated, shift(start_column));
            let head = content.start + byte_at_column(&updated, shift(end_column));
            plan.marks[index] = (
                Mark::InEdit {
                    edit,
                    bytes: anchor.saturating_sub(range_start),
                },
                Mark::InEdit {
                    edit,
                    bytes: head.saturating_sub(range_start),
                },
            );
        }
    }
    apply_step(store, view, plan.transaction(document, view), SEPARATE_STEP)
}

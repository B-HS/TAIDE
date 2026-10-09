use std::cmp::Ordering;
use std::collections::HashSet;
use std::ops::Range;

use crate::auto_indent;
use crate::document::{DocumentSnapshot, EditorError};
use crate::editing::{
    Mark, Plan, SEPARATE_STEP, apply_step, indentation, leading_whitespace, line_text,
    normalize_line_breaks, ordered, ranges_transaction, shift_language_lines, tab_width,
    view_document, visible_column, word_delete_inside_range,
};
use crate::indent::IndentOptions;
use crate::language_configuration::{
    Language, LanguageRules, LineSyntax, is_js_whitespace, token_kind_at,
};
use crate::store::{EditorStore, Transaction};
use crate::syntax::TokenKind;
use crate::view::{Selection, SelectionSet, ViewId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineCommand {
    MoveLinesUp,
    MoveLinesDown,
    CopyLinesUp,
    CopyLinesDown,
    DuplicateSelection,
    DeleteLines,
    InsertLineBefore,
    InsertLineAfter,
    JoinLines,
    DeleteAllLeft,
    DeleteAllRight,
    DeleteInsideWord,
    IndentLines,
    OutdentLines,
    TrimTrailingWhitespace,
    InsertFinalNewLine,
    SortLinesAscending,
    SortLinesDescending,
    RemoveDuplicateLines,
    ReverseLines,
    Transform(TextCase),
    ToggleLineComment,
    AddLineComment,
    RemoveLineComment,
    ToggleBlockComment,
    RemoveBrackets,
    Transpose,
    TransposeLetters,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextCase {
    Upper,
    Lower,
    Title,
    Snake,
    Camel,
    Pascal,
    Kebab,
}

pub trait TextTransforms {
    fn transform(&self, case: TextCase, text: &str) -> String;
}

#[derive(Clone, Copy)]
pub struct LineCommandContext<'a> {
    pub indent: IndentOptions,
    pub language: Option<Language<'a>>,
    pub syntax: &'a dyn LineSyntax,
    pub compare: Option<&'a dyn Fn(&str, &str) -> Ordering>,
    pub transforms: Option<&'a dyn TextTransforms>,
    pub word_rules: Option<&'a dyn LanguageRules>,
}

pub fn run_line_command(
    store: &mut EditorStore,
    view: ViewId,
    command: LineCommand,
    context: LineCommandContext<'_>,
) -> Result<bool, EditorError> {
    context.syntax.follow_edits(store);
    let (current, document) = view_document(store, view)?;
    if document.metadata.read_only {
        return Err(EditorError::ReadOnly);
    }
    if command == LineCommand::RemoveBrackets {
        let Some(language) = context.language else {
            return Ok(false);
        };
        let mut changed = false;
        for selected in &current.selection.selections {
            let (line, column) = position(&document, selected.head);
            let (current, snapshot) = view_document(store, view)?;
            let line = line.min(snapshot.rope.len_lines() - 1);
            let content = line_text(&snapshot, line);
            let head = content.start + byte_at_column(&content.text, column);
            let pairs = crate::bracket_navigation::bracket_pairs(&snapshot, language);
            if let Some(pair) = crate::bracket_navigation::matching_brackets(&pairs, head)
                .or_else(|| crate::bracket_navigation::enclosing_brackets(&pairs, head))
            {
                let mut plan = Plan::new(&current.selection);
                plan.edit(pair.open.clone(), String::new());
                plan.edit(pair.close.clone(), String::new());
                changed |= apply_step(
                    store,
                    view,
                    plan.transaction(&snapshot, view),
                    SEPARATE_STEP,
                )?;
                context.syntax.follow_edits(store);
            }
        }
        return Ok(changed);
    }
    let transaction =
        line_command_transaction(&document, view, &current.selection, command, context)?;
    store.set_composition(view, None)?;
    let unchanged = transaction.as_ref().is_some_and(|transaction| {
        transaction
            .edits
            .iter()
            .all(|edit| document.rope.byte_slice(edit.bytes.clone()).to_string() == edit.text)
    });
    if unchanged {
        if let Some(selection) = transaction.and_then(|transaction| transaction.selection_after) {
            let head = selection.selections[selection.primary].head;
            store.set_view_state(view, selection, current.scroll, current.folds)?;
            store.request_selection_reveal(view, head..head, false)?;
        }
        return apply_step(store, view, None, SEPARATE_STEP);
    }
    let changed = apply_step(store, view, transaction, SEPARATE_STEP)?;
    context.syntax.follow_edits(store);
    if changed {
        let selected = &store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .selection;
        let head = selected.selections[selected.primary].head;
        store.request_selection_reveal(view, head..head, false)?;
    }
    Ok(changed)
}

pub fn line_command_transaction(
    document: &DocumentSnapshot,
    view: ViewId,
    selection: &SelectionSet,
    command: LineCommand,
    context: LineCommandContext<'_>,
) -> Result<Option<Transaction>, EditorError> {
    selection.validate(&document.rope)?;
    let plan = match command {
        LineCommand::MoveLinesUp | LineCommand::MoveLinesDown => moved_lines(
            document,
            selection,
            command == LineCommand::MoveLinesDown,
            context,
        ),
        LineCommand::CopyLinesUp | LineCommand::CopyLinesDown | LineCommand::DuplicateSelection => {
            copied_lines(document, selection, command)
        }
        LineCommand::DeleteLines => deleted_lines(document, selection),
        LineCommand::InsertLineBefore | LineCommand::InsertLineAfter => inserted_lines(
            document,
            selection,
            command,
            context.indent,
            context.language,
        ),
        LineCommand::JoinLines => joined_lines(document, selection),
        LineCommand::Transpose => {
            let mut plan = Plan::new(selection);
            for (index, selected) in selection.selections.iter().enumerate() {
                if selected.anchor != selected.head {
                    continue;
                }
                let (line, column) = position(document, selected.head);
                let content = line_text(document, line);
                let at_end = selected.head >= content.start + content.text.len();
                if at_end && line + 1 == document.rope.len_lines() {
                    continue;
                }
                let begin = byte_at_column(&content.text, column.saturating_sub(1));
                let end = if at_end {
                    document.rope.line_to_byte(line + 1)
                } else {
                    let mut end = byte_at_column(&content.text, column + 1);
                    if content.text[..end].encode_utf16().count() < column + 1 {
                        end += content.text[end..].chars().next().unwrap().len_utf8();
                    }
                    content.start + end
                };
                let mut units: Vec<u16> = document
                    .rope
                    .byte_slice(content.start + begin..end)
                    .to_string()
                    .encode_utf16()
                    .collect();
                units.reverse();
                let replacement = normalize_line_breaks(
                    &String::from_utf16_lossy(&units),
                    document.metadata.line_ending.as_str(),
                );
                let target = column + 1 - content.text[..begin].encode_utf16().count();
                let bytes = byte_at_column(&replacement, target);
                if let Some(edit) = plan.edit(content.start + begin..end, replacement) {
                    let mark = if at_end {
                        Mark::AfterEdit(edit)
                    } else {
                        Mark::InEdit { edit, bytes }
                    };
                    plan.marks[index] = (mark, mark);
                }
            }
            plan
        }
        LineCommand::TransposeLetters => {
            let mut plan = Plan::new(selection);
            for (index, selected) in selection.selections.iter().enumerate() {
                if selected.anchor != selected.head {
                    continue;
                }
                let head = selected.head;
                let line = document.rope.byte_to_line(head);
                let content = line_text(document, line);
                if line == 0
                    && (head == 0
                        || (head == content.text.len() && content.text.encode_utf16().count() == 1))
                {
                    continue;
                }
                let end = if head == content.start + content.text.len() {
                    head
                } else {
                    crate::editing::grapheme_boundary(document, head, true)?
                };
                let middle = crate::editing::grapheme_boundary(document, end, false)?;
                let begin = crate::editing::grapheme_boundary(document, middle, false)?;
                let left = document.rope.byte_slice(begin..middle).to_string();
                let right = document.rope.byte_slice(middle..end).to_string();
                plan.replace(index, begin..end, format!("{right}{left}"));
            }
            plan
        }
        LineCommand::RemoveBrackets => {
            let mut plan = Plan::new(selection);
            if let Some(language) = context.language {
                let pairs = crate::bracket_navigation::bracket_pairs(document, language);
                for selected in &selection.selections {
                    if let Some(pair) =
                        crate::bracket_navigation::matching_brackets(&pairs, selected.head).or_else(
                            || crate::bracket_navigation::enclosing_brackets(&pairs, selected.head),
                        )
                    {
                        plan.edit(pair.open.clone(), String::new());
                        plan.edit(pair.close.clone(), String::new());
                    }
                }
            }
            plan
        }
        LineCommand::ToggleLineComment
        | LineCommand::AddLineComment
        | LineCommand::RemoveLineComment
        | LineCommand::ToggleBlockComment => commented_lines(document, selection, command, context),
        LineCommand::Transform(case) => {
            let mut plan = Plan::new(selection);
            if let Some(transforms) = context.transforms {
                for (index, selected) in selection.selections.iter().enumerate() {
                    let mut bytes = ordered(selected);
                    if bytes.is_empty() {
                        let content =
                            line_text(document, document.rope.byte_to_line(selected.head));
                        let Some(word) = context
                            .word_rules
                            .or(context.language.map(|language| language.rules))
                            .and_then(|rules| {
                                rules.word_range(&content.text, selected.head - content.start)
                            })
                        else {
                            continue;
                        };
                        bytes = content.start + word.start..content.start + word.end;
                    }
                    let text = document.rope.byte_slice(bytes.clone()).to_string();
                    let replacement = transforms.transform(case, &text);
                    let old_units = text.encode_utf16().count();
                    let new_units = replacement.encode_utf16().count();
                    if let Some(edit) = plan.edit(bytes.clone(), replacement.clone()) {
                        let mark = |offset| {
                            let units = text[..offset - bytes.start].encode_utf16().count();
                            let common = old_units.min(new_units);
                            let target =
                                if units < common || (units == common && old_units > new_units) {
                                    units
                                } else {
                                    new_units
                                };
                            Mark::InEdit {
                                edit,
                                bytes: byte_at_column(&replacement, target),
                            }
                        };
                        plan.marks[index] = (mark(selected.anchor), mark(selected.head));
                    }
                }
            }
            plan
        }
        LineCommand::SortLinesAscending
        | LineCommand::SortLinesDescending
        | LineCommand::RemoveDuplicateLines
        | LineCommand::ReverseLines => {
            reordered_lines(document, selection, command, context.compare)
        }
        LineCommand::DeleteAllLeft
        | LineCommand::DeleteAllRight
        | LineCommand::DeleteInsideWord => {
            let mut ranges = selection
                .selections
                .iter()
                .map(|selected| {
                    let range = ordered(selected);
                    let line = document.rope.byte_to_line(range.start);
                    let content = line_text(document, line);
                    match command {
                        LineCommand::DeleteAllLeft
                            if !range.is_empty() || range.start > content.start =>
                        {
                            content.start..range.end
                        }
                        LineCommand::DeleteAllLeft if line > 0 => {
                            let previous = line_text(document, line - 1);
                            previous.start + previous.text.len()..range.end
                        }
                        LineCommand::DeleteAllRight if range.is_empty() => {
                            let end = content.start + content.text.len();
                            let end = if range.start == end && line + 1 < document.rope.len_lines()
                            {
                                document.rope.line_to_byte(line + 1)
                            } else {
                                end
                            };
                            range.start..end
                        }
                        LineCommand::DeleteInsideWord if range.is_empty() => {
                            word_delete_inside_range(document, selected.head)
                        }
                        _ => range,
                    }
                })
                .collect::<Vec<_>>();
            let primary = ranges[selection.primary].clone();
            ranges.sort_by_key(|range| (range.start, range.end));
            let mut merged = Vec::<Range<usize>>::new();
            for range in ranges {
                if let Some(previous) = merged.last_mut()
                    && range.start <= previous.end
                {
                    previous.end = previous.end.max(range.end);
                    continue;
                }
                merged.push(range);
            }
            let primary = merged
                .iter()
                .position(|range| range.start <= primary.start && range.end >= primary.end)
                .unwrap_or(0);
            return Ok(ranges_transaction(document, view, primary, merged, ""));
        }
        LineCommand::IndentLines | LineCommand::OutdentLines => {
            let mut plan = Plan::new(selection);
            for (index, selected) in selection.selections.iter().enumerate() {
                shift_language_lines(
                    &mut plan,
                    document,
                    index,
                    selected,
                    context.indent,
                    command == LineCommand::OutdentLines,
                    context.language,
                );
            }
            plan
        }
        LineCommand::TrimTrailingWhitespace => {
            let selected = selection.selections[selection.primary];
            let mut plan = Plan::new(&SelectionSet {
                primary: 0,
                selections: vec![selected],
            });
            for line in 0..document.rope.len_lines() {
                let content = line_text(document, line);
                let keep = content.text.trim_end_matches([' ', '\t']).len();
                if keep == content.text.len() {
                    continue;
                }
                let Some(tokens) = context.syntax.accurate_tokens(document, line) else {
                    continue;
                };
                if matches!(
                    token_kind_at(&tokens, (keep + 1).min(content.text.len())),
                    TokenKind::String | TokenKind::Regex
                ) {
                    continue;
                }
                plan.edit(
                    content.start + keep..content.start + content.text.len(),
                    String::new(),
                );
            }
            plan
        }
        LineCommand::InsertFinalNewLine => {
            let selected = selection.selections[selection.primary];
            let mut plan = Plan::new(&SelectionSet {
                primary: 0,
                selections: vec![selected],
            });
            let last = line_text(document, document.rope.len_lines() - 1);
            if !last
                .text
                .chars()
                .all(|character| matches!(character, ' ' | '\t'))
            {
                let end = document.rope.len_bytes();
                plan.edit(end..end, document.metadata.line_ending.as_str().into());
                plan.marks[0] = (
                    Mark::Tracked {
                        offset: selected.anchor,
                        sticks: true,
                    },
                    Mark::Tracked {
                        offset: selected.head,
                        sticks: true,
                    },
                );
            }
            plan
        }
    };
    Ok(plan.transaction(document, view))
}

fn commented_lines(
    document: &DocumentSnapshot,
    selection: &SelectionSet,
    command: LineCommand,
    context: LineCommandContext<'_>,
) -> Plan {
    let mut plan = Plan::new(selection);
    let Some(comments) = context
        .language
        .and_then(|language| language.rules.comments())
    else {
        return plan;
    };
    let size = tab_width(context.indent);
    let mut claimed = HashSet::new();
    for (index, selected) in selection.selections.iter().enumerate() {
        if command == LineCommand::ToggleBlockComment || comments.line.is_none() {
            if let Some((open, close)) = &comments.block {
                let mut effective = *selected;
                let bytes = ordered(selected);
                let first = document.rope.byte_to_line(bytes.start);
                let last = document.rope.byte_to_line(bytes.end);
                if command != LineCommand::ToggleBlockComment
                    && first < last
                    && bytes.end == document.rope.line_to_byte(last)
                {
                    let previous = line_text(document, last - 1);
                    let end = previous.start + previous.text.len();
                    effective = if selected.anchor <= selected.head {
                        Selection {
                            anchor: bytes.start,
                            head: end,
                        }
                    } else {
                        Selection {
                            anchor: end,
                            head: bytes.start,
                        }
                    };
                }
                block_comment(
                    &mut plan,
                    document,
                    index,
                    &effective,
                    open,
                    close,
                    command == LineCommand::ToggleBlockComment,
                );
            }
            continue;
        }
        let token = comments.line.as_deref().unwrap();
        let lines = selected_lines(document, selected);
        let contents = lines
            .clone()
            .map(|line| line_text(document, line))
            .collect::<Vec<_>>();
        let only_blank = contents
            .iter()
            .all(|line| leading_whitespace(&line.text).len() == line.text.len());
        let ignored = |line: &crate::editing::LineText| {
            leading_whitespace(&line.text).len() == line.text.len()
                && !(command == LineCommand::ToggleLineComment && only_blank)
        };
        let has_comment = |line: &crate::editing::LineText| {
            let whitespace = leading_whitespace(&line.text);
            line.text[whitespace.len()..]
                .get(..token.len())
                .is_some_and(|text| text.eq_ignore_ascii_case(token))
        };
        let remove = command == LineCommand::RemoveLineComment
            || (command == LineCommand::ToggleLineComment
                && !only_blank
                && contents
                    .iter()
                    .filter(|line| !ignored(line))
                    .all(has_comment));
        let min_column = contents
            .iter()
            .filter(|line| !ignored(line))
            .map(|line| visible_column(leading_whitespace(&line.text), size))
            .min()
            .unwrap_or(0)
            / size
            * size;
        for (relative, line) in contents.iter().enumerate() {
            let line_number = lines.start + relative;
            if ignored(line) || !claimed.insert(line_number) {
                continue;
            }
            let whitespace = leading_whitespace(&line.text);
            if remove {
                if !has_comment(line) {
                    continue;
                }
                let start = line.start + whitespace.len();
                let end = start + token.len();
                let end =
                    end + usize::from(line.text.as_bytes().get(end - line.start) == Some(&b' '));
                plan.edit(start..end, String::new());
            } else {
                let mut byte = 0;
                let mut column = 0;
                for character in whitespace.chars() {
                    if column >= min_column {
                        break;
                    }
                    let width = if character == '\t' {
                        size - column % size
                    } else {
                        1
                    };
                    if column + width > min_column {
                        break;
                    }
                    column += width;
                    byte += character.len_utf8();
                }
                let start = line.start + byte;
                plan.edit(start..start, format!("{token} "));
            }
        }
    }
    plan
}

fn block_comment(
    plan: &mut Plan,
    document: &DocumentSnapshot,
    index: usize,
    selected: &Selection,
    open: &str,
    close: &str,
    block: bool,
) {
    let bytes = ordered(selected);
    let first = line_text(document, document.rope.byte_to_line(bytes.start));
    let last = line_text(document, document.rope.byte_to_line(bytes.end));
    let start_column = bytes.start - first.start;
    let end_column = bytes.end - last.start;
    let mut opening = first
        .text
        .match_indices(open)
        .filter(|(byte, _)| *byte <= start_column + open.len())
        .map(|(byte, _)| byte)
        .last();
    let mut closing = last
        .text
        .match_indices(close)
        .find(|(byte, _)| *byte >= end_column.saturating_sub(close.len()))
        .map(|(byte, _)| byte);
    if !block && bytes.is_empty() && (opening.is_none() || closing.is_none()) {
        opening = first.text.find(open);
        closing = opening.and_then(|byte| {
            first.text[byte + open.len()..]
                .find(close)
                .map(|end| byte + open.len() + end)
        });
    }
    if let (Some(start), Some(end)) = (opening, closing) {
        let start_end = start + open.len();
        let valid = if first.start == last.start {
            start_end <= end && !first.text[start_end..end].contains(close)
        } else {
            !first.text[start_end..].contains(close) && !last.text[..end].contains(close)
        };
        if valid {
            let start_end =
                start_end + usize::from(first.text.as_bytes().get(start_end) == Some(&b' '));
            let close_start = end.saturating_sub(usize::from(
                end > 0 && last.text.as_bytes().get(end - 1) == Some(&b' '),
            ));
            let open_bytes = first.start + start..first.start + start_end;
            let close_bytes = last.start + close_start..last.start + end + close.len();
            if open_bytes.end >= close_bytes.start {
                if let Some(edit) = plan.edit(open_bytes.start..close_bytes.end, String::new()) {
                    if block {
                        plan.marks[index] = (Mark::AfterEdit(edit), Mark::AfterEdit(edit));
                    }
                }
            } else if let Some(left) = plan.edit(open_bytes, String::new())
                && let Some(right) = plan.edit(close_bytes, String::new())
                && block
            {
                plan.marks[index] = (
                    Mark::AfterEdit(left),
                    Mark::InEdit {
                        edit: right,
                        bytes: 0,
                    },
                );
            }
            return;
        }
    }
    let bytes = if block {
        bytes
    } else {
        first.start + leading_whitespace(&first.text).len()..last.start + last.text.len()
    };
    if bytes.is_empty() {
        if let Some(edit) = plan.edit(bytes, format!("{open}  {close}")) {
            let mark = Mark::InEdit {
                edit,
                bytes: open.len() + 1,
            };
            plan.marks[index] = (mark, mark);
        }
        return;
    }
    if let Some(left) = plan.edit(bytes.start..bytes.start, format!("{open} "))
        && let Some(right) = plan.edit(bytes.end..bytes.end, format!(" {close}"))
        && block
    {
        plan.marks[index] = (
            Mark::AfterEdit(left),
            Mark::InEdit {
                edit: right,
                bytes: 0,
            },
        );
    }
}

fn reordered_lines(
    document: &DocumentSnapshot,
    selection: &SelectionSet,
    command: LineCommand,
    compare: Option<&dyn Fn(&str, &str) -> Ordering>,
) -> Plan {
    let whole_document = selection.selections.len() == 1
        && position(document, selection.selections[0].anchor).0
            == position(document, selection.selections[0].head).0;
    let is_sort = matches!(
        command,
        LineCommand::SortLinesAscending | LineCommand::SortLinesDescending
    );
    let sorting_selection = if whole_document && is_sort {
        SelectionSet {
            primary: 0,
            selections: vec![Selection {
                anchor: 0,
                head: document.rope.len_bytes(),
            }],
        }
    } else {
        selection.clone()
    };
    let mut plan = Plan::new(&sorting_selection);
    let eol = document.metadata.line_ending.as_str();
    for (index, selected) in selection.selections.iter().enumerate() {
        let effective = if whole_document {
            Selection {
                anchor: 0,
                head: document.rope.len_bytes(),
            }
        } else {
            *selected
        };
        let mut lines = if command == LineCommand::RemoveDuplicateLines {
            let bytes = ordered(&effective);
            document.rope.byte_to_line(bytes.start)..document.rope.byte_to_line(bytes.end) + 1
        } else {
            selected_lines(document, &effective)
        };
        if is_sort && position(document, ordered(&effective).end).1 == 0 {
            lines.end = document.rope.byte_to_line(ordered(&effective).end);
        }
        if command == LineCommand::ReverseLines
            && lines.end == document.rope.len_lines()
            && line_text(document, lines.end - 1).text.is_empty()
        {
            lines.end -= 1;
        }
        if lines.is_empty() || (is_sort && lines.len() < 2) {
            if is_sort {
                return Plan::new(selection);
            }
            continue;
        }
        let contents = lines
            .clone()
            .map(|line| line_text(document, line).text)
            .collect::<Vec<_>>();
        let mut replacement = contents.clone();
        match command {
            LineCommand::SortLinesAscending | LineCommand::SortLinesDescending => {
                let Some(compare) = compare else {
                    return Plan::new(selection);
                };
                replacement.sort_by(|left, right| compare(left, right));
                if command == LineCommand::SortLinesDescending {
                    replacement.reverse();
                }
                if contents == replacement {
                    return Plan::new(selection);
                }
            }
            LineCommand::RemoveDuplicateLines => {
                let mut seen = HashSet::new();
                replacement.retain(|line| seen.insert(line.clone()));
            }
            LineCommand::ReverseLines => replacement.reverse(),
            _ => unreachable!(),
        }
        let bytes = line_text(document, lines.start).start
            ..line_text(document, lines.end - 1).start + contents.last().unwrap().len();
        let text = replacement.join(eol);
        let length = text.len();
        if let Some(edit) = plan.edit(bytes, text) {
            if command == LineCommand::RemoveDuplicateLines && !whole_document {
                plan.marks[index] = (
                    Mark::InEdit { edit, bytes: 0 },
                    Mark::InEdit {
                        edit,
                        bytes: length,
                    },
                );
            }
            if command == LineCommand::ReverseLines {
                let mark = |offset| {
                    let (line, column) = position(document, offset);
                    if line >= lines.end {
                        return Mark::Tracked {
                            offset,
                            sticks: false,
                        };
                    }
                    let target = lines.end - 1 - line;
                    let bytes = replacement[..target]
                        .iter()
                        .map(|line| line.len() + eol.len())
                        .sum::<usize>()
                        + byte_at_column(&replacement[target], column);
                    Mark::InEdit { edit, bytes }
                };
                plan.marks[index] = (mark(selected.anchor), mark(selected.head));
            }
        }
    }
    plan
}

fn selected_lines(document: &DocumentSnapshot, selection: &Selection) -> Range<usize> {
    let bytes = ordered(selection);
    let first = document.rope.byte_to_line(bytes.start);
    let mut last = document.rope.byte_to_line(bytes.end);
    if first < last && document.rope.line_to_byte(last) == bytes.end {
        last -= 1;
    }
    first..last + 1
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

fn position(document: &DocumentSnapshot, byte: usize) -> (usize, usize) {
    let line = document.rope.byte_to_line(byte);
    let content = line_text(document, line);
    let column = (byte - content.start).min(content.text.len());
    (line, content.text[..column].encode_utf16().count())
}

struct MovedSyntax<'a> {
    source: &'a DocumentSnapshot,
    syntax: &'a dyn LineSyntax,
    first: usize,
    source_lines: &'a [usize],
}

impl LineSyntax for MovedSyntax<'_> {
    fn tokens(
        &self,
        _document: &DocumentSnapshot,
        line: usize,
    ) -> Option<Vec<crate::syntax::Token>> {
        let mapped = line
            .checked_sub(self.first)
            .and_then(|index| self.source_lines.get(index))
            .copied()
            .unwrap_or(line);
        self.syntax.tokens(self.source, mapped)
    }

    fn kind_if_inserting(
        &self,
        _document: &DocumentSnapshot,
        _line: usize,
        _byte: usize,
        _character: char,
    ) -> TokenKind {
        TokenKind::Other
    }
}

fn enter_move_delta(
    document: &DocumentSnapshot,
    language: Language<'_>,
    moving: usize,
    above: Option<usize>,
    indent: IndentOptions,
) -> Option<isize> {
    let above = (0..=above?).rev().find(|line| {
        !line_text(document, *line)
            .text
            .chars()
            .all(is_js_whitespace)
    })?;
    let prefix = auto_indent::enter_prefix(document, language, above, indent)?;
    let moving_text = line_text(document, moving).text;
    if !moving_text
        .trim_start_matches(is_js_whitespace)
        .contains(prefix.trim_start_matches(is_js_whitespace))
    {
        return None;
    }
    let size = tab_width(indent);
    let mut desired = visible_column(leading_whitespace(&prefix), size);
    if language
        .rules
        .indent_metadata(&moving_text)
        .is_some_and(|metadata| metadata.decreases)
    {
        desired = crate::editing::previous_tab_stop(desired, size);
    }
    Some(desired as isize - visible_column(leading_whitespace(&moving_text), size) as isize)
}

fn moved_lines(
    document: &DocumentSnapshot,
    selection: &SelectionSet,
    down: bool,
    context: LineCommandContext<'_>,
) -> Plan {
    let mut plan = Plan::new(selection);
    let eol = document.metadata.line_ending.as_str();
    for (index, selected) in selection.selections.iter().enumerate() {
        let original = ordered(selected);
        let original_first = document.rope.byte_to_line(original.start);
        let original_last = document.rope.byte_to_line(original.end);
        if (!down && original_first == 0)
            || (down && original_last + 1 == document.rope.len_lines())
        {
            continue;
        }
        let lines = selected_lines(document, selected);
        let first = if down { lines.start } else { lines.start - 1 };
        let last = if down { lines.end } else { lines.end - 1 };
        let mut contents = (first..=last)
            .map(|line| line_text(document, line).text)
            .collect::<Vec<_>>();
        let mut source_lines = (first..=last).collect::<Vec<_>>();
        if down {
            contents.rotate_right(1);
            source_lines.rotate_right(1);
        } else {
            contents.rotate_left(1);
            source_lines.rotate_left(1);
        }
        let bytes = line_text(document, first).start
            ..line_text(document, last).start + line_text(document, last).text.len();
        if let Some(language) = context.language
            && language.rules.indent_metadata("").is_some()
            && language.syntax.tokens(document, lines.start).is_some()
            && !(lines.len() == 1 && line_text(document, lines.start).text.is_empty())
        {
            let mapped = MovedSyntax {
                source: document,
                syntax: language.syntax,
                first,
                source_lines: &source_lines,
            };
            let virtual_language = Language {
                rules: language.rules,
                syntax: &mapped,
            };
            let make_virtual = |contents: &[String]| {
                let mut snapshot = document.clone();
                let chars =
                    snapshot.rope.byte_to_char(bytes.start)..snapshot.rope.byte_to_char(bytes.end);
                snapshot.rope.remove(chars.clone());
                snapshot.rope.insert(chars.start, &contents.join(eol));
                snapshot
            };
            let size = tab_width(context.indent);
            if down {
                let snapshot = make_virtual(&contents);
                let old = visible_column(leading_whitespace(&contents[0]), size);
                let desired = enter_move_delta(
                    document,
                    language,
                    last,
                    lines.start.checked_sub(1),
                    context.indent,
                )
                .map(|delta| old.saturating_add_signed(delta))
                .or_else(|| {
                    auto_indent::good_indent(&snapshot, virtual_language, first, context.indent)
                        .map(|prefix| visible_column(&prefix, size))
                });
                if let Some(desired) = desired {
                    contents[0] = format!(
                        "{}{}",
                        indentation(desired, context.indent),
                        contents[0].trim_start_matches(is_js_whitespace)
                    );
                }
            }
            let block_first = usize::from(down);
            let snapshot = make_virtual(&contents);
            let old = visible_column(leading_whitespace(&contents[block_first]), size);
            let above = if down {
                Some(last)
            } else {
                lines.start.checked_sub(2)
            };
            let delta = enter_move_delta(document, language, lines.start, above, context.indent)
                .or_else(|| {
                    auto_indent::good_indent(
                        &snapshot,
                        virtual_language,
                        first + block_first,
                        context.indent,
                    )
                    .map(|prefix| visible_column(&prefix, size) as isize - old as isize)
                });
            if let Some(delta) = delta {
                for content in &mut contents[block_first..block_first + lines.len()] {
                    let existing = leading_whitespace(content);
                    let desired = visible_column(existing, size).saturating_add_signed(delta);
                    *content = format!(
                        "{}{}",
                        indentation(desired, context.indent),
                        &content[existing.len()..]
                    );
                }
            }
        }
        let replacement = contents.join(eol);
        let Some(edit) = plan.edit(bytes, replacement) else {
            continue;
        };
        let mark = |offset| {
            let (line, mut column) = position(document, offset);
            let shifted = if down { line + 1 } else { line - 1 };
            let relative_line = shifted - first;
            if line >= lines.start && line < lines.end {
                let original = line_text(document, line).text;
                let old = leading_whitespace(&original).len();
                let new = leading_whitespace(&contents[relative_line]).len();
                column = if column >= old {
                    column - old + new
                } else {
                    column.min(new)
                };
            }
            let preceding = contents
                .iter()
                .take(relative_line)
                .map(|content| content.len() + eol.len())
                .sum::<usize>();
            let column = contents
                .get(relative_line)
                .map_or(0, |content| byte_at_column(content, column));
            Mark::InEdit {
                edit,
                bytes: preceding + column,
            }
        };
        plan.marks[index] = (mark(selected.anchor), mark(selected.head));
    }
    plan
}

fn copied_lines(
    document: &DocumentSnapshot,
    selection: &SelectionSet,
    command: LineCommand,
) -> Plan {
    let mut plan = Plan::new(selection);
    let eol = document.metadata.line_ending.as_str();
    let down = command != LineCommand::CopyLinesUp;
    let mut copied = Vec::<Range<usize>>::new();
    for (index, selected) in selection.selections.iter().enumerate() {
        let bytes = ordered(selected);
        if command == LineCommand::DuplicateSelection && !bytes.is_empty() {
            let text = document.rope.byte_slice(bytes.clone()).to_string();
            let length = text.len();
            if let Some(edit) = plan.edit(bytes.end..bytes.end, text) {
                plan.marks[index] = (
                    Mark::InEdit { edit, bytes: 0 },
                    Mark::InEdit {
                        edit,
                        bytes: length,
                    },
                );
            }
            continue;
        }
        let lines = selected_lines(document, selected);
        if copied
            .iter()
            .any(|previous| previous.start < lines.end && lines.start < previous.end)
        {
            continue;
        }
        copied.push(lines.clone());
        let text = lines
            .clone()
            .map(|line| line_text(document, line).text)
            .collect::<Vec<_>>()
            .join(eol);
        let location = if down {
            line_text(document, lines.start).start
        } else {
            let last = line_text(document, lines.end - 1);
            last.start + last.text.len()
        };
        let text = if down {
            format!("{text}{eol}")
        } else {
            format!("{eol}{text}")
        };
        if plan.edit(location..location, text).is_none() {
            continue;
        }
        plan.marks[index] = (
            Mark::Tracked {
                offset: selected.anchor,
                sticks: !down,
            },
            Mark::Tracked {
                offset: selected.head,
                sticks: !down,
            },
        );
    }
    plan
}

fn deleted_lines(document: &DocumentSnapshot, selection: &SelectionSet) -> Plan {
    let mut groups = selection
        .selections
        .iter()
        .map(|selected| {
            let lines = selected_lines(document, selected);
            let (_, column) = position(document, selected.head);
            (lines, column)
        })
        .collect::<Vec<_>>();
    groups.sort_by_key(|(lines, _)| (lines.start, lines.end));
    let mut merged = Vec::<(Range<usize>, usize)>::new();
    for (lines, column) in groups {
        if let Some((previous, _)) = merged.last_mut()
            && previous.end >= lines.start
        {
            previous.end = previous.end.max(lines.end);
            continue;
        }
        merged.push((lines, column));
    }
    let cursors = merged
        .iter()
        .map(|(lines, column)| {
            let target = if lines.end < document.rope.len_lines() {
                lines.end
            } else {
                lines.start.saturating_sub(1)
            };
            let content = line_text(document, target);
            let offset = content.start + byte_at_column(&content.text, *column);
            Selection {
                anchor: offset,
                head: offset,
            }
        })
        .collect();
    let mut plan = Plan::new(&SelectionSet {
        primary: 0,
        selections: cursors,
    });
    for (lines, _) in merged {
        let first = line_text(document, lines.start);
        let last = line_text(document, lines.end - 1);
        let bytes = if lines.end < document.rope.len_lines() {
            first.start..document.rope.line_to_byte(lines.end)
        } else if lines.start > 0 {
            let preceding = line_text(document, lines.start - 1);
            preceding.start + preceding.text.len()..last.start + last.text.len()
        } else {
            first.start..last.start + last.text.len()
        };
        plan.edit(bytes, String::new());
    }
    plan
}

fn inserted_lines(
    document: &DocumentSnapshot,
    selection: &SelectionSet,
    command: LineCommand,
    indent: IndentOptions,
    language: Option<Language<'_>>,
) -> Plan {
    let mut plan = Plan::new(selection);
    let eol = document.metadata.line_ending.as_str();
    for (index, selected) in selection.selections.iter().enumerate() {
        let line = document.rope.byte_to_line(selected.head);
        if command == LineCommand::InsertLineBefore && line == 0 {
            if let Some(edit) = plan.edit(0..0, eol.into()) {
                plan.marks[index] = (
                    Mark::InEdit { edit, bytes: 0 },
                    Mark::InEdit { edit, bytes: 0 },
                );
            }
            continue;
        }
        let above = if command == LineCommand::InsertLineBefore {
            line - 1
        } else {
            line
        };
        let content = line_text(document, above);
        let location = content.start + content.text.len();
        let entered = language.map(|language| {
            auto_indent::line_break(document, location..location, language, indent, eol)
        });
        let (bytes, text, caret_before_end) = match entered {
            Some(entered) => (entered.bytes, entered.text, entered.caret_before_end),
            None => {
                let columns = visible_column(leading_whitespace(&content.text), tab_width(indent));
                (
                    location..location,
                    format!("{eol}{}", indentation(columns, indent)),
                    0,
                )
            }
        };
        if let Some(edit) = plan.edit(bytes, text) {
            let mark = Mark::BeforeEditEnd {
                edit,
                bytes: caret_before_end,
            };
            plan.marks[index] = (mark, mark);
        }
    }
    plan
}

fn joined_lines(document: &DocumentSnapshot, selection: &SelectionSet) -> Plan {
    let mut selections = selection.selections.clone();
    selections.sort_by_key(|selected| ordered(selected).start);
    let mut merged = Vec::<Selection>::new();
    for selected in selections {
        let range = ordered(&selected);
        if let Some(previous) = merged.last_mut() {
            let previous_range = ordered(previous);
            let previous_end = document.rope.byte_to_line(previous_range.end);
            let next_start = document.rope.byte_to_line(range.start);
            let empty = previous_range.is_empty();
            if empty && previous_end == next_start {
                *previous = selected;
                continue;
            }
            if next_start <= previous_end + usize::from(empty) {
                *previous = Selection {
                    anchor: previous_range.start,
                    head: range.end,
                };
                continue;
            }
        }
        merged.push(selected);
    }
    let primary = ordered(&selection.selections[selection.primary]);
    let primary = merged
        .iter()
        .position(|selected| {
            let range = ordered(selected);
            let first = document.rope.byte_to_line(range.start);
            let last = document.rope.byte_to_line(range.end);
            first <= document.rope.byte_to_line(primary.start)
                && document.rope.byte_to_line(primary.end) <= last + usize::from(range.is_empty())
        })
        .unwrap_or(0);
    let selection = SelectionSet {
        primary,
        selections: merged,
    };
    let mut plan = Plan::new(&selection);
    for (index, selected) in selection.selections.iter().enumerate() {
        let range = ordered(selected);
        let first_line = document.rope.byte_to_line(range.start);
        let selected_last = document.rope.byte_to_line(range.end);
        let last_line = if first_line == selected_last {
            (first_line + 1).min(document.rope.len_lines() - 1)
        } else {
            selected_last
        };
        if first_line == last_line {
            continue;
        }
        let first = line_text(document, first_line);
        let last = line_text(document, last_line);
        let mut text = first.text.clone();
        let mut last_added = 0;
        for line in first_line + 1..=last_line {
            let content = line_text(document, line).text;
            let leading = leading_whitespace(&content).len();
            if leading == content.len() {
                last_added = 0;
                continue;
            }
            let mut insert_space = !text.is_empty();
            if insert_space && (text.ends_with(' ') || text.ends_with('\t')) {
                text.truncate(text.trim_end_matches(is_js_whitespace).len());
                text.push(' ');
                insert_space = false;
            }
            if insert_space {
                text.push(' ');
            }
            text.push_str(&content[leading..]);
            last_added = content.len() - leading + usize::from(insert_space);
        }
        let text_length = text.len();
        let (start_column, end_column) = (
            range.start - first.start,
            range.end - line_text(document, selected_last).start,
        );
        let (anchor, head) = if range.is_empty() {
            let caret = text_length - last_added;
            (caret, caret)
        } else if first_line == selected_last {
            let first_column = first.text[..start_column].encode_utf16().count();
            let last_column = first.text[..end_column].encode_utf16().count();
            (
                byte_at_column(&text, first_column),
                byte_at_column(&text, last_column),
            )
        } else {
            let anchor_column = first.text[..start_column].encode_utf16().count();
            let remaining = last.text[end_column..].encode_utf16().count();
            let head_column = text.encode_utf16().count().saturating_sub(remaining);
            (
                byte_at_column(&text, anchor_column),
                byte_at_column(&text, head_column),
            )
        };
        if let Some(edit) = plan.edit(first.start..last.start + last.text.len(), text) {
            plan.marks[index] = (
                Mark::InEdit {
                    edit,
                    bytes: anchor,
                },
                Mark::InEdit { edit, bytes: head },
            );
        }
    }
    plan
}

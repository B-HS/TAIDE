use std::ops::Range;

use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};

use crate::document::{DocumentSnapshot, Edit, EditorError, UndoGroup, byte_to_char};
use crate::store::{EditorStore, Transaction};
use crate::view::{Selection, SelectionSet, ViewId};

#[derive(Clone, Copy)]
pub enum Motion {
    Left,
    Right,
    Up,
    Down,
    LineStart,
    LineEnd,
    DocumentStart,
    DocumentEnd,
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
    let rope = &document.rope;
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
    let current = store
        .views()
        .get(view)
        .ok_or(EditorError::NotFound)?
        .clone();
    let document = store.documents().snapshot(current.document)?;
    let selections = current
        .selection
        .selections
        .iter()
        .map(|selection| {
            let head = selection.head;
            let line = document.rope.byte_to_line(head);
            let range = line_content_range(&document, line);
            let scalar = document.rope.byte_to_char(head);
            let column = scalar - document.rope.line_to_char(line);
            let target_line = match motion {
                Motion::Up => line.saturating_sub(1),
                Motion::Down => (line + 1).min(document.rope.len_lines() - 1),
                _ => line,
            };
            let target = match motion {
                Motion::Left if !extend && selection.anchor != head => selection.anchor.min(head),
                Motion::Right if !extend && selection.anchor != head => selection.anchor.max(head),
                Motion::Left => grapheme_boundary(&document, head, false)?,
                Motion::Right => grapheme_boundary(&document, head, true)?,
                Motion::LineStart => range.start,
                Motion::LineEnd => range.end,
                Motion::DocumentStart => 0,
                Motion::DocumentEnd => document.rope.len_bytes(),
                Motion::Up | Motion::Down => {
                    let target_range = line_content_range(&document, target_line);
                    let end_scalar = document.rope.byte_to_char(target_range.end);
                    let target_scalar =
                        (document.rope.line_to_char(target_line) + column).min(end_scalar);
                    document.rope.char_to_byte(target_scalar)
                }
            };
            Ok(Selection {
                anchor: if extend { selection.anchor } else { target },
                head: target,
            })
        })
        .collect::<Result<Vec<_>, EditorError>>()?;
    store.set_composition(view, None)?;
    store.set_view_state(
        view,
        SelectionSet {
            primary: current.selection.primary,
            selections,
        },
        current.scroll,
        current.folds,
    )
}

pub fn select_all(store: &mut EditorStore, view: ViewId) -> Result<(), EditorError> {
    let current = store
        .views()
        .get(view)
        .ok_or(EditorError::NotFound)?
        .clone();
    let document = store.documents().snapshot(current.document)?;
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
    let current = store
        .views()
        .get(view)
        .ok_or(EditorError::NotFound)?
        .clone();
    let document = store.documents().snapshot(current.document)?;
    let mut ranges = current
        .selection
        .selections
        .iter()
        .map(|selection| {
            let mut range =
                selection.anchor.min(selection.head)..selection.anchor.max(selection.head);
            if range.is_empty()
                && let Some(forward) = delete_forward
            {
                let boundary = grapheme_boundary(&document, selection.head, forward)?;
                range = boundary.min(selection.head)..boundary.max(selection.head);
            }
            Ok(range)
        })
        .collect::<Result<Vec<_>, EditorError>>()?;
    let primary_range = ranges[current.selection.primary].clone();
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
        return Ok(None);
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
    Ok(Some(Transaction {
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
    }))
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

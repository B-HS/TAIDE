use std::ops::Range;

use crate::document::{DocumentId, Edit, EditorError, UndoGroup, byte_to_char};
use crate::snippet_expansion::{Expansion, PlaceholderSpan};
use crate::snippet_syntax::{Index, Marker, ParseLimits};
use crate::store::{EditorStore, Transaction};
use crate::view::{Selection, SelectionSet, ViewId, ViewState};

pub struct PreparedSnippet {
    pub replace: Range<usize>,
    pub expansion: Expansion,
}

pub struct InsertedSnippet {
    pub cursor_index: usize,
    pub bytes: Range<usize>,
    pub markers: Vec<Marker>,
    pub placeholders: Vec<PlaceholderSpan>,
    pub line_leading_whitespace: String,
}

pub struct Insertion {
    pub document: DocumentId,
    pub view: ViewId,
    pub revision: u64,
    pub snippets: Vec<InsertedSnippet>,
    pub selection: SelectionSet,
    pub primary_cursor: usize,
}

pub fn insert(
    store: &mut EditorStore,
    expected_view: &ViewState,
    revision: u64,
    snippets: Vec<PreparedSnippet>,
    limits: ParseLimits,
) -> Result<Insertion, EditorError> {
    let view = expected_view.id;
    let owner = store.views().get(view).ok_or(EditorError::NotFound)?;
    if owner.document != expected_view.document {
        return Err(EditorError::InvalidIdentity);
    }
    if owner.selection != expected_view.selection {
        return Err(EditorError::Refused);
    }
    let document = store.documents().snapshot(owner.document)?;
    if document.revision != revision {
        return Err(EditorError::StaleRevision);
    }
    if document.metadata.read_only {
        return Err(EditorError::ReadOnly);
    }
    if owner.composition.is_some() {
        return Err(EditorError::Refused);
    }
    let selection_before = owner.selection.clone();
    if snippets.len() != selection_before.selections.len() || snippets.is_empty() {
        return Err(EditorError::InvalidBoundary);
    }
    let mut sorted = snippets.into_iter().enumerate().collect::<Vec<_>>();
    sorted.sort_by_key(|(_, snippet)| (snippet.replace.start, snippet.replace.end));
    let mut text_bytes = 0usize;
    let mut placeholder_count = 0usize;
    let mut previous_end = 0;
    for (_, snippet) in &sorted {
        let range = &snippet.replace;
        if range.start > range.end {
            return Err(EditorError::InvalidBoundary);
        }
        byte_to_char(&document.rope, range.start)?;
        byte_to_char(&document.rope, range.end)?;
        if range.start < previous_end {
            return Err(EditorError::Overlap);
        }
        previous_end = range.end;
        text_bytes = text_bytes
            .checked_add(snippet.expansion.text.len())
            .ok_or(EditorError::Capacity)?;
        placeholder_count = placeholder_count
            .checked_add(snippet.expansion.placeholders.len())
            .ok_or(EditorError::Capacity)?;
        if text_bytes > limits.max_bytes || placeholder_count > limits.max_markers {
            return Err(EditorError::Capacity);
        }
        for (position, span) in snippet.expansion.placeholders.iter().enumerate() {
            if span.bytes.start > span.bytes.end
                || snippet.expansion.text.get(span.bytes.clone()).is_none()
                || span.marker_path.is_empty()
                || span.marker_path.len().saturating_sub(1) > limits.max_nesting
            {
                return Err(EditorError::InvalidBoundary);
            }
            let mut nodes = snippet.expansion.markers.as_slice();
            let mut marker = None;
            for index in &span.marker_path {
                let node = nodes.get(*index).ok_or(EditorError::InvalidBoundary)?;
                marker = Some(node);
                nodes = match node {
                    Marker::Text(_) => &[],
                    Marker::Variable { children, .. } | Marker::Placeholder { children, .. } => {
                        children
                    }
                };
            }
            if !matches!(marker, Some(Marker::Placeholder { index, .. }) if *index == span.index) {
                return Err(EditorError::InvalidBoundary);
            }
            for parent in &span.enclosing {
                if *parent >= position {
                    return Err(EditorError::InvalidBoundary);
                }
                let parent = &snippet.expansion.placeholders[*parent];
                if parent.bytes.start > span.bytes.start
                    || parent.bytes.end < span.bytes.end
                    || parent.marker_path.len() >= span.marker_path.len()
                    || !span.marker_path.starts_with(&parent.marker_path)
                {
                    return Err(EditorError::InvalidBoundary);
                }
            }
        }
    }
    let mut removed = 0usize;
    let mut inserted = 0usize;
    let mut context_bytes = 0usize;
    let mut edits = Vec::new();
    let mut placements = Vec::new();
    for (cursor_index, snippet) in sorted {
        let line = document.rope.byte_to_line(snippet.replace.start);
        let line_leading_whitespace = document
            .rope
            .byte_slice(document.rope.line_to_byte(line)..snippet.replace.start)
            .chars()
            .take_while(|character| matches!(character, ' ' | '\t'))
            .take(limits.max_bytes.saturating_add(1))
            .collect::<String>();
        context_bytes = context_bytes
            .checked_add(line_leading_whitespace.len())
            .ok_or(EditorError::Capacity)?;
        if context_bytes > limits.max_bytes {
            return Err(EditorError::Capacity);
        }
        let offset = snippet
            .replace
            .start
            .checked_sub(removed)
            .and_then(|start| start.checked_add(inserted))
            .ok_or(EditorError::Capacity)?;
        let end = offset
            .checked_add(snippet.expansion.text.len())
            .ok_or(EditorError::Capacity)?;
        let mut placeholders = snippet.expansion.placeholders;
        for placeholder in &mut placeholders {
            placeholder.bytes.start = placeholder
                .bytes
                .start
                .checked_add(offset)
                .ok_or(EditorError::Capacity)?;
            placeholder.bytes.end = placeholder
                .bytes
                .end
                .checked_add(offset)
                .ok_or(EditorError::Capacity)?;
        }
        removed = removed
            .checked_add(snippet.replace.len())
            .ok_or(EditorError::Capacity)?;
        inserted = inserted
            .checked_add(snippet.expansion.text.len())
            .ok_or(EditorError::Capacity)?;
        edits.push(Edit {
            bytes: snippet.replace,
            text: snippet.expansion.text,
        });
        placements.push(InsertedSnippet {
            cursor_index,
            bytes: offset..end,
            markers: snippet.expansion.markers,
            placeholders,
            line_leading_whitespace,
        });
    }
    placements.sort_by_key(|snippet| snippet.cursor_index);
    let mut selections = Vec::new();
    let mut primary = 0;
    for snippet in &placements {
        if snippet.cursor_index == selection_before.primary {
            primary = selections.len();
        }
        let first = snippet
            .placeholders
            .iter()
            .map(|span| span.index)
            .filter(|index| *index != Index::FINAL)
            .min()
            .or_else(|| snippet.placeholders.first().map(|span| span.index));
        let first = first.map(|index| {
            snippet
                .placeholders
                .iter()
                .filter(move |span| span.index == index)
        });
        if let Some(first) = first {
            selections.extend(first.map(|span| Selection {
                anchor: span.bytes.start,
                head: span.bytes.end,
            }));
        } else {
            selections.push(Selection {
                anchor: snippet.bytes.end,
                head: snippet.bytes.end,
            });
        }
    }
    let selection = SelectionSet {
        primary,
        selections,
    };
    edits.retain(|edit| !edit.bytes.is_empty() || !edit.text.is_empty());
    let revision = store.apply_separate(
        document.id,
        Transaction {
            revision,
            edits,
            group: UndoGroup(revision),
            origin: Some(view),
            selection_after: Some(selection.clone()),
        },
    )?;
    if store
        .views()
        .get(view)
        .is_some_and(|view| view.selection != selection)
    {
        let state = store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .clone();
        store.set_view_state(view, selection.clone(), state.scroll, state.folds)?;
    }
    Ok(Insertion {
        document: document.id,
        view,
        revision,
        snippets: placements,
        selection,
        primary_cursor: selection_before.primary,
    })
}

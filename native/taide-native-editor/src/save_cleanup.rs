use std::collections::BTreeMap;

use crate::document::{DocumentId, Edit, EditorError, UndoGroup};
use crate::editing::line_content_range;
use crate::store::{EditorStore, Transaction};
use crate::syntax::TokenKind;
use crate::view::ViewId;

#[derive(Clone, Copy)]
pub struct CleanupFlags {
    pub trim_trailing_whitespace: bool,
    pub insert_final_newline: bool,
}

pub struct CleanupOutput {
    pub changed: bool,
    pub errors: Vec<EditorError>,
}

pub fn run(
    store: &mut EditorStore,
    document: DocumentId,
    view: Option<ViewId>,
    flags: CleanupFlags,
    auto_save: bool,
) -> Result<CleanupOutput, EditorError> {
    let snapshot = store.documents().snapshot(document)?;
    let config = snapshot.metadata.editor_config;
    let mut output = CleanupOutput {
        changed: false,
        errors: Vec::new(),
    };
    let Some(view) = view else {
        return Ok(output);
    };
    if store
        .views()
        .get(view)
        .is_none_or(|view| view.document != document)
    {
        return Err(EditorError::InvalidIdentity);
    }
    if config
        .trim_trailing_whitespace
        .unwrap_or(flags.trim_trailing_whitespace)
    {
        store.break_undo_group(document)?;
        match trim_trailing_whitespace(store, document, view, auto_save) {
            Ok(changed) => output.changed |= changed,
            Err(error) => output.errors.push(error),
        }
        store.break_undo_group(document)?;
    }
    if config
        .insert_final_newline
        .unwrap_or(flags.insert_final_newline)
    {
        store.break_undo_group(document)?;
        match insert_final_newline(store, document, view) {
            Ok(changed) => output.changed |= changed,
            Err(error) => output.errors.push(error),
        }
        store.break_undo_group(document)?;
    }
    Ok(output)
}

pub(crate) fn trim_trailing_whitespace(
    store: &mut EditorStore,
    document: DocumentId,
    view: ViewId,
    auto_save: bool,
) -> Result<bool, EditorError> {
    let snapshot = store.documents().snapshot(document)?;
    let mut cursors = BTreeMap::<usize, usize>::new();
    if auto_save {
        for selection in &store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .selection
            .selections
        {
            let line = snapshot.rope.byte_to_line(selection.head);
            cursors
                .entry(line)
                .and_modify(|head| *head = (*head).max(selection.head))
                .or_insert(selection.head);
        }
    }
    let syntax = store.syntax(document)?;
    let mut edits = Vec::new();
    for line in 0..snapshot.rope.len_lines() {
        let range = line_content_range(&snapshot, line);
        let mut trim_start = range.end;
        while trim_start > range.start {
            match snapshot.rope.byte(trim_start - 1) {
                b' ' | b'\t' => trim_start -= 1,
                _ => break,
            }
        }
        if trim_start == range.end {
            continue;
        }
        if snapshot.metadata.language_id != "plaintext" {
            let kind = syntax.and_then(|syntax| {
                syntax.token_at(line, (trim_start - range.start + 1).min(range.len()))
            });
            if !matches!(kind, Some(TokenKind::Other | TokenKind::Comment)) {
                continue;
            }
        }
        if let Some(cursor) = cursors.get(&line) {
            trim_start = trim_start.max(*cursor);
        }
        if trim_start < range.end {
            edits.push(Edit {
                bytes: trim_start..range.end,
                text: String::new(),
            });
        }
    }
    if edits.is_empty() {
        return Ok(false);
    }
    store.apply(
        document,
        Transaction {
            revision: snapshot.revision,
            edits,
            group: UndoGroup(snapshot.revision),
            origin: Some(view),
            selection_after: None,
        },
    )?;
    Ok(true)
}

pub(crate) fn insert_final_newline(
    store: &mut EditorStore,
    document: DocumentId,
    view: ViewId,
) -> Result<bool, EditorError> {
    let snapshot = store.documents().snapshot(document)?;
    let range = line_content_range(&snapshot, snapshot.rope.len_lines() - 1);
    let slice = snapshot.rope.byte_slice(range);
    if slice.chars().all(|scalar| matches!(scalar, ' ' | '\t')) {
        return Ok(false);
    }
    let end = snapshot.rope.len_bytes();
    let selection = store
        .views()
        .get(view)
        .ok_or(EditorError::NotFound)?
        .selection
        .clone();
    store.apply(
        document,
        Transaction {
            revision: snapshot.revision,
            edits: vec![Edit {
                bytes: end..end,
                text: snapshot.metadata.line_ending.as_str().into(),
            }],
            group: UndoGroup(snapshot.revision),
            origin: Some(view),
            selection_after: Some(selection),
        },
    )?;
    Ok(true)
}

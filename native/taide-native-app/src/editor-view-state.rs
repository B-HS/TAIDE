use std::collections::HashMap;

use serde::Deserialize;
use taide_native_editor::document::{DocumentId, DocumentSnapshot, EditorError};
use taide_native_editor::editing::line_content_range;
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{ScrollPosition, Selection, SelectionSet, ViewId};

const MAX_RESTORED_CURSORS: usize = 10_000;

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelPosition {
    line_number: Option<u64>,
    column: Option<u64>,
}

impl ModelPosition {
    fn byte(&self, document: &DocumentSnapshot) -> usize {
        let last_line = document.rope.len_lines().saturating_sub(1);
        let line = usize::try_from(self.line_number.unwrap_or(1).saturating_sub(1))
            .unwrap_or(last_line)
            .min(last_line);
        let range = line_content_range(document, line);
        let text = document.rope.byte_slice(range.clone());
        let requested = usize::try_from(self.column.unwrap_or(1).saturating_sub(1))
            .unwrap_or(usize::MAX)
            .min(text.len_utf16_cu());
        range.start + text.char_to_byte(text.utf16_cu_to_char(requested))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CursorState {
    selection_start: Option<ModelPosition>,
    position: Option<ModelPosition>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScrollState {
    scroll_left: Option<f32>,
    first_position: Option<ModelPosition>,
    first_position_delta_top: Option<f32>,
    scroll_top_without_view_zones: Option<f32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedViewState {
    cursor_state: Vec<CursorState>,
    view_state: ScrollState,
}

impl PersistedViewState {
    fn parse(saved: &str) -> Option<Self> {
        let value = serde_json::from_str::<serde_json::Value>(saved).ok()?;
        let fields = value.as_object()?;
        let cursors = fields.get("cursorState")?.as_array()?;
        let scroll = fields.get("viewState")?.as_object()?;
        if cursors.iter().any(|cursor| {
            !cursor.is_object()
                || ["position", "selectionStart"].iter().any(|field| {
                    cursor
                        .get(field)
                        .is_some_and(|position| !position.is_null() && !position.is_object())
                })
        }) || scroll
            .get("firstPosition")
            .is_some_and(|position| !position.is_null() && !position.is_object())
        {
            return None;
        }
        serde_json::from_value(value).ok()
    }
}

#[derive(Default)]
pub(crate) struct State {
    restored: HashMap<ViewId, DocumentId>,
}

impl State {
    pub(crate) fn begin_frame(&mut self, store: &EditorStore) {
        self.restored.retain(|view, document| {
            store
                .views()
                .get(*view)
                .is_some_and(|current| current.document == *document)
        });
    }

    pub(crate) fn restore(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
        saved: Option<&str>,
    ) -> Result<(), EditorError> {
        let current = store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .clone();
        if self.restored.get(&view) == Some(&current.document) {
            return Ok(());
        }
        let document = store.documents().snapshot(current.document)?;
        self.restored.insert(view, current.document);
        let Some(saved) = saved.and_then(PersistedViewState::parse) else {
            return Ok(());
        };
        let selections = saved
            .cursor_state
            .into_iter()
            .take(MAX_RESTORED_CURSORS)
            .map(|cursor| {
                let position = cursor.position.unwrap_or_default();
                let head = position.byte(&document);
                let anchor = cursor.selection_start.as_ref().map_or(head, |start| {
                    ModelPosition {
                        line_number: start.line_number.or(position.line_number),
                        column: start.column.or(position.column),
                    }
                    .byte(&document)
                });
                Selection { anchor, head }
            })
            .collect::<Vec<_>>();
        let selection = if selections.is_empty() {
            current.selection
        } else {
            SelectionSet {
                primary: 0,
                selections,
            }
        };
        let scroll = ScrollPosition {
            x: saved
                .view_state
                .scroll_left
                .filter(|value| value.is_finite())
                .unwrap_or(0.0)
                .max(0.0),
            y: saved
                .view_state
                .scroll_top_without_view_zones
                .filter(|value| value.is_finite())
                .unwrap_or(0.0)
                .max(0.0),
        };
        store.set_view_state(view, selection, scroll, current.folds)?;
        let position = saved
            .view_state
            .first_position
            .map(|position| position.byte(&document));
        let delta = saved
            .view_state
            .first_position_delta_top
            .filter(|value| value.is_finite())
            .unwrap_or(0.0);
        store.request_scroll_restore(view, position, delta)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "editor-view-state-tests.rs"]
mod tests;

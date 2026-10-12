use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use taide_model::error::AppResult;
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::layout::{ProjectLayout, TabKind};
use taide_native_editor::document::{DocumentId, DocumentSnapshot, EditorError};
use taide_native_editor::editing::line_content_range;
use taide_native_editor::lsp::byte_to_position;
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{ScrollPosition, Selection, SelectionSet, ViewId, ViewKey};
use taide_native_ui::editor_geometry::EditorGeometry;
use taide_runtime::{AppServices, AppState, EventSink};

const MAX_RESTORED_CURSORS: usize = 10_000;

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModelPosition {
    line_number: Option<u64>,
    column: Option<u64>,
}

impl ModelPosition {
    fn from_byte(document: &DocumentSnapshot, byte: usize) -> Result<Self, EditorError> {
        let position = byte_to_position(document, byte)?;
        Ok(Self {
            line_number: Some(u64::from(position.line) + 1),
            column: Some(u64::from(position.character) + 1),
        })
    }

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

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CursorState {
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    in_selection_mode: Option<bool>,
    selection_start: Option<ModelPosition>,
    position: Option<ModelPosition>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScrollState {
    scroll_left: Option<f32>,
    first_position: Option<ModelPosition>,
    first_position_delta_top: Option<f32>,
    scroll_top_without_view_zones: Option<f32>,
}

#[derive(Deserialize, Serialize)]
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

struct ScrollAnchor {
    revision: u64,
    scroll: ScrollPosition,
    byte: usize,
    delta_top: f32,
}

struct Observed {
    document: DocumentId,
    kind: TabKind,
    anchor: Option<ScrollAnchor>,
}

#[derive(PartialEq, Eq, Hash)]
struct ModelViewKey {
    window: String,
    pane: PaneId,
    document: DocumentId,
}

struct CachedModelView {
    revision: u64,
    saved: PersistedViewState,
}

pub(crate) struct Captured {
    project: ProjectId,
    tab: TabId,
    kind: TabKind,
    fields: serde_json::Map<String, serde_json::Value>,
}

pub(crate) async fn apply(services: &AppServices, captured: &[Captured]) -> AppResult<()> {
    apply_to(&services.state, services.events.as_ref(), captured).await
}

pub(crate) async fn apply_to(
    state: &AppState,
    events: &dyn EventSink,
    captured: &[Captured],
) -> AppResult<()> {
    if captured.is_empty() {
        return Ok(());
    }
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    for (project, layout) in &mut layouts {
        if !state.projects.read().contains_key(project) {
            continue;
        }
        let mut changed = false;
        for current in captured
            .iter()
            .filter(|current| &current.project == project)
        {
            let Some(tab) = taide_layout::service::all_roots(layout)
                .flat_map(crate::tabs::tabs_in)
                .find(|tab| tab.id == current.tab && tab.kind == current.kind)
            else {
                continue;
            };
            let mut saved = tab
                .view_state
                .as_deref()
                .and_then(|saved| serde_json::from_str::<serde_json::Value>(saved).ok())
                .and_then(|saved| saved.as_object().cloned())
                .unwrap_or_default();
            saved.extend(current.fields.clone());
            let saved = serde_json::to_string(&saved)?;
            if tab.view_state.as_deref() == Some(saved.as_str()) {
                continue;
            }
            taide_layout::service::set_view_state(layout, &current.tab, Some(saved))?;
            changed = true;
        }
        if changed {
            taide_runtime::layout_actions::finish_mutation(events, state, project, layout);
        }
    }
    *state.layouts.write() = layouts;
    Ok(())
}

#[derive(Default)]
pub(crate) struct State {
    restored: HashMap<ViewId, DocumentId>,
    observed: HashMap<ViewId, Observed>,
    preview_models: HashMap<ModelViewKey, CachedModelView>,
}

impl State {
    pub(crate) fn begin_frame(&mut self, store: &EditorStore) {
        self.restored.retain(|view, document| {
            store
                .views()
                .get(*view)
                .is_some_and(|current| current.document == *document)
        });
        self.observed.retain(|view, observed| {
            store
                .views()
                .get(*view)
                .is_some_and(|current| current.document == observed.document)
        });
        self.preview_models.retain(|key, cached| {
            store
                .documents()
                .snapshot(key.document)
                .is_ok_and(|document| document.revision == cached.revision)
        });
    }

    pub(crate) fn observe(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        kind: &TabKind,
        geometry: &EditorGeometry,
    ) {
        let Some(current) = store.views().get(view) else {
            return;
        };
        let Ok(document) = store.documents().snapshot(current.document) else {
            return;
        };
        let anchor = current
            .display
            .as_ref()
            .filter(|display| display.revision() == document.revision)
            .filter(|_| {
                current.scroll.x == geometry.scroll.x && current.scroll.y == geometry.scroll.y
            })
            .and_then(|_| geometry.scroll_anchor())
            .map(|(byte, delta_top)| ScrollAnchor {
                revision: document.revision,
                scroll: current.scroll.clone(),
                byte,
                delta_top,
            });
        self.observed.insert(
            view,
            Observed {
                document: document.id,
                kind: kind.clone(),
                anchor,
            },
        );
    }

    pub(crate) fn capture(
        &mut self,
        store: &EditorStore,
        layouts: &HashMap<ProjectId, ProjectLayout>,
        window: &str,
    ) -> Result<Vec<Captured>, EditorError> {
        let mut captured = Vec::new();
        let mut preview_models = Vec::new();
        for (project, layout) in layouts {
            for root in taide_layout::service::all_roots(layout) {
                for tab in crate::tabs::tabs_in(root) {
                    let Some((pane, _)) = taide_layout::service::find_tab(root, &tab.id) else {
                        continue;
                    };
                    let Some(view) = store.views().find(&ViewKey {
                        window: window.into(),
                        pane: pane.clone(),
                        tab: tab.id.clone(),
                    }) else {
                        continue;
                    };
                    let Some(current) = store.views().get(view) else {
                        continue;
                    };
                    let Some(observed) = self.observed.get(&view).filter(|observed| {
                        observed.document == current.document && observed.kind == tab.kind
                    }) else {
                        continue;
                    };
                    let document = store.documents().snapshot(current.document)?;
                    let primary = current.selection.primary;
                    let cursor_state = std::iter::once(&current.selection.selections[primary])
                        .chain(
                            current
                                .selection
                                .selections
                                .iter()
                                .enumerate()
                                .filter(|(index, _)| *index != primary)
                                .map(|(_, selection)| selection),
                        )
                        .map(|selection| {
                            Ok(CursorState {
                                in_selection_mode: Some(selection.anchor != selection.head),
                                selection_start: Some(ModelPosition::from_byte(
                                    &document,
                                    selection.anchor,
                                )?),
                                position: Some(ModelPosition::from_byte(
                                    &document,
                                    selection.head,
                                )?),
                            })
                        })
                        .collect::<Result<Vec<_>, EditorError>>()?;
                    let anchor = observed.anchor.as_ref().filter(|anchor| {
                        anchor.revision == document.revision && anchor.scroll == current.scroll
                    });
                    let first_position = anchor
                        .map(|anchor| ModelPosition::from_byte(&document, anchor.byte))
                        .transpose()?;
                    let saved = PersistedViewState {
                        cursor_state,
                        view_state: ScrollState {
                            scroll_left: Some(current.scroll.x),
                            first_position,
                            first_position_delta_top: anchor.map(|anchor| anchor.delta_top),
                            scroll_top_without_view_zones: anchor
                                .is_none()
                                .then_some(current.scroll.y),
                        },
                    };
                    let serde_json::Value::Object(fields) =
                        serde_json::to_value(&saved).map_err(|_| EditorError::InvalidBoundary)?
                    else {
                        return Err(EditorError::InvalidBoundary);
                    };
                    captured.push(Captured {
                        project: project.clone(),
                        tab: tab.id.clone(),
                        kind: tab.kind.clone(),
                        fields,
                    });
                    if tab.preview {
                        preview_models.push((
                            ModelViewKey {
                                window: window.into(),
                                pane: pane.clone(),
                                document: current.document,
                            },
                            CachedModelView {
                                revision: document.revision,
                                saved,
                            },
                        ));
                    }
                }
            }
        }
        self.preview_models.retain(|key, _| {
            layouts.values().any(|layout| {
                taide_layout::service::all_roots(layout)
                    .any(|root| taide_layout::service::contains_pane(root, &key.pane))
            })
        });
        self.preview_models.extend(preview_models);
        Ok(captured)
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
        let key = ModelViewKey {
            window: current.key.window.clone(),
            pane: current.key.pane.clone(),
            document: current.document,
        };
        let Some(saved) = saved.and_then(PersistedViewState::parse).or_else(|| {
            self.preview_models
                .remove(&key)
                .filter(|cached| cached.revision == document.revision)
                .map(|cached| cached.saved)
        }) else {
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

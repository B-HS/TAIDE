use std::sync::Arc;

use egui::{Color32, Id, Rect, Sense, Ui};
use taide_native_editor::document::{DocumentId, DocumentSnapshot};
use taide_native_editor::inlay_hints::{Hint, Kind};

use crate::editor_geometry::EditorGeometry;

pub(crate) const PADDING_DIVISOR: f32 = 3.0;
pub(crate) const PADDING_CHARACTER: char = '\u{200a}';

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub foreground: Color32,
    pub background: Color32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colors {
    pub default: Style,
    pub type_hint: Style,
    pub parameter: Style,
}

impl Colors {
    pub(crate) fn style(&self, kind: Option<Kind>) -> Style {
        match kind {
            Some(Kind::Type) => self.type_hint,
            Some(Kind::Parameter) => self.parameter,
            None => self.default,
        }
    }
}

#[derive(Clone)]
pub struct Presentation {
    pub document: DocumentId,
    pub revision: u64,
    pub items: Arc<[Hint]>,
    pub colors: Colors,
}

impl Presentation {
    pub(crate) fn describes(&self, document: &DocumentSnapshot) -> bool {
        self.document == document.id && self.revision == document.revision
    }
}

pub(crate) fn tooltip_id(editor: Id, hint: usize, row: usize) -> Id {
    editor.with(("inlay-hint", hint, row))
}

pub(crate) fn show_tooltips(
    ui: &mut Ui,
    editor: Id,
    document: &DocumentSnapshot,
    geometry: &EditorGeometry,
    presentation: &Presentation,
) {
    if !presentation.describes(document) {
        return;
    }
    for row in &*geometry.rows {
        for (index, rect) in row.hint_rects(false) {
            let Some(tooltip) = presentation
                .items
                .get(index)
                .and_then(|hint| hint.tooltip.as_ref())
            else {
                continue;
            };
            let clipped = rect.intersect(geometry.content_rect);
            if clipped.is_positive() {
                ui.interact(
                    clipped,
                    tooltip_id(editor, index, row.index),
                    Sense::hover(),
                )
                .on_hover_text(tooltip);
            }
        }
    }
}

pub fn hint_rects(geometry: &EditorGeometry) -> Vec<(usize, Rect)> {
    geometry
        .rows
        .iter()
        .flat_map(|row| row.hint_rects(false))
        .collect()
}

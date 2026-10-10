use egui::{Context, Event, Modifiers, PointerButton, Pos2, Rect, Stroke, Ui, pos2};
use taide_native_editor::document::{DocumentId, EditorError};
use taide_native_editor::store::EditorStore;
use taide_native_editor::symbol_locations::Mode;
use taide_native_editor::view::{SelectionSet, ViewId};

use crate::editor_geometry::EditorGeometry;
use crate::editor_locations::Provider;
use crate::editor_sticky_scroll::StickyState;

const UNDERLINE_WIDTH: f32 = 1.0;
const UNDERLINE_INSET: f32 = 2.0;

#[derive(Clone)]
struct Scene {
    document: DocumentId,
    revision: u64,
    selection: SelectionSet,
    geometry: EditorGeometry,
    sticky: StickyState,
    word_wrap: bool,
    folds: Vec<std::ops::Range<usize>>,
    peek: Option<String>,
}

#[derive(Default, Clone)]
pub(crate) struct State {
    scene: Option<Scene>,
    pressed: Option<(usize, Pos2)>,
    hovered: Option<usize>,
    keyboard: Option<String>,
}

fn triggered(context: &Context, modifiers: Modifiers) -> bool {
    if context.os().is_mac() {
        modifiers.mac_cmd
    } else {
        modifiers.ctrl
    }
}

fn content_byte_at(geometry: &EditorGeometry, sticky: &StickyState, point: Pos2) -> Option<usize> {
    if !geometry.content_rect.contains(point) {
        return None;
    }
    if sticky.contains_point(point) {
        return sticky.content_byte_at(point);
    }
    geometry.content_byte_at(point)
}

impl State {
    pub(crate) fn input(
        &mut self,
        ui: &Ui,
        store: &mut EditorStore,
        view: ViewId,
        rect: Rect,
        word_wrap: bool,
        peek: Option<&str>,
        provider: &mut Option<&mut dyn Provider>,
    ) -> Result<bool, EditorError> {
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        let document = store.documents().snapshot(current.document)?;
        let valid = self.scene.as_ref().is_some_and(|scene| {
            scene.document == document.id
                && scene.revision == document.revision
                && scene.selection == current.selection
                && scene.geometry.rect == rect
                && scene.geometry.scroll == egui::vec2(current.scroll.x, current.scroll.y)
                && scene.word_wrap == word_wrap
                && scene.folds == current.folds
                && scene.peek.as_deref() == peek
        });
        let available = provider
            .as_deref()
            .is_some_and(|provider| provider.definition_available(store, view));
        if !valid || !available || !ui.is_enabled() || current.composition.is_some() {
            self.pressed = None;
            self.hovered = None;
            self.keyboard = None;
            if let Some(provider) = provider.as_deref_mut() {
                provider.clear_link(view);
            }
            return Ok(false);
        }
        let mut consumed_pointer = false;
        let events = ui.input(|input| input.raw.events.clone());
        for event in events {
            if matches!(&event, Event::Key { pressed: true, .. })
                && self.keyboard.as_ref().is_some_and(|token| {
                    provider
                        .as_deref()
                        .and_then(|provider| provider.keyboard_link(store, view))
                        .is_some_and(|(current, _)| *token == current)
                })
            {
                if let Some(provider) = provider.as_deref_mut() {
                    provider.clear_link(view);
                }
                self.keyboard = None;
            }
            match event {
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers,
                } => {
                    let byte = self
                        .scene
                        .as_ref()
                        .and_then(|scene| content_byte_at(&scene.geometry, &scene.sticky, pos));
                    if pressed {
                        self.pressed = byte
                            .filter(|byte| {
                                triggered(ui.ctx(), modifiers)
                                    && provider.as_deref().is_some_and(|provider| {
                                        provider.source_word(store, view, *byte).is_some()
                                    })
                            })
                            .map(|byte| (document.rope.byte_to_line(byte), pos));
                        consumed_pointer |= self.pressed.is_some();
                    } else if let Some((line, origin)) = self.pressed.take() {
                        consumed_pointer = true;
                        let dragged = origin.distance(pos)
                            > ui.ctx()
                                .options(|options| options.input_options.max_click_dist);
                        if !dragged
                            && triggered(ui.ctx(), modifiers)
                            && let Some(byte) =
                                byte.filter(|byte| document.rope.byte_to_line(*byte) == line)
                            && let Some(provider) = provider.as_deref_mut()
                        {
                            provider.request_at(
                                store,
                                view,
                                byte,
                                if modifiers.alt {
                                    Mode::Aside
                                } else {
                                    Mode::GoTo
                                },
                            )?;
                            ui.ctx().request_repaint();
                        }
                    }
                }
                Event::WindowFocused(false)
                | Event::PointerGone
                | Event::Text(_)
                | Event::Paste(_)
                | Event::Ime(_) => {
                    self.pressed = None;
                    self.hovered = None;
                    self.keyboard = None;
                    if let Some(provider) = provider.as_deref_mut() {
                        provider.clear_link(view);
                    }
                }
                _ => {}
            }
        }
        Ok(consumed_pointer || self.pressed.is_some())
    }

    pub(crate) fn paint(
        &mut self,
        ui: &Ui,
        store: &mut EditorStore,
        view: ViewId,
        geometry: &EditorGeometry,
        sticky: &StickyState,
        response: &egui::Response,
        word_wrap: bool,
        peek: Option<String>,
        provider: &mut Option<&mut dyn Provider>,
        color: egui::Color32,
    ) -> Result<(), EditorError> {
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        let document = store.documents().snapshot(current.document)?;
        let modifiers = ui.input(|input| input.modifiers);
        let available = ui.is_enabled()
            && current.composition.is_none()
            && provider
                .as_deref()
                .is_some_and(|provider| provider.definition_available(store, view));
        let keyboard = provider
            .as_deref()
            .and_then(|provider| provider.keyboard_link(store, view))
            .filter(|_| available);
        let pointer = ui.input(|input| input.pointer.hover_pos());
        let byte = pointer
            .filter(|point| {
                available
                    && (response.contains_pointer() || sticky.contains_point(*point))
                    && triggered(ui.ctx(), modifiers)
            })
            .and_then(|point| content_byte_at(geometry, sticky, point));
        let sticky_hover =
            keyboard.is_none() && pointer.is_some_and(|point| sticky.contains_point(point));
        if let Some((token, _)) = &keyboard {
            self.keyboard = Some(token.clone());
            self.hovered = None;
        } else {
            self.keyboard = None;
        }
        if keyboard.is_none() && byte != self.hovered {
            self.hovered = None;
            if let Some(provider) = provider.as_deref_mut() {
                provider.clear_link(view);
                if let Some(byte) =
                    byte.filter(|byte| provider.source_word(store, view, *byte).is_some())
                {
                    provider.request_at(store, view, byte, Mode::Hover)?;
                    self.hovered = Some(byte);
                }
            }
        }
        if let Some(byte) = keyboard.as_ref().map(|(_, byte)| *byte).or(self.hovered)
            && let Some(provider) = provider.as_deref_mut()
            && let Some(preview) = provider.link_preview(store, view, byte)
        {
            let painter = ui.painter().with_clip_rect(geometry.content_rect);
            let ranges = if sticky_hover {
                sticky.definition_range_rects(&preview.bytes)
            } else {
                geometry.range_rects(preview.bytes)
            };
            for rect in &ranges {
                painter.line_segment(
                    [
                        pos2(rect.left(), rect.bottom() - UNDERLINE_INSET),
                        pos2(rect.right(), rect.bottom() - UNDERLINE_INSET),
                    ],
                    Stroke::new(UNDERLINE_WIDTH, color),
                );
            }
            let show = |ui: &mut Ui| {
                if let Some(code) = preview.code {
                    ui.label(code);
                } else {
                    ui.label(preview.text);
                }
            };
            if keyboard.is_some() {
                if let Some(anchor) = ranges.first() {
                    egui::Tooltip::always_open(
                        ui.ctx().clone(),
                        response.layer_id,
                        response.id.with("definition-preview-hover"),
                        *anchor,
                    )
                    .show(show);
                }
            } else {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                if !sticky_hover {
                    response.clone().on_hover_ui(show);
                }
            }
        }
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        self.scene = Some(Scene {
            document: document.id,
            revision: document.revision,
            selection: current.selection.clone(),
            geometry: geometry.clone(),
            sticky: sticky.clone(),
            word_wrap,
            folds: current.folds.clone(),
            peek,
        });
        Ok(())
    }
}

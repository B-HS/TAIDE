use std::ops::Range;
use std::sync::Arc;

use egui::{
    Align, Area, Color32, Event, EventFilter, FontId, Frame, Id, Key, Order, Rect, Stroke,
    TextEdit, Ui, pos2,
};
use taide_native_editor::document::{DocumentSnapshot, EditorError};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::ViewId;

use crate::editor_completion::Provider;
use crate::editor_geometry::EditorGeometry;
use crate::editor_surface::EditorAppearance;

const MIN_WIDTH: f32 = 288.0;
const INPUT_FONT_SIZE: f32 = 13.0;
const STATUS_FONT_SIZE: f32 = 12.0;
const HORIZONTAL_PADDING: i8 = 8;
const VERTICAL_PADDING: i8 = 6;
const GAP: f32 = 8.0;
const BORDER: f32 = 1.0;
const CORNER: u8 = 6;
const INPUT_FILTER: EventFilter = EventFilter {
    tab: true,
    horizontal_arrows: true,
    vertical_arrows: true,
    escape: true,
};

#[derive(Clone)]
pub struct Session {
    pub token: u128,
    pub focus: u128,
    pub range: Range<usize>,
    pub language: String,
    pub instruction: String,
    pub is_loading: bool,
    pub preview: Option<Arc<str>>,
}

#[derive(Clone, Copy)]
pub struct Colors {
    pub background: Color32,
    pub foreground: Color32,
    pub border: Color32,
    pub removed: Color32,
    pub primary: Color32,
    pub primary_foreground: Color32,
}

impl Colors {
    fn from_ui(ui: &Ui) -> Self {
        Self {
            background: ui.visuals().extreme_bg_color,
            foreground: ui.visuals().text_color(),
            border: ui.visuals().widgets.noninteractive.bg_stroke.color,
            removed: ui.visuals().error_fg_color.gamma_multiply(0.2),
            primary: ui.visuals().selection.bg_fill,
            primary_foreground: ui.visuals().selection.stroke.color,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct State {
    token: Option<u128>,
    phase: Option<(bool, bool, u128)>,
    control: Option<Id>,
    controls: Vec<Id>,
    needs_focus: bool,
    text: String,
    composing: bool,
}

#[derive(Default)]
pub(crate) struct Output {
    pub focus_ids: Vec<Id>,
    pub errors: Vec<EditorError>,
    pub changed: bool,
}

pub fn focus_id(editor: Id, token: u128) -> Id {
    editor.with(("ai-edit-control", token))
}

pub fn is_control(context: &egui::Context, editor: Id, control: Id) -> bool {
    context
        .data(|data| data.get_temp::<Vec<Id>>(editor.with("ai-edit-controls")))
        .is_some_and(|ids| ids.contains(&control))
}

pub fn owns_event(context: &egui::Context, editor: Id, control: Id, event: &Event) -> bool {
    is_control(context, editor, control)
        && matches!(
            event,
            Event::Key { .. }
                | Event::Text(_)
                | Event::Paste(_)
                | Event::Copy
                | Event::Cut
                | Event::Ime(_)
        )
}

pub(crate) fn zone(
    session: Option<&Session>,
    document: &DocumentSnapshot,
    line_height: f32,
) -> Option<(usize, f32)> {
    let session = session?;
    let text = session.preview.as_ref()?;
    Some((
        document.rope.byte_to_line(session.range.end),
        text.split('\n').count() as f32 * line_height,
    ))
}

impl State {
    pub(crate) fn owns_input(&self, ui: &Ui, session: &Session) -> bool {
        self.needs_focus
            || self.token != Some(session.token)
            || self.controls.iter().any(|id| {
                ui.memory(|memory| memory.has_focus(*id))
                    || ui.ctx().keyboard_focus_before_events() == Some(*id)
            })
    }
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        store: &mut EditorStore,
        view: ViewId,
        editor: Id,
        geometry: &EditorGeometry,
        provider: &mut dyn Provider,
    ) -> Output {
        let Some(session) = provider.inline_edit(store, view) else {
            ui.ctx()
                .data_mut(|data| data.remove::<Vec<Id>>(editor.with("ai-edit-controls")));
            *self = Self::default();
            return Output::default();
        };
        if !ui.is_enabled() {
            provider.close_inline_edit(view);
            *self = Self::default();
            return Output::default();
        }
        let fresh = self.token != Some(session.token);
        let phase = (session.preview.is_some(), session.is_loading, session.focus);
        if fresh {
            self.token = Some(session.token);
            self.text = session.instruction.clone();
            self.composing = false;
        }
        let focus = focus_id(editor, session.token);
        self.needs_focus |= fresh || self.phase != Some(phase);
        let request_focus = self.needs_focus;
        self.phase = Some(phase);
        let Some(anchor) = geometry.caret_rect(session.range.start) else {
            return Output {
                focus_ids: vec![focus],
                ..Default::default()
            };
        };
        if request_focus && !session.is_loading && session.preview.is_none() {
            ui.memory_mut(|memory| memory.request_focus_with_filter(focus, INPUT_FILTER));
        }
        let control = self.control.unwrap_or(focus);
        let owned = request_focus
            || self.owns_input(ui, &session)
            || ui.memory(|memory| memory.has_focus(control))
            || ui.ctx().keyboard_focus_before_events() == Some(control);
        let mut accept = false;
        let mut submit = false;
        let mut close = false;
        if owned {
            ui.input_mut(|input| {
                input.events.retain(|event| {
                    match event {
                        Event::Ime(egui::ImeEvent::Preedit { text, .. }) => {
                            self.composing = !text.is_empty()
                        }
                        Event::Ime(egui::ImeEvent::Commit(_)) => self.composing = false,
                        Event::Key {
                            key: Key::Escape,
                            pressed: true,
                            modifiers,
                            ..
                        } if !self.composing && !modifiers.any() => {
                            close = true;
                            return false;
                        }
                        Event::Key {
                            key: Key::Enter,
                            pressed: true,
                            modifiers,
                            ..
                        } if !self.composing && !modifiers.alt && !modifiers.shift => {
                            if session.preview.is_some()
                                && (modifiers.command || modifiers.ctrl || modifiers.mac_cmd)
                            {
                                accept = true;
                                return false;
                            }
                            if !session.is_loading && session.preview.is_none() && !modifiers.any()
                            {
                                submit = true;
                                return false;
                            }
                        }
                        _ => (),
                    }
                    true
                })
            });
        }
        let colors = provider
            .inline_edit_colors()
            .unwrap_or_else(|| Colors::from_ui(ui));
        let bounds = ui.ctx().content_rect();
        let width = MIN_WIDTH.min(bounds.width());
        let height = STATUS_FONT_SIZE + f32::from(VERTICAL_PADDING) * 2.0 + BORDER * 2.0;
        let y = if anchor.top() - height >= bounds.top() {
            anchor.top() - height
        } else {
            anchor.bottom()
        };
        let position = pos2(
            anchor
                .left()
                .clamp(bounds.left(), (bounds.right() - width).max(bounds.left())),
            y.clamp(bounds.top(), (bounds.bottom() - height).max(bounds.top())),
        );
        let label = provider.inline_edit_label("ai.inlineEditLabel");
        let placeholder = provider.inline_edit_label("ai.inlineEditPlaceholder");
        let generating = provider.inline_edit_label("ai.inlineEditGenerating");
        let cancel = provider.inline_edit_label("common.cancel");
        let reject = provider.inline_edit_label("ai.inlineEditReject");
        let accept_label = provider.inline_edit_label("ai.inlineEditAccept");
        self.controls.clear();
        Area::new(editor.with(("ai-edit-widget", session.token)))
            .order(Order::Foreground)
            .fixed_pos(position)
            .constrain(false)
            .show(ui.ctx(), |ui| {
                Frame::new()
                    .fill(colors.background)
                    .stroke(Stroke::new(BORDER, colors.border))
                    .corner_radius(CORNER)
                    .inner_margin(egui::Margin::symmetric(
                        HORIZONTAL_PADDING,
                        VERTICAL_PADDING,
                    ))
                    .shadow(ui.visuals().popup_shadow)
                    .show(ui, |ui| {
                        ui.set_width(
                            (width - f32::from(HORIZONTAL_PADDING) * 2.0 - BORDER * 2.0).max(0.0),
                        );
                        ui.spacing_mut().item_spacing.x = GAP;
                        ui.visuals_mut().override_text_color = Some(colors.foreground);
                        ui.horizontal(|ui| {
                            if session.preview.is_some() {
                                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                                    let response = ui.add(
                                        egui::Button::new(
                                            egui::RichText::new(&accept_label)
                                                .color(colors.primary_foreground),
                                        )
                                        .fill(colors.primary),
                                    );
                                    self.control = Some(response.id);
                                    self.controls.push(response.id);
                                    if request_focus {
                                        response.request_focus();
                                    }
                                    if ui.is_visible() && response.has_focus() {
                                        self.needs_focus = false;
                                    }
                                    accept |= response.clicked();
                                    let response = ui.button(reject);
                                    self.controls.push(response.id);
                                    close |= response.clicked();
                                });
                            } else if session.is_loading {
                                ui.spinner();
                                ui.label(egui::RichText::new(generating).size(STATUS_FONT_SIZE));
                                let response = ui.button(cancel);
                                self.control = Some(response.id);
                                self.controls.push(response.id);
                                if request_focus {
                                    response.request_focus();
                                }
                                if ui.is_visible() && response.has_focus() {
                                    self.needs_focus = false;
                                }
                                close |= response.clicked();
                            } else {
                                let response = ui.add(
                                    TextEdit::singleline(&mut self.text)
                                        .id(focus)
                                        .font(FontId::proportional(INPUT_FONT_SIZE))
                                        .hint_text(placeholder)
                                        .desired_width(ui.available_width()),
                                );
                                self.control = Some(response.id);
                                self.controls.push(response.id);
                                if request_focus {
                                    response.request_focus();
                                }
                                if ui.is_visible() && response.has_focus() {
                                    self.needs_focus = false;
                                }
                                response.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::TextEdit,
                                        true,
                                        label.as_str(),
                                    )
                                });
                            }
                        });
                    });
            });
        provider.set_inline_edit_instruction(view, self.text.clone());
        ui.ctx().data_mut(|data| {
            data.insert_temp(editor.with("ai-edit-controls"), self.controls.clone())
        });
        let mut output = Output {
            focus_ids: self.controls.clone(),
            ..Default::default()
        };
        if close {
            provider.close_inline_edit(view);
            ui.memory_mut(|memory| memory.request_focus(editor));
            *self = Self::default();
        }
        if submit && !close {
            provider.submit_inline_edit(store, view);
            ui.ctx().request_repaint();
        }
        if accept && !close {
            match provider.accept_inline_edit(store, view) {
                Ok(changed) => {
                    output.changed = changed;
                    ui.memory_mut(|memory| memory.request_focus(editor));
                    *self = Self::default();
                }
                Err(error) => output.errors.push(error),
            }
        }
        output
    }
}

pub(crate) fn paint_preview(
    ui: &Ui,
    session: &Session,
    rect: Rect,
    scroll_x: f32,
    appearance: &EditorAppearance,
    provider: &mut dyn Provider,
) {
    let Some(text) = session.preview.as_ref() else {
        return;
    };
    let mut job = provider
        .code(ui, &session.language, text)
        .unwrap_or_else(|| {
            egui::text::LayoutJob::simple(
                text.to_string(),
                appearance.font.clone(),
                appearance.foreground,
                f32::INFINITY,
            )
        });
    for section in &mut job.sections {
        section.format.line_height = Some(appearance.line_height);
    }
    job.wrap.max_width = f32::INFINITY;
    let galley = ui.painter().layout_job(job);
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    painter.rect_filled(rect, 0.0, appearance.background);
    painter.galley(
        pos2(rect.left() - scroll_x, rect.top()),
        galley,
        appearance.foreground,
    );
}

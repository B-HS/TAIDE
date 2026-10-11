use std::ops::Range;

use egui::{Color32, Event, EventFilter, FontId, Id, Key, Rect, Stroke, TextEdit, Ui};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::ViewId;

use crate::editor_geometry::EditorGeometry;
use crate::editor_surface::EditorAppearance;

const MIN_COLUMNS: f32 = 20.0;
const COLUMN_FACTOR: f32 = 1.1;
const MIN_WIDTH: f32 = 200.0;
const PADDING: i8 = 3;
const CORNER_RADIUS: u8 = 4;
const BORDER_WIDTH: f32 = 1.0;
const SHADOW_OFFSET: [i8; 2] = [0, 2];
const SHADOW_BLUR: u8 = 8;
const INPUT_FILTER: EventFilter = EventFilter {
    tab: true,
    horizontal_arrows: true,
    vertical_arrows: true,
    escape: true,
};

#[derive(Clone)]
pub struct Session {
    pub token: u128,
    pub range: Range<usize>,
    pub name: String,
    pub selection: Range<usize>,
    pub columns: usize,
}

pub trait Provider {
    fn current(&self, store: &EditorStore, view: ViewId) -> Option<Session>;
    fn accept(&mut self, store: &EditorStore, view: ViewId, token: u128, name: String);
    fn cancel(&mut self, view: ViewId, token: u128);
}

#[derive(Clone, Copy)]
pub struct Colors {
    pub background: Color32,
    pub foreground: Color32,
    pub border: Color32,
    pub shadow: Color32,
}

impl From<&crate::editor_find_widget::FindAppearance> for Colors {
    fn from(appearance: &crate::editor_find_widget::FindAppearance) -> Self {
        Self {
            background: appearance.input_background,
            foreground: appearance.input_foreground,
            border: appearance.input_border,
            shadow: appearance.shadow,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct State {
    token: Option<u128>,
    text: String,
    composing: bool,
}

#[derive(Default)]
pub(crate) struct Output {
    pub focus_ids: Vec<Id>,
    pub released: Vec<(usize, Event)>,
}

pub fn focus_id(editor: Id, token: u128) -> Id {
    editor.with(("rename-input", token))
}

impl State {
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        store: &EditorStore,
        view: ViewId,
        editor: Id,
        geometry: &EditorGeometry,
        appearance: &EditorAppearance,
        colors: Colors,
        provider: &mut dyn Provider,
    ) -> Output {
        let Some(session) = provider.current(store, view) else {
            *self = Self::default();
            return Output::default();
        };
        if !ui.is_enabled() {
            provider.cancel(view, session.token);
            ui.memory_mut(|memory| memory.surrender_focus(focus_id(editor, session.token)));
            *self = Self::default();
            return Output::default();
        }
        let fresh = self.token != Some(session.token);
        if fresh {
            self.token = Some(session.token);
            self.text = session.name.clone();
            self.composing = false;
        }
        let Some(anchor) = geometry.caret_rect(session.range.start) else {
            provider.cancel(view, session.token);
            *self = Self::default();
            return Output::default();
        };
        let id = focus_id(editor, session.token);
        if !fresh
            && (!ui.input(|input| input.raw.focused) || !ui.memory(|memory| memory.has_focus(id)))
        {
            provider.cancel(view, session.token);
            *self = Self::default();
            return Output::default();
        }
        if fresh {
            ui.memory_mut(|memory| memory.request_focus_with_filter(id, INPUT_FILTER));
            let mut state = TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(session.selection.start),
                    egui::text::CCursor::new(session.selection.end),
                )));
            state.store(ui.ctx(), id);
        }
        let mut action = None;
        let mut released = Vec::new();
        ui.input_mut(|input| {
            let raw = input.raw.events.clone();
            let mut next = 0;
            input.events.retain(|event| {
                let index = egui::Context::raw_event_index(&raw, event, &mut next);
                if action.is_some() && is_keyboard(event) {
                    released.push((index, event.clone()));
                    return false;
                }
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
                    } if !modifiers.alt
                        && !modifiers.ctrl
                        && !modifiers.command
                        && !modifiers.mac_cmd =>
                    {
                        action = Some(false);
                        return false;
                    }
                    Event::Key {
                        key: Key::Enter,
                        pressed: true,
                        modifiers,
                        ..
                    } if !modifiers.alt && !modifiers.shift => {
                        if !self.composing {
                            action = Some(true);
                        }
                        return false;
                    }
                    _ => (),
                }
                true
            });
        });
        let font: FontId = appearance.font.clone();
        let column_width = ui.fonts_mut(|fonts| fonts.glyph_width(&font, '0'));
        let width = ((session.columns as f32 * COLUMN_FACTOR).max(MIN_COLUMNS) * column_width)
            .max(MIN_WIDTH);
        let height = appearance.line_height + f32::from(PADDING) * 2.0;
        let screen = ui.ctx().content_rect();
        let top = if anchor.bottom() + height <= screen.bottom() {
            anchor.bottom()
        } else {
            (anchor.top() - height).max(screen.top())
        };
        let left = anchor
            .left()
            .min((screen.right() - width).max(screen.left()))
            .max(screen.left());
        let rect = Rect::from_min_size(
            egui::pos2(left, top),
            egui::vec2(width.min(screen.width()), height),
        );
        egui::Area::new(editor.with("rename-box"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .movable(false)
            .show(ui.ctx(), |ui| {
                egui::Frame::NONE
                    .fill(colors.background)
                    .stroke(Stroke::new(BORDER_WIDTH, colors.border))
                    .corner_radius(CORNER_RADIUS)
                    .inner_margin(PADDING)
                    .shadow(egui::epaint::Shadow {
                        offset: SHADOW_OFFSET,
                        blur: SHADOW_BLUR,
                        spread: 0,
                        color: colors.shadow,
                    })
                    .show(ui, |ui| {
                        let output = TextEdit::singleline(&mut self.text)
                            .id(id)
                            .font(font)
                            .text_color(colors.foreground)
                            .frame(egui::Frame::NONE)
                            .margin(egui::Margin::ZERO)
                            .desired_width((rect.width() - f32::from(PADDING) * 2.0).max(0.0))
                            .event_filter(INPUT_FILTER)
                            .show(ui);
                        if fresh {
                            ui.memory_mut(|memory| {
                                memory.request_focus_with_filter(id, INPUT_FILTER)
                            });
                        }
                        if output.response.lost_focus() && action.is_none() {
                            action = Some(false);
                        }
                    });
            });
        ui.input_mut(|input| input.events.retain(|event| !is_keyboard(event)));
        if let Some(accept) = action {
            if accept {
                provider.accept(store, view, session.token, self.text.clone());
            } else {
                provider.cancel(view, session.token);
            }
            ui.memory_mut(|memory| memory.request_focus_with_filter(editor, INPUT_FILTER));
            *self = Self::default();
            return Output {
                focus_ids: Vec::new(),
                released,
            };
        }
        Output {
            focus_ids: vec![id],
            released,
        }
    }
}

fn is_keyboard(event: &Event) -> bool {
    matches!(
        event,
        Event::Key { .. }
            | Event::Text(_)
            | Event::Paste(_)
            | Event::Copy
            | Event::Cut
            | Event::Ime(_)
    )
}

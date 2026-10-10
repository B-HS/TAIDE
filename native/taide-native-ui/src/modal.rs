use egui::emath::TSTransform;
use egui::epaint::Shadow;
use egui::{Color32, Event, Id, Key, Modifiers, Sense, Stroke, Ui};

use crate::tooltips::motion::Motion;

pub const CORNER_RADIUS: u8 = 8;
pub const BORDER_WIDTH: f32 = 1.0;
pub const FOCUS_FILTER: egui::EventFilter = egui::EventFilter {
    tab: true,
    horizontal_arrows: true,
    vertical_arrows: true,
    escape: true,
};
const SCRIM_OPACITY: f32 = 0.5;
const SHADOW_OFFSET: [i8; 2] = [0, 8];
const SHADOW_BLUR: u8 = 24;
const CONTENT_TRANSITION_SECONDS: f64 = 0.2;
const SETTLED_OPACITY: f32 = 1.0;
const SETTLED_SCALE: f32 = 1.0;
const BACKDROP_ID: &str = "backdrop-hit";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chrome {
    pub background: Color32,
    pub border: Color32,
    pub shadow: Color32,
}

impl Chrome {
    pub fn frame(&self) -> egui::Frame {
        egui::Frame::NONE
            .fill(self.background)
            .stroke(Stroke::new(BORDER_WIDTH, self.border))
            .shadow(Shadow {
                offset: SHADOW_OFFSET,
                blur: SHADOW_BLUR,
                spread: 0,
                color: self.shadow,
            })
            .corner_radius(CORNER_RADIUS)
    }

    pub fn scrim(&self, opacity: f32) -> Color32 {
        self.shadow.gamma_multiply(SCRIM_OPACITY * opacity)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transition {
    pub opacity: f32,
    pub scale: f32,
    pub scrim_opacity: f32,
    pub is_active: bool,
}

impl Transition {
    pub const SETTLED: Self = Self {
        opacity: SETTLED_OPACITY,
        scale: SETTLED_SCALE,
        scrim_opacity: SETTLED_OPACITY,
        is_active: false,
    };

    pub fn transform(&self, origin: egui::Pos2) -> TSTransform {
        TSTransform::new(origin.to_vec2() * (SETTLED_SCALE - self.scale), self.scale)
    }
}

#[derive(Clone, Copy)]
pub struct Presence {
    content: Motion,
    scrim: Motion,
    is_reduced_motion: bool,
}

impl Presence {
    pub fn enter(now: f64) -> Self {
        Self {
            content: Motion::with_duration(true, now, CONTENT_TRANSITION_SECONDS),
            scrim: Motion::new(true, now),
            is_reduced_motion: false,
        }
    }

    pub fn set_reduced_motion(&mut self, is_reduced_motion: bool) {
        self.is_reduced_motion = is_reduced_motion;
    }

    pub fn target(&mut self, is_open: bool, now: f64) {
        self.content.target(is_open, now);
        self.scrim.target(is_open, now);
    }

    pub fn is_present(&self, now: f64) -> bool {
        if self.is_reduced_motion {
            return self.content.is_open();
        }
        self.content.is_present(now)
    }

    pub fn sample(&self, now: f64) -> Transition {
        if self.is_reduced_motion {
            return Transition::SETTLED;
        }
        let content = self.content.sample_popup(now);
        let scrim = self.scrim.sample_popup(now);
        Transition {
            opacity: content.opacity,
            scale: content.scale,
            scrim_opacity: scrim.opacity,
            is_active: content.active || scrim.active,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Composition {
    is_composing: bool,
}

impl Composition {
    pub fn observe(&mut self, events: &[Event]) -> bool {
        let mut has_composition_event = false;
        for event in events {
            let Event::Ime(composition) = event else {
                continue;
            };
            has_composition_event = true;
            match composition {
                egui::ImeEvent::Preedit { text, .. } => self.is_composing = !text.is_empty(),
                egui::ImeEvent::Commit(_) => self.is_composing = false,
                _ => (),
            }
        }
        has_composition_event || self.is_composing
    }

    pub fn reset(&mut self) {
        self.is_composing = false;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FocusReturn {
    origin: Option<Id>,
    returning: Option<Id>,
}

impl FocusReturn {
    pub fn origin(&self) -> Option<Id> {
        self.origin
    }

    pub fn capture(&mut self, context: &egui::Context) {
        self.origin = self
            .returning
            .take()
            .or_else(|| context.memory(|memory| memory.focused()));
    }

    pub fn release(&mut self, context: &egui::Context, should_return: bool) {
        self.returning = self.origin.take().filter(|_| should_return);
        if let Some(origin) = self.returning {
            context.memory_mut(|memory| memory.request_focus(origin));
        }
    }

    pub fn settle(&mut self, context: &egui::Context) {
        let Some(origin) = self.returning.take() else {
            return;
        };
        context.memory_mut(|memory| memory.request_focus(origin));
        context.request_repaint();
    }
}

#[derive(Clone, Copy)]
pub struct Layer {
    pub id: Id,
    pub chrome: Chrome,
    pub padding: i8,
    pub transition: Option<Transition>,
    pub is_modal: bool,
}

impl Layer {
    pub fn layer_id(&self) -> egui::LayerId {
        layer_id(self.id)
    }

    pub fn show<T>(
        &self,
        context: &egui::Context,
        content: impl FnOnce(&mut Ui) -> T,
    ) -> egui::ModalResponse<T> {
        let frame = self.chrome.frame().inner_margin(self.padding);
        let transition = match self.transition {
            Some(transition) => transition,
            None if self.is_modal => {
                return egui::Modal::new(self.id)
                    .frame(frame)
                    .backdrop_color(self.chrome.scrim(SETTLED_OPACITY))
                    .show(context, content);
            }
            None => Transition::SETTLED,
        };
        let id = self.id;
        let area = egui::Modal::default_area(id).fade_in(false);
        let layer = area.layer();
        let transform = transition.transform(context.content_rect().center());
        context.set_transform_layer(layer, transform);
        context.register_dismissal_layer(id);
        let is_top_modal = self.is_modal
            && context.memory_mut(|memory| {
                memory.set_modal_layer(layer);
                memory.top_modal_layer() == Some(layer)
            });
        let any_popup_open = egui::Popup::is_any_open(context);
        let scrim = self.chrome.scrim(transition.scrim_opacity);
        let shown = area.show(context, |ui| {
            let backdrop_rect = transform.inverse() * ui.ctx().content_rect();
            ui.set_clip_rect(backdrop_rect);
            let backdrop_response = ui.interact(
                backdrop_rect,
                id.with(BACKDROP_ID),
                Sense::CLICK | Sense::DRAG,
            );
            ui.painter().rect_filled(backdrop_rect, 0.0, scrim);
            let inner = ui
                .scope_builder(
                    egui::UiBuilder::new().sense(Sense::CLICK | Sense::DRAG),
                    |ui| {
                        ui.set_opacity(transition.opacity);
                        frame.show(ui, content).inner
                    },
                )
                .inner;
            (inner, backdrop_response)
        });
        egui::ModalResponse {
            response: shown.response,
            backdrop_response: shown.inner.1,
            inner: shown.inner.0,
            is_top_modal,
            any_popup_open,
        }
    }
}

pub fn layer_id(id: Id) -> egui::LayerId {
    egui::Modal::default_area(id).layer()
}

pub fn unmount(context: &egui::Context, layer: egui::LayerId) {
    context.set_transform_layer(layer, TSTransform::IDENTITY);
    context.unregister_dismissal_layer(layer.id);
    let focused = context.memory(|memory| memory.focused());
    if let Some(focused) = focused
        && context
            .read_response(focused)
            .is_some_and(|response| response.layer_id == layer)
    {
        context.memory_mut(|memory| memory.surrender_focus(focused));
    }
}

pub fn takes_escape<T>(context: &egui::Context, response: &egui::ModalResponse<T>) -> bool {
    response.is_top_modal
        && !response.any_popup_open
        && context.dismissal_layers().last() == Some(&response.response.layer_id.id)
        && context.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape))
}

pub fn trap_focus(context: &egui::Context, order: &[Id]) {
    let focused = context.memory(|memory| memory.focused());
    if let Some(focused) = focused {
        context.memory_mut(|memory| memory.set_focus_lock_filter(focused, FOCUS_FILTER));
    }
    let direction = context.input_mut(|input| {
        let mut direction = None;
        input.events.retain(|event| {
            if let Event::Key {
                key: Key::Tab,
                modifiers,
                pressed: true,
                ..
            } = event
                && (!modifiers.any() || modifiers.shift_only())
            {
                direction.get_or_insert(!modifiers.shift);
                return false;
            }
            true
        });
        direction
    });
    let Some(is_forward) = direction else {
        return;
    };
    if order.is_empty() {
        return;
    }
    let index = focused.and_then(|focused| order.iter().position(|id| *id == focused));
    let next = match (index, is_forward) {
        (Some(index), true) => (index + 1) % order.len(),
        (Some(index), false) => (index + order.len() - 1) % order.len(),
        (None, true) => 0,
        (None, false) => order.len() - 1,
    };
    context.memory_mut(|memory| memory.request_focus_with_filter(order[next], FOCUS_FILTER));
}

#[cfg(test)]
#[path = "modal-tests.rs"]
mod tests;

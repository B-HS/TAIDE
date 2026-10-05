use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::egui::{
    self, Color32, Context, Event, FontId, Id, Pos2, Rect, Response, Stroke, ViewportId,
};
use taide_model::{error::AppResult, theme::ResolvedTheme};

#[path = "tooltip-placement.rs"]
mod placement;
use placement::{Arrow, Body, Placement};

#[path = "tooltip-motion.rs"]
pub(crate) mod motion;
use motion::Motion;

const DELAY_SECONDS: f64 = 0.4;
const SKIP_DELAY_SECONDS: f64 = 0.3;
const TRANSIT_PADDING: f32 = 5.0;
const FONT_SIZE: f32 = 12.0;
const RADIUS: u8 = 6;
const HORIZONTAL_PADDING: i8 = 12;
const VERTICAL_PADDING: i8 = 6;
const LINE_HEIGHT: f32 = 16.0;

#[derive(Clone, Copy)]
pub struct Appearance {
    background: Color32,
    border: Color32,
    foreground: Color32,
}

pub use super::tooltip_trigger::{Trigger, wrap_button};

impl Appearance {
    pub fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        Ok(Self {
            background: crate::presentation::color(theme, "tooltip.background")?,
            border: crate::presentation::color(theme, "tooltip.border")?,
            foreground: crate::presentation::color(theme, "app.foreground")?,
        })
    }
}

#[derive(Clone, Default)]
pub struct Provider(Arc<Mutex<HashMap<ViewportId, Viewport>>>);

struct Viewport {
    context: Context,
    pass: u64,
    prepared: bool,
    now: f64,
    events: Vec<Event>,
    replayed_actions: Vec<usize>,
    keyboard_closes: Vec<KeyboardClose>,
    dismissal_layers: Vec<Id>,
    pointer: Option<Pos2>,
    widgets: HashMap<Id, Widget>,
    layers: HashMap<egui::LayerId, u64>,
    open: Option<Id>,
    open_epoch: u64,
    skip_until: Option<f64>,
    is_delayed: bool,
    pending: Vec<Pending>,
}

struct KeyboardClose {
    id: Id,
    index: usize,
    press: Option<(u64, usize)>,
    open_epoch: u64,
}

struct Pending {
    response: Response,
    label: String,
    align: egui::RectAlign,
    appearance: Appearance,
    is_controlled: bool,
}

#[derive(Default)]
struct Widget {
    seen: u64,
    enabled: bool,
    trigger_rect: Option<Rect>,
    focus_target: Option<Id>,
    dismissal_id: Option<Id>,
    controlled: Option<bool>,
    hovered: bool,
    focused: bool,
    can_keyboard_activate: bool,
    space_armed: Option<(u64, usize)>,
    pointer_opened: bool,
    pointer_down: bool,
    pointer_replayed: bool,
    deadline: Option<f64>,
    content: Option<Rect>,
    body: Option<Body>,
    arrow: Option<Arrow>,
    content_hovered: bool,
    grace: Option<Grace>,
    motion: Option<Motion>,
}

struct Grace(Vec<Pos2>);

impl Widget {
    fn contains_content(&self, point: Pos2) -> bool {
        self.body.map_or_else(
            || self.content.is_some_and(|content| content.contains(point)),
            |body| body.contains(point),
        ) || self.arrow.is_some_and(|arrow| arrow.contains(point))
    }
}

#[derive(Clone, Copy)]
struct Painted {
    bounds: Rect,
    body: Option<Body>,
    arrow: Option<Arrow>,
}

impl Grace {
    fn new(exit: Pos2, source: Rect, target: Rect) -> Self {
        let sides = [
            (
                (source.left() - exit.x).abs(),
                egui::vec2(TRANSIT_PADDING, 0.0),
            ),
            (
                (source.right() - exit.x).abs(),
                egui::vec2(-TRANSIT_PADDING, 0.0),
            ),
            (
                (source.top() - exit.y).abs(),
                egui::vec2(0.0, TRANSIT_PADDING),
            ),
            (
                (source.bottom() - exit.y).abs(),
                egui::vec2(0.0, -TRANSIT_PADDING),
            ),
        ];
        let inward = sides
            .into_iter()
            .min_by(|left, right| left.0.total_cmp(&right.0))
            .unwrap()
            .1;
        let tangent = egui::vec2(inward.y, inward.x);
        let mut points = vec![
            exit + inward - tangent,
            exit + inward + tangent,
            target.left_top(),
            target.right_top(),
            target.right_bottom(),
            target.left_bottom(),
        ];
        points.sort_by(|left, right| left.x.total_cmp(&right.x).then(left.y.total_cmp(&right.y)));
        points.dedup();
        let cross =
            |a: Pos2, b: Pos2, c: Pos2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
        let mut lower = Vec::<Pos2>::new();
        let mut upper = Vec::<Pos2>::new();
        for point in &points {
            while lower.len() > 1
                && cross(lower[lower.len() - 2], lower[lower.len() - 1], *point) <= 0.0
            {
                lower.pop();
            }
            lower.push(*point);
        }
        for point in points.into_iter().rev() {
            while upper.len() > 1
                && cross(upper[upper.len() - 2], upper[upper.len() - 1], point) <= 0.0
            {
                upper.pop();
            }
            upper.push(point);
        }
        lower.pop();
        upper.pop();
        lower.extend(upper);
        Self(lower)
    }

    fn contains(&self, point: Pos2) -> bool {
        let mut inside = false;
        let Some(mut previous) = self.0.last().copied() else {
            return false;
        };
        for next in &self.0 {
            if (previous.y > point.y) != (next.y > point.y)
                && point.x
                    < (next.x - previous.x) * (point.y - previous.y) / (next.y - previous.y)
                        + previous.x
            {
                inside = !inside;
            }
            previous = *next;
        }
        inside
    }
}

impl Viewport {
    fn replay_keyboard(
        &mut self,
        window_focused: bool,
        clicked: Option<Id>,
        available: &[usize],
    ) -> Vec<usize> {
        self.replayed_actions.clear();
        self.keyboard_closes.clear();
        for widget in self.widgets.values_mut() {
            widget.pointer_replayed = false;
        }
        let mut consumed = Vec::new();
        for (index, event) in self.events.clone().into_iter().enumerate() {
            match event {
                Event::PointerButton {
                    pos,
                    pressed,
                    button,
                    ..
                } => {
                    let targets = self
                        .widgets
                        .iter()
                        .filter_map(|(id, widget)| {
                            if !widget.enabled {
                                return None;
                            }
                            let trigger_contains =
                                widget.trigger_rect.is_some_and(|rect| rect.contains(pos));
                            let open = widget.controlled.unwrap_or(self.open == Some(*id));
                            let content_present = open
                                || widget
                                    .motion
                                    .is_some_and(|motion| motion.is_present(self.now));
                            let is_click = !pressed
                                && button == egui::PointerButton::Primary
                                && (clicked == Some(*id)
                                    || widget
                                        .focus_target
                                        .is_some_and(|target| clicked == Some(target)));
                            (is_click
                                || (pressed
                                    && ((trigger_contains && open)
                                        || (content_present && !widget.contains_content(pos)))))
                            .then_some((*id, widget.controlled))
                        })
                        .collect::<Vec<_>>();
                    for (id, controlled) in targets {
                        self.request(id, false, controlled);
                    }
                    for widget in self.widgets.values_mut() {
                        widget.pointer_replayed = true;
                        if !pressed {
                            widget.pointer_down = false;
                        } else if widget.trigger_rect.is_some_and(|rect| rect.contains(pos)) {
                            widget.pointer_down = true;
                        }
                    }
                    self.replayed_actions.push(index);
                }
                Event::AccessKitActionRequest(request)
                    if request.target_tree == egui::accesskit::TreeId::ROOT
                        && request.action == egui::accesskit::Action::Focus =>
                {
                    let next = self.widgets.iter().find_map(|(id, widget)| {
                        (widget.enabled
                            && (request.target_node == id.accesskit_id()
                                || widget.focus_target.is_some_and(|target| {
                                    request.target_node == target.accesskit_id()
                                })))
                        .then_some(*id)
                    });
                    let Some(next) = next else {
                        continue;
                    };
                    let previous = self
                        .widgets
                        .iter()
                        .filter_map(|(id, widget)| {
                            (widget.focused && (*id != next || !window_focused))
                                .then_some((*id, widget.controlled))
                        })
                        .collect::<Vec<_>>();
                    for (id, controlled) in previous {
                        self.request(id, false, controlled);
                        let widget = self.widgets.get_mut(&id).unwrap();
                        widget.focused = false;
                        widget.space_armed = None;
                    }
                    let widget = self.widgets.get_mut(&next).unwrap();
                    let should_open = window_focused && !widget.focused && !widget.pointer_down;
                    widget.focused = window_focused;
                    let controlled = widget.controlled;
                    let closing_content = widget
                        .motion
                        .is_some_and(|motion| !motion.is_open() && motion.is_present(self.now));
                    if should_open {
                        self.request_open(next, controlled, closing_content);
                    }
                    self.replayed_actions.push(index);
                }
                Event::AccessKitActionRequest(request)
                    if request.target_tree == egui::accesskit::TreeId::ROOT
                        && request.action == egui::accesskit::Action::Click =>
                {
                    let target = self.widgets.iter().find_map(|(id, widget)| {
                        (widget.enabled
                            && (request.target_node == id.accesskit_id()
                                || widget.focus_target.is_some_and(|target| {
                                    request.target_node == target.accesskit_id()
                                })))
                        .then_some((*id, widget.controlled))
                    });
                    if let Some((id, controlled)) = target {
                        self.request(id, false, controlled);
                        self.replayed_actions.push(index);
                    }
                }
                Event::Key {
                    key: egui::Key::Escape,
                    pressed: true,
                    ..
                } if available.contains(&index) => {
                    let controlled_focused = self
                        .widgets
                        .values()
                        .any(|widget| widget.focused && widget.controlled.is_some());
                    let top = self.dismissal_layers.last();
                    let target = self.widgets.iter().find_map(|(id, widget)| {
                        (top.is_some() && top == widget.dismissal_id.as_ref())
                            .then_some((*id, widget.controlled))
                    });
                    if let Some((id, controlled)) = target {
                        self.request(id, false, controlled);
                        if !controlled_focused {
                            consumed.push(index);
                        }
                        self.replayed_actions.push(index);
                    }
                }
                Event::Key {
                    key,
                    pressed,
                    repeat,
                    ..
                } => {
                    let focused = self.widgets.iter().find_map(|(id, widget)| {
                        (widget.focused
                            && widget.can_keyboard_activate
                            && widget.controlled.is_none())
                        .then_some(*id)
                    });
                    let Some(id) = focused else {
                        continue;
                    };
                    if key == egui::Key::Enter && pressed {
                        self.keyboard_closes.push(KeyboardClose {
                            id,
                            index,
                            press: None,
                            open_epoch: self.open_epoch,
                        });
                        self.replayed_actions.push(index);
                    } else if key == egui::Key::Space {
                        let widget = self.widgets.get_mut(&id).unwrap();
                        if pressed && !repeat {
                            widget.space_armed = Some((self.pass, index));
                        }
                        if !pressed && let Some(press) = widget.space_armed.take() {
                            self.keyboard_closes.push(KeyboardClose {
                                id,
                                index,
                                press: Some(press),
                                open_epoch: self.open_epoch,
                            });
                        }
                        self.replayed_actions.push(index);
                    }
                }
                _ => {}
            }
        }
        consumed
    }

    fn available_key_indices(&self, events: &[Event]) -> Vec<usize> {
        let mut next = 0;
        events
            .iter()
            .filter_map(|event| {
                let index = Context::raw_event_index(&self.events, event, &mut next);
                (index != usize::MAX && matches!(event, Event::Key { .. })).then_some(index)
            })
            .collect()
    }

    fn apply_keyboard_close(&mut self, id: Id, available: &[usize]) {
        if self.keyboard_closes.iter().any(|action| {
            action.id == id
                && action.open_epoch == self.open_epoch
                && self.open == Some(id)
                && available.contains(&action.index)
                && action
                    .press
                    .is_none_or(|(pass, index)| pass != self.pass || available.contains(&index))
        }) {
            self.close(id);
        }
        if let Some(widget) = self.widgets.get_mut(&id)
            && widget
                .space_armed
                .is_some_and(|(pass, index)| pass == self.pass && !available.contains(&index))
        {
            widget.space_armed = None;
        }
    }

    fn close(&mut self, id: Id) {
        self.request(id, false, None);
    }

    fn request_open(&mut self, id: Id, controlled: Option<bool>, closing_content: bool) {
        let changed = !controlled.unwrap_or(self.open == Some(id));
        self.request(id, true, controlled);
        if changed && controlled.is_none() && closing_content {
            self.close(id);
        }
    }

    fn request(&mut self, id: Id, open: bool, controlled: Option<bool>) {
        if let Some(widget) = self.widgets.get_mut(&id) {
            widget.deadline = None;
            widget.grace = None;
        }
        if controlled.unwrap_or(self.open == Some(id)) == open {
            return;
        }
        if !open {
            if controlled.is_none() {
                self.open = None;
                if let Some(motion) = self
                    .widgets
                    .get_mut(&id)
                    .and_then(|widget| widget.motion.as_mut())
                {
                    motion.target(false, self.now);
                }
            }
            self.skip_until = Some(self.now + SKIP_DELAY_SECONDS);
            return;
        }
        let previous = self.open.filter(|previous| *previous != id);
        if let Some(previous) = previous
            && let Some(widget) = self.widgets.get_mut(&previous)
        {
            widget.deadline = None;
            widget.grace = None;
            if let Some(motion) = &mut widget.motion {
                motion.target(false, self.now);
            }
        }
        self.open = controlled.is_none().then_some(id);
        if controlled.is_none() {
            self.open_epoch = self.open_epoch.wrapping_add(1);
        }
        if controlled.is_none()
            && let Some(widget) = self.widgets.get_mut(&id)
        {
            if !widget
                .motion
                .is_some_and(|motion| motion.is_present(self.now))
                && let Some(dismissal_id) = widget.dismissal_id
            {
                self.dismissal_layers.retain(|layer| *layer != dismissal_id);
                self.dismissal_layers.push(dismissal_id);
            }
            let motion = widget
                .motion
                .get_or_insert_with(|| Motion::new(true, self.now));
            motion.target(true, self.now);
        }
        self.is_delayed = false;
        self.skip_until = None;
        if previous.is_some()
            || self
                .widgets
                .iter()
                .any(|(other, widget)| *other != id && widget.controlled == Some(true))
        {
            self.skip_until = Some(self.now + SKIP_DELAY_SECONDS);
        }
    }
}

impl Provider {
    #[cfg(feature = "inspection")]
    pub fn inspection_open(&self, context: &Context) -> Vec<(Id, Rect)> {
        let viewports = self.0.lock().expect("native tooltip owner is available");
        let Some(viewport) = viewports.get(&context.viewport_id()) else {
            return Vec::new();
        };
        if viewport.context != *context {
            return Vec::new();
        }
        viewport
            .widgets
            .iter()
            .filter_map(|(id, widget)| {
                widget
                    .controlled
                    .unwrap_or(viewport.open == Some(*id))
                    .then_some(widget.content)
                    .flatten()
                    .map(|content| (*id, content))
            })
            .collect()
    }

    pub fn show_controlled(
        &self,
        response: &Response,
        label: Option<&str>,
        align: egui::RectAlign,
        appearance: &Appearance,
    ) {
        self.begin_frame(&response.ctx);
        self.process_trigger(response, None, Some(label.is_some()));
        let Some(label) = label else {
            self.content(response, None);
            return;
        };
        self.queue(response, label, align, appearance, true);
    }

    fn queue(
        &self,
        response: &Response,
        label: &str,
        align: egui::RectAlign,
        appearance: &Appearance,
        is_controlled: bool,
    ) {
        let mut viewports = self.0.lock().expect("native tooltip owner is available");
        let viewport = viewports.get_mut(&response.ctx.viewport_id()).unwrap();
        viewport.pending.push(Pending {
            response: response.clone(),
            label: label.into(),
            align,
            appearance: *appearance,
            is_controlled,
        });
    }

    fn render_pending(&self, pending: Pending) {
        let response = &pending.response;
        if pending.is_controlled {
            self.track_layer(response);
            self.content_with_arrow(
                response,
                pending
                    .appearance
                    .render_controlled(response, &pending.label, pending.align),
            );
            return;
        }
        let motion = {
            let mut viewports = self.0.lock().expect("native tooltip owner is available");
            let viewport = viewports.get_mut(&response.ctx.viewport_id()).unwrap();
            let is_open = viewport.open == Some(response.id);
            let widget = viewport.widgets.get_mut(&response.id).unwrap();
            if is_open && widget.motion.is_none() {
                widget.motion = Some(Motion::new(true, viewport.now));
            }
            if let Some(motion) = &mut widget.motion {
                motion.target(is_open, viewport.now);
                if !motion.is_present(viewport.now) {
                    widget.motion = None;
                }
            }
            widget.motion
        };
        let Some(motion) = motion else {
            self.content(response, None);
            return;
        };
        self.track_layer(response);
        self.content_with_arrow(
            response,
            pending
                .appearance
                .render(response, &pending.label, pending.align, motion),
        );
    }

    fn track_layer(&self, response: &Response) {
        let layer =
            egui::LayerId::new(egui::Order::Tooltip, tooltip_id(&response.ctx, response.id));
        response.ctx.register_dismissal_layer(layer.id);
        let mut viewports = self.0.lock().expect("native tooltip owner is available");
        let viewport = viewports.get_mut(&response.ctx.viewport_id()).unwrap();
        viewport.layers.insert(layer, viewport.pass);
    }

    pub fn show_triggers(&self, triggers: &[Trigger], appearance: &Appearance) {
        for trigger in triggers {
            self.show_with_focus_target(
                &trigger.response,
                &trigger.label,
                trigger.align,
                appearance,
                trigger.focus_target,
            );
        }
    }

    pub fn show(
        &self,
        response: &Response,
        label: &str,
        align: egui::RectAlign,
        appearance: &Appearance,
    ) {
        self.show_with_focus_target(response, label, align, appearance, None);
    }

    fn show_with_focus_target(
        &self,
        response: &Response,
        label: &str,
        align: egui::RectAlign,
        appearance: &Appearance,
        focus_target: Option<Id>,
    ) {
        match focus_target {
            Some(target) => self.is_open_with_focus_target(response, Some(target)),
            None => self.is_open(response),
        };
        self.queue(response, label, align, appearance, false);
    }
}

impl Appearance {
    #[cfg(test)]
    pub fn show_controlled(
        &self,
        response: &Response,
        label: &str,
        align: egui::RectAlign,
    ) -> Option<Rect> {
        self.render_controlled(response, label, align)
            .map(|painted| painted.bounds)
    }

    fn render_controlled(
        &self,
        response: &Response,
        label: &str,
        align: egui::RectAlign,
    ) -> Option<Painted> {
        let id = tooltip_id(&response.ctx, response.id);
        let now = response.ctx.input(|input| input.time);
        let layer = egui::LayerId::new(egui::Order::Tooltip, id);
        let visible = response
            .ctx
            .memory(|memory| memory.areas().visible_last_frame(&layer));
        let started = egui::AreaState::load(&response.ctx, id)
            .filter(|_| visible)
            .and_then(|state| state.last_became_visible_at)
            .unwrap_or(now);
        self.render(response, label, align, Motion::new(true, started))
    }

    fn render(
        &self,
        response: &Response,
        label: &str,
        align: egui::RectAlign,
        motion: Motion,
    ) -> Option<Painted> {
        let id = tooltip_id(&response.ctx, response.id);
        let expected = egui::AreaState::load(&response.ctx, id).and_then(|state| state.size);
        let clip = response.ctx.content_rect();
        let placement = Placement::new(
            response.rect,
            expected.unwrap_or(egui::Vec2::ZERO),
            clip,
            align,
            response.ctx.pixels_per_point(),
        );
        let sample = motion.sample(response.ctx.input(|input| input.time), placement.align);
        if sample.active {
            response.ctx.request_repaint();
        }
        let shown = egui::Area::new(id)
            .order(egui::Order::Tooltip)
            .info(egui::UiStackInfo::new(egui::UiKind::Tooltip))
            .sense(egui::Sense::hover())
            .interactable(true)
            .constrain(false)
            .fade_in(false)
            .fixed_pos(placement.position)
            .default_size(expected.unwrap_or(egui::Vec2::ZERO))
            .sizing_pass(expected.is_none())
            .show(&response.ctx, |ui| {
                ui.multiply_opacity(sample.opacity);
                let start = ui
                    .ctx()
                    .graphics_mut(|graphics| graphics.entry(ui.layer_id()).next_idx());
                ui.ctx().accesskit_node_builder(id, |node| {
                    node.set_role(egui::accesskit::Role::Tooltip);
                    node.set_label(label);
                });
                let frame = egui::Frame::new()
                    .fill(self.background)
                    .stroke(Stroke::new(1.0, self.border))
                    .corner_radius(RADIUS)
                    .inner_margin(egui::Margin::symmetric(
                        HORIZONTAL_PADDING,
                        VERTICAL_PADDING,
                    ))
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(label)
                                    .font(FontId::proportional(FONT_SIZE))
                                    .color(self.foreground)
                                    .line_height(Some(LINE_HEIGHT)),
                            )
                            .wrap_mode(egui::TextWrapMode::Extend)
                            .selectable(false),
                        )
                    });
                let arrow = placement.arrow(response.rect, frame.response.rect);
                if let Some(arrow) = arrow {
                    ui.painter()
                        .with_clip_rect(clip)
                        .add(arrow.shape(self.background));
                }
                let transform = sample.transform(placement.origin(frame.response.rect, arrow));
                ui.ctx().set_transform_layer(ui.layer_id(), transform);
                let body = Body {
                    bounds: transform * frame.response.rect,
                    radius: f32::from(RADIUS) * sample.scale,
                };
                let arrow = arrow.map(|arrow| arrow.transformed(transform));
                let bounds = arrow.map_or(body.bounds, |arrow| body.bounds.union(arrow.bounds()));
                let input_bounds = transform.inverse() * bounds;
                ui.ctx()
                    .set_layer_input_region(ui.layer_id(), input_bounds, move |point| {
                        let point = transform * point;
                        clip.contains(point)
                            && (body.contains(point)
                                || arrow.is_some_and(|arrow| arrow.contains(point)))
                    });
                ui.interact(input_bounds, id.with("pointer-region"), egui::Sense::CLICK);
                let text_bounds = transform * frame.inner.rect;
                ui.ctx().accesskit_node_builder(frame.inner.id, |node| {
                    node.set_bounds(egui::accesskit::Rect {
                        x0: text_bounds.left().into(),
                        y0: text_bounds.top().into(),
                        x1: text_bounds.right().into(),
                        y1: text_bounds.bottom().into(),
                    });
                });
                ui.ctx().graphics_mut(|graphics| {
                    let list = graphics.entry(ui.layer_id());
                    let end = list.next_idx();
                    for index in start.0..end.0 {
                        list.mutate_shape(egui::layers::ShapeIdx(index), |shape| {
                            shape.clip_rect = transform.inverse() * clip;
                        });
                    }
                });
                (
                    Painted {
                        bounds: body.bounds,
                        body: Some(body),
                        arrow,
                    },
                    frame.response.rect.size(),
                )
            });
        if expected.is_some_and(|size| size != shown.inner.1) {
            response.ctx.request_repaint();
        }
        response.ctx.accesskit_node_builder(id, |node| {
            node.set_role(egui::accesskit::Role::Tooltip);
            node.set_label(label);
            node.set_bounds(egui::accesskit::Rect {
                x0: shown.inner.0.bounds.left().into(),
                y0: shown.inner.0.bounds.top().into(),
                x1: shown.inner.0.bounds.right().into(),
                y1: shown.inner.0.bounds.bottom().into(),
            });
        });
        response.ctx.accesskit_node_builder(response.id, |node| {
            if node.role() == egui::accesskit::Role::Unknown {
                node.set_role(egui::accesskit::Role::GenericContainer);
            }
            node.set_bounds(egui::accesskit::Rect {
                x0: response.rect.left().into(),
                y0: response.rect.top().into(),
                x1: response.rect.right().into(),
                y1: response.rect.bottom().into(),
            });
            if motion.is_open() {
                node.set_described_by(vec![id.accesskit_id()]);
            } else {
                node.clear_described_by();
            }
        });
        Some(shown.inner.0)
    }
}

impl Provider {
    pub fn begin_frame(&self, context: &Context) {
        let id = context.viewport_id();
        let pass = context.cumulative_pass_nr();
        if self
            .0
            .lock()
            .expect("native tooltip owner is available")
            .get(&id)
            .is_some_and(|viewport| {
                viewport.context == *context && viewport.pass == pass && viewport.prepared
            })
        {
            return;
        }
        let (now, events, pointer, live, window_focused, available) = context.input(|input| {
            let mut next = 0;
            let available = input
                .events
                .iter()
                .map(|event| Context::raw_event_index(&input.raw.events, event, &mut next))
                .collect::<Vec<_>>();
            (
                input.time,
                input.raw.events.clone(),
                input.pointer.hover_pos(),
                input.raw.viewports.keys().copied().collect::<Vec<_>>(),
                input.focused,
                available,
            )
        });
        let dismissal_layers = context.dismissal_layers();
        let clicked = context.interaction_snapshot(|interaction| interaction.clicked);
        let mut viewports = self.0.lock().expect("native tooltip owner is available");
        let mut released = Vec::new();
        viewports.retain(|id, viewport| {
            if live.contains(id) {
                return true;
            }
            released.extend(
                viewport
                    .layers
                    .keys()
                    .map(|layer| (viewport.context.clone(), *layer)),
            );
            false
        });
        let viewport = viewports.entry(id).or_insert_with(|| Viewport {
            context: context.clone(),
            pass,
            prepared: false,
            now,
            events: Vec::new(),
            replayed_actions: Vec::new(),
            keyboard_closes: Vec::new(),
            dismissal_layers: Vec::new(),
            pointer,
            widgets: HashMap::new(),
            layers: HashMap::new(),
            open: None,
            open_epoch: 0,
            skip_until: None,
            is_delayed: true,
            pending: Vec::new(),
        });
        if viewport.context != *context {
            released.extend(
                viewport
                    .layers
                    .keys()
                    .map(|layer| (viewport.context.clone(), *layer)),
            );
            viewport.layers.clear();
            viewport.context = context.clone();
            viewport.widgets.clear();
            viewport.open = None;
            viewport.skip_until = None;
            viewport.is_delayed = true;
            viewport.pass = pass;
            viewport.prepared = false;
        } else if viewport.pass == pass && viewport.prepared {
            return;
        }
        viewport.pass = pass;
        viewport.prepared = true;
        viewport.pending.clear();
        viewport.now = now;
        if viewport.skip_until.is_some_and(|until| until <= now) {
            viewport.skip_until = None;
            viewport.is_delayed = true;
        }
        viewport.events = events;
        viewport.dismissal_layers = dismissal_layers;
        viewport.pointer = pointer.filter(|point| point.is_finite());
        let stale = viewport
            .widgets
            .iter()
            .filter_map(|(id, widget)| (widget.seen.saturating_add(1) < pass).then_some(*id))
            .collect::<Vec<_>>();
        for id in &stale {
            viewport.close(*id);
            viewport.widgets.remove(id);
        }
        viewport.dismissal_layers.retain(|layer| {
            viewport.widgets.iter().all(|(id, widget)| {
                widget.dismissal_id != Some(*layer)
                    || widget.controlled == Some(true)
                    || viewport.open == Some(*id)
                    || widget.motion.is_some_and(|motion| motion.is_present(now))
            })
        });
        let consumed = viewport.replay_keyboard(window_focused, clicked, &available);
        drop(viewports);
        for (context, layer) in released {
            context.unregister_dismissal_layer(layer.id);
            context.set_transform_layer(layer, egui::emath::TSTransform::IDENTITY);
        }
        for id in stale {
            context.unregister_dismissal_layer(tooltip_id(context, id));
            clear_transform(context, id);
        }
        if !consumed.is_empty() {
            context.input_mut(|input| {
                input.events = std::mem::take(&mut input.events)
                    .into_iter()
                    .enumerate()
                    .filter_map(|(index, event)| {
                        (!consumed.contains(&available[index])).then_some(event)
                    })
                    .collect();
            });
        }
    }

    pub fn finish_frame(&self, context: &Context) {
        let pending = {
            let mut viewports = self.0.lock().expect("native tooltip owner is available");
            let Some(viewport) = viewports.get_mut(&context.viewport_id()) else {
                return;
            };
            if viewport.context != *context {
                return;
            }
            std::mem::take(&mut viewport.pending)
        };
        let scrolled = pending
            .iter()
            .filter_map(|pending| {
                context
                    .widget_ancestor_scrolled(pending.response.id)
                    .then_some(pending.response.id)
            })
            .collect::<Vec<_>>();
        {
            let mut viewports = self.0.lock().expect("native tooltip owner is available");
            let viewport = viewports.get_mut(&context.viewport_id()).unwrap();
            for id in scrolled {
                let Some(widget) = viewport.widgets.get(&id) else {
                    continue;
                };
                let controlled = widget.controlled;
                let content_is_present = controlled == Some(true)
                    || viewport.open == Some(id)
                    || widget
                        .motion
                        .is_some_and(|motion| motion.is_present(viewport.now));
                if content_is_present {
                    viewport.request(id, false, controlled);
                }
            }
        }
        for pending in pending {
            self.render_pending(pending);
        }
        let mut viewports = self.0.lock().expect("native tooltip owner is available");
        let Some(viewport) = viewports.get_mut(&context.viewport_id()) else {
            return;
        };
        if viewport.context != *context {
            return;
        }
        let mut released = Vec::new();
        viewport.layers.retain(|layer, seen| {
            if *seen == viewport.pass {
                return true;
            }
            released.push(*layer);
            false
        });
        let unseen = viewport
            .widgets
            .iter()
            .filter_map(|(id, widget)| (widget.seen != viewport.pass).then_some(*id))
            .collect::<Vec<_>>();
        for id in &unseen {
            viewport.close(*id);
            viewport.widgets.remove(id);
        }
        drop(viewports);
        for layer in released {
            context.unregister_dismissal_layer(layer.id);
            context.set_transform_layer(layer, egui::emath::TSTransform::IDENTITY);
        }
        for id in unseen {
            clear_transform(context, id);
        }
    }

    pub fn is_open(&self, response: &Response) -> bool {
        self.is_open_with_focus_target(response, None)
    }

    fn is_open_with_focus_target(&self, response: &Response, focus_target: Option<Id>) -> bool {
        self.process_trigger(response, focus_target, None)
    }

    fn process_trigger(
        &self,
        response: &Response,
        focus_target: Option<Id>,
        controlled: Option<bool>,
    ) -> bool {
        self.begin_frame(&response.ctx);
        let hovered = response.contains_pointer();
        let focused = response.has_focus()
            || focus_target.is_some_and(|id| response.ctx.memory(|memory| memory.has_focus(id)));
        let was_focused = response.ctx.memory(|memory| {
            memory.had_focus_last_frame(response.id)
                || focus_target.is_some_and(|id| memory.had_focus_last_frame(id))
        });
        let window_focused = response.ctx.input(|input| input.focused);
        let pointer_is_down = response.ctx.input(|input| input.pointer.any_down());
        let completed_key_click = response
            .ctx
            .button_key_activated(focus_target.unwrap_or(response.id));
        let available_events = response.ctx.input(|input| {
            input
                .events
                .iter()
                .filter(|event| matches!(event, Event::Key { .. }))
                .cloned()
                .collect::<Vec<_>>()
        });
        let dismissal_id = tooltip_id(&response.ctx, response.id);
        let mut viewports = self.0.lock().expect("native tooltip owner is available");
        let viewport = viewports.get_mut(&response.ctx.viewport_id()).unwrap();
        if !response.enabled() {
            viewport.close(response.id);
            viewport.widgets.insert(
                response.id,
                Widget {
                    seen: viewport.pass,
                    trigger_rect: Some(response.rect),
                    focus_target,
                    dismissal_id: Some(dismissal_id),
                    controlled,
                    ..Default::default()
                },
            );
            return false;
        }
        let available_keys = viewport.available_key_indices(&available_events);
        let can_keyboard_activate = response.sense.senses_click() || focus_target.is_some();
        if can_keyboard_activate && completed_key_click.is_none() {
            viewport.apply_keyboard_close(response.id, &available_keys);
        }
        let mut widget = viewport.widgets.remove(&response.id).unwrap_or_default();
        widget.can_keyboard_activate = can_keyboard_activate;
        if !can_keyboard_activate {
            widget.space_armed = None;
        }
        let replayed_focus = viewport.events.iter().enumerate().any(|(index, event)| {
            viewport.replayed_actions.contains(&index)
                && matches!(event, Event::AccessKitActionRequest(request)
                    if request.target_tree == egui::accesskit::TreeId::ROOT
                        && request.action == egui::accesskit::Action::Focus)
        });
        let focused = if replayed_focus {
            widget.focused
        } else {
            focused
        };
        widget.seen = viewport.pass;
        widget.enabled = true;
        widget.trigger_rect = Some(response.rect);
        widget.focus_target = focus_target;
        widget.dismissal_id = Some(dismissal_id);
        widget.controlled = controlled;
        let closing_content = widget
            .motion
            .is_some_and(|motion| !motion.is_open() && motion.is_present(viewport.now));
        let moved = viewport
            .events
            .iter()
            .any(|event| matches!(event, Event::PointerMoved(_)));
        let touched = viewport
            .events
            .iter()
            .any(|event| matches!(event, Event::Touch { .. }));
        if !pointer_is_down {
            widget.pointer_down = false;
        }
        for event in &viewport.events {
            if let Event::PointerButton { pos, pressed, .. } = event {
                if !pressed {
                    widget.pointer_down = false;
                } else if response.rect.contains(*pos) {
                    widget.pointer_down = true;
                }
            }
        }
        let pointer_focus = widget.pointer_down || viewport.events.iter().enumerate().any(|(index, event)|
            !(widget.pointer_replayed && viewport.replayed_actions.contains(&index))
            && matches!(event, Event::PointerButton { pos, pressed: true, .. } if response.rect.contains(*pos))
        );
        if !focused {
            widget.space_armed = None;
        }
        let has_focus_requests = viewport.events.iter().enumerate().any(|(index, event)| {
            if viewport.replayed_actions.contains(&index) {
                return false;
            }
            matches!(event,
            Event::AccessKitActionRequest(request)
                if request.target_tree == egui::accesskit::TreeId::ROOT
                    && request.action == egui::accesskit::Action::Focus)
        });
        let mut event_focus = if has_focus_requests {
            was_focused
        } else {
            focused
        };
        let click_replayed = viewport.events.iter().enumerate().any(|(index, event)| {
            viewport.replayed_actions.contains(&index)
                && match event {
                    Event::PointerButton {
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        ..
                    } => widget.pointer_replayed,
                    Event::AccessKitActionRequest(request) => {
                        request.target_tree == egui::accesskit::TreeId::ROOT
                            && request.action == egui::accesskit::Action::Click
                            && (request.target_node == response.id.accesskit_id()
                                || focus_target
                                    .is_some_and(|id| request.target_node == id.accesskit_id()))
                    }
                    _ => false,
                }
        });
        let mut focus_action = ((response.clicked_by(egui::PointerButton::Primary)
            && !click_replayed)
            || completed_key_click == Some(true))
        .then_some(false);
        for (index, event) in viewport.events.iter().enumerate() {
            if viewport.replayed_actions.contains(&index) {
                continue;
            }
            if matches!(event, Event::Key { .. }) && !available_keys.contains(&index) {
                continue;
            }
            match event {
                Event::AccessKitActionRequest(request)
                    if request.target_tree == egui::accesskit::TreeId::ROOT =>
                {
                    if request.action == egui::accesskit::Action::Focus {
                        let next = (request.target_node == response.id.accesskit_id()
                            || focus_target
                                .is_some_and(|id| request.target_node == id.accesskit_id()))
                            && response.enabled()
                            && window_focused;
                        if next != event_focus {
                            widget.space_armed = None;
                            if !next || !pointer_focus {
                                focus_action = Some(next);
                            }
                        }
                        event_focus = next;
                    } else if (request.target_node == response.id.accesskit_id()
                        || focus_target.is_some_and(|id| request.target_node == id.accesskit_id()))
                        && request.action == egui::accesskit::Action::Click
                    {
                        focus_action = Some(false);
                    }
                }
                Event::Key {
                    key: egui::Key::Enter,
                    pressed: true,
                    ..
                } if event_focus
                    && can_keyboard_activate
                    && completed_key_click.is_none()
                    && controlled.is_none() =>
                {
                    focus_action = Some(false);
                }
                Event::Key {
                    key: egui::Key::Space,
                    pressed: true,
                    repeat: false,
                    ..
                } if event_focus
                    && can_keyboard_activate
                    && completed_key_click.is_none()
                    && controlled.is_none() =>
                {
                    widget.space_armed = Some((viewport.pass, index));
                }
                Event::Key {
                    key: egui::Key::Space,
                    pressed: false,
                    ..
                } if event_focus
                    && can_keyboard_activate
                    && completed_key_click.is_none()
                    && controlled.is_none() =>
                {
                    if widget.space_armed.is_some() {
                        focus_action = Some(false);
                    }
                    widget.space_armed = None;
                }
                _ => {}
            }
        }
        let content_hovered = viewport
            .pointer
            .is_some_and(|point| widget.contains_content(point));
        if widget.hovered && !hovered {
            widget.deadline = None;
            widget.pointer_opened = false;
            if controlled.unwrap_or(viewport.open == Some(response.id))
                && let Some((point, content)) = viewport.pointer.zip(widget.content)
            {
                widget.grace = Some(Grace::new(point, response.rect, content));
            }
        }
        if widget.content_hovered
            && !content_hovered
            && controlled.unwrap_or(viewport.open == Some(response.id))
            && let Some((point, content)) = viewport.pointer.zip(widget.content)
        {
            widget.grace = Some(Grace::new(point, content, response.rect));
        }
        if hovered || content_hovered {
            widget.grace = None;
        }
        if moved
            && widget
                .grace
                .as_ref()
                .is_some_and(|grace| viewport.pointer.is_none_or(|point| !grace.contains(point)))
        {
            viewport.request(response.id, false, controlled);
            widget.grace = None;
        }
        let pointer_down = viewport.events.iter().enumerate().any(|(index, event)| {
            !(widget.pointer_replayed && viewport.replayed_actions.contains(&index))
                && matches!(event, Event::PointerButton { pos, pressed: true, .. }
                if !widget.contains_content(*pos))
        });
        if focus_action == Some(false)
            || (pointer_down && controlled.unwrap_or(viewport.open == Some(response.id)))
            || (widget.focused && !focused)
        {
            viewport.request(response.id, false, controlled);
            widget.deadline = None;
            widget.grace = None;
        } else if focus_action == Some(true)
            || (focus_action.is_none() && focused && !widget.focused && !pointer_focus)
        {
            widget.deadline = None;
            viewport.request_open(response.id, controlled, closing_content);
        } else if hovered
            && moved
            && !touched
            && !widget.pointer_opened
            && !viewport
                .widgets
                .values()
                .any(|widget| widget.grace.is_some())
        {
            widget.pointer_opened = true;
            if !viewport.is_delayed {
                viewport.request_open(response.id, controlled, closing_content);
            } else {
                widget.deadline = Some(viewport.now + DELAY_SECONDS);
            }
        }
        if widget
            .deadline
            .is_some_and(|deadline| deadline <= viewport.now)
        {
            widget.deadline = None;
            viewport.request_open(response.id, controlled, closing_content);
        }
        widget.hovered = hovered;
        widget.focused = focused;
        widget.content_hovered = content_hovered;
        let delay = widget
            .deadline
            .map(|deadline| (deadline - viewport.now).max(0.0));
        let open = controlled.unwrap_or(viewport.open == Some(response.id));
        viewport.widgets.insert(response.id, widget);
        drop(viewports);
        if let Some(delay) = delay {
            response.ctx.request_repaint_after_secs(delay as f32);
        }
        open
    }

    pub fn content(&self, response: &Response, content: Option<Rect>) {
        self.content_with_arrow(
            response,
            content.map(|bounds| Painted {
                bounds,
                body: None,
                arrow: None,
            }),
        );
    }

    fn content_with_arrow(&self, response: &Response, content: Option<Painted>) {
        let removed = {
            let mut viewports = self.0.lock().expect("native tooltip owner is available");
            let Some(widget) = viewports
                .get_mut(&response.ctx.viewport_id())
                .and_then(|viewport| viewport.widgets.get_mut(&response.id))
            else {
                return;
            };
            let removed = widget.content.is_some() && content.is_none();
            widget.content = content.map(|content| content.bounds);
            widget.body = content.and_then(|content| content.body);
            widget.arrow = content.and_then(|content| content.arrow);
            removed
        };
        if removed {
            clear_transform(&response.ctx, response.id);
        }
    }
}

fn clear_transform(context: &Context, id: Id) {
    context.set_transform_layer(
        egui::LayerId::new(egui::Order::Tooltip, tooltip_id(context, id)),
        egui::emath::TSTransform::IDENTITY,
    );
}

fn tooltip_id(context: &Context, trigger: Id) -> Id {
    let viewport = context.viewport_id();
    let anchor = if viewport == ViewportId::ROOT {
        trigger
    } else {
        trigger.with(viewport)
    };
    egui::Tooltip::next_tooltip_id(context, anchor)
}

#[cfg(test)]
mod tests {
    include!("../../taide-native-app/src/tooltips-tests.rs");
}

use std::collections::HashSet;
use std::ops::Range;
use std::sync::Arc;
use std::time::Duration;

use egui::{Event, FontId, Id, Key, Modifiers, Rect, Ui, Vec2, pos2, vec2};
use taide_native_editor::document::{DocumentId, EditorError};
use taide_native_editor::documentation::{
    Command, Kind, RichDocument, SignatureCommand, SignatureTriggers, Signatures,
};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{SelectionSet, ViewId};

use crate::editor_geometry::EditorGeometry;
use crate::editor_surface::EditorAppearance;

use crate::editor_markup as markup;
pub use crate::editor_markup::Colors;

const HOVER_REQUEST_DELAY: f64 = 0.150;
const HOVER_VISIBLE_DELAY: f64 = 0.300;
const HOVER_HIDE_DELAY: f64 = 0.300;
const HOVER_LOADING_DELAY: f64 = 0.900;
const SIGNATURE_DELAY: f64 = 0.120;
const HOVER_MINIMUM_MAX_WIDTH: f32 = 750.0;
const HOVER_EDITOR_WIDTH_RATIO: f32 = 0.66;
const RESIZE_HORIZONTAL_PADDING: f32 = 14.0;
const SIGNATURE_WIDTH: f32 = 440.0;
const MINIMUM_MAX_HEIGHT: f32 = 250.0;
const HEIGHT_DIVISOR: f32 = 4.0;
const BORDER_WIDTH: f32 = 1.0;
const PADDING: i8 = 5;
const CORNER_RADIUS: u8 = 6;
const BUTTON_SIZE: f32 = 22.0;
const MINIMUM_HOVER_SIZE: f32 = 10.0;
const RESIZE_HANDLE_WIDTH: f32 = 4.0;
const SCROLL_PAGE_FRACTION: f32 = 0.9;
const HOVER_FILTER: egui::EventFilter = egui::EventFilter {
    tab: true,
    horizontal_arrows: true,
    vertical_arrows: true,
    escape: true,
};

#[derive(Clone)]
pub struct Part {
    pub range: Range<usize>,
    pub documents: Vec<Arc<RichDocument>>,
}

#[derive(Clone)]
pub enum Content {
    Hover(Vec<Part>),
    Signature {
        model: Arc<Signatures>,
        parameter: Option<Arc<RichDocument>>,
        documentation: Option<Arc<RichDocument>>,
    },
}

#[derive(Clone)]
pub struct Widget {
    pub token: String,
    pub byte: usize,
    pub keyboard: bool,
    pub pending: bool,
    pub content: Option<Content>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Geometry {
    pub hover: Option<Rect>,
    pub signature: Option<Rect>,
}

pub trait Provider {
    fn available(&self, store: &EditorStore, view: ViewId, kind: Kind) -> bool;
    fn version(&self, store: &EditorStore, view: ViewId) -> String;
    fn triggers(&self, store: &EditorStore, view: ViewId) -> SignatureTriggers;
    fn word(&self, store: &EditorStore, view: ViewId, byte: usize) -> Option<Range<usize>>;
    fn request(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        kind: Kind,
        byte: usize,
        keyboard: bool,
    ) -> Result<bool, EditorError>;
    fn current(&self, store: &EditorStore, view: ViewId, kind: Kind) -> Option<Widget>;
    fn close(&mut self, view: ViewId, kind: Kind);
    fn cycle(&mut self, store: &EditorStore, view: ViewId, forward: bool) -> bool;
    fn open_link(&mut self, ui: &Ui, target: &str) -> bool;
    fn code(&mut self, _ui: &Ui, _language: &str, _text: &str) -> Option<egui::text::LayoutJob> {
        None
    }
    fn image(
        &mut self,
        _ui: &mut Ui,
        _source: &str,
        _alt: &str,
        _dimensions: taide_native_editor::documentation::ImageDimensions,
    ) -> Option<egui::Response> {
        None
    }
}

#[derive(Clone)]
struct Scene {
    document: DocumentId,
    revision: u64,
    language: String,
    selection: SelectionSet,
    providers: String,
    geometry: EditorGeometry,
}

#[derive(Clone)]
struct Hover {
    byte: usize,
    range: Range<usize>,
    started: f64,
    requested: bool,
    keyboard: bool,
    immediate: bool,
    hide_at: Option<f64>,
}

#[derive(Default, Clone)]
pub(crate) struct State {
    scene: Option<Scene>,
    hover: Option<Hover>,
    hover_rect: Option<Rect>,
    hover_focus: Option<Id>,
    focus_requested: bool,
    hover_scroll: Vec2,
    signature_due: Option<f64>,
    signature: Option<Widget>,
    signature_rect: Option<Rect>,
    signature_scroll: Vec2,
}

#[derive(Default)]
pub(crate) struct Input {
    pub consumed: HashSet<usize>,
    pub release_at: Option<usize>,
    pub release_from_start: bool,
}

impl State {
    #[cfg(feature = "inspection")]
    pub(crate) fn geometry(&self) -> Geometry {
        Geometry {
            hover: self.hover_rect,
            signature: self.signature_rect,
        }
    }
    fn close_hover(
        &mut self,
        ui: &Ui,
        body: Id,
        provider: &mut dyn Provider,
        view: ViewId,
    ) -> bool {
        let restore = self
            .hover_focus
            .is_some_and(|id| ui.memory(|memory| memory.has_focus(id)));
        if restore {
            ui.memory_mut(|memory| memory.request_focus_with_filter(body, HOVER_FILTER));
        }
        provider.close(view, Kind::Hover);
        self.hover = None;
        self.hover_rect = None;
        self.hover_scroll = Vec2::ZERO;
        self.focus_requested = false;
        self.hover_focus = None;
        restore
    }

    fn close_signature(&mut self, provider: &mut dyn Provider, view: ViewId) {
        provider.close(view, Kind::Signature);
        self.signature = None;
        self.signature_due = None;
        self.signature_rect = None;
        self.signature_scroll = Vec2::ZERO;
    }

    pub(crate) fn contains_pointer(&self, ui: &Ui) -> bool {
        ui.input(|input| input.pointer.hover_pos())
            .is_some_and(|position| {
                self.hover_rect.is_some_and(|rect| rect.contains(position))
                    || self
                        .signature_rect
                        .is_some_and(|rect| rect.contains(position))
            })
    }

    pub(crate) fn command(
        &mut self,
        ui: &Ui,
        store: &EditorStore,
        view: ViewId,
        provider: &mut dyn Provider,
        command: Command,
    ) -> Result<bool, EditorError> {
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        if current.composition.is_some() || !ui.is_enabled() {
            return Ok(false);
        }
        let byte = current.selection.selections[current.selection.primary].head;
        match command {
            Command::ShowHover => {
                if self.hover_rect.is_some() {
                    self.focus_requested = true;
                    if let Some(hover) = &mut self.hover {
                        hover.keyboard = true;
                    }
                    return Ok(true);
                }
                if !provider.request(store, view, Kind::Hover, byte, true)? {
                    return Ok(false);
                }
                let time = ui.input(|input| input.time);
                self.hover = Some(Hover {
                    byte,
                    range: provider.word(store, view, byte).unwrap_or(byte..byte),
                    started: time,
                    requested: true,
                    keyboard: true,
                    immediate: true,
                    hide_at: None,
                });
                Ok(true)
            }
            Command::Signature(SignatureCommand::Trigger) => {
                self.signature_due = None;
                provider.request(store, view, Kind::Signature, byte, true)
            }
            Command::Signature(SignatureCommand::Close) => {
                self.close_signature(provider, view);
                Ok(true)
            }
            Command::Signature(SignatureCommand::Previous | SignatureCommand::Next) => Ok(provider
                .cycle(
                    store,
                    view,
                    command == Command::Signature(SignatureCommand::Next),
                )),
        }
    }

    pub(crate) fn input(
        &mut self,
        ui: &Ui,
        store: &EditorStore,
        view: ViewId,
        body: Id,
        provider: &mut dyn Provider,
        commands: &[Command],
    ) -> Result<Input, EditorError> {
        let mut result = Input::default();
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        let document = store.documents().snapshot(current.document)?;
        let changed_owner = self.scene.as_ref().is_some_and(|scene| {
            scene.document != document.id
                || scene.language != document.metadata.language_id
                || scene.providers != provider.version(store, view)
        });
        if changed_owner || !ui.is_enabled() || current.composition.is_some() {
            result.release_from_start |= self.close_hover(ui, body, provider, view);
            self.close_signature(provider, view);
        }
        if self.scene.as_ref().is_some_and(|scene| {
            scene.revision != document.revision || scene.selection != current.selection
        }) {
            result.release_from_start |= self.close_hover(ui, body, provider, view);
            if self.signature.is_some() {
                self.signature_due = Some(ui.input(|input| input.time) + SIGNATURE_DELAY);
            }
        }
        for command in commands {
            self.command(ui, store, view, provider, *command)?;
        }
        let events = ui.input(|input| input.raw.events.clone());
        let hover_route = self
            .hover_focus
            .and_then(|id| ui.ctx().keyboard_input_route(id));
        let hover_start = self.hover_focus == ui.ctx().keyboard_focus_before_events();
        let body_route = ui.ctx().keyboard_input_route(body);
        let body_start = Some(body) == ui.ctx().keyboard_focus_before_events();
        let mut released = result.release_from_start;
        for (index, event) in events.iter().enumerate() {
            let hover_owned = !released
                && self.hover_rect.is_some()
                && hover_route
                    .as_ref()
                    .and_then(|route| route.0.get(index))
                    .and_then(|(owned, _)| *owned)
                    .unwrap_or(hover_start);
            let body_owned = released
                || body_route
                    .as_ref()
                    .and_then(|route| route.0.get(index))
                    .and_then(|(owned, _)| *owned)
                    .unwrap_or(body_start);
            if matches!(event, Event::WindowFocused(false)) {
                self.close_hover(ui, body, provider, view);
                self.close_signature(provider, view);
            }
            if let Event::PointerButton {
                pos, pressed: true, ..
            } = event
            {
                if self.hover_rect.is_some_and(|rect| rect.contains(*pos)) {
                    continue;
                }
                self.close_hover(ui, body, provider, view);
                if self.signature_rect.is_some_and(|rect| rect.contains(*pos)) {
                    continue;
                }
                self.close_signature(provider, view);
            }
            if matches!(event, Event::MouseWheel { .. }) && !self.contains_pointer(ui) {
                self.close_hover(ui, body, provider, view);
            }
            if hover_owned
                && let Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } = event
            {
                if *key == Key::Escape {
                    self.close_hover(ui, body, provider, view);
                    ui.memory_mut(|memory| memory.request_focus_with_filter(body, HOVER_FILTER));
                    result.release_at = Some(index);
                    released = true;
                } else {
                    let step = self
                        .scene
                        .as_ref()
                        .map_or(0.0, |scene| scene.geometry.line_height);
                    let page = self
                        .hover_rect
                        .map_or(0.0, |rect| rect.height() * SCROLL_PAGE_FRACTION);
                    match key {
                        Key::ArrowUp => self.hover_scroll.y = (self.hover_scroll.y - step).max(0.0),
                        Key::ArrowDown => self.hover_scroll.y += step,
                        Key::PageUp => self.hover_scroll.y = (self.hover_scroll.y - page).max(0.0),
                        Key::PageDown => self.hover_scroll.y += page,
                        Key::ArrowLeft => {
                            self.hover_scroll.x = (self.hover_scroll.x - step).max(0.0)
                        }
                        Key::ArrowRight => self.hover_scroll.x += step,
                        Key::Home if modifiers.command || modifiers.ctrl => {
                            self.hover_scroll = Vec2::ZERO
                        }
                        Key::End if modifiers.command || modifiers.ctrl => {
                            self.hover_scroll.y = f32::MAX
                        }
                        _ => continue,
                    }
                }
                result.consumed.insert(index);
                ui.ctx().request_repaint();
                continue;
            }
            if body_owned
                && matches!(
                    event,
                    Event::Text(_)
                        | Event::Paste(_)
                        | Event::Ime(_)
                        | Event::Key { pressed: true, .. }
                )
            {
                self.close_hover(ui, body, provider, view);
            }
            if !body_owned && ui.ctx().keyboard_focus_request_at(index).is_some() && !hover_owned {
                self.close_hover(ui, body, provider, view);
                self.close_signature(provider, view);
            }
        }
        Ok(result)
    }

    pub(crate) fn event(
        &mut self,
        ui: &Ui,
        store: &EditorStore,
        view: ViewId,
        provider: &mut dyn Provider,
        event: &Event,
    ) -> Result<bool, EditorError> {
        if store
            .views()
            .get(view)
            .is_some_and(|view| view.composition.is_none())
            && self.signature.is_some()
            && let Some(command) =
                signature_shortcut(event, ui.ctx().os().is_mac(), self.signature.as_ref())
        {
            self.command(ui, store, view, provider, Command::Signature(command))?;
            ui.ctx().request_repaint();
            return Ok(true);
        }
        Ok(false)
    }

    pub(crate) fn after_event(
        &mut self,
        ui: &Ui,
        store: &EditorStore,
        view: ViewId,
        provider: &mut dyn Provider,
        event: &Event,
        changed: bool,
    ) {
        let active = self.signature.is_some()
            || self.signature_due.is_some()
            || provider.current(store, view, Kind::Signature).is_some();
        let trigger = match event {
            Event::Text(text) => provider.triggers(store, view).matches(text, active),
            Event::Ime(egui::ImeEvent::Commit(text)) => {
                provider.triggers(store, view).matches(text, active)
            }
            _ => false,
        };
        if (trigger || active && changed) && provider.available(store, view, Kind::Signature) {
            self.signature_due = Some(ui.input(|input| input.time) + SIGNATURE_DELAY);
            ui.ctx()
                .request_repaint_after(Duration::from_secs_f64(SIGNATURE_DELAY));
        }
    }

    pub(crate) fn paint(
        &mut self,
        ui: &Ui,
        store: &EditorStore,
        view: ViewId,
        body: Id,
        geometry: &EditorGeometry,
        appearance: &EditorAppearance,
        colors: Colors,
        provider: &mut dyn Provider,
        definition_active: bool,
    ) -> Result<Vec<Id>, EditorError> {
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        let document = store.documents().snapshot(current.document)?;
        let time = ui.input(|input| input.time);
        if self.scene.as_ref().is_some_and(|scene| {
            scene.geometry.scroll != geometry.scroll || scene.geometry.rect != geometry.rect
        }) {
            self.close_hover(ui, body, provider, view);
        }
        if let Some(due) = self.signature_due {
            if time >= due && current.composition.is_none() {
                let byte = current.selection.selections[current.selection.primary].head;
                provider.request(store, view, Kind::Signature, byte, false)?;
                self.signature_due = None;
            } else {
                ui.ctx()
                    .request_repaint_after(Duration::from_secs_f64((due - time).max(0.0)));
            }
        }
        if let Some(widget) = provider.current(store, view, Kind::Signature) {
            if widget.content.is_some() || !widget.pending {
                self.signature = widget.content.as_ref().map(|_| widget.clone());
            }
        }
        if !provider.available(store, view, Kind::Signature) {
            self.close_signature(provider, view);
        }
        let modifiers = ui.input(|input| input.modifiers);
        let definition_key = if ui.ctx().os().is_mac() {
            modifiers.mac_cmd
        } else {
            modifiers.ctrl
        };
        if definition_active
            || definition_key && self.hover.as_ref().is_none_or(|hover| !hover.keyboard)
            || !provider.available(store, view, Kind::Hover)
        {
            self.close_hover(ui, body, provider, view);
        }
        let popup_hovered = self.contains_pointer(ui)
            || self.hover_focus.is_some_and(|id| {
                ui.memory(|memory| memory.has_focus(id))
                    || [true, false]
                        .into_iter()
                        .any(|edge| ui.ctx().dragged_id() == Some(id.with(("resize", edge))))
            });
        let pointer_byte = ui
            .input(|input| input.pointer.hover_pos())
            .and_then(|position| geometry.content_byte_at(position));
        if !definition_active
            && !definition_key
            && provider.available(store, view, Kind::Hover)
            && current.composition.is_none()
            && ui.is_enabled()
            && !popup_hovered
        {
            if let Some(byte) = pointer_byte {
                let in_current = self
                    .hover
                    .as_ref()
                    .is_some_and(|hover| hover.keyboard || hover.range.contains(&byte));
                if !in_current {
                    self.close_hover(ui, body, provider, view);
                    self.hover = Some(Hover {
                        byte,
                        range: provider.word(store, view, byte).unwrap_or(byte..byte),
                        started: time,
                        requested: false,
                        keyboard: false,
                        immediate: false,
                        hide_at: None,
                    });
                } else if let Some(hover) = &mut self.hover {
                    hover.hide_at = None;
                }
            } else if let Some(hover) = &mut self.hover
                && !hover.keyboard
            {
                let deadline = *hover.hide_at.get_or_insert(time + HOVER_HIDE_DELAY);
                if time >= deadline {
                    self.close_hover(ui, body, provider, view);
                } else {
                    ui.ctx()
                        .request_repaint_after(Duration::from_secs_f64(deadline - time));
                }
            }
        }
        if popup_hovered && let Some(hover) = &mut self.hover {
            hover.hide_at = None;
        }
        if let Some(hover) = &mut self.hover
            && !hover.requested
        {
            let due = hover.started + HOVER_REQUEST_DELAY;
            if time >= due {
                hover.requested =
                    provider.request(store, view, Kind::Hover, hover.byte, hover.keyboard)?;
            } else {
                ui.ctx()
                    .request_repaint_after(Duration::from_secs_f64(due - time));
            }
        }
        let mut focus_ids = Vec::new();
        if let Some(hover) = &self.hover {
            let visible = hover.started
                + if hover.immediate {
                    0.0
                } else {
                    HOVER_VISIBLE_DELAY
                };
            if time < visible {
                ui.ctx()
                    .request_repaint_after(Duration::from_secs_f64(visible - time));
            } else if let Some(widget) = provider.current(store, view, Kind::Hover) {
                let has_contents =
                    matches!(&widget.content, Some(Content::Hover(parts)) if !parts.is_empty());
                let loading = widget.pending && time >= hover.started + HOVER_LOADING_DELAY;
                if has_contents || loading {
                    let id = body.with("documentation-hover");
                    self.hover_focus = Some(id);
                    let width = (geometry.rect.width() * HOVER_EDITOR_WIDTH_RATIO)
                        .max(HOVER_MINIMUM_MAX_WIDTH);
                    let (rect, offset) = popup(
                        ui,
                        store,
                        view,
                        geometry,
                        hover.byte,
                        id,
                        width,
                        appearance,
                        colors,
                        &widget,
                        self.hover_scroll,
                        loading,
                        true,
                        provider,
                    );
                    self.hover_rect = rect;
                    self.hover_scroll = offset;
                    if let Some(rect) = rect {
                        ui.ctx()
                            .layer_painter(egui::LayerId::new(egui::Order::Tooltip, id))
                            .rect_stroke(
                                rect,
                                CORNER_RADIUS,
                                egui::Stroke::new(BORDER_WIDTH, colors.border),
                                egui::StrokeKind::Inside,
                            );
                        if std::mem::take(&mut self.focus_requested) {
                            ui.memory_mut(|memory| {
                                memory.request_focus_with_filter(id, HOVER_FILTER)
                            });
                        }
                        focus_ids.push(id);
                    }
                } else if widget.pending {
                    ui.ctx().request_repaint_after(Duration::from_secs_f64(
                        (hover.started + HOVER_LOADING_DELAY - time).max(0.0),
                    ));
                } else {
                    self.hover_rect = None;
                }
            }
        }
        if let Some(signature) = &self.signature {
            let byte = current.selection.selections[current.selection.primary].head;
            let (rect, offset) = popup(
                ui,
                store,
                view,
                geometry,
                byte,
                body.with("documentation-signature"),
                SIGNATURE_WIDTH,
                appearance,
                colors,
                signature,
                self.signature_scroll,
                false,
                false,
                provider,
            );
            self.signature_rect = rect;
            self.signature_scroll = offset;
        } else {
            self.signature_rect = None;
        }
        self.scene = Some(Scene {
            document: document.id,
            revision: document.revision,
            language: document.metadata.language_id.clone(),
            selection: current.selection.clone(),
            providers: provider.version(store, view),
            geometry: geometry.clone(),
        });
        Ok(focus_ids)
    }
}

fn signature_shortcut(
    event: &Event,
    mac: bool,
    widget: Option<&Widget>,
) -> Option<SignatureCommand> {
    let Event::Key {
        key,
        pressed: true,
        modifiers,
        ..
    } = event
    else {
        return None;
    };
    let plain = !modifiers.command && !modifiers.ctrl && !modifiers.mac_cmd && !modifiers.shift;
    if *key == Key::Escape
        && !modifiers.command
        && !modifiers.ctrl
        && !modifiers.mac_cmd
        && !modifiers.alt
    {
        return Some(SignatureCommand::Close);
    }
    let multiple = matches!(widget.and_then(|widget| widget.content.as_ref()), Some(Content::Signature { model, .. }) if model.value().signatures.len() > 1);
    if !multiple {
        return None;
    }
    match key {
        Key::ArrowUp if plain => Some(SignatureCommand::Previous),
        Key::ArrowDown if plain => Some(SignatureCommand::Next),
        Key::P if mac && *modifiers == Modifiers::CTRL => Some(SignatureCommand::Previous),
        Key::N if mac && *modifiers == Modifiers::CTRL => Some(SignatureCommand::Next),
        _ => None,
    }
}

fn popup(
    ui: &Ui,
    store: &EditorStore,
    view: ViewId,
    geometry: &EditorGeometry,
    byte: usize,
    id: Id,
    width: f32,
    appearance: &EditorAppearance,
    colors: Colors,
    widget: &Widget,
    scroll: Vec2,
    loading: bool,
    resizable: bool,
    provider: &mut dyn Provider,
) -> (Option<Rect>, Vec2) {
    let Some(anchor) = geometry.caret_rect(byte) else {
        return (None, scroll);
    };
    let bounds = ui.ctx().content_rect();
    let height = (geometry.rect.height() / HEIGHT_DIVISOR)
        .max(MINIMUM_MAX_HEIGHT)
        .min(bounds.height());
    let manual = resizable
        .then(|| {
            ui.ctx()
                .data(|data| data.get_temp::<Vec2>(id.with("manual-size")))
        })
        .flatten();
    let size = manual.unwrap_or_else(|| {
        ui.ctx()
            .data(|data| data.get_temp::<Vec2>(id.with("size")))
            .unwrap_or(vec2(width, appearance.line_height))
    });
    let above = anchor.top() - size.y >= bounds.top();
    let y = if above {
        anchor.top() - size.y
    } else {
        anchor.bottom()
    };
    let position = pos2(
        anchor
            .left()
            .min(bounds.right() - size.x)
            .max(bounds.left()),
        y.min(bounds.bottom() - size.y).max(bounds.top()),
    );
    let mut offset = scroll;
    let mut maximum = bounds.size();
    let output = egui::Area::new(id)
        .fixed_pos(position)
        .order(egui::Order::Tooltip)
        .movable(false)
        .show(ui.ctx(), |ui| {
            let frame_padding = (f32::from(PADDING) + BORDER_WIDTH) * 2.0;
            let content_width = manual
                .map_or(width, |size| size.x - frame_padding)
                .min(bounds.width() - frame_padding)
                .max(0.0);
            ui.set_max_width(content_width);
            ui.style_mut().override_font_id = Some(FontId::proportional(appearance.font.size));
            ui.visuals_mut().override_text_color = Some(colors.foreground);
            ui.spacing_mut().item_spacing = vec2(0.0, PADDING as f32);
            egui::Frame::new()
                .fill(colors.background)
                .stroke(egui::Stroke::new(BORDER_WIDTH, colors.border))
                .corner_radius(CORNER_RADIUS)
                .inner_margin(PADDING)
                .shadow(ui.visuals().popup_shadow)
                .show(ui, |ui| {
                    if manual.is_some() {
                        ui.set_width(content_width);
                    }
                    let content_height = manual
                        .map_or(height, |size| size.y - frame_padding)
                        .min(bounds.height() - frame_padding)
                        .max(0.0);
                    let mut unwrapped_width = 0.0f32;
                    let output = egui::ScrollArea::both()
                        .id_salt(id.with("scroll"))
                        .max_height(content_height)
                        .auto_shrink([manual.is_none(), true])
                        .scroll_offset(scroll)
                        .show(ui, |ui| {
                            match &widget.content {
                                Some(Content::Hover(parts)) => {
                                    for (index, part) in parts.iter().enumerate() {
                                        if index > 0 {
                                            ui.separator();
                                        }
                                        for document in &part.documents {
                                            unwrapped_width =
                                                unwrapped_width.max(markup::unwrapped_width(
                                                    ui, document, appearance, provider,
                                                ));
                                            markup::show(
                                                ui, document, appearance, colors, provider,
                                            );
                                        }
                                    }
                                }
                                Some(Content::Signature {
                                    model,
                                    parameter,
                                    documentation,
                                }) => {
                                    let mut job = egui::text::LayoutJob::default();
                                    let label = &model.active().label;
                                    let parameter_range = model.parameter_range();
                                    let normal = egui::TextFormat {
                                        font_id: appearance.font.clone(),
                                        color: colors.foreground,
                                        line_height: Some(appearance.line_height),
                                        ..Default::default()
                                    };
                                    if let Some(range) = parameter_range {
                                        job.append(&label[..range.start], 0.0, normal.clone());
                                        let bold = FontId::new(
                                            appearance.font.size,
                                            egui::FontFamily::Name(
                                                crate::font_families::EDITOR_BOLD_FAMILY.into(),
                                            ),
                                        );
                                        job.append(
                                            &label[range.clone()],
                                            0.0,
                                            egui::TextFormat {
                                                font_id: bold,
                                                color: colors.highlight,
                                                ..normal.clone()
                                            },
                                        );
                                        job.append(&label[range.end..], 0.0, normal);
                                    } else {
                                        job.append(label, 0.0, normal);
                                    }
                                    ui.horizontal_top(|ui| {
                                        if model.value().signatures.len() > 1 {
                                            ui.vertical(|ui| {
                                                if signature_button(
                                                    ui,
                                                    id.with("previous"),
                                                    false,
                                                    colors,
                                                ) {
                                                    provider.cycle(store, view, false);
                                                    ui.ctx().request_repaint();
                                                }
                                                ui.label(format!("{}", model.index() + 1));
                                                if signature_button(
                                                    ui,
                                                    id.with("next"),
                                                    true,
                                                    colors,
                                                ) {
                                                    provider.cycle(store, view, true);
                                                    ui.ctx().request_repaint();
                                                }
                                            });
                                            ui.separator();
                                        }
                                        ui.vertical(|ui| {
                                            ui.add(egui::Label::new(job).selectable(true));
                                            if parameter.is_some() || documentation.is_some() {
                                                ui.separator();
                                            }
                                            if let Some(parameter) = parameter {
                                                markup::show(
                                                    ui, parameter, appearance, colors, provider,
                                                );
                                            }
                                            if let Some(documentation) = documentation {
                                                markup::show(
                                                    ui,
                                                    documentation,
                                                    appearance,
                                                    colors,
                                                    provider,
                                                );
                                            }
                                        });
                                    });
                                }
                                None => {}
                            }
                            if loading {
                                ui.label("Loading...");
                            }
                        });
                    offset = output.state.offset;
                    let available_height = if above {
                        anchor.top() - bounds.top()
                    } else {
                        bounds.bottom() - anchor.bottom()
                    };
                    maximum.y = (output.content_size.y + frame_padding).min(available_height);
                    maximum.x = if unwrapped_width + frame_padding > width
                        || output.content_size.x > output.inner_rect.width()
                    {
                        bounds.width() - RESIZE_HORIZONTAL_PADDING
                    } else {
                        (unwrapped_width + frame_padding).max(output.content_size.x + frame_padding)
                    };
                });
            ui.interact(ui.min_rect(), id, egui::Sense::focusable_noninteractive());
            ui.memory_mut(|memory| memory.set_focus_lock_filter(id, HOVER_FILTER));
            if resizable {
                resize_hover(ui, id, ui.min_rect(), above, maximum.min(bounds.size()));
            }
        });
    ui.ctx()
        .data_mut(|data| data.insert_temp(id.with("size"), output.response.rect.size()));
    (Some(output.response.rect), offset)
}

fn signature_button(ui: &mut Ui, id: Id, down: bool, colors: Colors) -> bool {
    const CHEVRON_RATIO: f32 = 0.2;
    let (_, rect) = ui.allocate_space(vec2(BUTTON_SIZE, BUTTON_SIZE));
    let response = ui.interact(rect, id, egui::Sense::CLICK);
    ui.ctx()
        .register_pointer_preserves_keyboard_focus(response.id);
    let center = rect.center();
    let half = BUTTON_SIZE * CHEVRON_RATIO;
    let sign = if down { 1.0 } else { -1.0 };
    ui.painter().add(egui::Shape::line(
        vec![
            pos2(center.x - half, center.y - sign * half),
            pos2(center.x, center.y + sign * half),
            pos2(center.x + half, center.y - sign * half),
        ],
        egui::Stroke::new(BORDER_WIDTH, colors.foreground),
    ));
    response.clicked()
}

fn resize_hover(ui: &Ui, id: Id, rect: Rect, above: bool, maximum: Vec2) {
    let right = Rect::from_center_size(
        pos2(rect.right(), rect.center().y),
        vec2(RESIZE_HANDLE_WIDTH, rect.height()),
    );
    let vertical = Rect::from_center_size(
        pos2(
            rect.center().x,
            if above { rect.top() } else { rect.bottom() },
        ),
        vec2(rect.width(), RESIZE_HANDLE_WIDTH),
    );
    for (edge, handle, cursor) in [
        (true, right, egui::CursorIcon::ResizeHorizontal),
        (false, vertical, egui::CursorIcon::ResizeVertical),
    ] {
        let response = ui
            .interact(handle, id.with(("resize", edge)), egui::Sense::DRAG)
            .on_hover_and_drag_cursor(cursor);
        ui.ctx()
            .register_pointer_preserves_keyboard_focus(response.id);
        if response.dragged() {
            let mut size = ui
                .ctx()
                .data(|data| data.get_temp::<Vec2>(id.with("manual-size")))
                .unwrap_or(rect.size());
            let delta = response.drag_delta();
            if edge {
                size.x += delta.x;
            } else {
                size.y += if above { -delta.y } else { delta.y };
            }
            size.x = size
                .x
                .clamp(MINIMUM_HOVER_SIZE, maximum.x.max(MINIMUM_HOVER_SIZE));
            size.y = size
                .y
                .clamp(MINIMUM_HOVER_SIZE, maximum.y.max(MINIMUM_HOVER_SIZE));
            ui.ctx()
                .data_mut(|data| data.insert_temp(id.with("manual-size"), size));
            ui.ctx().request_repaint();
        }
    }
}

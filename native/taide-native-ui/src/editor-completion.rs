use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use egui::{Color32, Event, Id, Key, Modifiers, Rect, Ui, Vec2, pos2, vec2};
use taide_native_editor::completion::{Command, CompletionItemKind};
use taide_native_editor::completion_model::Model;
use taide_native_editor::completion_preview::GhostText;
use taide_native_editor::completion_preview_view::ViewData;
use taide_native_editor::document::EditorError;
use taide_native_editor::documentation::RichDocument;
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::ViewId;

use crate::editor_geometry::EditorGeometry;
use crate::editor_markup;
use crate::editor_surface::EditorAppearance;

const QUICK_DELAY: f64 = 0.010;
const LOADING_DELAY: f64 = 0.050;
const WIDTH: f32 = 430.0;
const DETAILS_WIDTH: f32 = 330.0;
const DETAILS_MINIMUM_ROWS: f32 = 2.0;
const ROWS: usize = 12;
const SHORT_TEXT_LENGTH: usize = 3;
const SHORT_INLINE_LENGTH: usize = 5;
const MINIMUM_WIDTH: f32 = 220.0;
const HORIZONTAL_PADDING: f32 = 14.0;
const VERTICAL_PADDING: f32 = 22.0;
const RESIZE_HANDLE_WIDTH: f32 = 4.0;
const MINIMUM_STORED_ROWS: f32 = 4.3;
const MIN_ROW_HEIGHT: f32 = 8.0;
const MAX_ROW_HEIGHT: f32 = 1000.0;
const BORDER: f32 = 1.0;
const ICON_SIZE: f32 = 16.0;
const ICON_WIDTH: f32 = 22.0;
const ROW_PADDING: f32 = 4.0;
const DETAILS_PADDING: i8 = 5;
const CORNER_RADIUS: u8 = 6;
const DETAIL_FONT_SCALE: f32 = 0.85;
const DETAIL_MARGIN_EM: f32 = 1.1;
const DEPRECATED_OPACITY: f32 = 0.66;
const FILTER: egui::EventFilter = egui::EventFilter {
    tab: true,
    horizontal_arrows: true,
    vertical_arrows: true,
    escape: true,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colors {
    pub background: Color32,
    pub foreground: Color32,
    pub border: Color32,
    pub selected_background: Color32,
    pub selected_foreground: Color32,
    pub selected_icon: Color32,
    pub highlight: Color32,
    pub selected_highlight: Color32,
    pub resize: Color32,
    pub documentation: editor_markup::Colors,
    pub preview: PreviewColors,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewColors {
    pub foreground: Color32,
    pub background: Option<Color32>,
    pub border: Option<Color32>,
}

impl PreviewColors {
    pub fn for_dark_mode(is_dark: bool) -> Self {
        const DARK: [u8; 4] = [255, 255, 255, 86];
        const LIGHT: [u8; 4] = [0, 0, 0, 119];
        let [red, green, blue, alpha] = if is_dark { DARK } else { LIGHT };
        Self {
            foreground: Color32::from_rgba_unmultiplied(red, green, blue, alpha),
            background: None,
            border: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trigger {
    Manual,
    Automatic,
    Character(String),
}

#[derive(Clone)]
pub struct Widget {
    pub token: String,
    pub byte: usize,
    pub automatic: bool,
    pub pending: bool,
    pub model: Option<Rc<RefCell<Model>>>,
    pub leading: String,
    pub delta: isize,
}

pub trait Provider: editor_markup::Provider {
    fn is_embedded(&self, _view: ViewId) -> bool {
        false
    }
    fn available(&self, store: &EditorStore, view: ViewId) -> bool;
    fn request(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        trigger: Trigger,
    ) -> Result<bool, EditorError>;
    fn current(&self, store: &EditorStore, view: ViewId) -> Option<Widget>;
    fn close(&mut self, view: ViewId);
    fn triggers(&self, store: &EditorStore, view: ViewId) -> Vec<String>;
    fn should_auto_trigger(&self, store: &EditorStore, view: ViewId) -> bool;
    fn after_event(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        event: &Event,
    ) -> Result<(), EditorError>;
    fn take_commands(&mut self, _view: ViewId) -> Vec<Command> {
        Vec::new()
    }
    fn preview(
        &mut self,
        _store: &EditorStore,
        _view: ViewId,
        _token: &str,
        _candidate: usize,
        _alternate: bool,
    ) -> Vec<GhostText> {
        Vec::new()
    }
    fn preview_tokens(
        &self,
        _view: ViewId,
        _line: usize,
    ) -> Option<Arc<taide_native_editor::line_tokens::PreviewTokens>> {
        None
    }
    fn snippet_event(
        &mut self,
        _store: &mut EditorStore,
        _view: ViewId,
        _event: &Event,
    ) -> Result<bool, EditorError> {
        Ok(false)
    }
    fn accept(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
        token: &str,
        candidate: usize,
        alternate: bool,
    ) -> Result<bool, EditorError>;
    fn documentation(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        token: &str,
        candidate: usize,
    ) -> Option<Arc<RichDocument>>;
    fn detail(
        &self,
        _store: &EditorStore,
        _view: ViewId,
        _token: &str,
        _candidate: usize,
    ) -> Option<String> {
        None
    }
    fn icon_color(&self, _ui: &Ui, _kind: CompletionItemKind) -> Option<Color32> {
        None
    }
}

#[derive(Debug, Clone, Default)]
pub struct Geometry {
    pub list: Option<Rect>,
    pub details: Option<Rect>,
    pub details_close: Option<Rect>,
    pub rows: Vec<(usize, Rect)>,
    pub selected: Option<usize>,
    pub resize_handles: Vec<Rect>,
    pub item_height: f32,
    pub viewport_height: f32,
}

#[derive(Clone, Copy)]
struct Resize {
    size: Vec2,
    initial: Vec2,
    stored: Option<Vec2>,
    above: bool,
    width: bool,
    height: bool,
}

#[derive(Clone, Copy)]
struct DetailsLayout {
    rect: Rect,
    minimum: Vec2,
    maximum: Vec2,
    above: bool,
    left: bool,
}

#[derive(Clone, Copy)]
struct DetailsResize {
    initial: DetailsLayout,
    current: DetailsLayout,
}

fn resize_edges(
    ui: &Ui,
    id: Id,
    rect: Rect,
    above: bool,
    left: bool,
    color: Color32,
) -> [(bool, bool, egui::Response); 3] {
    let x = if left { rect.left() } else { rect.right() };
    let y = if above { rect.top() } else { rect.bottom() };
    let horizontal = Rect::from_center_size(
        pos2(x, rect.center().y),
        vec2(RESIZE_HANDLE_WIDTH, rect.height()),
    );
    let vertical = Rect::from_center_size(
        pos2(rect.center().x, y),
        vec2(rect.width(), RESIZE_HANDLE_WIDTH),
    );
    let corner = Rect::from_center_size(pos2(x, y), Vec2::splat(RESIZE_HANDLE_WIDTH));
    [
        (true, false, horizontal, egui::CursorIcon::ResizeHorizontal),
        (false, true, vertical, egui::CursorIcon::ResizeVertical),
        (
            true,
            true,
            corner,
            if above != left {
                egui::CursorIcon::ResizeNeSw
            } else {
                egui::CursorIcon::ResizeNwSe
            },
        ),
    ]
    .map(|(width, height, rect, cursor)| {
        let response = ui
            .interact(
                rect,
                id.with(("resize", width, height)),
                egui::Sense::click_and_drag(),
            )
            .on_hover_and_drag_cursor(cursor);
        ui.ctx()
            .register_pointer_preserves_keyboard_focus(response.id);
        if response.hovered() || response.dragged() {
            ui.painter().rect_filled(rect, 0.0, color);
        }
        (width, height, response)
    })
}

fn details_layout(
    bounds: Rect,
    list: Rect,
    wanted: Vec2,
    row_height: f32,
    prefer_top: bool,
) -> DetailsLayout {
    let top_space = (bounds.bottom() - list.top() - BORDER * 2.0 - VERTICAL_PADDING).max(1.0);
    let bottom_space = (list.bottom() - bounds.top() - BORDER * 2.0 - VERTICAL_PADDING).max(1.0);
    let side = |is_left: bool| {
        let width = if is_left {
            list.left() - bounds.left() - BORDER - HORIZONTAL_PADDING
        } else {
            bounds.right() - list.right() - BORDER - HORIZONTAL_PADDING
        };
        let height = wanted.y.min(top_space.max(bottom_space));
        let align_top = if prefer_top {
            height <= top_space
        } else {
            height > bottom_space
        };
        let maximum = vec2(
            width.max(1.0),
            if align_top { top_space } else { bottom_space },
        );
        let minimum = vec2(
            MINIMUM_WIDTH.min(maximum.x),
            (row_height * DETAILS_MINIMUM_ROWS).min(maximum.y),
        );
        let size = wanted.clamp(minimum, maximum);
        let x = if is_left {
            (list.left() - size.x - BORDER).max(bounds.left() + HORIZONTAL_PADDING)
        } else {
            list.right() - BORDER
        };
        let y = if !align_top && size.y > list.height() {
            list.bottom() - BORDER * 2.0 - size.y
        } else {
            list.top()
        };
        (
            DetailsLayout {
                rect: Rect::from_min_size(pos2(x, y), size),
                minimum,
                maximum,
                above: !align_top,
                left: is_left,
            },
            width - wanted.x,
        )
    };
    let maximum = vec2(
        (list.width() - BORDER * 2.0).max(1.0),
        (if prefer_top {
            bounds.bottom() - list.bottom()
        } else {
            list.top() - bounds.top()
        } - VERTICAL_PADDING)
            .max(1.0),
    );
    let minimum = vec2(
        MINIMUM_WIDTH.min(maximum.x),
        (row_height * DETAILS_MINIMUM_ROWS).min(maximum.y),
    );
    let size = wanted.clamp(minimum, maximum);
    let y = if prefer_top {
        list.bottom() - BORDER
    } else {
        list.top() - size.y + BORDER
    };
    let vertical = (
        DetailsLayout {
            rect: Rect::from_min_size(pos2(list.left(), y), size),
            minimum,
            maximum,
            above: !prefer_top,
            left: true,
        },
        maximum.y - wanted.y,
    );
    let placements = [side(false), side(true), vertical];
    placements
        .iter()
        .find(|(_, fit)| *fit >= 0.0)
        .or_else(|| {
            placements
                .iter()
                .max_by(|(_, first), (_, second)| first.total_cmp(second))
        })
        .unwrap()
        .0
}

fn size_key(ui: &Ui, embedded: bool) -> Id {
    Id::new((
        "native-editor-suggest-size",
        ui.ctx().viewport_id(),
        embedded,
    ))
}

#[derive(Default, Clone)]
pub(crate) struct State {
    token: Option<String>,
    selected: Option<usize>,
    index: usize,
    quick_due: Option<f64>,
    loading_since: Option<f64>,
    details_open: bool,
    details_focused: bool,
    details_focus_requested: bool,
    release_at: Option<usize>,
    scroll: f32,
    reveal: bool,
    details_scroll: Vec2,
    geometry: Geometry,
    resizing: Option<Resize>,
    resize_focus: Option<Id>,
    details_size: Option<Vec2>,
    details_resizing: Option<DetailsResize>,
    details_content_height: f32,
    details_candidate: Option<(String, usize)>,
    details_position: Option<(Rect, Rect, DetailsLayout)>,
}

#[derive(Default)]
pub(crate) struct Preview {
    pub(crate) views: Vec<ViewData>,
    pub(crate) styles: Vec<PreviewStyle>,
}

pub(crate) struct PreviewStyle {
    pub(crate) line: usize,
    pub(crate) tokens: Option<Arc<taide_native_editor::line_tokens::PreviewTokens>>,
    pub(crate) short: bool,
    pub(crate) additional_start: usize,
    pub(crate) short_anchors: Vec<usize>,
}

impl State {
    fn page(&mut self, count: usize, next: bool) -> usize {
        let item = self.geometry.item_height;
        let height = self.geometry.viewport_height;
        if item <= 0.0 || height <= 0.0 {
            return if next {
                (self.index + ROWS).min(count - 1)
            } else {
                self.index.saturating_sub(ROWS)
            };
        }
        if next {
            let last = |scroll: f32| ((scroll + height) / item).floor().max(1.0) as usize - 1;
            let current = last(self.scroll).min(count - 1);
            if self.index < current {
                return current;
            }
            self.scroll = (self.scroll + height).min((count as f32 * item - height).max(0.0));
            return last(self.scroll).min(count - 1);
        }
        let first = |scroll: f32| {
            if scroll <= 0.0 {
                0
            } else {
                ((scroll - 1.0).max(0.0) / item).floor() as usize + 1
            }
        };
        let current = first(self.scroll).min(count - 1);
        if self.index > current {
            return current;
        }
        self.scroll = (self.scroll - height).max(0.0);
        first(self.scroll).min(count - 1)
    }

    fn resize(
        &mut self,
        ui: &Ui,
        id: Id,
        key: Id,
        rect: Rect,
        size: Vec2,
        stored: Option<Vec2>,
        default_size: Vec2,
        minimum: Vec2,
        maximum: Vec2,
        above: bool,
        row_height: f32,
        color: Color32,
        ranked: &[usize],
        widget: &Widget,
        appearance: &EditorAppearance,
    ) -> bool {
        let mut consumed = false;
        for (width, height, response) in resize_edges(ui, id, rect, above, false, color) {
            self.geometry.resize_handles.push(response.rect);
            if response.is_pointer_button_down_on() {
                self.resize_focus = ui.memory(|memory| memory.focused());
            }
            consumed |= response.clicked() || response.dragged() || response.drag_stopped();
            if response.dragged() {
                let resize = self.resizing.get_or_insert(Resize {
                    size,
                    initial: size,
                    stored,
                    above,
                    width: false,
                    height: false,
                });
                let delta = response.total_drag_delta().unwrap_or_default();
                if width {
                    resize.size.x = resize.initial.x + delta.x;
                    resize.width = true;
                }
                if height {
                    resize.size.y = resize.initial.y + if above { -delta.y } else { delta.y };
                    resize.height = true;
                }
                resize.size = resize.size.clamp(minimum, maximum);
                ui.ctx().request_repaint();
            }
            if response.drag_stopped()
                && let Some(resize) = self.resizing.take()
            {
                let threshold = (row_height / 2.0).round();
                let original = resize.stored.unwrap_or(default_size);
                let width = if resize.width && (resize.size.x - resize.initial.x).abs() > threshold
                {
                    resize.size.x
                } else {
                    original.x
                };
                let height =
                    if resize.height && (resize.size.y - resize.initial.y).abs() > threshold {
                        resize.size.y
                    } else {
                        original.y
                    };
                ui.ctx()
                    .data_mut(|data| data.insert_temp(key, vec2(width, height)));
                ui.ctx().request_repaint();
            }
            if response.double_clicked() {
                let mut wanted = stored.unwrap_or(default_size);
                if width {
                    let length = widget.model.as_ref().map_or(0, |model| {
                        let model = model.borrow();
                        ranked
                            .iter()
                            .filter_map(|index| model.candidate(*index))
                            .map(|candidate| candidate.item.label.encode_utf16().count())
                            .max()
                            .unwrap_or(0)
                    });
                    let typical = ui
                        .painter()
                        .layout_no_wrap("n".into(), appearance.font.clone(), color)
                        .size()
                        .x;
                    wanted.x = (length as f32 * typical).clamp(minimum.x, maximum.x);
                }
                if height {
                    wanted.y = default_size.y;
                }
                self.resizing = None;
                ui.ctx().data_mut(|data| data.insert_temp(key, wanted));
                ui.ctx().request_repaint();
            }
        }
        if !ui.input(|input| input.pointer.any_down()) {
            self.resize_focus = None;
        }
        consumed
    }

    fn resize_details(&mut self, ui: &Ui, id: Id, layout: DetailsLayout, color: Color32) {
        for (width, height, response) in
            resize_edges(ui, id, layout.rect, layout.above, layout.left, color)
        {
            self.geometry.resize_handles.push(response.rect);
            if response.is_pointer_button_down_on() {
                self.resize_focus = ui.memory(|memory| memory.focused());
            }
            if response.dragged() {
                let mut initial = layout;
                let bounds = ui.ctx().content_rect();
                initial.maximum.x = initial
                    .maximum
                    .x
                    .min(if initial.left {
                        initial.rect.right() - bounds.left() - HORIZONTAL_PADDING
                    } else {
                        bounds.right() - initial.rect.left() - HORIZONTAL_PADDING
                    })
                    .max(1.0);
                initial.maximum.y = initial
                    .maximum
                    .y
                    .min(if initial.above {
                        initial.rect.bottom() - bounds.top() - VERTICAL_PADDING
                    } else {
                        bounds.bottom() - initial.rect.top() - VERTICAL_PADDING
                    })
                    .max(1.0);
                initial.minimum = initial.minimum.min(initial.maximum);
                let resize = self.details_resizing.get_or_insert(DetailsResize {
                    initial,
                    current: initial,
                });
                let delta = response.total_drag_delta().unwrap_or_default();
                let initial = resize.initial;
                let mut size = resize.current.rect.size();
                if width {
                    size.x = initial.rect.width() + if initial.left { -delta.x } else { delta.x };
                }
                if height {
                    size.y = initial.rect.height() + if initial.above { -delta.y } else { delta.y };
                }
                size = size.clamp(initial.minimum, initial.maximum);
                let x = if initial.left {
                    initial.rect.right() - size.x
                } else {
                    initial.rect.left()
                };
                let y = if initial.above {
                    initial.rect.bottom() - size.y
                } else {
                    initial.rect.top()
                };
                resize.current.rect = Rect::from_min_size(pos2(x, y), size);
                ui.ctx().request_repaint();
            }
            if response.drag_stopped()
                && let Some(resize) = self.details_resizing.take()
            {
                self.details_size = Some(resize.current.rect.size());
                if let Some(list) = self.geometry.list {
                    self.details_position = Some((list, ui.ctx().content_rect(), resize.current));
                }
                ui.ctx().request_repaint();
            }
        }
        if !ui.input(|input| input.pointer.any_down()) {
            self.resize_focus = None;
        }
    }

    pub(crate) fn preview(
        &mut self,
        ui: &Ui,
        store: &EditorStore,
        view: ViewId,
        provider: &mut dyn Provider,
    ) -> Preview {
        let Some(widget) = provider.current(store, view) else {
            return Preview::default();
        };
        self.ranked(&widget);
        let Some(selected) = self.selected else {
            return Preview::default();
        };
        let Some(document) = store
            .views()
            .get(view)
            .and_then(|view| store.documents().snapshot(view.document).ok())
        else {
            return Preview::default();
        };
        let ghosts = provider.preview(
            store,
            view,
            &widget.token,
            selected,
            ui.input(|input| input.modifiers.shift),
        );
        let mut result = Preview::default();
        for ghost in &ghosts {
            let Ok(data) = ViewData::new(&document, ghost) else {
                continue;
            };
            let non_whitespace = ghost
                .parts
                .iter()
                .flat_map(|part| part.text.chars())
                .filter(|character| !character.is_whitespace())
                .count();
            let short = non_whitespace > 0 && non_whitespace < SHORT_TEXT_LENGTH;
            let additional_start = 1 + result
                .views
                .iter()
                .filter(|previous| previous.line == data.line)
                .map(|previous| previous.additional.len())
                .sum::<usize>();
            let short_anchors = data
                .inline
                .iter()
                .filter(|part| part.text.encode_utf16().count() < SHORT_INLINE_LENGTH)
                .map(|part| part.byte)
                .collect();
            result.styles.push(PreviewStyle {
                line: data.line,
                tokens: provider.preview_tokens(view, data.line),
                short,
                additional_start,
                short_anchors,
            });
            result.views.push(data);
        }
        result
    }

    #[cfg(feature = "inspection")]
    pub(crate) fn geometry(&self) -> Geometry {
        self.geometry.clone()
    }

    pub(crate) fn contains_pointer(&self, ui: &Ui) -> bool {
        ui.input(|input| input.pointer.hover_pos())
            .is_some_and(|position| {
                self.geometry
                    .list
                    .is_some_and(|rect| rect.contains(position))
                    || self
                        .geometry
                        .details
                        .is_some_and(|rect| rect.contains(position))
                    || self
                        .geometry
                        .resize_handles
                        .iter()
                        .any(|rect| rect.contains(position))
            })
    }

    pub(crate) fn owns_event(&self, ui: &Ui, body: Id, index: usize) -> bool {
        if self.release_at.is_some_and(|released| index > released) {
            return true;
        }
        if !self.details_focused {
            return false;
        }
        let details = body.with("completion-details");
        ui.ctx()
            .keyboard_input_route(details)
            .and_then(|route| route.0.get(index).copied())
            .and_then(|(owned, _)| owned)
            .unwrap_or(ui.ctx().keyboard_focus_before_events() == Some(details))
    }

    fn close(&mut self, ui: &Ui, body: Id, provider: &mut dyn Provider, view: ViewId) {
        let key = size_key(ui, provider.is_embedded(view));
        if let Some(mut size) = ui.ctx().data(|data| data.get_temp::<Vec2>(key)) {
            size.y = size
                .y
                .max((self.geometry.item_height * MINIMUM_STORED_ROWS).ceil());
            ui.ctx().data_mut(|data| data.insert_temp(key, size));
        }
        self.resizing = None;
        self.details_resizing = None;
        self.details_position = None;
        self.resize_focus = None;
        provider.close(view);
        self.quick_due = None;
        self.loading_since = None;
        self.token = None;
        self.selected = None;
        self.geometry = Geometry::default();
        if self.details_focused {
            ui.memory_mut(|memory| memory.request_focus_with_filter(body, FILTER));
        }
        self.details_focused = false;
        self.details_focus_requested = false;
    }

    fn ranked(&mut self, widget: &Widget) -> Vec<usize> {
        if self.token.as_deref() != Some(widget.token.as_str()) {
            self.resizing = None;
            self.token = Some(widget.token.clone());
            self.index = 0;
            self.selected = None;
            self.scroll = 0.0;
            self.reveal = true;
        }
        let ranked = widget.model.as_ref().map_or_else(Vec::new, |model| {
            model
                .borrow_mut()
                .filter(&widget.leading, widget.delta)
                .iter()
                .map(|ranked| ranked.candidate)
                .collect::<Vec<_>>()
        });
        self.index = self
            .selected
            .and_then(|candidate| ranked.iter().position(|index| *index == candidate))
            .unwrap_or(self.index.min(ranked.len().saturating_sub(1)));
        self.selected = ranked.get(self.index).copied();
        ranked
    }

    pub(crate) fn begin(
        &mut self,
        ui: &Ui,
        store: &mut EditorStore,
        view: ViewId,
        body: Id,
        provider: &mut dyn Provider,
        commands: &[Command],
    ) -> Result<(), EditorError> {
        self.release_at = None;
        if let Some(focus) = self.resize_focus
            && ui.input(|input| input.focused && input.pointer.any_released())
            && ui.memory(|memory| memory.focused().is_none())
        {
            ui.memory_mut(|memory| memory.request_focus_with_filter(focus, FILTER));
        }
        let pending = provider.take_commands(view);
        if commands.contains(&Command::ResetSize) || pending.contains(&Command::ResetSize) {
            self.resizing = None;
            let key = size_key(ui, provider.is_embedded(view));
            ui.ctx().data_mut(|data| data.remove::<Vec2>(key));
        }
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        let document = store.documents().snapshot(current.document)?;
        let focused = ui.memory(|memory| {
            memory.has_focus(body) || memory.has_focus(body.with("completion-details"))
        });
        if !focused || !ui.input(|input| input.focused) {
            self.details_focused = false;
            self.close(ui, body, provider, view);
            return Ok(());
        }
        if !ui.is_enabled()
            || current.composition.is_some()
            || document.metadata.read_only
            || !provider.available(store, view)
        {
            self.close(ui, body, provider, view);
            return Ok(());
        }
        for command in commands.iter().copied().chain(pending) {
            self.command(ui, store, view, body, provider, command)?;
        }
        if self
            .quick_due
            .is_some_and(|due| ui.input(|input| input.time) >= due)
        {
            self.quick_due = None;
            if provider.should_auto_trigger(store, view) {
                provider.request(store, view, Trigger::Automatic)?;
            }
        }
        if let Some(due) = self.quick_due {
            ui.ctx().request_repaint_after(Duration::from_secs_f64(
                (due - ui.input(|input| input.time)).max(0.0),
            ));
        }
        Ok(())
    }

    pub(crate) fn flush_commands(
        &mut self,
        ui: &Ui,
        store: &mut EditorStore,
        view: ViewId,
        body: Id,
        provider: &mut dyn Provider,
    ) -> Result<(), EditorError> {
        for command in provider.take_commands(view) {
            self.command(ui, store, view, body, provider, command)?;
        }
        Ok(())
    }

    pub(crate) fn command(
        &mut self,
        ui: &Ui,
        store: &mut EditorStore,
        view: ViewId,
        body: Id,
        provider: &mut dyn Provider,
        command: Command,
    ) -> Result<bool, EditorError> {
        if command == Command::ResetSize {
            self.resizing = None;
            let key = size_key(ui, provider.is_embedded(view));
            ui.ctx().data_mut(|data| data.remove::<Vec2>(key));
            return Ok(true);
        }
        if command == Command::Trigger && provider.current(store, view).is_none() {
            self.quick_due = None;
            self.loading_since = Some(ui.input(|input| input.time));
            return provider.request(store, view, Trigger::Manual);
        }
        let Some(widget) = provider.current(store, view) else {
            return Ok(false);
        };
        let ranked = self.ranked(&widget);
        match command {
            Command::Trigger | Command::ToggleDetails => {
                if self.selected.is_some() {
                    self.details_open = !self.details_open;
                    self.details_position = None;
                    if !self.details_open && self.details_focused {
                        self.details_focused = false;
                        ui.memory_mut(|memory| memory.request_focus_with_filter(body, FILTER));
                    }
                }
            }
            Command::ToggleDetailsFocus => {
                self.details_open = true;
                self.details_focused = !self.details_focused;
                self.details_focus_requested = self.details_focused;
                if !self.details_focused {
                    ui.memory_mut(|memory| memory.request_focus_with_filter(body, FILTER));
                }
            }
            Command::Hide => self.close(ui, body, provider, view),
            Command::Accept { alternate } => {
                let Some(candidate) = self.selected else {
                    return Ok(false);
                };
                if !provider.accept(store, view, &widget.token, candidate, alternate)? {
                    return Ok(false);
                }
                self.close(ui, body, provider, view);
                ui.memory_mut(|memory| memory.request_focus_with_filter(body, FILTER));
            }
            Command::Next
            | Command::Previous
            | Command::NextPage
            | Command::PreviousPage
            | Command::First
            | Command::Last => {
                if ranked.is_empty() {
                    return Ok(false);
                }
                self.index = match command {
                    Command::Next => (self.index + 1) % ranked.len(),
                    Command::Previous => (self.index + ranked.len() - 1) % ranked.len(),
                    Command::NextPage => self.page(ranked.len(), true),
                    Command::PreviousPage => self.page(ranked.len(), false),
                    Command::First => 0,
                    Command::Last => ranked.len() - 1,
                    _ => self.index,
                };
                self.selected = ranked.get(self.index).copied();
                self.reveal = true;
                self.details_scroll = Vec2::ZERO;
            }
            Command::ResetSize => {}
        }
        ui.ctx().request_repaint();
        Ok(true)
    }

    pub(crate) fn event(
        &mut self,
        ui: &Ui,
        store: &mut EditorStore,
        view: ViewId,
        body: Id,
        provider: &mut dyn Provider,
        event: &Event,
        index: usize,
        composing: bool,
    ) -> Result<bool, EditorError> {
        if matches!(event, Event::WindowFocused(false))
            || matches!(event, Event::PointerButton { pos, pressed: true, .. } if !self.geometry.list.is_some_and(|rect| rect.contains(*pos)) && !self.geometry.details.is_some_and(|rect| rect.contains(*pos)) && !self.geometry.resize_handles.iter().any(|rect| rect.contains(*pos)))
        {
            self.close(ui, body, provider, view);
            return Ok(false);
        }
        if composing {
            return Ok(false);
        }
        let visible = provider.current(store, view).is_some();
        let details_navigation = self.details_focused
            && matches!(
                event,
                Event::Key {
                    key: Key::ArrowUp
                        | Key::ArrowDown
                        | Key::PageUp
                        | Key::PageDown
                        | Key::ArrowLeft
                        | Key::ArrowRight
                        | Key::Home
                        | Key::End,
                    pressed: true,
                    ..
                }
            );
        if !details_navigation
            && let Some(command) = shortcut(event, ui.ctx().os().is_mac(), visible)
        {
            let was_focused = self.details_focused;
            let handled = self.command(ui, store, view, body, provider, command)?;
            if handled && was_focused && !self.details_focused {
                self.release_at = Some(index);
            }
            return Ok(handled);
        }
        if self.details_focused && self.owns_event(ui, body, index) {
            if let Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = event
            {
                let step = ui.text_style_height(&egui::TextStyle::Body);
                let page = self.geometry.details.map_or(step, |rect| rect.height());
                match key {
                    Key::ArrowUp => self.details_scroll.y = (self.details_scroll.y - step).max(0.0),
                    Key::ArrowDown => self.details_scroll.y += step,
                    Key::PageUp => self.details_scroll.y = (self.details_scroll.y - page).max(0.0),
                    Key::PageDown => self.details_scroll.y += page,
                    Key::ArrowLeft => {
                        self.details_scroll.x = (self.details_scroll.x - step).max(0.0)
                    }
                    Key::ArrowRight => self.details_scroll.x += step,
                    Key::Home if modifiers.command => self.details_scroll = Vec2::ZERO,
                    Key::End if modifiers.command => self.details_scroll.y = f32::MAX,
                    _ => return Ok(false),
                }
                ui.ctx().request_repaint();
                return Ok(true);
            }
            return Ok(matches!(
                event,
                Event::Text(_) | Event::Paste(_) | Event::Ime(_)
            ));
        }
        provider.snippet_event(store, view, event)
    }

    pub(crate) fn after_event(
        &mut self,
        ui: &Ui,
        store: &EditorStore,
        view: ViewId,
        provider: &mut dyn Provider,
        event: &Event,
    ) -> Result<(), EditorError> {
        provider.after_event(store, view, event)?;
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        if current.composition.is_some() {
            provider.close(view);
            self.quick_due = None;
            return Ok(());
        }
        let text = match event {
            Event::Text(text) | Event::Ime(egui::ImeEvent::Commit(text)) => text,
            _ => return Ok(()),
        };
        if let Some(trigger) = provider
            .triggers(store, view)
            .into_iter()
            .find(|trigger| !trigger.is_empty() && text.ends_with(trigger))
        {
            self.quick_due = None;
            provider.request(store, view, Trigger::Character(trigger))?;
        } else if provider.current(store, view).is_none()
            && provider.should_auto_trigger(store, view)
        {
            self.quick_due = Some(ui.input(|input| input.time) + QUICK_DELAY);
            ui.ctx()
                .request_repaint_after(Duration::from_secs_f64(QUICK_DELAY));
        }
        Ok(())
    }

    pub(crate) fn paint(
        &mut self,
        ui: &Ui,
        store: &mut EditorStore,
        view: ViewId,
        body: Id,
        geometry: &EditorGeometry,
        appearance: &EditorAppearance,
        colors: Colors,
        provider: &mut dyn Provider,
    ) -> Result<Vec<Id>, EditorError> {
        self.geometry = Geometry::default();
        let Some(widget) = provider.current(store, view) else {
            return Ok(Vec::new());
        };
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        if current.composition.is_some()
            || store
                .documents()
                .snapshot(current.document)?
                .metadata
                .read_only
        {
            self.close(ui, body, provider, view);
            return Ok(Vec::new());
        }
        let ranked = self.ranked(&widget);
        let time = ui.input(|input| input.time);
        if widget.pending {
            let started = *self.loading_since.get_or_insert(time);
            if widget.automatic {
                return Ok(Vec::new());
            }
            if time < started + LOADING_DELAY {
                ui.ctx().request_repaint_after(Duration::from_secs_f64(
                    (started + LOADING_DELAY - time).max(0.0),
                ));
                return Ok(Vec::new());
            }
        } else {
            self.loading_since = None;
            if ranked.is_empty() && widget.automatic {
                self.close(ui, body, provider, view);
                return Ok(Vec::new());
            }
        }
        let Some(anchor) = geometry.caret_rect(widget.byte) else {
            return Ok(Vec::new());
        };
        if !geometry.rect.intersects(anchor) {
            return Ok(Vec::new());
        }
        let bounds = ui.ctx().content_rect();
        let row_height = appearance.line_height.clamp(MIN_ROW_HEIGHT, MAX_ROW_HEIGHT);
        let empty = widget.pending || ranked.is_empty();
        let key = size_key(ui, provider.is_embedded(view));
        let default_size = vec2(WIDTH, ROWS as f32 * row_height + BORDER * 2.0);
        let stored = ui.ctx().data(|data| data.get_temp::<Vec2>(key));
        let wanted = self
            .resizing
            .map(|resize| resize.size)
            .or(stored)
            .unwrap_or(default_size);
        let full_height = ranked.len().max(1) as f32 * row_height + BORDER * 2.0;
        let below = (bounds.bottom() - anchor.bottom() - VERTICAL_PADDING)
            .max(0.0)
            .min(full_height);
        let above_space = (anchor.top() - bounds.top() - VERTICAL_PADDING)
            .max(0.0)
            .min(full_height);
        let above = self.resizing.map_or(
            !empty && wanted.y.min(full_height) > below && above_space > below,
            |resize| resize.above,
        );
        let maximum = vec2(
            (bounds.width() - BORDER * 2.0 - HORIZONTAL_PADDING * 2.0).max(1.0),
            (if above { above_space } else { below })
                .max(row_height + BORDER * 2.0)
                .min(full_height)
                .min(bounds.height()),
        );
        let minimum = vec2(
            MINIMUM_WIDTH.min(maximum.x),
            (row_height + BORDER * 2.0).min(maximum.y),
        );
        let size = if empty {
            vec2(
                (WIDTH / 2.0).min(bounds.width()),
                (row_height + BORDER * 2.0).min(bounds.height()),
            )
        } else {
            wanted.clamp(minimum, maximum)
        };
        let width = size.x;
        let height = size.y;
        let position = pos2(
            anchor.left().min(bounds.right() - width).max(bounds.left()),
            if above {
                (anchor.top() - height).max(bounds.top())
            } else {
                anchor
                    .bottom()
                    .min(bounds.bottom() - height)
                    .max(bounds.top())
            },
        );
        let list_id = body.with("completion-list");
        let mut clicked = None;
        let output = egui::Area::new(list_id)
            .fixed_pos(position)
            .order(egui::Order::Tooltip)
            .movable(false)
            .show(ui.ctx(), |ui| {
                ui.set_max_size(size);
                ui.set_clip_rect(bounds);
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let frame = egui::Frame::new()
                    .fill(colors.background)
                    .stroke(egui::Stroke::new(BORDER, colors.border))
                    .corner_radius(CORNER_RADIUS)
                    .shadow(ui.visuals().popup_shadow)
                    .show(ui, |ui| {
                        ui.set_width((width - BORDER * 2.0).max(0.0));
                        if empty {
                            let text = if widget.pending {
                                "Loading..."
                            } else {
                                "No suggestions."
                            };
                            let (rect, _) = ui.allocate_exact_size(
                                vec2(width - BORDER * 2.0, row_height),
                                egui::Sense::hover(),
                            );
                            ui.painter().text(
                                rect.left_center() + vec2(ICON_WIDTH, 0.0),
                                egui::Align2::LEFT_CENTER,
                                text,
                                appearance.font.clone(),
                                colors.foreground,
                            );
                            return;
                        }
                        let model = widget.model.as_ref().unwrap();
                        let mut model = model.borrow_mut();
                        let scores = model.filter(&widget.leading, widget.delta).to_vec();
                        let viewport_height = (height - BORDER * 2.0).max(row_height);
                        if self.reveal {
                            let top = self.index as f32 * row_height;
                            if top < self.scroll {
                                self.scroll = top;
                            }
                            if top + row_height > self.scroll + viewport_height {
                                self.scroll = top + row_height - viewport_height;
                            }
                            self.reveal = false;
                        }
                        let result = egui::ScrollArea::vertical()
                            .id_salt(list_id)
                            .max_height(viewport_height)
                            .auto_shrink([false, false])
                            .vertical_scroll_offset(self.scroll)
                            .show_rows(ui, row_height, ranked.len(), |ui, rows| {
                                for row in rows {
                                    let Some(candidate) = model.candidate(ranked[row]) else {
                                        continue;
                                    };
                                    let selected = row == self.index;
                                    let (rect, response) = ui.allocate_exact_size(
                                        vec2(ui.available_width(), row_height),
                                        egui::Sense::click(),
                                    );
                                    ui.ctx()
                                        .register_pointer_preserves_keyboard_focus(response.id);
                                    self.geometry.rows.push((ranked[row], rect));
                                    if selected {
                                        ui.painter().rect_filled(
                                            rect,
                                            0.0,
                                            colors.selected_background,
                                        );
                                    }
                                    let kind =
                                        candidate.item.kind.unwrap_or(CompletionItemKind::TEXT);
                                    let icon_color = if selected {
                                        colors.selected_icon
                                    } else {
                                        provider.icon_color(ui, kind).unwrap_or(colors.foreground)
                                    };
                                    ui.painter().text(
                                        rect.left_center()
                                            + vec2(ROW_PADDING + ICON_WIDTH / 2.0, 0.0),
                                        egui::Align2::CENTER_CENTER,
                                        glyph(kind),
                                        egui::FontId::new(
                                            ICON_SIZE,
                                            crate::font_families::codicons(ui),
                                        ),
                                        icon_color,
                                    );
                                    let foreground = if selected {
                                        colors.selected_foreground
                                    } else {
                                        colors.foreground
                                    };
                                    let highlight = if selected {
                                        colors.selected_highlight
                                    } else {
                                        colors.highlight
                                    };
                                    let mut job = label_job(
                                        ui,
                                        &candidate.item.label,
                                        &scores[row].score.highlights(),
                                        appearance.font.clone(),
                                        foreground,
                                        highlight,
                                        candidate.is_deprecated(),
                                    );
                                    job.wrap.max_width =
                                        (rect.width() - ICON_WIDTH - ROW_PADDING * 2.0).max(0.0);
                                    job.wrap.max_rows = 1;
                                    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
                                    let label_width = galley.size().x;
                                    ui.painter().galley(
                                        pos2(
                                            rect.left() + ICON_WIDTH + ROW_PADDING,
                                            rect.center().y - galley.size().y / 2.0,
                                        ),
                                        galley,
                                        foreground,
                                    );
                                    if selected
                                        && !self.details_open
                                        && let Some(detail) = candidate
                                            .item
                                            .detail
                                            .as_deref()
                                            .filter(|detail| !detail.is_empty())
                                    {
                                        let space = rect.width()
                                            - ICON_WIDTH
                                            - label_width
                                            - ROW_PADDING * 2.0
                                            - appearance.font.size * DETAIL_MARGIN_EM;
                                        if space > ICON_WIDTH {
                                            let mut job = egui::text::LayoutJob::simple_singleline(
                                                detail.replace('\r', "").replace('\n', ""),
                                                egui::FontId::new(
                                                    appearance.font.size * DETAIL_FONT_SCALE,
                                                    appearance.font.family.clone(),
                                                ),
                                                foreground,
                                            );
                                            job.wrap.max_width = space;
                                            job.wrap.max_rows = 1;
                                            let galley =
                                                ui.fonts_mut(|fonts| fonts.layout_job(job));
                                            ui.painter().galley(
                                                pos2(
                                                    rect.right() - ROW_PADDING - galley.size().x,
                                                    rect.center().y - galley.size().y / 2.0,
                                                ),
                                                galley,
                                                foreground,
                                            );
                                        }
                                    }
                                    if response.clicked() {
                                        self.index = row;
                                        self.selected = Some(ranked[row]);
                                        clicked = Some(ranked[row]);
                                    }
                                }
                            });
                        self.scroll = result.state.offset.y;
                    });
                if !empty {
                    if self.resize(
                        ui,
                        list_id,
                        key,
                        frame.response.rect,
                        size,
                        stored,
                        default_size,
                        minimum,
                        maximum,
                        above,
                        row_height,
                        colors.resize,
                        &ranked,
                        &widget,
                        appearance,
                    ) {
                        clicked = None;
                    }
                }
            });
        self.geometry.list = Some(output.response.rect);
        self.geometry.selected = self.selected;
        self.geometry.item_height = row_height;
        self.geometry.viewport_height = (height - BORDER * 2.0).max(row_height);
        let mut ids = vec![list_id];
        if let Some(candidate) = clicked {
            if provider.accept(store, view, &widget.token, candidate, false)? {
                self.close(ui, body, provider, view);
                ui.memory_mut(|memory| memory.request_focus_with_filter(body, FILTER));
                return Ok(Vec::new());
            }
        }
        if self.details_open
            && let Some(candidate) = self.selected
            && let Some((detail, document)) = {
                let detail = provider.detail(store, view, &widget.token, candidate);
                let document = provider.documentation(store, view, &widget.token, candidate);
                (detail.is_some() || document.is_some()).then_some((detail, document))
            }
        {
            let id = body.with("completion-details");
            let list = output.response.rect;
            if self
                .details_candidate
                .as_ref()
                .is_none_or(|(token, previous)| token != &widget.token || *previous != candidate)
            {
                self.details_candidate = Some((widget.token.clone(), candidate));
                self.details_content_height = 0.0;
                self.details_position = None;
            }
            let padding = f32::from(DETAILS_PADDING) * 2.0 + BORDER * 2.0;
            let wanted = self.details_size.unwrap_or(vec2(
                DETAILS_WIDTH,
                self.details_content_height
                    .max(row_height * DETAILS_MINIMUM_ROWS),
            ));
            let layout = self
                .details_resizing
                .map(|resize| resize.current)
                .or_else(|| {
                    self.details_position
                        .filter(|(previous_list, previous_bounds, _)| {
                            *previous_list == list && *previous_bounds == bounds
                        })
                        .map(|(_, _, layout)| layout)
                })
                .unwrap_or_else(|| details_layout(bounds, list, wanted, row_height, !above));
            let mut close_details = false;
            let output = egui::Area::new(id.with("area"))
                .fixed_pos(layout.rect.min)
                .order(egui::Order::Tooltip)
                .movable(false)
                .show(ui.ctx(), |ui| {
                    ui.set_max_size(layout.rect.size());
                    ui.set_clip_rect(bounds);
                    ui.visuals_mut().override_text_color = Some(colors.documentation.foreground);
                    let frame = egui::Frame::new()
                        .fill(colors.documentation.background)
                        .stroke(egui::Stroke::new(BORDER, colors.documentation.border))
                        .inner_margin(DETAILS_PADDING)
                        .show(ui, |ui| {
                            ui.set_width((layout.rect.width() - padding).max(0.0));
                            let result = egui::ScrollArea::both()
                                .id_salt(id)
                                .min_scrolled_height(0.0)
                                .min_scrolled_width(0.0)
                                .max_height((layout.rect.height() - padding).max(0.0))
                                .auto_shrink([
                                    false,
                                    self.details_size.is_none() && self.details_resizing.is_none(),
                                ])
                                .scroll_offset(self.details_scroll)
                                .show(ui, |ui| {
                                    if let Some(detail) = &detail {
                                        let width = (ui.available_width() - row_height).max(0.0);
                                        ui.scope(|ui| {
                                            ui.set_max_width(width);
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(detail)
                                                        .font(appearance.font.clone()),
                                                )
                                                .wrap(),
                                            );
                                        });
                                    }
                                    if let Some(document) = &document {
                                        editor_markup::show(
                                            ui,
                                            document,
                                            appearance,
                                            colors.documentation,
                                            provider,
                                        );
                                    }
                                });
                            self.details_scroll = result.state.offset;
                            let natural = result.content_size.y + padding;
                            if (self.details_content_height - natural).abs() > f32::EPSILON {
                                self.details_content_height = natural;
                                ui.ctx().request_repaint();
                            }
                        });
                    let rect = frame.response.rect;
                    let mut actual = layout;
                    actual.rect = rect;
                    self.resize_details(ui, id, actual, colors.resize);
                    let close_rect = Rect::from_min_size(
                        pos2(rect.right() - BORDER - row_height, rect.top() + BORDER),
                        Vec2::splat(row_height),
                    );
                    let close = ui.interact(close_rect, id.with("close"), egui::Sense::click());
                    ui.ctx().register_pointer_preserves_keyboard_focus(close.id);
                    ui.painter().text(
                        close_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "\u{ea76}",
                        egui::FontId::new(ICON_SIZE, crate::font_families::codicons(ui)),
                        colors.documentation.foreground,
                    );
                    close_details = close.clicked();
                    self.geometry.details_close = Some(close_rect);
                    ui.interact(ui.min_rect(), id, egui::Sense::focusable_noninteractive());
                    ui.memory_mut(|memory| memory.set_focus_lock_filter(id, FILTER));
                });
            self.geometry.details = Some(output.response.rect);
            if close_details {
                self.details_open = false;
                self.details_focused = false;
                self.details_focus_requested = false;
                self.details_resizing = None;
                ui.memory_mut(|memory| memory.request_focus_with_filter(body, FILTER));
                ui.ctx().request_repaint();
            }
            if self.details_focus_requested {
                ui.memory_mut(|memory| memory.request_focus_with_filter(id, FILTER));
                self.details_focus_requested = false;
            }
            ids.push(id);
        } else if self.details_focused {
            self.details_focused = false;
            ui.memory_mut(|memory| memory.request_focus_with_filter(body, FILTER));
        }
        Ok(ids)
    }
}

pub fn shortcut(event: &Event, mac: bool, visible: bool) -> Option<Command> {
    let Event::Key {
        key,
        pressed: true,
        modifiers,
        ..
    } = event
    else {
        return None;
    };
    let control = if mac {
        modifiers.ctrl && !modifiers.mac_cmd && !modifiers.command
    } else {
        modifiers.ctrl || modifiers.command
    };
    let primary = modifiers.command && !modifiers.alt && !modifiers.shift;
    if *key == Key::Space && control && !modifiers.shift {
        return Some(if modifiers.alt && visible {
            Command::ToggleDetailsFocus
        } else if !modifiers.alt {
            Command::Trigger
        } else {
            return None;
        });
    }
    if *key == Key::I && primary
        || !visible && mac && *key == Key::Escape && *modifiers == Modifiers::ALT
    {
        return Some(Command::Trigger);
    }
    if !visible {
        return None;
    }
    let plain = !modifiers.command && !modifiers.ctrl && !modifiers.mac_cmd && !modifiers.alt;
    if plain {
        return match key {
            Key::Escape => Some(Command::Hide),
            Key::Enter | Key::Tab => Some(Command::Accept {
                alternate: modifiers.shift,
            }),
            Key::ArrowDown if !modifiers.shift => Some(Command::Next),
            Key::ArrowUp if !modifiers.shift => Some(Command::Previous),
            Key::PageDown if !modifiers.shift => Some(Command::NextPage),
            Key::PageUp if !modifiers.shift => Some(Command::PreviousPage),
            _ => None,
        };
    }
    if primary {
        return match key {
            Key::ArrowDown => Some(Command::Next),
            Key::ArrowUp => Some(Command::Previous),
            Key::PageDown => Some(Command::NextPage),
            Key::PageUp => Some(Command::PreviousPage),
            _ => None,
        };
    }
    if mac && *modifiers == Modifiers::CTRL {
        return match key {
            Key::N => Some(Command::Next),
            Key::P => Some(Command::Previous),
            _ => None,
        };
    }
    None
}

fn label_job(
    ui: &Ui,
    label: &str,
    highlights: &[std::ops::Range<usize>],
    font: egui::FontId,
    foreground: Color32,
    highlight: Color32,
    deprecated: bool,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let bold_family = egui::FontFamily::Name(crate::font_families::EDITOR_BOLD_FAMILY.into());
    let bold = if ui.fonts(|fonts| fonts.definitions().families.contains_key(&bold_family)) {
        egui::FontId::new(font.size, bold_family)
    } else {
        font.clone()
    };
    let mut unit = 0;
    let mut characters = label.chars().peekable();
    while let Some(character) = characters.next() {
        let mut end = unit + character.len_utf16();
        let text = match character {
            '\r' => {
                if characters.peek() == Some(&'\n') {
                    characters.next();
                    end += 1;
                }
                "\u{23ce}".into()
            }
            '\n' => "\u{23ce}".into(),
            _ => character.to_string(),
        };
        let matched = !deprecated
            && highlights
                .iter()
                .any(|range| range.start < end && range.end > unit);
        let color = if matched {
            highlight
        } else if deprecated {
            foreground.gamma_multiply(DEPRECATED_OPACITY)
        } else {
            foreground
        };
        job.append(
            &text,
            0.0,
            egui::TextFormat {
                font_id: if matched { bold.clone() } else { font.clone() },
                color,
                strikethrough: if deprecated {
                    egui::Stroke::new(BORDER, foreground)
                } else {
                    egui::Stroke::NONE
                },
                ..Default::default()
            },
        );
        unit = end;
    }
    job
}

fn glyph(kind: CompletionItemKind) -> &'static str {
    match kind {
        CompletionItemKind::METHOD
        | CompletionItemKind::FUNCTION
        | CompletionItemKind::CONSTRUCTOR => "\u{ea8c}",
        CompletionItemKind::FIELD => "\u{eb5f}",
        CompletionItemKind::VARIABLE => "\u{ea88}",
        CompletionItemKind::CLASS => "\u{eb5b}",
        CompletionItemKind::INTERFACE => "\u{eb61}",
        CompletionItemKind::MODULE => "\u{ea8b}",
        CompletionItemKind::PROPERTY => "\u{eb65}",
        CompletionItemKind::UNIT => "\u{ea96}",
        CompletionItemKind::VALUE | CompletionItemKind::ENUM => "\u{ea95}",
        CompletionItemKind::KEYWORD => "\u{eb62}",
        CompletionItemKind::SNIPPET => "\u{eb66}",
        CompletionItemKind::COLOR => "\u{eb5c}",
        CompletionItemKind::FILE => "\u{eb60}",
        CompletionItemKind::REFERENCE => "\u{ea94}",
        CompletionItemKind::FOLDER => "\u{ea83}",
        CompletionItemKind::ENUM_MEMBER => "\u{eb5e}",
        CompletionItemKind::CONSTANT => "\u{eb5d}",
        CompletionItemKind::STRUCT => "\u{ea91}",
        CompletionItemKind::EVENT => "\u{ea86}",
        CompletionItemKind::OPERATOR => "\u{eb64}",
        CompletionItemKind::TYPE_PARAMETER => "\u{ea92}",
        _ => "\u{ea93}",
    }
}

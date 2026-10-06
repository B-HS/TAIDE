use std::sync::Arc;
use std::time::Duration;

use egui::os::OperatingSystem;
use egui::{
    Color32, Event, FontFamily, FontId, Id, ImeEvent, Key, Modifiers, Painter, Pos2, Rect,
    Response, Sense, Ui, Vec2, pos2, vec2,
};
use taide_native_editor::display_layout::VerticalLayout;
use taide_native_editor::display_map::DisplayMap;
use taide_native_editor::document::{DocumentId, DocumentSnapshot, EditorError};
use taide_native_editor::editing::{
    ClipboardText, Motion, clipboard_text, compose_text, cut, delete_backward, delete_forward,
    delete_to_line_start, delete_word, insert_line_break, move_selection_displayed, outdent, paste,
    reveal_position, select_all, tab, type_text,
};
use taide_native_editor::indent::{IndentOptions, resolve};
use taide_native_editor::line_tokens::{LineTokens, TokenStyleTable};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{
    Composition, Selection, SelectionSet, ViewId, ViewState, WrapAffinities,
};

use crate::editor_geometry::{
    EditorGeometry, Row, RowLayout, gutter_width, half_leading, scroll_x_revealing, wrap_settings,
};
use crate::editor_paint::{Carets, Layers};

const ROW_OVERSCAN: usize = 1;
const CENTER_DIVISOR: f32 = 2.0;
const FALLBACK_TAB_SIZE: u32 = 4;
const PAGE_OVERLAP_LINES: isize = 2;
const VERTICAL_SCROLLBAR_SIZE: f32 = 14.0;
const HORIZONTAL_SCROLLBAR_SIZE: f32 = 12.0;
const SCROLLBAR_MIN_SLIDER: f32 = 20.0;
const SCROLL_BEYOND_LAST_COLUMN: usize = 4;
const SCROLLBAR_HIDE_DELAY: f64 = 0.5;
const SCROLLBAR_FADE_IN: f32 = 0.1;
const SCROLLBAR_FADE_OUT: f32 = 0.8;
const CLIPBOARD_MEMORY: &str = "native-code-editor-clipboard";
const SCROLLBAR_FADE: &str = "scrollbar-fade";

type KeyboardInputRoute = (Vec<(Option<bool>, bool)>, (Option<bool>, bool));

#[derive(Clone)]
pub struct EditorAppearance {
    pub font: FontId,
    pub line_height: f32,
    pub horizontal_padding: f32,
    pub background: Color32,
    pub foreground: Color32,
    pub muted: Color32,
    pub selection: Color32,
    pub cursor: Color32,
    pub current_line: Color32,
    pub line_numbers: bool,
    pub indent: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RenderWhitespace {
    #[default]
    None,
    Boundary,
    Selection,
    All,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CursorStyle {
    #[default]
    LineThin,
    Line,
    Block,
    Underline,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CursorBlinking {
    #[default]
    Solid,
    Blink,
    Smooth,
    Phase,
    Expand,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EditorDisplayOptions {
    pub word_wrap: bool,
    pub render_whitespace: RenderWhitespace,
    pub rulers: Vec<u32>,
    pub cursor_style: CursorStyle,
    pub cursor_blinking: CursorBlinking,
    pub smooth_caret: bool,
    pub scroll_beyond_last_line: bool,
    pub smooth_scrolling: bool,
    pub sticky_scroll: bool,
    pub minimap: bool,
    pub folding: bool,
    pub bracket_pair_colorization: bool,
    pub bracket_pair_guides: bool,
    pub bold_family: Option<FontFamily>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EditorPresentation {
    pub options: EditorDisplayOptions,
}

#[derive(Debug, Clone, Copy)]
pub struct EditorTokens<'a> {
    pub revision: u64,
    pub lines: &'a LineTokens,
    pub styles: &'a TokenStyleTable,
}

impl EditorTokens<'_> {
    fn describes(&self, document: &DocumentSnapshot) -> bool {
        self.revision == document.revision && self.lines.line_count() == document.rope.len_lines()
    }
}

fn registered_family<'a>(ui: &Ui, family: Option<&'a FontFamily>) -> Option<&'a FontFamily> {
    family.filter(|family| ui.fonts(|fonts| fonts.definitions().families.contains_key(*family)))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScrollAxis {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy)]
struct ScrollbarDrag {
    axis: ScrollAxis,
    pointer: f32,
    slider: f32,
}

#[derive(Clone, Copy)]
struct RenderedViewport {
    scroll_top: f32,
    line_height: f32,
}

#[derive(Default, Clone)]
struct InputState {
    ime_revision: Option<u64>,
    widest_line: f32,
    widest_document: Option<DocumentId>,
    widest_wrap_column: Option<u32>,
    scrollbar_drag: Option<ScrollbarDrag>,
    scrolled_at: Option<f64>,
    rendered_viewport: Option<RenderedViewport>,
}

struct Projection<'a> {
    painter: Painter,
    appearance: &'a EditorAppearance,
    width: f32,
    wrap_tab_size: Option<u32>,
    cached: Option<Arc<DisplayMap>>,
    rendered_viewport: Option<RenderedViewport>,
    stable_scroll_top: Option<f32>,
}

impl Projection<'_> {
    fn map(&mut self, document: &DocumentSnapshot) -> &DisplayMap {
        self.map_with_stable_scroll_top(document).0
    }

    fn map_with_stable_scroll_top(
        &mut self,
        document: &DocumentSnapshot,
    ) -> (&DisplayMap, Option<f32>) {
        let wrap = self.wrap_tab_size.map(|tab_size| {
            let gutter = gutter_width(&self.painter, document.rope.len_lines(), self.appearance);
            wrap_settings(
                &self.painter,
                self.appearance,
                self.width - gutter,
                VERTICAL_SCROLLBAR_SIZE,
                tab_size,
            )
        });
        let viewport_start = self
            .cached
            .take_if(|map| map.wrap_settings() != wrap.as_ref())
            .filter(|stale| stale.revision() == document.revision)
            .zip(
                self.rendered_viewport
                    .filter(|rendered| rendered.scroll_top > 0.0),
            )
            .map(|(stale, rendered)| {
                let layout = VerticalLayout::new(rendered.line_height, stale.row_count());
                let row = layout.row_at(rendered.scroll_top);
                (
                    stale.segment(document, row).bytes.start,
                    rendered.scroll_top - layout.row_top(row),
                )
            });
        let map = Arc::make_mut(
            self.cached
                .get_or_insert_with(|| Arc::new(DisplayMap::build(document, wrap))),
        );
        map.refresh(document);
        if let Some((byte, delta)) = viewport_start {
            let layout = VerticalLayout::new(self.appearance.line_height, map.row_count());
            self.stable_scroll_top = Some(layout.row_top(map.row_of_byte(document, byte)) + delta);
        }
        (map, self.stable_scroll_top)
    }
}

struct InputContext<'a> {
    is_mac: bool,
    force_crlf: bool,
    page_lines: isize,
    indent: IndentOptions,
    clipboard: Option<ClipboardText>,
    projection: Projection<'a>,
}

pub struct EditorOutput {
    pub response: Response,
    pub save_requested: bool,
    pub changed: bool,
    pub rendered_lines: std::ops::Range<usize>,
    pub errors: Vec<EditorError>,
    pub geometry: EditorGeometry,
}

#[derive(Default)]
struct InputOutput {
    copied: Option<ClipboardText>,
    errors: Vec<EditorError>,
}

enum KeyAction {
    Move(Motion),
    SelectAll,
    Undo,
    Redo,
    DeleteBackward,
    DeleteForward,
    DeleteWord { forward: bool },
    DeleteToLineStart,
    LineBreak,
    Tab,
    Outdent,
    Escape,
}

struct PointerPress {
    down: bool,
    origin: Option<Pos2>,
    position: Option<Pos2>,
}

#[derive(Clone, Copy)]
struct Scrollbar {
    axis: ScrollAxis,
    track: Rect,
    slider_size: f32,
    ratio: f32,
    maximum: f32,
}

impl Scrollbar {
    fn new(axis: ScrollAxis, track: Rect, visible: f32, content: f32) -> Option<Self> {
        let length = Self::along(axis, track.size());
        if !(content > visible && visible > 0.0) {
            return None;
        }
        let slider_size = (visible * length / content)
            .floor()
            .max(SCROLLBAR_MIN_SLIDER);
        if length <= slider_size {
            return None;
        }
        Some(Self {
            axis,
            track,
            slider_size,
            ratio: (length - slider_size) / (content - visible),
            maximum: content - visible,
        })
    }

    fn along(axis: ScrollAxis, vector: Vec2) -> f32 {
        match axis {
            ScrollAxis::Vertical => vector.y,
            ScrollAxis::Horizontal => vector.x,
        }
    }

    fn slider_position(&self, scroll: f32) -> f32 {
        (scroll * self.ratio).round()
    }

    fn scroll_for_slider(&self, slider: f32) -> f32 {
        (slider / self.ratio).round().clamp(0.0, self.maximum)
    }

    fn slider_rect(&self, scroll: f32) -> Rect {
        let start = self.slider_position(scroll);
        let offset = match self.axis {
            ScrollAxis::Vertical => vec2(0.0, start),
            ScrollAxis::Horizontal => vec2(start, 0.0),
        };
        let size = match self.axis {
            ScrollAxis::Vertical => vec2(self.track.width(), self.slider_size),
            ScrollAxis::Horizontal => vec2(self.slider_size, self.track.height()),
        };
        Rect::from_min_size(self.track.min + offset, size)
    }

    fn drag(&self, state: &mut InputState, press: &PointerPress, scroll: f32) -> f32 {
        if !press.down {
            return scroll;
        }
        let along = |position: Pos2| Self::along(self.axis, position.to_vec2());
        let mut scroll = scroll;
        if state.scrollbar_drag.is_none()
            && let Some(origin) = press.origin
            && self.track.contains(origin)
        {
            let offset = along(origin) - along(self.track.min);
            let slider = self.slider_position(scroll);
            if !(slider..=slider + self.slider_size).contains(&offset) {
                scroll = self.scroll_for_slider(offset - self.slider_size / CENTER_DIVISOR);
            }
            state.scrollbar_drag = Some(ScrollbarDrag {
                axis: self.axis,
                pointer: along(origin),
                slider: self.slider_position(scroll),
            });
        }
        match (state.scrollbar_drag, press.position) {
            (Some(drag), Some(position))
                if drag.axis == self.axis && along(position) != drag.pointer =>
            {
                self.scroll_for_slider(drag.slider + along(position) - drag.pointer)
            }
            _ => scroll,
        }
    }
}

pub struct NativeEditor {
    pub appearance: EditorAppearance,
}

impl NativeEditor {
    pub fn with_indent(&self, options: IndentOptions) -> Self {
        let mut appearance = self.appearance.clone();
        appearance.indent = if options.insert_spaces {
            " ".repeat(options.tab_size as usize)
        } else {
            "\t".into()
        };
        Self { appearance }
    }

    pub fn indent_options(&self, document: &DocumentSnapshot) -> IndentOptions {
        let unit = &self.appearance.indent;
        if !unit.contains('\t') {
            return IndentOptions {
                tab_size: u32::try_from(unit.len()).unwrap_or(u32::MAX).max(1),
                insert_spaces: true,
            };
        }
        let resolved = resolve(
            &document.metadata.editor_config,
            IndentOptions {
                tab_size: FALLBACK_TAB_SIZE,
                insert_spaces: false,
            },
        );
        IndentOptions {
            tab_size: resolved.tab_size.max(1),
            insert_spaces: false,
        }
    }

    pub fn reveal(
        &self,
        ui: &Ui,
        store: &mut EditorStore,
        view: ViewId,
        line: f64,
        column: f64,
    ) -> Result<(), EditorError> {
        self.reveal_presented(
            ui,
            store,
            view,
            line,
            column,
            &EditorPresentation::default(),
        )
    }

    pub fn reveal_presented(
        &self,
        ui: &Ui,
        store: &mut EditorStore,
        view: ViewId,
        line: f64,
        column: f64,
        presentation: &EditorPresentation,
    ) -> Result<(), EditorError> {
        self.reveal_tokenized(ui, store, view, line, column, presentation, None)
    }

    pub fn reveal_tokenized(
        &self,
        ui: &Ui,
        store: &mut EditorStore,
        view: ViewId,
        line: f64,
        column: f64,
        presentation: &EditorPresentation,
        tokens: Option<EditorTokens<'_>>,
    ) -> Result<(), EditorError> {
        let appearance = &self.appearance;
        if !appearance.line_height.is_finite()
            || appearance.line_height <= 0.0
            || !appearance.horizontal_padding.is_finite()
            || appearance.horizontal_padding < 0.0
        {
            return Err(EditorError::InvalidBoundary);
        }
        let rect = ui.available_rect_before_wrap().intersect(ui.clip_rect());
        if !rect.is_finite() || rect.width() <= 0.0 || rect.height() <= 0.0 {
            return Err(EditorError::InvalidBoundary);
        }
        let cached = store.take_display(view)?;
        let byte = reveal_position(store, view, line, column)?;
        let current = store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .clone();
        let document = store.documents().snapshot(current.document)?;
        let tab_size = self.indent_options(&document).tab_size;
        let mut projection = Projection {
            painter: ui.painter().clone(),
            appearance,
            width: rect.width(),
            wrap_tab_size: presentation.options.word_wrap.then_some(tab_size),
            cached,
            rendered_viewport: None,
            stable_scroll_top: None,
        };
        let display = projection.map(&document);
        let layout = VerticalLayout::new(appearance.line_height, display.row_count());
        let index = display.row_of_byte(&document, byte);
        let row = RowLayout {
            painter: ui.painter(),
            document: &document,
            display,
            appearance,
            half_leading: half_leading(ui.painter(), appearance),
            tab_size,
            tokens: tokens.filter(|tokens| tokens.describes(&document)),
            bold_family: registered_family(ui, presentation.options.bold_family.as_ref()),
        }
        .row(index, Pos2::ZERO);
        let gutter = gutter_width(ui.painter(), document.rope.len_lines(), appearance);
        let text_width = (rect.width() - gutter).max(0.0);
        let mut scroll = current.scroll;
        scroll.x = scroll_x_revealing(scroll.x, row.caret(byte), text_width).max(0.0);
        let maximum = (layout.content_height() - rect.height()).max(0.0);
        scroll.y = (layout.row_center(index) - rect.height() / CENTER_DIVISOR).clamp(0.0, maximum);
        store.set_display(view, projection.cached)?;
        store.set_view_state(view, current.selection, scroll, current.folds)?;
        Ok(())
    }

    pub fn show(
        &self,
        ui: &mut Ui,
        store: &mut EditorStore,
        view: ViewId,
        request_focus: bool,
    ) -> Result<EditorOutput, EditorError> {
        let mut save_requested = false;
        let mut output = self.show_with_keymap(ui, store, view, request_focus, |_, event, _| {
            if matches!(event, Event::Key { key: Key::S, pressed: true, modifiers, .. } if modifiers.command) {
                save_requested = true;
                return true;
            }
            false
        })?;
        output.save_requested = save_requested;
        Ok(output)
    }

    pub fn show_with_keymap(
        &self,
        ui: &mut Ui,
        store: &mut EditorStore,
        view: ViewId,
        request_focus: bool,
        keymap: impl FnMut(&Ui, &Event, bool) -> bool,
    ) -> Result<EditorOutput, EditorError> {
        self.show_with_input_route(ui, store, view, request_focus, keymap, |_| None)
    }

    #[doc = "Renders with caller-provided (ownership, focus-lost-before-event) entries and (final-focus, focus-lost-after-events), preserving default routing when absent."]
    pub fn show_with_input_route(
        &self,
        ui: &mut Ui,
        store: &mut EditorStore,
        view: ViewId,
        request_focus: bool,
        keymap: impl FnMut(&Ui, &Event, bool) -> bool,
        route: impl FnOnce(&Response) -> Option<KeyboardInputRoute>,
    ) -> Result<EditorOutput, EditorError> {
        self.show_presented(
            ui,
            store,
            view,
            request_focus,
            keymap,
            route,
            &EditorPresentation::default(),
        )
    }

    pub fn show_presented(
        &self,
        ui: &mut Ui,
        store: &mut EditorStore,
        view: ViewId,
        request_focus: bool,
        keymap: impl FnMut(&Ui, &Event, bool) -> bool,
        route: impl FnOnce(&Response) -> Option<KeyboardInputRoute>,
        presentation: &EditorPresentation,
    ) -> Result<EditorOutput, EditorError> {
        self.show_tokenized(
            ui,
            store,
            view,
            request_focus,
            keymap,
            route,
            presentation,
            |_| None,
        )
    }

    pub fn show_tokenized<'tokens>(
        &self,
        ui: &mut Ui,
        store: &mut EditorStore,
        view: ViewId,
        request_focus: bool,
        mut keymap: impl FnMut(&Ui, &Event, bool) -> bool,
        route: impl FnOnce(&Response) -> Option<KeyboardInputRoute>,
        presentation: &EditorPresentation,
        tokens: impl FnOnce(&EditorStore) -> Option<EditorTokens<'tokens>>,
    ) -> Result<EditorOutput, EditorError> {
        let appearance = &self.appearance;
        if !appearance.line_height.is_finite()
            || appearance.line_height <= 0.0
            || !appearance.horizontal_padding.is_finite()
            || appearance.horizontal_padding < 0.0
        {
            return Err(EditorError::InvalidBoundary);
        }
        let id = ui.make_persistent_id(("native-code-editor", view));
        let rect = ui.available_rect_before_wrap().intersect(ui.clip_rect());
        let response = ui.interact(rect, id, Sense::click_and_drag());
        ui.allocate_rect(rect, Sense::hover());
        if ui.is_enabled() && (request_focus || response.clicked() || response.drag_started()) {
            response.request_focus();
        }
        let cached = store.take_display(view)?;
        let current = store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .clone();
        let previous = store.documents().snapshot(current.document)?;
        let mut input_state = ui
            .ctx()
            .data_mut(|data| data.get_temp::<InputState>(id).unwrap_or_default());
        let mut output = InputOutput::default();
        let os = ui.ctx().os();
        let indent = self.indent_options(&previous);
        let mut input_context = InputContext {
            is_mac: os.is_mac(),
            force_crlf: os == OperatingSystem::Windows,
            page_lines: ((rect.height() / appearance.line_height).floor() as isize
                - PAGE_OVERLAP_LINES)
                .max(1),
            indent,
            clipboard: ui
                .ctx()
                .data_mut(|data| data.get_temp(Id::new(CLIPBOARD_MEMORY))),
            projection: Projection {
                painter: ui.painter().clone(),
                appearance,
                width: rect.width(),
                wrap_tab_size: presentation.options.word_wrap.then_some(indent.tab_size),
                cached,
                rendered_viewport: input_state
                    .rendered_viewport
                    .filter(|rendered| rendered.scroll_top == current.scroll.y),
                stable_scroll_top: None,
            },
        };
        let (ownership, (routed_focus, lost_after_events)) = route(&response).unwrap_or_default();
        let focused = routed_focus.unwrap_or_else(|| response.has_focus());
        let has_owned_input = ownership.iter().any(|(owned, _)| *owned == Some(true));
        if (response.has_focus() || has_owned_input) && ui.is_enabled() {
            ui.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    id,
                    egui::EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                    },
                )
            });
            let events = ui.input_mut(|input| std::mem::take(&mut input.events));
            let mut remaining = Vec::new();
            for (index, event) in events.into_iter().enumerate() {
                let (owned, lost) = ownership.get(index).copied().unwrap_or_default();
                if lost {
                    input_state.ime_revision = None;
                    store.set_composition(view, None)?;
                }
                if !owned.unwrap_or_else(|| response.has_focus()) {
                    remaining.push(event);
                    continue;
                }
                let composing = store
                    .views()
                    .get(view)
                    .is_some_and(|view| view.composition.is_some());
                if keymap(ui, &event, composing) {
                    continue;
                }
                match self.input(
                    store,
                    view,
                    &event,
                    &mut input_state,
                    &mut output,
                    &mut input_context,
                ) {
                    Ok(true) => {}
                    Ok(false) => remaining.push(event),
                    Err(error) => output.errors.push(error),
                }
            }
            ui.input_mut(|input| input.events = remaining);
        }
        if lost_after_events || !focused || !ui.is_enabled() {
            input_state.ime_revision = None;
            store.set_composition(view, None)?;
        }
        if let Some(copied) = output.copied {
            ui.ctx().copy_text(copied.text.clone());
            ui.ctx()
                .data_mut(|data| data.insert_temp(Id::new(CLIPBOARD_MEMORY), copied));
        }
        let document = store.documents().snapshot(current.document)?;
        let tokens = tokens(store).filter(|tokens| tokens.describes(&document));
        let mut state = store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .clone();
        let mut projection = input_context.projection;
        let (display, stable_scroll_top) = projection.map_with_stable_scroll_top(&document);
        let wrap_column = display.wrap_settings().map(|settings| settings.wrap_column);
        let head_row = |state: &ViewState, selection: usize| {
            display.row_of_head(
                &document,
                state.selection.selections[selection].head,
                state.head_at_row_end(selection, document.revision),
            )
        };
        let layout = VerticalLayout::new(appearance.line_height, display.row_count());
        let content_height = layout.content_height();
        let scroll_max = (content_height - rect.height()).max(0.0);
        state.scroll.y = stable_scroll_top.unwrap_or(state.scroll.y).min(scroll_max);
        let moved = document.revision != previous.revision || state.selection != current.selection;
        let head = state.selection.selections[state.selection.primary].head;
        let caret_row = head_row(&state, state.selection.primary);
        if moved {
            let top = layout.row_top(caret_row);
            if top < state.scroll.y {
                state.scroll.y = top;
            }
            let bottom = layout.row_bottom(caret_row);
            if bottom > state.scroll.y + rect.height() {
                state.scroll.y = (bottom - rect.height()).max(0.0);
            }
        }
        let wheel = if response.hovered() && ui.is_enabled() {
            ui.input_mut(|input| std::mem::take(&mut input.smooth_scroll_delta))
        } else {
            Vec2::ZERO
        };
        state.scroll.y = (state.scroll.y - wheel.y).clamp(0.0, scroll_max);
        let press = PointerPress {
            down: ui.is_enabled()
                && response.is_pointer_button_down_on()
                && ui.input(|input| input.pointer.primary_down()),
            origin: ui.input(|input| input.pointer.press_origin()),
            position: response.interact_pointer_pos(),
        };
        let was_scrolling = input_state.scrollbar_drag.is_some();
        let vertical = Scrollbar::new(
            ScrollAxis::Vertical,
            Rect::from_min_max(
                pos2(
                    (rect.right() - VERTICAL_SCROLLBAR_SIZE).max(rect.left()),
                    rect.top(),
                ),
                rect.max,
            ),
            rect.height(),
            content_height,
        );
        if let Some(scrollbar) = &vertical {
            state.scroll.y = scrollbar.drag(&mut input_state, &press, state.scroll.y);
        }
        let painter = ui.painter().with_clip_rect(rect);
        let gutter = gutter_width(&painter, document.rope.len_lines(), appearance);
        let text_rect = Rect::from_min_max(pos2(rect.left() + gutter, rect.top()), rect.max);
        let visible = layout.visible_rows(state.scroll.y, rect.height(), ROW_OVERSCAN);
        let row_layout = RowLayout {
            painter: &painter,
            document: &document,
            display,
            appearance,
            half_leading: half_leading(&painter, appearance),
            tab_size: indent.tab_size,
            tokens,
            bold_family: registered_family(ui, presentation.options.bold_family.as_ref()),
        };
        let mut rows: Vec<Row> = visible
            .clone()
            .map(|index| {
                row_layout.row(
                    index,
                    pos2(
                        text_rect.left(),
                        rect.top() + layout.row_top(index) - state.scroll.y,
                    ),
                )
            })
            .collect();
        if input_state.widest_document != Some(document.id)
            || input_state.widest_wrap_column != wrap_column
            || (visible.start == 0 && visible.end == display.row_count())
        {
            input_state.widest_document = Some(document.id);
            input_state.widest_wrap_column = wrap_column;
            input_state.widest_line = 0.0;
        }
        input_state.widest_line = rows
            .iter()
            .map(|row| row.galley.size().x.ceil())
            .fold(input_state.widest_line, f32::max);
        let beyond_last_column = painter
            .layout_no_wrap(
                " ".repeat(SCROLL_BEYOND_LAST_COLUMN),
                appearance.font.clone(),
                appearance.foreground,
            )
            .size()
            .x;
        let text_width = text_rect.width().max(0.0);
        let content_width = if wrap_column.is_some() {
            input_state.widest_line
        } else {
            input_state.widest_line + beyond_last_column + VERTICAL_SCROLLBAR_SIZE
        };
        let scroll_x_max = (content_width - text_width).max(0.0);
        state.scroll.x -= wheel.x;
        if moved && let Some(row) = rows.iter().find(|row| row.index == caret_row) {
            state.scroll.x = scroll_x_revealing(
                state.scroll.x,
                row.caret(head.min(row.segment.bytes.end)),
                text_width,
            );
        }
        state.scroll.x = state.scroll.x.clamp(0.0, scroll_x_max);
        let horizontal = Scrollbar::new(
            ScrollAxis::Horizontal,
            Rect::from_min_max(
                pos2(
                    text_rect.left(),
                    (rect.bottom() - HORIZONTAL_SCROLLBAR_SIZE).max(rect.top()),
                ),
                pos2(
                    (rect.right() - VERTICAL_SCROLLBAR_SIZE).max(text_rect.left()),
                    rect.bottom(),
                ),
            ),
            text_width,
            content_width,
        );
        if let Some(scrollbar) = &horizontal {
            state.scroll.x = scrollbar.drag(&mut input_state, &press, state.scroll.x);
        }
        let scrolling = was_scrolling || input_state.scrollbar_drag.is_some();
        if !press.down {
            input_state.scrollbar_drag = None;
        }
        for row in &mut rows {
            row.origin.x -= state.scroll.x;
        }
        store.set_view_state(
            view,
            state.selection.clone(),
            state.scroll.clone(),
            state.folds.clone(),
        )?;
        if !scrolling
            && (response.clicked() || response.drag_started() || response.dragged())
            && let Some(pointer) = response.interact_pointer_pos()
            && !rows.is_empty()
        {
            let target = layout
                .row_at(pointer.y - rect.top() + state.scroll.y)
                .clamp(rows[0].index, rows[rows.len() - 1].index);
            if let Some(row) = rows.iter().find(|row| row.index == target) {
                let head = row.byte_at(pointer);
                let extending = response.dragged() || ui.input(|input| input.modifiers.shift);
                let anchor = if extending {
                    state.selection.selections[state.selection.primary].anchor
                } else {
                    head
                };
                state.selection = SelectionSet {
                    primary: 0,
                    selections: vec![Selection { anchor, head }],
                };
                state.wrap_affinities =
                    (display.row_of_byte(&document, head) != row.index).then(|| WrapAffinities {
                        revision: document.revision,
                        heads_at_row_end: vec![true],
                    });
                state.composition = None;
                input_state.ime_revision = None;
                store.set_composition(view, None)?;
                ui.memory_mut(|memory| memory.interrupt_ime());
                store.break_undo_group(document.id)?;
                store.set_view_state(
                    view,
                    state.selection.clone(),
                    state.scroll.clone(),
                    state.folds.clone(),
                )?;
                store.set_wrap_affinities(view, state.wrap_affinities.clone())?;
            }
        }
        let text_painter = painter.with_clip_rect(text_rect);
        let layers = Layers {
            painter: &painter,
            text_painter: &text_painter,
            rect,
            text_rect,
            gutter,
            appearance,
        };
        let primary = state.selection.selections[state.selection.primary];
        let head_rows: Vec<usize> = (0..state.selection.selections.len())
            .map(|selection| head_row(&state, selection))
            .collect();
        let primary_row = head_rows[state.selection.primary];
        let carets = Carets {
            selections: &state.selection,
            head_rows: &head_rows,
            primary_row,
            primary_line: display.segment(&document, primary_row).line,
            focused,
        };
        layers.background();
        for row in &rows {
            layers.row(row, &carets);
        }
        let caret_rect = rows
            .iter()
            .find(|row| row.index == carets.primary_row)
            .map(|row| row.caret_rect(primary.head.min(row.segment.bytes.end)));
        if focused
            && ui.is_enabled()
            && !document.metadata.read_only
            && let Some(cursor_rect) = caret_rect
        {
            if let Some(composition) = &state.composition {
                layers.composition(cursor_rect, &composition.preedit);
            }
            let transform = ui
                .ctx()
                .layer_transform_to_global(ui.layer_id())
                .unwrap_or_default();
            ui.output_mut(|output| {
                output.mutable_text_under_cursor = response.hovered();
                output.ime = Some(egui::output::IMEOutput {
                    purpose: egui::IMEPurpose::Normal,
                    rect: transform * rect,
                    cursor_rect: transform * cursor_rect,
                    should_interrupt_composition: false,
                });
            });
        }
        let time = ui.input(|input| input.time);
        if state.scroll != current.scroll {
            input_state.scrolled_at = Some(time);
        }
        let dragged_axis = input_state.scrollbar_drag.map(|drag| drag.axis);
        let hide_after = input_state
            .scrolled_at
            .map(|scrolled_at| SCROLLBAR_HIDE_DELAY - (time - scrolled_at))
            .filter(|remaining| *remaining > 0.0);
        let revealed = ui.is_enabled()
            && (response.hovered() || dragged_axis.is_some() || hide_after.is_some());
        let opacity = ui.ctx().animate_bool_with_time(
            id.with(SCROLLBAR_FADE),
            revealed,
            if revealed {
                SCROLLBAR_FADE_IN
            } else {
                SCROLLBAR_FADE_OUT
            },
        );
        if let Some(remaining) = hide_after {
            ui.ctx()
                .request_repaint_after(Duration::from_secs_f64(remaining));
        }
        if opacity > 0.0 {
            let pointer = response.hover_pos();
            for (scrollbar, scroll) in [(vertical, state.scroll.y), (horizontal, state.scroll.x)] {
                let Some(scrollbar) = scrollbar else {
                    continue;
                };
                let slider = scrollbar.slider_rect(scroll);
                let engaged = dragged_axis == Some(scrollbar.axis)
                    || pointer.is_some_and(|pointer| slider.contains(pointer));
                layers.scrollbar(slider, engaged, opacity);
            }
        }
        let rendered_lines = rows
            .first()
            .zip(rows.last())
            .map_or(visible.clone(), |(first, last)| {
                first.segment.line..last.segment.line + 1
            });
        store.set_display(view, projection.cached)?;
        input_state.rendered_viewport = Some(RenderedViewport {
            scroll_top: state.scroll.y,
            line_height: appearance.line_height,
        });
        ui.ctx().data_mut(|data| data.insert_temp(id, input_state));
        Ok(EditorOutput {
            response,
            save_requested: false,
            changed: document.revision != previous.revision,
            rendered_lines,
            errors: output.errors,
            geometry: EditorGeometry {
                rect,
                content_rect: text_rect,
                gutter_rect: Rect::from_min_max(rect.min, pos2(text_rect.left(), rect.bottom())),
                line_height: appearance.line_height,
                visible_rows: visible,
                scroll: vec2(state.scroll.x, state.scroll.y),
            },
        })
    }

    fn input(
        &self,
        store: &mut EditorStore,
        view: ViewId,
        event: &Event,
        state: &mut InputState,
        output: &mut InputOutput,
        context: &mut InputContext<'_>,
    ) -> Result<bool, EditorError> {
        let current = store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .clone();
        let document = store.documents().snapshot(current.document)?;
        match event {
            Event::Copy => {
                if let Some(copied) = clipboard_text(store, view, context.force_crlf)? {
                    output.copied = Some(copied);
                }
            }
            Event::Cut => {
                if let Some(copied) = clipboard_text(store, view, context.force_crlf)? {
                    output.copied = Some(copied);
                }
                if !document.metadata.read_only {
                    cut(store, view)?;
                }
            }
            Event::Paste(text) => {
                state.ime_revision = None;
                let source = output
                    .copied
                    .as_ref()
                    .or(context.clipboard.as_ref())
                    .filter(|copied| is_same_clipboard_text(&copied.text, text));
                paste(store, view, text, source)?;
            }
            Event::Text(text) if current.composition.is_none() => {
                type_text(store, view, text)?;
            }
            Event::Text(_) => {}
            Event::Ime(ImeEvent::Preedit { text, .. }) => {
                if document.metadata.read_only {
                    return Err(EditorError::ReadOnly);
                }
                if state
                    .ime_revision
                    .is_some_and(|revision| revision != document.revision)
                {
                    state.ime_revision = None;
                    store.set_composition(view, None)?;
                    return Err(EditorError::StaleRevision);
                }
                let primary = current.selection.selections[current.selection.primary];
                let replace = current
                    .composition
                    .map(|composition| composition.replace)
                    .unwrap_or(primary.anchor.min(primary.head)..primary.anchor.max(primary.head));
                state.ime_revision = Some(document.revision);
                if text.is_empty() {
                    store.set_composition(view, None)?;
                } else {
                    store.set_composition(
                        view,
                        Some(Composition {
                            revision: document.revision,
                            replace,
                            preedit: text.clone(),
                        }),
                    )?;
                }
            }
            Event::Ime(ImeEvent::Commit(text)) => {
                let revision = state.ime_revision.take();
                if revision.is_some_and(|revision| revision != document.revision) {
                    store.set_composition(view, None)?;
                    return Err(EditorError::StaleRevision);
                }
                if !text.is_empty() {
                    if let Some(composition) = current.composition {
                        compose_text(store, view, composition.replace, text)?;
                    } else {
                        type_text(store, view, text)?;
                    }
                }
                store.set_composition(view, None)?;
            }
            Event::Ime(ImeEvent::DeleteSurrounding {
                before_chars,
                after_chars,
            }) => {
                let primary = current.selection.selections[current.selection.primary];
                let scalar = document.rope.byte_to_char(primary.head);
                let start = document
                    .rope
                    .char_to_byte(scalar.saturating_sub(*before_chars));
                let end = document.rope.char_to_byte(
                    scalar
                        .saturating_add(*after_chars)
                        .min(document.rope.len_chars()),
                );
                compose_text(store, view, start..end, "")?;
            }
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                let action = match command_action(*key, *modifiers) {
                    Some(action) => action,
                    None if current.composition.is_some() => return Ok(false),
                    None => match key_action(*key, *modifiers, context) {
                        Some(action) => action,
                        None => return Ok(false),
                    },
                };
                match action {
                    KeyAction::Move(motion) => move_selection_displayed(
                        store,
                        view,
                        motion,
                        modifiers.shift,
                        context.projection.map(&document),
                    )?,
                    KeyAction::SelectAll => select_all(store, view)?,
                    KeyAction::Undo => {
                        store.undo(document.id)?;
                    }
                    KeyAction::Redo => {
                        store.redo(document.id)?;
                    }
                    KeyAction::DeleteBackward => {
                        delete_backward(store, view, context.indent)?;
                    }
                    KeyAction::DeleteForward => {
                        delete_forward(store, view)?;
                    }
                    KeyAction::DeleteWord { forward } => {
                        delete_word(store, view, forward)?;
                    }
                    KeyAction::DeleteToLineStart => {
                        delete_to_line_start(store, view)?;
                    }
                    KeyAction::LineBreak => {
                        insert_line_break(store, view, context.indent)?;
                    }
                    KeyAction::Tab => {
                        tab(store, view, context.indent)?;
                    }
                    KeyAction::Outdent => {
                        outdent(store, view, context.indent)?;
                    }
                    KeyAction::Escape => {
                        let primary = current.selection.selections[current.selection.primary];
                        if current.selection.selections.len() == 1 && primary.anchor == primary.head
                        {
                            return Ok(false);
                        }
                        let selection = if current.selection.selections.len() > 1 {
                            primary
                        } else {
                            Selection {
                                anchor: primary.head,
                                head: primary.head,
                            }
                        };
                        let affinities = current
                            .head_at_row_end(current.selection.primary, document.revision)
                            .then(|| WrapAffinities {
                                revision: document.revision,
                                heads_at_row_end: vec![true],
                            });
                        store.break_undo_group(document.id)?;
                        store.set_view_state(
                            view,
                            SelectionSet {
                                primary: 0,
                                selections: vec![selection],
                            },
                            current.scroll,
                            current.folds,
                        )?;
                        store.set_wrap_affinities(view, affinities)?;
                    }
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}

fn command_action(key: Key, modifiers: Modifiers) -> Option<KeyAction> {
    if !modifiers.command {
        return None;
    }
    Some(match key {
        Key::A => KeyAction::SelectAll,
        Key::Z if modifiers.shift => KeyAction::Redo,
        Key::Z => KeyAction::Undo,
        Key::Y => KeyAction::Redo,
        Key::Home => KeyAction::Move(Motion::DocumentStart),
        Key::End => KeyAction::Move(Motion::DocumentEnd),
        _ => return None,
    })
}

fn key_action(key: Key, modifiers: Modifiers, context: &InputContext<'_>) -> Option<KeyAction> {
    let line_chord = context.is_mac && modifiers.mac_cmd && !modifiers.alt && !modifiers.ctrl;
    let word_chord = if context.is_mac {
        modifiers.alt && !modifiers.ctrl && !modifiers.command
    } else {
        modifiers.ctrl && !modifiers.alt && !modifiers.mac_cmd
    };
    let is_plain = !(modifiers.alt || modifiers.ctrl || modifiers.mac_cmd || modifiers.command);
    let vertical = |lines| {
        KeyAction::Move(Motion::Vertical {
            lines,
            tab_size: context.indent.tab_size,
        })
    };
    Some(match key {
        Key::ArrowLeft if line_chord => KeyAction::Move(Motion::LineStart),
        Key::ArrowRight if line_chord => KeyAction::Move(Motion::LineEnd),
        Key::ArrowUp if line_chord => KeyAction::Move(Motion::DocumentStart),
        Key::ArrowDown if line_chord => KeyAction::Move(Motion::DocumentEnd),
        Key::Backspace if line_chord && !modifiers.shift => KeyAction::DeleteToLineStart,
        Key::ArrowLeft if word_chord => KeyAction::Move(Motion::WordLeft),
        Key::ArrowRight if word_chord => KeyAction::Move(Motion::WordRight),
        Key::Backspace if word_chord && !modifiers.shift => {
            KeyAction::DeleteWord { forward: false }
        }
        Key::Delete if word_chord && !modifiers.shift => KeyAction::DeleteWord { forward: true },
        _ if !is_plain => return None,
        Key::ArrowLeft => KeyAction::Move(Motion::Left),
        Key::ArrowRight => KeyAction::Move(Motion::Right),
        Key::ArrowUp => vertical(-1),
        Key::ArrowDown => vertical(1),
        Key::PageUp => vertical(-context.page_lines),
        Key::PageDown => vertical(context.page_lines),
        Key::Home => KeyAction::Move(Motion::LineStart),
        Key::End => KeyAction::Move(Motion::LineEnd),
        Key::Backspace => KeyAction::DeleteBackward,
        Key::Delete => KeyAction::DeleteForward,
        Key::Enter => KeyAction::LineBreak,
        Key::Tab if modifiers.shift => KeyAction::Outdent,
        Key::Tab => KeyAction::Tab,
        Key::Escape => KeyAction::Escape,
        _ => return None,
    })
}

fn is_same_clipboard_text(copied: &str, pasted: &str) -> bool {
    copied.replace("\r\n", "\n") == pasted.replace("\r\n", "\n")
}

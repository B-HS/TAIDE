use std::borrow::Cow;
use std::ops::Range;
use std::sync::Arc;
use std::time::Duration;

use egui::os::OperatingSystem;
use egui::{
    Color32, Event, FontFamily, FontId, Id, ImeEvent, Key, Modifiers, Painter, PointerButton, Pos2,
    Rect, Response, Sense, Ui, Vec2, pos2, vec2,
};
use taide_native_editor::auto_closing::AutoClosedPairs;
use taide_native_editor::decoration::DecorationLayer;
use taide_native_editor::display_layout::VerticalLayout;
use taide_native_editor::display_map::DisplayMap;
use taide_native_editor::document::{DocumentId, DocumentSnapshot, EditorError};
use taide_native_editor::editing::{
    ClipboardText, Motion, clipboard_text, compose_text, cut, delete_forward, delete_to_line_start,
    delete_word, move_selection_displayed, paste, reveal_position, select_all, tab_with_language,
};
use taide_native_editor::folding::{
    FoldClick, FoldCommand, FoldRegion, FoldToggle, FoldingModel, MAX_FOLDING_REGIONS, click_fold,
    hidden_lines, indent_regions, language_regions, reconcile_folds, reveal_carets,
    run_fold_command, run_language_fold_command,
};
use taide_native_editor::indent::{IndentOptions, resolve};
use taide_native_editor::language_configuration::{Language, LanguageRules};
use taide_native_editor::language_typing::{
    Typing, commit_composition, delete_backward, insert_line_break, type_text,
};
use taide_native_editor::line_tokens::{LineTokens, TokenStyleTable};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{
    Composition, Selection, SelectionSet, ViewId, ViewState, WrapAffinities,
};

use crate::editor_geometry::{
    EditorGeometry, FOLD_PLACEHOLDER, FOLD_PLACEHOLDER_MARGIN_EM, Row, RowLayout, half_leading,
    scroll_x_revealing, wrap_settings,
};
use crate::editor_gutter::{FoldControlFade, Gutter};
use crate::editor_paint::{Carets, Layers, frame_decorations};
use crate::editor_pointer::{PointerInput, PointerSelection};

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
const COLLAPSED_CHEVRON_ROTATION: f32 = 0.0;
const EXPANDED_CHEVRON_ROTATION: f32 = std::f32::consts::FRAC_PI_2;

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
    #[cfg(feature = "native-host")]
    pub colors: Option<crate::editor_display::EditorDisplayColors>,
    #[cfg(feature = "native-host")]
    pub bracket_colors: Option<crate::editor_brackets::EditorBracketColors>,
    #[cfg(feature = "native-host")]
    pub bracket_widget_focus: bool,
    #[cfg(feature = "native-host")]
    pub sticky_colors: Option<crate::editor_sticky_scroll::EditorStickyColors>,
    #[cfg(feature = "native-host")]
    pub sticky_model: Option<Arc<taide_native_editor::sticky_model::StickyModel>>,
    #[cfg(feature = "native-host")]
    pub sticky_toggle_label: Option<String>,
    #[cfg(feature = "native-host")]
    pub minimap_colors: Option<crate::editor_minimap::EditorMinimapColors>,
    #[cfg(feature = "native-host")]
    pub diagnostic_colors: Option<crate::editor_diagnostics::DiagnosticColors>,
    #[cfg(feature = "native-host")]
    pub diagnostics: Option<Arc<taide_native_editor::diagnostics::MarkerSet>>,
    #[cfg(feature = "native-host")]
    pub overview_colors: Option<crate::editor_overview::OverviewColors>,
    #[cfg(feature = "native-host")]
    pub problem_colors: Option<crate::editor_problems::Colors>,
    #[cfg(feature = "native-host")]
    pub location_colors: Option<crate::editor_locations::Colors>,
    #[cfg(feature = "native-host")]
    pub documentation_colors: Option<crate::editor_documentation::Colors>,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoldControl {
    pub rect: Rect,
    pub chevron_rotation: f32,
    pub color: Color32,
}

pub type FoldControlPainter<'a> = dyn FnMut(&Ui, FoldControl) + 'a;

pub struct EditorRequest<'a, Keymap, Route, Tokens> {
    pub request_focus: bool,
    pub keymap: Keymap,
    pub route: Route,
    pub presentation: &'a EditorPresentation,
    pub tokens: Tokens,
    pub language: Option<Language<'a>>,
    pub decorations: &'a [&'a DecorationLayer],
    pub fold_commands: &'a [FoldCommand],
    pub fold_controls: Option<&'a mut FoldControlPainter<'a>>,
    #[cfg(feature = "native-host")]
    pub problems: Option<&'a mut dyn crate::editor_problems::Provider>,
    #[cfg(feature = "native-host")]
    pub locations: Option<&'a mut dyn crate::editor_locations::Provider>,
    #[cfg(feature = "native-host")]
    pub documentation: Option<&'a mut dyn crate::editor_documentation::Provider>,
    #[cfg(feature = "native-host")]
    pub documentation_commands: &'a [taide_native_editor::documentation::Command],
    #[cfg(feature = "native-host")]
    pub syntax_folds: Option<Arc<taide_native_editor::syntax_folding::SyntaxFolds>>,
}

#[cfg(feature = "native-host")]
fn execute_problem_shortcut(
    problems: &mut Option<&mut dyn crate::editor_problems::Provider>,
    store: &mut EditorStore,
    view: ViewId,
    event: &Event,
    composing: bool,
) -> Result<bool, EditorError> {
    let Some(provider) = problems.as_deref_mut() else {
        return Ok(false);
    };
    let visible = provider.current(store, view).is_some();
    let Some(command) = crate::editor_problems::shortcut(event, composing, visible) else {
        return Ok(false);
    };
    provider.execute(store, view, command)
}

fn tracked_layers<'a>(
    store: &EditorStore,
    document: &DocumentSnapshot,
    layers: &[&'a DecorationLayer],
) -> Result<Vec<Cow<'a, DecorationLayer>>, EditorError> {
    let mut tracked = Vec::with_capacity(layers.len());
    for layer in layers {
        let changes = store.changes_since(document.id, layer.revision())?;
        tracked.extend(
            layer
                .tracking(changes)
                .filter(|layer| layer.revision() == document.revision),
        );
    }
    tracked.sort_by_key(|layer| layer.z_order());
    Ok(tracked)
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
    #[cfg(feature = "native-host")]
    problem_zone: Option<(usize, f32)>,
}

#[derive(Clone)]
struct FoldRegionCache {
    document: DocumentId,
    revision: u64,
    tab_size: u32,
    language_id: Option<String>,
    regions: Arc<[FoldRegion]>,
    #[cfg(feature = "native-host")]
    syntax: Option<Arc<taide_native_editor::syntax_folding::SyntaxFolds>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct FoldPress {
    line: usize,
    is_on_control: bool,
}

#[derive(Default, Clone)]
struct InputState {
    #[cfg(feature = "native-host")]
    minimap: crate::editor_minimap::MinimapState,
    #[cfg(feature = "native-host")]
    sticky: crate::editor_sticky_scroll::StickyState,
    #[cfg(feature = "native-host")]
    problems: crate::editor_problems::State,
    #[cfg(feature = "native-host")]
    locations: crate::editor_locations::State,
    #[cfg(feature = "native-host")]
    definition_link: crate::editor_definition_link::State,
    #[cfg(feature = "native-host")]
    documentation: crate::editor_documentation::State,
    #[cfg(feature = "native-host")]
    caret: crate::editor_caret::CaretState,
    #[cfg(feature = "native-host")]
    scroll: crate::editor_scroll::ScrollState,
    ime_revision: Option<u64>,
    widest_line: f32,
    widest_document: Option<DocumentId>,
    widest_wrap_column: Option<u32>,
    scrollbar_drag: Option<ScrollbarDrag>,
    scrolled_at: Option<f64>,
    rendered_viewport: Option<RenderedViewport>,
    fold_regions: Option<FoldRegionCache>,
    #[cfg(feature = "native-host")]
    syntax_folds: Option<Arc<taide_native_editor::syntax_folding::SyntaxFolds>>,
    fold_press: Option<FoldPress>,
    clicked_fold_line: Option<usize>,
    fold_control_fade: FoldControlFade,
    auto_closed: AutoClosedPairs,
    pointer_selection: PointerSelection,
}

impl InputState {
    fn fold_regions(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
        tab_size: u32,
        rules: Option<&dyn LanguageRules>,
    ) -> Result<Arc<[FoldRegion]>, EditorError> {
        let owner = store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .document;
        let document = store.documents().snapshot(owner)?;
        let language_id = rules.map(|_| document.metadata.language_id.as_str());
        let cached = self.fold_regions.as_ref().filter(|cache| {
            #[cfg(feature = "native-host")]
            if !match (&cache.syntax, &self.syntax_folds) {
                (Some(previous), Some(current)) => Arc::ptr_eq(previous, current),
                (None, None) => true,
                _ => false,
            } {
                return false;
            }
            cache.document == document.id
                && cache.revision == document.revision
                && cache.tab_size == tab_size
                && cache.language_id.as_deref() == language_id
        });
        if let Some(cache) = cached {
            return Ok(Arc::clone(&cache.regions));
        }
        let fallback = || -> Arc<[FoldRegion]> {
            match rules {
                Some(rules) => {
                    language_regions(&document.rope, tab_size, MAX_FOLDING_REGIONS, rules)
                }
                None => indent_regions(&document.rope, tab_size, MAX_FOLDING_REGIONS),
            }
            .into()
        };
        #[cfg(feature = "native-host")]
        let regions = self
            .syntax_folds
            .as_ref()
            .filter(|syntax| syntax.describes(&document))
            .map_or_else(fallback, |syntax| Arc::clone(syntax.regions()));
        #[cfg(not(feature = "native-host"))]
        let regions = fallback();
        reconcile_folds(store, view, &regions)?;
        self.fold_regions = Some(FoldRegionCache {
            document: document.id,
            revision: document.revision,
            tab_size,
            language_id: language_id.map(str::to_owned),
            regions: Arc::clone(&regions),
            #[cfg(feature = "native-host")]
            syntax: self.syntax_folds.clone(),
        });
        Ok(regions)
    }
}

#[cfg(feature = "native-host")]
fn apply_sticky_action(
    ui: &Ui,
    store: &mut EditorStore,
    view: ViewId,
    input: &mut InputState,
    action: crate::editor_sticky_scroll::StickyAction,
    tab_size: u32,
    rules: Option<&dyn LanguageRules>,
) -> Result<bool, EditorError> {
    use crate::editor_sticky_scroll::StickyAction;
    let current = store.views().get(view).ok_or(EditorError::NotFound)?;
    let document = store.documents().snapshot(current.document)?;
    let Some(action) = input.sticky.track_action(store, &document, action)? else {
        ui.ctx().request_repaint();
        return Ok(false);
    };
    match action {
        StickyAction::Focus(_) => {}
        StickyAction::Exit => {
            return Ok(true);
        }
        StickyAction::Jump { byte, center } => {
            let current = store
                .views()
                .get(view)
                .ok_or(EditorError::NotFound)?
                .clone();
            store.set_composition(view, None)?;
            input.ime_revision = None;
            store.break_undo_group(current.document)?;
            store.set_view_state(
                view,
                SelectionSet {
                    selections: vec![Selection {
                        anchor: byte,
                        head: byte,
                    }],
                    primary: 0,
                },
                current.scroll,
                current.folds,
            )?;
            store.request_selection_reveal(view, byte..byte, center)?;
            ui.ctx().request_repaint();
            return Ok(true);
        }
        StickyAction::Fold { line, end, index } => {
            let regions = input.fold_regions(store, view, tab_size, rules)?;
            let current = store.views().get(view).ok_or(EditorError::NotFound)?;
            let document = store.documents().snapshot(current.document)?;
            let collapsed = FoldingModel::with_manual(
                &regions,
                &document,
                &current.folds,
                &current.manual_folds,
            )
            .header(line)
                == Some(true);
            if click_fold(
                store,
                view,
                &regions,
                FoldClick {
                    line,
                    is_on_control: true,
                    toggle: FoldToggle::Region,
                },
            )? {
                input
                    .sticky
                    .reveal_fold(if collapsed { line } else { end }, index);
                ui.ctx().request_repaint();
                return Ok(true);
            }
        }
    }
    Ok(false)
}

#[derive(Clone, Copy)]
enum FoldReveal {
    Head,
    SelectionStart,
    LineStart(usize),
}

fn maintain_folds(
    store: &mut EditorStore,
    view: ViewId,
    state: &mut InputState,
    tab_size: Option<u32>,
    commands: &[FoldCommand],
    rules: Option<&dyn LanguageRules>,
) -> Result<(Arc<[FoldRegion]>, Option<FoldReveal>), EditorError> {
    let clicked_line = state.clicked_fold_line.take();
    let Some(tab_size) = tab_size else {
        state.fold_regions = None;
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        if !current.folds.is_empty() {
            let (selection, scroll) = (current.selection.clone(), current.scroll.clone());
            store.set_view_state(view, selection, scroll, Vec::new())?;
        }
        return Ok((Arc::from(Vec::new()), None));
    };
    let regions = state.fold_regions(store, view, tab_size, rules)?;
    let mut reveal = clicked_line.map(FoldReveal::LineStart);
    if reveal_carets(store, view)? {
        reveal = Some(FoldReveal::Head);
    }
    for command in commands {
        #[cfg(feature = "native-host")]
        if let Some(syntax) = state.syntax_folds.as_ref().filter(|syntax| {
            store
                .views()
                .get(view)
                .and_then(|view| store.documents().snapshot(view.document).ok())
                .is_some_and(|document| syntax.describes(&document))
        }) {
            if !syntax.has_kinds()
                && matches!(
                    command,
                    FoldCommand::FoldAllBlockComments
                        | FoldCommand::FoldAllMarkerRegions
                        | FoldCommand::UnfoldAllMarkerRegions
                )
                && let Some(rules) = rules
            {
                run_language_fold_command(store, view, &regions, *command, rules)?;
                reveal = Some(FoldReveal::SelectionStart);
                continue;
            }
            taide_native_editor::folding::run_syntax_fold_command(store, view, syntax, *command)?;
            reveal = Some(FoldReveal::SelectionStart);
            continue;
        }
        match rules {
            Some(rules) => run_language_fold_command(store, view, &regions, *command, rules)?,
            None => run_fold_command(store, view, &regions, *command)?,
        };
        reveal = Some(FoldReveal::SelectionStart);
    }
    Ok((regions, reveal))
}

struct Projection<'a> {
    painter: Painter,
    appearance: &'a EditorAppearance,
    width: f32,
    wrap_tab_size: Option<u32>,
    has_folding: bool,
    cached: Option<Arc<DisplayMap>>,
    rendered_viewport: Option<RenderedViewport>,
    stable_scroll_top: Option<f32>,
    #[cfg(feature = "native-host")]
    problem_zone: Option<(usize, f32)>,
}

impl Projection<'_> {
    fn map(&mut self, document: &DocumentSnapshot, folds: &[Range<usize>]) -> &DisplayMap {
        self.map_with_stable_scroll_top(document, folds).0
    }

    fn map_with_stable_scroll_top(
        &mut self,
        document: &DocumentSnapshot,
        folds: &[Range<usize>],
    ) -> (&DisplayMap, Option<f32>) {
        #[cfg(feature = "native-host")]
        let zone_changed = self
            .rendered_viewport
            .is_some_and(|rendered| rendered.problem_zone != self.problem_zone);
        #[cfg(not(feature = "native-host"))]
        let zone_changed = false;
        let wrap = self.wrap_tab_size.map(|tab_size| {
            let gutter = Gutter::measure(
                &self.painter,
                document.rope.len_lines(),
                self.appearance,
                self.has_folding,
            );
            wrap_settings(
                &self.painter,
                self.appearance,
                self.width - gutter.width(),
                VERTICAL_SCROLLBAR_SIZE,
                tab_size,
            )
        });
        let hidden = if self.has_folding {
            hidden_lines(&document.rope, folds)
        } else {
            Vec::new()
        };
        let viewport_start = self
            .cached
            .as_ref()
            .filter(|stale| {
                stale.revision() == document.revision
                    && (stale.wrap_settings() != wrap.as_ref()
                        || stale.hidden_lines() != hidden
                        || zone_changed)
            })
            .zip(
                self.rendered_viewport
                    .filter(|rendered| rendered.scroll_top > 0.0),
            )
            .map(|(stale, rendered)| {
                let layout = VerticalLayout::new(rendered.line_height, stale.row_count());
                #[cfg(feature = "native-host")]
                let layout = rendered.problem_zone.map_or_else(
                    || layout.clone(),
                    |(position, height)| {
                        layout.clone().with_zone(
                            stale.row_of_byte(document, position.min(document.rope.len_bytes())),
                            height,
                        )
                    },
                );
                let row = layout.row_at(rendered.scroll_top);
                (
                    stale.segment(document, row).bytes.start,
                    rendered.scroll_top - layout.row_top(row),
                )
            });
        self.cached
            .take_if(|map| map.wrap_settings() != wrap.as_ref());
        let map = Arc::make_mut(
            self.cached
                .get_or_insert_with(|| Arc::new(DisplayMap::build(document, wrap))),
        );
        map.refresh(document);
        map.set_hidden_lines(&hidden);
        if let Some((byte, delta)) = viewport_start
            && map
                .hidden_lines_at(document.rope.byte_to_line(byte))
                .is_none()
        {
            let layout = VerticalLayout::new(self.appearance.line_height, map.row_count());
            #[cfg(feature = "native-host")]
            let layout = self.problem_zone.map_or_else(
                || layout.clone(),
                |(position, height)| {
                    layout.clone().with_zone(
                        map.row_of_byte(document, position.min(document.rope.len_bytes())),
                        height,
                    )
                },
            );
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
    language: Option<Language<'a>>,
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
    #[cfg(feature = "native-host")]
    pub focus_ids: Vec<Id>,
    #[cfg(feature = "native-host")]
    pub toggle_sticky_scroll: bool,
    #[cfg(all(feature = "native-host", feature = "inspection"))]
    pub documentation_geometry: crate::editor_documentation::Geometry,
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
        if presentation.options.folding {
            reveal_carets(store, view)?;
        }
        let current = store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .clone();
        let document = store.documents().snapshot(current.document)?;
        let tab_size = self.indent_options(&document).tab_size;
        #[cfg(feature = "native-host")]
        let problem_zone = ui
            .ctx()
            .data(|data| {
                data.get_temp::<InputState>(ui.make_persistent_id(("native-code-editor", view)))
            })
            .and_then(|input| {
                input
                    .problems
                    .zone(store, &document, appearance.line_height, rect.height())
            });
        let mut projection = Projection {
            painter: ui.painter().clone(),
            appearance,
            width: rect.width(),
            wrap_tab_size: presentation.options.word_wrap.then_some(tab_size),
            has_folding: presentation.options.folding,
            cached,
            rendered_viewport: None,
            stable_scroll_top: None,
            #[cfg(feature = "native-host")]
            problem_zone,
        };
        let display = projection.map(&document, &current.folds);
        let layout = VerticalLayout::new(appearance.line_height, display.row_count());
        #[cfg(feature = "native-host")]
        let layout = problem_zone.map_or_else(
            || layout.clone(),
            |(position, height)| {
                layout
                    .clone()
                    .with_zone(display.row_of_byte(&document, position), height)
            },
        );
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
            decorations: &[],
            #[cfg(feature = "native-host")]
            brackets: None,
        }
        .row(index, Pos2::ZERO);
        let gutter = Gutter::measure(
            ui.painter(),
            document.rope.len_lines(),
            appearance,
            presentation.options.folding,
        );
        let text_width = (rect.width() - gutter.width()).max(0.0);
        let mut scroll = current.scroll.clone();
        scroll.x = scroll_x_revealing(scroll.x, row.caret(byte), text_width).max(0.0);
        #[cfg(feature = "native-host")]
        let height = crate::editor_scroll::content_height(
            layout.content_height(),
            rect.height(),
            appearance.line_height,
            presentation.options.scroll_beyond_last_line,
        );
        #[cfg(not(feature = "native-host"))]
        let height = layout.content_height();
        let maximum = (height - rect.height()).max(0.0);
        scroll.y = (layout.row_center(index) - rect.height() / CENTER_DIVISOR).clamp(0.0, maximum);
        #[cfg(feature = "native-host")]
        if presentation.options.smooth_scrolling {
            let id = ui.make_persistent_id(("native-code-editor", view));
            ui.ctx().data_mut(|data| {
                let mut input = data.get_temp::<InputState>(id).unwrap_or_default();
                input.scroll.reveal_from = Some(vec2(current.scroll.x, current.scroll.y));
                data.insert_temp(id, input);
            });
        }
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
        keymap: impl FnMut(&Ui, &Event, bool) -> bool,
        route: impl FnOnce(&Response) -> Option<KeyboardInputRoute>,
        presentation: &EditorPresentation,
        tokens: impl FnOnce(&EditorStore) -> Option<EditorTokens<'tokens>>,
    ) -> Result<EditorOutput, EditorError> {
        self.show_request(
            ui,
            store,
            view,
            EditorRequest {
                request_focus,
                keymap,
                route,
                presentation,
                tokens,
                language: None,
                decorations: &[],
                fold_commands: &[],
                fold_controls: None,
                #[cfg(feature = "native-host")]
                problems: None,
                #[cfg(feature = "native-host")]
                locations: None,
                #[cfg(feature = "native-host")]
                syntax_folds: None,
                #[cfg(feature = "native-host")]
                documentation: None,
                #[cfg(feature = "native-host")]
                documentation_commands: &[],
            },
        )
    }

    pub fn show_request<'tokens>(
        &self,
        ui: &mut Ui,
        store: &mut EditorStore,
        view: ViewId,
        request: EditorRequest<
            '_,
            impl FnMut(&Ui, &Event, bool) -> bool,
            impl FnOnce(&Response) -> Option<KeyboardInputRoute>,
            impl FnOnce(&EditorStore) -> Option<EditorTokens<'tokens>>,
        >,
    ) -> Result<EditorOutput, EditorError> {
        self.show_request_with_editor_keymap(ui, store, view, request, |_, _, _, _, _| false)
    }

    pub fn show_request_with_editor_keymap<'tokens>(
        &self,
        ui: &mut Ui,
        store: &mut EditorStore,
        view: ViewId,
        request: EditorRequest<
            '_,
            impl FnMut(&Ui, &Event, bool) -> bool,
            impl FnOnce(&Response) -> Option<KeyboardInputRoute>,
            impl FnOnce(&EditorStore) -> Option<EditorTokens<'tokens>>,
        >,
        mut editor_keymap: impl FnMut(&Ui, &mut EditorStore, ViewId, &Event, bool) -> bool,
    ) -> Result<EditorOutput, EditorError> {
        let EditorRequest {
            request_focus,
            mut keymap,
            route,
            presentation,
            tokens,
            language,
            decorations,
            fold_commands,
            mut fold_controls,
            #[cfg(feature = "native-host")]
            mut problems,
            #[cfg(feature = "native-host")]
            mut locations,
            #[cfg(feature = "native-host")]
            mut documentation,
            #[cfg(feature = "native-host")]
            documentation_commands,
            #[cfg(feature = "native-host")]
            syntax_folds,
        } = request;
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
        let current = store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .clone();
        let previous = store.documents().snapshot(current.document)?;
        #[cfg(feature = "native-host")]
        let minimap_dimensions = crate::editor_minimap::measure(
            ui,
            &previous,
            appearance,
            &presentation.options,
            rect,
            VERTICAL_SCROLLBAR_SIZE,
        );
        #[cfg(feature = "native-host")]
        let minimap_width = minimap_dimensions.map_or(0.0, |dimensions| dimensions.width as f32);
        #[cfg(not(feature = "native-host"))]
        let minimap_width = 0.0;
        #[cfg(feature = "native-host")]
        let has_overview = presentation.options.overview_colors.is_some();
        #[cfg(not(feature = "native-host"))]
        let has_overview = false;
        let editor_right = if minimap_width > 0.0 || has_overview {
            (rect.right() - minimap_width - VERTICAL_SCROLLBAR_SIZE).max(rect.left())
        } else {
            rect.right()
        };
        #[cfg(feature = "native-host")]
        let minimap_rect = minimap_dimensions.map(|_| {
            Rect::from_min_max(
                pos2(editor_right, rect.top()),
                pos2(rect.right() - VERTICAL_SCROLLBAR_SIZE, rect.bottom()),
            )
        });
        let response = ui.interact(
            Rect::from_min_max(rect.min, pos2(editor_right, rect.bottom())),
            id,
            Sense::click_and_drag(),
        );
        ui.allocate_rect(rect, Sense::hover());
        if ui.is_enabled() && (request_focus || response.clicked() || response.drag_started()) {
            response.request_focus();
        }
        let cached = store.take_display(view)?;
        let mut input_state = ui
            .ctx()
            .data_mut(|data| data.get_temp::<InputState>(id).unwrap_or_default());
        #[cfg(feature = "native-host")]
        {
            input_state.syntax_folds = syntax_folds.filter(|syntax| syntax.describes(&previous));
        }
        #[cfg(feature = "native-host")]
        if !ui.is_enabled() {
            input_state.problems.detach();
        }
        #[cfg(feature = "native-host")]
        let external_scroll = input_state
            .scroll
            .begin(previous.id, vec2(current.scroll.x, current.scroll.y));
        let mut output = InputOutput::default();
        #[cfg(feature = "native-host")]
        let documentation_input = if let Some(provider) = documentation.as_deref_mut() {
            input_state.documentation.input(
                ui,
                store,
                view,
                id,
                provider,
                documentation_commands,
            )?
        } else {
            crate::editor_documentation::Input::default()
        };
        #[cfg(feature = "native-host")]
        let initial_location = locations
            .as_deref_mut()
            .and_then(|provider| provider.current(store, view))
            .filter(|_| presentation.options.location_colors.is_some());
        #[cfg(feature = "native-host")]
        let location_input = input_state.locations.input(
            ui,
            store,
            view,
            id,
            rect,
            presentation.options.word_wrap,
            initial_location.as_ref(),
            &mut locations,
        )?;
        #[cfg(feature = "native-host")]
        let link_pointer = input_state.definition_link.input(
            ui,
            store,
            view,
            rect,
            presentation.options.word_wrap,
            initial_location
                .as_ref()
                .map(|widget| widget.token.as_str()),
            &mut locations,
        )?;
        #[cfg(feature = "native-host")]
        let initial_problem = problems
            .as_deref_mut()
            .and_then(|provider| provider.current(store, view))
            .filter(|_| {
                presentation.options.problem_colors.is_some() && initial_location.is_none()
            });
        #[cfg(feature = "native-host")]
        let problem_ids = input_state.problems.focus_ids();
        #[cfg(feature = "native-host")]
        let problem_routes = problem_ids
            .iter()
            .map(|id| ui.ctx().keyboard_input_route(*id))
            .collect::<Vec<_>>();
        #[cfg(feature = "native-host")]
        let problem_start = problem_ids
            .iter()
            .position(|id| Some(*id) == ui.ctx().keyboard_focus_before_events());
        #[cfg(feature = "native-host")]
        let initial_scene = initial_problem
            .as_ref()
            .map(|widget| crate::editor_problems::Scene {
                document: previous.clone(),
                widget: widget.clone(),
                rect,
                scroll: vec2(current.scroll.x, current.scroll.y),
                line_height: appearance.line_height,
                word_wrap: presentation.options.word_wrap,
                folds: current.folds.clone(),
            });
        #[cfg(feature = "native-host")]
        let problem_valid = input_state.problems.prepare(initial_scene.as_ref());
        #[cfg(feature = "native-host")]
        let mut problem_released = !problem_valid && problem_start.is_some();
        let os = ui.ctx().os();
        let indent = self.indent_options(&previous);
        input_state.auto_closed.follow(store, view)?;
        let mut input_context = InputContext {
            is_mac: os.is_mac(),
            force_crlf: os == OperatingSystem::Windows,
            page_lines: ((rect.height() / appearance.line_height).floor() as isize
                - PAGE_OVERLAP_LINES)
                .max(1),
            indent,
            language,
            clipboard: ui
                .ctx()
                .data_mut(|data| data.get_temp(Id::new(CLIPBOARD_MEMORY))),
            projection: Projection {
                painter: ui.painter().clone(),
                appearance,
                width: (rect.width() - minimap_width).max(0.0),
                wrap_tab_size: presentation.options.word_wrap.then_some(indent.tab_size),
                has_folding: presentation.options.folding,
                cached,
                rendered_viewport: input_state
                    .rendered_viewport
                    .filter(|rendered| rendered.scroll_top == current.scroll.y),
                stable_scroll_top: None,
                #[cfg(feature = "native-host")]
                problem_zone: initial_location
                    .as_ref()
                    .map(|widget| {
                        (
                            widget.position,
                            input_state
                                .locations
                                .height(appearance.line_height, rect.height()),
                        )
                    })
                    .or_else(|| {
                        initial_problem.as_ref().map(|widget| {
                            (
                                widget.position,
                                widget.height(appearance.line_height, rect.height()),
                            )
                        })
                    }),
            },
        };
        #[cfg(feature = "native-host")]
        let sticky_ids = input_state.sticky.focus_ids();
        #[cfg(feature = "native-host")]
        let sticky_routes = sticky_ids
            .iter()
            .map(|id| ui.ctx().keyboard_input_route(*id))
            .collect::<Vec<_>>();
        #[cfg(feature = "native-host")]
        let sticky_start = sticky_ids
            .iter()
            .position(|id| Some(*id) == ui.ctx().keyboard_focus_before_events());
        #[cfg(feature = "native-host")]
        let sticky_valid = input_state.sticky.prepare(
            &previous,
            Rect::from_min_max(
                rect.min,
                pos2(
                    (rect.right() - minimap_width - VERTICAL_SCROLLBAR_SIZE).max(rect.left()),
                    rect.bottom(),
                ),
            ),
            vec2(current.scroll.x, current.scroll.y),
            appearance,
            &presentation.options,
            indent.tab_size,
            &current.folds,
        );
        #[cfg(feature = "native-host")]
        let mut sticky_released = !sticky_valid && sticky_start.is_some();
        #[cfg(feature = "native-host")]
        let mut sticky_navigation = None;
        #[cfg(feature = "native-host")]
        let mut sticky_detached = false;
        let (ownership, (routed_focus, lost_after_events)) = route(&response).unwrap_or_default();
        let focused = routed_focus.unwrap_or_else(|| response.has_focus());
        let has_owned_input = ownership.iter().any(|(owned, _)| *owned == Some(true));
        #[cfg(feature = "native-host")]
        let mut last_editor_event = None;
        #[cfg(feature = "native-host")]
        let sticky_owned = sticky_start.is_some()
            || ui.memory(|memory| sticky_ids.iter().any(|id| memory.has_focus(*id)))
            || sticky_routes
                .iter()
                .flatten()
                .any(|route| route.0.iter().any(|(owned, _)| *owned == Some(true)));
        #[cfg(feature = "native-host")]
        if !sticky_valid && sticky_owned {
            sticky_released = true;
        }
        #[cfg(not(feature = "native-host"))]
        let sticky_owned = false;
        #[cfg(feature = "native-host")]
        let event_count = ui.input(|input| input.raw.events.len());
        #[cfg(feature = "native-host")]
        let problem_owned = problem_start.is_some()
            || problem_routes
                .iter()
                .flatten()
                .any(|route| route.0.iter().any(|(owned, _)| *owned == Some(true)))
            || (0..event_count).any(|index| {
                ui.ctx()
                    .pointer_focus_preserving_trigger_at(index)
                    .is_some_and(|id| problem_ids.contains(&id))
            });
        #[cfg(not(feature = "native-host"))]
        let problem_owned = false;
        #[cfg(feature = "native-host")]
        let location_owned = !location_input.consumed.is_empty();
        #[cfg(feature = "native-host")]
        let documentation_owned = !documentation_input.consumed.is_empty();
        #[cfg(not(feature = "native-host"))]
        let documentation_owned = false;
        #[cfg(not(feature = "native-host"))]
        let location_owned = false;
        if (response.has_focus()
            || has_owned_input
            || sticky_owned
            || problem_owned
            || location_owned
            || documentation_owned)
            && ui.is_enabled()
        {
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
            #[cfg(feature = "native-host")]
            let raw_events = ui.input(|input| input.raw.events.clone());
            #[cfg(feature = "native-host")]
            let mut raw_next = 0;
            let events = ui.input_mut(|input| std::mem::take(&mut input.events));
            let mut remaining = Vec::new();
            for (_index, event) in events.into_iter().enumerate() {
                #[cfg(feature = "native-host")]
                let raw_index = egui::Context::raw_event_index(&raw_events, &event, &mut raw_next);
                #[cfg(feature = "native-host")]
                if location_input.consumed.contains(&raw_index)
                    || documentation_input.consumed.contains(&raw_index)
                {
                    continue;
                }
                #[cfg(feature = "native-host")]
                let route_index = raw_index;
                #[cfg(not(feature = "native-host"))]
                let route_index = _index;
                let (owned, lost) = ownership.get(route_index).copied().unwrap_or_default();
                if lost {
                    input_state.ime_revision = None;
                    store.set_composition(view, None)?;
                }
                #[cfg(feature = "native-host")]
                {
                    let request = ui.ctx().keyboard_focus_request_at(raw_index);
                    let ours = request.is_some_and(|node| {
                        node == id.accesskit_id()
                            || sticky_ids.iter().any(|id| node == id.accesskit_id())
                            || problem_ids.iter().any(|id| node == id.accesskit_id())
                    }) || ui
                        .ctx()
                        .pointer_focus_preserving_trigger_at(raw_index)
                        .is_some_and(|id| problem_ids.contains(&id));
                    if !ours
                        && (request.is_some()
                            || matches!(&event, Event::PointerButton { pos, pressed: true, .. } if !rect.contains(*pos))
                            || matches!(&event, Event::WindowFocused(false)))
                    {
                        problem_released = false;
                        input_state.problems.detach();
                    }
                    if !sticky_ids.is_empty() && ours {
                        sticky_detached = false;
                    } else if !sticky_ids.is_empty()
                        && (request.is_some()
                            || matches!(&event, Event::PointerButton { pos, pressed: true, .. } if !rect.contains(*pos))
                            || matches!(&event, Event::WindowFocused(false)))
                    {
                        sticky_released = false;
                        sticky_navigation = None;
                        sticky_detached = true;
                        ui.memory_mut(|memory| {
                            memory.surrender_focus(id);
                            for id in &sticky_ids {
                                memory.surrender_focus(*id);
                            }
                        });
                    }
                    if let Some(action) = input_state.sticky.pointer(ui.ctx(), &event, raw_index) {
                        last_editor_event = Some(raw_index);
                        sticky_released |= apply_sticky_action(
                            ui,
                            store,
                            view,
                            &mut input_state,
                            action,
                            indent.tab_size,
                            language.map(|language| language.rules),
                        )?;
                        continue;
                    }
                    if matches!(event, Event::PointerButton { .. }) {
                        let document = store.documents().snapshot(
                            store
                                .views()
                                .get(view)
                                .ok_or(EditorError::NotFound)?
                                .document,
                        )?;
                        if !input_state.problems.describes(&document)
                            || problems
                                .as_deref_mut()
                                .and_then(|provider| provider.current(store, view))
                                .is_none()
                        {
                            input_state.problems.clear();
                        }
                    }
                    if let Some(command) = input_state.problems.pointer(ui.ctx(), &event, raw_index)
                        && let Some(provider) = problems.as_deref_mut()
                    {
                        provider.execute(store, view, command)?;
                        input_state.problems.clear();
                        last_editor_event = Some(raw_index);
                        problem_released = true;
                        response.request_focus();
                        continue;
                    }
                }
                #[cfg(feature = "native-host")]
                let problem_owner = problem_routes
                    .iter()
                    .position(|route| {
                        route.as_ref().is_some_and(|route| {
                            route
                                .0
                                .get(route_index)
                                .is_some_and(|(owned, _)| *owned == Some(true))
                        })
                    })
                    .or_else(|| {
                        (owned.is_none() && problem_routes.iter().all(Option::is_none))
                            .then_some(problem_start)
                            .flatten()
                    });
                #[cfg(not(feature = "native-host"))]
                let problem_owner: Option<usize> = None;
                #[cfg(feature = "native-host")]
                let sticky_owner = sticky_routes
                    .iter()
                    .position(|route| {
                        route.as_ref().is_some_and(|route| {
                            route
                                .0
                                .get(route_index)
                                .is_some_and(|(owned, _)| *owned == Some(true))
                        })
                    })
                    .or_else(|| {
                        (owned.is_none() && sticky_routes.iter().all(Option::is_none))
                            .then_some(sticky_start)
                            .flatten()
                    });
                #[cfg(not(feature = "native-host"))]
                let sticky_owner: Option<usize> = None;
                #[cfg(feature = "native-host")]
                let fallback_focus = problem_released || !sticky_detached && response.has_focus();
                #[cfg(not(feature = "native-host"))]
                let fallback_focus = response.has_focus();
                #[cfg(feature = "native-host")]
                let owns_body = location_input
                    .release_at
                    .is_some_and(|index| raw_index > index)
                    || documentation_input
                        .release_at
                        .is_some_and(|index| raw_index > index)
                    || documentation_input.release_from_start
                    || problem_released
                    || owned.unwrap_or(fallback_focus);
                #[cfg(not(feature = "native-host"))]
                let owns_body = owned.unwrap_or(fallback_focus);
                if !owns_body && sticky_owner.is_none() && problem_owner.is_none() {
                    remaining.push(event);
                    continue;
                }
                let composing = store
                    .views()
                    .get(view)
                    .is_some_and(|view| view.composition.is_some());
                #[cfg(feature = "native-host")]
                if let Some(index) = problem_owner.filter(|_| !problem_released) {
                    if editor_keymap(ui, store, view, &event, composing)
                        || keymap(ui, &event, composing)
                    {
                        continue;
                    }
                    let command = (!composing)
                        .then(|| input_state.problems.key(&event, problem_ids[index]))
                        .flatten();
                    if let Some(command) = command
                        && let Some(provider) = problems.as_deref_mut()
                    {
                        provider.execute(store, view, command)?;
                        input_state.problems.clear();
                        last_editor_event = Some(raw_index);
                        problem_released = true;
                        response.request_focus();
                        continue;
                    }
                    if execute_problem_shortcut(&mut problems, store, view, &event, composing)? {
                        last_editor_event = Some(raw_index);
                        problem_released = true;
                        response.request_focus();
                        continue;
                    }
                    if matches!(
                        event,
                        Event::Text(_)
                            | Event::Paste(_)
                            | Event::Ime(_)
                            | Event::Key {
                                key: Key::Space | Key::Enter,
                                ..
                            }
                    ) {
                        continue;
                    }
                    remaining.push(event);
                    continue;
                }
                #[cfg(feature = "native-host")]
                if let Some(index) = sticky_owner.filter(|_| !sticky_released) {
                    let index = sticky_navigation.unwrap_or(index);
                    if let Some(action) = input_state.sticky.key(&event, index) {
                        last_editor_event = Some(raw_index);
                        if let crate::editor_sticky_scroll::StickyAction::Focus(index) = action {
                            sticky_navigation = Some(index);
                        }
                        sticky_released |= apply_sticky_action(
                            ui,
                            store,
                            view,
                            &mut input_state,
                            action,
                            indent.tab_size,
                            language.map(|language| language.rules),
                        )?;
                        continue;
                    }
                    if editor_keymap(ui, store, view, &event, composing)
                        || keymap(ui, &event, composing)
                    {
                        continue;
                    }
                    if execute_problem_shortcut(&mut problems, store, view, &event, composing)? {
                        last_editor_event = Some(raw_index);
                        sticky_released = true;
                        response.request_focus();
                        continue;
                    }
                    if matches!(event, Event::Text(_) | Event::Paste(_) | Event::Ime(_)) {
                        continue;
                    }
                    remaining.push(event);
                    continue;
                }
                if editor_keymap(ui, store, view, &event, composing)
                    || keymap(ui, &event, composing)
                {
                    #[cfg(feature = "native-host")]
                    {
                        last_editor_event = Some(raw_index);
                    }
                    continue;
                }
                #[cfg(feature = "native-host")]
                if execute_problem_shortcut(&mut problems, store, view, &event, composing)? {
                    last_editor_event = Some(raw_index);
                    response.request_focus();
                    continue;
                }
                if let Some(language) = language {
                    language.syntax.follow_edits(store);
                }
                #[cfg(feature = "native-host")]
                let documentation_before = store.views().get(view).and_then(|view| {
                    Some((
                        store.documents().snapshot(view.document).ok()?.revision,
                        view.selection.clone(),
                    ))
                });
                match self.input(
                    store,
                    view,
                    &event,
                    &mut input_state,
                    &mut output,
                    &mut input_context,
                ) {
                    Ok(true) => {
                        #[cfg(feature = "native-host")]
                        {
                            last_editor_event = Some(raw_index);
                        }
                        input_state.auto_closed.follow(store, view)?;
                        #[cfg(feature = "native-host")]
                        if let Some(provider) = documentation.as_deref_mut() {
                            let changed = store.views().get(view).is_some_and(|view| {
                                store
                                    .documents()
                                    .snapshot(view.document)
                                    .is_ok_and(|document| {
                                        documentation_before.as_ref().is_none_or(
                                            |(revision, selection)| {
                                                *revision != document.revision
                                                    || *selection != view.selection
                                            },
                                        )
                                    })
                            });
                            input_state
                                .documentation
                                .after_event(ui, store, view, provider, &event, changed);
                        }
                    }
                    Ok(false) => remaining.push(event),
                    Err(error) => output.errors.push(error),
                }
            }
            ui.input_mut(|input| input.events = remaining);
        }
        #[cfg(feature = "native-host")]
        let focused = if sticky_released || problem_released {
            response.request_focus();
            true
        } else {
            if !sticky_detached && let Some(index) = sticky_navigation {
                input_state.sticky.focus(ui, index);
            }
            focused
        };
        if lost_after_events || !focused || !ui.is_enabled() {
            input_state.ime_revision = None;
            store.set_composition(view, None)?;
        }
        if let Some(copied) = output.copied {
            ui.ctx().copy_text(copied.text.clone());
            ui.ctx()
                .data_mut(|data| data.insert_temp(Id::new(CLIPBOARD_MEMORY), copied));
        }
        let has_folding = presentation.options.folding;
        let (fold_regions, fold_reveal) = maintain_folds(
            store,
            view,
            &mut input_state,
            has_folding.then_some(indent.tab_size),
            fold_commands,
            language.map(|language| language.rules),
        )?;
        let document = store.documents().snapshot(current.document)?;
        let tokens = tokens(store).filter(|tokens| tokens.describes(&document));
        #[cfg(feature = "native-host")]
        let sticky_model = if presentation.options.sticky_scroll
            && presentation.options.sticky_colors.is_some()
            && crate::editor_sticky_scroll::is_available(&document)
        {
            Some(
                input_state.sticky.model(
                    &document,
                    indent.tab_size,
                    language.map(|language| language.rules),
                    input_state
                        .syntax_folds
                        .as_ref()
                        .filter(|syntax| syntax.describes(&document))
                        .map(|syntax| syntax.regions())
                        .or_else(|| has_folding.then_some(&fold_regions)),
                    presentation.options.sticky_model.as_ref(),
                ),
            )
        } else {
            input_state.sticky.clear();
            None
        };
        #[cfg(feature = "native-host")]
        let bracket_model = presentation
            .options
            .bracket_colors
            .map(|_| {
                store.bracket_model(
                    document.id,
                    language.map(|language| language.rules),
                    indent.tab_size,
                    tokens.map(
                        |tokens| taide_native_editor::bracket_model::BracketTokenData {
                            lines: tokens.lines,
                            styles: tokens.styles,
                        },
                    ),
                )
            })
            .transpose()?;
        let tracked = tracked_layers(store, &document, decorations)?;
        #[cfg(feature = "native-host")]
        let diagnostic_markers = presentation
            .options
            .diagnostics
            .as_ref()
            .and_then(|markers| {
                markers.tracked(
                    &document,
                    store.changes_since(document.id, markers.revision()).ok()?,
                )
            });
        let mut state = store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .clone();
        let mut projection = input_context.projection;
        #[cfg(feature = "native-host")]
        let location_widget = locations
            .as_deref_mut()
            .and_then(|provider| provider.current(store, view))
            .filter(|_| presentation.options.location_colors.is_some());
        #[cfg(feature = "native-host")]
        let problem_widget = problems
            .as_deref_mut()
            .and_then(|provider| provider.current(store, view))
            .filter(|widget| {
                widget.coordinate.problem.document == document.id
                    && presentation.options.problem_colors.is_some()
                    && location_widget.is_none()
            });
        #[cfg(feature = "native-host")]
        {
            projection.problem_zone = location_widget
                .as_ref()
                .map(|widget| {
                    (
                        widget.position,
                        input_state
                            .locations
                            .height(appearance.line_height, rect.height()),
                    )
                })
                .or_else(|| {
                    problem_widget.as_ref().map(|widget| {
                        (
                            widget.position,
                            widget.height(appearance.line_height, rect.height()),
                        )
                    })
                });
        }
        #[cfg(feature = "native-host")]
        let zone_position = projection.problem_zone;
        let (display, stable_scroll_top) =
            projection.map_with_stable_scroll_top(&document, &state.folds);
        let wrap_column = display.wrap_settings().map(|settings| settings.wrap_column);
        let head_row = |state: &ViewState, selection: usize| {
            display.row_of_head(
                &document,
                state.selection.selections[selection].head,
                state.head_at_row_end(selection, document.revision),
            )
        };
        let painter = ui.painter().with_clip_rect(rect);
        let gutter = Gutter::measure(
            &painter,
            document.rope.len_lines(),
            appearance,
            presentation.options.folding,
        );
        let text_rect = Rect::from_min_max(
            pos2(rect.left() + gutter.width(), rect.top()),
            pos2(
                editor_right.max(rect.left() + gutter.width()),
                rect.bottom(),
            ),
        );
        let layout = VerticalLayout::new(appearance.line_height, display.row_count());
        #[cfg(feature = "native-host")]
        let layout = zone_position.map_or_else(
            || layout.clone(),
            |(position, height)| {
                layout
                    .clone()
                    .with_zone(display.row_of_byte(&document, position), height)
            },
        );
        let content_height = layout.content_height();
        #[cfg(feature = "native-host")]
        let base_content_height = crate::editor_scroll::content_height(
            content_height,
            rect.height(),
            appearance.line_height,
            presentation.options.scroll_beyond_last_line,
        );
        #[cfg(feature = "native-host")]
        let content_height = {
            let width = if wrap_column.is_none() {
                input_state.widest_line
                    + painter
                        .layout_no_wrap(
                            " ".repeat(SCROLL_BEYOND_LAST_COLUMN),
                            appearance.font.clone(),
                            appearance.foreground,
                        )
                        .size()
                        .x
                    + VERTICAL_SCROLLBAR_SIZE
            } else {
                input_state.widest_line
            };
            base_content_height
                + if presentation.options.colors.is_some()
                    && !presentation.options.scroll_beyond_last_line
                    && width > text_rect.width()
                {
                    HORIZONTAL_SCROLLBAR_SIZE
                } else {
                    0.0
                }
        };
        let scroll_max = (content_height - rect.height()).max(0.0);
        state.scroll.y = stable_scroll_top.unwrap_or(state.scroll.y).min(scroll_max);
        #[cfg(feature = "native-host")]
        let sticky_reveal = input_state.sticky.take_fold_reveal();
        #[cfg(feature = "native-host")]
        let did_fold_reveal = fold_reveal.is_some() || sticky_reveal.is_some();
        if let Some(reveal) = fold_reveal {
            let primary = state.selection.selections[state.selection.primary];
            let row = display.row_of_byte(
                &document,
                match reveal {
                    FoldReveal::Head => primary.head,
                    FoldReveal::SelectionStart => primary.anchor.min(primary.head),
                    FoldReveal::LineStart(line) => document
                        .rope
                        .line_to_byte(line.min(document.rope.len_lines() - 1)),
                },
            );
            if layout.row_top(row) < state.scroll.y
                || layout.row_bottom(row) > state.scroll.y + rect.height()
            {
                state.scroll.y = (layout.row_center(row) - rect.height() / CENTER_DIVISOR)
                    .clamp(0.0, scroll_max);
            }
        }
        let requested_reveal = store.take_selection_reveal(view)?;
        let moved = document.revision != previous.revision
            || state.selection != current.selection
            || requested_reveal.is_some();
        let head = state.selection.selections[state.selection.primary].head;
        let caret_row = head_row(&state, state.selection.primary);
        let reveal_byte = requested_reveal.as_ref().map_or(head, |reveal| {
            reveal.bytes.end.min(document.rope.len_bytes())
        });
        let reveal_row = requested_reveal
            .as_ref()
            .map_or(caret_row, |_| display.row_of_byte(&document, reveal_byte));
        if moved {
            let top = layout.row_top(reveal_row);
            if requested_reveal
                .as_ref()
                .is_some_and(|reveal| reveal.near_top_if_outside)
                && (top < state.scroll.y
                    || layout.row_bottom(reveal_row) > state.scroll.y + rect.height())
            {
                const NEAR_TOP_MARGIN_LINES: f32 = 5.0;
                const NEAR_TOP_VIEWPORT_RATIO: f32 = 0.2;
                let gap = (NEAR_TOP_MARGIN_LINES * appearance.line_height)
                    .max(rect.height() * NEAR_TOP_VIEWPORT_RATIO);
                state.scroll.y = (top - gap)
                    .max(layout.row_bottom(reveal_row) - rect.height())
                    .clamp(0.0, scroll_max);
            }
            if requested_reveal
                .as_ref()
                .is_some_and(|reveal| reveal.center_if_outside)
                && (top < state.scroll.y
                    || layout.row_bottom(reveal_row) > state.scroll.y + rect.height())
            {
                state.scroll.y = (layout.row_center(reveal_row) - rect.height() / CENTER_DIVISOR)
                    .clamp(0.0, scroll_max);
            }
            if top < state.scroll.y {
                state.scroll.y = top;
            }
            let bottom = layout.row_bottom(reveal_row);
            if bottom > state.scroll.y + rect.height() {
                state.scroll.y = (bottom - rect.height()).max(0.0);
            }
        }
        #[cfg(feature = "native-host")]
        if let Some((line, index)) = sticky_reveal {
            let row = display.row_of_byte(
                &document,
                document
                    .rope
                    .line_to_byte(line.min(document.rope.len_lines() - 1)),
            );
            state.scroll.y = (layout.row_top(row) - index as f32 * appearance.line_height + 1.0)
                .clamp(0.0, scroll_max);
        }
        #[cfg(feature = "native-host")]
        let minimap_input = minimap_dimensions
            .zip(minimap_rect)
            .map(|(dimensions, map_rect)| {
                let map_layout = input_state.minimap.layout(
                    &document,
                    dimensions,
                    crate::editor_minimap_layout::MinimapViewport::new(
                        display.row_count(),
                        f64::from(rect.height()),
                        f64::from(appearance.line_height),
                        f64::from(state.scroll.y),
                        f64::from(content_height),
                        presentation.options.scroll_beyond_last_line,
                    ),
                );
                input_state.minimap.interact(
                    ui,
                    id,
                    map_rect,
                    &map_layout,
                    dimensions,
                    rect.height(),
                    appearance.line_height,
                    display.row_count(),
                )
            });
        #[cfg(feature = "native-host")]
        let location_hovered = location_widget.is_some()
            && layout.zone().is_some_and(|zone| {
                ui.input(|input| input.pointer.hover_pos())
                    .is_some_and(|point| {
                        Rect::from_min_max(
                            pos2(rect.left(), rect.top() + zone.start - state.scroll.y),
                            pos2(editor_right, rect.top() + zone.end - state.scroll.y),
                        )
                        .contains(point)
                    })
            });
        #[cfg(feature = "native-host")]
        let (wheel, precise_wheel) = if ui.is_enabled()
            && !location_hovered
            && !input_state.documentation.contains_pointer(ui)
            && (response.hovered()
                || minimap_input
                    .as_ref()
                    .is_some_and(|input| input.response.hovered())
                || input_state.sticky.focus_ids().iter().any(|id| {
                    ui.ctx()
                        .read_response(*id)
                        .is_some_and(|response| response.hovered())
                })) {
            input_state.scroll.wheel(ui)
        } else {
            (Vec2::ZERO, false)
        };
        #[cfg(not(feature = "native-host"))]
        let wheel = if response.hovered() && ui.is_enabled() {
            ui.input_mut(|input| std::mem::take(&mut input.smooth_scroll_delta))
        } else {
            Vec2::ZERO
        };
        #[cfg(feature = "native-host")]
        let requested_scroll_y = minimap_input
            .as_ref()
            .filter(|input| {
                !((moved || did_fold_reveal || sticky_reveal.is_some())
                    && last_editor_event
                        .zip(input.event_index)
                        .is_some_and(|(editor, map)| editor > map))
            })
            .and_then(|input| input.requested_scroll.map(|(scroll, _)| scroll))
            .or_else(|| {
                if wheel.y != 0.0 {
                    return Some(input_state.scroll.y.future(state.scroll.y) - wheel.y);
                }
                (moved || did_fold_reveal).then_some(state.scroll.y)
            });
        #[cfg(not(feature = "native-host"))]
        {
            state.scroll.y = (state.scroll.y - wheel.y).clamp(0.0, scroll_max);
        }
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
        #[cfg(feature = "native-host")]
        let mut vertical = vertical;
        if let Some(scrollbar) = &vertical {
            state.scroll.y = scrollbar.drag(&mut input_state, &press, state.scroll.y);
        }
        #[cfg(feature = "native-host")]
        {
            let dragging = input_state.scrollbar_drag.is_some();
            let reveal_from = input_state.scroll.reveal_from;
            let target = if dragging || reveal_from.is_some() {
                Some(state.scroll.y)
            } else {
                requested_scroll_y.or_else(|| {
                    (external_scroll || stable_scroll_top.is_some()).then_some(state.scroll.y)
                })
            };
            let from = reveal_from.map_or(current.scroll.y, |scroll| scroll.y);
            state.scroll.y = input_state.scroll.y.sample(
                ui,
                from,
                target,
                scroll_max,
                rect.height(),
                !presentation.options.smooth_scrolling
                    || precise_wheel
                    || dragging
                    || minimap_input.as_ref().is_some_and(|input| {
                        input
                            .requested_scroll
                            .is_some_and(|(_, immediate)| immediate)
                    })
                    || (external_scroll && reveal_from.is_none())
                    || stable_scroll_top.is_some(),
            );
        }
        let visible = layout.visible_rows(state.scroll.y, rect.height(), ROW_OVERSCAN);
        let visible_lines = if visible.is_empty() {
            0..0
        } else {
            display.segment(&document, visible.start).line
                ..display.segment(&document, visible.end - 1).line + 1
        };
        let decorations = frame_decorations(&document, &tracked, visible_lines);
        let row_layout = RowLayout {
            painter: &painter,
            document: &document,
            display,
            appearance,
            half_leading: half_leading(&painter, appearance),
            tab_size: indent.tab_size,
            tokens,
            bold_family: registered_family(ui, presentation.options.bold_family.as_ref()),
            decorations: &decorations,
            #[cfg(feature = "native-host")]
            brackets: bracket_model
                .as_deref()
                .zip(presentation.options.bracket_colors.as_ref())
                .filter(|_| presentation.options.bracket_pair_colorization),
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
        #[cfg(feature = "native-host")]
        let wheel_target_x =
            (wheel.x != 0.0).then(|| input_state.scroll.x.future(state.scroll.x) - wheel.x);
        #[cfg(not(feature = "native-host"))]
        {
            state.scroll.x -= wheel.x;
        }
        if moved && let Some(row) = rows.iter().find(|row| row.index == reveal_row) {
            state.scroll.x = scroll_x_revealing(
                state.scroll.x,
                row.caret(reveal_byte.min(row.segment.bytes.end)),
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
                    text_rect
                        .right()
                        .min(rect.right() - VERTICAL_SCROLLBAR_SIZE)
                        .max(text_rect.left()),
                    rect.bottom(),
                ),
            ),
            text_width,
            content_width,
        );
        if let Some(scrollbar) = &horizontal {
            state.scroll.x = scrollbar.drag(&mut input_state, &press, state.scroll.x);
        }
        #[cfg(feature = "native-host")]
        let content_height = {
            let adjusted = base_content_height
                + if presentation.options.colors.is_some()
                    && !presentation.options.scroll_beyond_last_line
                    && horizontal.is_some()
                {
                    HORIZONTAL_SCROLLBAR_SIZE
                } else {
                    0.0
                };
            if adjusted != content_height {
                let previous_y = state.scroll.y;
                vertical = Scrollbar::new(
                    ScrollAxis::Vertical,
                    Rect::from_min_max(
                        pos2(
                            (rect.right() - VERTICAL_SCROLLBAR_SIZE).max(rect.left()),
                            rect.top(),
                        ),
                        rect.max,
                    ),
                    rect.height(),
                    adjusted,
                );
                if let Some(scrollbar) = &vertical {
                    state.scroll.y = scrollbar.drag(&mut input_state, &press, state.scroll.y);
                }
                let dragging = input_state.scrollbar_drag.is_some();
                let reveal_from = input_state.scroll.reveal_from;
                let target = if dragging || reveal_from.is_some() {
                    Some(state.scroll.y)
                } else {
                    requested_scroll_y
                };
                state.scroll.y = input_state.scroll.y.sample(
                    ui,
                    reveal_from.map_or(current.scroll.y, |scroll| scroll.y),
                    target,
                    (adjusted - rect.height()).max(0.0),
                    rect.height(),
                    !presentation.options.smooth_scrolling
                        || precise_wheel
                        || dragging
                        || (external_scroll && reveal_from.is_none())
                        || stable_scroll_top.is_some(),
                );
                for row in &mut rows {
                    row.origin.y += previous_y - state.scroll.y;
                }
            }
            adjusted
        };
        #[cfg(feature = "native-host")]
        {
            let dragging = input_state.scrollbar_drag.is_some();
            let reveal_from = input_state.scroll.reveal_from.take();
            let target = if dragging || moved || reveal_from.is_some() {
                Some(state.scroll.x)
            } else {
                wheel_target_x.or_else(|| {
                    (external_scroll || stable_scroll_top.is_some()).then_some(state.scroll.x)
                })
            };
            let from = reveal_from.map_or(current.scroll.x, |scroll| scroll.x);
            state.scroll.x = input_state.scroll.x.sample(
                ui,
                from,
                target,
                scroll_x_max,
                text_width,
                !presentation.options.smooth_scrolling
                    || precise_wheel
                    || dragging
                    || (external_scroll && reveal_from.is_none())
                    || stable_scroll_top.is_some(),
            );
            input_state
                .scroll
                .rendered(vec2(state.scroll.x, state.scroll.y));
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
        #[cfg(feature = "native-host")]
        let sticky_layout = sticky_model
            .as_ref()
            .map_or_else(Default::default, |model| {
                let visible = layout.visible_rows(state.scroll.y, rect.height(), 0);
                let lines = if visible.is_empty() {
                    0..0
                } else {
                    display.segment(&document, visible.start).line
                        ..display.segment(&document, visible.end - 1).line + 1
                };
                model.layout(
                    &taide_native_editor::sticky_model::StickyViewport {
                        visible_lines: lines,
                        scroll_top: state.scroll.y,
                        height: rect.height(),
                        line_height: appearance.line_height,
                    },
                    &hidden_lines(&document.rope, &state.folds),
                    |line| {
                        layout.row_top(
                            display.row_of_byte(&document, document.rope.line_to_byte(line)),
                        )
                    },
                    |line| layout.row_top(display.rows_of_line(line).end),
                )
            });
        #[cfg(feature = "native-host")]
        let sticky_rect = Rect::from_min_max(
            rect.min,
            pos2(
                (rect.right() - minimap_width - VERTICAL_SCROLLBAR_SIZE).max(rect.left()),
                rect.top() + sticky_layout.height(appearance.line_height),
            ),
        );
        #[cfg(feature = "native-host")]
        let is_sticky_press = press
            .origin
            .is_some_and(|origin| sticky_rect.contains(origin) && sticky_rect.height() > 0.0);
        #[cfg(not(feature = "native-host"))]
        let is_sticky_press = false;
        let fold_placeholder_width = painter
            .layout_no_wrap(
                FOLD_PLACEHOLDER.into(),
                appearance.font.clone(),
                appearance.foreground,
            )
            .size()
            .x
            + FOLD_PLACEHOLDER_MARGIN_EM * appearance.font.size;
        let fold_target = |position: Pos2| {
            if !has_folding || !rect.contains(position) {
                return None;
            }
            #[cfg(feature = "native-host")]
            if sticky_rect.height() > 0.0 && sticky_rect.contains(position) {
                return None;
            }
            let row = rows.iter().find(|row| {
                (row.origin.y..row.origin.y + appearance.line_height).contains(&position.y)
            })?;
            let is_on_control = gutter.fold_click_zone(rect.left()).contains(position.x);
            let is_on_folded_end = row.segment.ends_folded
                && text_rect.contains(position)
                && row.byte_at(position) == row.segment.bytes.end
                && position.x < row.fold_placeholder_left(appearance) + fold_placeholder_width;
            (is_on_control || is_on_folded_end).then_some(FoldPress {
                line: row.segment.line,
                is_on_control,
            })
        };
        let (is_fold_button_pressed, is_fold_button_released, is_middle_released) =
            ui.input(|input| {
                let pointer = &input.pointer;
                (
                    pointer.button_pressed(PointerButton::Primary)
                        || pointer.button_pressed(PointerButton::Middle),
                    pointer.button_released(PointerButton::Primary)
                        || pointer.button_released(PointerButton::Middle),
                    pointer.button_released(PointerButton::Middle),
                )
            });
        #[cfg(feature = "native-host")]
        let problem_rect = layout.zone().map(|zone| {
            Rect::from_min_max(
                pos2(rect.left(), rect.top() + zone.start - state.scroll.y),
                pos2(editor_right, rect.top() + zone.end - state.scroll.y),
            )
        });
        #[cfg(feature = "native-host")]
        let is_problem_press = link_pointer
            || problem_rect
                .is_some_and(|rect| press.origin.is_some_and(|point| rect.contains(point)));
        #[cfg(not(feature = "native-host"))]
        let is_problem_press = false;
        if is_fold_button_pressed {
            input_state.fold_press = press
                .origin
                .filter(|_| ui.is_enabled() && response.is_pointer_button_down_on())
                .and_then(&fold_target);
        }
        let fold_press = input_state.fold_press;
        if is_fold_button_released {
            input_state.fold_press = None;
        }
        if !scrolling
            && !is_sticky_press
            && !is_problem_press
            && !fold_press.is_some_and(|press| press.is_on_control)
            && (press.down || response.clicked() || response.drag_started() || response.dragged())
            && let Some(pointer) = response.interact_pointer_pos()
            && press.origin.is_none_or(|origin| {
                origin.x >= text_rect.left() || gutter.is_line_number(rect.left(), origin.x)
            })
            && !rows.is_empty()
        {
            let target = layout
                .row_at(pointer.y - rect.top() + state.scroll.y)
                .min(display.row_count() - 1);
            let row_at = |index| {
                row_layout.row(
                    index,
                    pos2(
                        text_rect.left() - state.scroll.x,
                        rect.top() + layout.row_top(index) - state.scroll.y,
                    ),
                )
            };
            let row = row_at(target);
            {
                let head = row.byte_at(pointer);
                let (pressed, modifiers, time) = ui.input(|input| {
                    (
                        input.pointer.button_pressed(PointerButton::Primary),
                        input.modifiers,
                        input.time,
                    )
                });
                let options = ui.ctx().options(|options| options.input_options);
                let input = PointerInput {
                    pressed,
                    down: press.down,
                    head,
                    row: target,
                    position: pointer,
                    modifiers,
                    time,
                    max_click_dist: options.max_click_dist,
                    max_double_click_delay: options.max_double_click_delay,
                    gutter: gutter.is_line_number(rect.left(), pointer.x),
                };
                if let Some(selection) = input_state.pointer_selection.update(
                    &document,
                    &state.selection,
                    input,
                    row_at,
                    |byte| display.row_of_byte(&document, byte),
                    language,
                ) {
                    state.selection = selection;
                    state.wrap_affinities = (display.row_of_byte(&document, head) != row.index)
                        .then(|| WrapAffinities {
                            revision: document.revision,
                            heads_at_row_end: state
                                .selection
                                .selections
                                .iter()
                                .map(|selection| selection.head == head)
                                .collect(),
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
        }
        if is_fold_button_released
            && let Some(pressed) = fold_press
            && ui
                .input(|input| input.pointer.interact_pos())
                .and_then(&fold_target)
                == Some(pressed)
        {
            let modifiers = ui.input(|input| input.modifiers);
            let toggle = if modifiers.alt {
                FoldToggle::Surrounding
            } else if modifiers.shift || is_middle_released {
                FoldToggle::Recursive
            } else {
                FoldToggle::Region
            };
            let click = FoldClick {
                line: pressed.line,
                is_on_control: pressed.is_on_control,
                toggle,
            };
            if click_fold(store, view, &fold_regions, click)? {
                input_state.clicked_fold_line = Some(pressed.line);
                ui.ctx().request_repaint();
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
            decorations: &decorations,
        };
        let primary = state.selection.selections[state.selection.primary];
        #[cfg(feature = "native-host")]
        let bracket_matches = bracket_model.as_deref().map_or_else(Vec::new, |model| {
            let focused = ui.is_enabled()
                && (response.has_focus()
                    || presentation.options.bracket_widget_focus
                    || ui.memory(|memory| {
                        input_state
                            .sticky
                            .focus_ids()
                            .iter()
                            .any(|id| memory.has_focus(*id))
                    }));
            crate::editor_brackets::matching_brackets(model, &state.selection, focused)
        });
        let head_rows: Vec<usize> = (0..state.selection.selections.len())
            .map(|selection| head_row(&state, selection))
            .collect();
        let primary_row = head_rows[state.selection.primary];
        let carets = Carets {
            selections: &state.selection,
            head_rows: &head_rows,
            primary_row,
            primary_line: display.segment(&document, primary_row).line,
            focused: focused && {
                #[cfg(feature = "native-host")]
                {
                    presentation.options.cursor_style == CursorStyle::LineThin
                        && presentation.options.cursor_blinking == CursorBlinking::Solid
                        && !presentation.options.smooth_caret
                        && presentation.options.colors.is_none()
                }
                #[cfg(not(feature = "native-host"))]
                {
                    true
                }
            },
        };
        layers.background();
        #[cfg(feature = "native-host")]
        if !presentation.options.rulers.is_empty() || bracket_model.is_some() {
            for row in &rows {
                layers.row_background(row, &carets);
            }
            crate::editor_display::rulers(
                &text_painter,
                text_rect,
                vec2(state.scroll.x, state.scroll.y),
                content_height.max(rect.height()),
                appearance,
                &presentation.options,
            );
            if let Some(model) = bracket_model.as_deref() {
                crate::editor_brackets::paint_guides(
                    &text_painter,
                    crate::editor_brackets::GuideFrame {
                        document: &document,
                        display,
                        rows: &rows,
                        model,
                        primary: primary.head,
                        tab_size: indent.tab_size,
                        appearance,
                        options: &presentation.options,
                    },
                );
                if let Some(colors) = presentation.options.bracket_colors.as_ref() {
                    crate::editor_brackets::paint_matching(
                        &text_painter,
                        &rows,
                        &bracket_matches,
                        colors,
                        appearance.line_height,
                    );
                }
            }
            for row in &rows {
                crate::editor_display::whitespace(
                    &text_painter,
                    row,
                    &document,
                    &state.selection,
                    appearance,
                    &presentation.options,
                );
                layers.row_content(row, &carets);
            }
        } else {
            for row in &rows {
                layers.row(row, &carets);
                crate::editor_display::whitespace(
                    &text_painter,
                    row,
                    &document,
                    &state.selection,
                    appearance,
                    &presentation.options,
                );
            }
        }
        #[cfg(not(feature = "native-host"))]
        for row in &rows {
            layers.row(row, &carets);
        }
        #[cfg(feature = "native-host")]
        if presentation.options.cursor_style != CursorStyle::LineThin
            || presentation.options.cursor_blinking != CursorBlinking::Solid
            || presentation.options.smooth_caret
            || presentation.options.colors.is_some()
        {
            input_state.caret.paint(
                ui,
                &text_painter,
                crate::editor_caret::CaretFrame {
                    document: &document,
                    selections: &state.selection,
                    head_rows: &head_rows,
                    rows: &rows,
                    appearance,
                    options: &presentation.options,
                    scroll: vec2(state.scroll.x, state.scroll.y),
                    origin: text_rect.min,
                    focused: focused && ui.is_enabled(),
                    composing: state.composition.is_some(),
                },
            )?;
        }
        let time = ui.input(|input| input.time);
        if has_folding && let Some(paint) = fold_controls.as_deref_mut() {
            let gutter_rect = Rect::from_min_max(rect.min, pos2(text_rect.left(), rect.bottom()));
            let is_gutter_hovered = ui.is_enabled()
                && response
                    .hover_pos()
                    .is_some_and(|pointer| gutter_rect.contains(pointer));
            let (opacity, is_fading) = input_state
                .fold_control_fade
                .sample(is_gutter_hovered, time);
            if is_fading {
                ui.ctx().request_repaint();
            }
            let model = FoldingModel::with_manual(
                &fold_regions,
                &document,
                &state.folds,
                &state.manual_folds,
            );
            let color = ui.visuals().weak_text_color();
            let clip = ui.clip_rect();
            ui.set_clip_rect(gutter_rect.intersect(clip));
            for row in rows.iter().filter(|row| !row.segment.is_continuation) {
                let (chevron_rotation, color) = match model.header(row.segment.line) {
                    Some(true) => (COLLAPSED_CHEVRON_ROTATION, color),
                    Some(false) if opacity > 0.0 => {
                        (EXPANDED_CHEVRON_ROTATION, color.gamma_multiply(opacity))
                    }
                    _ => continue,
                };
                paint(
                    ui,
                    FoldControl {
                        rect: gutter.fold_control_rect(rect.left(), row, appearance),
                        chevron_rotation,
                        color,
                    },
                );
            }
            ui.set_clip_rect(clip);
        }
        #[cfg(feature = "native-host")]
        if let Some(widget) = problem_widget.as_ref()
            && let Some(problem_rect) = problem_rect
            && let Some(colors) = presentation.options.problem_colors
        {
            let arrow_x = rows
                .iter()
                .find(|row| {
                    row.segment.bytes.start <= widget.position
                        && widget.position <= row.segment.bytes.end
                })
                .map_or(text_rect.left(), |row| {
                    row.caret_rect(widget.position).left()
                });
            let output = crate::editor_problems::paint(
                ui,
                crate::editor_problems::Frame {
                    id,
                    widget,
                    rect: problem_rect,
                    clip: Rect::from_min_max(
                        pos2(
                            rect.left(),
                            rect.top() + sticky_layout.height(appearance.line_height),
                        ),
                        pos2(editor_right, rect.bottom()),
                    ),
                    arrow_x,
                    line_height: appearance.line_height,
                    font: &appearance.font,
                    colors,
                },
            );
            input_state.problems.install(
                crate::editor_problems::Scene {
                    document: document.clone(),
                    widget: widget.clone(),
                    rect,
                    scroll: vec2(state.scroll.x, state.scroll.y),
                    line_height: appearance.line_height,
                    word_wrap: presentation.options.word_wrap,
                    folds: state.folds.clone(),
                },
                output,
            );
        } else {
            input_state.problems.clear();
        }
        #[cfg(feature = "native-host")]
        if let Some(widget) = location_widget.as_ref()
            && let Some(zone_rect) = problem_rect
            && let Some(colors) = presentation.options.location_colors
            && let Some(provider) = locations.as_deref_mut()
        {
            input_state.locations.paint(
                ui,
                store,
                view,
                provider,
                crate::editor_locations::Frame {
                    id,
                    widget,
                    rect: zone_rect,
                    owner_rect: rect,
                    anchor_x: rows
                        .iter()
                        .find(|row| {
                            row.segment.bytes.start <= widget.position
                                && widget.position <= row.segment.bytes.end
                        })
                        .map_or(text_rect.left(), |row| {
                            row.caret_rect(widget.position).left()
                        }),
                    word_wrap: presentation.options.word_wrap,
                    clip: Rect::from_min_max(
                        pos2(
                            rect.left(),
                            rect.top() + sticky_layout.height(appearance.line_height),
                        ),
                        pos2(editor_right, rect.bottom()),
                    ),
                    line_height: appearance.line_height,
                    font: &appearance.font,
                    colors,
                },
            )?;
        } else {
            input_state.locations.clear();
        }
        #[cfg(feature = "native-host")]
        if let Some(markers) = diagnostic_markers.as_ref()
            && let Some(colors) = presentation.options.diagnostic_colors
        {
            crate::editor_diagnostics::paint(
                ui,
                crate::editor_diagnostics::DiagnosticFrame {
                    id,
                    rows: &rows,
                    markers: markers.markers(),
                    rect: Rect::from_min_max(
                        pos2(
                            text_rect.left(),
                            text_rect.top() + sticky_layout.height(appearance.line_height),
                        ),
                        text_rect.max,
                    ),
                    height: appearance.line_height,
                    font: &appearance.font,
                    colors,
                    read_only: document.metadata.read_only,
                },
            );
        }
        #[cfg(feature = "native-host")]
        let toggle_sticky_scroll = if let Some(colors) = presentation.options.sticky_colors {
            crate::editor_sticky_scroll::paint(
                ui,
                &mut input_state.sticky,
                crate::editor_sticky_scroll::StickyFrame {
                    id,
                    layout: &sticky_layout,
                    rows: &row_layout,
                    rect: Rect::from_min_max(rect.min, pos2(sticky_rect.right(), rect.bottom())),
                    text_rect,
                    scroll: vec2(state.scroll.x, state.scroll.y),
                    gutter,
                    colors,
                    layers: &tracked,
                    regions: &fold_regions,
                    folds: &state.folds,
                    selection: &state.selection,
                    options: &presentation.options,
                },
                &mut fold_controls,
            )
        } else {
            false
        };
        #[cfg(feature = "native-host")]
        let scroll_marks = presentation
            .options
            .overview_colors
            .map_or_else(Vec::new, |colors| {
                crate::editor_overview::marks(
                    &tracked,
                    diagnostic_markers
                        .as_ref()
                        .map_or(&[], |markers| markers.markers()),
                    &bracket_matches,
                    colors,
                    document.metadata.read_only,
                )
            });
        #[cfg(feature = "native-host")]
        let minimap_layout = if let Some(dimensions) = minimap_dimensions {
            let minimap_layout = input_state.minimap.layout(
                &document,
                dimensions,
                crate::editor_minimap_layout::MinimapViewport::new(
                    display.row_count(),
                    f64::from(rect.height()),
                    f64::from(appearance.line_height),
                    f64::from(state.scroll.y),
                    f64::from(content_height),
                    presentation.options.scroll_beyond_last_line,
                ),
            );
            crate::editor_minimap::paint(
                ui,
                &mut input_state.minimap,
                crate::editor_minimap::MinimapFrame {
                    id,
                    document: &document,
                    display,
                    dimensions,
                    layout: &minimap_layout,
                    rect: minimap_rect.unwrap(),
                    appearance,
                    colors: presentation.options.minimap_colors.unwrap(),
                    tokens,
                    tab_size: indent.tab_size,
                    selection: &state.selection,
                    marks: &scroll_marks,
                    hovered: minimap_input
                        .as_ref()
                        .is_some_and(|input| input.response.hovered()),
                    horizontal_overflow: state.scroll.x + text_width < content_width,
                },
            );
            Some(minimap_layout)
        } else {
            input_state.minimap.clear();
            None
        };
        let caret_rect = rows
            .iter()
            .find(|row| row.index == carets.primary_row)
            .map(|row| row.caret_rect(primary.head.min(row.segment.bytes.end)));
        if let Some(anchor) = store.selection_anchor(view)
            && let Some(row) = rows
                .iter()
                .find(|row| row.index == display.row_of_byte(&document, anchor))
        {
            layers.selection_anchor(row, anchor);
        }
        if focused
            && ui.is_enabled()
            && !document.metadata.read_only
            && let Some(cursor_rect) = caret_rect
        {
            if let Some(composition) = &state.composition {
                for (index, selection) in state.selection.selections.iter().enumerate() {
                    if let Some(row) = rows.iter().find(|row| row.index == head_rows[index]) {
                        layers.composition(
                            row.caret_rect(selection.head.min(row.segment.bytes.end)),
                            &composition.preedit,
                        );
                    }
                }
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
        #[cfg(feature = "native-host")]
        if let Some(colors) = presentation.options.overview_colors {
            crate::editor_overview::paint(
                &painter,
                &document,
                display,
                Rect::from_min_max(
                    pos2(
                        (rect.right() - VERTICAL_SCROLLBAR_SIZE).max(rect.left()),
                        rect.top(),
                    ),
                    rect.max,
                ),
                &layout,
                content_height,
                &scroll_marks,
                &state.selection,
                appearance.cursor,
                colors,
            );
        }
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
                #[cfg(feature = "native-host")]
                if let Some(colors) = presentation.options.colors {
                    painter.rect_filled(
                        slider,
                        0.0,
                        if engaged {
                            colors.scrollbar_hover
                        } else {
                            colors.scrollbar
                        }
                        .gamma_multiply(opacity),
                    );
                } else {
                    layers.scrollbar(slider, engaged, opacity);
                }
                #[cfg(not(feature = "native-host"))]
                layers.scrollbar(slider, engaged, opacity);
            }
        }
        let rendered_lines = rows
            .first()
            .zip(rows.last())
            .map_or(visible.clone(), |(first, last)| {
                first.segment.line..last.segment.line + 1
            });
        #[cfg(feature = "native-host")]
        let rendered_lines = minimap_layout
            .filter(|layout| !layout.rows.is_empty())
            .map_or_else(
                || rendered_lines.clone(),
                |layout| {
                    let first = display.segment(&document, layout.rows.start).line;
                    let last = display.segment(&document, layout.rows.end - 1).line;
                    rendered_lines.start.min(first)..rendered_lines.end.max(last + 1)
                },
            );
        #[cfg(feature = "native-host")]
        let mut focus_ids: Vec<Id> = input_state
            .sticky
            .focus_ids()
            .into_iter()
            .chain(input_state.problems.focus_ids())
            .chain(input_state.locations.focus_ids())
            .collect();
        store.set_display(view, projection.cached)?;
        input_state.rendered_viewport = Some(RenderedViewport {
            scroll_top: state.scroll.y,
            line_height: appearance.line_height,
            #[cfg(feature = "native-host")]
            problem_zone: zone_position,
        });
        let geometry = EditorGeometry {
            rect,
            content_rect: text_rect,
            gutter_rect: Rect::from_min_max(rect.min, pos2(text_rect.left(), rect.bottom())),
            line_height: appearance.line_height,
            visible_rows: visible,
            scroll: vec2(state.scroll.x, state.scroll.y),
            #[cfg(feature = "native-host")]
            minimap_rect,
            rows: rows.into(),
        };
        #[cfg(feature = "native-host")]
        if let Some(provider) = documentation.as_deref_mut()
            && let Some(colors) = presentation.options.documentation_colors
        {
            let definition_active = locations
                .as_deref()
                .and_then(|provider| provider.keyboard_link(store, view))
                .is_some();
            focus_ids.extend(input_state.documentation.paint(
                ui,
                store,
                view,
                id,
                &geometry,
                appearance,
                colors,
                provider,
                definition_active,
            )?);
        }
        #[cfg(feature = "native-host")]
        input_state.definition_link.paint(
            ui,
            store,
            view,
            &geometry,
            &response,
            presentation.options.word_wrap,
            location_widget.as_ref().map(|widget| widget.token.clone()),
            &mut locations,
            presentation
                .options
                .location_colors
                .map_or(appearance.foreground, |colors| colors.link),
        )?;
        #[cfg(all(feature = "native-host", feature = "inspection"))]
        let documentation_geometry = input_state.documentation.geometry();
        ui.ctx().data_mut(|data| data.insert_temp(id, input_state));
        Ok(EditorOutput {
            response,
            save_requested: false,
            changed: document.revision != previous.revision,
            rendered_lines,
            errors: output.errors,
            #[cfg(feature = "native-host")]
            focus_ids,
            #[cfg(feature = "native-host")]
            toggle_sticky_scroll,
            #[cfg(all(feature = "native-host", feature = "inspection"))]
            documentation_geometry,
            geometry,
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
        let mut typing = Typing {
            language: context.language,
            indent: context.indent,
            auto_closed: &mut state.auto_closed,
        };
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
                type_text(store, view, text, &mut typing)?;
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
                        commit_composition(store, view, composition.replace, text, &mut typing)?;
                    } else {
                        type_text(store, view, text, &mut typing)?;
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
                        context.projection.map(&document, &current.folds),
                    )?,
                    KeyAction::SelectAll => select_all(store, view)?,
                    KeyAction::Undo => {
                        store.undo(document.id)?;
                    }
                    KeyAction::Redo => {
                        store.redo(document.id)?;
                    }
                    KeyAction::DeleteBackward => {
                        delete_backward(store, view, &mut typing)?;
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
                        insert_line_break(store, view, &mut typing)?;
                    }
                    KeyAction::Tab => {
                        tab_with_language(store, view, context.indent, context.language)?;
                    }
                    KeyAction::Outdent => {
                        taide_native_editor::line_commands::run_line_command(
                            store,
                            view,
                            taide_native_editor::line_commands::LineCommand::OutdentLines,
                            taide_native_editor::line_commands::LineCommandContext {
                                indent: context.indent,
                                language: context.language,
                                syntax: context.language.map_or(
                                    &taide_native_editor::language_configuration::UntokenizedLines,
                                    |language| language.syntax,
                                ),
                                compare: None,
                                transforms: None,
                                word_rules: None,
                            },
                        )?;
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

use std::ops::{Range, RangeInclusive};

use egui::{
    Align2, Color32, CursorIcon, FontId, Id, Rect, Sense, Stroke, Ui, UiBuilder, Vec2, pos2, vec2,
};
use taide_native_editor::decoration::{
    Decoration, DecorationKind, DecorationLayer, LaneMark, Stickiness,
};
use taide_native_editor::document::{DocumentId, EditorError};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{ScrollPosition, ViewId};

use crate::editor_geometry::EditorGeometry;
use crate::editor_surface::{
    EditorOutput, EditorPresentation, EditorRequest, EditorTokens, ExtraZone, NativeEditor,
    SurfaceExtras,
};

/// Widths at or below this render inline (monaco `renderSideBySideInlineBreakpoint`).
pub const INLINE_BREAKPOINT: f32 = 900.0;
/// Initial original-side share of the width (monaco `splitViewDefaultRatio`).
pub const DEFAULT_SPLIT: f32 = 0.5;
pub const MIN_SPLIT: f32 = 0.1;
pub const MAX_SPLIT: f32 = 0.9;
const SASH_WIDTH: f32 = 4.0;
const BORDER_WIDTH: f32 = 1.0;
const DIAGONAL_FILL_PERIOD: f32 = 8.0;
const DIAGONAL_FILL_STROKE: f32 = std::f32::consts::SQRT_2;
const CENTER_DIVISOR: f32 = 2.0;
const DECORATION_Z_ORDER: u8 = 0;
const SIDE_SALT: [&str; 2] = ["taide-diff-original", "taide-diff-modified"];
/// Unchanged lines kept next to every change (monaco `hideUnchangedRegions.contextLineCount`).
pub const UNCHANGED_CONTEXT_LINES: u32 = 3;
/// The fewest lines a collapsed region hides (monaco `hideUnchangedRegions.minimumLineCount`).
pub const UNCHANGED_MINIMUM_LINES: u32 = 3;
/// Lines one press on a collapsed region edge reveals (monaco `hideUnchangedRegions.revealLineCount`).
pub const UNCHANGED_REVEAL_LINES: u32 = 20;
/// Height of the collapsed region bar (monaco `PlaceholderViewZone(.., 24)`).
pub const UNCHANGED_BAR_HEIGHT: f32 = 24.0;
const UNCHANGED_FONT_SIZE: f32 = 13.0;
const CODICON_SIZE: f32 = 16.0;
const UNFOLD_CODICON: char = '\u{eb73}';
const HIDDEN_LINES_LABEL: &str = "hidden lines";
const EDGE_STRIP_ABOVE: f32 = 6.0;
const EDGE_STRIP_BELOW: f32 = 4.0;
const FIRST_HIDEABLE_LINE: u32 = 2;

/// One changed line run in libgit2 hunk header terms (1-based; a side with no lines names the
/// line its counterpart follows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colors {
    pub inserted_line: Color32,
    pub removed_line: Color32,
    pub diagonal_fill: Color32,
    pub border: Color32,
    pub stage: Color32,
    pub foreground: Color32,
    pub unchanged_code: Color32,
    pub unchanged_region: Color32,
    pub unchanged_region_foreground: Color32,
}

pub struct Request<'a> {
    pub original: ViewId,
    pub modified: ViewId,
    pub hunks: &'a [Hunk],
    pub side_by_side: bool,
    pub stage_lanes: bool,
    /// Collapses long unchanged runs behind a bar (monaco `hideUnchangedRegions.enabled`).
    pub hide_unchanged: bool,
    pub presentation: &'a EditorPresentation,
    pub colors: Colors,
}

/// An unchanged run the diff can collapse (monaco `UnchangedRegion`): 1-based first lines on each
/// side and the line count both sides share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnchangedRegion {
    pub original_start: u32,
    pub modified_start: u32,
    pub line_count: u32,
}

/// Lines shown at the top and the bottom of an [`UnchangedRegion`] (monaco `visibleLineCountTop`
/// and `visibleLineCountBottom`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Reveal {
    pub top: u32,
    pub bottom: u32,
}

/// The lines of an [`UnchangedRegion`] still hidden under a [`Reveal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hidden {
    pub original_first: u32,
    pub modified_first: u32,
    pub count: u32,
}

impl UnchangedRegion {
    /// The hidden part under `reveal`; the first document line always stays shown because the
    /// editor display never hides it.
    pub fn hidden(&self, reveal: Reveal) -> Hidden {
        let skip = FIRST_HIDEABLE_LINE
            .saturating_sub(self.original_start.min(self.modified_start) + reveal.top);
        let first = reveal.top + skip;
        Hidden {
            original_first: self.original_start + first,
            modified_first: self.modified_start + first,
            count: self.line_count.saturating_sub(first + reveal.bottom),
        }
    }

    /// monaco `showMoreAbove(revealLineCount)`.
    pub fn show_more_above(&self, reveal: Reveal) -> Reveal {
        Reveal {
            top: (reveal.top + UNCHANGED_REVEAL_LINES)
                .min(self.line_count.saturating_sub(reveal.bottom)),
            ..reveal
        }
    }

    /// monaco `showMoreBelow(revealLineCount)`.
    pub fn show_more_below(&self, reveal: Reveal) -> Reveal {
        Reveal {
            bottom: (reveal.bottom + UNCHANGED_REVEAL_LINES)
                .min(self.line_count.saturating_sub(reveal.top)),
            ..reveal
        }
    }

    /// monaco `showAll`.
    pub fn show_all(&self, reveal: Reveal) -> Reveal {
        Reveal {
            bottom: self.line_count.saturating_sub(reveal.top),
            ..reveal
        }
    }
}

fn side_lines(start: u32, lines: u32) -> Range<u32> {
    if lines == 0 {
        start + 1..start + 1
    } else {
        start..start + lines
    }
}

/// monaco `UnchangedRegion.fromDiffs`: each unchanged run between hunks that still hides
/// [`UNCHANGED_MINIMUM_LINES`] after keeping [`UNCHANGED_CONTEXT_LINES`] next to every change
/// (a run touching the document start or end keeps context only on its inner side).
pub fn unchanged_regions(
    hunks: &[Hunk],
    original_lines: u32,
    modified_lines: u32,
) -> Vec<UnchangedRegion> {
    let mut runs = Vec::with_capacity(hunks.len() + 1);
    let (mut original, mut modified) = (1, 1);
    for hunk in hunks {
        let old = side_lines(hunk.old_start, hunk.old_lines);
        let new = side_lines(hunk.new_start, hunk.new_lines);
        runs.push((original, modified, old.start.saturating_sub(original)));
        (original, modified) = (old.end, new.end);
    }
    runs.push((
        original,
        modified,
        (original_lines + 1).saturating_sub(original),
    ));
    runs.into_iter()
        .filter_map(|(original_start, modified_start, length)| {
            let at_start = original_start == 1 && modified_start == 1;
            let at_end = original_start + length == original_lines + 1
                && modified_start + length == modified_lines + 1;
            if (at_start || at_end) && length >= UNCHANGED_CONTEXT_LINES + UNCHANGED_MINIMUM_LINES {
                let shift = if at_end && !at_start {
                    UNCHANGED_CONTEXT_LINES
                } else {
                    0
                };
                let trimmed = if at_start == at_end {
                    0
                } else {
                    UNCHANGED_CONTEXT_LINES
                };
                return Some(UnchangedRegion {
                    original_start: original_start + shift,
                    modified_start: modified_start + shift,
                    line_count: length - trimmed,
                });
            }
            (length >= UNCHANGED_CONTEXT_LINES * 2 + UNCHANGED_MINIMUM_LINES).then(|| {
                UnchangedRegion {
                    original_start: original_start + UNCHANGED_CONTEXT_LINES,
                    modified_start: modified_start + UNCHANGED_CONTEXT_LINES,
                    line_count: length - UNCHANGED_CONTEXT_LINES * 2,
                }
            })
        })
        .collect()
}

/// Supplies a document's syntax tokens for the frame, borrowing the caller's token state.
pub trait Tokens {
    fn tokens(&mut self, store: &EditorStore, document: DocumentId) -> Option<EditorTokens<'_>>;
}

pub struct Output {
    pub is_side_by_side: bool,
    /// The 1-based inclusive modified-side hunk range whose gutter lane was pressed.
    pub lane_press: Option<RangeInclusive<u32>>,
    pub original: Option<EditorOutput>,
    pub modified: EditorOutput,
    pub sash: Option<Rect>,
}

#[derive(Clone)]
struct State {
    split: f32,
    scroll: Option<Vec2>,
    reveals: Vec<(UnchangedRegion, Reveal)>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            split: DEFAULT_SPLIT,
            scroll: None,
            reveals: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BarPress {
    Above,
    Below,
    All,
}

/// A collapsed region with its bar zone index on each side.
struct Collapsed {
    index: usize,
    hidden: Hidden,
}

fn after_line(anchor: u32) -> Option<usize> {
    (anchor as usize).checked_sub(1)
}

fn changed_lines(start: u32, count: u32) -> Range<usize> {
    let first = (start as usize).saturating_sub(1);
    first..first + count as usize
}

/// Filler bands that keep both sides aligned: the shorter side of every hunk gets the line count
/// difference after its last line of that hunk.
pub fn fillers(hunks: &[Hunk], line_height: f32) -> (Vec<ExtraZone>, Vec<ExtraZone>) {
    let mut original = Vec::new();
    let mut modified = Vec::new();
    for hunk in hunks {
        let (shorter, start, lines, difference) = if hunk.old_lines < hunk.new_lines {
            (
                &mut original,
                hunk.old_start,
                hunk.old_lines,
                hunk.new_lines - hunk.old_lines,
            )
        } else {
            (
                &mut modified,
                hunk.new_start,
                hunk.new_lines,
                hunk.old_lines - hunk.new_lines,
            )
        };
        if difference == 0 {
            continue;
        }
        let last_line = if lines == 0 { start } else { start + lines - 1 };
        shorter.push(ExtraZone {
            after_line: after_line(last_line),
            height: difference as f32 * line_height,
        });
    }
    (original, modified)
}

/// Inline mode's removed-line bands on the modified side, each with the original lines it shows.
pub fn inline_zones(hunks: &[Hunk], line_height: f32) -> Vec<(ExtraZone, Range<usize>)> {
    hunks
        .iter()
        .filter(|hunk| hunk.old_lines > 0)
        .map(|hunk| {
            let anchor = if hunk.new_lines == 0 {
                hunk.new_start
            } else {
                hunk.new_start - 1
            };
            (
                ExtraZone {
                    after_line: after_line(anchor),
                    height: hunk.old_lines as f32 * line_height,
                },
                changed_lines(hunk.old_start, hunk.old_lines),
            )
        })
        .collect()
}

/// Gutter stage ranges on the modified side, the backend `gutter_range` rule (a pure deletion
/// marks `max(new_start, 1)`).
pub fn stage_ranges(hunks: &[Hunk]) -> Vec<RangeInclusive<u32>> {
    hunks
        .iter()
        .map(|hunk| {
            if hunk.new_lines == 0 {
                let marker = hunk.new_start.max(1);
                marker..=marker
            } else {
                hunk.new_start..=hunk.new_start + hunk.new_lines - 1
            }
        })
        .collect()
}

pub fn is_side_by_side(requested: bool, width: f32) -> bool {
    requested && width > INLINE_BREAKPOINT
}

fn layer(
    store: &EditorStore,
    document: DocumentId,
    items: Vec<(Range<usize>, DecorationKind)>,
) -> Result<DecorationLayer, EditorError> {
    let snapshot = store.documents().snapshot(document)?;
    let count = snapshot.rope.len_lines();
    Ok(DecorationLayer::new(
        snapshot.revision,
        DECORATION_Z_ORDER,
        items
            .into_iter()
            .filter(|(lines, _)| !lines.is_empty() && lines.start < count)
            .map(|(lines, kind)| Decoration {
                bytes: snapshot.rope.line_to_byte(lines.start)
                    ..snapshot.rope.line_to_byte((lines.end - 1).min(count - 1)),
                kind,
                stickiness: Stickiness::NeverGrowsWhenTypingAtEdges,
            })
            .collect(),
    ))
}

fn rgba(color: Color32) -> [u8; 4] {
    color.to_srgba_unmultiplied()
}

fn paint_diagonal_fill(ui: &Ui, clip: Rect, zone: Rect, color: Color32) {
    let painter = ui.painter().with_clip_rect(zone.intersect(clip));
    let stroke = Stroke::new(DIAGONAL_FILL_STROKE, color);
    let mut x = zone.left() - zone.height();
    while x < zone.right() {
        painter.line_segment(
            [pos2(x, zone.bottom()), pos2(x + zone.height(), zone.top())],
            stroke,
        );
        x += DIAGONAL_FILL_PERIOD;
    }
}

struct Side<'a> {
    view: ViewId,
    rect: Rect,
    zones: Vec<ExtraZone>,
    hidden_lines: Vec<Range<usize>>,
    decorations: Vec<(Range<usize>, DecorationKind)>,
    presentation: &'a EditorPresentation,
    salt: &'static str,
}

fn show_side(
    ui: &mut Ui,
    store: &mut EditorStore,
    editor: &NativeEditor,
    side: Side<'_>,
    tokens: &mut dyn Tokens,
) -> Result<EditorOutput, EditorError> {
    let document = store
        .views()
        .get(side.view)
        .ok_or(EditorError::NotFound)?
        .document;
    let decorations = layer(store, document, side.decorations)?;
    let side_tokens = tokens.tokens(store, document);
    let mut child = ui.new_child(UiBuilder::new().id_salt(side.salt).max_rect(side.rect));
    child.set_clip_rect(side.rect.intersect(ui.clip_rect()));
    editor.show_request_with_extras(
        &mut child,
        store,
        side.view,
        EditorRequest {
            request_focus: false,
            keymap: |_: &Ui, _: &egui::Event, _: bool| false,
            route: |_: &egui::Response| None,
            presentation: side.presentation,
            tokens: move |_: &EditorStore| side_tokens,
            language: None,
            decorations: &[&decorations],
            fold_commands: &[],
            fold_controls: None,
            problems: None,
            locations: None,
            documentation: None,
            documentation_commands: &[],
            completion: None,
            completion_commands: &[],
            syntax_folds: None,
        },
        |_, _, _, _, _| false,
        &SurfaceExtras {
            zones: side.zones,
            hidden_lines: side.hidden_lines,
        },
    )
}

/// Paints a collapsed region bar over `zone` (monaco `CollapsedCodeOverlayWidget`): the unfold
/// icon over the gutter, the hidden line count over the text, and press strips on both edges.
fn unchanged_bar(
    ui: &Ui,
    id: Id,
    output: &EditorOutput,
    zone: Rect,
    hidden: u32,
    reveal: Reveal,
    colors: Colors,
) -> Option<BarPress> {
    let geometry = &output.geometry;
    let bar = Rect::from_x_y_ranges(geometry.rect.x_range(), zone.y_range());
    let painter = ui
        .painter()
        .with_clip_rect(geometry.rect.intersect(ui.clip_rect()));
    painter.rect_filled(bar, 0.0, colors.unchanged_region);
    let icon_cell = Rect::from_x_y_ranges(
        geometry.rect.left()..=geometry.content_rect.left(),
        bar.y_range(),
    );
    let label_cell = Rect::from_x_y_ranges(
        geometry.content_rect.left()..=geometry.rect.right(),
        bar.y_range(),
    );
    painter.text(
        icon_cell.center(),
        Align2::CENTER_CENTER,
        UNFOLD_CODICON,
        FontId::new(CODICON_SIZE, crate::font_families::codicons(ui)),
        colors.unchanged_region_foreground,
    );
    painter.text(
        label_cell.center(),
        Align2::CENTER_CENTER,
        format!("{hidden} {HIDDEN_LINES_LABEL}"),
        FontId::proportional(UNCHANGED_FONT_SIZE),
        colors.unchanged_region_foreground,
    );
    let label = ui.interact(label_cell, id.with("label"), Sense::click());
    let icon = ui
        .interact(icon_cell, id.with("icon"), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand);
    let strip = |edge: f32| {
        Rect::from_x_y_ranges(
            bar.x_range(),
            edge - EDGE_STRIP_ABOVE..=edge + EDGE_STRIP_BELOW,
        )
    };
    let above = ui
        .interact(strip(bar.top()), id.with("above"), Sense::click())
        .on_hover_cursor(if reveal.top > 0 {
            CursorIcon::ResizeVertical
        } else {
            CursorIcon::ResizeSouth
        });
    let below = ui
        .interact(strip(bar.bottom()), id.with("below"), Sense::click())
        .on_hover_cursor(if reveal.bottom > 0 {
            CursorIcon::ResizeVertical
        } else {
            CursorIcon::ResizeNorth
        });
    if above.clicked() {
        return Some(BarPress::Above);
    }
    if below.clicked() {
        return Some(BarPress::Below);
    }
    (icon.clicked() || label.double_clicked()).then_some(BarPress::All)
}

struct Collapse {
    bars: Vec<ExtraZone>,
    hidden_lines: Vec<Range<usize>>,
    backgrounds: Vec<(Range<usize>, DecorationKind)>,
}

/// One side's collapsed region bars, hidden lines and unchanged code backgrounds.
fn collapse(
    regions: &[UnchangedRegion],
    collapsed: &[Collapsed],
    is_original: bool,
    colors: Colors,
) -> Collapse {
    let first = |hidden: &Hidden| {
        if is_original {
            hidden.original_first
        } else {
            hidden.modified_first
        }
    };
    Collapse {
        bars: collapsed
            .iter()
            .map(|item| ExtraZone {
                after_line: after_line(first(&item.hidden) - 1),
                height: UNCHANGED_BAR_HEIGHT,
            })
            .collect(),
        hidden_lines: collapsed
            .iter()
            .map(|item| changed_lines(first(&item.hidden), item.hidden.count))
            .collect(),
        backgrounds: regions
            .iter()
            .map(|region| {
                let start = if is_original {
                    region.original_start
                } else {
                    region.modified_start
                };
                (
                    changed_lines(start, region.line_count),
                    DecorationKind::LineBackground(rgba(colors.unchanged_code)),
                )
            })
            .collect(),
    }
}

fn press_bars(
    ui: &Ui,
    id: Id,
    output: &EditorOutput,
    first_bar: usize,
    collapsed: &[Collapsed],
    reveals: &[Reveal],
    colors: Colors,
) -> Option<(usize, BarPress)> {
    let mut pressed = None;
    for (order, item) in collapsed.iter().enumerate() {
        let Some(Some(zone)) = output.extra_zones.get(first_bar + order) else {
            continue;
        };
        if let Some(press) = unchanged_bar(
            ui,
            id.with(("unchanged", item.index)),
            output,
            *zone,
            item.hidden.count,
            reveals[item.index],
            colors,
        ) {
            pressed = Some((item.index, press));
        }
    }
    pressed
}

/// Applies a bar press and returns the scroll that keeps the lines below a region in place after
/// revealing its bottom (monaco keeps `endLineNumberExclusive` at the same screen top).
fn apply_press(
    regions: &[UnchangedRegion],
    reveals: &mut [Reveal],
    pressed: Option<(usize, BarPress)>,
    line_height: f32,
) -> f32 {
    let Some((index, press)) = pressed else {
        return 0.0;
    };
    let region = regions[index];
    let before = reveals[index];
    let after = match press {
        BarPress::Above => region.show_more_above(before),
        BarPress::Below => region.show_more_below(before),
        BarPress::All => region.show_all(before),
    };
    reveals[index] = after;
    if press != BarPress::Below {
        return 0.0;
    }
    let removed_bar = if region.hidden(after).count == 0 {
        UNCHANGED_BAR_HEIGHT
    } else {
        0.0
    };
    (after.bottom - before.bottom) as f32 * line_height - removed_bar
}

fn scroll_by(store: &mut EditorStore, view: ViewId, delta: f32) -> Result<(), EditorError> {
    let current = store.views().get(view).ok_or(EditorError::NotFound)?;
    let scroll = ScrollPosition {
        x: current.scroll.x,
        y: current.scroll.y + delta,
    };
    let (selection, folds) = (current.selection.clone(), current.folds.clone());
    store.set_view_state(view, selection, scroll, folds)
}

fn paint_removed_lines(
    ui: &Ui,
    store: &EditorStore,
    editor: &NativeEditor,
    original: DocumentId,
    output: &EditorOutput,
    zones: &[(ExtraZone, Range<usize>)],
    colors: Colors,
) -> Result<(), EditorError> {
    let snapshot = store.documents().snapshot(original)?;
    let geometry = &output.geometry;
    let painter = ui
        .painter()
        .with_clip_rect(geometry.rect.intersect(ui.clip_rect()));
    for (zone, (_, lines)) in output.extra_zones.iter().zip(zones) {
        let Some(zone) = zone else {
            continue;
        };
        painter.rect_filled(
            Rect::from_x_y_ranges(geometry.rect.x_range(), zone.y_range()),
            0.0,
            colors.removed_line,
        );
        for (index, line) in lines.clone().enumerate() {
            if line >= snapshot.rope.len_lines() {
                break;
            }
            let text = snapshot.rope.line(line).to_string();
            let galley = painter.layout_no_wrap(
                text.trim_end_matches(['\n', '\r']).to_owned(),
                editor.appearance.font.clone(),
                colors.foreground,
            );
            let top = zone.top()
                + index as f32 * geometry.line_height
                + (geometry.line_height - galley.size().y) / CENTER_DIVISOR;
            painter.galley(
                pos2(
                    geometry.content_rect.left() + editor.appearance.horizontal_padding
                        - geometry.scroll.x,
                    top,
                ),
                galley,
                colors.foreground,
            );
        }
    }
    Ok(())
}

fn lane_press(
    ui: &Ui,
    output: &EditorOutput,
    ranges: &[RangeInclusive<u32>],
    has_folding: bool,
) -> Option<RangeInclusive<u32>> {
    let range_of = |line: u32| ranges.iter().find(|range| range.contains(&line)).cloned();
    let line = lane_line(
        ui,
        (&output.geometry, &output.response),
        has_folding,
        true,
        |line| range_of(line).is_some(),
    )?;
    range_of(line)
}

/// The 1-based document line whose gutter lane (line decorations column) the primary pointer
/// pressed this frame, among the lines `accepts` takes; with `pointer` the hovered lane of such a
/// line shows the pointing hand.
pub fn lane_line(
    ui: &Ui,
    (geometry, response): (&EditorGeometry, &egui::Response),
    has_folding: bool,
    pointer: bool,
    accepts: impl Fn(u32) -> bool,
) -> Option<u32> {
    let column = crate::editor_gutter::lane_column(geometry.gutter_rect, has_folding);
    let line_at = |position: egui::Pos2| {
        if !column.contains(position.x) || !geometry.gutter_rect.contains(position) {
            return None;
        }
        let row = geometry.rows.iter().find(|row| {
            (row.origin.y..row.origin.y + geometry.line_height).contains(&position.y)
        })?;
        let line = row.segment.line as u32 + 1;
        accepts(line).then_some(line)
    };
    if pointer
        && response.hovered()
        && let Some(position) = ui.input(|input| input.pointer.hover_pos())
        && line_at(position).is_some()
    {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let pressed = ui.input(|input| {
        input.events.iter().find_map(|event| match event {
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                ..
            } => Some(*pos),
            _ => None,
        })
    })?;
    if !response.hovered() {
        return None;
    }
    line_at(pressed)
}

/// Draws a two-sided diff over the original and modified views: side by side with filler bands
/// above `INLINE_BREAKPOINT` when asked, otherwise the modified side with the removed lines in
/// bands; both sides scroll together, the modified gutter can offer hunk stage lanes, and long
/// unchanged regions can collapse behind bars.
pub fn show(
    ui: &mut Ui,
    store: &mut EditorStore,
    editor: &NativeEditor,
    request: Request<'_>,
    tokens: &mut dyn Tokens,
) -> Result<Output, EditorError> {
    let rect = ui.available_rect_before_wrap();
    ui.allocate_rect(rect, Sense::hover());
    let is_side_by_side = is_side_by_side(request.side_by_side, rect.width());
    let line_height = editor.appearance.line_height;
    let mut presentation = request.presentation.clone();
    presentation.options.folding = false;
    presentation.options.minimap = false;
    presentation.options.sticky_scroll = false;
    let memory = Id::new(("taide-diff-view", request.modified));
    let mut state = ui
        .ctx()
        .data(|data| data.get_temp::<State>(memory))
        .unwrap_or_default();
    let ranges = if request.stage_lanes {
        stage_ranges(request.hunks)
    } else {
        Vec::new()
    };
    let stage_items = ranges.iter().map(|range| {
        (
            *range.start() as usize - 1..*range.end() as usize,
            DecorationKind::Lane {
                mark: LaneMark::Bar,
                color: rgba(request.colors.stage),
            },
        )
    });
    let inserted_items = request.hunks.iter().map(|hunk| {
        (
            changed_lines(hunk.new_start, hunk.new_lines),
            DecorationKind::LineBackground(rgba(request.colors.inserted_line)),
        )
    });
    let original_document = store
        .views()
        .get(request.original)
        .ok_or(EditorError::NotFound)?
        .document;
    let modified_document = store
        .views()
        .get(request.modified)
        .ok_or(EditorError::NotFound)?
        .document;
    let regions = if request.hide_unchanged {
        unchanged_regions(
            request.hunks,
            store
                .documents()
                .snapshot(original_document)?
                .rope
                .len_lines() as u32,
            store
                .documents()
                .snapshot(modified_document)?
                .rope
                .len_lines() as u32,
        )
    } else {
        Vec::new()
    };
    let mut reveals = regions
        .iter()
        .map(|region| {
            state
                .reveals
                .iter()
                .find(|(known, _)| known == region)
                .map_or_else(Reveal::default, |(_, reveal)| *reveal)
        })
        .collect::<Vec<_>>();
    let collapsed = regions
        .iter()
        .zip(&reveals)
        .enumerate()
        .filter_map(|(index, (region, reveal))| {
            let hidden = region.hidden(*reveal);
            (hidden.count > 0).then_some(Collapsed { index, hidden })
        })
        .collect::<Vec<_>>();
    let [original_collapse, modified_collapse] = [true, false]
        .map(|is_original| collapse(&regions, &collapsed, is_original, request.colors));
    let modified_items = inserted_items
        .chain(stage_items)
        .chain(modified_collapse.backgrounds)
        .collect::<Vec<_>>();
    let mut pressed = None;
    if !is_side_by_side {
        let zones = inline_zones(request.hunks, line_height);
        let modified = show_side(
            ui,
            store,
            editor,
            Side {
                view: request.modified,
                rect,
                zones: zones
                    .iter()
                    .map(|(zone, _)| *zone)
                    .chain(modified_collapse.bars)
                    .collect(),
                hidden_lines: modified_collapse.hidden_lines,
                decorations: modified_items,
                presentation: &presentation,
                salt: SIDE_SALT[1],
            },
            tokens,
        )?;
        paint_removed_lines(
            ui,
            store,
            editor,
            original_document,
            &modified,
            &zones,
            request.colors,
        )?;
        pressed = pressed.or(press_bars(
            ui,
            memory.with(SIDE_SALT[1]),
            &modified,
            zones.len(),
            &collapsed,
            &reveals,
            request.colors,
        ));
        let scroll_delta = apply_press(&regions, &mut reveals, pressed, line_height);
        if scroll_delta != 0.0 {
            scroll_by(store, request.modified, scroll_delta)?;
        }
        if pressed.is_some() {
            ui.ctx().request_repaint();
        }
        state.reveals = regions.into_iter().zip(reveals).collect();
        ui.ctx().data_mut(|data| data.insert_temp(memory, state));
        let lane_press = lane_press(ui, &modified, &ranges, presentation.options.folding);
        return Ok(Output {
            is_side_by_side,
            lane_press,
            original: None,
            modified,
            sash: None,
        });
    }
    let split_x = rect.left() + (rect.width() * state.split).round();
    let original_rect = Rect::from_min_max(rect.min, pos2(split_x, rect.bottom()));
    let modified_rect = Rect::from_min_max(pos2(split_x + BORDER_WIDTH, rect.top()), rect.max);
    let (original_fillers, modified_fillers) = fillers(request.hunks, line_height);
    let removed_items = request
        .hunks
        .iter()
        .map(|hunk| {
            (
                changed_lines(hunk.old_start, hunk.old_lines),
                DecorationKind::LineBackground(rgba(request.colors.removed_line)),
            )
        })
        .chain(original_collapse.backgrounds)
        .collect();
    let filler_counts = [original_fillers.len(), modified_fillers.len()];
    let original = show_side(
        ui,
        store,
        editor,
        Side {
            view: request.original,
            rect: original_rect,
            zones: original_fillers
                .into_iter()
                .chain(original_collapse.bars)
                .collect(),
            hidden_lines: original_collapse.hidden_lines,
            decorations: removed_items,
            presentation: &presentation,
            salt: SIDE_SALT[0],
        },
        tokens,
    )?;
    let modified = show_side(
        ui,
        store,
        editor,
        Side {
            view: request.modified,
            rect: modified_rect,
            zones: modified_fillers
                .into_iter()
                .chain(modified_collapse.bars)
                .collect(),
            hidden_lines: modified_collapse.hidden_lines,
            decorations: modified_items,
            presentation: &presentation,
            salt: SIDE_SALT[1],
        },
        tokens,
    )?;
    for ((output, side_rect), fillers) in [(&original, original_rect), (&modified, modified_rect)]
        .into_iter()
        .zip(filler_counts)
    {
        for zone in output.extra_zones.iter().take(fillers).flatten() {
            paint_diagonal_fill(ui, side_rect, *zone, request.colors.diagonal_fill);
        }
    }
    for ((output, salt), fillers) in [(&original, SIDE_SALT[0]), (&modified, SIDE_SALT[1])]
        .into_iter()
        .zip(filler_counts)
    {
        pressed = pressed.or(press_bars(
            ui,
            memory.with(salt),
            output,
            fillers,
            &collapsed,
            &reveals,
            request.colors,
        ));
    }
    let scroll_delta = apply_press(&regions, &mut reveals, pressed, line_height);
    if pressed.is_some() {
        ui.ctx().request_repaint();
    }
    state.reveals = regions.into_iter().zip(reveals).collect();
    ui.painter().rect_filled(
        Rect::from_min_max(
            pos2(split_x, rect.top()),
            pos2(split_x + BORDER_WIDTH, rect.bottom()),
        ),
        0.0,
        request.colors.border,
    );
    let sash = Rect::from_center_size(
        pos2(split_x + BORDER_WIDTH / CENTER_DIVISOR, rect.center().y),
        vec2(SASH_WIDTH, rect.height()),
    );
    let sash_response = ui
        .interact(sash, memory.with("sash"), Sense::drag())
        .on_hover_cursor(CursorIcon::ResizeHorizontal);
    if sash_response.dragged()
        && let Some(position) = sash_response.interact_pointer_pos()
        && rect.width() > 0.0
    {
        state.split = ((position.x - rect.left()) / rect.width()).clamp(MIN_SPLIT, MAX_SPLIT);
        ui.ctx().request_repaint();
    }
    let [original_scroll, modified_scroll] =
        [&original, &modified].map(|output| output.geometry.scroll);
    let target = match state.scroll {
        Some(last) if modified_scroll != last => modified_scroll,
        Some(last) if original_scroll != last => original_scroll,
        _ => modified_scroll,
    } + vec2(0.0, scroll_delta);
    for (view, scroll) in [
        (request.original, original_scroll),
        (request.modified, modified_scroll),
    ] {
        if scroll != target {
            let current = store.views().get(view).ok_or(EditorError::NotFound)?;
            let (selection, folds) = (current.selection.clone(), current.folds.clone());
            store.set_view_state(
                view,
                selection,
                ScrollPosition {
                    x: target.x,
                    y: target.y,
                },
                folds,
            )?;
            ui.ctx().request_repaint();
        }
    }
    state.scroll = Some(target);
    ui.ctx().data_mut(|data| data.insert_temp(memory, state));
    let lane_press = lane_press(ui, &modified, &ranges, presentation.options.folding);
    Ok(Output {
        is_side_by_side,
        lane_press,
        original: Some(original),
        modified,
        sash: Some(sash),
    })
}

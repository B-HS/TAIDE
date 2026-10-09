use std::borrow::Cow;
use std::ops::Range;

use egui::{Color32, Painter, Rect, pos2};
use taide_native_editor::bracket_model::BracketMatch;
use taide_native_editor::decoration::{DecorationKind, DecorationLayer, OverviewLane};
use taide_native_editor::diagnostics::{Marker, Severity};
use taide_native_editor::display_layout::VerticalLayout;
use taide_native_editor::display_map::DisplayMap;
use taide_native_editor::document::DocumentSnapshot;
use taide_native_editor::view::SelectionSet;

const MIN_MARK_HEIGHT: f32 = 6.0;
const LANES: f32 = 3.0;
const HALF: f32 = 2.0;
const CURSOR_OPACITY: f32 = 0.7;
const CURSOR_HEIGHT: f32 = 2.0;
const BORDER_WIDTH: f32 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverviewColors {
    pub error: Color32,
    pub warning: Color32,
    pub information: Color32,
    pub find: Color32,
    pub minimap_find: Color32,
    pub bracket: Color32,
    pub border: Color32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScrollMark {
    pub(crate) bytes: Range<usize>,
    pub(crate) lane: OverviewLane,
    pub(crate) color: Color32,
    pub(crate) minimap: Option<Color32>,
}

pub(crate) fn marks(
    layers: &[Cow<'_, DecorationLayer>],
    diagnostics: &[Marker],
    brackets: &[BracketMatch],
    colors: OverviewColors,
    read_only: bool,
) -> Vec<ScrollMark> {
    let mut marks = layers
        .iter()
        .flat_map(|layer| layer.items())
        .filter_map(|decoration| {
            let DecorationKind::Overview {
                lane,
                color,
                minimap,
            } = decoration.kind
            else {
                return None;
            };
            Some(ScrollMark {
                bytes: decoration.bytes.clone(),
                lane,
                color: crate::editor_paint::color32(color),
                minimap: minimap.map(crate::editor_paint::color32),
            })
        })
        .collect::<Vec<_>>();
    if !read_only {
        let mut diagnostics = diagnostics
            .iter()
            .filter(|marker| marker.message.severity != Severity::Hint)
            .collect::<Vec<_>>();
        diagnostics.sort_by_key(|marker| std::cmp::Reverse(marker.message.severity));
        marks.extend(diagnostics.into_iter().map(|marker| {
            let color = match marker.message.severity {
                Severity::Warning => colors.warning,
                Severity::Information => colors.information,
                _ => colors.error,
            };
            ScrollMark {
                bytes: marker.bytes.clone(),
                lane: OverviewLane::Right,
                color,
                minimap: Some(color),
            }
        }));
    }
    marks.extend(
        brackets
            .iter()
            .filter(|matched| matched.is_near)
            .flat_map(|matched| [&matched.open, &matched.close])
            .map(|bytes| ScrollMark {
                bytes: bytes.clone(),
                lane: OverviewLane::Center,
                color: colors.bracket,
                minimap: None,
            }),
    );
    marks
}

pub(crate) fn paint(
    painter: &Painter,
    document: &DocumentSnapshot,
    display: &DisplayMap,
    rect: Rect,
    layout: &VerticalLayout,
    content_height: f32,
    marks: &[ScrollMark],
    selections: &SelectionSet,
    cursor: Color32,
    colors: OverviewColors,
) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 || content_height <= 0.0 {
        return;
    }
    let painter = painter.with_clip_rect(rect);
    let content_height = content_height.max(rect.height());
    let unit = 1.0 / painter.ctx().pixels_per_point();
    let lane_width = ((rect.width() - unit) / LANES / unit).floor() * unit;
    let center_width = rect.width() - unit - HALF * lane_width;
    let mut groups: Vec<(OverviewLane, Color32, Vec<(f32, f32)>)> = Vec::new();
    for mark in marks {
        let first = display.row_of_byte(document, mark.bytes.start.min(document.rope.len_bytes()));
        let last = display.row_of_byte(document, mark.bytes.end.min(document.rope.len_bytes()));
        let range = projected(
            rect,
            layout.row_top(first),
            layout.row_bottom(last),
            content_height,
            MIN_MARK_HEIGHT,
        );
        if let Some(group) = groups
            .iter_mut()
            .find(|group| group.0 == mark.lane && group.1 == mark.color)
        {
            group.2.push(range);
        } else {
            groups.push((mark.lane, mark.color, vec![range]));
        }
    }
    for (lane, color, ranges) in groups {
        let (left, width) = match lane {
            OverviewLane::Left => (rect.left() + unit, lane_width),
            OverviewLane::Center => (rect.left() + unit + lane_width, center_width),
            OverviewLane::Right => (rect.right() - lane_width, lane_width),
            OverviewLane::Full => (rect.left() + unit, rect.width() - unit),
        };
        paint_ranges(&painter, left, width, ranges, color, unit);
    }
    let mut cursors = Vec::new();
    for selection in &selections.selections {
        let row = display.row_of_byte(document, selection.head);
        let range = projected(
            rect,
            layout.row_top(row),
            layout.row_top(row),
            content_height,
            CURSOR_HEIGHT,
        );
        cursors.push(range);
    }
    paint_ranges(
        &painter,
        rect.left() + unit,
        rect.width() - unit,
        cursors,
        cursor.gamma_multiply(CURSOR_OPACITY),
        unit,
    );
    painter.rect_filled(
        Rect::from_min_max(rect.min, pos2(rect.left() + BORDER_WIDTH, rect.bottom())),
        0.0,
        colors.border,
    );
}

fn paint_ranges(
    painter: &Painter,
    left: f32,
    width: f32,
    mut ranges: Vec<(f32, f32)>,
    color: Color32,
    gap: f32,
) {
    ranges.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f32, f32)> = Vec::new();
    for range in ranges {
        if let Some(last) = merged.last_mut()
            && range.0 <= last.1 + gap
        {
            last.1 = last.1.max(range.1);
        } else {
            merged.push(range);
        }
    }
    for range in merged {
        painter.rect_filled(
            Rect::from_min_max(pos2(left, range.0), pos2(left + width, range.1)),
            0.0,
            color,
        );
    }
}

fn projected(rect: Rect, top: f32, bottom: f32, content: f32, minimum: f32) -> (f32, f32) {
    let top = rect.height() * top / content;
    let bottom = rect.height() * bottom / content;
    let half = ((bottom - top) / HALF)
        .max(minimum / HALF)
        .min(rect.height() / HALF);
    let center = ((top + bottom) / HALF).clamp(half, rect.height() - half);
    (rect.top() + center - half, rect.top() + center + half)
}

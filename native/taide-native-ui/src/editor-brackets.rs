use egui::{Color32, Painter, Rect, Stroke, StrokeKind, pos2, vec2};
use taide_native_editor::bracket_model::{BracketMatch, BracketModel, BracketPairInfo};
use taide_native_editor::display_map::DisplayMap;
use taide_native_editor::document::DocumentSnapshot;
use taide_native_editor::view::SelectionSet;

use crate::editor_geometry::Row;
use crate::editor_surface::{EditorAppearance, EditorDisplayOptions};

const GUIDE_WIDTH: f32 = 1.0;
const INACTIVE_GUIDE_OPACITY: f32 = 0.3;
const MAX_MATCH_SELECTIONS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorBracketColors {
    pub palette: [Color32; 3],
    pub unexpected: Color32,
    pub indent: Color32,
    pub active_indent: Color32,
    pub match_background: Color32,
    pub match_border: Color32,
}

pub(crate) fn matching_brackets(
    model: &BracketModel,
    selections: &SelectionSet,
    focused: bool,
) -> Vec<BracketMatch> {
    if !focused || selections.selections.len() > MAX_MATCH_SELECTIONS {
        return Vec::new();
    }
    let mut matches = selections
        .selections
        .iter()
        .filter(|selection| selection.anchor == selection.head)
        .filter_map(|selection| model.matching_brackets(selection.head))
        .collect::<Vec<_>>();
    matches.sort_by_key(|matched| (matched.open.start, matched.close.end, !matched.is_near));
    matches.dedup_by(|current, previous| {
        current.open == previous.open && current.close == previous.close
    });
    matches
}

pub(crate) fn paint_matching(
    painter: &Painter,
    rows: &[Row],
    matches: &[BracketMatch],
    colors: &EditorBracketColors,
    line_height: f32,
) {
    for matched in matches {
        for bytes in [&matched.open, &matched.close] {
            for row in rows {
                if let Some(extent) = row.extent(bytes) {
                    painter.rect(
                        Rect::from_min_max(
                            pos2(extent.min, row.origin.y),
                            pos2(extent.max, row.origin.y + line_height),
                        ),
                        0.0,
                        colors.match_background,
                        Stroke::new(GUIDE_WIDTH, colors.match_border),
                        StrokeKind::Inside,
                    );
                }
            }
        }
    }
}

pub(crate) struct GuideFrame<'a> {
    pub(crate) document: &'a DocumentSnapshot,
    pub(crate) display: &'a DisplayMap,
    pub(crate) rows: &'a [Row],
    pub(crate) model: &'a BracketModel,
    pub(crate) primary: usize,
    pub(crate) tab_size: u32,
    pub(crate) appearance: &'a EditorAppearance,
    pub(crate) options: &'a EditorDisplayOptions,
}

fn horizontal(painter: &Painter, start: f32, end: f32, y: f32, color: Color32) -> bool {
    if end <= start {
        return false;
    }
    painter.rect_filled(
        Rect::from_min_max(pos2(start, y), pos2(end, y + GUIDE_WIDTH)),
        0.0,
        color,
    );
    true
}

fn vertical(painter: &Painter, row: &Row, x: f32, height: f32, color: Color32) {
    painter.rect_filled(
        Rect::from_min_size(pos2(x, row.origin.y), vec2(GUIDE_WIDTH, height)),
        0.0,
        color,
    );
}

fn shown(frame: &GuideFrame<'_>, line: usize) -> bool {
    frame.display.hidden_lines_at(line).is_none()
}

fn pair_row(
    painter: &Painter,
    frame: &GuideFrame<'_>,
    row: &Row,
    pair: &BracketPairInfo,
    active: bool,
    space_width: f32,
    colors: &EditorBracketColors,
) -> (bool, Option<usize>) {
    let Some(close) = pair.close.as_ref() else {
        return (false, None);
    };
    if row.segment.line < pair.open_line || row.segment.line > pair.close_line {
        return (false, None);
    }
    let color = colors.palette[pair.guide_level % colors.palette.len()];
    let color = if active {
        color
    } else {
        color.gamma_multiply(INACTIVE_GUIDE_OPACITY)
    };
    if pair.open_line == pair.close_line {
        if !active
            || !shown(frame, pair.open_line)
            || row.segment.bytes.end < pair.open.end
            || row.segment.bytes.start > close.start
        {
            return (false, None);
        }
        let start = row
            .caret_rect(pair.open.end.max(row.segment.bytes.start))
            .left();
        let end = row
            .caret_rect(close.start.min(row.segment.bytes.end))
            .left();
        return (
            horizontal(
                painter,
                start,
                end,
                row.origin.y + frame.appearance.line_height - GUIDE_WIDTH,
                color,
            ),
            None,
        );
    }
    let open_row = frame.display.row_of_byte(frame.document, pair.open.start);
    let close_row = frame.display.row_of_byte(frame.document, close.start);
    let x = row.origin.x + pair.guide_column as f32 * space_width;
    let mut any = false;
    let is_vertical = row.index > open_row
        && (row.index < close_row || pair.has_text_before_close && row.index == close_row);
    if is_vertical {
        vertical(painter, row, x, frame.appearance.line_height, color);
        any = true;
    }
    if active
        && row.index == open_row
        && shown(frame, pair.open_line)
        && pair.open_column > pair.guide_column
    {
        any |= horizontal(
            painter,
            x,
            row.caret_rect(pair.open.start).left(),
            row.origin.y + frame.appearance.line_height - GUIDE_WIDTH,
            color,
        );
    }
    if active
        && row.index == close_row
        && shown(frame, pair.close_line)
        && pair.close_column > pair.guide_column
    {
        let y = row.origin.y
            + if pair.has_text_before_close {
                frame.appearance.line_height - GUIDE_WIDTH
            } else {
                0.0
            };
        any |= horizontal(painter, x, row.caret_rect(close.start).left(), y, color);
    }
    (any, is_vertical.then_some(pair.guide_column))
}

pub(crate) fn paint_guides(painter: &Painter, frame: GuideFrame<'_>) {
    let Some(colors) = frame.options.bracket_colors.as_ref() else {
        return;
    };
    let Some(first) = frame.rows.first() else {
        return;
    };
    let Some(last) = frame.rows.last() else {
        return;
    };
    let active_pair = frame.model.active_pair(frame.primary);
    let active_indent = frame.model.active_indent(
        frame.document.rope.byte_to_line(frame.primary),
        first.segment.line..last.segment.line + 1,
    );
    let space_width = painter
        .layout_no_wrap(
            " ".into(),
            frame.appearance.font.clone(),
            frame.appearance.foreground,
        )
        .size()
        .x;
    let pairs = if frame.options.bracket_pair_guides {
        frame
            .model
            .pairs_in(first.segment.bytes.start..last.segment.bytes.end)
    } else {
        Vec::new()
    };
    for row in frame.rows {
        let mut occupied = Vec::new();
        let mut any_bracket = false;
        for pair in &pairs {
            let active = active_pair
                .is_some_and(|active| active.open == pair.open && active.end == pair.end);
            let (painted, column) =
                pair_row(painter, &frame, row, pair, active, space_width, colors);
            any_bracket |= painted;
            occupied.extend(column);
        }
        let line_start = frame.document.rope.line_to_byte(row.segment.line);
        if row.segment.bytes.start > line_start && row.segment.indent_columns == 0 {
            continue;
        }
        for level in 1..=frame.model.indent_level(row.segment.line) {
            let column = (level - 1) * frame.tab_size.max(1) as usize;
            if occupied.contains(&column) {
                continue;
            }
            let active = !any_bracket
                && active_indent.lines.start <= row.segment.line
                && row.segment.line <= active_indent.lines.end
                && level == active_indent.level;
            vertical(
                painter,
                row,
                row.origin.x + column as f32 * space_width,
                frame.appearance.line_height,
                if active {
                    colors.active_indent
                } else {
                    colors.indent
                },
            );
        }
    }
}

use egui::epaint::Mesh;
use egui::{Color32, Painter, Rect, Shape, pos2, vec2};
use taide_native_editor::document::DocumentSnapshot;
use taide_native_editor::view::SelectionSet;

use crate::editor_geometry::Row;
use crate::editor_surface::{EditorAppearance, EditorDisplayOptions, RenderWhitespace};

const WHITESPACE_RADIUS_DIVISOR: f32 = 7.0;
const CENTER_DIVISOR: f32 = 2.0;
const ARROW_SHAFT_FRACTION: f32 = 0.8;
const ARROW_INNER_FRACTION: f32 = 0.2;
const ARROW_OUTER_FRACTION: f32 = 0.1;
const ARROW_HEAD_FRACTION: f32 = 0.35;
const ARROW_TRIANGLES: [[u32; 3]; 8] = [
    [0, 1, 8],
    [0, 8, 9],
    [1, 2, 7],
    [1, 7, 8],
    [2, 3, 6],
    [2, 6, 7],
    [3, 4, 5],
    [3, 5, 6],
];
const RULER_WIDTH: f32 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EditorDisplayColors {
    pub whitespace: Color32,
    pub ruler: Color32,
    pub scrollbar: Color32,
    pub scrollbar_hover: Color32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WhitespaceKind {
    Space,
    Tab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WhitespaceMark {
    byte: usize,
    kind: WhitespaceKind,
}

fn whitespace_marks(
    source: &str,
    start: usize,
    wraps: bool,
    selections: &SelectionSet,
    mode: RenderWhitespace,
) -> Vec<WhitespaceMark> {
    if mode == RenderWhitespace::None {
        return Vec::new();
    }
    let is_whitespace = |byte: u8| matches!(byte, b' ' | b'\t');
    let bytes = source.as_bytes();
    let first = bytes.iter().position(|byte| !is_whitespace(*byte));
    let last = bytes.iter().rposition(|byte| !is_whitespace(*byte));
    source
        .char_indices()
        .filter_map(|(offset, character)| {
            let kind = match character {
                ' ' => WhitespaceKind::Space,
                '\t' => WhitespaceKind::Tab,
                _ => return None,
            };
            if mode == RenderWhitespace::Selection
                && !selections.selections.iter().any(|selection| {
                    selection.anchor.min(selection.head) <= start + offset
                        && start + offset < selection.anchor.max(selection.head)
                })
            {
                return None;
            }
            if mode == RenderWhitespace::Boundary && kind == WhitespaceKind::Space {
                let previous = offset.checked_sub(1).and_then(|index| bytes.get(index));
                let next = bytes.get(offset + 1);
                if first
                    .zip(last)
                    .is_some_and(|(first, last)| first <= offset && offset <= last)
                    && previous != Some(&b' ')
                    && next != Some(&b' ')
                {
                    return None;
                }
                if wraps
                    && offset + 1 == bytes.len()
                    && !previous.is_some_and(|byte| is_whitespace(*byte))
                {
                    return None;
                }
            }
            Some(WhitespaceMark {
                byte: start + offset,
                kind,
            })
        })
        .collect()
}

fn arrow(left: f32, center: f32, width: f32, color: Color32) -> Shape {
    let shaft = width * ARROW_SHAFT_FRACTION;
    let half_stroke = width / WHITESPACE_RADIUS_DIVISOR / CENTER_DIVISOR;
    let inner = pos2(
        shaft * (1.0 - ARROW_INNER_FRACTION),
        half_stroke + ARROW_INNER_FRACTION * shaft,
    );
    let outer = inner + vec2(ARROW_OUTER_FRACTION * shaft, ARROW_OUTER_FRACTION * shaft);
    let tip = outer + vec2(ARROW_HEAD_FRACTION * shaft, -ARROW_HEAD_FRACTION * shaft);
    let upper = [
        pos2(0.0, half_stroke),
        pos2(shaft, half_stroke),
        inner,
        outer,
        tip,
    ];
    let mut mesh = Mesh::default();
    for point in upper
        .into_iter()
        .chain(upper.into_iter().rev().map(|point| pos2(point.x, -point.y)))
    {
        mesh.colored_vertex(pos2(left + point.x, center + point.y), color);
    }
    for [a, b, c] in ARROW_TRIANGLES {
        mesh.add_triangle(a, b, c);
    }
    Shape::mesh(mesh)
}

pub(crate) fn whitespace(
    painter: &Painter,
    row: &Row,
    document: &DocumentSnapshot,
    selections: &SelectionSet,
    appearance: &EditorAppearance,
    options: &EditorDisplayOptions,
) {
    if options.render_whitespace == RenderWhitespace::None {
        return;
    }
    let source = document
        .rope
        .byte_slice(row.segment.bytes.clone())
        .to_string();
    let marks = whitespace_marks(
        &source,
        row.segment.bytes.start,
        row.wraps,
        selections,
        options.render_whitespace,
    );
    if marks.is_empty() {
        return;
    }
    let width = painter
        .layout_no_wrap(" ".into(), appearance.font.clone(), appearance.foreground)
        .size()
        .x;
    let color = options
        .colors
        .map_or(appearance.muted, |colors| colors.whitespace);
    let center = row.origin.y + appearance.line_height / CENTER_DIVISOR;
    for mark in marks {
        let left = row.caret_rect(mark.byte).left();
        match mark.kind {
            WhitespaceKind::Space => {
                painter.circle_filled(
                    pos2(left + width / CENTER_DIVISOR, center),
                    width / WHITESPACE_RADIUS_DIVISOR,
                    color,
                );
            }
            WhitespaceKind::Tab => {
                painter.add(arrow(left, center, width, color));
            }
        }
    }
}

pub(crate) fn rulers(
    painter: &Painter,
    rect: Rect,
    scroll: egui::Vec2,
    content_height: f32,
    appearance: &EditorAppearance,
    options: &EditorDisplayOptions,
) {
    if options.rulers.is_empty() {
        return;
    }
    let width = painter
        .layout_no_wrap("n".into(), appearance.font.clone(), appearance.foreground)
        .size()
        .x;
    let color = options
        .colors
        .map_or(appearance.muted, |colors| colors.ruler);
    let top = rect.top() - scroll.y;
    let bottom = top + content_height;
    for column in &options.rulers {
        let left = rect.left() - scroll.x + *column as f32 * width;
        painter.rect_filled(
            Rect::from_min_max(pos2(left, top), pos2(left + RULER_WIDTH, bottom)),
            0.0,
            color,
        );
    }
}

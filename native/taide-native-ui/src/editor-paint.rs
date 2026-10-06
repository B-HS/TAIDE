use std::borrow::Cow;
use std::iter::once;
use std::ops::{Range, RangeInclusive};

use egui::{Align2, Color32, Painter, Rangef, Rect, Shape, Stroke, pos2, vec2};
use taide_native_editor::decoration::{DecorationKind, DecorationLayer, UnderlineKind};
use taide_native_editor::document::DocumentSnapshot;
use taide_native_editor::view::{Selection, SelectionSet};

use crate::editor_geometry::{CURSOR_STROKE, FOLD_PLACEHOLDER, Row};
use crate::editor_gutter::Gutter;
use crate::editor_surface::EditorAppearance;

const SCROLLBAR_IDLE_OPACITY: f32 = 0.4;
const SCROLLBAR_ENGAGED_OPACITY: f32 = 0.7;
const FOLD_BACKGROUND_OPACITY: f32 = 0.3;
const FOLD_PLACEHOLDER_COLOR: Color32 = Color32::from_gray(128);
const SQUIGGLE_PERIOD: f32 = 6.0;
const SQUIGGLE_HEIGHT: f32 = 3.0;
const SQUIGGLE_TROUGH_OFFSET: f32 = 1.75;
const SQUIGGLE_CREST_OFFSET: f32 = 4.75;
const SQUIGGLE_STROKE: f32 = 1.0;

pub(crate) fn color32([red, green, blue, alpha]: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(red, green, blue, alpha)
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FrameDecoration {
    pub(crate) z_order: u8,
    pub(crate) bytes: Range<usize>,
    pub(crate) lines: RangeInclusive<usize>,
    pub(crate) kind: DecorationKind,
}

pub(crate) fn frame_decorations(
    document: &DocumentSnapshot,
    layers: &[Cow<'_, DecorationLayer>],
    lines: Range<usize>,
) -> Vec<FrameDecoration> {
    if lines.is_empty() {
        return Vec::new();
    }
    let rope = &document.rope;
    let boundary = |byte: usize| rope.char_to_byte(rope.byte_to_char(byte.min(rope.len_bytes())));
    let shown = rope.line_to_byte(lines.start)..rope.line_to_byte(lines.end);
    layers
        .iter()
        .flat_map(|layer| {
            layer
                .intersecting(shown.clone())
                .map(|item| (layer.z_order(), item))
        })
        .map(|(z_order, item)| {
            let bytes = boundary(item.bytes.start)..boundary(item.bytes.end);
            FrameDecoration {
                z_order,
                lines: rope.byte_to_line(bytes.start)..=rope.byte_to_line(bytes.end),
                bytes,
                kind: item.kind,
            }
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq)]
struct RangeOverlay {
    background: Option<[u8; 4]>,
    squiggle: Option<[u8; 4]>,
}

pub(crate) struct Carets<'a> {
    pub(crate) selections: &'a SelectionSet,
    pub(crate) head_rows: &'a [usize],
    pub(crate) primary_row: usize,
    pub(crate) primary_line: usize,
    pub(crate) focused: bool,
}

pub(crate) struct Layers<'a> {
    pub(crate) painter: &'a Painter,
    pub(crate) text_painter: &'a Painter,
    pub(crate) rect: Rect,
    pub(crate) text_rect: Rect,
    pub(crate) gutter: Gutter,
    pub(crate) appearance: &'a EditorAppearance,
    pub(crate) decorations: &'a [FrameDecoration],
}

impl Layers<'_> {
    pub(crate) fn background(&self) {
        self.painter
            .rect_filled(self.rect, 0.0, self.appearance.background);
    }

    pub(crate) fn row(&self, row: &Row, carets: &Carets<'_>) {
        if row.segment.line == carets.primary_line {
            self.current_line(row);
        }
        for selection in &carets.selections.selections {
            self.selection(row, selection);
        }
        if row.heads_hidden_lines {
            self.folded_background(row);
        }
        self.line_backgrounds(row);
        self.range_overlays(row);
        for (selection, head_row) in carets.selections.selections.iter().zip(carets.head_rows) {
            if carets.focused && selection.anchor == selection.head && *head_row == row.index {
                self.caret(row, selection.head);
            }
        }
        self.text(row);
        if row.segment.ends_folded {
            self.fold_placeholder(row);
        }
        self.gutter.paint_row(
            self.painter,
            self.rect.left(),
            row,
            self.appearance,
            self.decorations,
        );
    }

    fn row_band(&self, row: &Row) -> Rangef {
        Rangef::new(row.origin.y, row.origin.y + self.appearance.line_height)
    }

    fn current_line(&self, row: &Row) {
        self.painter.rect_filled(
            Rect::from_min_size(
                pos2(self.rect.left(), row.origin.y),
                vec2(self.rect.width(), self.appearance.line_height),
            ),
            0.0,
            self.appearance.current_line,
        );
    }

    fn selection(&self, row: &Row, selection: &Selection) {
        let start = selection.anchor.min(selection.head);
        let end = selection.anchor.max(selection.head);
        let bytes = &row.segment.bytes;
        let starts_within = if row.wraps {
            start < bytes.end
        } else {
            start <= bytes.end
        };
        if start < end && starts_within && end > bytes.start {
            let left = row.caret_rect(start.max(bytes.start));
            let right = row.caret_rect(end.min(bytes.end));
            let right_x = if end > bytes.end && !row.wraps {
                self.text_rect.right().max(right.left())
            } else {
                right.left()
            };
            self.text_painter.rect_filled(
                Rect::from_min_max(
                    pos2(left.left(), row.origin.y),
                    pos2(right_x, row.origin.y + self.appearance.line_height),
                ),
                0.0,
                self.appearance.selection,
            );
        }
    }

    fn folded_background(&self, row: &Row) {
        self.text_painter.rect_filled(
            Rect::from_x_y_ranges(self.text_rect.x_range(), self.row_band(row)),
            0.0,
            self.appearance
                .selection
                .gamma_multiply(FOLD_BACKGROUND_OPACITY),
        );
    }

    fn fold_placeholder(&self, row: &Row) {
        self.text_painter.text(
            pos2(
                row.fold_placeholder_left(self.appearance),
                row.text_origin().y,
            ),
            Align2::LEFT_TOP,
            FOLD_PLACEHOLDER,
            self.appearance.font.clone(),
            FOLD_PLACEHOLDER_COLOR,
        );
    }

    fn line_backgrounds(&self, row: &Row) {
        for decoration in self.decorations {
            let DecorationKind::LineBackground(color) = decoration.kind else {
                continue;
            };
            if decoration.lines.contains(&row.segment.line) {
                self.text_painter.rect_filled(
                    Rect::from_x_y_ranges(self.text_rect.x_range(), self.row_band(row)),
                    0.0,
                    color32(color),
                );
            }
        }
    }

    fn range_overlays(&self, row: &Row) {
        let mut groups: Vec<(u8, RangeOverlay, Vec<Rangef>)> = Vec::new();
        for decoration in self.decorations {
            let DecorationKind::Inline(style) = decoration.kind else {
                continue;
            };
            let overlay = RangeOverlay {
                background: style.background,
                squiggle: style
                    .underline
                    .filter(|underline| underline.kind == UnderlineKind::Squiggly)
                    .map(|underline| underline.color),
            };
            if overlay.background.is_none() && overlay.squiggle.is_none() {
                continue;
            }
            let Some(extent) = row.extent(&decoration.bytes) else {
                continue;
            };
            let group = groups.iter_mut().find(|(z_order, grouped, _)| {
                *z_order == decoration.z_order && *grouped == overlay
            });
            match group {
                Some((_, _, extents)) => extents.push(extent),
                None => groups.push((decoration.z_order, overlay, vec![extent])),
            }
        }
        for (_, overlay, mut extents) in groups {
            extents.sort_by(|left, right| left.min.total_cmp(&right.min));
            let mut merged: Option<Rangef> = None;
            for extent in extents {
                match &mut merged {
                    Some(current) if extent.min <= current.max => {
                        current.max = current.max.max(extent.max);
                    }
                    _ => {
                        if let Some(finished) = merged.replace(extent) {
                            self.range_overlay(row, overlay, finished);
                        }
                    }
                }
            }
            if let Some(finished) = merged {
                self.range_overlay(row, overlay, finished);
            }
        }
    }

    fn range_overlay(&self, row: &Row, overlay: RangeOverlay, extent: Rangef) {
        let band = self.row_band(row);
        if let Some(color) = overlay.background {
            self.text_painter
                .rect_filled(Rect::from_x_y_ranges(extent, band), 0.0, color32(color));
        }
        if let Some(color) = overlay.squiggle {
            self.squiggle(extent, band.max, color32(color));
        }
    }

    fn squiggle(&self, extent: Rangef, bottom: f32, color: Color32) {
        let top = bottom - SQUIGGLE_HEIGHT;
        let periods = (extent.span() / SQUIGGLE_PERIOD).ceil() as usize + 1;
        let points = once(pos2(
            extent.min + SQUIGGLE_CREST_OFFSET - SQUIGGLE_PERIOD,
            top,
        ))
        .chain((0..periods).flat_map(|period| {
            let start = extent.min + period as f32 * SQUIGGLE_PERIOD;
            [
                pos2(start + SQUIGGLE_TROUGH_OFFSET, bottom),
                pos2(start + SQUIGGLE_CREST_OFFSET, top),
            ]
        }))
        .collect();
        self.text_painter
            .with_clip_rect(Rect::from_x_y_ranges(extent, Rangef::new(top, bottom)))
            .add(Shape::line(points, Stroke::new(SQUIGGLE_STROKE, color)));
    }

    fn caret(&self, row: &Row, head: usize) {
        let cursor = row.caret_rect(head.min(row.segment.bytes.end));
        self.text_painter.line_segment(
            [
                pos2(cursor.left(), row.origin.y),
                pos2(cursor.left(), row.origin.y + self.appearance.line_height),
            ],
            Stroke::new(CURSOR_STROKE, self.appearance.cursor),
        );
    }

    fn text(&self, row: &Row) {
        self.text_painter.galley(
            row.text_origin(),
            row.galley.clone(),
            self.appearance.foreground,
        );
    }

    pub(crate) fn composition(&self, caret: Rect, preedit: &str) {
        let galley = self.text_painter.layout_no_wrap(
            preedit.to_owned(),
            self.appearance.font.clone(),
            self.appearance.foreground,
        );
        let preedit_rect = Rect::from_min_size(caret.min, galley.size());
        self.text_painter
            .rect_filled(preedit_rect, 0.0, self.appearance.background);
        self.text_painter
            .galley(caret.min, galley, self.appearance.foreground);
        self.text_painter.line_segment(
            [preedit_rect.left_bottom(), preedit_rect.right_bottom()],
            Stroke::new(CURSOR_STROKE, self.appearance.cursor),
        );
    }

    pub(crate) fn scrollbar(&self, slider: Rect, engaged: bool, opacity: f32) {
        let strength = if engaged {
            SCROLLBAR_ENGAGED_OPACITY
        } else {
            SCROLLBAR_IDLE_OPACITY
        };
        self.painter.rect_filled(
            slider,
            0.0,
            self.appearance.muted.gamma_multiply(strength * opacity),
        );
    }
}

use egui::{Align2, Painter, Rangef, Rect, Shape, Stroke, Vec2, pos2, vec2};
use taide_native_editor::decoration::{DecorationKind, LaneMark};

use crate::css_motion::ease;
use crate::editor_geometry::{FALLBACK_CHARACTER_WIDTH, Row, UNREADABLE_CHARACTER_WIDTH};
use crate::editor_paint::{FrameDecoration, color32};
use crate::editor_surface::EditorAppearance;

const DIGITS: &str = "0123456789";
const LINE_NUMBERS_MIN_CHARS: usize = 3;
const LINE_DECORATIONS_WIDTH: f32 = 10.0;
const FOLDING_CONTROLS_WIDTH: f32 = 16.0;
const LANE_BAR_WIDTH: f32 = 3.0;
const DELETED_TRIANGLE_WIDTH: f32 = 6.0;
const DELETED_TRIANGLE_HALF_HEIGHT: f32 = 4.0;
const FOLD_CONTROL_MARGIN: f32 = 2.0;
const FOLD_CONTROL_CLICK_INSET: f32 = 4.0;
const FOLD_CONTROL_FONT_SCALE: f32 = 1.4;
const FOLD_CONTROL_FADE_SECONDS: f64 = 0.5;
const CENTER_DIVISOR: f32 = 2.0;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct FoldControlFade {
    start: f32,
    end: f32,
    reversing_start: f32,
    shortening: f32,
    started: f64,
    duration: f64,
}

impl FoldControlFade {
    fn progress(&self, time: f64) -> f32 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        ((time - self.started) / self.duration).clamp(0.0, 1.0) as f32
    }

    fn opacity(&self, time: f64) -> f32 {
        let eased = ease(self.progress(time));
        self.start * (1.0 - eased) + self.end * eased
    }

    pub(crate) fn sample(&mut self, is_shown: bool, time: f64) -> (f32, bool) {
        let target = f32::from(u8::from(is_shown));
        if self.end != target {
            let progress = self.progress(time);
            let current = self.opacity(time);
            if progress < 1.0 && target == self.reversing_start {
                self.shortening = (ease(progress) * self.shortening + 1.0 - self.shortening)
                    .abs()
                    .clamp(0.0, 1.0);
                self.reversing_start = self.end;
            } else {
                self.shortening = 1.0;
                self.reversing_start = current;
            }
            self.start = current;
            self.end = target;
            self.started = time;
            self.duration = FOLD_CONTROL_FADE_SECONDS * f64::from(self.shortening);
        }
        (self.opacity(time), self.progress(time) < 1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Gutter {
    line_numbers_width: f32,
    decorations_width: f32,
}

impl Gutter {
    pub(crate) fn measure(
        painter: &Painter,
        line_count: usize,
        appearance: &EditorAppearance,
        has_folding: bool,
    ) -> Self {
        let line_numbers_width = if appearance.line_numbers {
            let digits = line_count.to_string().len().max(LINE_NUMBERS_MIN_CHARS);
            (digits as f32 * max_digit_width(painter, appearance)).round()
        } else {
            0.0
        };
        let folding_controls_width = if has_folding {
            FOLDING_CONTROLS_WIDTH
        } else {
            0.0
        };
        Self {
            line_numbers_width,
            decorations_width: LINE_DECORATIONS_WIDTH + folding_controls_width,
        }
    }

    pub(crate) fn width(&self) -> f32 {
        self.line_numbers_width + self.decorations_width
    }

    pub(crate) fn is_line_number(&self, left: f32, x: f32) -> bool {
        left <= x && x < left + self.line_numbers_width
    }

    pub(crate) fn fold_click_zone(&self, left: f32) -> Rangef {
        Rangef::new(
            left + self.line_numbers_width + FOLD_CONTROL_MARGIN + FOLD_CONTROL_CLICK_INSET,
            left + self.width(),
        )
    }

    pub(crate) fn fold_control_rect(
        &self,
        left: f32,
        row: &Row,
        appearance: &EditorAppearance,
    ) -> Rect {
        Rect::from_center_size(
            pos2(
                left + self.line_numbers_width
                    + FOLD_CONTROL_MARGIN
                    + self.decorations_width / CENTER_DIVISOR,
                row.origin.y + appearance.line_height / CENTER_DIVISOR,
            ),
            Vec2::splat(appearance.font.size * FOLD_CONTROL_FONT_SCALE),
        )
    }

    pub(crate) fn paint_row(
        &self,
        painter: &Painter,
        left: f32,
        row: &Row,
        appearance: &EditorAppearance,
        decorations: &[FrameDecoration],
    ) {
        let lane_left = left + self.line_numbers_width;
        let top = row.origin.y;
        let mut painted = Vec::new();
        for decoration in decorations {
            let DecorationKind::Lane { mark, color } = decoration.kind else {
                continue;
            };
            if !decoration.lines.contains(&row.segment.line) || painted.contains(&(mark, color)) {
                continue;
            }
            painted.push((mark, color));
            let shape = match mark {
                LaneMark::Bar => Shape::rect_filled(
                    Rect::from_min_size(
                        pos2(lane_left, top),
                        vec2(LANE_BAR_WIDTH, appearance.line_height),
                    ),
                    0.0,
                    color32(color),
                ),
                LaneMark::DeletedTriangle => Shape::convex_polygon(
                    vec![
                        pos2(lane_left, top - DELETED_TRIANGLE_HALF_HEIGHT),
                        pos2(lane_left + DELETED_TRIANGLE_WIDTH, top),
                        pos2(lane_left, top + DELETED_TRIANGLE_HALF_HEIGHT),
                    ],
                    color32(color),
                    Stroke::NONE,
                ),
            };
            painter.add(shape);
        }
        if appearance.line_numbers && !row.segment.is_continuation {
            painter.text(
                pos2(lane_left, row.text_origin().y),
                Align2::RIGHT_TOP,
                (row.segment.line + 1).to_string(),
                appearance.font.clone(),
                appearance.muted,
            );
        }
    }
}

fn max_digit_width(painter: &Painter, appearance: &EditorAppearance) -> f32 {
    let width = painter
        .layout_no_wrap(DIGITS.into(), appearance.font.clone(), appearance.muted)
        .rows
        .first()
        .map_or(0.0, |row| {
            row.row
                .glyphs
                .iter()
                .map(|glyph| glyph.advance_width)
                .fold(0.0, f32::max)
        });
    if width <= UNREADABLE_CHARACTER_WIDTH {
        width.max(FALLBACK_CHARACTER_WIDTH)
    } else {
        width
    }
}

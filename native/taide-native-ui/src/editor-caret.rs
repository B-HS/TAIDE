use std::time::Duration;

use egui::{Color32, Painter, Rect, Ui, Vec2, pos2, vec2};
use taide_native_editor::document::{DocumentId, DocumentSnapshot, EditorError};
use taide_native_editor::editing::grapheme_boundary;
use taide_native_editor::view::SelectionSet;

use crate::editor_geometry::Row;
use crate::editor_surface::{CursorBlinking, CursorStyle, EditorAppearance, EditorDisplayOptions};

const BLINK_INTERVAL: f64 = 0.5;
const BLINK_ITERATIONS: f64 = 20.0;
const ALTERNATE_PHASES: f64 = 2.0;
const SMOOTH_MOVE_TIME: f64 = 0.08;
const ANIMATION_PLATEAU: f32 = 0.2;
const SMOOTH_FADE_END: f32 = 0.6;
const PHASE_FADE_END: f32 = 0.9;
const EXPAND_FADE_END: f32 = 0.8;
const EASE_IN_OUT_X: [f64; 2] = [0.42, 0.58];
const EASE_IN_OUT_Y: [f64; 2] = [0.0, 1.0];
const LINE_WIDTH: f32 = 2.0;
const THIN_LINE_WIDTH: f32 = 1.0;
const LINE_LEFT_SHIFT: f32 = 1.0;
const UNDERLINE_HEIGHT: f32 = 2.0;
const CENTER_DIVISOR: f32 = 2.0;

#[derive(Clone, PartialEq)]
struct Signature {
    document: DocumentId,
    revision: u64,
    selections: SelectionSet,
    style: CursorStyle,
    blinking: CursorBlinking,
    focused: bool,
    read_only: bool,
    composing: bool,
}

#[derive(Clone)]
struct Motion {
    from: Rect,
    to: Rect,
    started: f64,
}

impl Motion {
    fn sample(&self, time: f64) -> Rect {
        let progress = crate::css_motion::ease(((time - self.started) / SMOOTH_MOVE_TIME) as f32);
        Rect::from_min_max(
            self.from.min + (self.to.min - self.from.min) * progress,
            self.from.max + (self.to.max - self.from.max) * progress,
        )
    }

    fn pending(&self, time: f64) -> bool {
        self.from != self.to && time < self.started + SMOOTH_MOVE_TIME
    }
}

#[derive(Clone, Default)]
pub(crate) struct CaretState {
    signature: Option<Signature>,
    started: f64,
    motions: Vec<Option<Motion>>,
}

fn blink(blinking: CursorBlinking, elapsed: f64) -> (f32, f32, Option<Duration>) {
    if blinking == CursorBlinking::Solid {
        return (1.0, 1.0, None);
    }
    if blinking == CursorBlinking::Blink {
        let phase = elapsed.max(0.0) / BLINK_INTERVAL;
        let opacity = if phase.floor() % ALTERNATE_PHASES == 0.0 {
            1.0
        } else {
            0.0
        };
        return (
            opacity,
            1.0,
            Some(Duration::from_secs_f64(
                (1.0 - phase.fract()) * BLINK_INTERVAL,
            )),
        );
    }
    if elapsed < BLINK_INTERVAL {
        return (
            1.0,
            1.0,
            Some(Duration::from_secs_f64(BLINK_INTERVAL - elapsed.max(0.0))),
        );
    }
    let phase = (elapsed - BLINK_INTERVAL) / BLINK_INTERVAL;
    if phase >= BLINK_ITERATIONS {
        return (1.0, 1.0, None);
    }
    let progress = if phase.floor() % ALTERNATE_PHASES == 0.0 {
        phase.fract()
    } else {
        1.0 - phase.fract()
    } as f32;
    let end = match blinking {
        CursorBlinking::Smooth => SMOOTH_FADE_END,
        CursorBlinking::Phase => PHASE_FADE_END,
        CursorBlinking::Expand => EXPAND_FADE_END,
        _ => unreachable!(),
    };
    let fade = crate::css_motion::cubic_bezier(
        (progress - ANIMATION_PLATEAU) / (end - ANIMATION_PLATEAU),
        EASE_IN_OUT_X,
        EASE_IN_OUT_Y,
    );
    let value = 1.0 - fade;
    if blinking == CursorBlinking::Expand {
        return (1.0, value, Some(Duration::ZERO));
    }
    (value, 1.0, Some(Duration::ZERO))
}

fn target(
    painter: &Painter,
    row: &Row,
    document: &DocumentSnapshot,
    head: usize,
    appearance: &EditorAppearance,
    style: CursorStyle,
) -> Result<Rect, EditorError> {
    let head = head.min(row.segment.bytes.end);
    let next = if head < row.segment.bytes.end {
        grapheme_boundary(document, head, true)?.min(row.segment.bytes.end)
    } else {
        head
    };
    let start = if next > head {
        grapheme_boundary(document, next, false)?.max(row.segment.bytes.start)
    } else {
        head
    };
    let left = row.caret_rect(start).left();
    if matches!(style, CursorStyle::Line | CursorStyle::LineThin) {
        let desired = if style == CursorStyle::Line {
            LINE_WIDTH
        } else {
            THIN_LINE_WIDTH
        };
        let dpr = painter.pixels_per_point();
        let width = (desired * dpr).floor().max(1.0) / dpr;
        let local_left = row.caret(start).left();
        let left = if width >= LINE_WIDTH && local_left >= LINE_LEFT_SHIFT {
            left - LINE_LEFT_SHIFT
        } else {
            left
        };
        return Ok(Rect::from_min_size(
            pos2(left, row.origin.y),
            vec2(width, appearance.line_height),
        ));
    }
    let tab = start < next && document.rope.byte_slice(start..next) == "\t";
    let mut width = row.caret_rect(next).left() - left;
    if tab || width < THIN_LINE_WIDTH {
        width = painter
            .layout_no_wrap("n".into(), appearance.font.clone(), appearance.foreground)
            .size()
            .x;
    }
    let (top, height) = if style == CursorStyle::Underline {
        (
            row.origin.y + appearance.line_height - UNDERLINE_HEIGHT,
            UNDERLINE_HEIGHT,
        )
    } else {
        (row.origin.y, appearance.line_height)
    };
    Ok(Rect::from_min_size(pos2(left, top), vec2(width, height)))
}

pub(crate) struct CaretFrame<'a> {
    pub(crate) document: &'a DocumentSnapshot,
    pub(crate) selections: &'a SelectionSet,
    pub(crate) head_rows: &'a [usize],
    pub(crate) rows: &'a [Row],
    pub(crate) appearance: &'a EditorAppearance,
    pub(crate) options: &'a EditorDisplayOptions,
    pub(crate) scroll: Vec2,
    pub(crate) origin: egui::Pos2,
    pub(crate) focused: bool,
    pub(crate) composing: bool,
}

impl CaretState {
    pub(crate) fn paint(
        &mut self,
        ui: &Ui,
        painter: &Painter,
        frame: CaretFrame<'_>,
    ) -> Result<(), EditorError> {
        let time = ui.input(|input| input.time);
        let signature = Signature {
            document: frame.document.id,
            revision: frame.document.revision,
            selections: frame.selections.clone(),
            style: frame.options.cursor_style,
            blinking: frame.options.cursor_blinking,
            focused: frame.focused,
            read_only: frame.document.metadata.read_only,
            composing: frame.composing,
        };
        let reset = self.signature.as_ref() != Some(&signature);
        let snap = !frame.options.smooth_caret
            || self.signature.as_ref().is_none_or(|previous| {
                previous.document != signature.document
                    || !previous.focused
                    || previous.composing
                    || previous.style != signature.style
                    || previous.selections.selections.len() != signature.selections.selections.len()
            });
        if reset {
            self.started = time;
        }
        self.signature = Some(signature);
        self.motions.resize(frame.selections.selections.len(), None);
        let translation = frame.origin.to_vec2() - frame.scroll;
        let mut targets = Vec::new();
        for (index, (selection, head_row)) in frame
            .selections
            .selections
            .iter()
            .zip(frame.head_rows)
            .enumerate()
        {
            let Some(row) = frame.rows.iter().find(|row| row.index == *head_row) else {
                self.motions[index] = None;
                continue;
            };
            let target = target(
                painter,
                row,
                frame.document,
                selection.head,
                frame.appearance,
                frame.options.cursor_style,
            )?;
            let local = target.translate(-translation);
            let from = self.motions[index]
                .as_ref()
                .map_or(local, |motion| motion.sample(time));
            if snap
                || self.motions[index]
                    .as_ref()
                    .is_none_or(|motion| motion.to != local)
            {
                self.motions[index] = Some(Motion {
                    from: if snap { local } else { from },
                    to: local,
                    started: time,
                });
            }
            targets.push((index, row, target));
        }
        if !frame.focused || frame.composing {
            return Ok(());
        }
        let blinking = if frame.document.metadata.read_only {
            CursorBlinking::Solid
        } else {
            frame.options.cursor_blinking
        };
        let (opacity, scale, repaint) = blink(blinking, time - self.started);
        if let Some(delay) = repaint {
            ui.ctx().request_repaint_after(delay);
        }
        for (index, row, target) in targets {
            let motion = self.motions[index].as_ref().unwrap();
            if motion.pending(time) {
                ui.ctx().request_repaint();
            }
            if opacity <= 0.0 || scale <= 0.0 {
                continue;
            }
            let animated = motion.sample(time).translate(translation);
            let half_height = animated.height() * scale / CENTER_DIVISOR;
            let rect = Rect::from_x_y_ranges(
                animated.x_range(),
                (animated.center().y - half_height)..=(animated.center().y + half_height),
            );
            painter.rect_filled(rect, 0.0, frame.appearance.cursor.gamma_multiply(opacity));
            if frame.options.cursor_style == CursorStyle::Block {
                let [r, g, b, a] = frame.appearance.cursor.to_srgba_unmultiplied();
                let text_color =
                    Color32::from_rgba_unmultiplied(u8::MAX - r, u8::MAX - g, u8::MAX - b, a)
                        .gamma_multiply(opacity);
                painter
                    .with_clip_rect(rect)
                    .galley_with_override_text_color(
                        row.text_origin() + (animated.min - target.min),
                        row.galley.clone(),
                        text_color,
                    );
            }
        }
        Ok(())
    }
}

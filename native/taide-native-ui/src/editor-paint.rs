use egui::{Align2, Painter, Rect, Stroke, pos2, vec2};
use taide_native_editor::view::{Selection, SelectionSet};

use crate::editor_geometry::{CURSOR_STROKE, Row};
use crate::editor_surface::EditorAppearance;

const SCROLLBAR_IDLE_OPACITY: f32 = 0.4;
const SCROLLBAR_ENGAGED_OPACITY: f32 = 0.7;

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
    pub(crate) gutter: f32,
    pub(crate) appearance: &'a EditorAppearance,
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
        for (selection, head_row) in carets.selections.selections.iter().zip(carets.head_rows) {
            self.selection(row, selection);
            if carets.focused && selection.anchor == selection.head && *head_row == row.index {
                self.caret(row, selection.head);
            }
        }
        self.text(row);
        if self.appearance.line_numbers && !row.segment.is_continuation {
            self.line_number(row);
        }
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

    fn line_number(&self, row: &Row) {
        self.painter.text(
            pos2(
                self.rect.left() + self.gutter - self.appearance.horizontal_padding,
                row.text_origin().y,
            ),
            Align2::RIGHT_TOP,
            (row.segment.line + 1).to_string(),
            self.appearance.font.clone(),
            self.appearance.muted,
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

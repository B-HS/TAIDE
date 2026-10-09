use std::collections::HashSet;

use egui::{Color32, FontId, Id, Rect, Sense, Stroke, Ui, pos2};
use taide_native_editor::diagnostics::{Marker, Message, Severity};

use crate::editor_geometry::Row;

const HINT_DOTS: usize = 3;
const HINT_DOT_SIZE: f32 = 1.0;
const HINT_DOT_SPACING: f32 = 2.0;
const HINT_BOTTOM_OFFSET: f32 = 1.5;
const HOVER_MAX_WIDTH: f32 = 500.0;
const HOVER_PADDING: f32 = 8.0;
const BORDER_WIDTH: f32 = 1.0;
const DETAIL_OPACITY: f32 = 0.6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticColors {
    pub error: Color32,
    pub warning: Color32,
    pub information: Color32,
    pub hint: Color32,
    pub background: Color32,
    pub foreground: Color32,
    pub border: Color32,
}

impl DiagnosticColors {
    pub(crate) fn severity(self, severity: Severity) -> Color32 {
        match severity {
            Severity::Error => self.error,
            Severity::Warning => self.warning,
            Severity::Information => self.information,
            Severity::Hint => self.hint,
        }
    }
}

pub(crate) fn row_rect(row: &Row, marker: &Marker, height: f32, empty_width: f32) -> Option<Rect> {
    let extent = row.extent(&marker.bytes).or_else(|| {
        (marker.bytes.is_empty()
            && (row.segment.bytes.contains(&marker.bytes.start)
                || marker.bytes.start == row.segment.bytes.end && !row.wraps))
            .then(|| {
                let x = row.caret_rect(marker.bytes.start).left();
                egui::Rangef::new(x, x + empty_width)
            })
    })?;
    Some(Rect::from_min_max(
        pos2(extent.min, row.origin.y),
        pos2(extent.max, row.origin.y + height),
    ))
}

pub(crate) struct DiagnosticFrame<'a> {
    pub(crate) id: Id,
    pub(crate) rows: &'a [Row],
    pub(crate) markers: &'a [Marker],
    pub(crate) rect: Rect,
    pub(crate) height: f32,
    pub(crate) font: &'a FontId,
    pub(crate) colors: DiagnosticColors,
    pub(crate) read_only: bool,
}

pub(crate) fn paint(ui: &Ui, frame: DiagnosticFrame<'_>) {
    let painter = ui.painter().with_clip_rect(frame.rect);
    let width = painter
        .layout_no_wrap("n".into(), frame.font.clone(), frame.colors.foreground)
        .size()
        .x;
    let mut markers = frame.markers.iter().collect::<Vec<_>>();
    markers.sort_by_key(|marker| std::cmp::Reverse(marker.message.severity));
    for marker in markers {
        for row in frame.rows {
            let Some(rect) = row_rect(row, marker, frame.height, width) else {
                continue;
            };
            if !rect.intersects(frame.rect) || frame.read_only {
                continue;
            }
            let color = frame.colors.severity(marker.message.severity);
            if marker.message.severity == Severity::Hint {
                for index in 0..HINT_DOTS {
                    let x = rect.left() + HINT_DOT_SIZE + index as f32 * HINT_DOT_SPACING;
                    if x <= rect.right() {
                        painter.circle_filled(
                            pos2(x, rect.bottom() - HINT_BOTTOM_OFFSET),
                            HINT_DOT_SIZE,
                            color,
                        );
                    }
                }
            } else {
                crate::editor_paint::squiggle(&painter, rect.x_range(), rect.bottom(), color);
            }
        }
    }
    if !ui.is_enabled() {
        return;
    }
    let pointer = ui.input(|input| input.pointer.hover_pos());
    let mut anchors = HashSet::new();
    for marker in frame.markers {
        for (index, row) in frame.rows.iter().enumerate() {
            let Some(anchor) = row_rect(row, marker, frame.height, width)
                .filter(|rect| rect.intersects(frame.rect))
            else {
                continue;
            };
            let anchor = anchor.intersect(frame.rect);
            let id = frame.id.with((
                "diagnostic-hover",
                marker.bytes.start,
                marker.bytes.end,
                index,
            ));
            if !anchors.insert(id) {
                continue;
            }
            let response = ui.interact(anchor, id, Sense::hover());
            egui::Tooltip::for_enabled(&response)
                .width(HOVER_MAX_WIDTH)
                .show(|ui| {
                    egui::Frame::NONE
                        .fill(frame.colors.background)
                        .stroke(Stroke::new(BORDER_WIDTH, frame.colors.border))
                        .inner_margin(HOVER_PADDING)
                        .show(ui, |ui| {
                            ui.style_mut().interaction.selectable_labels = true;
                            ui.set_max_width(HOVER_MAX_WIDTH);
                            let point = pointer
                                .filter(|point| anchor.contains(*point))
                                .unwrap_or_else(|| anchor.center());
                            for marker in frame.markers.iter().filter(|marker| {
                                row_rect(row, marker, frame.height, width)
                                    .is_some_and(|rect| rect.contains(point))
                            }) {
                                show_message(ui, &marker.message, frame.font, frame.colors);
                            }
                        });
                });
        }
    }
}

pub(crate) fn message_details(message: &Message) -> String {
    match (message.source.as_deref(), message.code.as_deref()) {
        (Some(source), Some(code)) => format!("{source}({code})"),
        (Some(source), None) => source.into(),
        (None, Some(code)) => format!("({code})"),
        (None, None) => String::new(),
    }
}

pub(crate) fn show_message(
    ui: &mut Ui,
    message: &Message,
    font: &FontId,
    colors: DiagnosticColors,
) {
    let details = message_details(message);
    ui.label(
        egui::RichText::new(&message.text)
            .font(font.clone())
            .color(colors.foreground),
    );
    if !details.is_empty() {
        ui.label(
            egui::RichText::new(details)
                .font(font.clone())
                .color(colors.foreground.gamma_multiply(DETAIL_OPACITY)),
        );
    }
}

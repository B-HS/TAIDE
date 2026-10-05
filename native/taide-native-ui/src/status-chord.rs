use egui::{Color32, FontId, Response, Stroke, Ui, vec2};
use taide_model::{error::AppResult, locale::ResolvedLocale, theme::ResolvedTheme};

use crate::presentation::{color, message};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ChordStatus {
    pub shortcut: Option<String>,
    pub no_match: bool,
}

const TEXT_SIZE: f32 = 11.0;
const ICON_SIZE: f32 = 12.0;
const ICON_VIEWBOX: f32 = 24.0;
const ICON_STROKE: f32 = 2.0;
const ICON_CORNER: f32 = 2.0;
const ICON_GAP: f32 = 4.0;
const KEYBOARD_RECT: [f32; 4] = [2.0, 4.0, 22.0, 20.0];
const KEYBOARD_KEYS: [[f32; 2]; 7] = [
    [10.0, 8.0],
    [12.0, 12.0],
    [14.0, 8.0],
    [16.0, 12.0],
    [18.0, 8.0],
    [6.0, 8.0],
    [8.0, 12.0],
];
const KEYBOARD_SPACE: [[f32; 2]; 2] = [[7.0, 16.0], [17.0, 16.0]];

pub struct Appearance {
    warning: Color32,
    error: Color32,
}

impl Appearance {
    pub fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        Ok(Self {
            warning: color(theme, "statusIndicator.warning")?,
            error: color(theme, "statusIndicator.error")?,
        })
    }
}

fn label(locale: &ResolvedLocale, status: &ChordStatus) -> Option<String> {
    if status.no_match {
        return Some(message(locale, "keymap.chordNoMatch", &[]));
    }
    status
        .shortcut
        .as_ref()
        .map(|shortcut| message(locale, "keymap.chordPending", &[("shortcut", shortcut)]))
}

pub fn show(
    ui: &mut Ui,
    locale: &ResolvedLocale,
    appearance: &Appearance,
    status: &ChordStatus,
) -> Option<Response> {
    let text = label(locale, status)?;
    let color = if status.no_match {
        appearance.error
    } else {
        appearance.warning
    };
    Some(
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = ICON_GAP;
            let (rect, _) =
                ui.allocate_exact_size(vec2(ICON_SIZE, ICON_SIZE), egui::Sense::hover());
            let scale = ICON_SIZE / ICON_VIEWBOX;
            let point = |x, y| rect.min + vec2(x, y) * scale;
            let stroke = Stroke::new(ICON_STROKE * scale, color);
            ui.painter().rect_stroke(
                egui::Rect::from_min_max(
                    point(KEYBOARD_RECT[0], KEYBOARD_RECT[1]),
                    point(KEYBOARD_RECT[2], KEYBOARD_RECT[3]),
                ),
                ICON_CORNER * scale,
                stroke,
                egui::StrokeKind::Middle,
            );
            for [x, y] in KEYBOARD_KEYS {
                ui.painter()
                    .circle_filled(point(x, y), stroke.width / 2.0, color);
            }
            let ends = KEYBOARD_SPACE.map(|[x, y]| point(x, y));
            ui.painter().line_segment(ends, stroke);
            for end in ends {
                ui.painter().circle_filled(end, stroke.width / 2.0, color);
            }
            ui.add(
                egui::Label::new(
                    egui::RichText::new(text)
                        .font(FontId::proportional(TEXT_SIZE))
                        .color(color),
                )
                .extend()
                .selectable(false),
            );
        })
        .response,
    )
}

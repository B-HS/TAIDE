use eframe::egui::{self, Color32, FontId, Id, Stroke, Ui, vec2};
use taide_model::ids::TabId;
use taide_model::locale::ResolvedLocale;

use crate::presentation::message;
use crate::preview::Failure;

pub const EXTERNAL_ACTION_KEY: &str = "preview.openExternally";
const TEXT_SIZE: f32 = 12.0;
const ICON_VIEWBOX: f32 = 24.0;
const ICON_STROKE: f32 = 2.0;
const STATUS_TEXT_SIZE: f32 = 14.0;
const STATUS_ICON_SIZE: f32 = 20.0;
const STATUS_GAP: f32 = 12.0;
const UNSUPPORTED_ICON_SIZE: f32 = 40.0;
const SUBTITLE_GAP: f32 = 4.0;
const SUBTITLE_ALPHA: f32 = 0.7;
const ACTION_PADDING: f32 = 12.0;
const EXTERNAL_ICON_SIZE: f32 = 16.0;
const GAP: f32 = 8.0;
const BUTTON_CORNER: u8 = 6;
const DIVIDER_WIDTH: f32 = 1.0;
const ACTION_HEIGHT: f32 = 32.0;

pub struct Appearance {
    pub background: Color32,
    pub border: Color32,
    pub foreground: Color32,
    pub muted: Color32,
}

pub fn control_id(viewport: egui::ViewportId, tab: &TabId, key: &str) -> Id {
    Id::new(("native-preview-status-control", viewport, tab, key))
}

fn file_icon(ui: &Ui, rect: egui::Rect, color: Color32, question: bool) {
    let scale = rect.width() / ICON_VIEWBOX;
    let point = |[x, y]: [f32; 2]| rect.min + vec2(x, y) * scale;
    let stroke = Stroke::new(ICON_STROKE * scale, color);
    let line = |values: &[[f32; 2]]| {
        ui.painter().add(egui::Shape::line(
            values.iter().copied().map(point).collect(),
            stroke,
        ));
    };
    let curve = |values: [[f32; 2]; 4]| {
        ui.painter()
            .add(egui::epaint::CubicBezierShape::from_points_stroke(
                values.map(point),
                false,
                Color32::TRANSPARENT,
                stroke,
            ));
    };
    curve([[6.0, 22.0], [4.895, 22.0], [4.0, 21.105], [4.0, 20.0]]);
    line(&[[4.0, 20.0], [4.0, 4.0]]);
    curve([[4.0, 4.0], [4.0, 2.895], [4.895, 2.0], [6.0, 2.0]]);
    line(&[[6.0, 2.0], [14.0, 2.0]]);
    curve([[14.0, 2.0], [14.636, 2.0], [15.248, 2.252], [15.704, 2.706]]);
    line(&[[15.704, 2.706], [19.292, 6.294]]);
    curve([[19.292, 6.294], [19.742, 6.742], [20.0, 7.364], [20.0, 8.0]]);
    line(&[[20.0, 8.0], [20.0, 20.0]]);
    curve([[20.0, 20.0], [20.0, 21.105], [19.105, 22.0], [18.0, 22.0]]);
    line(&[[18.0, 22.0], [6.0, 22.0]]);
    line(&[[12.0, 17.0], [12.01, 17.0]]);
    if question {
        curve([[9.1, 9.0], [9.74, 6.668], [13.125, 6.105], [14.515, 8.14]]);
        curve([
            [14.515, 8.14],
            [14.783, 8.716],
            [14.92, 9.35],
            [14.92, 10.0],
        ]);
        curve([[14.92, 10.0], [14.92, 12.0], [11.92, 13.0], [11.92, 13.0]]);
    } else {
        line(&[[12.0, 9.0], [12.0, 13.0]]);
    }
}

fn open_external(
    ui: &mut Ui,
    tab: &TabId,
    with_icon: bool,
    locale: &ResolvedLocale,
    appearance: &Appearance,
) -> bool {
    let label = message(locale, "preview.openExternally", &[]);
    let text = ui.painter().layout_no_wrap(
        label.clone(),
        FontId::proportional(STATUS_TEXT_SIZE),
        appearance.foreground,
    );
    let icon_width = if with_icon {
        EXTERNAL_ICON_SIZE + GAP
    } else {
        0.0
    };
    let (_, rect) = ui.allocate_space(vec2(
        text.size().x + ACTION_PADDING * 2.0 + icon_width,
        ACTION_HEIGHT,
    ));
    let response = ui.interact(
        rect,
        control_id(ui.ctx().viewport_id(), tab, "preview.openExternally"),
        egui::Sense::click(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &label)
    });
    let visuals = ui.style().interact(&response);
    let background = if response.hovered() || response.has_focus() {
        visuals.weak_bg_fill
    } else {
        appearance.background
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(BUTTON_CORNER), background);
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(BUTTON_CORNER),
        Stroke::new(DIVIDER_WIDTH, appearance.border),
        egui::StrokeKind::Inside,
    );
    ui.painter().galley(
        egui::pos2(
            rect.left() + ACTION_PADDING + icon_width,
            rect.center().y - text.size().y / 2.0,
        ),
        text,
        appearance.foreground,
    );
    if with_icon {
        let scale = EXTERNAL_ICON_SIZE / ICON_VIEWBOX;
        let origin = egui::pos2(
            rect.left() + ACTION_PADDING,
            rect.center().y - EXTERNAL_ICON_SIZE / 2.0,
        );
        let point = |[x, y]: [f32; 2]| origin + vec2(x, y) * scale;
        let stroke = Stroke::new(ICON_STROKE * scale, appearance.foreground);
        let line = |values: &[[f32; 2]]| {
            ui.painter().add(egui::Shape::line(
                values.iter().copied().map(point).collect(),
                stroke,
            ));
        };
        let curve = |values: [[f32; 2]; 4]| {
            ui.painter()
                .add(egui::epaint::CubicBezierShape::from_points_stroke(
                    values.map(point),
                    false,
                    Color32::TRANSPARENT,
                    stroke,
                ));
        };
        line(&[[15.0, 3.0], [21.0, 3.0], [21.0, 9.0]]);
        line(&[[10.0, 14.0], [21.0, 3.0]]);
        line(&[[18.0, 13.0], [18.0, 19.0]]);
        curve([[18.0, 19.0], [18.0, 20.105], [17.105, 21.0], [16.0, 21.0]]);
        line(&[[16.0, 21.0], [5.0, 21.0]]);
        curve([[5.0, 21.0], [3.895, 21.0], [3.0, 20.105], [3.0, 19.0]]);
        line(&[[3.0, 19.0], [3.0, 8.0]]);
        curve([[3.0, 8.0], [3.0, 6.895], [3.895, 6.0], [5.0, 6.0]]);
        line(&[[5.0, 6.0], [11.0, 6.0]]);
    }
    response.clicked()
}

pub fn show(
    ui: &mut Ui,
    tab: &TabId,
    failure: Option<&Failure>,
    file_name: &str,
    locale: &ResolvedLocale,
    appearance: &Appearance,
    decode_error_key: &str,
) -> bool {
    let is_failed = failure.is_some();
    let is_read = matches!(failure, Some(Failure::Read(_)));
    let key = if is_read {
        "preview.notSupported"
    } else if is_failed {
        decode_error_key
    } else {
        "common.loading"
    };
    let label = message(locale, key, &[]);
    let icon_size = if is_read {
        UNSUPPORTED_ICON_SIZE
    } else {
        STATUS_ICON_SIZE
    };
    let text_height = ui
        .painter()
        .layout_no_wrap(
            label.clone(),
            FontId::proportional(STATUS_TEXT_SIZE),
            appearance.foreground,
        )
        .size()
        .y;
    let subtitle_height = if is_read {
        SUBTITLE_GAP
            + ui.painter()
                .layout_no_wrap(
                    file_name.into(),
                    FontId::proportional(TEXT_SIZE),
                    appearance.foreground,
                )
                .size()
                .y
    } else {
        0.0
    };
    let action_height = if is_failed {
        STATUS_GAP + ACTION_HEIGHT
    } else {
        0.0
    };
    let height = icon_size + STATUS_GAP + text_height + subtitle_height + action_height;
    ui.add_space(((ui.available_height() - height) / 2.0).max(0.0));
    ui.vertical_centered(|ui| {
        ui.spacing_mut().item_spacing.y = STATUS_GAP;
        if is_failed {
            let (_, rect) = ui.allocate_space(vec2(icon_size, icon_size));
            file_icon(
                ui,
                rect,
                if is_read {
                    appearance.muted
                } else {
                    appearance.foreground
                },
                is_read,
            );
        } else {
            ui.add(egui::Spinner::new().size(STATUS_ICON_SIZE));
        }
        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = SUBTITLE_GAP;
            ui.label(
                egui::RichText::new(label)
                    .size(STATUS_TEXT_SIZE)
                    .color(if is_read {
                        appearance.muted
                    } else {
                        appearance.foreground
                    }),
            );
            if is_read {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(file_name)
                            .size(TEXT_SIZE)
                            .color(appearance.foreground.gamma_multiply(SUBTITLE_ALPHA)),
                    )
                    .truncate(),
                );
            }
        });
        is_failed && open_external(ui, tab, is_read, locale, appearance)
    })
    .inner
}

pub fn show_empty(ui: &mut Ui, locale: &ResolvedLocale, appearance: &Appearance, key: &str) {
    let label = message(locale, key, &[]);
    let text_height = ui
        .painter()
        .layout_no_wrap(
            label.clone(),
            FontId::proportional(STATUS_TEXT_SIZE),
            appearance.foreground,
        )
        .size()
        .y;
    let height = STATUS_ICON_SIZE + STATUS_GAP + text_height;
    ui.add_space(((ui.available_height() - height) / 2.0).max(0.0));
    ui.vertical_centered(|ui| {
        ui.spacing_mut().item_spacing.y = STATUS_GAP;
        let (_, rect) = ui.allocate_space(vec2(STATUS_ICON_SIZE, STATUS_ICON_SIZE));
        file_icon(ui, rect, appearance.foreground, false);
        ui.label(
            egui::RichText::new(label)
                .size(STATUS_TEXT_SIZE)
                .color(appearance.foreground),
        );
    });
}

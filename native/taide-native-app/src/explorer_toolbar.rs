use eframe::egui::{self, Color32, Id, Response, Stroke, Ui, vec2};
use taide_model::locale::ResolvedLocale;

const BUTTON_SIZE: f32 = 24.0;
const ICON_SIZE: f32 = 16.0;
const ICON_VIEWBOX: f32 = 24.0;
const ICON_STROKE: f32 = 2.0;
#[cfg(test)]
#[path = "explorer-tooltip-tests.rs"]
mod tests;
pub const GAP: f32 = 2.0;
pub const WIDTH: f32 = BUTTON_SIZE * 4.0 + GAP * 3.0;
pub const KEYS: [&str; 4] = [
    "explorer.newFile",
    "explorer.newFolder",
    "explorer.refresh",
    "explorer.collapseAll",
];

pub fn button(ui: &mut Ui, key: &str, id: Id, opacity: f32, locale: &ResolvedLocale) -> Response {
    let label = crate::presentation::message(locale, key, &[]);
    let (_, rect) = ui.allocate_space(vec2(BUTTON_SIZE, BUTTON_SIZE));
    let response = ui.interact(rect, id, egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &label)
    });
    if ui.is_rect_visible(rect) {
        let mut painter = ui.painter().clone();
        painter.multiply_opacity(opacity);
        let visuals = ui.style().interact(&response);
        if response.hovered() || response.has_focus() {
            painter.rect_filled(rect, visuals.corner_radius, visuals.weak_bg_fill);
        }
        let scale = ICON_SIZE / ICON_VIEWBOX;
        let origin = rect.center() - vec2(ICON_SIZE, ICON_SIZE) / 2.0;
        let point = |value: [f32; 2]| origin + vec2(value[0], value[1]) * scale;
        let stroke = Stroke::new(ICON_STROKE * scale, visuals.fg_stroke.color);
        let lines = |values: &[[f32; 2]]| {
            painter.add(egui::Shape::line(
                values.iter().copied().map(point).collect(),
                stroke,
            ));
        };
        let curve = |values: [[f32; 2]; 4]| {
            painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
                values.map(point),
                false,
                Color32::TRANSPARENT,
                stroke,
            ));
        };
        match key {
            "explorer.newFile" => {
                lines(&[[6.0, 22.0], [18.0, 22.0]]);
                curve([[18.0, 22.0], [19.105, 22.0], [20.0, 21.105], [20.0, 20.0]]);
                lines(&[[20.0, 20.0], [20.0, 8.0], [14.0, 2.0], [6.0, 2.0]]);
                curve([[6.0, 2.0], [4.895, 2.0], [4.0, 2.895], [4.0, 4.0]]);
                lines(&[[4.0, 4.0], [4.0, 20.0]]);
                curve([[4.0, 20.0], [4.0, 21.105], [4.895, 22.0], [6.0, 22.0]]);
                lines(&[[14.0, 2.0], [14.0, 7.0]]);
                curve([[14.0, 7.0], [14.0, 7.552], [14.448, 8.0], [15.0, 8.0]]);
                lines(&[[15.0, 8.0], [20.0, 8.0]]);
                lines(&[[9.0, 15.0], [15.0, 15.0]]);
                lines(&[[12.0, 18.0], [12.0, 12.0]]);
            }
            "explorer.newFolder" => {
                lines(&[[12.0, 10.0], [12.0, 16.0]]);
                lines(&[[9.0, 13.0], [15.0, 13.0]]);
                lines(&[[4.0, 20.0], [20.0, 20.0]]);
                curve([[20.0, 20.0], [21.105, 20.0], [22.0, 19.105], [22.0, 18.0]]);
                lines(&[[22.0, 18.0], [22.0, 8.0]]);
                curve([[22.0, 8.0], [22.0, 6.895], [21.105, 6.0], [20.0, 6.0]]);
                lines(&[[20.0, 6.0], [12.1, 6.0]]);
                curve([[12.1, 6.0], [11.428, 6.0], [10.8, 5.662], [10.41, 5.1]]);
                lines(&[[10.41, 5.1], [9.6, 3.9]]);
                curve([[9.6, 3.9], [9.227, 3.33], [8.58, 3.0], [7.93, 3.0]]);
                lines(&[[7.93, 3.0], [4.0, 3.0]]);
                curve([[4.0, 3.0], [2.895, 3.0], [2.0, 3.895], [2.0, 5.0]]);
                lines(&[[2.0, 5.0], [2.0, 18.0]]);
                curve([[2.0, 18.0], [2.0, 19.105], [2.895, 20.0], [4.0, 20.0]]);
            }
            "explorer.refresh" => {
                curve([[3.0, 12.0], [3.0, 7.03], [7.03, 3.0], [12.0, 3.0]]);
                curve([[12.0, 3.0], [14.68, 3.0], [17.09, 4.09], [18.74, 5.74]]);
                lines(&[[18.74, 5.74], [21.0, 8.0], [21.0, 3.0]]);
                lines(&[[21.0, 8.0], [16.0, 8.0]]);
                curve([[21.0, 12.0], [21.0, 16.97], [16.97, 21.0], [12.0, 21.0]]);
                curve([[12.0, 21.0], [9.32, 21.0], [6.91, 19.91], [5.26, 18.26]]);
                lines(&[[5.26, 18.26], [3.0, 16.0], [3.0, 21.0]]);
                lines(&[[3.0, 16.0], [8.0, 16.0]]);
            }
            "explorer.collapseAll" => {
                lines(&[[7.0, 20.0], [12.0, 15.0], [17.0, 20.0]]);
                lines(&[[7.0, 4.0], [12.0, 9.0], [17.0, 4.0]]);
            }
            _ => {}
        }
    }
    response
}

pub(crate) fn show_tooltips(
    output: &crate::explorer::Output,
    locale: &ResolvedLocale,
    tooltips: &crate::tooltips::Provider,
    appearance: &crate::tooltips::Appearance,
) {
    for key in KEYS {
        if let Some(response) = output.toolbar.get(key) {
            tooltips.show(
                response,
                &crate::presentation::message(locale, key, &[]),
                egui::RectAlign::BOTTOM,
                appearance,
            );
        }
    }
    if let Some(input) = &output.input {
        if output.validation_error.is_some() {
            input.ctx.accesskit_node_builder(input.id, |node| {
                node.set_invalid(egui::accesskit::Invalid::True);
            });
        }
        tooltips.show_controlled(
            input,
            output.validation_error.as_deref(),
            egui::RectAlign::BOTTOM,
            appearance,
        );
    }
}

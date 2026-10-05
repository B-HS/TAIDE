use eframe::egui::{self, FontId, Id, Response, Stroke, Ui, vec2};
use taide_model::{ids::TabId, locale::ResolvedLocale};

use crate::presentation::message;
use crate::preview_hwp::Cache;
pub use crate::preview_pdf_surface::Appearance;

const BUTTON_SIZE: f32 = 24.0;
const ICON_SIZE: f32 = 14.0;
const ICON_VIEWBOX: f32 = 24.0;
const ICON_STROKE: f32 = 2.0;
const TEXT_SIZE: f32 = 12.0;
const GAP: f32 = 8.0;
const BUTTON_CORNER: u8 = 6;
const HEADER_PADDING_X: i8 = 12;
const HEADER_PADDING_Y: i8 = 6;
const DIVIDER_WIDTH: f32 = 1.0;
const CANVAS_MARGIN: f32 = 16.0;

pub fn control_id(viewport: egui::ViewportId, tab: &TabId, key: &str) -> Id {
    if key == crate::preview_status::EXTERNAL_ACTION_KEY {
        return crate::preview_status::control_id(viewport, tab, key);
    }
    Id::new(("native-hwp-control", viewport, tab, key))
}

fn button(
    ui: &mut Ui,
    tab: &TabId,
    key: &str,
    enabled: bool,
    locale: &ResolvedLocale,
    tooltips: &mut Vec<crate::tooltips::Trigger>,
) -> Response {
    let label = message(locale, key, &[]);
    let response = ui
        .add_enabled_ui(enabled, |ui| {
            let (_, rect) = ui.allocate_space(vec2(BUTTON_SIZE, BUTTON_SIZE));
            let response = ui.interact(
                rect,
                control_id(ui.ctx().viewport_id(), tab, key),
                egui::Sense::click(),
            );
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &label)
            });
            let visuals = ui.style().interact(&response);
            if response.hovered() || response.has_focus() {
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::same(BUTTON_CORNER),
                    visuals.weak_bg_fill,
                );
            }
            let scale = ICON_SIZE / ICON_VIEWBOX;
            let origin = rect.center() - vec2(ICON_SIZE, ICON_SIZE) / 2.0;
            let points = if key == "preview.hwp.previousPage" {
                [[15.0, 18.0], [9.0, 12.0], [15.0, 6.0]]
            } else {
                [[9.0, 18.0], [15.0, 12.0], [9.0, 6.0]]
            };
            ui.painter().add(egui::Shape::line(
                points.map(|[x, y]| origin + vec2(x, y) * scale).to_vec(),
                Stroke::new(ICON_STROKE * scale, visuals.fg_stroke.color),
            ));
            response
        })
        .inner;
    tooltips.push(crate::tooltips::Trigger {
        response: response.clone(),
        label,
        align: egui::RectAlign::BOTTOM,
        focus_target: None,
    });
    response
}

pub fn show(
    ui: &mut Ui,
    cache: &mut Cache,
    tab: &TabId,
    path: &str,
    file_name: &str,
    locale: &ResolvedLocale,
    appearance: &Appearance,
) -> bool {
    show_with_tooltips(ui, cache, tab, path, file_name, locale, appearance).external
}

pub(crate) fn show_with_tooltips(
    ui: &mut Ui,
    cache: &mut Cache,
    tab: &TabId,
    path: &str,
    file_name: &str,
    locale: &ResolvedLocale,
    appearance: &Appearance,
) -> crate::preview_pdf_surface::Output {
    let mut tooltips = Vec::new();
    cache.ensure(tab, path);
    ui.painter().rect_filled(
        ui.max_rect(),
        egui::CornerRadius::ZERO,
        appearance.background,
    );
    let status = crate::preview_status::Appearance {
        background: appearance.background,
        border: appearance.border,
        foreground: appearance.foreground,
        muted: appearance.muted,
    };
    let external = ui
        .scope(|ui| {
            ui.visuals_mut().override_text_color = Some(appearance.foreground);
            ui.spacing_mut().item_spacing.y = 0.0;
            if let Some(failure) = cache.error(tab) {
                return crate::preview_status::show(
                    ui,
                    tab,
                    Some(failure),
                    file_name,
                    locale,
                    &status,
                    "preview.hwp.loadFailed",
                );
            }
            let Some(pages) = cache.total_pages(tab) else {
                return cache.has_source(tab)
                    && crate::preview_status::show(
                        ui,
                        tab,
                        None,
                        file_name,
                        locale,
                        &status,
                        "preview.hwp.loadFailed",
                    );
            };
            if pages == 0 {
                crate::preview_status::show_empty(ui, locale, &status, "preview.hwp.noPages");
                return false;
            }
            let page = cache.selection(tab).unwrap_or_default();
            let label = message(
                locale,
                "preview.hwp.pageIndicator",
                &[
                    ("current", &(page + 1).to_string()),
                    ("total", &pages.to_string()),
                ],
            );
            let text_width = ui
                .painter()
                .layout_no_wrap(
                    label.clone(),
                    FontId::proportional(TEXT_SIZE),
                    appearance.foreground,
                )
                .size()
                .x;
            let width = BUTTON_SIZE * 2.0 + GAP * 2.0 + text_width;
            let frame = egui::Frame::NONE
                .fill(appearance.header)
                .inner_margin(egui::Margin::symmetric(HEADER_PADDING_X, HEADER_PADDING_Y))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.x = GAP;
                    ui.horizontal(|ui| {
                        ui.add_space(((ui.available_width() - width) / 2.0 - GAP).max(0.0));
                        if button(
                            ui,
                            tab,
                            "preview.hwp.previousPage",
                            page > 0,
                            locale,
                            &mut tooltips,
                        )
                        .clicked()
                        {
                            cache.change(tab, page - 1);
                        }
                        ui.label(egui::RichText::new(&label).size(TEXT_SIZE));
                        if button(
                            ui,
                            tab,
                            "preview.hwp.nextPage",
                            page + 1 < pages,
                            locale,
                            &mut tooltips,
                        )
                        .clicked()
                        {
                            cache.change(tab, page + 1);
                        }
                    });
                });
            ui.painter().hline(
                frame.response.rect.x_range(),
                frame.response.rect.bottom(),
                Stroke::new(DIVIDER_WIDTH, appearance.border),
            );
            if let Some(texture) = cache.texture(tab) {
                egui::ScrollArea::both()
                    .id_salt(("native-hwp-scroll", tab, cache.scroll_generation(tab)))
                    .show(ui, |ui| {
                        let available = ui.available_width();
                        let natural = texture.size_vec2();
                        let size = natural * (available / natural.x).min(1.0);
                        ui.add_space(CANVAS_MARGIN);
                        ui.allocate_ui_with_layout(
                            vec2(available, size.y),
                            egui::Layout::top_down(egui::Align::Center),
                            |ui| {
                                ui.add(
                                    egui::Image::new(texture)
                                        .fit_to_exact_size(size)
                                        .alt_text(&label),
                                );
                            },
                        );
                        ui.add_space(CANVAS_MARGIN);
                    });
            }
            false
        })
        .inner;
    crate::preview_pdf_surface::Output { external, tooltips }
}

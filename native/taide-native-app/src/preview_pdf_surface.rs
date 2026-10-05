use eframe::egui::{self, Color32, FontId, Id, Response, Stroke, TextureHandle, Ui, vec2};
use taide_model::ids::TabId;
use taide_model::locale::ResolvedLocale;

use crate::presentation::message;
use crate::preview_pdf::{Cache, MAX_ZOOM, MIN_ZOOM, Selection, ZOOM_DIVISOR};

const BUTTON_SIZE: f32 = 24.0;
const ICON_SIZE: f32 = 14.0;
const ICON_VIEWBOX: f32 = 24.0;
const ICON_STROKE: f32 = 2.0;
const TEXT_SIZE: f32 = 12.0;
const GAP: f32 = 8.0;
const BUTTON_CORNER: u8 = 6;
const HEADER_PADDING_X: i8 = 12;
const HEADER_PADDING_Y: i8 = 6;
const DIVIDER_HEIGHT: f32 = 16.0;
const DIVIDER_WIDTH: f32 = 1.0;
const DIVIDER_MARGIN: f32 = 4.0;
const CANVAS_MARGIN: f32 = 16.0;
const PERCENT: u16 = 100;
const CONTROLS: usize = 7;
const BUTTONS: usize = 4;

pub struct Appearance {
    pub background: Color32,
    pub header: Color32,
    pub border: Color32,
    pub foreground: Color32,
    pub muted: Color32,
}

pub(crate) struct Output {
    pub(crate) external: bool,
    pub(crate) tooltips: Vec<crate::tooltips::Trigger>,
}

pub fn control_id(viewport: egui::ViewportId, tab: &TabId, key: &str) -> Id {
    if key == crate::preview_status::EXTERNAL_ACTION_KEY {
        return crate::preview_status::control_id(viewport, tab, key);
    }
    Id::new(("native-pdf-control", viewport, tab, key))
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
            let point = |[x, y]: [f32; 2]| origin + vec2(x, y) * scale;
            let stroke = Stroke::new(ICON_STROKE * scale, visuals.fg_stroke.color);
            let lines = |values: &[[f32; 2]]| {
                ui.painter().add(egui::Shape::line(
                    values.iter().copied().map(point).collect(),
                    stroke,
                ));
            };
            match key {
                "preview.pdf.previousPage" => lines(&[[15.0, 18.0], [9.0, 12.0], [15.0, 6.0]]),
                "preview.pdf.nextPage" => lines(&[[9.0, 18.0], [15.0, 12.0], [9.0, 6.0]]),
                "preview.pdf.zoomOut" | "preview.pdf.zoomIn" => {
                    ui.painter()
                        .circle_stroke(point([11.0, 11.0]), 8.0 * scale, stroke);
                    lines(&[[21.0, 21.0], [16.65, 16.65]]);
                    lines(&[[8.0, 11.0], [14.0, 11.0]]);
                    if key == "preview.pdf.zoomIn" {
                        lines(&[[11.0, 8.0], [11.0, 14.0]]);
                    }
                }
                _ => {}
            }
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

fn controls(
    ui: &mut Ui,
    tab: &TabId,
    selection: Selection,
    pages: usize,
    locale: &ResolvedLocale,
    appearance: &Appearance,
    tooltips: &mut Vec<crate::tooltips::Trigger>,
) -> Selection {
    let mut selected = selection;
    let page_label = message(
        locale,
        "preview.pdf.pageIndicator",
        &[
            ("current", &selection.page.to_string()),
            ("total", &pages.to_string()),
        ],
    );
    let percent_label = format!(
        "{}%",
        u16::from(selection.zoom) * PERCENT / u16::from(ZOOM_DIVISOR)
    );
    let font = FontId::proportional(TEXT_SIZE);
    let text_width = [page_label.as_str(), percent_label.as_str()]
        .iter()
        .map(|text| {
            ui.painter()
                .layout_no_wrap((*text).into(), font.clone(), appearance.foreground)
                .size()
                .x
        })
        .sum::<f32>();
    let width = BUTTON_SIZE * BUTTONS as f32
        + text_width
        + DIVIDER_WIDTH
        + DIVIDER_MARGIN * 2.0
        + GAP * (CONTROLS - 1) as f32;
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
                    "preview.pdf.previousPage",
                    selection.page > 1,
                    locale,
                    tooltips,
                )
                .clicked()
                {
                    selected.page -= 1;
                }
                ui.label(egui::RichText::new(page_label).size(TEXT_SIZE));
                if button(
                    ui,
                    tab,
                    "preview.pdf.nextPage",
                    selection.page < pages,
                    locale,
                    tooltips,
                )
                .clicked()
                {
                    selected.page += 1;
                }
                let (rect, _) = ui.allocate_exact_size(
                    vec2(DIVIDER_WIDTH + DIVIDER_MARGIN * 2.0, DIVIDER_HEIGHT),
                    egui::Sense::hover(),
                );
                ui.painter().vline(
                    rect.center().x,
                    rect.y_range(),
                    Stroke::new(DIVIDER_WIDTH, appearance.border),
                );
                if button(
                    ui,
                    tab,
                    "preview.pdf.zoomOut",
                    selection.zoom > MIN_ZOOM,
                    locale,
                    tooltips,
                )
                .clicked()
                {
                    selected.zoom -= 1;
                }
                ui.label(egui::RichText::new(percent_label).size(TEXT_SIZE));
                if button(
                    ui,
                    tab,
                    "preview.pdf.zoomIn",
                    selection.zoom < MAX_ZOOM,
                    locale,
                    tooltips,
                )
                .clicked()
                {
                    selected.zoom += 1;
                }
            });
        });
    ui.painter().hline(
        frame.response.rect.x_range(),
        frame.response.rect.bottom(),
        Stroke::new(DIVIDER_WIDTH, appearance.border),
    );
    selected
}

fn canvas(ui: &mut Ui, tab: &TabId, generation: u64, texture: &TextureHandle, file_name: &str) {
    egui::ScrollArea::both()
        .id_salt(("native-pdf-scroll", tab, generation))
        .show(ui, |ui| {
            let size = texture.size_vec2();
            let width = ui.available_width().max(size.x);
            ui.add_space(CANVAS_MARGIN);
            ui.allocate_ui_with_layout(
                vec2(width, size.y),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                    let shadow = egui::epaint::Shadow {
                        offset: [0, 1],
                        blur: 3,
                        spread: 0,
                        color: Color32::from_black_alpha(26),
                    };
                    ui.painter()
                        .add(shadow.as_shape(rect, egui::CornerRadius::ZERO));
                    ui.put(
                        rect,
                        egui::Image::new(texture)
                            .fit_to_exact_size(size)
                            .alt_text(file_name),
                    );
                },
            );
            ui.add_space(CANVAS_MARGIN);
        });
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
) -> Output {
    let mut tooltips = Vec::new();
    cache.ensure(tab, path);
    ui.painter().rect_filled(
        ui.max_rect(),
        egui::CornerRadius::ZERO,
        appearance.background,
    );
    let status_appearance = crate::preview_status::Appearance {
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
                    &status_appearance,
                    "preview.pdf.loadFailed",
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
                        &status_appearance,
                        "preview.pdf.loadFailed",
                    );
            };
            let selection = cache.selection(tab).unwrap_or_default();
            let next = controls(ui, tab, selection, pages, locale, appearance, &mut tooltips);
            cache.change(tab, next);
            if let Some(texture) = cache.texture(tab) {
                canvas(ui, tab, cache.scroll_generation(tab), texture, file_name);
            }
            false
        })
        .inner;
    Output { external, tooltips }
}

use eframe::egui::{self, Color32, FontId, Id, Stroke, Ui, vec2};
use taide_model::ids::TabId;
use taide_model::locale::ResolvedLocale;

use crate::presentation::message;
use crate::preview_presentation_cache::Cache;

const SIDEBAR_WIDTH: f32 = 192.0;
const SIDEBAR_PADDING_Y: f32 = 4.0;
const BUTTON_PADDING_X: f32 = 12.0;
const BUTTON_PADDING_Y: f32 = 6.0;
const TEXT_SIZE: f32 = 12.0;
const TEXT_LINE_HEIGHT: f32 = 16.0;
const BODY_TEXT_SIZE: f32 = 14.0;
const BODY_LINE_HEIGHT: f32 = 20.0;
const BODY_PADDING_X: i8 = 16;
const BODY_PADDING_Y: i8 = 12;
const DISCLAIMER_PADDING_X: i8 = 12;
const DISCLAIMER_PADDING_Y: i8 = 6;
const WARNING_ALPHA: f32 = 0.15;
const EMPTY_ALPHA: f32 = 0.6;
const ICON_SIZE: f32 = 14.0;
const ICON_VIEWBOX: f32 = 24.0;
const ICON_STROKE: f32 = 2.0;
const GAP: f32 = 8.0;
const BORDER: f32 = 1.0;

pub struct Appearance {
    pub status: crate::preview_status::Appearance,
    pub border: Color32,
    pub selected: Color32,
    pub hover: Color32,
    pub warning: Color32,
}

pub fn slide_id(viewport: egui::ViewportId, tab: &TabId, index: usize) -> Id {
    Id::new(("native-presentation-slide", viewport, tab, index))
}

fn disclaimer(ui: &mut Ui, locale: &ResolvedLocale, appearance: &Appearance) {
    egui::Frame::NONE
        .fill(appearance.warning.gamma_multiply(WARNING_ALPHA))
        .inner_margin(egui::Margin::symmetric(
            DISCLAIMER_PADDING_X,
            DISCLAIMER_PADDING_Y,
        ))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.x = GAP;
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(vec2(ICON_SIZE, ICON_SIZE), egui::Sense::hover());
                let point = |[x, y]: [f32; 2]| rect.min + vec2(x, y) * (ICON_SIZE / ICON_VIEWBOX);
                let stroke =
                    Stroke::new(ICON_STROKE * ICON_SIZE / ICON_VIEWBOX, appearance.warning);
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
                line(&[[21.73, 18.0], [13.73, 4.0]]);
                curve([[13.73, 4.0], [12.96, 2.67], [11.02, 2.67], [10.25, 4.0]]);
                line(&[[10.25, 4.0], [2.25, 18.0]]);
                curve([[2.25, 18.0], [1.48, 19.34], [2.46, 21.0], [4.0, 21.0]]);
                line(&[[4.0, 21.0], [20.0, 21.0]]);
                curve([[20.0, 21.0], [21.54, 21.0], [22.5, 19.33], [21.73, 18.0]]);
                line(&[[12.0, 9.0], [12.0, 13.0]]);
                line(&[[12.0, 17.0], [12.01, 17.0]]);
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(message(
                            locale,
                            "preview.presentation.layoutDisclaimer",
                            &[],
                        ))
                        .size(TEXT_SIZE)
                        .line_height(Some(TEXT_LINE_HEIGHT))
                        .color(appearance.warning),
                    )
                    .wrap(),
                );
            });
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
    cache.ensure(tab, path);
    ui.painter().rect_filled(
        ui.max_rect(),
        egui::CornerRadius::ZERO,
        appearance.status.background,
    );
    ui.scope(|ui| {
        ui.visuals_mut().override_text_color = Some(appearance.status.foreground);
        ui.spacing_mut().item_spacing.y = 0.0;
        if let Some(failure) = cache.error(tab) {
            return crate::preview_status::show(
                ui,
                tab,
                Some(failure),
                file_name,
                locale,
                &appearance.status,
                "preview.presentation.loadFailed",
            );
        }
        let Some(outline) = cache.outline(tab).cloned() else {
            return cache.has_source(tab)
                && crate::preview_status::show(
                    ui,
                    tab,
                    None,
                    file_name,
                    locale,
                    &appearance.status,
                    "preview.presentation.loadFailed",
                );
        };
        disclaimer(ui, locale, appearance);
        let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
        let sidebar_rect = egui::Rect::from_min_size(rect.min, vec2(SIDEBAR_WIDTH, rect.height()));
        let mut sidebar = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("native-presentation-sidebar", tab))
                .max_rect(egui::Rect {
                    max: egui::pos2(sidebar_rect.right() - BORDER, sidebar_rect.bottom()),
                    ..sidebar_rect
                })
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        sidebar.set_clip_rect(sidebar_rect.intersect(ui.clip_rect()));
        sidebar.spacing_mut().item_spacing.y = 0.0;
        let generation = cache.scroll_generation(tab);
        egui::ScrollArea::vertical()
            .id_salt(("native-presentation-sidebar-scroll", tab, generation))
            .show(&mut sidebar, |ui| {
                ui.add_space(SIDEBAR_PADDING_Y);
                let selected = cache.selected(tab).unwrap_or_default();
                for (position, slide) in outline.slides.iter().enumerate() {
                    let label = message(
                        locale,
                        "preview.presentation.slideLabel",
                        &[("index", &slide.index.to_string())],
                    );
                    let (_, rect) = ui.allocate_space(vec2(
                        ui.available_width(),
                        TEXT_LINE_HEIGHT + BUTTON_PADDING_Y * 2.0,
                    ));
                    let response = ui.interact(
                        rect,
                        slide_id(ui.ctx().viewport_id(), tab, slide.index),
                        egui::Sense::click(),
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &label)
                    });
                    if selected == position {
                        ui.painter().rect_filled(
                            rect,
                            egui::CornerRadius::ZERO,
                            appearance.selected,
                        );
                    } else if response.hovered() {
                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::ZERO, appearance.hover);
                    }
                    if response.has_focus() {
                        ui.painter().rect_stroke(
                            rect,
                            egui::CornerRadius::ZERO,
                            ui.visuals().selection.stroke,
                            egui::StrokeKind::Inside,
                        );
                    }
                    let mut job = egui::text::LayoutJob::simple(
                        label,
                        FontId::proportional(TEXT_SIZE),
                        appearance.status.foreground,
                        (rect.width() - BUTTON_PADDING_X * 2.0).max(0.0),
                    );
                    job.wrap.max_rows = 1;
                    job.wrap.break_anywhere = true;
                    for section in &mut job.sections {
                        section.format.line_height = Some(TEXT_LINE_HEIGHT);
                    }
                    let text = ui.painter().layout_job(job);
                    ui.painter().galley(
                        egui::pos2(
                            rect.left() + BUTTON_PADDING_X,
                            rect.center().y - text.size().y / 2.0,
                        ),
                        text,
                        appearance.status.foreground,
                    );
                    if response.clicked() && cache.select(tab, position) {
                        ui.ctx().request_repaint();
                    }
                }
                ui.add_space(SIDEBAR_PADDING_Y);
            });
        ui.painter().vline(
            sidebar_rect.right() - BORDER / 2.0,
            rect.y_range(),
            Stroke::new(BORDER, appearance.border),
        );
        if rect.right() <= sidebar_rect.right() {
            return false;
        }
        let content_rect =
            egui::Rect::from_min_max(egui::pos2(sidebar_rect.right(), rect.top()), rect.max);
        let mut content = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("native-presentation-content", tab))
                .max_rect(content_rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        content.set_clip_rect(content_rect.intersect(ui.clip_rect()));
        egui::ScrollArea::vertical()
            .id_salt(("native-presentation-content-scroll", tab, generation))
            .show(&mut content, |ui| {
                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(BODY_PADDING_X, BODY_PADDING_Y))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = GAP;
                        let selected = cache.selected(tab).unwrap_or_default();
                        let Some(slide) = outline.slides.get(selected) else {
                            return;
                        };
                        if slide.paragraphs.is_empty() {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(message(
                                        locale,
                                        "preview.presentation.noText",
                                        &[],
                                    ))
                                    .size(TEXT_SIZE)
                                    .line_height(Some(TEXT_LINE_HEIGHT))
                                    .color(
                                        appearance.status.foreground.gamma_multiply(EMPTY_ALPHA),
                                    ),
                                )
                                .wrap(),
                            );
                            return;
                        }
                        for paragraph in &slide.paragraphs {
                            let text = paragraph
                                .split(|character| {
                                    matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{c}')
                                })
                                .filter(|part| !part.is_empty())
                                .collect::<Vec<_>>()
                                .join(" ");
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(text)
                                        .size(BODY_TEXT_SIZE)
                                        .line_height(Some(BODY_LINE_HEIGHT)),
                                )
                                .wrap(),
                            );
                        }
                    });
            });
        false
    })
    .inner
}

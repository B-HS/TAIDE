use eframe::egui::{self, Color32, FontId, Id, Rect, Response, Stroke, Ui, vec2};
use resvg::{tiny_skia, usvg};
use taide_model::{
    error::{AppError, AppResult},
    locale::ResolvedLocale,
    system::{SystemUsage, SystemUsageProcess, SystemUsageProcessKind},
    theme::ResolvedTheme,
};

use crate::presentation::{color, message};

const BYTES_PER_MEBIBYTE: f64 = 1024.0 * 1024.0;
const ICON_SIZE: f32 = 12.0;
const ICON_VIEWBOX: f32 = 24.0;
const MAX_RASTER_SIDE: f32 = 1024.0;
const STATUS_FONT: f32 = 11.0;
const BUTTON_HEIGHT: f32 = 16.0;
const BUTTON_PADDING: f32 = 4.0;
const ICON_GAP: f32 = 4.0;
const MAX_WIDTH: f32 = 672.0;
const HEIGHT_RATIO: f32 = 0.7;
const SCREEN_MARGIN: f32 = 16.0;
const PADDING: i8 = 24;
const BORDER: f32 = 1.0;
const CORNER_RADIUS: u8 = 8;
const SECTION_GAP: f32 = 16.0;
const GROUP_GAP: f32 = 12.0;
const GROUP_HEADER_GAP: f32 = 4.0;
const COLUMN_FONT: f32 = 10.0;
const PROCESS_FONT: f32 = 12.0;
const TITLE_FONT: f32 = 18.0;
const CPU_WIDTH: f32 = 64.0;
const MEMORY_WIDTH: f32 = 80.0;
const ROW_PADDING_X: f32 = 12.0;
const ROW_PADDING_Y: f32 = 4.0;
const LINE_HEIGHT: f32 = 16.0;
const SCROLL_PADDING: f32 = 8.0;
const CLOSE_SIZE: f32 = 16.0;
const CLOSE_STROKE: f32 = 2.0;
const CLOSE_INSET_RATIO: f32 = 0.25;
const SMALL_RADIUS: u8 = 2;
const SHADOW_OFFSET: [i8; 2] = [0, 8];
const SHADOW_BLUR: u8 = 24;
const SCRIM_OPACITY: f32 = 0.5;
const GROUPS: [(SystemUsageProcessKind, &str); 5] = [
    (SystemUsageProcessKind::App, "window.systemUsageKindApp"),
    (
        SystemUsageProcessKind::Terminal,
        "window.systemUsageKindTerminal",
    ),
    (SystemUsageProcessKind::Lsp, "window.systemUsageKindLsp"),
    (SystemUsageProcessKind::Agent, "window.systemUsageKindAgent"),
    (SystemUsageProcessKind::Other, "window.systemUsageKindOther"),
];

pub(crate) struct Appearance {
    background: Color32,
    border: Color32,
    foreground: Color32,
    muted: Color32,
    hover: Color32,
    shadow: Color32,
    focus: Color32,
    tooltip: crate::tooltips::Appearance,
}

impl Appearance {
    pub(crate) fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        Ok(Self {
            background: color(theme, "modal.background")?,
            border: color(theme, "modal.border")?,
            foreground: color(theme, "app.foreground")?,
            muted: color(theme, "appSidebar.iconDefault")?,
            hover: color(theme, "explorer.itemHover")?,
            shadow: color(theme, "app.shadow")?,
            focus: color(theme, "app.focusBorder")?,
            tooltip: crate::tooltips::Appearance::new(theme)?,
        })
    }
}

pub(crate) struct Icon {
    tree: usvg::Tree,
    texture: Option<(f32, egui::TextureHandle)>,
}

impl Icon {
    pub(crate) fn new() -> AppResult<Self> {
        let options = usvg::Options {
            image_href_resolver: usvg::ImageHrefResolver {
                resolve_string: Box::new(|_, _| None),
                resolve_data: Box::new(|_, _, _| None),
            },
            ..Default::default()
        };
        Ok(Self {
            tree: usvg::Tree::from_data(
                include_bytes!("../resources/status/activity.svg"),
                &options,
            )
            .map_err(|error| AppError::Internal(format!("native system usage icon: {error}")))?,
            texture: None,
        })
    }

    fn prepare(&mut self, context: &egui::Context) -> AppResult<egui::TextureId> {
        let scale = context.pixels_per_point();
        if let Some((previous, texture)) = &self.texture
            && *previous == scale
        {
            return Ok(texture.id());
        }
        let pixels = (ICON_SIZE * scale).ceil();
        if !pixels.is_finite() || !(1.0..=MAX_RASTER_SIDE).contains(&pixels) {
            return Err(AppError::Internal(
                "native system usage icon scale is out of range".into(),
            ));
        }
        let side = pixels as u32;
        let mut pixmap = tiny_skia::Pixmap::new(side, side).ok_or_else(|| {
            AppError::Internal("native system usage icon allocation failed".into())
        })?;
        resvg::render(
            &self.tree,
            tiny_skia::Transform::from_scale(
                ICON_SIZE * scale / ICON_VIEWBOX,
                ICON_SIZE * scale / ICON_VIEWBOX,
            ),
            &mut pixmap.as_mut(),
        );
        let texture = context.load_texture(
            "native-system-usage-icon",
            egui::ColorImage::from_rgba_unmultiplied(
                [side as usize, side as usize],
                &pixmap.take_demultiplied(),
            ),
            egui::TextureOptions::LINEAR,
        );
        let id = texture.id();
        self.texture = Some((scale, texture));
        Ok(id)
    }
}

fn summary_text(locale: &ResolvedLocale, usage: &SystemUsage) -> String {
    let cpu = usage
        .cpu_percent
        .map(|cpu| format!("{:.0}", cpu.round()))
        .unwrap_or_else(|| "--".into());
    let memory = format!("{:.0}", (usage.memory_bytes / BYTES_PER_MEBIBYTE).round());
    message(
        locale,
        "window.systemUsage",
        &[("cpu", &cpu), ("memory", &memory)],
    )
}

pub(crate) fn show_status(
    ui: &mut Ui,
    locale: &ResolvedLocale,
    appearance: &Appearance,
    icon: &mut Icon,
    usage: Option<&SystemUsage>,
    tooltips: &crate::tooltips::Provider,
) -> AppResult<Option<Response>> {
    let Some(usage) = usage else {
        return Ok(None);
    };
    let texture = icon.prepare(ui.ctx())?;
    let text = summary_text(locale, usage);
    let galley = ui.painter().layout_no_wrap(
        text.clone(),
        FontId::proportional(STATUS_FONT),
        appearance.muted,
    );
    let (id, rect) = ui.allocate_space(vec2(
        galley.size().x + ICON_SIZE + ICON_GAP + BUTTON_PADDING * 2.0,
        BUTTON_HEIGHT,
    ));
    let response = ui.interact(rect, id.with("native-system-usage"), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &text)
    });
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, SMALL_RADIUS, appearance.hover);
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            SMALL_RADIUS,
            Stroke::new(BORDER, appearance.focus),
            egui::StrokeKind::Inside,
        );
    }
    let icon_rect = Rect::from_min_size(
        egui::pos2(
            rect.left() + BUTTON_PADDING,
            rect.center().y - ICON_SIZE / 2.0,
        ),
        vec2(ICON_SIZE, ICON_SIZE),
    );
    ui.painter().image(
        texture,
        icon_rect,
        Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        appearance.muted,
    );
    ui.painter().galley(
        egui::pos2(
            icon_rect.right() + ICON_GAP,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        appearance.muted,
    );
    tooltips.show(
        &response,
        &message(locale, "window.systemUsageHint", &[]),
        egui::RectAlign::TOP,
        &appearance.tooltip,
    );
    Ok(Some(response))
}

fn columns(ui: &mut Ui, appearance: &Appearance, labels: [&str; 3], header: bool) {
    let height = if header {
        LINE_HEIGHT
    } else {
        LINE_HEIGHT + ROW_PADDING_Y * 2.0
    };
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
    if !header && response.hovered() {
        ui.painter()
            .rect_filled(rect, SMALL_RADIUS, appearance.hover);
    }
    let content = rect.shrink2(vec2(ROW_PADDING_X, 0.0));
    let memory = Rect::from_min_max(
        egui::pos2(
            (content.right() - MEMORY_WIDTH).max(content.left()),
            content.top(),
        ),
        content.right_bottom(),
    );
    let cpu_right = (memory.left() - GROUP_GAP).max(content.left());
    let cpu = Rect::from_min_max(
        egui::pos2((cpu_right - CPU_WIDTH).max(content.left()), content.top()),
        egui::pos2(cpu_right, content.bottom()),
    );
    let label = Rect::from_min_max(
        content.left_top(),
        egui::pos2(
            (cpu.left() - GROUP_GAP).max(content.left()),
            content.bottom(),
        ),
    );
    let font = FontId::proportional(if header { COLUMN_FONT } else { PROCESS_FONT });
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(label)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.set_clip_rect(ui.clip_rect().intersect(label));
            let response = ui.add_sized(
                label.size(),
                egui::Label::new(egui::RichText::new(labels[0]).font(font.clone()).color(
                    if header {
                        appearance.muted
                    } else {
                        appearance.foreground
                    },
                ))
                .truncate()
                .selectable(false),
            );
            if !header {
                response.on_hover_text(labels[0]);
            }
        },
    );
    for (bounds, text) in [(cpu, labels[1]), (memory, labels[2])] {
        ui.painter()
            .with_clip_rect(ui.clip_rect().intersect(bounds))
            .text(
                bounds.right_center(),
                egui::Align2::RIGHT_CENTER,
                text,
                font.clone(),
                appearance.muted,
            );
    }
}

pub(crate) fn show_detail(
    context: &egui::Context,
    locale: &ResolvedLocale,
    appearance: &Appearance,
    processes: &[SystemUsageProcess],
) -> bool {
    let frame = egui::Frame::popup(&context.global_style())
        .fill(appearance.background)
        .stroke(Stroke::new(BORDER, appearance.border))
        .corner_radius(CORNER_RADIUS)
        .inner_margin(PADDING)
        .shadow(egui::epaint::Shadow {
            offset: SHADOW_OFFSET,
            blur: SHADOW_BLUR,
            spread: 0,
            color: appearance.shadow,
        });
    let response = egui::Modal::new(Id::new("native-system-usage-detail"))
        .frame(frame)
        .backdrop_color(appearance.shadow.gamma_multiply(SCRIM_OPACITY))
        .show(context, |ui| {
            let available = context.content_rect().size();
            let margin = (f32::from(PADDING) + BORDER) * 2.0;
            let width = ((available.x - SCREEN_MARGIN * 2.0).min(MAX_WIDTH) - margin).max(0.0);
            let height = (available.y * HEIGHT_RATIO - margin).max(0.0);
            ui.set_width(width);
            ui.set_height(height);
            ui.spacing_mut().item_spacing.y = SECTION_GAP;
            ui.visuals_mut().override_text_color = Some(appearance.foreground);
            let mut close = false;
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    vec2(
                        (ui.available_width() - CLOSE_SIZE - GROUP_GAP).max(0.0),
                        TITLE_FONT,
                    ),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(message(
                                    locale,
                                    "window.systemUsageDetailTitle",
                                    &[],
                                ))
                                .size(TITLE_FONT)
                                .strong(),
                            )
                            .truncate()
                            .selectable(false),
                        );
                    },
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (rect, response) =
                        ui.allocate_exact_size(vec2(CLOSE_SIZE, CLOSE_SIZE), egui::Sense::click());
                    let title = message(locale, "common.close", &[]);
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &title)
                    });
                    let stroke = Stroke::new(CLOSE_STROKE, appearance.muted);
                    let glyph = rect.shrink(CLOSE_SIZE * CLOSE_INSET_RATIO);
                    ui.painter()
                        .line_segment([glyph.left_top(), glyph.right_bottom()], stroke);
                    ui.painter()
                        .line_segment([glyph.right_top(), glyph.left_bottom()], stroke);
                    if response.has_focus() {
                        ui.painter().rect_stroke(
                            rect,
                            SMALL_RADIUS,
                            Stroke::new(BORDER, appearance.focus),
                            egui::StrokeKind::Outside,
                        );
                    }
                    close = response.clicked();
                    response.on_hover_text(title);
                });
            });
            let headers = [
                "window.systemUsageProcessColumn",
                "window.systemUsageCpuColumn",
                "window.systemUsageMemoryColumn",
            ]
            .map(|key| message(locale, key, &[]).to_uppercase());
            columns(
                ui,
                appearance,
                [&headers[0], &headers[1], &headers[2]],
                true,
            );
            egui::ScrollArea::vertical()
                .id_salt("native-system-usage-processes")
                .max_height(ui.available_height())
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = GROUP_GAP;
                    ui.set_width((width - SCROLL_PADDING).max(0.0));
                    if processes.is_empty() {
                        ui.add_space(SECTION_GAP);
                        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(message(
                                    locale,
                                    "window.systemUsageEmpty",
                                    &[],
                                ))
                                .size(PROCESS_FONT)
                                .color(appearance.muted),
                            );
                        });
                        return;
                    }
                    for (kind, key) in GROUPS {
                        let rows = processes
                            .iter()
                            .filter(|process| process.kind == kind)
                            .collect::<Vec<_>>();
                        if rows.is_empty() {
                            continue;
                        }
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = GROUP_HEADER_GAP;
                            ui.label(
                                egui::RichText::new(message(locale, key, &[]).to_uppercase())
                                    .size(COLUMN_FONT)
                                    .strong()
                                    .color(appearance.muted),
                            );
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                for process in rows {
                                    ui.push_id(process.pid, |ui| {
                                        let cpu = format!(
                                            "{:.0}%",
                                            process.cpu_percent.unwrap_or_default().round()
                                        );
                                        let memory = format!(
                                            "{:.0}MB",
                                            (process.memory_bytes / BYTES_PER_MEBIBYTE).round()
                                        );
                                        columns(
                                            ui,
                                            appearance,
                                            [&process.label, &cpu, &memory],
                                            false,
                                        );
                                    });
                                }
                            });
                        });
                    }
                });
            close
        });
    response.inner || response.should_close()
}

#[cfg(test)]
#[path = "system-usage-view-tests.rs"]
mod tests;

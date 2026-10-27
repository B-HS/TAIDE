use eframe::egui;
use taide_model::error::{AppError, AppResult};

const BORDER_WIDTH: f32 = 4.0;
const DASH_LENGTH: f32 = BORDER_WIDTH * 3.0;
const BACKGROUND_OPACITY: f32 = 0.85;
const ICON_SIZE: f32 = 40.0;
const CONTENT_GAP: f32 = 12.0;
const LABEL_FONT_SIZE: f32 = 14.0;
const LABEL_LINE_HEIGHT: f32 = 20.0;
const OVERLAY_LAYER: &str = "native-file-drop-overlay";

#[derive(Debug)]
pub enum Notice {
    OpenProjectFirst,
    Failed(AppError),
}

pub(crate) struct Colors {
    pub(crate) background: egui::Color32,
    pub(crate) accent: egui::Color32,
    pub(crate) foreground: egui::Color32,
}

impl Colors {
    pub(crate) fn new(theme: &taide_model::theme::ResolvedTheme) -> AppResult<Self> {
        let color = |key| crate::presentation::color(theme, key);
        Ok(Self {
            background: color("app.background")?,
            accent: color("app.accent")?,
            foreground: color("app.foreground")?,
        })
    }
}

pub(crate) fn is_hovering(context: &egui::Context) -> bool {
    context.input(|input| !input.raw.hovered_files.is_empty())
}

pub(crate) fn dropped_paths(context: &egui::Context) -> Vec<String> {
    context.input(|input| {
        input
            .raw
            .dropped_files
            .iter()
            .filter_map(|file| file.path().to_str().map(str::to_owned))
            .collect()
    })
}

pub(crate) fn paint_overlay(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    colors: &Colors,
    label: &str,
    icons: &mut crate::navigation_icons::Icons,
) -> AppResult<()> {
    let overlay = ui.new_child(
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::new(
                egui::Order::Middle,
                egui::Id::new(OVERLAY_LAYER),
            ))
            .max_rect(rect),
    );
    let painter = overlay.painter();
    painter.rect_filled(
        rect,
        0.0,
        colors.background.gamma_multiply(BACKGROUND_OPACITY),
    );
    let stroke = egui::Stroke::new(BORDER_WIDTH, colors.accent);
    let inset = BORDER_WIDTH / 2.0;
    for side in [
        [
            egui::pos2(rect.left(), rect.top() + inset),
            egui::pos2(rect.right(), rect.top() + inset),
        ],
        [
            egui::pos2(rect.right() - inset, rect.top()),
            egui::pos2(rect.right() - inset, rect.bottom()),
        ],
        [
            egui::pos2(rect.right(), rect.bottom() - inset),
            egui::pos2(rect.left(), rect.bottom() - inset),
        ],
        [
            egui::pos2(rect.left() + inset, rect.bottom()),
            egui::pos2(rect.left() + inset, rect.top()),
        ],
    ] {
        painter.extend(egui::Shape::dashed_line(
            &side,
            stroke,
            DASH_LENGTH,
            DASH_LENGTH,
        ));
    }
    let mut job = egui::text::LayoutJob::single_section(
        label.to_owned(),
        egui::TextFormat {
            font_id: egui::FontId::new(
                LABEL_FONT_SIZE,
                taide_native_ui::font_families::medium(&overlay),
            ),
            line_height: Some(LABEL_LINE_HEIGHT),
            color: colors.foreground,
            ..Default::default()
        },
    );
    job.wrap.max_width = rect.width();
    let galley = overlay.fonts_mut(|fonts| fonts.layout_job(job));
    let top = rect.center().y - (ICON_SIZE + CONTENT_GAP + galley.size().y) / 2.0;
    let icon = egui::Rect::from_min_size(
        egui::pos2(rect.center().x - ICON_SIZE / 2.0, top),
        egui::Vec2::splat(ICON_SIZE),
    );
    icons.paint(
        &overlay,
        icon,
        crate::navigation_icons::Icon::FolderInput,
        colors.accent,
        0.0,
    )?;
    painter.galley(
        egui::pos2(
            rect.center().x - galley.size().x / 2.0,
            icon.bottom() + CONTENT_GAP,
        ),
        galley,
        colors.foreground,
    );
    Ok(())
}

#[cfg(test)]
#[path = "file-drop-tests.rs"]
mod tests;

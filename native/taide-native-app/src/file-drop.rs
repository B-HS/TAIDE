use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use eframe::egui;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::TabId;

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

#[derive(Default)]
struct WebDragState {
    hovering: HashSet<TabId>,
    dropped: Vec<String>,
}

/// OS file drags a preview web view received instead of the window. The original preview lived in
/// the app's own webview, whose drag-drop events reached `app-shell.tsx` 124-135 like any other.
/// Each method answers whether the web view should keep the drag from its default handling.
#[derive(Clone, Default)]
pub(crate) struct WebDrags(Arc<Mutex<WebDragState>>);

impl WebDrags {
    fn state(&self) -> MutexGuard<'_, WebDragState> {
        self.0.lock().unwrap_or_else(|error| error.into_inner())
    }

    pub(crate) fn enter(&self, tab: &TabId, paths: &[PathBuf]) -> bool {
        let mut state = self.state();
        if paths.is_empty() {
            state.hovering.remove(tab);
            return false;
        }
        state.hovering.insert(tab.clone());
        true
    }

    pub(crate) fn over(&self, tab: &TabId) -> bool {
        self.state().hovering.contains(tab)
    }

    pub(crate) fn leave(&self, tab: &TabId) -> bool {
        self.state().hovering.remove(tab)
    }

    pub(crate) fn drop_paths(&self, tab: &TabId, paths: &[PathBuf]) -> bool {
        let mut state = self.state();
        if !state.hovering.remove(tab) {
            return false;
        }
        state.dropped.extend(
            paths
                .iter()
                .filter_map(|path| path.to_str().map(str::to_owned)),
        );
        true
    }

    /// The tab's web view is gone, so no `leave` or drop will come from it.
    pub(crate) fn forget(&self, tab: &TabId) {
        self.state().hovering.remove(tab);
    }

    pub(crate) fn is_hovering(&self) -> bool {
        !self.state().hovering.is_empty()
    }

    pub(crate) fn take_dropped(&self) -> Vec<String> {
        std::mem::take(&mut self.state().dropped)
    }
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

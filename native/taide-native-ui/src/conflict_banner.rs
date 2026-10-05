use egui::{Align, Color32, Layout, Rect, RichText, Sense, Stroke, Ui, Vec2, pos2};
use taide_model::locale::ResolvedLocale;

const FONT_SIZE: f32 = 12.0;
const ICON_SIZE: f32 = 14.0;
const ICON_STROKE: f32 = 1.0;
const BUTTON_HEIGHT: f32 = 24.0;
const PADDING_X: i8 = 12;
const PADDING_Y: i8 = 6;
const BACKGROUND_ALPHA: f32 = 0.15;
const ITEM_GAP: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerVariant {
    ChangedOnDisk,
    MirrorRestored,
    MirrorRestoredConflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerAction {
    ViewDisk,
    KeepMine,
    Dismiss,
}

pub struct BannerAppearance {
    pub error: Color32,
    pub warning: Color32,
}

pub struct BannerOutput {
    pub action: Option<BannerAction>,
    pub actions: Vec<(BannerAction, Rect)>,
    pub rect: Rect,
}

pub fn show(
    ui: &mut Ui,
    locale: &ResolvedLocale,
    appearance: &BannerAppearance,
    variant: BannerVariant,
) -> BannerOutput {
    let (key, color) = match variant {
        BannerVariant::ChangedOnDisk => ("editor.changedOnDisk", appearance.error),
        BannerVariant::MirrorRestored => ("editor.mirrorRestored", appearance.warning),
        BannerVariant::MirrorRestoredConflict => {
            ("editor.mirrorRestoredConflict", appearance.error)
        }
    };
    let mut action = None;
    let mut actions = Vec::new();
    let width = ui.available_width();
    let response = egui::Frame::NONE
        .fill(color.gamma_multiply(BACKGROUND_ALPHA))
        .inner_margin(egui::Margin::symmetric(PADDING_X, PADDING_Y))
        .show(ui, |ui| {
            ui.set_min_width((width - f32::from(PADDING_X) * 2.0).max(0.0));
            ui.spacing_mut().item_spacing.x = ITEM_GAP;
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(ICON_SIZE), Sense::hover());
                let stroke = Stroke::new(ICON_STROKE, color);
                if variant == BannerVariant::MirrorRestored {
                    ui.painter()
                        .circle_stroke(rect.center(), ICON_SIZE / 2.0, stroke);
                } else {
                    ui.painter().line_segment(
                        [pos2(rect.center().x, rect.top()), rect.left_bottom()],
                        stroke,
                    );
                    ui.painter()
                        .line_segment([rect.left_bottom(), rect.right_bottom()], stroke);
                    ui.painter().line_segment(
                        [rect.right_bottom(), pos2(rect.center().x, rect.top())],
                        stroke,
                    );
                }
                let stem = rect.shrink(ICON_SIZE / 3.0);
                ui.painter().line_segment(
                    [
                        pos2(rect.center().x, stem.top()),
                        pos2(rect.center().x, stem.bottom()),
                    ],
                    stroke,
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let choices: &[(_, _, _)] = if variant == BannerVariant::MirrorRestored {
                        &[(BannerAction::Dismiss, "common.close", false)]
                    } else {
                        &[
                            (BannerAction::KeepMine, "editor.keepMine", false),
                            (BannerAction::ViewDisk, "editor.viewDiskContent", true),
                        ]
                    };
                    for (choice, label, outline) in choices {
                        let text = locale
                            .messages
                            .get(*label)
                            .cloned()
                            .unwrap_or_else(|| (*label).into());
                        let response = ui.add(
                            egui::Button::new(RichText::new(text).size(FONT_SIZE).color(color))
                                .frame(*outline)
                                .min_size(egui::vec2(0.0, BUTTON_HEIGHT)),
                        );
                        actions.push((*choice, response.rect));
                        if response.clicked() {
                            action = Some(*choice);
                        }
                    }
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        ui.label(
                            RichText::new(
                                locale
                                    .messages
                                    .get(key)
                                    .cloned()
                                    .unwrap_or_else(|| key.into()),
                            )
                            .size(FONT_SIZE)
                            .color(color),
                        );
                    });
                });
            });
        });
    BannerOutput {
        action,
        actions,
        rect: response.response.rect,
    }
}

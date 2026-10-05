use egui::{self, Color32, FontId, RichText, Stroke, Ui};
use taide_model::{
    error::{AppError, AppResult},
    locale::ResolvedLocale,
    theme::ResolvedTheme,
};

use crate::{
    presentation::{color, message, parse_color},
    theme_editor_tokens::TERMINAL,
};

const FONT: f32 = 12.0;
const BORDER: f32 = 1.0;
const RADIUS: u8 = 6;
const TAB_X: i8 = 8;
const TAB_Y: i8 = 4;
const TAB_INDICATOR: f32 = 2.0;
const DIRTY_RADIUS: f32 = 3.0;
const DIRTY_SIZE: f32 = 6.0;
const TAB_GAP: f32 = 6.0;
const EDITOR_PADDING: i8 = 12;
const TERMINAL_Y: i8 = 8;
const LINE_GAP: f32 = 2.0;
const INDENT: f32 = 16.0;
const CURSOR_HEIGHT: f32 = 14.0;
const ANSI_COUNT: usize = 16;
const ANSI_COLUMNS: usize = 8;
const ANSI_GAP: f32 = 4.0;
const ANSI_SIZE: f32 = 12.0;

pub fn show(
    ui: &mut Ui,
    theme: &ResolvedTheme,
    locale: &ResolvedLocale,
) -> AppResult<Vec<crate::tooltip_trigger::Trigger>> {
    let mut tooltips = Vec::new();
    let border = color(theme, "app.border")?;
    let tab_background = color(theme, "tabBar.background")?;
    let tab_active = color(theme, "tabBar.tabActiveBackground")?;
    let tab_inactive = color(theme, "tabBar.tabInactiveBackground")?;
    let active_text = color(theme, "tabBar.tabActiveForeground")?;
    let inactive_text = color(theme, "tabBar.tabInactiveForeground")?;
    let indicator = color(theme, "tabBar.tabActiveIndicator")?;
    let dirty = color(theme, "tabBar.dirtyDot")?;
    let editor = color(theme, "editor.background")?;
    let foreground = color(theme, "editor.foreground")?;
    let success = color(theme, "statusIndicator.success")?;
    let terminal = |key: &str| -> AppResult<Color32> {
        let value = theme.terminal.get(key).ok_or_else(|| {
            AppError::Internal(format!("native terminal preview token missing: {key}"))
        })?;
        parse_color(value, key)
    };
    let terminal_background = terminal("background")?;
    let terminal_foreground = terminal("foreground")?;
    let cursor = terminal("cursor")?;
    let ansi = TERMINAL[..ANSI_COUNT]
        .iter()
        .map(|key| terminal(key))
        .collect::<AppResult<Vec<_>>>()?;
    let syntax = theme
        .syntax
        .iter()
        .map(|(key, style)| parse_color(&style.fg, key).map(|color| (key.clone(), color)))
        .collect::<AppResult<std::collections::BTreeMap<_, _>>>()?;
    let width = ui.available_width();
    egui::Frame::NONE
        .stroke(Stroke::new(BORDER, border))
        .corner_radius(RADIUS)
        .show(ui, |ui| {
            ui.set_width((width - BORDER - BORDER).max(0.0));
            ui.spacing_mut().item_spacing.y = 0.0;
            egui::Frame::NONE
                .fill(tab_background)
                .inner_margin(TAB_Y)
                .show(ui, |ui| {
                    ui.set_min_width(
                        (width - BORDER - BORDER - f32::from(TAB_Y) - f32::from(TAB_Y)).max(0.0),
                    );
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = BORDER;
                        let active = egui::Frame::NONE
                            .fill(tab_active)
                            .inner_margin(egui::Margin::symmetric(TAB_X, TAB_Y))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = TAB_GAP;
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::Vec2::splat(DIRTY_SIZE),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter()
                                        .circle_filled(rect.center(), DIRTY_RADIUS, dirty);
                                    ui.label(
                                        RichText::new(message(
                                            locale,
                                            "themeEditor.previewEditorTab",
                                            &[],
                                        ))
                                        .size(FONT)
                                        .color(active_text),
                                    );
                                });
                            });
                        ui.painter().hline(
                            active.response.rect.x_range(),
                            active.response.rect.top(),
                            Stroke::new(TAB_INDICATOR, indicator),
                        );
                        egui::Frame::NONE
                            .fill(tab_inactive)
                            .inner_margin(egui::Margin::symmetric(TAB_X, TAB_Y))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(message(
                                        locale,
                                        "themeEditor.previewTerminalTab",
                                        &[],
                                    ))
                                    .size(FONT)
                                    .color(inactive_text),
                                );
                            });
                    });
                });
            let piece = |ui: &mut Ui, token: &str, text: &str| {
                let mut text = RichText::new(text)
                    .font(FontId::monospace(FONT))
                    .color(syntax.get(token).copied().unwrap_or(foreground));
                if let Some(style) = theme.syntax.get(token) {
                    if style.bold {
                        text = text.strong();
                    }
                    if style.italic {
                        text = text.italics();
                    }
                }
                ui.label(text);
            };
            egui::Frame::NONE
                .fill(editor)
                .inner_margin(EDITOR_PADDING)
                .show(ui, |ui| {
                    ui.set_min_width(
                        (width
                            - BORDER
                            - BORDER
                            - f32::from(EDITOR_PADDING)
                            - f32::from(EDITOR_PADDING))
                        .max(0.0),
                    );
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, LINE_GAP);
                    ui.horizontal(|ui| {
                        piece(ui, "keyword", "const");
                        piece(ui, "", " ");
                        piece(ui, "variable", "greeting");
                        piece(ui, "", " = ");
                        piece(ui, "string", "'taide'");
                    });
                    ui.horizontal(|ui| {
                        piece(ui, "keyword", "function");
                        piece(ui, "", " ");
                        piece(ui, "function", "render");
                        piece(ui, "punctuation", "(");
                        piece(ui, "parameter", "props");
                        piece(ui, "punctuation", ")");
                        piece(ui, "", " {");
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(INDENT);
                        piece(ui, "decorator", "@decorator");
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(INDENT);
                        piece(
                            ui,
                            "comment",
                            &format!(
                                "// {}",
                                message(locale, "themeEditor.previewCommentText", &[])
                            ),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(INDENT);
                        piece(ui, "keyword", "return");
                        piece(ui, "", " ");
                        piece(ui, "number", "42");
                    });
                    piece(ui, "", "}");
                });
            egui::Frame::NONE
                .fill(terminal_background)
                .inner_margin(egui::Margin::symmetric(EDITOR_PADDING, TERMINAL_Y))
                .show(ui, |ui| {
                    ui.set_min_width(
                        (width
                            - BORDER
                            - BORDER
                            - f32::from(EDITOR_PADDING)
                            - f32::from(EDITOR_PADDING))
                        .max(0.0),
                    );
                    ui.spacing_mut().item_spacing = egui::vec2(ANSI_GAP, f32::from(TERMINAL_Y));
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(message(
                                locale,
                                "themeEditor.previewTerminalPrompt",
                                &[],
                            ))
                            .font(FontId::monospace(FONT))
                            .color(success),
                        );
                        ui.label(
                            RichText::new(message(
                                locale,
                                "themeEditor.previewTerminalCommand",
                                &[],
                            ))
                            .font(FontId::monospace(FONT))
                            .color(terminal_foreground),
                        );
                    });
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(DIRTY_SIZE, CURSOR_HEIGHT),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(rect, 0, cursor);
                    egui::Grid::new("ansi")
                        .num_columns(ANSI_COLUMNS)
                        .spacing(egui::Vec2::splat(ANSI_GAP))
                        .show(ui, |ui| {
                            for (index, color) in ansi.into_iter().enumerate() {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::Vec2::splat(ANSI_SIZE),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(rect, 2, color);
                                tooltips.push(crate::tooltip_trigger::Trigger {
                                    response,
                                    label: TERMINAL[index].into(),
                                    align: egui::RectAlign::TOP,
                                    focus_target: None,
                                });
                                if (index + 1) % ANSI_COLUMNS == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                });
        });
    Ok(tooltips)
}

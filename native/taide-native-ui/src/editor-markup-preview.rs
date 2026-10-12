use std::collections::HashSet;

use egui::{FontFamily, Margin, RichText, Stroke, Ui};
use taide_native_editor::documentation::{Alignment, Block, Inline, RichDocument};

use super::{Colors, Provider, inlines};
use crate::editor_surface::EditorAppearance;

const FONT_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 20.0;
const PARAGRAPH_LINE_HEIGHT: f32 = FONT_SIZE * 1.625;
const HEADING_SIZES: [f32; 3] = [24.0, 20.0, 18.0];
const HEADING_LINE_HEIGHTS: [f32; 3] = [32.0, 28.0, 28.0];
const HEADING_TOP: [f32; 3] = [16.0, 16.0, 12.0];
const HEADING_BOTTOM: f32 = 8.0;
const PARAGRAPH_MARGIN: f32 = 8.0;
const RULE_MARGIN: f32 = 16.0;
const QUOTE_INDENT: i8 = 12;
const QUOTE_BORDER: f32 = 2.0;
const QUOTE_OPACITY: f32 = 0.8;
const LIST_INDENT: i8 = 20;
const CODE_PADDING: i8 = 12;
pub(super) const CORNER_RADIUS: u8 = 4;
pub(super) const INLINE_CODE_PADDING: Margin = Margin::symmetric(4, 2);
const CELL_PADDING: Margin = Margin::symmetric(8, 4);
const CELL_BORDER: f32 = 1.0;

fn inline_contents(
    ui: &mut Ui,
    contents: &[Inline],
    appearance: &EditorAppearance,
    colors: Colors,
    provider: &mut (impl Provider + ?Sized),
    size: f32,
    line_height: f32,
    bold: bool,
) {
    let mut appearance = appearance.clone();
    appearance.line_height = line_height;
    let contents = contents
        .iter()
        .cloned()
        .map(|inline| match inline {
            Inline::Text(mut span) => {
                span.style.bold |= bold;
                Inline::Text(span)
            }
            inline => inline,
        })
        .collect::<Vec<_>>();
    inlines(
        ui,
        &contents,
        &appearance,
        colors,
        provider,
        size,
        false,
        true,
    );
}

fn blocks(
    ui: &mut Ui,
    contents: &[Block],
    appearance: &EditorAppearance,
    colors: Colors,
    provider: &mut (impl Provider + ?Sized),
) {
    for (index, block) in contents.iter().enumerate() {
        ui.push_id(index, |ui| match block {
            Block::Paragraph(contents) => {
                ui.add_space(PARAGRAPH_MARGIN);
                inline_contents(
                    ui,
                    contents,
                    appearance,
                    colors,
                    provider,
                    FONT_SIZE,
                    PARAGRAPH_LINE_HEIGHT,
                    false,
                );
                ui.add_space(PARAGRAPH_MARGIN);
            }
            Block::Heading { level, contents } => {
                let index = usize::from(level.saturating_sub(1));
                let size = HEADING_SIZES.get(index).copied().unwrap_or(FONT_SIZE);
                let line_height = HEADING_LINE_HEIGHTS
                    .get(index)
                    .copied()
                    .unwrap_or(LINE_HEIGHT);
                ui.add_space(HEADING_TOP.get(index).copied().unwrap_or_default());
                inline_contents(
                    ui,
                    contents,
                    appearance,
                    colors,
                    provider,
                    size,
                    line_height,
                    HEADING_SIZES.get(index).is_some(),
                );
                ui.add_space(if HEADING_SIZES.get(index).is_some() {
                    HEADING_BOTTOM
                } else {
                    0.0
                });
            }
            Block::Quote(contents) => {
                let output = egui::Frame::new()
                    .inner_margin(Margin {
                        left: QUOTE_INDENT,
                        ..Margin::ZERO
                    })
                    .show(ui, |ui| {
                        ui.multiply_opacity(QUOTE_OPACITY);
                        blocks(ui, contents, appearance, colors, provider);
                    });
                ui.painter().vline(
                    output.response.rect.left(),
                    output.response.rect.y_range(),
                    Stroke::new(QUOTE_BORDER, colors.border.gamma_multiply(QUOTE_OPACITY)),
                );
            }
            Block::List { start, items } => {
                for (index, item) in items.iter().enumerate() {
                    ui.push_id(index, |ui| {
                        egui::Frame::new()
                            .inner_margin(Margin {
                                left: LIST_INDENT,
                                ..Margin::ZERO
                            })
                            .show(ui, |ui| {
                                ui.horizontal_top(|ui| {
                                    if let Some(checked) = item.checked {
                                        ui.add_enabled(
                                            false,
                                            egui::Checkbox::without_text(&mut checked.clone()),
                                        );
                                    } else if let Some(start) = start {
                                        ui.label(
                                            RichText::new(format!(
                                                "{}.",
                                                item.ordinal.unwrap_or_else(|| taide_native_editor::documentation::list_ordinal(*start, index))
                                            ))
                                            .size(FONT_SIZE),
                                        );
                                    } else {
                                        ui.label(RichText::new("\u{2022}").size(FONT_SIZE));
                                    }
                                    ui.vertical(|ui| {
                                        blocks(ui, &item.blocks, appearance, colors, provider)
                                    });
                                });
                            });
                    });
                }
            }
            Block::Code { text, .. } => {
                ui.add_space(PARAGRAPH_MARGIN);
                egui::Frame::new()
                    .fill(colors.code_background)
                    .corner_radius(CORNER_RADIUS)
                    .inner_margin(CODE_PADDING)
                    .show(ui, |ui| {
                        egui::ScrollArea::horizontal()
                            .id_salt("code-scroll")
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(text)
                                            .font(egui::FontId::new(
                                                FONT_SIZE,
                                                FontFamily::Monospace,
                                            ))
                                            .line_height(Some(LINE_HEIGHT))
                                            .color(colors.foreground),
                                    )
                                    .selectable(true)
                                    .extend(),
                                );
                            });
                    });
                ui.add_space(PARAGRAPH_MARGIN);
            }
            Block::Table {
                columns,
                header,
                rows,
            } => {
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                    let mut cells = Vec::new();
                    egui::Grid::new("table")
                        .spacing(egui::Vec2::ZERO)
                        .show(ui, |ui| {
                            for (index, row) in std::iter::once(header)
                                .chain(rows.iter())
                                .filter(|row| !row.is_empty())
                                .enumerate()
                            {
                                let mut row_cells = Vec::new();
                                for (column, contents) in row.iter().enumerate() {
                                    let align = match columns.get(column) {
                                        Some(Alignment::Center) => egui::Align::Center,
                                        Some(Alignment::Right) => egui::Align::Max,
                                        _ => egui::Align::Min,
                                    };
                                    ui.with_layout(egui::Layout::top_down(align), |ui| {
                                        let output = egui::Frame::new()
                                            .inner_margin(CELL_PADDING)
                                            .show(ui, |ui| {
                                                inline_contents(
                                                    ui,
                                                    contents,
                                                    appearance,
                                                    colors,
                                                    provider,
                                                    FONT_SIZE,
                                                    LINE_HEIGHT,
                                                    !header.is_empty() && index == 0,
                                                );
                                            });
                                        row_cells.push(output.response.rect);
                                    });
                                }
                                cells.push(row_cells);
                                ui.end_row();
                            }
                        });
                    let count = cells.iter().map(Vec::len).max().unwrap_or_default();
                    let columns = (0..count)
                        .map(|column| {
                            cells
                                .iter()
                                .filter_map(|row| row.get(column).copied())
                                .reduce(|left, right| left.union(right))
                        })
                        .collect::<Vec<_>>();
                    let mut edges = HashSet::new();
                    for row in &cells {
                        let Some(bounds) =
                            row.iter().copied().reduce(|left, right| left.union(right))
                        else {
                            continue;
                        };
                        for (column, _) in row.iter().enumerate() {
                            let Some(column) = columns[column] else {
                                continue;
                            };
                            let left = column.left();
                            let right = column.right();
                            let top = bounds.top();
                            let bottom = bounds.bottom();
                            for points in [
                                [egui::pos2(left, top), egui::pos2(right, top)],
                                [egui::pos2(left, bottom), egui::pos2(right, bottom)],
                                [egui::pos2(left, top), egui::pos2(left, bottom)],
                                [egui::pos2(right, top), egui::pos2(right, bottom)],
                            ] {
                                let key = [
                                    points[0].x.to_bits(),
                                    points[0].y.to_bits(),
                                    points[1].x.to_bits(),
                                    points[1].y.to_bits(),
                                ];
                                if edges.insert(key) {
                                    ui.painter().line_segment(
                                        points,
                                        Stroke::new(CELL_BORDER, colors.border),
                                    );
                                }
                            }
                        }
                    }
                });
            }
            Block::Rule => {
                ui.add_space(RULE_MARGIN);
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), CELL_BORDER),
                    egui::Sense::hover(),
                );
                ui.painter().hline(
                    rect.x_range(),
                    rect.center().y,
                    Stroke::new(CELL_BORDER, colors.border),
                );
                ui.add_space(RULE_MARGIN);
            }
        });
    }
}

pub(super) fn show(
    ui: &mut Ui,
    document: &RichDocument,
    appearance: &EditorAppearance,
    colors: Colors,
    provider: &mut (impl Provider + ?Sized),
) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        blocks(ui, &document.blocks, appearance, colors, provider);
    });
}

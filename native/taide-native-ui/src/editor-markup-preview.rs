use std::collections::HashSet;

use egui::{FontFamily, Margin, RichText, Stroke, Ui};
use taide_native_editor::documentation::{Alignment, Block, Inline, RichDocument};

use super::{Colors, Provider, inlines};
use crate::editor_surface::EditorAppearance;

#[path = "editor-markup-table.rs"]
mod table;

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
const CJK_BREAK_RANGES: &[std::ops::RangeInclusive<char>] = &[
    '\u{4e00}'..='\u{9fff}',
    '\u{3400}'..='\u{4dbf}',
    '\u{2b740}'..='\u{2b81f}',
    '\u{3040}'..='\u{309f}',
    '\u{30a0}'..='\u{30ff}',
    '\u{ac00}'..='\u{d7a3}',
];
const CJK_UNBREAKABLE_START: &str = ")]｝〕〉》」』】〙〗〟'\"｠»ヽヾーァィゥェォッャュョヮヵヶぁぃぅぇぉっゃゅょゎゕゖㇰㇱㇲㇳㇴㇵㇶㇷㇸㇹㇺㇻㇼㇽㇾㇿ々〻‐゠–〜?!‼⁇⁈⁉・、:;,。.";

fn text_widths(ui: &Ui, job: egui::text::LayoutJob) -> (f32, f32) {
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let mut minimum = 0.0f32;
    for row in &galley.rows {
        let mut start = 0.0;
        for (index, glyph) in row.glyphs.iter().enumerate() {
            let is_space = glyph.chr.is_whitespace() && !matches!(glyph.chr, '\u{a0}' | '\u{202f}');
            let is_cjk = CJK_BREAK_RANGES
                .iter()
                .any(|range| range.contains(&glyph.chr))
                && row
                    .glyphs
                    .get(index + 1)
                    .is_none_or(|next| !CJK_UNBREAKABLE_START.contains(next.chr));
            if is_space {
                minimum = minimum.max(glyph.pos.x - start);
                start = glyph.max_x();
            } else if is_cjk || glyph.chr == '-' || glyph.chr.is_ascii_punctuation() {
                minimum = minimum.max(glyph.max_x() - start);
                start = glyph.max_x();
            }
        }
        if let Some(last) = row.glyphs.last() {
            minimum = minimum.max(last.max_x() - start)
        }
    }
    (minimum, galley.size().x)
}

fn cell_widths(
    ui: &Ui,
    cell: &[Inline],
    appearance: &EditorAppearance,
    provider: &(impl Provider + ?Sized),
    bold: bool,
) -> (f32, f32) {
    let padding = f32::from(CELL_PADDING.left) + f32::from(CELL_PADDING.right);
    let code_padding = f32::from(INLINE_CODE_PADDING.left) + f32::from(INLINE_CODE_PADDING.right);
    let (job, _) = super::inline_job(ui, cell, appearance, FONT_SIZE, bold);
    let (mut min, mut max) = text_widths(ui, job);
    for inline in cell {
        match inline {
            Inline::Image {
                source,
                alt,
                dimensions,
                ..
            } => {
                let (image_min, image_max) = provider
                    .image_size(source, *dimensions)
                    .map(|size| (size.x, size.x))
                    .unwrap_or_else(|| {
                        text_widths(
                            ui,
                            egui::text::LayoutJob::simple(
                                alt.clone(),
                                egui::FontId::proportional(FONT_SIZE),
                                ui.visuals().text_color(),
                                f32::INFINITY,
                            ),
                        )
                    });
                min = min.max(image_min);
                max += image_max;
            }
            Inline::Text(span) if span.style.code => {
                min += code_padding;
                max += code_padding;
            }
            _ => {}
        }
    }
    (min + padding, max + padding)
}

fn table_widths(
    ui: &Ui,
    header: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
    appearance: &EditorAppearance,
    provider: &(impl Provider + ?Sized),
) -> Vec<f32> {
    let count = std::iter::once(header)
        .chain(rows.iter().map(Vec::as_slice))
        .map(<[Vec<Inline>]>::len)
        .max()
        .unwrap_or_default();
    let mut minimum = vec![0.0f32; count];
    let mut preferred = minimum.clone();
    for (row_index, row) in std::iter::once(header)
        .chain(rows.iter().map(Vec::as_slice))
        .enumerate()
    {
        for (column, cell) in row.iter().enumerate() {
            let (min, max) = cell_widths(
                ui,
                cell,
                appearance,
                provider,
                !header.is_empty() && row_index == 0,
            );
            minimum[column] = minimum[column].max(min);
            preferred[column] = preferred[column].max(max);
        }
    }
    let min = minimum.iter().sum::<f32>();
    let max = preferred.iter().sum::<f32>();
    let available = ui.available_width();
    if max <= available {
        return preferred;
    }
    if min >= available || max <= min {
        return minimum;
    }
    let fraction = (available - min) / (max - min);
    minimum
        .into_iter()
        .zip(preferred)
        .map(|(min, max)| min + (max - min) * fraction)
        .collect()
}

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

fn table_cells(
    ui: &mut Ui,
    columns: &[Alignment],
    header: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
    widths: &[f32],
    appearance: &EditorAppearance,
    colors: Colors,
    provider: &mut (impl Provider + ?Sized),
    pass: &str,
) -> Vec<Vec<egui::Rect>> {
    let mut cells = Vec::new();
    egui::Grid::new("table")
        .spacing(egui::Vec2::ZERO)
        .show(ui, |ui| {
            for (index, row) in std::iter::once(header)
                .chain(rows.iter().map(Vec::as_slice))
                .filter(|row| !row.is_empty())
                .enumerate()
            {
                let mut row_cells = Vec::new();
                for (column, contents) in row.iter().enumerate() {
                    let start = ui.cursor().min;
                    let align = match columns.get(column) {
                        Some(Alignment::Center) => egui::Align::Center,
                        Some(Alignment::Right) => egui::Align::Max,
                        _ => egui::Align::Min,
                    };
                    ui.with_layout(egui::Layout::top_down(align), |ui| {
                        ui.push_id(pass, |ui| {
                            egui::Frame::new()
                                .inner_margin(CELL_PADDING)
                                .show(ui, |ui| {
                                    if let Some(width) = widths.get(column) {
                                        ui.set_width(
                                            (*width
                                                - f32::from(CELL_PADDING.left)
                                                - f32::from(CELL_PADDING.right))
                                            .max(0.0),
                                        );
                                    }
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
                        });
                    });
                    row_cells.push(egui::Rect::from_min_max(
                        start,
                        egui::pos2(ui.cursor().left(), start.y),
                    ));
                }
                ui.end_row();
                for cell in &mut row_cells {
                    cell.max.y = ui.cursor().top()
                }
                cells.push(row_cells);
            }
        });
    cells
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
                                    } else if let Some(marker) = item.marker(*start, index) {
                                        ui.label(RichText::new(marker).size(FONT_SIZE));
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
                layout,
            } => {
                if let Some(layout) = layout {
                    table::show(ui, rows, layout, appearance, colors, provider);
                    return;
                }
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                    let image_sizes = std::iter::once(header)
                        .chain(rows)
                        .flatten()
                        .flatten()
                        .filter_map(|inline| {
                            let Inline::Image {
                                source, dimensions, ..
                            } = inline
                            else {
                                return None;
                            };
                            Some(
                                provider
                                    .image_size(source, *dimensions)
                                    .map(|size| [size.x.to_bits(), size.y.to_bits()]),
                            )
                        })
                        .collect::<Vec<_>>();
                    let widths = table_widths(ui, header, rows, appearance, provider);
                    let metrics = widths
                        .iter()
                        .map(|width| width.to_bits())
                        .collect::<Vec<_>>();
                    let input = egui::Id::new((
                        columns,
                        header,
                        rows,
                        ui.available_width().to_bits(),
                        ui.ctx().pixels_per_point().to_bits(),
                        &appearance.font.family,
                        crate::font_families::semibold(ui),
                        image_sizes,
                        metrics,
                    ));
                    let marker = ui.make_persistent_id("table-input");
                    let changed = ui.ctx().data_mut(|data| {
                        let changed = data.get_temp::<egui::Id>(marker) != Some(input);
                        data.insert_temp(marker, input);
                        changed
                    });
                    if changed {
                        let mut measurement = ui.new_child(
                            egui::UiBuilder::new()
                                .id(ui.id())
                                .max_rect(ui.available_rect_before_wrap())
                                .sizing_pass()
                                .invisible(),
                        );
                        table_cells(
                            &mut measurement,
                            columns,
                            header,
                            rows,
                            &widths,
                            appearance,
                            colors,
                            provider,
                            "measure-width",
                        );
                        let mut measurement = ui.new_child(
                            egui::UiBuilder::new()
                                .id(ui.id())
                                .max_rect(ui.available_rect_before_wrap())
                                .invisible(),
                        );
                        table_cells(
                            &mut measurement,
                            columns,
                            header,
                            rows,
                            &widths,
                            appearance,
                            colors,
                            provider,
                            "measure-height",
                        );
                    }
                    let mut table_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .id(ui.id())
                            .max_rect(ui.available_rect_before_wrap()),
                    );
                    let cells = table_cells(
                        &mut table_ui,
                        columns,
                        header,
                        rows,
                        &widths,
                        appearance,
                        colors,
                        provider,
                        "contents",
                    );
                    ui.advance_cursor_after_rect(table_ui.min_rect());
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

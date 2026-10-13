use std::collections::HashMap;
use std::sync::Arc;

use egui::{Id, Rect, Ui};
use taide_native_editor::documentation::{Alignment, Block, Inline, TableLayout};

use super::{CELL_BORDER, CELL_PADDING, FONT_SIZE, LINE_HEIGHT, cell_widths, inline_contents};
use crate::editor_markup::{Colors, Provider, TextWeight};
use crate::editor_surface::EditorAppearance;

const CENTER_FRACTION: f32 = 0.5;

struct Geometry {
    input: Id,
    columns: Vec<f32>,
    rows: Vec<f32>,
    caption_height: f32,
    cell_heights: Vec<Vec<f32>>,
    width: f32,
}

fn paint_edges(ui: &Ui, groups: HashMap<u32, Vec<(f32, f32)>>, vertical: bool, colors: Colors) {
    for (coordinate, mut intervals) in groups {
        intervals.sort_by(|left, right| left.0.total_cmp(&right.0));
        let mut merged = Vec::<(f32, f32)>::new();
        for (start, end) in intervals {
            if start >= end {
                continue;
            }
            if let Some(last) = merged.last_mut()
                && start <= last.1
            {
                last.1 = last.1.max(end);
                continue;
            }
            merged.push((start, end));
        }
        for (start, end) in merged {
            let coordinate = f32::from_bits(coordinate);
            let points = if vertical {
                [egui::pos2(coordinate, start), egui::pos2(coordinate, end)]
            } else {
                [egui::pos2(start, coordinate), egui::pos2(end, coordinate)]
            };
            ui.painter()
                .line_segment(points, egui::Stroke::new(CELL_BORDER, colors.border));
        }
    }
}

fn grow(tracks: &mut [f32], required: f32) {
    if tracks.is_empty() {
        return;
    }
    let deficit = required - tracks.iter().sum::<f32>();
    if deficit <= 0.0 {
        return;
    }
    let increment = deficit / tracks.len() as f32;
    for track in tracks {
        *track += increment
    }
}

fn offsets(tracks: &[f32]) -> Vec<f32> {
    let mut total = 0.0;
    std::iter::once(0.0)
        .chain(tracks.iter().map(|size| {
            total += size;
            total
        }))
        .collect()
}

fn cell_sizes(
    ui: &Ui,
    values: &[Inline],
    blocks: Option<&[Block]>,
    appearance: &EditorAppearance,
    provider: &(impl Provider + ?Sized),
) -> (f32, f32) {
    if let Some(blocks) = blocks {
        let padding = f32::from(CELL_PADDING.left) + f32::from(CELL_PADDING.right);
        let (min, max) = super::block_widths(ui, blocks, appearance, provider);
        return (min + padding, max + padding);
    }
    cell_widths(ui, values, appearance, provider, false)
}

fn inline_image_metrics(
    contents: &[Inline],
    provider: &(impl Provider + ?Sized),
    sizes: &mut Vec<Option<[u32; 2]>>,
) {
    for value in contents {
        if let Inline::Image {
            source, dimensions, ..
        } = value
        {
            sizes.push(
                provider
                    .image_size(source, *dimensions)
                    .map(|size| [size.x.to_bits(), size.y.to_bits()]),
            );
        }
    }
}

fn block_image_metrics(
    contents: &[Block],
    provider: &(impl Provider + ?Sized),
    sizes: &mut Vec<Option<[u32; 2]>>,
) {
    for block in contents {
        match block {
            Block::Inline(values)
            | Block::Paragraph(values)
            | Block::Heading {
                contents: values, ..
            } => inline_image_metrics(values, provider, sizes),
            Block::Quote(values) => block_image_metrics(values, provider, sizes),
            Block::List { items, .. } => {
                for item in items {
                    block_image_metrics(&item.blocks, provider, sizes)
                }
            }
            Block::Table {
                header,
                rows,
                layout,
                ..
            } => {
                for values in header.iter().chain(rows.iter().flatten()) {
                    inline_image_metrics(values, provider, sizes)
                }
                if let Some(layout) = layout {
                    inline_image_metrics(&layout.caption, provider, sizes);
                    for cell in layout.cells.iter().flatten() {
                        if let Some(blocks) = &cell.blocks {
                            block_image_metrics(blocks, provider, sizes)
                        }
                    }
                }
            }
            Block::Code { .. } | Block::Rule => {}
        }
    }
}

fn contents(
    ui: &mut Ui,
    id: Id,
    rect: Rect,
    values: &[Inline],
    blocks: Option<&[Block]>,
    alignment: Alignment,
    appearance: &EditorAppearance,
    colors: Colors,
    provider: &mut (impl Provider + ?Sized),
    margin: egui::Margin,
    measured: bool,
) -> f32 {
    let padding = f32::from(CELL_PADDING.left) + f32::from(CELL_PADDING.right);
    let (_, preferred) = cell_sizes(ui, values, blocks, appearance, provider);
    let available = (rect.width() - f32::from(margin.left) - f32::from(margin.right)).max(0.0);
    let width = (preferred - padding).min(available).max(0.0);
    let offset = match alignment {
        Alignment::Center => (available - width) * CENTER_FRACTION,
        Alignment::Right => available - width,
        _ => 0.0,
    };
    let mut builder = egui::UiBuilder::new().id(id).max_rect(Rect::from_min_size(
        rect.min + egui::vec2(offset, 0.0),
        egui::vec2(
            width + f32::from(margin.left) + f32::from(margin.right),
            f32::INFINITY,
        ),
    ));
    if measured {
        builder = builder.invisible()
    }
    let mut child = ui.new_child(builder);
    if !measured {
        child.set_clip_rect(rect.intersect(ui.clip_rect()))
    }
    let output = egui::Frame::new()
        .inner_margin(margin)
        .show(&mut child, |ui| {
            ui.set_width(width);
            if let Some(blocks) = blocks {
                super::blocks(ui, blocks, appearance, colors, provider);
                return;
            }
            inline_contents(
                ui,
                values,
                appearance,
                colors,
                provider,
                FONT_SIZE,
                LINE_HEIGHT,
                TextWeight::Normal,
            );
        });
    output.response.rect.height()
}

pub(super) fn intrinsic_widths(
    ui: &Ui,
    rows: &[Vec<Vec<Inline>>],
    layout: &TableLayout,
    appearance: &EditorAppearance,
    provider: &(impl Provider + ?Sized),
) -> Option<(Vec<f32>, Vec<f32>)> {
    if layout.cells.len() != rows.len() || layout.row_count != rows.len() {
        return None;
    }
    let mut minimum = vec![0.0f32; layout.column_count];
    let mut preferred = minimum.clone();
    for (row, cells) in rows.iter().zip(&layout.cells) {
        if row.len() != cells.len() {
            return None;
        }
        for (values, cell) in row.iter().zip(cells) {
            if cell.column_span == 0
                || cell.row_span == 0
                || cell
                    .column
                    .checked_add(cell.column_span)
                    .is_none_or(|end| end > layout.column_count)
            {
                return None;
            }
            let sizes = cell_sizes(ui, values, cell.blocks.as_deref(), appearance, provider);
            if cell.column_span == 1 {
                minimum[cell.column] = minimum[cell.column].max(sizes.0);
                preferred[cell.column] = preferred[cell.column].max(sizes.1);
            }
        }
    }
    for (row_index, (row, cells)) in rows.iter().zip(&layout.cells).enumerate() {
        for (values, cell) in row.iter().zip(cells) {
            if row_index
                .checked_add(cell.row_span)
                .is_none_or(|end| end > layout.row_count)
            {
                return None;
            }
            if cell.column_span > 1 {
                let sizes = cell_sizes(ui, values, cell.blocks.as_deref(), appearance, provider);
                grow(
                    &mut minimum[cell.column..cell.column + cell.column_span],
                    sizes.0,
                );
                grow(
                    &mut preferred[cell.column..cell.column + cell.column_span],
                    sizes.1,
                );
            }
        }
    }
    let caption = cell_widths(ui, &layout.caption, appearance, provider, false);
    let padding = f32::from(CELL_PADDING.left) + f32::from(CELL_PADDING.right);
    grow(&mut minimum, (caption.0 - padding).max(0.0));
    grow(&mut preferred, (caption.1 - padding).max(0.0));
    Some((minimum, preferred))
}

pub(super) fn show(
    ui: &mut Ui,
    rows: &[Vec<Vec<Inline>>],
    layout: &TableLayout,
    appearance: &EditorAppearance,
    colors: Colors,
    provider: &mut (impl Provider + ?Sized),
) {
    let Some((minimum, preferred)) = intrinsic_widths(ui, rows, layout, appearance, provider)
    else {
        return;
    };
    let caption = cell_widths(ui, &layout.caption, appearance, provider, false);
    let padding = f32::from(CELL_PADDING.left) + f32::from(CELL_PADDING.right);
    let mut metrics = Vec::new();
    let mut image_sizes = Vec::new();
    for (row, cells) in rows.iter().zip(&layout.cells) {
        for (values, cell) in row.iter().zip(cells) {
            let sizes = cell_sizes(ui, values, cell.blocks.as_deref(), appearance, provider);
            metrics.push([sizes.0.to_bits(), sizes.1.to_bits()]);
            inline_image_metrics(values, provider, &mut image_sizes);
            if let Some(blocks) = &cell.blocks {
                block_image_metrics(blocks, provider, &mut image_sizes)
            }
        }
    }
    inline_image_metrics(&layout.caption, provider, &mut image_sizes);
    let min = minimum.iter().sum::<f32>();
    let max = preferred.iter().sum::<f32>();
    let available = ui.available_width();
    let widths = if max <= available {
        preferred
    } else if min >= available || max <= min {
        minimum
    } else {
        let fraction = (available - min) / (max - min);
        minimum
            .into_iter()
            .zip(preferred)
            .map(|(min, max)| min + (max - min) * fraction)
            .collect()
    };
    let input = Id::new((
        rows,
        layout,
        ui.available_width().to_bits(),
        ui.ctx().pixels_per_point().to_bits(),
        &appearance.font.family,
        metrics,
        image_sizes,
        [caption.0.to_bits(), caption.1.to_bits()],
    ));
    let owner = ui.make_persistent_id("html-table");
    let cached = ui
        .ctx()
        .data(|data| data.get_temp::<Arc<Geometry>>(owner))
        .filter(|geometry| geometry.input == input);
    let origin = ui.cursor().min;
    let geometry = cached.unwrap_or_else(|| {
        let columns = offsets(&widths);
        let mut heights = vec![0.0f32; layout.row_count];
        let mut measured = Vec::new();
        let mut cell_heights = Vec::new();
        for (row_index, (row, cells)) in rows.iter().zip(&layout.cells).enumerate() {
            let mut row_heights = Vec::new();
            for (index, (values, cell)) in row.iter().zip(cells).enumerate() {
                let width = columns[cell.column + cell.column_span] - columns[cell.column];
                let rect = Rect::from_min_size(origin, egui::vec2(width, f32::INFINITY));
                let height = contents(
                    ui,
                    owner.with(("measure", row_index, index)),
                    rect,
                    values,
                    cell.blocks.as_deref(),
                    cell.alignment,
                    appearance,
                    colors,
                    provider,
                    CELL_PADDING,
                    true,
                );
                if cell.row_span == 1 {
                    heights[row_index] = heights[row_index].max(height)
                }
                measured.push((row_index, cell.row_span, height));
                row_heights.push(height);
            }
            cell_heights.push(row_heights);
        }
        for (row, span, height) in measured {
            if span > 1 {
                grow(&mut heights[row..row + span], height)
            }
        }
        let width = if layout.column_count == 0 {
            (caption.1 - padding)
                .min(available)
                .max((caption.0 - padding).max(0.0))
        } else {
            columns.last().copied().unwrap_or_default()
        };
        let caption_height = if layout.caption.is_empty() {
            0.0
        } else {
            contents(
                ui,
                owner.with("caption-measure"),
                Rect::from_min_size(origin, egui::vec2(width, f32::INFINITY)),
                &layout.caption,
                None,
                Alignment::Center,
                appearance,
                colors,
                provider,
                egui::Margin::ZERO,
                true,
            )
        };
        let geometry = Arc::new(Geometry {
            input,
            columns,
            rows: offsets(&heights),
            caption_height,
            cell_heights,
            width,
        });
        ui.ctx()
            .data_mut(|data| data.insert_temp(owner, geometry.clone()));
        geometry
    });
    let width = geometry.width;
    if !layout.caption.is_empty() {
        contents(
            ui,
            owner.with("caption"),
            Rect::from_min_size(origin, egui::vec2(width, geometry.caption_height)),
            &layout.caption,
            None,
            Alignment::Center,
            appearance,
            colors,
            provider,
            egui::Margin::ZERO,
            false,
        );
    }
    let origin = origin + egui::vec2(0.0, geometry.caption_height);
    let mut horizontal_edges = HashMap::<u32, Vec<(f32, f32)>>::new();
    let mut vertical_edges = HashMap::<u32, Vec<(f32, f32)>>::new();
    for (row_index, (row, cells)) in rows.iter().zip(&layout.cells).enumerate() {
        for (index, (values, cell)) in row.iter().zip(cells).enumerate() {
            let column_end = cell.column + cell.column_span;
            let row_end = row_index + cell.row_span;
            let rect = Rect::from_min_max(
                origin + egui::vec2(geometry.columns[cell.column], geometry.rows[row_index]),
                origin + egui::vec2(geometry.columns[column_end], geometry.rows[row_end]),
            );
            let offset = (rect.height() - geometry.cell_heights[row_index][index]).max(0.0)
                * CENTER_FRACTION;
            let content_rect = Rect::from_min_max(rect.min + egui::vec2(0.0, offset), rect.max);
            contents(
                ui,
                owner.with(("contents", row_index, index)),
                content_rect,
                values,
                cell.blocks.as_deref(),
                cell.alignment,
                appearance,
                colors,
                provider,
                CELL_PADDING,
                false,
            );
            for y in [rect.top(), rect.bottom()] {
                horizontal_edges
                    .entry(y.to_bits())
                    .or_default()
                    .push((rect.left(), rect.right()));
            }
            for x in [rect.left(), rect.right()] {
                vertical_edges
                    .entry(x.to_bits())
                    .or_default()
                    .push((rect.top(), rect.bottom()));
            }
        }
    }
    paint_edges(ui, horizontal_edges, false, colors);
    paint_edges(ui, vertical_edges, true, colors);
    let height = geometry.caption_height + geometry.rows.last().copied().unwrap_or_default();
    ui.advance_cursor_after_rect(Rect::from_min_size(
        ui.cursor().min,
        egui::vec2(width, height),
    ));
}

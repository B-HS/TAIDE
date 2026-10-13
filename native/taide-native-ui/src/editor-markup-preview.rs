use std::collections::HashSet;

use egui::{FontFamily, Margin, RichText, Stroke, Ui};
use taide_native_editor::documentation::{Alignment, Block, Inline, RichDocument};

use super::{Colors, Provider, TextWeight, inlines};
use crate::editor_surface::EditorAppearance;

#[path = "editor-markup-table.rs"]
mod table;

#[path = "editor-markup-flow.rs"]
mod flow;

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
pub(super) const TASK_ICON_SIZE: f32 = 13.0;
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

fn inline_widths(
    ui: &Ui,
    cell: &[Inline],
    appearance: &EditorAppearance,
    provider: &(impl Provider + ?Sized),
    font_size: f32,
    weight: TextWeight,
) -> (f32, f32) {
    if cell
        .iter()
        .any(|value| matches!(value, Inline::Image { .. }))
    {
        let mut result = (0.0f32, 0.0f32);
        let mut start = 0;
        for (index, value) in cell.iter().enumerate() {
            let Inline::Image {
                source,
                alt,
                dimensions,
                ..
            } = value
            else {
                continue;
            };
            let before = inline_widths(
                ui,
                &cell[start..index],
                appearance,
                provider,
                font_size,
                weight,
            );
            let image = provider
                .image_size(source, *dimensions)
                .map(|size| (size.x, size.x))
                .unwrap_or_else(|| {
                    text_widths(
                        ui,
                        egui::text::LayoutJob::simple(
                            alt.clone(),
                            egui::FontId::proportional(font_size),
                            ui.visuals().text_color(),
                            f32::INFINITY,
                        ),
                    )
                });
            result = (
                result.0.max(before.0).max(image.0),
                result.1.max(before.1).max(image.1),
            );
            start = index + 1;
        }
        let after = inline_widths(ui, &cell[start..], appearance, provider, font_size, weight);
        return (result.0.max(after.0), result.1.max(after.1));
    }
    let code_padding = f32::from(INLINE_CODE_PADDING.left) + f32::from(INLINE_CODE_PADDING.right);
    let (job, _) = super::inline_job(ui, cell, appearance, font_size, weight, true);
    let (mut min, mut max) = text_widths(ui, job);
    for inline in cell {
        if let Inline::Text(span) = inline
            && span.style.code
        {
            min += code_padding;
            max += code_padding;
        }
    }
    (min, max)
}

fn cell_widths(
    ui: &Ui,
    cell: &[Inline],
    appearance: &EditorAppearance,
    provider: &(impl Provider + ?Sized),
    bold: bool,
) -> (f32, f32) {
    let padding = f32::from(CELL_PADDING.left) + f32::from(CELL_PADDING.right);
    let (min, max) = inline_widths(
        ui,
        cell,
        appearance,
        provider,
        FONT_SIZE,
        if bold {
            TextWeight::Bold
        } else {
            TextWeight::Normal
        },
    );
    (min + padding, max + padding)
}

fn block_widths(
    ui: &Ui,
    contents: &[Block],
    appearance: &EditorAppearance,
    provider: &(impl Provider + ?Sized),
) -> (f32, f32) {
    contents
        .iter()
        .map(|block| match block {
            Block::Inline(values) | Block::Paragraph(values) => inline_widths(
                ui,
                values,
                appearance,
                provider,
                FONT_SIZE,
                TextWeight::Normal,
            ),
            Block::Heading { level, contents } => {
                let index = usize::from(level.saturating_sub(1));
                inline_widths(
                    ui,
                    contents,
                    appearance,
                    provider,
                    HEADING_SIZES.get(index).copied().unwrap_or(FONT_SIZE),
                    if HEADING_SIZES.get(index).is_some() {
                        TextWeight::Semibold
                    } else {
                        TextWeight::Normal
                    },
                )
            }
            Block::Quote(contents) => {
                let (min, max) = block_widths(ui, contents, appearance, provider);
                (min + f32::from(QUOTE_INDENT), max + f32::from(QUOTE_INDENT))
            }
            Block::List { items, .. } => items
                .iter()
                .map(|item| {
                    let (min, max) = block_widths(ui, &item.blocks, appearance, provider);
                    if item.checked.is_some() && item.blocks.first().is_some_and(can_prefix) {
                        let values = match &item.blocks[0] {
                            Block::Inline(values) | Block::Paragraph(values) => values,
                            _ => unreachable!(),
                        };
                        let (mut job, _) = super::inline_job(
                            ui,
                            values,
                            appearance,
                            FONT_SIZE,
                            TextWeight::Normal,
                            true,
                        );
                        if let Some(section) = job.sections.first_mut() {
                            section.leading_space += TASK_ICON_SIZE + space_width(ui);
                        }
                        let first = text_widths(ui, job);
                        let rest = block_widths(ui, &item.blocks[1..], appearance, provider);
                        return (
                            first.0.max(rest.0) + f32::from(LIST_INDENT),
                            first.1.max(rest.1) + f32::from(LIST_INDENT),
                        );
                    }
                    let task_width = if item.checked.is_some() {
                        TASK_ICON_SIZE + space_width(ui)
                    } else {
                        0.0
                    };
                    let indent = f32::from(LIST_INDENT) + task_width;
                    (min + indent, max + indent)
                })
                .fold((0.0f32, 0.0f32), |(min, max), (next_min, next_max)| {
                    (min.max(next_min), max.max(next_max))
                }),
            Block::Code { text, style, .. } => {
                let width = text_widths(
                    ui,
                    egui::text::LayoutJob::simple(
                        text.clone(),
                        egui::FontId::new(
                            FONT_SIZE,
                            if style.bold {
                                crate::font_families::code_bold(ui)
                            } else {
                                FontFamily::Monospace
                            },
                        ),
                        ui.visuals().text_color(),
                        f32::INFINITY,
                    ),
                )
                .1 + f32::from(CODE_PADDING) * 2.0;
                (width, width)
            }
            Block::Table {
                header,
                rows,
                layout,
                ..
            } => {
                if let Some(layout) = layout {
                    if layout.column_count == 0 {
                        if let Some(blocks) = &layout.caption_blocks {
                            return block_widths(ui, blocks, appearance, provider);
                        }
                        return inline_widths(
                            ui,
                            &layout.caption,
                            appearance,
                            provider,
                            FONT_SIZE,
                            TextWeight::Normal,
                        );
                    }
                    return table::intrinsic_widths(ui, rows, layout, appearance, provider)
                        .map(|(minimum, preferred)| (minimum.iter().sum(), preferred.iter().sum()))
                        .unwrap_or_default();
                }
                let count = std::iter::once(header)
                    .chain(rows)
                    .map(Vec::len)
                    .max()
                    .unwrap_or_default();
                let mut minimum = vec![0.0f32; count];
                let mut preferred = minimum.clone();
                for (index, row) in std::iter::once(header).chain(rows).enumerate() {
                    for (column, values) in row.iter().enumerate() {
                        let (min, max) = cell_widths(ui, values, appearance, provider, index == 0);
                        minimum[column] = minimum[column].max(min);
                        preferred[column] = preferred[column].max(max);
                    }
                }
                (minimum.iter().sum(), preferred.iter().sum())
            }
            Block::Rule => (0.0, 0.0),
        })
        .fold((0.0f32, 0.0f32), |(min, max), (next_min, next_max)| {
            (min.max(next_min), max.max(next_max))
        })
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
    weight: TextWeight,
) {
    let mut appearance = appearance.clone();
    appearance.line_height = line_height;
    if contents
        .iter()
        .any(|value| matches!(value, Inline::Image { .. }))
    {
        let mut start = 0;
        for (index, value) in contents.iter().enumerate() {
            if !matches!(value, Inline::Image { .. }) {
                continue;
            }
            if flow::has_content(&contents[start..index]) {
                inline_contents(
                    ui,
                    &contents[start..index],
                    &appearance,
                    colors,
                    provider,
                    size,
                    line_height,
                    weight,
                );
            }
            let values = &contents[index..index + 1];
            let (_, preferred) = inline_widths(ui, values, &appearance, provider, size, weight);
            ui.allocate_ui_with_layout(
                egui::vec2(preferred.min(ui.available_width()), 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    inlines(
                        ui,
                        values,
                        &appearance,
                        colors,
                        provider,
                        size,
                        false,
                        true,
                        weight,
                    )
                },
            );
            start = index + 1;
        }
        if flow::has_content(&contents[start..]) {
            inline_contents(
                ui,
                &contents[start..],
                &appearance,
                colors,
                provider,
                size,
                line_height,
                weight,
            );
        }
        return;
    }
    if super::aligned::show(ui, contents, &appearance, colors, provider, size, weight) {
        return;
    }
    if ui.layout().horizontal_align() != egui::Align::Min {
        let (_, preferred) = inline_widths(ui, contents, &appearance, provider, size, weight);
        ui.allocate_ui_with_layout(
            egui::vec2(preferred.min(ui.available_width()), 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                inlines(
                    ui,
                    contents,
                    &appearance,
                    colors,
                    provider,
                    size,
                    false,
                    true,
                    weight,
                )
            },
        );
        return;
    }
    inlines(
        ui,
        contents,
        &appearance,
        colors,
        provider,
        size,
        false,
        true,
        weight,
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
                                        if !header.is_empty() && index == 0 {
                                            TextWeight::Bold
                                        } else {
                                            TextWeight::Normal
                                        },
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
    flow_blocks(ui, contents, appearance, colors, provider, false);
}

pub(super) fn space_width(ui: &Ui) -> f32 {
    ui.fonts_mut(|fonts| {
        fonts
            .layout_no_wrap(
                " ".into(),
                egui::FontId::proportional(FONT_SIZE),
                ui.visuals().text_color(),
            )
            .size()
            .x
    })
}

fn can_prefix(block: &Block) -> bool {
    match block {
        Block::Inline(values) | Block::Paragraph(values) => {
            flow::has_content(values)
                && values.iter().all(|value| match value {
                    Inline::Text(span) => !span.style.code,
                    Inline::Break => true,
                    Inline::Image { .. } => false,
                })
        }
        _ => false,
    }
}

fn first_baseline(ui: &Ui, contents: &[Block], appearance: &EditorAppearance, width: f32) -> f32 {
    let block = contents.iter().find(|block| !flow::edges(block).through);
    let (mut job, line_height, inset) = match block {
        Some(Block::Inline(values) | Block::Paragraph(values)) => {
            let line_height = if matches!(block, Some(Block::Paragraph(_))) {
                PARAGRAPH_LINE_HEIGHT
            } else {
                LINE_HEIGHT
            };
            (
                super::inline_job(ui, values, appearance, FONT_SIZE, TextWeight::Normal, true).0,
                line_height,
                0.0,
            )
        }
        Some(Block::Heading { level, contents }) => {
            let index = usize::from(level.saturating_sub(1));
            let size = HEADING_SIZES.get(index).copied().unwrap_or(FONT_SIZE);
            let weight = if HEADING_SIZES.get(index).is_some() {
                TextWeight::Semibold
            } else {
                TextWeight::Normal
            };
            (
                super::inline_job(ui, contents, appearance, size, weight, true).0,
                HEADING_LINE_HEIGHTS
                    .get(index)
                    .copied()
                    .unwrap_or(LINE_HEIGHT),
                0.0,
            )
        }
        Some(Block::Code { text, style, .. }) => (
            egui::text::LayoutJob::simple(
                text.clone(),
                egui::FontId::new(
                    FONT_SIZE,
                    if style.bold {
                        crate::font_families::code_bold(ui)
                    } else {
                        FontFamily::Monospace
                    },
                ),
                ui.visuals().text_color(),
                f32::INFINITY,
            ),
            LINE_HEIGHT,
            f32::from(CODE_PADDING),
        ),
        Some(Block::Quote(contents)) => {
            return first_baseline(
                ui,
                contents,
                appearance,
                (width - f32::from(QUOTE_INDENT)).max(0.0),
            );
        }
        Some(Block::List { items, .. }) => {
            return items
                .first()
                .map(|item| {
                    first_baseline(
                        ui,
                        &item.blocks,
                        appearance,
                        (width - f32::from(LIST_INDENT)).max(0.0),
                    )
                })
                .unwrap_or_default();
        }
        _ => (
            egui::text::LayoutJob::simple(
                " ".into(),
                egui::FontId::proportional(FONT_SIZE),
                ui.visuals().text_color(),
                width,
            ),
            LINE_HEIGHT,
            0.0,
        ),
    };
    if job.text.is_empty() {
        job = egui::text::LayoutJob::simple(
            " ".into(),
            egui::FontId::proportional(FONT_SIZE),
            ui.visuals().text_color(),
            width,
        );
    }
    for section in &mut job.sections {
        section.format.line_height = Some(line_height);
    }
    job.wrap.max_width = width;
    inset
        + ui.fonts_mut(|fonts| fonts.layout_job(job))
            .rows
            .first()
            .and_then(|row| row.glyphs.first().map(|glyph| row.pos.y + glyph.pos.y))
            .unwrap_or_default()
}

fn flow_blocks(
    ui: &mut Ui,
    contents: &[Block],
    appearance: &EditorAppearance,
    colors: Colors,
    provider: &mut (impl Provider + ?Sized),
    trim_edges: bool,
) {
    flow_blocks_with_prefix(ui, contents, appearance, colors, provider, trim_edges, None);
}

fn flow_blocks_with_prefix(
    ui: &mut Ui,
    contents: &[Block],
    appearance: &EditorAppearance,
    colors: Colors,
    provider: &mut (impl Provider + ?Sized),
    trim_edges: bool,
    prefix: Option<bool>,
) {
    let mut previous_margin = 0.0f32;
    let mut is_first = true;
    for (index, block) in contents.iter().enumerate() {
        let edges = flow::edges(block);
        if edges.through {
            previous_margin = previous_margin.max(edges.top).max(edges.bottom);
            continue;
        }
        if !is_first || !trim_edges {
            ui.add_space(previous_margin.max(edges.top));
        }
        is_first = false;
        ui.push_id(index, |ui| match block {
            Block::Inline(contents) => {
                if index == 0
                    && let Some(checked) = prefix
                {
                    let mut appearance = appearance.clone();
                    appearance.line_height = LINE_HEIGHT;
                    super::aligned::show_with_prefix(
                        ui,
                        contents,
                        &appearance,
                        colors,
                        provider,
                        FONT_SIZE,
                        TextWeight::Normal,
                        Some(checked),
                    );
                    return;
                }
                inline_contents(
                    ui,
                    contents,
                    appearance,
                    colors,
                    provider,
                    FONT_SIZE,
                    LINE_HEIGHT,
                    TextWeight::Normal,
                );
            }
            Block::Paragraph(contents) => {
                if index == 0
                    && let Some(checked) = prefix
                {
                    let mut appearance = appearance.clone();
                    appearance.line_height = PARAGRAPH_LINE_HEIGHT;
                    super::aligned::show_with_prefix(
                        ui,
                        contents,
                        &appearance,
                        colors,
                        provider,
                        FONT_SIZE,
                        TextWeight::Normal,
                        Some(checked),
                    );
                    return;
                }
                inline_contents(
                    ui,
                    contents,
                    appearance,
                    colors,
                    provider,
                    FONT_SIZE,
                    PARAGRAPH_LINE_HEIGHT,
                    TextWeight::Normal,
                );
            }
            Block::Heading { level, contents } => {
                let index = usize::from(level.saturating_sub(1));
                let size = HEADING_SIZES.get(index).copied().unwrap_or(FONT_SIZE);
                let line_height = HEADING_LINE_HEIGHTS
                    .get(index)
                    .copied()
                    .unwrap_or(LINE_HEIGHT);
                inline_contents(
                    ui,
                    contents,
                    appearance,
                    colors,
                    provider,
                    size,
                    line_height,
                    if HEADING_SIZES.get(index).is_some() {
                        TextWeight::Semibold
                    } else {
                        TextWeight::Normal
                    },
                );
            }
            Block::Quote(contents) => {
                let output = egui::Frame::new()
                    .inner_margin(Margin {
                        left: QUOTE_INDENT,
                        ..Margin::ZERO
                    })
                    .show(ui, |ui| {
                        ui.multiply_opacity(QUOTE_OPACITY);
                        flow_blocks(ui, contents, appearance, colors, provider, true);
                    });
                ui.painter().vline(
                    output.response.rect.left(),
                    output.response.rect.y_range(),
                    Stroke::new(QUOTE_BORDER, colors.border.gamma_multiply(QUOTE_OPACITY)),
                );
            }
            Block::List {
                start,
                items,
                style,
            } => {
                let mut previous = 0.0f32;
                for (index, item) in items.iter().enumerate() {
                    let item_edges = flow::group(&item.blocks);
                    if index > 0 {
                        ui.add_space(previous.max(item_edges.top))
                    }
                    ui.push_id(index, |ui| {
                        egui::Frame::new()
                            .inner_margin(Margin {
                                left: LIST_INDENT,
                                ..Margin::ZERO
                            })
                            .show(ui, |ui| {
                                let origin = ui.available_rect_before_wrap().min;
                                let width = ui.available_width();
                                ui.set_width(width);
                                if let Some(marker) = item.marker(*start, index) {
                                    let font = egui::FontId::new(
                                        FONT_SIZE,
                                        if style.bold {
                                            crate::font_families::bold(ui)
                                        } else {
                                            FontFamily::Proportional
                                        },
                                    );
                                    let mut job = egui::text::LayoutJob::simple(
                                        marker,
                                        font,
                                        colors.foreground,
                                        f32::INFINITY,
                                    );
                                    job.sections[0].format.line_height = Some(LINE_HEIGHT);
                                    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
                                    let baseline = galley.rows[0]
                                        .glyphs
                                        .first()
                                        .map_or(0.0, |glyph| glyph.pos.y);
                                    let top = origin.y
                                        + first_baseline(ui, &item.blocks, appearance, width)
                                        - baseline;
                                    let mut marker_ui = ui.new_child(
                                        egui::UiBuilder::new()
                                            .id_salt("marker")
                                            .max_rect(egui::Rect::from_min_size(
                                                egui::pos2(
                                                    origin.x - space_width(ui) - galley.size().x,
                                                    top,
                                                ),
                                                galley.size(),
                                            ))
                                            .layout(egui::Layout::top_down(egui::Align::Min)),
                                    );
                                    marker_ui.add(egui::Label::new(galley));
                                }
                                if let Some(checked) = item.checked {
                                    if item.blocks.first().is_some_and(can_prefix) {
                                        flow_blocks_with_prefix(
                                            ui,
                                            &item.blocks,
                                            appearance,
                                            colors,
                                            provider,
                                            true,
                                            Some(checked),
                                        );
                                        return;
                                    }
                                    let (_, preferred) =
                                        block_widths(ui, &item.blocks, appearance, provider);
                                    let task_width =
                                        (preferred + TASK_ICON_SIZE + space_width(ui)).min(width);
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(task_width, 0.0),
                                        egui::Layout::left_to_right(egui::Align::Center),
                                        |ui| {
                                            ui.spacing_mut().item_spacing.x = space_width(ui);
                                            ui.spacing_mut().icon_width = TASK_ICON_SIZE;
                                            ui.spacing_mut().interact_size.y = TASK_ICON_SIZE;
                                            ui.add_enabled(
                                                false,
                                                egui::Checkbox::without_text(&mut checked.clone()),
                                            );
                                            ui.with_layout(
                                                egui::Layout::top_down(egui::Align::Min),
                                                |ui| {
                                                    flow_blocks(
                                                        ui,
                                                        &item.blocks,
                                                        appearance,
                                                        colors,
                                                        provider,
                                                        true,
                                                    )
                                                },
                                            );
                                        },
                                    );
                                } else {
                                    flow_blocks(
                                        ui,
                                        &item.blocks,
                                        appearance,
                                        colors,
                                        provider,
                                        true,
                                    )
                                }
                            });
                    });
                    previous = item_edges.bottom;
                }
            }
            Block::Code { text, style, .. } => {
                egui::Frame::new()
                    .fill(colors.code_background)
                    .corner_radius(CORNER_RADIUS)
                    .inner_margin(CODE_PADDING)
                    .show(ui, |ui| {
                        egui::ScrollArea::horizontal()
                            .id_salt("code-scroll")
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                if text.is_empty() {
                                    return;
                                }
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(text)
                                            .font(egui::FontId::new(
                                                FONT_SIZE,
                                                if style.bold {
                                                    crate::font_families::code_bold(ui)
                                                } else {
                                                    FontFamily::Monospace
                                                },
                                            ))
                                            .line_height(Some(LINE_HEIGHT))
                                            .color(colors.foreground),
                                    )
                                    .selectable(true)
                                    .halign(ui.layout().horizontal_align())
                                    .extend(),
                                );
                            });
                    });
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
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), CELL_BORDER),
                    egui::Sense::hover(),
                );
                ui.painter().hline(
                    rect.x_range(),
                    rect.center().y,
                    Stroke::new(CELL_BORDER, colors.border),
                );
            }
        });
        previous_margin = edges.bottom;
    }
    if !trim_edges {
        ui.add_space(previous_margin);
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

use egui::{Color32, FontId, RichText, Ui};
use taide_native_editor::documentation::{Alignment, Block, Inline, RichDocument};

use crate::editor_surface::EditorAppearance;

const LIST_INDENT: f32 = 20.0;
const CODE_PADDING: i8 = 5;
const HEADING_SCALE: [f32; 6] = [2.0, 1.5, 1.17, 1.0, 0.83, 0.67];
const HEADING_LINE_HEIGHT: f32 = 1.1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colors {
    pub background: Color32,
    pub foreground: Color32,
    pub border: Color32,
    pub highlight: Color32,
    pub link: Color32,
    pub code_background: Color32,
    pub shadow: Color32,
}

pub trait Provider {
    fn open_link(&mut self, ui: &Ui, target: &str) -> bool;
    fn code(&mut self, _ui: &Ui, _language: &str, _text: &str) -> Option<egui::text::LayoutJob> {
        None
    }
    fn image(
        &mut self,
        _ui: &mut Ui,
        _source: &str,
        _alt: &str,
        _dimensions: taide_native_editor::documentation::ImageDimensions,
    ) -> Option<egui::Response> {
        None
    }
}

impl<T: crate::editor_documentation::Provider + ?Sized> Provider for T {
    fn open_link(&mut self, ui: &Ui, target: &str) -> bool {
        crate::editor_documentation::Provider::open_link(self, ui, target)
    }

    fn code(&mut self, ui: &Ui, language: &str, text: &str) -> Option<egui::text::LayoutJob> {
        crate::editor_documentation::Provider::code(self, ui, language, text)
    }

    fn image(
        &mut self,
        ui: &mut Ui,
        source: &str,
        alt: &str,
        dimensions: taide_native_editor::documentation::ImageDimensions,
    ) -> Option<egui::Response> {
        crate::editor_documentation::Provider::image(self, ui, source, alt, dimensions)
    }
}

pub(crate) fn show(
    ui: &mut Ui,
    document: &RichDocument,
    appearance: &EditorAppearance,
    colors: Colors,
    provider: &mut (impl Provider + ?Sized),
) {
    blocks(ui, &document.blocks, appearance, colors, provider);
}

pub(crate) fn unwrapped_width(
    ui: &Ui,
    document: &RichDocument,
    appearance: &EditorAppearance,
    provider: &mut (impl Provider + ?Sized),
) -> f32 {
    block_width(ui, &document.blocks, appearance, provider)
}

fn inline_font(
    ui: &Ui,
    style: taide_native_editor::documentation::Style,
    appearance: &EditorAppearance,
    size: f32,
    heading: bool,
) -> FontId {
    let bold = style.bold || heading;
    let family = if style.code {
        if bold {
            egui::FontFamily::Name(crate::font_families::EDITOR_BOLD_FAMILY.into())
        } else {
            appearance.font.family.clone()
        }
    } else if bold {
        crate::font_families::semibold(ui)
    } else {
        egui::FontFamily::Proportional
    };
    FontId::new(size, family)
}

fn inline_width(
    ui: &Ui,
    contents: &[Inline],
    appearance: &EditorAppearance,
    size: f32,
    heading: bool,
) -> f32 {
    let mut job = egui::text::LayoutJob::default();
    let mut images = 0.0;
    for inline in contents {
        match inline {
            Inline::Text(span) => job.append(
                &span.text,
                0.0,
                egui::TextFormat {
                    font_id: inline_font(ui, span.style, appearance, size, heading),
                    italics: span.style.italic,
                    ..Default::default()
                },
            ),
            Inline::Break => job.append("\n", 0.0, egui::TextFormat::default()),
            Inline::Image { dimensions, .. } => {
                images += dimensions.width.unwrap_or_default() as f32;
            }
        }
    }
    ui.fonts_mut(|fonts| fonts.layout_job(job).size().x) + images
}

fn block_width(
    ui: &Ui,
    blocks: &[Block],
    appearance: &EditorAppearance,
    provider: &mut (impl Provider + ?Sized),
) -> f32 {
    blocks
        .iter()
        .map(|block| match block {
            Block::Paragraph(contents) => {
                inline_width(ui, contents, appearance, appearance.font.size, false)
            }
            Block::Heading { level, contents } => inline_width(
                ui,
                contents,
                appearance,
                appearance.font.size
                    * HEADING_SCALE
                        .get(usize::from(level.saturating_sub(1)))
                        .copied()
                        .unwrap_or(1.0),
                true,
            ),
            Block::Quote(blocks) => {
                block_width(ui, blocks, appearance, provider)
                    + LIST_INDENT
                    + f32::from(CODE_PADDING) * 2.0
            }
            Block::List { items, .. } => {
                items
                    .iter()
                    .map(|item| block_width(ui, &item.blocks, appearance, provider))
                    .fold(0.0, f32::max)
                    + LIST_INDENT
            }
            Block::Code { language, text } => {
                let mut job = provider.code(ui, language, text).unwrap_or_else(|| {
                    egui::text::LayoutJob::simple(
                        text.clone(),
                        appearance.font.clone(),
                        ui.visuals().text_color(),
                        f32::INFINITY,
                    )
                });
                job.wrap.max_width = f32::INFINITY;
                ui.fonts_mut(|fonts| fonts.layout_job(job).size().x) + f32::from(CODE_PADDING) * 2.0
            }
            Block::Table { header, rows, .. } => {
                let mut widths = vec![0.0f32; header.len()];
                for (row_index, row) in std::iter::once(header).chain(rows).enumerate() {
                    for (index, cell) in row.iter().enumerate() {
                        if let Some(width) = widths.get_mut(index) {
                            *width = width.max(inline_width(
                                ui,
                                cell,
                                appearance,
                                appearance.font.size,
                                row_index == 0,
                            ));
                        }
                    }
                }
                widths.iter().sum::<f32>()
                    + ui.spacing().item_spacing.x * widths.len().saturating_sub(1) as f32
            }
            Block::Rule => 0.0,
        })
        .fold(0.0, f32::max)
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
            Block::Paragraph(contents) => inlines(
                ui,
                contents,
                appearance,
                colors,
                provider,
                appearance.font.size,
                false,
            ),
            Block::Heading { level, contents } => inlines(
                ui,
                contents,
                appearance,
                colors,
                provider,
                appearance.font.size
                    * HEADING_SCALE
                        .get(usize::from(level.saturating_sub(1)))
                        .copied()
                        .unwrap_or(1.0),
                true,
            ),
            Block::Quote(contents) => {
                egui::Frame::new()
                    .inner_margin(CODE_PADDING)
                    .show(ui, |ui| {
                        ui.indent("quote", |ui| {
                            blocks(ui, contents, appearance, colors, provider)
                        });
                    });
            }
            Block::List { start, items } => {
                for (index, item) in items.iter().enumerate() {
                    ui.push_id(index, |ui| {
                        ui.horizontal_top(|ui| {
                            if let Some(checked) = item.checked {
                                ui.add_enabled(
                                    false,
                                    egui::Checkbox::without_text(&mut checked.clone()),
                                );
                            } else if let Some(start) = start {
                                ui.label(format!("{}.", start + index as u64));
                            } else {
                                ui.label("\u{2022}");
                            }
                            ui.vertical(|ui| {
                                ui.set_max_width((ui.available_width() - LIST_INDENT).max(0.0));
                                blocks(ui, &item.blocks, appearance, colors, provider);
                            });
                        })
                    });
                }
            }
            Block::Code { language, text } => {
                egui::Frame::new()
                    .fill(colors.code_background)
                    .inner_margin(CODE_PADDING)
                    .show(ui, |ui| {
                        if let Some(job) = provider.code(ui, language, text) {
                            ui.add(egui::Label::new(job).selectable(true));
                        } else {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(text)
                                        .font(appearance.font.clone())
                                        .color(colors.foreground),
                                )
                                .selectable(true),
                            );
                        }
                    });
            }
            Block::Table {
                columns,
                header,
                rows,
            } => {
                egui::Grid::new("table").striped(false).show(ui, |ui| {
                    for (row_index, row) in std::iter::once(header).chain(rows.iter()).enumerate() {
                        for (column, contents) in row.iter().enumerate() {
                            let align = match columns.get(column) {
                                Some(Alignment::Center) => egui::Align::Center,
                                Some(Alignment::Right) => egui::Align::Max,
                                _ => egui::Align::Min,
                            };
                            ui.with_layout(egui::Layout::top_down(align), |ui| {
                                inlines(
                                    ui,
                                    contents,
                                    appearance,
                                    colors,
                                    provider,
                                    appearance.font.size,
                                    row_index == 0,
                                )
                            });
                        }
                        ui.end_row();
                    }
                });
            }
            Block::Rule => {
                ui.separator();
            }
        });
    }
}

fn inlines(
    ui: &mut Ui,
    contents: &[Inline],
    appearance: &EditorAppearance,
    colors: Colors,
    provider: &mut (impl Provider + ?Sized),
    size: f32,
    heading: bool,
) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for inline in contents {
            match inline {
                Inline::Text(span) => {
                    let mut text = RichText::new(&span.text)
                        .font(inline_font(ui, span.style, appearance, size, heading))
                        .color(colors.foreground)
                        .line_height(Some(if heading {
                            size * HEADING_LINE_HEIGHT
                        } else {
                            appearance.line_height
                        }));
                    if span.style.italic {
                        text = text.italics();
                    }
                    if span.style.strike {
                        text = text.strikethrough();
                    }
                    if span.style.code {
                        text = text.background_color(colors.code_background);
                    }
                    if let Some(link) = &span.link {
                        let response = ui.add(
                            egui::Label::new(text.color(colors.link).underline())
                                .selectable(true)
                                .sense(egui::Sense::click()),
                        );
                        ui.ctx()
                            .register_pointer_preserves_keyboard_focus(response.id);
                        if response.clicked() {
                            provider.open_link(ui, &link.target);
                        }
                        if !link.title.is_empty() {
                            response.on_hover_text(&link.title);
                        }
                    } else {
                        ui.add(egui::Label::new(text).selectable(true));
                    }
                }
                Inline::Image {
                    source,
                    title,
                    alt,
                    link,
                    dimensions,
                } => {
                    let response =
                        provider
                            .image(ui, source, alt, *dimensions)
                            .unwrap_or_else(|| {
                                ui.add(
                                    egui::Label::new(alt)
                                        .selectable(true)
                                        .sense(egui::Sense::click()),
                                )
                            });
                    ui.ctx()
                        .register_pointer_preserves_keyboard_focus(response.id);
                    if let Some(link) = link
                        && response.clicked()
                    {
                        provider.open_link(ui, &link.target);
                    }
                    if !title.is_empty() {
                        response.on_hover_text(title);
                    }
                }
                Inline::Break => {
                    ui.end_row();
                }
            }
        }
    });
}

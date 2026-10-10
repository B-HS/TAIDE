use std::iter::repeat_n;
use std::ops::Range;

use egui::text::{ByteIndex, CCursor, LayoutJob, LayoutSection, TextFormat, TextWrapping};
use egui::{Color32, FontFamily, FontId, Galley, Painter, Pos2, Rect, Stroke, StrokeKind, pos2};
use taide_native_editor::completion_preview_view::AdditionalLine;
use taide_native_editor::display_map::RowProjection;
use taide_native_editor::line_breaks::{is_full_width_character, tab_columns};
use taide_native_editor::line_tokens::{TokenStyle, TokenStyleTable};

const FIRST_SUPPLEMENTARY_CODE: u32 = 0x10000;
const WIDE_CHARACTER_COLUMNS: f64 = 2.0;
const TAB_HALVES: usize = 2;
const SPAN_FIELDS: usize = 2;
const TEXT_DECORATION_STROKE: f32 = 1.0;
const DOTTED_UNDERLINE_RADIUS: f32 = 0.5;
const DOTTED_UNDERLINE_PERIOD: f32 = 2.0;

#[derive(Debug, Clone, PartialEq)]
pub struct RowSection {
    pub chars: Range<usize>,
    pub foreground: Color32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RowFontStyle {
    pub is_italic: bool,
    pub is_bold: bool,
    pub is_underlined: bool,
    pub is_struck_through: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowColumns {
    pub tab_size: u32,
    pub start_column: f64,
    pub indent_columns: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct RowTokens<'a> {
    pub spans: &'a [u32],
    pub styles: &'a TokenStyleTable,
    pub row_start_byte: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RowInlineStyle {
    pub foreground: Option<Color32>,
    pub underline: Option<Color32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RowText {
    pub text: String,
    pub sections: Vec<RowSection>,
    pub font_styles: Vec<RowFontStyle>,
    pub underline_colors: Vec<Option<Color32>>,
    pub model_bytes: Vec<u32>,
    pub virtual_bytes: Vec<u32>,
    pub indent_chars: usize,
    pub injected: Vec<bool>,
    pub preview: Vec<bool>,
    pub injection_background: Option<Color32>,
    pub injection_border: Option<Color32>,
    pub injection_dotted: Vec<bool>,
    pub injection_foreground: Color32,
    pub injection_opacity: f32,
}

fn section_style(style: TokenStyle) -> (Color32, RowFontStyle) {
    let [red, green, blue, alpha] = style.foreground;
    (
        Color32::from_rgba_unmultiplied(red, green, blue, alpha),
        RowFontStyle {
            is_italic: style.is_italic,
            is_bold: style.is_bold,
            is_underlined: style.is_underlined,
            is_struck_through: style.is_struck_through,
        },
    )
}

impl RowText {
    pub fn expanded(source: &str, columns: RowColumns, foreground: Color32) -> Self {
        let indent_chars = columns.indent_columns as usize;
        let mut text = " ".repeat(indent_chars);
        let mut model_bytes = vec![0; indent_chars];
        let mut visible_column = columns.start_column;
        let mapped = |offset: usize| u32::try_from(offset).unwrap_or(u32::MAX);
        for (offset, character) in source.char_indices() {
            if character == '\t' {
                let width = tab_columns(visible_column, columns.tab_size);
                let spaces = width as usize;
                text.extend(repeat_n(' ', spaces));
                model_bytes.extend(repeat_n(mapped(offset), spaces));
                visible_column += width;
                continue;
            }
            let is_wide = u32::from(character) >= FIRST_SUPPLEMENTARY_CODE
                || is_full_width_character(character);
            text.push(character);
            model_bytes.push(mapped(offset));
            visible_column += if is_wide { WIDE_CHARACTER_COLUMNS } else { 1.0 };
        }
        model_bytes.push(mapped(source.len()));
        let injected = vec![false; model_bytes.len()];
        let preview = vec![false; model_bytes.len()];
        let injection_dotted = vec![false; model_bytes.len()];
        Self {
            sections: vec![RowSection {
                chars: 0..model_bytes.len() - 1,
                foreground,
            }],
            font_styles: vec![RowFontStyle::default()],
            underline_colors: vec![None],
            text,
            virtual_bytes: model_bytes.clone(),
            model_bytes,
            indent_chars,
            injected,
            preview,
            injection_background: None,
            injection_border: None,
            injection_dotted,
            injection_foreground: foreground,
            injection_opacity: 1.0,
        }
    }

    pub fn projected(projection: &RowProjection, columns: RowColumns, foreground: Color32) -> Self {
        let mut row = Self::expanded(&projection.text, columns, foreground);
        for byte in &mut row.virtual_bytes {
            *byte = byte.saturating_add(u32::try_from(projection.text_start).unwrap_or(u32::MAX));
        }
        for (index, byte) in row.model_bytes.iter_mut().enumerate() {
            let at = projection
                .units
                .partition_point(|unit| unit.text_byte <= *byte as usize)
                .saturating_sub(1);
            let unit = &projection.units[at];
            *byte = u32::try_from(
                unit.source_byte
                    .saturating_sub(projection.segment.bytes.start),
            )
            .unwrap_or(u32::MAX);
            if index >= row.indent_chars {
                row.injected[index] = unit.injected;
                row.preview[index] = unit.is_preview;
            }
        }
        row
    }

    pub fn additional(
        line: &AdditionalLine,
        source_start: usize,
        columns: RowColumns,
        foreground: Color32,
    ) -> Self {
        let mut row = Self::expanded(&line.text, columns, foreground);
        let mut run_index = 0;
        let mut preceding = source_start;
        for (index, byte) in row.model_bytes.iter_mut().enumerate() {
            let text_byte = *byte as usize;
            while let Some(run) = line
                .runs
                .get(run_index)
                .filter(|run| run.bytes.end <= text_byte)
            {
                if let Some(source) = &run.source {
                    preceding = source.end;
                }
                run_index += 1;
            }
            let source = if let Some(run) = line.runs.get(run_index) {
                if let Some(source) = &run.source {
                    source.start + text_byte.saturating_sub(run.bytes.start)
                } else {
                    if index >= row.indent_chars {
                        row.injected[index] = true;
                    }
                    preceding
                }
            } else {
                preceding
            };
            *byte = u32::try_from(source.saturating_sub(source_start)).unwrap_or(u32::MAX);
        }
        row
    }

    pub fn injection_anchor(&self, model_byte: usize) -> bool {
        self.model_bytes
            .iter()
            .zip(&self.injected)
            .any(|(byte, injected)| *injected && *byte as usize == model_byte)
    }

    pub fn style_injections(
        &mut self,
        foreground: Color32,
        tokens: Option<RowTokens<'_>>,
        is_syntax_highlighted: bool,
    ) {
        self.injection_foreground = foreground;
        self.injection_opacity = if is_syntax_highlighted {
            crate::editor_geometry::PREVIEW_OPACITY
        } else {
            1.0
        };
        let mut first = self.indent_chars;
        while first + 1 < self.model_bytes.len() {
            if !self.injected[first] {
                first += 1;
                continue;
            }
            let preview = self.preview[first];
            let style_at = |index: usize| {
                tokens.map(|tokens| {
                    let spans = tokens.spans.as_chunks::<SPAN_FIELDS>().0;
                    let at = spans
                        .partition_point(|[byte, _]| *byte <= self.virtual_bytes[index])
                        .saturating_sub(1);
                    spans
                        .get(at)
                        .map_or(tokens.styles.default_style(), |[_, style]| {
                            tokens.styles.style(*style)
                        })
                })
            };
            let style = style_at(first);
            let mut end = first + 1;
            while end + 1 < self.model_bytes.len()
                && self.injected[end]
                && self.preview[end] == preview
                && style_at(end) == style
            {
                end += 1;
            }
            let foreground = style.map_or(foreground, |style| {
                section_style(style)
                    .0
                    .gamma_multiply(self.injection_opacity)
            });
            self.decorate(
                first..end,
                RowInlineStyle {
                    foreground: Some(foreground),
                    underline: None,
                },
            );
            for (section, font_style) in self.sections.iter().zip(&mut self.font_styles) {
                if section.chars.start >= first && section.chars.end <= end {
                    if let Some(style) = style {
                        *font_style = section_style(style).1;
                    }
                    font_style.is_italic = !preview;
                }
            }
            first = end;
        }
    }

    pub(crate) fn paint_injections(&self, painter: &Painter, origin: Pos2, galley: &Galley) {
        if self.injection_border.is_none() && !self.injection_dotted.iter().any(|dotted| *dotted) {
            return;
        }
        let mut first = self.indent_chars;
        while first + 1 < self.model_bytes.len() {
            if !self.injected[first] {
                first += 1;
                continue;
            }
            let dotted = self.injection_dotted[first];
            let mut end = first + 1;
            while end + 1 < self.model_bytes.len()
                && self.injected[end]
                && self.injection_dotted[end] == dotted
            {
                end += 1;
            }
            let left = origin.x + galley.pos_from_cursor(CCursor::new(first)).left();
            let right = origin.x + galley.pos_from_cursor(CCursor::new(end)).left();
            let rect = Rect::from_min_max(
                pos2(left, origin.y + galley.rect.top()),
                pos2(right, origin.y + galley.rect.bottom()),
            );
            if let Some(color) = self.injection_border {
                painter.rect_stroke(
                    rect,
                    0.0,
                    Stroke::new(
                        TEXT_DECORATION_STROKE,
                        color.gamma_multiply(self.injection_opacity),
                    ),
                    StrokeKind::Inside,
                );
            }
            if dotted {
                let mut x = left + DOTTED_UNDERLINE_RADIUS;
                while x < right {
                    painter.circle_filled(
                        pos2(x, rect.bottom()),
                        DOTTED_UNDERLINE_RADIUS,
                        self.injection_foreground
                            .gamma_multiply(self.injection_opacity),
                    );
                    x += DOTTED_UNDERLINE_PERIOD;
                }
            }
            first = end;
        }
    }

    pub fn highlight(&mut self, tokens: RowTokens<'_>) {
        let char_count = self.model_bytes.len() - 1;
        let default_style = tokens.styles.default_style();
        let spans = tokens.spans.as_chunks::<SPAN_FIELDS>().0;
        let covering = spans
            .partition_point(|[start, _]| *start as usize <= tokens.row_start_byte)
            .saturating_sub(1);
        let row_char = |line_byte: u32| {
            self.display_char((line_byte as usize).saturating_sub(tokens.row_start_byte))
        };
        let mut sections: Vec<RowSection> = Vec::new();
        let mut font_styles: Vec<RowFontStyle> = Vec::new();
        let mut cover = |chars: Range<usize>, style: TokenStyle| {
            if chars.is_empty() {
                return;
            }
            let (foreground, font_style) = section_style(style);
            match sections.last_mut().zip(font_styles.last()) {
                Some((previous, previous_font_style))
                    if previous.foreground == foreground && *previous_font_style == font_style =>
                {
                    previous.chars.end = chars.end;
                }
                _ => {
                    sections.push(RowSection { chars, foreground });
                    font_styles.push(font_style);
                }
            }
        };
        cover(0..self.indent_chars, default_style);
        let mut covered = self.indent_chars;
        for (index, [start, style_id]) in spans.iter().enumerate().skip(covering) {
            if covered == char_count {
                break;
            }
            let first = row_char(*start).max(covered);
            let end = spans
                .get(index + 1)
                .map_or(char_count, |[next, _]| row_char(*next))
                .max(first);
            cover(covered..first, default_style);
            cover(first..end, tokens.styles.style(*style_id));
            covered = end;
        }
        cover(covered..char_count, default_style);
        if sections.is_empty() {
            let (foreground, font_style) = section_style(default_style);
            sections.push(RowSection {
                chars: 0..char_count,
                foreground,
            });
            font_styles.push(font_style);
        }
        self.underline_colors = vec![None; sections.len()];
        self.sections = sections;
        self.font_styles = font_styles;
    }

    pub fn decorate(&mut self, chars: Range<usize>, style: RowInlineStyle) {
        if chars.is_empty() {
            return;
        }
        self.font_styles
            .resize(self.sections.len(), RowFontStyle::default());
        self.underline_colors.resize(self.sections.len(), None);
        self.split_section(chars.start);
        self.split_section(chars.end);
        let decorated = self
            .sections
            .iter_mut()
            .zip(&mut self.underline_colors)
            .filter(|(section, _)| {
                chars.start <= section.chars.start && section.chars.end <= chars.end
            });
        for (section, underline) in decorated {
            section.foreground = style.foreground.unwrap_or(section.foreground);
            *underline = style.underline.or(*underline);
        }
    }

    fn split_section(&mut self, display_char: usize) {
        let Some(index) = self.sections.iter().position(|section| {
            section.chars.start < display_char && display_char < section.chars.end
        }) else {
            return;
        };
        let mut tail = self.sections[index].clone();
        tail.chars.start = display_char;
        self.sections[index].chars.end = display_char;
        self.sections.insert(index + 1, tail);
        self.font_styles.insert(index + 1, self.font_styles[index]);
        self.underline_colors
            .insert(index + 1, self.underline_colors[index]);
    }

    pub fn layout_job(&self, font: &FontId) -> LayoutJob {
        self.styled_layout_job(font, None)
    }

    pub fn styled_layout_job(&self, font: &FontId, bold_family: Option<&FontFamily>) -> LayoutJob {
        let mut located = (0, 0);
        let mut text_byte = |display_char: usize| {
            let (from_char, from_byte) = if display_char < located.0 {
                (0, 0)
            } else {
                located
            };
            let byte = self.text[from_byte..]
                .char_indices()
                .nth(display_char - from_char)
                .map_or(self.text.len(), |(offset, _)| from_byte + offset);
            located = (display_char, byte);
            ByteIndex(byte)
        };
        LayoutJob {
            sections: self
                .sections
                .iter()
                .enumerate()
                .map(|(index, section)| {
                    let font_style = self.font_styles.get(index).copied().unwrap_or_default();
                    let decoration = |is_drawn: bool| {
                        if is_drawn {
                            Stroke::new(TEXT_DECORATION_STROKE, section.foreground)
                        } else {
                            Stroke::NONE
                        }
                    };
                    let underline = self.underline_colors.get(index).copied().flatten();
                    LayoutSection {
                        leading_space: 0.0,
                        byte_range: text_byte(section.chars.start)..text_byte(section.chars.end),
                        format: TextFormat {
                            background: self
                                .injection_background
                                .filter(|_| {
                                    self.injected
                                        .get(section.chars.start)
                                        .copied()
                                        .unwrap_or(false)
                                })
                                .map_or(Color32::TRANSPARENT, |color| {
                                    color.gamma_multiply(self.injection_opacity)
                                }),
                            italics: font_style.is_italic,
                            underline: underline.map_or_else(
                                || decoration(font_style.is_underlined),
                                |color| Stroke::new(TEXT_DECORATION_STROKE, color),
                            ),
                            strikethrough: decoration(font_style.is_struck_through),
                            ..TextFormat::simple(
                                match bold_family.filter(|_| font_style.is_bold) {
                                    Some(family) => FontId::new(font.size, family.clone()),
                                    None => font.clone(),
                                },
                                section.foreground,
                            )
                        },
                    }
                })
                .collect(),
            text: self.text.clone(),
            wrap: TextWrapping {
                max_width: f32::INFINITY,
                ..Default::default()
            },
            break_on_newline: true,
            ..Default::default()
        }
    }

    fn content(&self) -> &[u32] {
        &self.model_bytes[self.indent_chars..]
    }

    fn first_display_char(&self, mapped: u32) -> usize {
        self.indent_chars
            + self
                .content()
                .partition_point(|candidate| *candidate < mapped)
    }

    fn display_char_after(&self, mapped: u32) -> usize {
        self.indent_chars
            + self
                .content()
                .partition_point(|candidate| *candidate <= mapped)
    }

    pub fn model_byte(&self, display_char: usize) -> usize {
        let index = display_char
            .max(self.indent_chars)
            .min(self.model_bytes.len() - 1);
        let mapped = self.model_bytes[index];
        if self.injected[index] {
            return mapped as usize;
        }
        let mut first = self.first_display_char(mapped);
        let next = self.display_char_after(mapped);
        while first < next && self.injected[first] {
            first += 1;
        }
        let nearest = if (index - first) * TAB_HALVES <= next - first {
            mapped
        } else {
            self.model_bytes[next]
        };
        nearest as usize
    }

    pub fn display_char(&self, model_byte: usize) -> usize {
        let model_byte = u32::try_from(model_byte).unwrap_or(u32::MAX);
        let after = self.display_char_after(model_byte);
        let Some(last) = after
            .checked_sub(1)
            .filter(|last| *last >= self.indent_chars)
        else {
            return self.indent_chars;
        };
        let mapped = self.model_bytes[last];
        if mapped == model_byte || after == self.model_bytes.len() {
            return self.first_display_char(mapped);
        }
        let ends_before =
            self.text.chars().nth(last).is_some_and(|character| {
                mapped as usize + character.len_utf8() <= model_byte as usize
            });
        if ends_before {
            after
        } else {
            self.first_display_char(mapped)
        }
    }
}

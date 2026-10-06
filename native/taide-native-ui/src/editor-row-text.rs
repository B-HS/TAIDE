use std::iter::repeat_n;
use std::ops::Range;

use egui::text::{ByteIndex, LayoutJob, LayoutSection, TextFormat, TextWrapping};
use egui::{Color32, FontId};
use taide_native_editor::line_breaks::{is_full_width_character, tab_columns};

const FIRST_SUPPLEMENTARY_CODE: u32 = 0x10000;
const WIDE_CHARACTER_COLUMNS: f64 = 2.0;
const TAB_HALVES: usize = 2;

#[derive(Debug, Clone, PartialEq)]
pub struct RowSection {
    pub chars: Range<usize>,
    pub foreground: Color32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowColumns {
    pub tab_size: u32,
    pub start_column: f64,
    pub indent_columns: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RowText {
    pub text: String,
    pub sections: Vec<RowSection>,
    pub model_bytes: Vec<u32>,
    pub indent_chars: usize,
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
        Self {
            sections: vec![RowSection {
                chars: 0..model_bytes.len() - 1,
                foreground,
            }],
            text,
            model_bytes,
            indent_chars,
        }
    }

    pub fn layout_job(&self, font: &FontId) -> LayoutJob {
        let text_byte = |display_char: usize| {
            ByteIndex(
                self.text
                    .char_indices()
                    .nth(display_char)
                    .map_or(self.text.len(), |(offset, _)| offset),
            )
        };
        LayoutJob {
            sections: self
                .sections
                .iter()
                .map(|section| LayoutSection {
                    leading_space: 0.0,
                    byte_range: text_byte(section.chars.start)..text_byte(section.chars.end),
                    format: TextFormat::simple(font.clone(), section.foreground),
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
        let first = self.first_display_char(mapped);
        let next = self.display_char_after(mapped);
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

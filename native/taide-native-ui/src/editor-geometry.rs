use std::borrow::Cow;
use std::ops::Range;
use std::sync::Arc;

use egui::text::CCursor;
use egui::{Color32, FontFamily, FontId, Galley, Painter, Pos2, Rangef, Rect, Vec2, pos2, vec2};
use taide_native_editor::decoration::{DecorationKind, UnderlineKind};
use taide_native_editor::display_map::{DisplayMap, RowSegment, WrapSettings, WrappingIndent};
use taide_native_editor::document::DocumentSnapshot;

use crate::editor_paint::{FrameDecoration, color32};
use crate::editor_row_text::{RowColumns, RowInlineStyle, RowText, RowTokens};
use crate::editor_surface::{EditorAppearance, EditorTokens};

pub(crate) const CURSOR_STROKE: f32 = 1.0;
const LEADING_SIDES: f32 = 2.0;
const TYPICAL_HALF_WIDTH_CHARACTER: char = 'n';
const TYPICAL_FULL_WIDTH_CHARACTER: char = '\u{FF4D}';
pub(crate) const UNREADABLE_CHARACTER_WIDTH: f32 = 2.0;
pub(crate) const FALLBACK_CHARACTER_WIDTH: f32 = 5.0;
const WRAP_CURSOR_ROOM: f32 = 2.0;
pub(crate) const PREVIEW_OPACITY: f32 = 0.7;
pub(crate) const FOLD_PLACEHOLDER: &str = "\u{22EF}";
pub(crate) const FOLD_PLACEHOLDER_MARGIN_EM: f32 = 0.2;

#[derive(Debug, Clone, PartialEq)]
pub struct EditorGeometry {
    pub rect: Rect,
    pub content_rect: Rect,
    pub gutter_rect: Rect,
    pub line_height: f32,
    pub visible_rows: Range<usize>,
    pub scroll: Vec2,
    #[cfg(feature = "native-host")]
    pub minimap_rect: Option<Rect>,
    pub(crate) rows: Arc<[Row]>,
}

impl EditorGeometry {
    fn row_band(&self, row: &Row) -> Rangef {
        Rangef::new(row.origin.y, row.origin.y + self.line_height)
    }

    fn shown_rows(&self) -> impl Iterator<Item = &Row> {
        self.rows.iter().filter(|row| {
            let band = self.row_band(row);
            band.max > self.rect.top() && band.min < self.rect.bottom()
        })
    }

    pub fn caret_rect(&self, byte: usize) -> Option<Rect> {
        let row = self.shown_rows().find(|row| {
            let bytes = &row.segment.bytes;
            bytes.start <= byte
                && (byte < bytes.end
                    || (byte == bytes.end
                        && (!row.wraps || row.text.injection_anchor(byte - bytes.start))))
        })?;
        Some(Rect::from_x_y_ranges(
            Rangef::point(row.caret_rect(byte).left()),
            self.row_band(row),
        ))
    }

    pub fn range_rects(&self, bytes: Range<usize>) -> Vec<Rect> {
        self.shown_rows()
            .filter_map(|row| {
                row.extent(&bytes)
                    .map(|extent| Rect::from_x_y_ranges(extent, self.row_band(row)))
            })
            .collect()
    }

    pub fn byte_at(&self, position: Pos2) -> Option<usize> {
        if !self.content_rect.contains(position) {
            return None;
        }
        self.rows
            .iter()
            .find(|row| {
                let band = self.row_band(row);
                band.min <= position.y && position.y < band.max
            })
            .map(|row| row.byte_at(position))
    }

    #[cfg(feature = "native-host")]
    pub fn content_byte_at(&self, position: Pos2) -> Option<usize> {
        if !self.content_rect.contains(position) {
            return None;
        }
        self.shown_rows()
            .find(|row| {
                let band = self.row_band(row);
                band.min <= position.y
                    && position.y < band.max
                    && row.caret_rect(row.segment.bytes.start).left() <= position.x
                    && position.x < row.caret_rect(row.segment.bytes.end).left()
            })
            .map(|row| row.byte_at(position))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Row {
    pub(crate) index: usize,
    pub(crate) segment: RowSegment,
    pub(crate) wraps: bool,
    pub(crate) heads_hidden_lines: bool,
    pub(crate) text: RowText,
    pub(crate) origin: Pos2,
    pub(crate) half_leading: f32,
    pub(crate) galley: Arc<Galley>,
    injection_carets: Vec<(usize, f32)>,
}

#[derive(Clone, Copy)]
pub(crate) struct RowLayout<'a> {
    pub(crate) painter: &'a Painter,
    pub(crate) document: &'a DocumentSnapshot,
    pub(crate) display: &'a DisplayMap,
    pub(crate) appearance: &'a EditorAppearance,
    pub(crate) half_leading: f32,
    pub(crate) tab_size: u32,
    pub(crate) tokens: Option<EditorTokens<'a>>,
    #[cfg(feature = "native-host")]
    pub(crate) preview_styles: &'a [crate::editor_completion::PreviewStyle],
    #[cfg(feature = "native-host")]
    pub(crate) preview_colors: crate::editor_completion::PreviewColors,
    pub(crate) bold_family: Option<&'a FontFamily>,
    pub(crate) decorations: &'a [FrameDecoration],
    #[cfg(feature = "native-host")]
    pub(crate) brackets: Option<(
        &'a taide_native_editor::bracket_model::BracketModel,
        &'a crate::editor_brackets::EditorBracketColors,
    )>,
}

impl RowLayout<'_> {
    pub(crate) fn row(&self, index: usize, origin: Pos2) -> Row {
        let segment = self.display.segment(self.document, index);
        let source = Cow::from(self.document.rope.byte_slice(segment.bytes.clone()));
        let columns = RowColumns {
            tab_size: self.tab_size,
            start_column: self.display.row_start_column(index),
            indent_columns: segment.indent_columns,
        };
        let mut text = self.display.row_projection(index).map_or_else(
            || RowText::expanded(&source, columns, self.appearance.foreground),
            |projection| RowText::projected(projection, columns, self.appearance.foreground),
        );
        if let Some(tokens) = self.tokens {
            let line_start = self.document.rope.line_to_byte(segment.line);
            text.highlight(RowTokens {
                spans: tokens.lines.spans(segment.line),
                styles: tokens.styles,
                row_start_byte: segment.bytes.start.saturating_sub(line_start),
            });
        }
        #[cfg(feature = "native-host")]
        if let Some((model, colors)) = self.brackets {
            for bracket in model
                .brackets_in(segment.bytes.clone())
                .iter()
                .filter(|bracket| bracket.colorized)
            {
                let start = bracket.bytes.start.max(segment.bytes.start);
                let end = bracket.bytes.end.min(segment.bytes.end);
                let chars = text.display_char(start - segment.bytes.start)
                    ..text.display_char(end - segment.bytes.start);
                text.decorate(
                    chars,
                    RowInlineStyle {
                        foreground: Some(if bracket.invalid {
                            colors.unexpected
                        } else {
                            colors.palette[bracket.level % colors.palette.len()]
                        }),
                        underline: None,
                    },
                );
            }
        }
        for decoration in self.decorations {
            let DecorationKind::Inline(inline) = decoration.kind else {
                continue;
            };
            let style = RowInlineStyle {
                foreground: inline.foreground.map(color32),
                underline: inline
                    .underline
                    .filter(|underline| underline.kind == UnderlineKind::Straight)
                    .map(|underline| color32(underline.color)),
            };
            let start = decoration.bytes.start.max(segment.bytes.start);
            let end = decoration.bytes.end.min(segment.bytes.end);
            if start >= end || style == RowInlineStyle::default() {
                continue;
            }
            let row_char = |byte: usize| text.display_char(byte - segment.bytes.start);
            let chars = row_char(start)..row_char(end);
            text.decorate(chars, style);
        }
        if self.display.row_projection(index).is_some() {
            #[cfg(feature = "native-host")]
            let tokens = self
                .preview_styles
                .iter()
                .find(|style| style.line == segment.line && !style.short)
                .and_then(|style| style.tokens.as_deref())
                .map(|tokens| RowTokens {
                    spans: tokens.lines.spans(0),
                    styles: &tokens.styles,
                    row_start_byte: 0,
                });
            #[cfg(not(feature = "native-host"))]
            let tokens = None;
            #[cfg(feature = "native-host")]
            let is_syntax_highlighted = !self
                .preview_styles
                .iter()
                .any(|style| style.line == segment.line && style.short);
            #[cfg(feature = "native-host")]
            let foreground = if is_syntax_highlighted {
                self.appearance.foreground.gamma_multiply(PREVIEW_OPACITY)
            } else {
                self.preview_colors.foreground
            };
            #[cfg(not(feature = "native-host"))]
            let foreground = self.appearance.foreground.gamma_multiply(PREVIEW_OPACITY);
            #[cfg(not(feature = "native-host"))]
            let is_syntax_highlighted = true;
            text.style_injections(foreground, tokens, is_syntax_highlighted);
            #[cfg(feature = "native-host")]
            {
                text.injection_background = self.preview_colors.background;
                text.injection_border = self.preview_colors.border;
                text.injection_foreground = self.preview_colors.foreground;
                for (index, byte) in text.model_bytes.iter().enumerate() {
                    text.injection_dotted[index] = text.injected[index]
                        && self.preview_styles.iter().any(|style| {
                            style.line == segment.line
                                && (style.short
                                    || style
                                        .short_anchors
                                        .contains(&(*byte as usize + segment.bytes.start)))
                        });
                }
            }
        }
        let job = text.styled_layout_job(&self.appearance.font, self.bold_family);
        let mut injection_carets = Vec::new();
        for (index, (byte, injected)) in text.model_bytes.iter().zip(&text.injected).enumerate() {
            if !injected
                || injection_carets
                    .iter()
                    .any(|(source, _)| *source == *byte as usize)
            {
                continue;
            }
            let index = text.display_char(*byte as usize).min(index);
            let boundary = job
                .text
                .char_indices()
                .nth(index)
                .map_or(job.text.len(), |(byte, _)| byte);
            let mut prefix = job.clone();
            prefix.text.truncate(boundary);
            prefix
                .sections
                .retain(|section| section.byte_range.start < egui::text::ByteIndex(boundary));
            for section in &mut prefix.sections {
                section.byte_range.end =
                    section.byte_range.end.min(egui::text::ByteIndex(boundary));
            }
            let galley = self.painter.layout_job(prefix);
            injection_carets.push((
                *byte as usize,
                galley.pos_from_cursor(CCursor::new(index)).left(),
            ));
        }
        let galley = self.painter.layout_job(job);
        Row {
            index,
            wraps: index + 1 < self.display.rows_of_line(segment.line).end,
            heads_hidden_lines: self.display.hidden_lines_at(segment.line + 1).is_some(),
            segment,
            text,
            origin,
            half_leading: self.half_leading,
            galley,
            injection_carets,
        }
    }
}

impl Row {
    pub(crate) fn text_origin(&self) -> Pos2 {
        pos2(self.origin.x, self.origin.y + self.half_leading)
    }

    pub(crate) fn caret(&self, byte: usize) -> Rect {
        let local = byte.saturating_sub(self.segment.bytes.start);
        let rect = self
            .galley
            .pos_from_cursor(CCursor::new(self.text.display_char(local)));
        self.injection_carets
            .iter()
            .find(|(source, _)| *source == local)
            .map_or(rect, |(_, x)| rect.translate(vec2(*x - rect.left(), 0.0)))
    }

    pub(crate) fn caret_rect(&self, byte: usize) -> Rect {
        self.caret(byte).translate(self.text_origin().to_vec2())
    }

    pub(crate) fn byte_at(&self, position: Pos2) -> usize {
        let cursor = self.galley.cursor_from_pos(vec2(
            position.x - self.origin.x,
            self.galley.rect.center().y,
        ));
        (self.segment.bytes.start + self.text.model_byte(cursor.index.0))
            .min(self.segment.bytes.end)
    }

    pub(crate) fn fold_placeholder_left(&self, appearance: &EditorAppearance) -> f32 {
        self.caret_rect(self.segment.bytes.end).left()
            + FOLD_PLACEHOLDER_MARGIN_EM * appearance.font.size
    }

    pub(crate) fn extent(&self, bytes: &Range<usize>) -> Option<Rangef> {
        let start = bytes.start.max(self.segment.bytes.start);
        let end = bytes.end.min(self.segment.bytes.end);
        if start >= end {
            return None;
        }
        let extent = Rangef::new(self.caret_rect(start).left(), self.caret_rect(end).left());
        (extent.span() > 0.0).then_some(extent)
    }
}

pub(crate) fn half_leading(painter: &Painter, appearance: &EditorAppearance) -> f32 {
    let font_height = painter.fonts_mut(|fonts| fonts.row_height(&appearance.font));
    ((appearance.line_height - font_height) / LEADING_SIDES).round()
}

fn advance_width(painter: &Painter, font: &FontId, character: char) -> f32 {
    painter
        .layout_no_wrap(character.into(), font.clone(), Color32::PLACEHOLDER)
        .rows
        .first()
        .and_then(|row| row.row.glyphs.first())
        .map_or(0.0, |glyph| glyph.advance_width)
}

pub(crate) fn wrap_settings(
    painter: &Painter,
    appearance: &EditorAppearance,
    content_width: f32,
    vertical_scrollbar_width: f32,
    tab_size: u32,
) -> WrapSettings {
    let measured = [TYPICAL_HALF_WIDTH_CHARACTER, TYPICAL_FULL_WIDTH_CHARACTER]
        .map(|character| advance_width(painter, &appearance.font, character));
    let [half_width, full_width] = if measured
        .iter()
        .any(|width| *width <= UNREADABLE_CHARACTER_WIDTH)
    {
        measured.map(|width| width.max(FALLBACK_CHARACTER_WIDTH))
    } else {
        measured
    };
    let viewport_column =
        ((content_width - vertical_scrollbar_width - WRAP_CURSOR_ROOM) / half_width).floor();
    WrapSettings {
        wrap_column: viewport_column.max(1.0) as u32,
        tab_size,
        full_width_columns: f64::from(full_width) / f64::from(half_width),
        wrapping_indent: WrappingIndent::Same,
    }
}

pub(crate) fn scroll_x_revealing(scroll_x: f32, caret: Rect, text_width: f32) -> f32 {
    let scroll_x = if caret.left() < scroll_x {
        caret.left()
    } else {
        scroll_x
    };
    if caret.right() + CURSOR_STROKE > scroll_x + text_width {
        caret.right() + CURSOR_STROKE - text_width
    } else {
        scroll_x
    }
}

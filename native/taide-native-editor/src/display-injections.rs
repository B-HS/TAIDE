use std::collections::BTreeMap;
use std::ops::Range;

use crate::completion_preview::Part;
use crate::completion_preview_view::ViewData;
use crate::display_map::{DisplayMap, RowSegment};
use crate::document::{DocumentSnapshot, EditorError, byte_to_char};
use crate::editing::line_content_range;
use crate::line_breaks::create_line_breaks;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUnit {
    pub text_byte: usize,
    pub source_byte: usize,
    pub injected: bool,
    pub is_preview: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RowProjection {
    pub text: String,
    pub text_start: usize,
    pub units: Vec<SourceUnit>,
    pub segment: RowSegment,
    pub start_column: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct InjectedLine {
    line: usize,
    original: Range<usize>,
    rows: Vec<RowProjection>,
    anchors: Vec<usize>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Injections {
    pub(crate) views: Vec<ViewData>,
    lines: Vec<InjectedLine>,
}

impl Injections {
    pub(crate) fn build(
        document: &DocumentSnapshot,
        display: &DisplayMap,
        views: &[ViewData],
    ) -> Result<Self, EditorError> {
        let mut grouped = BTreeMap::<usize, Vec<&ViewData>>::new();
        for view in views {
            if view.line >= document.rope.len_lines() {
                return Err(EditorError::InvalidBoundary);
            }
            grouped.entry(view.line).or_default().push(view);
        }
        let mut lines = Vec::with_capacity(grouped.len());
        for (line, views) in grouped {
            let original = display.base_rows_of_line(line);
            if original.is_empty() {
                continue;
            }
            let source = line_content_range(document, line);
            let mut inline = views
                .iter()
                .flat_map(|view| view.inline.iter())
                .collect::<Vec<_>>();
            inline.sort_by_key(|part| part.byte);
            for part in &inline {
                if part.byte < source.start
                    || part.byte > source.end
                    || part.text.contains(['\n', '\r'])
                {
                    return Err(EditorError::InvalidBoundary);
                }
                byte_to_char(&document.rope, part.byte)?;
            }
            let hidden = views
                .iter()
                .filter_map(|view| view.hidden_source.as_ref())
                .map(|range| {
                    if range.start < source.start
                        || range.end != source.end
                        || range.start > range.end
                    {
                        return Err(EditorError::InvalidBoundary);
                    }
                    byte_to_char(&document.rope, range.start)?;
                    Ok(range.start)
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .min();
            let mut text = String::new();
            let mut units = Vec::new();
            let mut previous = source.start;
            let append_source =
                |start: usize, end: usize, text: &mut String, units: &mut Vec<SourceUnit>| {
                    let end = end.min(hidden.unwrap_or(source.end));
                    if start >= end {
                        return;
                    }
                    let value = document.rope.byte_slice(start..end).to_string();
                    for (offset, character) in value.char_indices() {
                        units.push(SourceUnit {
                            text_byte: text.len(),
                            source_byte: start + offset,
                            injected: false,
                            is_preview: false,
                        });
                        text.push(character);
                    }
                };
            for Part {
                byte,
                text: value,
                is_preview,
            } in &inline
            {
                append_source(previous, *byte, &mut text, &mut units);
                for character in value.chars() {
                    units.push(SourceUnit {
                        text_byte: text.len(),
                        source_byte: *byte,
                        injected: true,
                        is_preview: *is_preview,
                    });
                    text.push(character);
                }
                previous = *byte;
            }
            append_source(previous, source.end, &mut text, &mut units);
            units.push(SourceUnit {
                text_byte: text.len(),
                source_byte: source.end,
                injected: false,
                is_preview: false,
            });
            let breaks = display
                .wrap_settings()
                .and_then(|settings| create_line_breaks(settings, &text));
            let offsets = breaks
                .as_ref()
                .map_or_else(|| vec![text.len()], |breaks| breaks.break_offsets.clone());
            let mut start = 0;
            let mut rows = Vec::with_capacity(offsets.len());
            for (index, end) in offsets.into_iter().enumerate() {
                let first = units.partition_point(|unit| unit.text_byte < start);
                let after = units.partition_point(|unit| unit.text_byte < end);
                let mut mapped = units[first..after]
                    .iter()
                    .cloned()
                    .map(|mut unit| {
                        unit.text_byte -= start;
                        unit
                    })
                    .collect::<Vec<_>>();
                let last = units[after].source_byte;
                let beginning = if index == 0 {
                    source.start
                } else {
                    units[first].source_byte
                };
                let mut boundary = units[after].clone();
                boundary.text_byte = end - start;
                mapped.push(boundary);
                rows.push(RowProjection {
                    text: text[start..end].into(),
                    text_start: start,
                    units: mapped,
                    segment: RowSegment {
                        line,
                        bytes: beginning..last,
                        is_continuation: index > 0,
                        indent_columns: breaks
                            .as_ref()
                            .filter(|_| index > 0)
                            .map_or(0, |breaks| breaks.wrapped_text_indent_length),
                        ends_folded: false,
                    },
                    start_column: breaks
                        .as_ref()
                        .zip(index.checked_sub(1))
                        .map_or(0.0, |(breaks, previous)| {
                            breaks.break_offsets_visible_column[previous]
                        }),
                });
                start = end;
            }
            if let Some(last) = rows.last_mut() {
                last.segment.ends_folded = display.hidden_lines_at(line + 1).is_some();
            }
            lines.push(InjectedLine {
                line,
                original,
                rows,
                anchors: inline.into_iter().map(|part| part.byte).collect(),
            });
        }
        Ok(Self {
            views: views.to_vec(),
            lines,
        })
    }

    pub(crate) fn row_count(&self, original: usize) -> usize {
        self.lines.iter().fold(original, |count, line| {
            count - line.original.len() + line.rows.len()
        })
    }

    pub(crate) fn boundary(&self, original: usize) -> usize {
        let mut boundary = original;
        for line in &self.lines {
            if line.original.end > original {
                break;
            }
            boundary = boundary - line.original.len() + line.rows.len();
        }
        boundary
    }

    pub(crate) fn locate(&self, row: usize) -> Result<&RowProjection, usize> {
        let mut original = row;
        for line in &self.lines {
            if original < line.original.start {
                break;
            }
            let index = original - line.original.start;
            if let Some(projected) = line.rows.get(index) {
                return Ok(projected);
            }
            original = original - line.rows.len() + line.original.len();
        }
        Err(original)
    }

    pub(crate) fn row_of_byte(&self, line: usize, byte: usize) -> Option<usize> {
        let injected = self.lines.iter().find(|injected| injected.line == line)?;
        let anchor = injected.anchors.contains(&byte);
        let index = injected
            .rows
            .iter()
            .position(|row| {
                row.segment.bytes.start <= byte
                    && (byte < row.segment.bytes.end || anchor && byte == row.segment.bytes.end)
            })
            .unwrap_or(injected.rows.len() - 1);
        Some(self.boundary(injected.original.start) + index)
    }
}

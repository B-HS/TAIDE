use crate::editor_geometry::Row;
use egui::{Modifiers, Pos2};
use std::ops::Range;
use taide_native_editor::bracket_navigation::bracket_pairs;
use taide_native_editor::document::DocumentSnapshot;
use taide_native_editor::editing::{line_content_range, mouse_word_range};
use taide_native_editor::language_configuration::Language;
use taide_native_editor::syntax::TokenKind;
use taide_native_editor::view::{Selection, SelectionSet};

const WORD_CLICK: usize = 2;
const LINE_CLICK: usize = 3;
const ALL_CLICK: usize = 4;
const RTL_RANGES: [(u32, u32); 46] = [
    (0x5BE, 0x5BE),
    (0x5C0, 0x5C0),
    (0x5C3, 0x5C3),
    (0x5C6, 0x5C6),
    (0x5D0, 0x5F4),
    (0x608, 0x608),
    (0x60B, 0x60B),
    (0x60D, 0x60D),
    (0x61B, 0x64A),
    (0x66D, 0x66F),
    (0x671, 0x6D5),
    (0x6E5, 0x6E6),
    (0x6EE, 0x6EF),
    (0x6FA, 0x710),
    (0x712, 0x72F),
    (0x74D, 0x7A5),
    (0x7B1, 0x7EA),
    (0x7F4, 0x7F5),
    (0x7FA, 0x7FA),
    (0x7FE, 0x815),
    (0x81A, 0x81A),
    (0x824, 0x824),
    (0x828, 0x828),
    (0x830, 0x858),
    (0x85E, 0x88E),
    (0x8A0, 0x8C9),
    (0x200F, 0x200F),
    (0xFB1D, 0xFB1D),
    (0xFB1F, 0xFB28),
    (0xFB2A, 0xFD3D),
    (0xFD50, 0xFDC7),
    (0xFDF0, 0xFDFC),
    (0xFE70, 0xFEFC),
    (0x10800, 0x1091B),
    (0x10920, 0x10A00),
    (0x10A10, 0x10A35),
    (0x10A40, 0x10AE4),
    (0x10AEB, 0x10B35),
    (0x10B40, 0x10D23),
    (0x10E80, 0x10EA9),
    (0x10EAD, 0x10F45),
    (0x10F51, 0x10F81),
    (0x10F86, 0x10FF6),
    (0x1E800, 0x1E8CF),
    (0x1E900, 0x1E943),
    (0x1E94B, 0x1EEBB),
];

#[derive(Clone, Copy)]
enum DragUnit {
    Character,
    Word,
    Line,
    Column,
}

#[derive(Clone)]
struct Drag {
    revision: u64,
    selections: SelectionSet,
    active: usize,
    unit: DragUnit,
    initial: Range<usize>,
    row: usize,
    x: f32,
}

#[derive(Default, Clone)]
pub(crate) struct PointerSelection {
    previous: Option<(f64, Pos2, usize)>,
    drag: Option<Drag>,
}

pub(crate) struct PointerInput {
    pub pressed: bool,
    pub down: bool,
    pub head: usize,
    pub row: usize,
    pub position: Pos2,
    pub modifiers: Modifiers,
    pub time: f64,
    pub max_click_dist: f32,
    pub max_double_click_delay: f64,
    pub gutter: bool,
}

fn line_range(document: &DocumentSnapshot, byte: usize) -> Range<usize> {
    let line = document.rope.byte_to_line(byte);
    let end = if line + 1 < document.rope.len_lines() {
        document.rope.line_to_byte(line + 1)
    } else {
        document.rope.len_bytes()
    };
    document.rope.line_to_byte(line)..end
}

fn block_range(
    document: &DocumentSnapshot,
    byte: usize,
    language: Option<Language<'_>>,
) -> Option<Range<usize>> {
    let language = language?;
    if let Some(pair) = bracket_pairs(document, language)
        .iter()
        .find(|pair| pair.open.end == byte || pair.close.start == byte)
    {
        return Some(pair.open.end..pair.close.start);
    }
    let line = document.rope.byte_to_line(byte);
    let range = line_content_range(document, line);
    let text = document.rope.byte_slice(range.clone()).to_string();
    let tokens = language.syntax.accurate_tokens(document, line)?;
    let offset = byte.saturating_sub(range.start);
    let index = tokens
        .partition_point(|token| token.start_byte <= offset)
        .saturating_sub(1);
    if tokens.get(index)?.kind != TokenKind::String {
        return None;
    }
    let mut first = index;
    let mut last = index;
    while first > 0 && tokens[first - 1].kind == TokenKind::String {
        first -= 1;
    }
    while tokens
        .get(last + 1)
        .is_some_and(|token| token.kind == TokenKind::String)
    {
        last += 1;
    }
    let start = tokens[first].start_byte;
    let end = tokens
        .get(last + 1)
        .map_or(text.len(), |token| token.start_byte);
    let content = text.get(start..end)?;
    if content.chars().any(|character| {
        RTL_RANGES
            .iter()
            .any(|(first, last)| (*first..=*last).contains(&u32::from(character)))
    }) {
        return None;
    }
    let quote = content.chars().next()?;
    if !matches!(quote, '\'' | '"' | '`')
        || content.chars().last() != Some(quote)
        || end < start + 2
        || offset != start + 1 && offset != end - 1
    {
        return None;
    }
    Some(range.start + start + 1..range.start + end - 1)
}

impl PointerSelection {
    pub(crate) fn update(
        &mut self,
        document: &DocumentSnapshot,
        selection: &SelectionSet,
        input: PointerInput,
        row_at: impl Fn(usize) -> Row,
        row_of: impl Fn(usize) -> usize,
        language: Option<Language<'_>>,
    ) -> Option<SelectionSet> {
        if input.pressed {
            if input.modifiers.alt && (input.modifiers.ctrl || input.modifiers.mac_cmd) {
                self.drag = None;
                return None;
            }
            let count = self
                .previous
                .filter(|(time, position, _)| {
                    input.time - time < input.max_double_click_delay
                        && position.distance(input.position) < input.max_click_dist
                })
                .map_or(1, |(_, _, count)| count + 1);
            self.previous = Some((input.time, input.position, count));
            if count >= ALL_CLICK {
                self.drag = None;
                return Some(SelectionSet {
                    primary: 0,
                    selections: vec![Selection {
                        anchor: 0,
                        head: document.rope.len_bytes(),
                    }],
                });
            }
            let unit = if input.gutter || count == LINE_CLICK {
                DragUnit::Line
            } else if count == WORD_CLICK {
                DragUnit::Word
            } else if input.modifiers.alt && input.modifiers.shift {
                DragUnit::Column
            } else {
                DragUnit::Character
            };
            let mut selections = selection.clone();
            let initial = match unit {
                DragUnit::Word => (!input.modifiers.alt)
                    .then(|| block_range(document, input.head, language))
                    .flatten()
                    .unwrap_or_else(|| mouse_word_range(document, input.head, false)),
                DragUnit::Line => line_range(document, input.head),
                DragUnit::Column => {
                    let anchor = selection.selections[selection.primary].anchor;
                    anchor..anchor
                }
                DragUnit::Character if input.modifiers.shift => {
                    let anchor = selection.selections[selection.primary].anchor;
                    anchor..anchor
                }
                DragUnit::Character => input.head..input.head,
            };
            let active = if input.modifiers.alt && !input.modifiers.shift {
                if count == 1 {
                    if selections.selections.len() > 1
                        && let Some(index) = selections.selections.iter().position(|s| {
                            s.anchor.min(s.head) <= input.head && input.head <= s.anchor.max(s.head)
                        })
                    {
                        selections.selections.remove(index);
                        selections.primary = selections
                            .primary
                            .saturating_sub(usize::from(index < selections.primary))
                            .min(selections.selections.len() - 1);
                        self.drag = None;
                        return Some(selections);
                    }
                    selections.selections.push(Selection {
                        anchor: input.head,
                        head: input.head,
                    });
                }
                selections.selections.len() - 1
            } else {
                selections = SelectionSet::default();
                0
            };
            let origin_row = row_of(initial.start);
            let origin_x = row_at(origin_row).caret_rect(initial.start).left();
            self.drag = Some(Drag {
                revision: document.revision,
                selections,
                active,
                unit,
                initial,
                row: origin_row,
                x: origin_x,
            });
        }
        let drag = self.drag.as_ref()?;
        if drag.revision != document.revision {
            self.drag = None;
            return None;
        }
        let mut result = drag.selections.clone();
        match drag.unit {
            DragUnit::Column => {
                let lines: Vec<_> = if input.row >= drag.row {
                    (drag.row..=input.row).collect()
                } else {
                    (input.row..=drag.row).rev().collect()
                };
                let from = drag.x;
                let to = input.position.x;
                let mut selections = Vec::new();
                for line in &lines {
                    let row = row_at(*line);
                    let anchor = row.byte_at(Pos2::new(from, row.origin.y));
                    let head = row.byte_at(Pos2::new(to, row.origin.y));
                    let start = row.caret_rect(anchor).left();
                    let end = row.caret_rect(head).left();
                    if from < to && (start > to || end < from)
                        || from > to && (end > from || start < to)
                    {
                        continue;
                    }
                    selections.push(Selection { anchor, head });
                }
                if selections.is_empty() {
                    selections = lines
                        .into_iter()
                        .map(|line| {
                            let byte = row_at(line).segment.bytes.end;
                            Selection {
                                anchor: byte,
                                head: byte,
                            }
                        })
                        .collect();
                }
                result = SelectionSet {
                    primary: 0,
                    selections,
                };
            }
            DragUnit::Character => {
                result.selections[drag.active] = Selection {
                    anchor: drag.initial.start,
                    head: input.head,
                }
            }
            DragUnit::Word | DragUnit::Line => {
                let target = if matches!(drag.unit, DragUnit::Word) {
                    mouse_word_range(document, input.head, true)
                } else {
                    line_range(document, input.head)
                };
                result.selections[drag.active] = if input.head < drag.initial.start {
                    Selection {
                        anchor: drag.initial.end,
                        head: target.start,
                    }
                } else {
                    Selection {
                        anchor: drag.initial.start,
                        head: target.end.max(drag.initial.end),
                    }
                };
            }
        }
        if !input.down {
            self.drag = None;
        }
        Some(result.normalized())
    }
}

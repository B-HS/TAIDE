use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, LazyLock};

use egui::{Align2, Color32, FontId, Id, Rect, Sense, Ui, Vec2, pos2};
use taide_native_editor::display_layout::VerticalLayout;
use taide_native_editor::display_map::DisplayMap;
use taide_native_editor::document::{DocumentId, DocumentSnapshot};
use taide_native_editor::editing::line_content_range;

use crate::editor_geometry::RowLayout;
use crate::editor_surface::EditorAppearance;

const FONT_SCALE: f32 = 0.9;
const MINIMUM_LINE_FACTOR: f32 = 1.3;
const CENTER: f32 = 0.5;
const SEPARATOR: &str = "\u{00a0}|\u{00a0}";
const FADE_SECONDS: f64 = 0.1;
const SPIN_SECONDS: f64 = 1.5;
const SPIN_STEPS: f64 = 30.0;
const DISABLED_OPACITY: f32 = 0.4;

static CODICONS: LazyLock<BTreeMap<String, u32>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("editor-code-lens-codicons.json"))
        .expect("bundled CodeLens codicon mapping is valid")
});

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub anchor: usize,
    pub title: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colors {
    pub foreground: Color32,
    pub hover: Color32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Presentation {
    pub document: DocumentId,
    pub revision: u64,
    pub items: Arc<[Item]>,
    pub colors: Colors,
}

impl Presentation {
    pub(crate) fn describes(&self, document: &DocumentSnapshot) -> bool {
        self.document == document.id && self.revision == document.revision
    }

    fn groups(&self, document: &DocumentSnapshot) -> Vec<(usize, Vec<usize>)> {
        let mut groups = BTreeMap::<usize, Vec<usize>>::new();
        for (index, item) in self.items.iter().enumerate() {
            if item.anchor <= document.rope.len_bytes()
                && document
                    .rope
                    .char_to_byte(document.rope.byte_to_char(item.anchor))
                    == item.anchor
            {
                let line = document.rope.byte_to_line(item.anchor);
                if item.anchor <= line_content_range(document, line).end {
                    groups.entry(line).or_default().push(index);
                }
            }
        }
        groups.into_iter().collect()
    }
}

pub(crate) fn font(appearance: &EditorAppearance) -> FontId {
    FontId::new(
        (appearance.font.size * FONT_SCALE).floor().max(1.0),
        appearance.font.family.clone(),
    )
}

pub(crate) fn height(appearance: &EditorAppearance) -> f32 {
    (font(appearance).size
        * (appearance.line_height / appearance.font.size).max(MINIMUM_LINE_FACTOR))
    .floor()
    .max(1.0)
}

pub(crate) fn presentation_height(
    presentation: Option<&Presentation>,
    document: &DocumentSnapshot,
    appearance: &EditorAppearance,
) -> f32 {
    presentation
        .filter(|value| value.describes(document) && !value.items.is_empty())
        .map_or(0.0, |_| height(appearance))
}

pub(crate) fn layout(
    layout: VerticalLayout,
    document: &DocumentSnapshot,
    display: &DisplayMap,
    height: f32,
    presentation: Option<&Presentation>,
) -> VerticalLayout {
    let Some(presentation) = presentation.filter(|value| value.describes(document)) else {
        return layout;
    };
    layout.with_before_zones(presentation.groups(document).into_iter().map(|(line, _)| {
        let rows = display.rows_of_line(line);
        (
            if rows.is_empty() {
                usize::MAX
            } else {
                rows.start
            },
            height,
        )
    }))
}

#[derive(Default, Clone, Debug)]
pub struct Geometry {
    pub zones: Vec<Rect>,
    pub commands: Vec<(usize, Rect)>,
}

#[derive(Default)]
pub struct Output {
    pub geometry: Geometry,
    pub clicked: Option<usize>,
    pub visible: Vec<usize>,
}

#[derive(Default, Clone)]
struct LineState {
    had_command: bool,
    appeared: Option<f64>,
}

#[derive(Default, Clone)]
pub(crate) struct State {
    document: Option<DocumentId>,
    lines: HashMap<usize, LineState>,
}

pub(crate) struct Frame<'a, 'document> {
    pub id: Id,
    pub document: &'a DocumentSnapshot,
    pub display: &'a DisplayMap,
    pub layout: &'a VerticalLayout,
    pub rows: &'a RowLayout<'document>,
    pub appearance: &'a EditorAppearance,
    pub rect: Rect,
    pub text_rect: Rect,
    pub scroll: Vec2,
}

struct Span {
    text: String,
    icon: bool,
    spin: bool,
    disabled: bool,
}

fn spans(title: &str) -> Vec<Span> {
    let mut result = Vec::new();
    let mut collapsed = String::new();
    let mut was_space = false;
    for character in title.chars() {
        let is_space = matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{c}');
        if !is_space || !was_space {
            collapsed.push(if is_space { ' ' } else { character });
        }
        was_space = is_space;
    }
    let mut remaining = collapsed.as_str();
    while let Some(start) = remaining.find("$(") {
        let Some(end) = remaining[start..].find(')') else {
            break;
        };
        let end = start + end;
        let value = &remaining[start + 2..end];
        let (name, modifier) = value.split_once('~').unwrap_or((value, ""));
        let valid = !name.is_empty()
            && name
                .bytes()
                .all(|value| value.is_ascii_alphanumeric() || value == b'-')
            && modifier.bytes().all(|value| value.is_ascii_alphabetic());
        if !valid {
            let stop = end + 1;
            result.push(Span {
                text: remaining[..stop].into(),
                icon: false,
                spin: false,
                disabled: false,
            });
            remaining = &remaining[stop..];
            continue;
        }
        let escaped = remaining[..start].ends_with('\\');
        let plain_end = start.saturating_sub(usize::from(escaped));
        if plain_end > 0 {
            result.push(Span {
                text: remaining[..plain_end].into(),
                icon: false,
                spin: false,
                disabled: false,
            });
        }
        if escaped {
            result.push(Span {
                text: remaining[start..end + 1].into(),
                icon: false,
                spin: false,
                disabled: false,
            });
        } else if let Some(character) = CODICONS.get(name).and_then(|value| char::from_u32(*value))
        {
            result.push(Span {
                text: character.into(),
                icon: true,
                spin: name == "loading"
                    || name == "tree-item-loading"
                    || (modifier == "spin"
                        && matches!(name, "sync" | "gear" | "notebook-state-executing")),
                disabled: modifier == "disabled",
            });
        }
        remaining = &remaining[end + 1..];
    }
    if !remaining.is_empty() {
        result.push(Span {
            text: remaining.into(),
            icon: false,
            spin: false,
            disabled: false,
        });
    }
    result
}

pub(crate) fn label_parts(title: &str) -> Vec<(String, bool)> {
    spans(title)
        .into_iter()
        .map(|span| (span.text, span.icon))
        .collect()
}

impl State {
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        frame: Frame<'_, '_>,
        presentation: Option<&Presentation>,
    ) -> Output {
        let mut output = Output::default();
        let Some(presentation) = presentation.filter(|value| value.describes(frame.document))
        else {
            self.document = None;
            self.lines.clear();
            return output;
        };
        if self.document != Some(frame.document.id) {
            self.document = Some(frame.document.id);
            self.lines.clear();
        }
        let groups = presentation.groups(frame.document);
        self.lines.retain(|line, _| {
            groups
                .binary_search_by_key(line, |(current, _)| *current)
                .is_ok()
        });
        let font = font(frame.appearance);
        let painter = ui.painter().with_clip_rect(frame.text_rect);
        let time = ui.input(|input| input.time);
        for (order, (line, indices)) in groups.into_iter().enumerate() {
            let had_line = self.lines.contains_key(&line);
            let state = self.lines.entry(line).or_default();
            let commands = indices
                .iter()
                .copied()
                .filter_map(|index| {
                    presentation.items[index]
                        .title
                        .as_ref()
                        .map(|title| (index, title))
                })
                .collect::<Vec<_>>();
            if had_line && !state.had_command && !commands.is_empty() {
                state.appeared = Some(time);
            }
            state.had_command = !commands.is_empty();
            let Some(zone) = frame.layout.before_zone(order) else {
                continue;
            };
            let zone_rect = Rect::from_min_max(
                pos2(
                    frame.rect.left(),
                    frame.rect.top() + zone.start - frame.scroll.y,
                ),
                pos2(
                    frame.rect.right(),
                    frame.rect.top() + zone.end - frame.scroll.y,
                ),
            )
            .intersect(frame.rect);
            if !zone_rect.is_positive() {
                continue;
            }
            output.geometry.zones.push(zone_rect);
            output.visible.extend(indices.iter().copied());
            let rows = frame.display.rows_of_line(line);
            if rows.is_empty() || commands.is_empty() {
                continue;
            }
            let row = frame.rows.row(
                rows.start,
                pos2(
                    frame.text_rect.left() - frame.scroll.x,
                    frame.rect.top() + frame.layout.row_top(rows.start) - frame.scroll.y,
                ),
            );
            let bytes = line_content_range(frame.document, line);
            let indent = frame
                .document
                .rope
                .byte_slice(bytes.clone())
                .chars()
                .take_while(|character| matches!(character, ' ' | '\t'))
                .map(char::len_utf8)
                .sum::<usize>();
            let mut x = row.caret_rect(bytes.start + indent).left();
            let opacity = state.appeared.map_or(1.0, |started| {
                ((time - started) / FADE_SECONDS).clamp(0.0, 1.0) as f32
            });
            if opacity < 1.0 {
                ui.ctx().request_repaint();
            }
            for (position, (index, title)) in commands.iter().enumerate() {
                let title = title.trim_matches(|value: char| {
                    value == '\u{feff}' || (value.is_whitespace() && value != '\u{85}')
                });
                let parts = spans(title)
                    .into_iter()
                    .map(|span| {
                        let family = if span.icon {
                            crate::font_families::codicons(ui)
                        } else {
                            font.family.clone()
                        };
                        let galley = painter.layout_no_wrap(
                            span.text.clone(),
                            FontId::new(font.size, family),
                            Color32::PLACEHOLDER,
                        );
                        (span, galley)
                    })
                    .collect::<Vec<_>>();
                let width = parts.iter().map(|(_, galley)| galley.size().x).sum::<f32>();
                let top = frame.rect.top() + zone.start - frame.scroll.y;
                let rect =
                    Rect::from_min_size(pos2(x, top), egui::vec2(width, zone.end - zone.start));
                let clipped = rect.intersect(frame.text_rect);
                let response = clipped.is_positive().then(|| {
                    ui.interact(
                        clipped,
                        frame
                            .id
                            .with(("code-lens-command", frame.document.id, line, index)),
                        Sense::click(),
                    )
                });
                if clipped.is_positive() {
                    output.geometry.commands.push((*index, clipped));
                }
                if response.as_ref().is_some_and(|response| response.clicked()) {
                    output.clicked = Some(*index);
                }
                let hovered = response.as_ref().is_some_and(|response| response.hovered());
                let color = if hovered {
                    presentation.colors.hover
                } else {
                    presentation.colors.foreground
                };
                if hovered {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                for (span, galley) in parts {
                    let y = top + (zone.end - zone.start - galley.size().y) * CENTER;
                    let mut shape = egui::epaint::TextShape::new(pos2(x, y), galley.clone(), color);
                    shape.opacity_factor =
                        opacity * if span.disabled { DISABLED_OPACITY } else { 1.0 };
                    if span.spin {
                        let progress = ((time % SPIN_SECONDS) / SPIN_SECONDS * SPIN_STEPS).floor()
                            / SPIN_STEPS;
                        shape = shape.with_angle_and_anchor(
                            (progress as f32) * std::f32::consts::TAU,
                            Align2::CENTER_CENTER,
                        );
                        ui.ctx()
                            .request_repaint_after(std::time::Duration::from_secs_f64(
                                SPIN_SECONDS / SPIN_STEPS,
                            ));
                    }
                    painter.add(shape);
                    x += galley.size().x;
                }
                if position + 1 < commands.len() {
                    let galley = painter.layout_no_wrap(
                        SEPARATOR.into(),
                        font.clone(),
                        presentation.colors.foreground,
                    );
                    let y = top + (zone.end - zone.start - galley.size().y) * CENTER;
                    painter.galley(
                        pos2(x, y),
                        galley.clone(),
                        presentation.colors.foreground.gamma_multiply(opacity),
                    );
                    x += galley.size().x;
                }
            }
        }
        output
    }
}

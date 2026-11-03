use std::collections::{HashMap, HashSet};

use eframe::egui::{
    self, Color32, FontId, Id, Rect, Sense, Ui, pos2,
    text::{LayoutJob, TextFormat},
    vec2,
};
use taide_model::error::AppResult;
use taide_model::locale::ResolvedLocale;
use taide_model::search::{SearchFileMatches, SearchLineMatch};

use crate::explorer::PaintIcon;
use crate::navigation_icons::Icon;
use crate::problems_icons::Glyph;

const GROUP_HEADER_HEIGHT: f32 = 20.0;
const MATCH_LINE_HEIGHT: f32 = 16.0;
const CONTEXT_LINE_HEIGHT: f32 = 20.0;
const MATCH_ROW_VERTICAL_PADDING: f32 = 4.0;
const OVERSCAN: usize = 12;
const TEXT_SIZE: f32 = 12.0;
const PADDING: f32 = 8.0;
const GAP: f32 = 4.0;
const CHEVRON_SIZE: f32 = 12.0;
const FILE_ICON_SIZE: f32 = 14.0;
const CHECKBOX_SIZE: f32 = 14.0;
const CHECK_ICON_SIZE: f32 = 12.0;
const CHECKBOX_RADIUS: u8 = 4;
const BORDER_WIDTH: f32 = 1.0;
const MATCH_INDENT: f32 = 32.0;
const LINE_NUMBER_WIDTH: f32 = 24.0;
const LINE_NUMBER_GAP: f32 = 8.0;
const ROW_VERTICAL_PADDING: f32 = 2.0;
const CONTEXT_OPACITY: f32 = 0.7;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group {
    pub(crate) path: String,
    pub(crate) matches: Vec<SearchLineMatch>,
}

/// One run's grouped results, appended batch by batch: a path's group is found through
/// `index_by_path`, so each batch costs its own size and never regroups what came before.
#[derive(Debug, Default)]
pub(crate) struct Accumulator {
    groups: Vec<Group>,
    index_by_path: HashMap<String, usize>,
    total: u32,
}

impl Accumulator {
    pub(crate) fn append(&mut self, batches: Vec<SearchFileMatches>) {
        for batch in batches {
            self.total = self
                .total
                .saturating_add(u32::try_from(batch.matches.len()).unwrap_or(u32::MAX));
            match self.index_by_path.get(&batch.path) {
                Some(&index) => {
                    let group = &mut self.groups[index];
                    let mut matches = std::mem::take(&mut group.matches);
                    matches.extend(batch.matches);
                    group.matches = dedupe_adjacent_context(matches);
                }
                None => {
                    self.index_by_path
                        .insert(batch.path.clone(), self.groups.len());
                    self.groups.push(Group {
                        path: batch.path,
                        matches: dedupe_adjacent_context(batch.matches),
                    });
                }
            }
        }
    }

    pub(crate) fn groups(&self) -> &[Group] {
        &self.groups
    }

    pub(crate) fn total(&self) -> u32 {
        self.total
    }
}

fn dedupe_adjacent_context(matches: Vec<SearchLineMatch>) -> Vec<SearchLineMatch> {
    let lines = matches
        .iter()
        .map(|entry| i64::from(entry.line))
        .collect::<Vec<_>>();
    let mut covered: Option<i64> = None;
    matches
        .into_iter()
        .enumerate()
        .map(|(index, mut entry)| {
            let line = i64::from(entry.line);
            let before_start = line - entry.before.len() as i64;
            let skip = covered.map_or(0, |covered| (covered + 1 - before_start).max(0)) as usize;
            entry.before.drain(..skip.min(entry.before.len()));
            covered = Some(covered.map_or(line, |covered| covered.max(line)));
            if let Some(next) = lines.get(index + 1) {
                let keep = (next - line - 1).clamp(0, entry.after.len() as i64) as usize;
                entry.after.truncate(keep);
            }
            covered = covered.map(|covered| covered.max(line + entry.after.len() as i64));
            entry
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Row {
    Group { group: usize, collapsed: bool },
    Match { group: usize, index: usize },
}

/// Group headers and match rows as one list; a collapsed group keeps only its header.
pub(crate) fn rows(groups: &[Group], collapsed: &HashSet<String>) -> Vec<Row> {
    groups
        .iter()
        .enumerate()
        .flat_map(|(group, entry)| {
            let is_collapsed = collapsed.contains(&entry.path);
            std::iter::once(Row::Group {
                group,
                collapsed: is_collapsed,
            })
            .chain(
                (0..entry.matches.len())
                    .filter(move |_| !is_collapsed)
                    .map(move |index| Row::Match { group, index }),
            )
        })
        .collect()
}

pub(crate) fn row_id(groups: &[Group], row: Row) -> String {
    match row {
        Row::Group { group, .. } => format!("group:{}", groups[group].path),
        Row::Match { group, index } => {
            let entry = &groups[group];
            let found = &entry.matches[index];
            format!("match:{}:{}:{}", entry.path, found.line, found.column)
        }
    }
}

pub(crate) fn estimate_height(groups: &[Group], row: Row) -> f32 {
    match row {
        Row::Group { .. } => GROUP_HEADER_HEIGHT,
        Row::Match { group, index } => {
            let found = &groups[group].matches[index];
            MATCH_ROW_VERTICAL_PADDING
                + MATCH_LINE_HEIGHT
                + (found.before.len() + found.after.len()) as f32 * CONTEXT_LINE_HEIGHT
        }
    }
}

/// The byte offset of a 0-based UTF-16 offset into `text`; an offset inside a surrogate pair
/// moves to the next character boundary.
pub(crate) fn utf16_byte_offset(text: &str, offset: u32) -> usize {
    let mut units = 0u32;
    for (byte, character) in text.char_indices() {
        if units >= offset {
            return byte;
        }
        units += character.len_utf16() as u32;
    }
    text.len()
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn collapse_whitespace<const N: usize>(pieces: [&str; N]) -> [String; N] {
    let mut is_after_space = true;
    pieces.map(|piece| {
        let mut collapsed = String::with_capacity(piece.len());
        for character in piece.chars() {
            if character.is_whitespace() {
                if !is_after_space {
                    collapsed.push(' ');
                }
                is_after_space = true;
            } else {
                collapsed.push(character);
                is_after_space = false;
            }
        }
        collapsed
    })
}

pub(crate) enum Action {
    Toggle(String),
    Select(String),
    Open {
        path: String,
        line: u32,
        column: u32,
    },
}

pub(crate) struct List<'a> {
    pub(crate) groups: &'a [Group],
    pub(crate) collapsed: &'a HashSet<String>,
    pub(crate) excluded: Option<&'a HashSet<String>>,
}

fn activated(ui: &Ui, response: &egui::Response) -> bool {
    response.clicked()
        || (ui.is_enabled()
            && response.has_focus()
            && ui.input_mut(|input| {
                input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                    || input.consume_key(egui::Modifiers::NONE, egui::Key::Space)
            }))
}

fn paint_line(ui: &Ui, rect: Rect, job: LayoutJob) -> f32 {
    let mut job = job;
    job.wrap.max_width = rect.width().max(0.0);
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let galley = ui.painter().layout_job(job);
    let width = galley.size().x;
    let position = rect.left_center() - vec2(0.0, galley.size().y / 2.0);
    ui.painter()
        .with_clip_rect(rect.intersect(ui.clip_rect()))
        .galley(position, galley, Color32::PLACEHOLDER);
    width
}

fn text_job(text: &str, font: FontId, color: Color32) -> LayoutJob {
    LayoutJob::single_section(text.to_owned(), TextFormat::simple(font, color))
}

fn paint_numbered(ui: &Ui, rect: Rect, line: u32, text: LayoutJob, color: Color32) {
    let number = Rect::from_min_size(rect.min, vec2(LINE_NUMBER_WIDTH, rect.height()));
    ui.painter()
        .with_clip_rect(number.intersect(ui.clip_rect()))
        .text(
            number.right_center(),
            egui::Align2::RIGHT_CENTER,
            line.to_string(),
            FontId::proportional(TEXT_SIZE),
            color,
        );
    let content = Rect::from_min_max(pos2(number.right() + LINE_NUMBER_GAP, rect.top()), rect.max);
    paint_line(ui, content, text);
}

pub(crate) fn show<I>(
    ui: &mut Ui,
    id: Id,
    list: List<'_>,
    appearance: &crate::explorer::Appearance,
    locale: &ResolvedLocale,
    icons: &mut I,
) -> AppResult<Vec<Action>>
where
    I: PaintIcon<Glyph> + PaintIcon<Icon>,
{
    let colors = appearance.search();
    let rows = rows(list.groups, list.collapsed);
    let heights = rows
        .iter()
        .map(|row| estimate_height(list.groups, *row))
        .collect::<Vec<_>>();
    let tops = heights
        .iter()
        .scan(0.0, |top, height| {
            let current = *top;
            *top += height;
            Some(current)
        })
        .collect::<Vec<_>>();
    let total = heights.iter().sum::<f32>();
    let mut actions = Vec::new();
    ui.spacing_mut().item_spacing.y = 0.0;
    egui::ScrollArea::vertical()
        .id_salt(id)
        .auto_shrink([false, false])
        .show_viewport(ui, |ui, viewport| -> AppResult<()> {
            ui.set_height(total);
            let start = tops
                .iter()
                .zip(&heights)
                .position(|(top, height)| top + height > viewport.min.y)
                .unwrap_or(rows.len())
                .saturating_sub(OVERSCAN);
            let end = tops
                .partition_point(|top| *top < viewport.max.y)
                .saturating_add(OVERSCAN)
                .min(rows.len());
            let origin = ui.max_rect().min;
            let width = ui.available_width();
            for index in start..end {
                let rect = Rect::from_min_size(
                    origin + vec2(0.0, tops[index]),
                    vec2(width, heights[index]),
                );
                let row = rows[index];
                let key = row_id(list.groups, row);
                match row {
                    Row::Group { group, collapsed } => {
                        let entry = &list.groups[group];
                        let name = file_name(&entry.path);
                        if ui.rect_contains_pointer(rect) {
                            ui.painter().rect_filled(rect, 0.0, colors.hover);
                        }
                        let mut left = rect.left() + PADDING;
                        if let Some(excluded) = list.excluded {
                            let selected = !excluded.contains(&entry.path);
                            let bounds = Rect::from_min_size(
                                pos2(left, rect.center().y - CHECKBOX_SIZE / 2.0),
                                vec2(CHECKBOX_SIZE, CHECKBOX_SIZE),
                            );
                            let checkbox =
                                ui.interact(bounds, id.with(("select", &key)), Sense::click());
                            let label = crate::presentation::message(
                                locale,
                                "search.selectFile",
                                &[("path", &entry.path)],
                            );
                            checkbox.widget_info(|| {
                                egui::WidgetInfo::selected(
                                    egui::WidgetType::Checkbox,
                                    ui.is_enabled(),
                                    selected,
                                    &label,
                                )
                            });
                            if selected {
                                ui.painter().rect_filled(
                                    bounds,
                                    CHECKBOX_RADIUS,
                                    colors.checkbox_checked,
                                );
                                PaintIcon::<Icon>::paint(
                                    icons,
                                    ui,
                                    Rect::from_center_size(
                                        bounds.center(),
                                        vec2(CHECK_ICON_SIZE, CHECK_ICON_SIZE),
                                    ),
                                    Icon::Check,
                                    colors.checkbox_check,
                                    0.0,
                                )?;
                            } else {
                                ui.painter().rect_stroke(
                                    bounds,
                                    CHECKBOX_RADIUS,
                                    egui::Stroke::new(BORDER_WIDTH, colors.checkbox_border),
                                    egui::StrokeKind::Inside,
                                );
                            }
                            if activated(ui, &checkbox) {
                                actions.push(Action::Select(entry.path.clone()));
                            }
                            left = bounds.right() + GAP;
                        }
                        let button_rect = Rect::from_min_max(pos2(left, rect.top()), rect.max);
                        let button = ui.interact(button_rect, id.with(&key), Sense::click());
                        let count = entry.matches.len().to_string();
                        let label = format!("{name} {} {count}", entry.path);
                        button.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                ui.is_enabled(),
                                &label,
                            )
                        });
                        ui.ctx().accesskit_node_builder(button.id, |node| {
                            node.set_expanded(!collapsed)
                        });
                        let chevron = Rect::from_center_size(
                            pos2(left + CHEVRON_SIZE / 2.0, rect.center().y),
                            vec2(CHEVRON_SIZE, CHEVRON_SIZE),
                        );
                        PaintIcon::<Glyph>::paint(
                            icons,
                            ui,
                            chevron,
                            Glyph::ChevronRight,
                            colors.foreground,
                            if collapsed {
                                0.0
                            } else {
                                std::f32::consts::FRAC_PI_2
                            },
                        )?;
                        let (glyph, color) = crate::problems_icons::file(name);
                        let file_icon = Rect::from_center_size(
                            pos2(
                                chevron.right() + GAP + FILE_ICON_SIZE / 2.0,
                                rect.center().y,
                            ),
                            vec2(FILE_ICON_SIZE, FILE_ICON_SIZE),
                        );
                        PaintIcon::<Glyph>::paint(
                            icons,
                            ui,
                            file_icon,
                            glyph,
                            appearance.file_color(color),
                            0.0,
                        )?;
                        let font = FontId::proportional(TEXT_SIZE);
                        let count_width = ui
                            .painter()
                            .layout_no_wrap(count.clone(), font.clone(), colors.muted)
                            .size()
                            .x;
                        let right = rect.right() - PADDING;
                        ui.painter().text(
                            pos2(right, rect.center().y),
                            egui::Align2::RIGHT_CENTER,
                            &count,
                            font.clone(),
                            colors.muted,
                        );
                        let mut content = Rect::from_min_max(
                            pos2(file_icon.right() + GAP, rect.top()),
                            pos2(
                                (right - count_width - GAP).max(file_icon.right()),
                                rect.bottom(),
                            ),
                        );
                        let name_width = paint_line(
                            ui,
                            content,
                            text_job(
                                name,
                                FontId::new(TEXT_SIZE, taide_native_ui::font_families::medium(ui)),
                                colors.foreground,
                            ),
                        );
                        content.min.x = (content.min.x + name_width + GAP).min(content.max.x);
                        paint_line(ui, content, text_job(&entry.path, font, colors.muted));
                        if activated(ui, &button) {
                            actions.push(Action::Toggle(entry.path.clone()));
                        }
                    }
                    Row::Match { group, index } => {
                        let entry = &list.groups[group];
                        let found = &entry.matches[index];
                        let response = ui.interact(rect, id.with(&key), Sense::click());
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                ui.is_enabled(),
                                format!("{} {}", found.line, found.preview),
                            )
                        });
                        if response.hovered() || response.has_focus() {
                            ui.painter().rect_filled(rect, 0.0, colors.hover);
                        }
                        let content = Rect::from_min_max(
                            pos2(
                                rect.left() + MATCH_INDENT,
                                rect.top() + ROW_VERTICAL_PADDING,
                            ),
                            pos2(rect.right() - PADDING, rect.bottom() - ROW_VERTICAL_PADDING),
                        );
                        let font = FontId::proportional(TEXT_SIZE);
                        let context_color = colors.muted.gamma_multiply(CONTEXT_OPACITY);
                        let mut top = content.top();
                        let context = |line: u32, text: &str, top: &mut f32| {
                            let line_rect = Rect::from_min_max(
                                pos2(content.left(), *top + ROW_VERTICAL_PADDING),
                                pos2(
                                    content.right(),
                                    *top + CONTEXT_LINE_HEIGHT - ROW_VERTICAL_PADDING,
                                ),
                            );
                            let [text] = collapse_whitespace([text]);
                            paint_numbered(
                                ui,
                                line_rect,
                                line,
                                text_job(&text, font.clone(), context_color),
                                context_color,
                            );
                            *top += CONTEXT_LINE_HEIGHT;
                        };
                        let before_count = found.before.len() as u32;
                        for (offset, text) in found.before.iter().enumerate() {
                            context(found.line - before_count + offset as u32, text, &mut top);
                        }
                        let line_rect = Rect::from_min_size(
                            pos2(content.left(), top),
                            vec2(content.width(), MATCH_LINE_HEIGHT),
                        );
                        let start = utf16_byte_offset(&found.preview, found.match_start);
                        let end = utf16_byte_offset(&found.preview, found.match_end).max(start);
                        let [before, matched, after] = collapse_whitespace([
                            &found.preview[..start],
                            &found.preview[start..end],
                            &found.preview[end..],
                        ]);
                        let mut job = LayoutJob::default();
                        job.append(
                            &before,
                            0.0,
                            TextFormat::simple(font.clone(), colors.foreground),
                        );
                        job.append(
                            &matched,
                            0.0,
                            TextFormat::simple(
                                FontId::new(
                                    TEXT_SIZE,
                                    taide_native_ui::font_families::semibold(ui),
                                ),
                                colors.match_highlight,
                            ),
                        );
                        job.append(
                            &after,
                            0.0,
                            TextFormat::simple(font.clone(), colors.foreground),
                        );
                        paint_numbered(ui, line_rect, found.line, job, colors.muted);
                        top += MATCH_LINE_HEIGHT;
                        for (offset, text) in found.after.iter().enumerate() {
                            context(found.line + offset as u32 + 1, text, &mut top);
                        }
                        if activated(ui, &response) {
                            actions.push(Action::Open {
                                path: entry.path.clone(),
                                line: found.line,
                                column: found.column,
                            });
                        }
                    }
                }
            }
            Ok(())
        })
        .inner?;
    Ok(actions)
}

#[cfg(test)]
#[path = "search-results-tests.rs"]
mod tests;

use std::sync::Arc;

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke, Ui, pos2, vec2};
use taide_model::error::AppResult;
use taide_model::git::{LogEntry, TagInfo};
use taide_native_ui::commands::ShellIntent;
use taide_native_ui::intents::git::Intent;

use crate::git_scm::{ARROWS, TagDialog, View};
use crate::navigation_icons::{Icon, PaintIcon};
use crate::presentation;

pub(crate) const ROW_HEIGHT: f32 = 24.0;
pub(crate) const LANE_COLOR_COUNT: usize = 12;
pub(crate) const SHORT_HASH_LENGTH: usize = 7;
const LANE_WIDTH: f32 = 14.0;
const NODE_RADIUS: f32 = 3.0;
const STROKE_WIDTH: f32 = 1.5;
const OVERSCAN: usize = 12;
const PADDING_X: f32 = 8.0;
const GAP: f32 = 8.0;
const REF_GAP: f32 = 4.0;
const TEXT_SIZE: f32 = 12.0;
const REF_TEXT_SIZE: f32 = 10.0;
const REF_PADDING_X: f32 = 4.0;
const REF_RADIUS: u8 = 2;
const REF_BORDER: f32 = 1.0;
const LINE_HEIGHT_RATIO: f32 = 4.0 / 3.0;
const MENU_ICON_SIZE: f32 = 16.0;
const DIALOG_MAX_WIDTH: f32 = 512.0;
const ELLIPSIS: char = '…';
const SECONDS_PER_MINUTE: f64 = 60.0;
const MINUTES_PER_HOUR: f64 = 60.0;
const HOURS_PER_DAY: f64 = 24.0;
const ROW_ID: &str = "git-graph-row";
const MENU_ID: &str = "git-graph-menu";
const TAG_DIALOG_ID: &str = "git-create-tag";

/// An edge from a commit's node down to one of its parents' lanes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Edge<'a> {
    pub to_lane: usize,
    pub parent: &'a str,
}

/// One commit's place in the graph (`graph-lanes.ts` `GraphNode`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Node<'a> {
    pub id: &'a str,
    pub lane: usize,
    pub continues_from_above: bool,
    pub edges: Vec<Edge<'a>>,
    pub passthrough: Vec<usize>,
}

fn claim_free_lane(lanes: &mut Vec<Option<&str>>) -> usize {
    match lanes.iter().position(Option::is_none) {
        Some(free) => free,
        None => {
            lanes.push(None);
            lanes.len() - 1
        }
    }
}

/// `computeGraphLanes`: walks the commits newest first, keeping one open lane per expected parent.
pub(crate) fn compute_lanes(commits: &[LogEntry]) -> Vec<Node<'_>> {
    let mut lanes: Vec<Option<&str>> = Vec::new();
    commits
        .iter()
        .map(|commit| {
            let before = lanes.clone();
            let open =
                |lanes: &[Option<&str>], id: &str| lanes.iter().position(|lane| *lane == Some(id));
            let found = open(&lanes, &commit.id);
            let lane = found.unwrap_or_else(|| claim_free_lane(&mut lanes));
            let mut parents = commit.parents.iter().map(String::as_str);
            let mut edges = Vec::new();
            match parents.next() {
                Some(first) => match open(&lanes, first) {
                    Some(existing) if existing != lane => {
                        lanes[lane] = None;
                        edges.push(Edge {
                            to_lane: existing,
                            parent: first,
                        });
                    }
                    _ => {
                        lanes[lane] = Some(first);
                        edges.push(Edge {
                            to_lane: lane,
                            parent: first,
                        });
                    }
                },
                None => lanes[lane] = None,
            }
            for parent in parents {
                let existing = open(&lanes, parent);
                let to_lane = existing.unwrap_or_else(|| claim_free_lane(&mut lanes));
                if existing.is_none() {
                    lanes[to_lane] = Some(parent);
                }
                edges.push(Edge { to_lane, parent });
            }
            Node {
                id: &commit.id,
                lane,
                continues_from_above: found.is_some(),
                edges,
                passthrough: before
                    .iter()
                    .enumerate()
                    .filter(|(index, value)| value.is_some() && *index != lane)
                    .map(|(index, _)| index)
                    .collect(),
            }
        })
        .collect()
}

/// The widest lane any row draws into: its node, an edge's end or a line passing through.
pub(crate) fn max_lane(nodes: &[Node<'_>]) -> usize {
    nodes
        .iter()
        .flat_map(|node| {
            std::iter::once(node.lane)
                .chain(node.edges.iter().map(|edge| edge.to_lane))
                .chain(node.passthrough.iter().copied())
        })
        .max()
        .unwrap_or(0)
}

/// One stroke of a row's lane drawing, with the lane whose color it takes.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Mark {
    Line { from: Pos2, to: Pos2, lane: usize },
    Curve { points: [Pos2; 4], lane: usize },
    Node { center: Pos2, lane: usize },
}

fn lane_x(origin: Pos2, lane: usize) -> f32 {
    origin.x + lane as f32 * LANE_WIDTH + LANE_WIDTH / 2.0
}

/// The row's SVG in paint order (`commit-graph.tsx`): lines passing through, the half line from
/// above, the edges to the parents (a cubic curve when they change lane) and the node.
pub(crate) fn marks(node: &Node<'_>, origin: Pos2) -> Vec<Mark> {
    let (top, center, bottom) = (origin.y, origin.y + ROW_HEIGHT / 2.0, origin.y + ROW_HEIGHT);
    let x = |lane| lane_x(origin, lane);
    let own = x(node.lane);
    let passthrough = node.passthrough.iter().map(|lane| Mark::Line {
        from: pos2(x(*lane), top),
        to: pos2(x(*lane), bottom),
        lane: *lane,
    });
    let above = node.continues_from_above.then(|| Mark::Line {
        from: pos2(own, top),
        to: pos2(own, center),
        lane: node.lane,
    });
    let edges = node.edges.iter().map(|edge| {
        if edge.to_lane == node.lane {
            Mark::Line {
                from: pos2(own, center),
                to: pos2(own, bottom),
                lane: node.lane,
            }
        } else {
            let to = x(edge.to_lane);
            Mark::Curve {
                points: [
                    pos2(own, center),
                    pos2(own, bottom),
                    pos2(to, center),
                    pos2(to, bottom),
                ],
                lane: node.lane,
            }
        }
    });
    passthrough
        .chain(above)
        .chain(edges)
        .chain(std::iter::once(Mark::Node {
            center: pos2(own, center),
            lane: node.lane,
        }))
        .collect()
}

/// `relativeTimeToken`: the age of `time_unix` at `now_unix` in whole days, hours or minutes.
pub(crate) fn relative_time(now_unix: f64, time_unix: f64) -> (&'static str, Option<u64>) {
    let seconds = (now_unix.floor() - time_unix).max(0.0);
    let minutes = (seconds / SECONDS_PER_MINUTE).floor();
    let hours = (minutes / MINUTES_PER_HOUR).floor();
    let days = (hours / HOURS_PER_DAY).floor();
    if days > 0.0 {
        ("git.timeDaysAgo", Some(days as u64))
    } else if hours > 0.0 {
        ("git.timeHoursAgo", Some(hours as u64))
    } else if minutes > 0.0 {
        ("git.timeMinutesAgo", Some(minutes as u64))
    } else {
        ("git.timeJustNow", None)
    }
}

/// `tagsTargetingCommit`.
pub(crate) fn tags_targeting<'a>(
    tags: &'a [TagInfo],
    commit: &'a str,
) -> impl Iterator<Item = &'a TagInfo> + 'a {
    tags.iter().filter(move |tag| tag.target == commit)
}

pub(crate) fn short_hash(id: &str) -> &str {
    id.char_indices()
        .nth(SHORT_HASH_LENGTH)
        .map_or(id, |(end, _)| &id[..end])
}

fn truncated(ui: &Ui, text: String, font: FontId, color: Color32, width: f32) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(text, font, color, width.max(0.0));
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some(ELLIPSIS);
    ui.painter().layout_job(job)
}

/// The expanded graph pane under its header: the commit list, and the selected commit's details
/// below it at most half the pane high (`git-panel.tsx` graph `Panel`).
pub(crate) fn show_pane(
    ui: &mut Ui,
    body: Rect,
    view: &View<'_>,
    icons: &mut impl PaintIcon<Icon>,
    intents: &mut Vec<ShellIntent>,
) -> AppResult<()> {
    ui.painter().rect_filled(body, 0.0, view.appearance.sidebar);
    let selected = view
        .panel
        .and_then(|panel| panel.selected_commit())
        .and_then(|id| view.graph.log.iter().find(|commit| commit.id == id));
    let pane_height = body.height() + ROW_HEIGHT;
    let detail = selected.map(|commit| {
        let height =
            crate::git_commit_detail::height(view.graph.commit_files).min(pane_height / 2.0);
        (
            commit,
            Rect::from_x_y_ranges(
                body.x_range(),
                (body.bottom() - height).max(body.top())..=body.bottom(),
            ),
        )
    });
    let list = detail.map_or(body, |(_, rect)| {
        Rect::from_x_y_ranges(body.x_range(), body.top()..=rect.top())
    });
    show_rows(ui, list, view, icons, intents)?;
    if let Some((commit, rect)) = detail {
        crate::git_commit_detail::show(ui, rect, view, commit, icons, intents)?;
    }
    Ok(())
}

/// `CommitGraph`: the virtualized commit rows (12 rows of overscan) and their shared menu.
fn show_rows(
    ui: &mut Ui,
    rect: Rect,
    view: &View<'_>,
    icons: &mut impl PaintIcon<Icon>,
    intents: &mut Vec<ShellIntent>,
) -> AppResult<()> {
    let log = view.graph.log;
    let nodes = compute_lanes(log);
    let width = (max_lane(&nodes) + 1) as f32 * LANE_WIDTH;
    let mut opened = None;
    let mut list = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    egui::ScrollArea::vertical()
        .id_salt(("git-graph", view.slot))
        .auto_shrink([false, false])
        .show_viewport(&mut list, |ui, viewport| {
            ui.set_height(log.len() as f32 * ROW_HEIGHT);
            let x = ui.max_rect().x_range();
            let top = ui.max_rect().top();
            let first = ((viewport.min.y / ROW_HEIGHT).floor().max(0.0) as usize)
                .saturating_sub(OVERSCAN)
                .min(log.len());
            let end =
                ((viewport.max.y / ROW_HEIGHT).ceil().max(0.0) as usize + OVERSCAN).min(log.len());
            for index in first..end.max(first) {
                let row_top = top + index as f32 * ROW_HEIGHT;
                row(
                    ui,
                    Rect::from_x_y_ranges(x, row_top..=row_top + ROW_HEIGHT),
                    view,
                    (&log[index], &nodes[index], width),
                    intents,
                    &mut opened,
                );
            }
        });
    menu(ui, view, opened, icons, intents)
}

/// One commit row: lane drawing, ref badges, summary, then author, age and short hash in the
/// muted color. A click or Enter/Space toggles the selection, a right click opens the menu.
fn row(
    ui: &mut Ui,
    rect: Rect,
    view: &View<'_>,
    (commit, node, graph_width): (&LogEntry, &Node<'_>, f32),
    intents: &mut Vec<ShellIntent>,
    opened: &mut Option<String>,
) {
    let appearance = view.appearance;
    let id = view.mounted_id(ROW_ID).with(&commit.id);
    let is_selected = view
        .panel
        .and_then(|panel| panel.selected_commit())
        .is_some_and(|selected| selected == commit.id);
    let response = ui.interact(rect, id, Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, is_selected, &commit.summary)
    });
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, appearance.hover);
    } else if is_selected {
        ui.painter().rect_filled(rect, 0.0, appearance.selected);
    }
    let origin = pos2(rect.left() + PADDING_X, rect.top());
    let stroke = |lane: usize| Stroke::new(STROKE_WIDTH, appearance.lanes[lane % LANE_COLOR_COUNT]);
    for mark in marks(node, origin) {
        match mark {
            Mark::Line { from, to, lane } => {
                ui.painter().line_segment([from, to], stroke(lane));
            }
            Mark::Curve { points, lane } => {
                ui.painter()
                    .add(egui::epaint::CubicBezierShape::from_points_stroke(
                        points,
                        false,
                        Color32::TRANSPARENT,
                        stroke(lane),
                    ));
            }
            Mark::Node { center, lane } => {
                ui.painter().circle_filled(
                    center,
                    NODE_RADIUS,
                    appearance.lanes[lane % LANE_COLOR_COUNT],
                );
            }
        }
    }
    let center = rect.center().y;
    let mut left = origin.x + graph_width + GAP;
    let badge_height = REF_TEXT_SIZE * LINE_HEIGHT_RATIO + 2.0 * REF_BORDER;
    for name in &commit.refs {
        let text = ui.painter().layout_no_wrap(
            name.clone(),
            FontId::proportional(REF_TEXT_SIZE),
            appearance.ref_branch,
        );
        let badge = Rect::from_min_size(
            pos2(left, center - badge_height / 2.0),
            vec2(
                text.size().x + 2.0 * (REF_PADDING_X + REF_BORDER),
                badge_height,
            ),
        );
        ui.painter().rect_stroke(
            badge,
            REF_RADIUS,
            Stroke::new(REF_BORDER, appearance.ref_branch),
            egui::StrokeKind::Inside,
        );
        ui.painter().galley(
            badge.center() - text.size() / 2.0,
            text,
            appearance.ref_branch,
        );
        left = badge.right() + REF_GAP;
    }
    if !commit.refs.is_empty() {
        left += GAP - REF_GAP;
    }
    let (key, count) = relative_time(view.graph.now_unix, commit.time_unix);
    let age = count.map_or_else(
        || view.text(key),
        |count| presentation::message(view.locale, key, &[("n", &count.to_string())]),
    );
    let muted =
        |text: String, font: FontId| ui.painter().layout_no_wrap(text, font, appearance.muted);
    let trailing = [
        muted(commit.author.clone(), FontId::proportional(TEXT_SIZE)),
        muted(age, FontId::proportional(TEXT_SIZE)),
        muted(
            short_hash(&commit.id).to_owned(),
            FontId::monospace(TEXT_SIZE),
        ),
    ];
    let fixed: f32 = trailing.iter().map(|galley| galley.size().x + GAP).sum();
    let summary = truncated(
        ui,
        commit.summary.clone(),
        FontId::proportional(TEXT_SIZE),
        appearance.foreground,
        rect.right() - PADDING_X - left - fixed,
    );
    let summary_width = summary.size().x;
    ui.painter().galley(
        pos2(left, center - summary.size().y / 2.0),
        summary,
        appearance.foreground,
    );
    left += summary_width + GAP;
    for galley in trailing {
        let width = galley.size().x;
        ui.painter().galley(
            pos2(left, center - galley.size().y / 2.0),
            galley,
            appearance.muted,
        );
        left += width + GAP;
    }
    if response.has_focus() {
        ui.memory_mut(|memory| memory.set_focus_lock_filter(id, ARROWS));
    }
    if response.clicked() {
        if response.clicked_by(egui::PointerButton::Primary) {
            ui.memory_mut(|memory| memory.request_focus_with_filter(id, ARROWS));
        }
        intents.push(ShellIntent::Git(Intent::SelectCommit {
            project: view.project.clone(),
            slot: view.slot.clone(),
            commit: Some(commit.id.clone()),
        }));
    }
    if response.secondary_clicked() {
        intents.push(ShellIntent::Git(Intent::GraphMenu {
            project: view.project.clone(),
            slot: view.slot.clone(),
            commit: commit.id.clone(),
        }));
        *opened = Some(commit.id.clone());
    }
}

/// A menu entry with room for its 16px icon in front of the label (`ContextMenuItem` with a Lucide
/// icon); [`paint_menu_icon`] draws the icon there.
fn icon_button(label: String) -> egui::Button<'static> {
    egui::Button::new((
        egui::Atom::custom(egui::Id::NULL, vec2(MENU_ICON_SIZE, MENU_ICON_SIZE)),
        label,
    ))
}

fn paint_menu_icon(
    ui: &Ui,
    response: &egui::Response,
    icon: Icon,
    icons: &mut impl PaintIcon<Icon>,
) -> AppResult<()> {
    let left = response.rect.left() + ui.spacing().button_padding.x;
    icons.paint(
        ui,
        Rect::from_min_size(
            pos2(left, response.rect.center().y - MENU_ICON_SIZE / 2.0),
            vec2(MENU_ICON_SIZE, MENU_ICON_SIZE),
        ),
        icon,
        ui.visuals().text_color(),
        0.0,
    )
}

/// The row menu (`commit-graph.tsx` `ContextMenuContent`): Revert, Create Tag, and Delete Tag with
/// the commit's tags when it has any. Kept by commit id so a log refresh that drops it closes it.
fn menu(
    ui: &Ui,
    view: &View<'_>,
    opened: Option<String>,
    icons: &mut impl PaintIcon<Icon>,
    intents: &mut Vec<ShellIntent>,
) -> AppResult<()> {
    let is_opening = opened.is_some();
    let Some(target) = opened.or_else(|| {
        view.panel
            .and_then(|panel| panel.graph_menu())
            .map(str::to_owned)
    }) else {
        return Ok(());
    };
    let Some(commit) = view.graph.log.iter().find(|commit| commit.id == target) else {
        return Ok(());
    };
    let tags: Vec<_> = tags_targeting(view.graph.tags, &commit.id).collect();
    let project = view.project;
    let shown = egui::Popup::new(
        view.mounted_id(MENU_ID).with(&target),
        ui.ctx().clone(),
        egui::PopupAnchor::PointerFixed,
        ui.layer_id(),
    )
    .kind(egui::PopupKind::Menu)
    .layout(egui::Layout::top_down_justified(egui::Align::Min))
    .style(egui::menu::menu_style)
    .open_memory(is_opening.then_some(egui::SetOpenCommand::Bool(true)))
    .show(|ui| -> AppResult<()> {
        let revert = ui.add(icon_button(view.text("git.revert")));
        paint_menu_icon(ui, &revert, Icon::RotateCcw, icons)?;
        if revert.clicked() {
            intents.push(ShellIntent::Git(Intent::Revert {
                project: project.clone(),
                rev: commit.id.clone(),
            }));
            ui.close();
        }
        let create = ui.add(icon_button(view.text("git.createTag")));
        paint_menu_icon(ui, &create, Icon::Tag, icons)?;
        if create.clicked() {
            intents.push(ShellIntent::Git(Intent::RequestCreateTag {
                project: project.clone(),
                slot: view.slot.clone(),
                target: commit.id.clone(),
            }));
            ui.close();
        }
        if tags.is_empty() {
            return Ok(());
        }
        let destructive = ui.visuals().error_fg_color;
        let (trigger, _) = egui::containers::menu::SubMenuButton::from_button(
            icon_button(view.text("git.deleteTag"))
                .right_text(egui::containers::menu::SubMenuButton::RIGHT_ARROW),
        )
        .ui(ui, |ui| {
            for tag in &tags {
                if ui
                    .button(egui::RichText::new(&tag.name).color(destructive))
                    .clicked()
                {
                    intents.push(ShellIntent::Git(Intent::DeleteTag {
                        project: project.clone(),
                        name: tag.name.clone(),
                    }));
                    ui.close();
                }
            }
        });
        paint_menu_icon(ui, &trigger, Icon::Tags, icons)
    });
    shown.map_or(Ok(()), |shown| shown.inner)
}

/// `CreateTagDialog`: the target, a name input that takes focus, an optional message, Cancel and
/// Create Tag (off while the trimmed name is empty or the tag is being created). Escape or a click
/// outside closes it; Enter does nothing. Each opening starts with empty inputs.
pub(crate) fn tag_dialog(
    context: &egui::Context,
    view: &View<'_>,
    dialog: &TagDialog,
    intents: &mut Vec<ShellIntent>,
) {
    let id = view.mounted_id(TAG_DIALOG_ID).with(dialog.opened);
    let mut draft: (String, String) = context.data(|data| data.get_temp(id)).unwrap_or_default();
    let focus_id = id.with("focused");
    let is_focused = context.data(|data| data.get_temp::<bool>(focus_id).unwrap_or_default());
    let response = egui::Modal::new(id).show(context, |ui| {
        ui.set_max_width(DIALOG_MAX_WIDTH);
        ui.heading(view.text("git.createTag"));
        ui.label(&dialog.target);
        let name = ui.add(
            egui::TextEdit::singleline(&mut draft.0)
                .id(id.with("name"))
                .hint_text(view.text("git.tagNamePlaceholder"))
                .return_key(None)
                .desired_width(f32::INFINITY),
        );
        ui.add(
            egui::TextEdit::singleline(&mut draft.1)
                .id(id.with("message"))
                .hint_text(view.text("git.tagMessagePlaceholder"))
                .return_key(None)
                .desired_width(f32::INFINITY),
        );
        if !is_focused {
            name.request_focus();
        }
        ui.horizontal(|ui| {
            let cancel = ui.button(view.text("common.cancel"));
            let name = draft.0.trim();
            let confirm = ui.add_enabled(
                !name.is_empty() && !dialog.is_pending,
                egui::Button::new(view.text("git.createTag")),
            );
            if cancel.clicked() {
                Some(None)
            } else if confirm.clicked() {
                Some(Some((name.to_owned(), draft.1.trim().to_owned())))
            } else {
                None
            }
        })
        .inner
    });
    context.data_mut(|data| {
        data.insert_temp(focus_id, true);
        data.insert_temp(id, draft.clone());
    });
    let (project, slot) = (view.project.clone(), view.slot.clone());
    match response.inner {
        Some(Some((name, message))) => intents.push(ShellIntent::Git(Intent::CreateTag {
            project,
            slot,
            name,
            message,
        })),
        Some(None) => intents.push(ShellIntent::Git(Intent::CloseTagDialog { project, slot })),
        None if response.should_close() => {
            intents.push(ShellIntent::Git(Intent::CloseTagDialog { project, slot }))
        }
        None => {}
    }
}

#[cfg(test)]
#[path = "git-graph-tests.rs"]
mod tests;

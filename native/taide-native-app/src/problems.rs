use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{
    self, Align, Color32, FontId, Id, Layout, Rect, Response, Sense, Stroke, Ui, vec2,
};
use taide_lsp::native::protocol::lsp_types::Diagnostic;
use taide_model::{
    error::AppResult, ids::ShellSlotId, locale::ResolvedLocale, project::ShellSlotTree,
    theme::ResolvedTheme,
};

use crate::diagnostics::{self, Store};
use crate::problems_icons::{self, FileColor, Glyph, Icons};

const SEVERITY_KEYS: [&str; 4] = ["error", "warning", "info", "hint"];
const ROW_HEIGHT: f32 = 20.0;
const OVERSCAN: usize = 12;
const TEXT_SIZE: f32 = 12.0;
const STATUS_TEXT_SIZE: f32 = 11.0;
const STATUS_LINE_HEIGHT: f32 = 1.5;
const BUTTON_RADIUS: u8 = 4;
const ICON_SIZE: f32 = 12.0;
const PROBLEM_ICON_SIZE: f32 = 14.0;
const EMPTY_ICON_SIZE: f32 = 20.0;
const PADDING: f32 = 8.0;
const GAP: f32 = 4.0;
const HEADER_PADDING: f32 = 6.0;
const PROBLEM_INDENT: f32 = 32.0;
const INACTIVE_OPACITY: f32 = 0.6;

pub(crate) struct Appearance {
    background: Color32,
    foreground: Color32,
    muted: Color32,
    border: Color32,
    selected: Color32,
    hover: Color32,
    tooltip: crate::tooltips::Appearance,
    severities: [Color32; 4],
    files: HashMap<FileColor, Color32>,
}

impl Appearance {
    pub(crate) fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        let color = |key| crate::presentation::color(theme, key);
        Ok(Self {
            background: color("panel.background")?,
            foreground: color("app.foreground")?,
            muted: color("appSidebar.iconDefault")?,
            border: color("app.border")?,
            selected: color("explorer.itemSelected")?,
            hover: color("explorer.itemHover")?,
            tooltip: crate::tooltips::Appearance::new(theme)?,
            severities: [
                color("statusIndicator.error")?,
                color("statusIndicator.warning")?,
                color("statusIndicator.info")?,
                color("appSidebar.iconDefault")?,
            ],
            files: FileColor::ALL
                .into_iter()
                .map(|key| Ok((key, color(key.theme_key())?)))
                .collect::<AppResult<_>>()?,
        })
    }
}

struct Group {
    path: Arc<PathBuf>,
    problems: Vec<Arc<Diagnostic>>,
    icon: (Glyph, FileColor),
}

enum Row {
    Group { group: usize, collapsed: bool },
    Problem { group: usize, index: usize },
}

enum PanelAction {
    Filter(usize),
    Collapse(Arc<PathBuf>),
    Open { path: Arc<PathBuf>, index: usize },
    Close,
}

struct Panel {
    viewport: Option<ScrollViewport>,
    spaces: SpaceInputs,
    active: [bool; 4],
    collapsed: HashSet<PathBuf>,
    revision: Option<Arc<()>>,
    groups: Vec<Group>,
    rows: Vec<Row>,
}

impl Default for Panel {
    fn default() -> Self {
        Self {
            viewport: None,
            spaces: SpaceInputs::default(),
            active: [true; 4],
            collapsed: HashSet::new(),
            revision: None,
            groups: Vec::new(),
            rows: Vec::new(),
        }
    }
}

impl Panel {
    fn refresh(&mut self, store: &Store, collation: &crate::keybinding_search::Search) {
        if self
            .revision
            .as_ref()
            .is_some_and(|revision| Arc::ptr_eq(revision, store.revision()))
        {
            return;
        }
        let mut grouped = HashMap::<PathBuf, Vec<Arc<Diagnostic>>>::new();
        for batch in store.batches() {
            for diagnostic in &batch.diagnostics {
                if self.active[diagnostics::severity(diagnostic)] {
                    grouped
                        .entry(batch.path.clone())
                        .or_default()
                        .push(diagnostic.clone());
                }
            }
        }
        self.groups = grouped
            .into_iter()
            .map(|(path, mut problems)| {
                problems.sort_by_key(|problem| {
                    (problem.range.start.line, problem.range.start.character)
                });
                Group {
                    icon: problems_icons::file(
                        &path.file_name().unwrap_or_default().to_string_lossy(),
                    ),
                    path: Arc::new(path),
                    problems,
                }
            })
            .collect();
        self.groups.sort_by(|left, right| {
            collation.compare(&left.path.to_string_lossy(), &right.path.to_string_lossy())
        });
        self.revision = Some(store.revision().clone());
        self.flatten();
    }

    fn flatten(&mut self) {
        self.rows.clear();
        for (group, entry) in self.groups.iter().enumerate() {
            let collapsed = self.collapsed.contains(entry.path.as_ref());
            self.rows.push(Row::Group { group, collapsed });
            if !collapsed {
                for index in 0..entry.problems.len() {
                    self.rows.push(Row::Problem { group, index });
                }
            }
        }
        if self.rows.is_empty() {
            self.viewport = None;
        } else if self.viewport.is_none() {
            self.viewport = Some(ScrollViewport {
                mount: uuid::Uuid::new_v4(),
                states: HashMap::new(),
            });
        }
    }
}

struct ScrollViewport {
    mount: uuid::Uuid,
    states: HashMap<egui::ViewportId, ScrollState>,
}

struct ScrollState {
    context: egui::Context,
    id: Id,
}

impl Drop for ScrollState {
    fn drop(&mut self) {
        self.context
            .data_mut(|data| data.remove::<egui::scroll_area::State>(self.id));
    }
}

#[derive(Default)]
struct SpaceInputs {
    armed: HashMap<egui::ViewportId, Id>,
}

pub(crate) struct Views {
    panels: HashMap<ShellSlotId, Panel>,
    collation: crate::keybinding_search::Search,
    icons: Icons,
    status_spaces: SpaceInputs,
    tooltips: crate::tooltips::Provider,
}

impl Views {
    #[cfg(test)]
    pub(crate) fn new(locale: &str) -> AppResult<Self> {
        Self::with_tooltips(locale, crate::tooltips::Provider::default())
    }

    pub(crate) fn with_tooltips(
        locale: &str,
        tooltips: crate::tooltips::Provider,
    ) -> AppResult<Self> {
        Ok(Self {
            panels: HashMap::new(),
            collation: crate::keybinding_search::Search::new(locale)?,
            icons: Icons::new()?,
            status_spaces: SpaceInputs::default(),
            tooltips,
        })
    }

    pub(crate) fn is_open(&self, slot: &ShellSlotId) -> bool {
        self.panels.contains_key(slot)
    }

    pub(crate) fn toggle(&mut self, slot: &ShellSlotId) {
        if self.panels.remove(slot).is_none() {
            self.panels.insert(slot.clone(), Panel::default());
        }
    }

    pub(crate) fn reconcile(&mut self, tree: Option<&ShellSlotTree>) {
        fn collect(tree: &ShellSlotTree, slots: &mut HashSet<ShellSlotId>) {
            match tree {
                ShellSlotTree::Leaf { slot_id, .. } => {
                    slots.insert(slot_id.clone());
                }
                ShellSlotTree::Split { children, .. } => {
                    for child in children {
                        collect(child, slots);
                    }
                }
            }
        }
        let mut slots = HashSet::new();
        if let Some(tree) = tree {
            collect(tree, &mut slots);
        }
        self.panels.retain(|slot, _| slots.contains(slot));
    }

    pub(crate) fn show_status(
        &mut self,
        ui: &mut Ui,
        slot: Option<&ShellSlotId>,
        store: &Store,
        locale: &ResolvedLocale,
        appearance: &Appearance,
    ) -> AppResult<Response> {
        let count = store.counts()[diagnostics::ERROR];
        let selected = slot.is_some_and(|slot| self.is_open(slot));
        let color = if count > 0 {
            appearance.severities[diagnostics::ERROR]
        } else {
            appearance.muted
        };
        let label = crate::presentation::message(locale, "problems.toggleAriaLabel", &[]);
        let response = icon_count_button(
            ui,
            appearance,
            &mut self.icons,
            &self.tooltips,
            CountButton {
                label: &label,
                count,
                severity: diagnostics::ERROR,
                selected,
                icon_color: color,
                text_color: color,
                opacity: 1.0,
                size: STATUS_TEXT_SIZE,
                padding: GAP,
                height: STATUS_TEXT_SIZE * STATUS_LINE_HEIGHT,
                tooltip_align: egui::RectAlign::TOP,
            },
        )?;
        let mut inputs = ActivationInputs::new(ui);
        let activations = inputs.activate(
            ui,
            &response,
            &mut self.status_spaces,
            SpaceActivation::Release,
        );
        if let Some(slot) = slot {
            for _ in activations {
                self.toggle(slot);
            }
        }
        Ok(response)
    }

    pub(crate) fn show_panel(
        &mut self,
        ui: &mut Ui,
        slot: &ShellSlotId,
        store: &Store,
        locale: &ResolvedLocale,
        appearance: &Appearance,
    ) -> AppResult<Vec<Open>> {
        let Self {
            panels,
            collation,
            icons,
            tooltips,
            ..
        } = self;
        let Some(panel) = panels.get_mut(slot) else {
            return Ok(Vec::new());
        };
        let mut inputs = ActivationInputs::new(ui);
        ui.painter()
            .rect_filled(ui.available_rect_before_wrap(), 0.0, appearance.background);
        panel.refresh(store, collation);
        let mut actions = Vec::new();
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(PADDING as i8, HEADER_PADDING as i8))
            .show(ui, |ui| -> AppResult<()> {
                ui.horizontal(|ui| -> AppResult<()> {
                    ui.spacing_mut().item_spacing.x = PADDING;
                    ui.label(
                        egui::RichText::new(crate::presentation::message(
                            locale,
                            "problems.title",
                            &[],
                        ))
                        .font(FontId::new(TEXT_SIZE, crate::ui_fonts::medium(ui)))
                        .color(appearance.foreground),
                    );
                    let filters = ui.horizontal(|ui| -> AppResult<()> {
                        ui.spacing_mut().item_spacing.x = GAP;
                        for (severity, key) in SEVERITY_KEYS.iter().enumerate() {
                            let label = crate::presentation::message(
                                locale,
                                &format!("problems.severity.{key}"),
                                &[],
                            );
                            let response = icon_count_button(
                                ui,
                                appearance,
                                icons,
                                tooltips,
                                CountButton {
                                    label: &label,
                                    count: store.counts()[severity],
                                    severity,
                                    selected: panel.active[severity],
                                    icon_color: appearance.severities[severity],
                                    text_color: if panel.active[severity] {
                                        appearance.foreground
                                    } else {
                                        appearance.muted
                                    },
                                    opacity: if panel.active[severity] {
                                        1.0
                                    } else {
                                        INACTIVE_OPACITY
                                    },
                                    size: TEXT_SIZE,
                                    padding: HEADER_PADDING,
                                    height: ROW_HEIGHT,
                                    tooltip_align: egui::RectAlign::BOTTOM,
                                },
                            )?;
                            let activations = inputs.activate(
                                ui,
                                &response,
                                &mut panel.spaces,
                                SpaceActivation::Release,
                            );
                            for order in activations {
                                actions.push((order, PanelAction::Filter(severity)));
                            }
                        }
                        Ok(())
                    });
                    filters.inner?;
                    let filter_label =
                        crate::presentation::message(locale, "problems.filterAriaLabel", &[]);
                    ui.ctx()
                        .accesskit_node_builder(filters.response.id, |node| {
                            node.set_role(egui::accesskit::Role::Group);
                            node.set_label(filter_label);
                        });
                    ui.with_layout(
                        Layout::right_to_left(Align::Center),
                        |ui| -> AppResult<()> {
                            let label = crate::presentation::message(locale, "common.close", &[]);
                            let (rect, response) = ui
                                .allocate_exact_size(vec2(ROW_HEIGHT, ROW_HEIGHT), Sense::click());
                            response.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    ui.is_enabled(),
                                    &label,
                                )
                            });
                            if response.hovered() {
                                ui.painter()
                                    .rect_filled(rect, BUTTON_RADIUS, appearance.hover);
                            }
                            icons.paint(
                                ui,
                                Rect::from_center_size(
                                    rect.center(),
                                    vec2(PROBLEM_ICON_SIZE, PROBLEM_ICON_SIZE),
                                ),
                                Glyph::X,
                                appearance.muted,
                                0.0,
                            )?;
                            for order in inputs.activate(
                                ui,
                                &response,
                                &mut panel.spaces,
                                SpaceActivation::Release,
                            ) {
                                actions.push((order, PanelAction::Close));
                            }
                            tooltips.show(
                                &response,
                                &label,
                                egui::RectAlign::BOTTOM,
                                &appearance.tooltip,
                            );
                            Ok(())
                        },
                    )
                    .inner?;
                    Ok(())
                })
                .inner?;
                Ok(())
            })
            .inner?;
        panel.refresh(store, collation);
        let y = ui.cursor().top();
        ui.painter().hline(
            ui.max_rect().x_range(),
            y,
            Stroke::new(1.0, appearance.border),
        );
        if panel.rows.is_empty() {
            let rect = ui.available_rect_before_wrap();
            ui.allocate_rect(rect, Sense::hover());
            let icon = Rect::from_center_size(
                rect.center() - vec2(0.0, TEXT_SIZE),
                vec2(EMPTY_ICON_SIZE, EMPTY_ICON_SIZE),
            );
            icons.paint(
                ui,
                icon,
                Glyph::CircleCheck,
                appearance.muted.gamma_multiply(INACTIVE_OPACITY),
                0.0,
            )?;
            let key = if store.counts().iter().any(|count| *count > 0) {
                "problems.emptyFiltered"
            } else {
                "problems.empty"
            };
            ui.painter().text(
                rect.center() + vec2(0.0, PADDING),
                egui::Align2::CENTER_TOP,
                crate::presentation::message(locale, key, &[]),
                FontId::proportional(TEXT_SIZE),
                appearance.muted,
            );
        } else {
            ui.spacing_mut().item_spacing.y = 0.0;
            let viewport_mount = panel.viewport.as_ref().map(|viewport| viewport.mount);
            let scroll = egui::ScrollArea::vertical()
                .id_salt(("problems", slot, viewport_mount))
                .auto_shrink([false, false])
                .show_viewport(ui, |ui, viewport| -> AppResult<()> {
                    let total = panel.rows.len();
                    ui.set_height(ROW_HEIGHT * total as f32);
                    let start = ((viewport.min.y / ROW_HEIGHT).floor().max(0.0) as usize)
                        .saturating_sub(OVERSCAN)
                        .min(total);
                    let end = ((viewport.max.y / ROW_HEIGHT).ceil().max(0.0) as usize)
                        .saturating_add(OVERSCAN)
                        .min(total);
                    for index in start..end {
                        let rect = Rect::from_min_size(
                            ui.max_rect().min + vec2(0.0, index as f32 * ROW_HEIGHT),
                            vec2(ui.available_width(), ROW_HEIGHT),
                        );
                        let (group, problem, collapsed) = match panel.rows[index] {
                            Row::Group { group, collapsed } => (group, None, collapsed),
                            Row::Problem { group, index } => (group, Some(index), false),
                        };
                        let entry = &panel.groups[group];
                        let response = ui.interact(
                            rect,
                            Id::new(("problem-row", slot, entry.path.as_ref(), problem)),
                            Sense::click(),
                        );
                        let label = match problem {
                            Some(index) => {
                                let diagnostic = &entry.problems[index];
                                let severity = crate::presentation::message(
                                    locale,
                                    &format!(
                                        "problems.severity.{}",
                                        SEVERITY_KEYS[diagnostics::severity(diagnostic)]
                                    ),
                                    &[],
                                );
                                format!(
                                    "{severity} {} {} {}:{}",
                                    diagnostic.message,
                                    diagnostic.source.as_deref().unwrap_or_default(),
                                    u64::from(diagnostic.range.start.line) + 1,
                                    u64::from(diagnostic.range.start.character) + 1
                                )
                            }
                            None => entry.path.to_string_lossy().into_owned(),
                        };
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                ui.is_enabled(),
                                &label,
                            )
                        });
                        if problem.is_none() {
                            ui.ctx().accesskit_node_builder(response.id, |node| {
                                node.set_expanded(!collapsed)
                            });
                        }
                        if response.hovered() || response.has_focus() {
                            ui.painter().rect_filled(rect, 0.0, appearance.hover);
                        }
                        let mut content = rect.shrink2(vec2(PADDING, 0.0));
                        if let Some(index) = problem {
                            let problem = &entry.problems[index];
                            let severity = diagnostics::severity(problem);
                            let icon = Rect::from_center_size(
                                rect.left_center()
                                    + vec2(PROBLEM_INDENT + PROBLEM_ICON_SIZE / 2.0, 0.0),
                                vec2(PROBLEM_ICON_SIZE, PROBLEM_ICON_SIZE),
                            );
                            icons.paint(
                                ui,
                                icon,
                                problems_icons::SEVERITIES[severity],
                                appearance.severities[severity],
                                0.0,
                            )?;
                            content.min.x = icon.right() + PADDING;
                            paint_tail(
                                ui,
                                &mut content,
                                &format!(
                                    "{}:{}",
                                    u64::from(problem.range.start.line) + 1,
                                    u64::from(problem.range.start.character) + 1
                                ),
                                appearance.muted,
                            );
                            if let Some(source) =
                                problem.source.as_ref().filter(|source| !source.is_empty())
                            {
                                paint_tail(ui, &mut content, source, appearance.muted);
                            }
                            paint_label(
                                ui,
                                content,
                                &problem.message,
                                appearance.foreground,
                                FontId::proportional(TEXT_SIZE),
                            );
                        } else {
                            let icon = Rect::from_center_size(
                                content.left_center() + vec2(ICON_SIZE / 2.0, 0.0),
                                vec2(ICON_SIZE, ICON_SIZE),
                            );
                            icons.paint(
                                ui,
                                icon,
                                Glyph::ChevronRight,
                                appearance.foreground,
                                if collapsed {
                                    0.0
                                } else {
                                    std::f32::consts::FRAC_PI_2
                                },
                            )?;
                            content.min.x = icon.right() + GAP;
                            let file_icon = Rect::from_center_size(
                                content.left_center() + vec2(PROBLEM_ICON_SIZE / 2.0, 0.0),
                                vec2(PROBLEM_ICON_SIZE, PROBLEM_ICON_SIZE),
                            );
                            icons.paint(
                                ui,
                                file_icon,
                                entry.icon.0,
                                appearance.files[&entry.icon.1],
                                0.0,
                            )?;
                            content.min.x = file_icon.right() + GAP;
                            paint_tail(
                                ui,
                                &mut content,
                                &entry.problems.len().to_string(),
                                appearance.muted,
                            );
                            let name = entry.path.file_name().unwrap_or_default().to_string_lossy();
                            let width = paint_label(
                                ui,
                                content,
                                &name,
                                appearance.foreground,
                                FontId::new(TEXT_SIZE, crate::ui_fonts::medium(ui)),
                            );
                            content.min.x += width + GAP;
                            paint_label(
                                ui,
                                content,
                                &entry.path.to_string_lossy(),
                                appearance.muted,
                                FontId::proportional(TEXT_SIZE),
                            );
                        }
                        let activations = inputs.activate(
                            ui,
                            &response,
                            &mut panel.spaces,
                            SpaceActivation::Press,
                        );
                        if !activations.is_empty() {
                            if let Some(index) = problem {
                                for order in activations {
                                    actions.push((
                                        order,
                                        PanelAction::Open {
                                            path: entry.path.clone(),
                                            index,
                                        },
                                    ));
                                }
                            } else {
                                for order in activations {
                                    actions
                                        .push((order, PanelAction::Collapse(entry.path.clone())));
                                }
                            }
                        }
                    }
                    Ok(())
                });
            if let Some(viewport) = panel.viewport.as_mut() {
                let live_viewports =
                    ui.input(|input| input.raw.viewports.keys().copied().collect::<HashSet<_>>());
                viewport.states.retain(|id, _| live_viewports.contains(id));
                viewport
                    .states
                    .entry(ui.ctx().viewport_id())
                    .and_modify(|state| {
                        if state.id != scroll.id || state.context != *ui.ctx() {
                            *state = ScrollState {
                                context: ui.ctx().clone(),
                                id: scroll.id,
                            };
                        }
                    })
                    .or_insert_with(|| ScrollState {
                        context: ui.ctx().clone(),
                        id: scroll.id,
                    });
            }
            scroll.inner?;
        }
        actions.sort_by_key(|(order, _)| *order);
        let mut open = Vec::new();
        let mut close = false;
        let mut should_repaint = false;
        for (_, action) in actions {
            match action {
                PanelAction::Filter(severity) => {
                    panel.active[severity] = !panel.active[severity];
                    panel.revision = None;
                    panel.refresh(store, collation);
                    should_repaint = true;
                }
                PanelAction::Collapse(path) => {
                    if !panel.groups.iter().any(|group| group.path == path) {
                        continue;
                    }
                    if !panel.collapsed.remove(path.as_ref()) {
                        panel.collapsed.insert(path.as_ref().clone());
                    }
                    panel.flatten();
                    should_repaint = true;
                }
                PanelAction::Open { path, index } => {
                    if panel.collapsed.contains(path.as_ref()) {
                        continue;
                    }
                    let Some(diagnostic) = panel
                        .groups
                        .iter()
                        .find(|group| group.path == path)
                        .and_then(|group| group.problems.get(index))
                    else {
                        continue;
                    };
                    open.push(Open {
                        path: path.to_string_lossy().into_owned(),
                        line: u64::from(diagnostic.range.start.line) + 1,
                        column: u64::from(diagnostic.range.start.character) + 1,
                    });
                }
                PanelAction::Close => {
                    close = true;
                    break;
                }
            }
        }
        if close || should_repaint {
            ui.ctx().request_repaint();
        }
        if close {
            panels.remove(slot);
        }
        Ok(open)
    }
}

pub(crate) struct Open {
    pub(crate) path: String,
    pub(crate) line: u64,
    pub(crate) column: u64,
}

fn paint_label(ui: &Ui, rect: Rect, text: &str, color: Color32, font: FontId) -> f32 {
    let text = text.split_ascii_whitespace().collect::<Vec<_>>().join(" ");
    let mut job = egui::text::LayoutJob::simple(text, font, color, rect.width().max(0.0));
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let galley = ui.painter().layout_job(job);
    let width = galley.size().x;
    let position = rect.left_center() - vec2(0.0, galley.size().y / 2.0);
    ui.painter()
        .with_clip_rect(rect.intersect(ui.clip_rect()))
        .galley(position, galley, color);
    width
}

fn paint_tail(ui: &Ui, rect: &mut Rect, text: &str, color: Color32) {
    let width = ui
        .painter()
        .layout_no_wrap(text.into(), FontId::proportional(TEXT_SIZE), color)
        .size()
        .x
        .min(rect.width().max(0.0));
    let tail = Rect::from_min_max(egui::pos2(rect.right() - width, rect.top()), rect.max);
    paint_label(ui, tail, text, color, FontId::proportional(TEXT_SIZE));
    rect.max.x = (tail.left() - PADDING).max(rect.min.x);
}

struct CountButton<'a> {
    label: &'a str,
    count: usize,
    severity: usize,
    selected: bool,
    icon_color: Color32,
    text_color: Color32,
    opacity: f32,
    size: f32,
    padding: f32,
    height: f32,
    tooltip_align: egui::RectAlign,
}

fn icon_count_button(
    ui: &mut Ui,
    appearance: &Appearance,
    icons: &mut Icons,
    tooltips: &crate::tooltips::Provider,
    button: CountButton<'_>,
) -> AppResult<Response> {
    let font = FontId::proportional(button.size);
    let text_color = button.text_color.gamma_multiply(button.opacity);
    let text = ui
        .painter()
        .layout_no_wrap(button.count.to_string(), font, text_color);
    let (rect, response) = ui.allocate_exact_size(
        vec2(
            ICON_SIZE + GAP + text.size().x + button.padding * 2.0,
            button.height,
        ),
        Sense::click(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Button,
            ui.is_enabled(),
            button.selected,
            button.label,
        )
    });
    if button.selected || response.hovered() {
        ui.painter().rect_filled(
            rect,
            BUTTON_RADIUS,
            (if button.selected {
                appearance.selected
            } else {
                appearance.hover
            })
            .gamma_multiply(button.opacity),
        );
    }
    icons.paint(
        ui,
        Rect::from_center_size(
            rect.left_center() + vec2(button.padding + ICON_SIZE / 2.0, 0.0),
            vec2(ICON_SIZE, ICON_SIZE),
        ),
        problems_icons::SEVERITIES[button.severity],
        button.icon_color.gamma_multiply(button.opacity),
        0.0,
    )?;
    ui.painter().galley(
        rect.left_center() + vec2(button.padding + ICON_SIZE + GAP, -text.size().y / 2.0),
        text,
        text_color,
    );
    tooltips.show(
        &response,
        button.label,
        button.tooltip_align,
        &appearance.tooltip,
    );
    Ok(response)
}

#[derive(Clone, Copy)]
enum SpaceActivation {
    Press,
    Release,
}

struct ActivationInputs {
    events: Vec<egui::Event>,
    consumed: HashSet<usize>,
}

impl ActivationInputs {
    fn new(ui: &Ui) -> Self {
        Self {
            events: ui.input(|input| input.events.clone()),
            consumed: HashSet::new(),
        }
    }

    fn activate(
        &mut self,
        ui: &mut Ui,
        response: &Response,
        spaces: &mut SpaceInputs,
        space: SpaceActivation,
    ) -> Vec<usize> {
        let viewport = ui.ctx().viewport_id();
        let has_focus_requests = self.events.iter().any(|event| {
            matches!(
                event,
                egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                    action: egui::accesskit::Action::Focus,
                    target_tree: egui::accesskit::TreeId::ROOT,
                    data: None,
                    ..
                })
            )
        });
        let window_focused = ui.input(|input| input.focused);
        let mut has_focus = response.enabled()
            && window_focused
            && if has_focus_requests {
                ui.memory(|memory| memory.had_focus_last_frame(response.id))
            } else {
                response.has_focus()
            };
        let mut activations = ui.input_mut(|input| {
            spaces
                .armed
                .retain(|id, _| input.raw.viewports.contains_key(id));
            let mut armed = has_focus && spaces.armed.get(&viewport) == Some(&response.id);
            let mut originals = self
                .events
                .iter()
                .enumerate()
                .filter(|(index, _)| !self.consumed.contains(index));
            let positions = input
                .events
                .iter()
                .map(|event| {
                    originals
                        .find(|(_, original)| *original == event)
                        .map(|(index, _)| index)
                })
                .collect::<Vec<_>>();
            let available = positions.iter().flatten().copied().collect::<HashSet<_>>();
            let mut used = HashSet::new();
            let mut activations = Vec::new();
            for (order, event) in self.events.iter().enumerate() {
                if let egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                    action: egui::accesskit::Action::Focus,
                    target_node,
                    target_tree: egui::accesskit::TreeId::ROOT,
                    data: None,
                }) = event
                {
                    has_focus = response.enabled()
                        && window_focused
                        && *target_node == response.id.accesskit_id();
                    if !has_focus {
                        armed = false;
                    }
                    continue;
                }
                if !available.contains(&order) {
                    continue;
                }
                if response.enabled()
                    && matches!(
                        event,
                        egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                            action: egui::accesskit::Action::Click,
                            target_node,
                            target_tree: egui::accesskit::TreeId::ROOT,
                            ..
                        }) if *target_node == response.id.accesskit_id()
                    )
                {
                    activations.push(order);
                    used.insert(order);
                }
                if !has_focus {
                    continue;
                }
                match event {
                    egui::Event::Key {
                        key: egui::Key::Space,
                        pressed: true,
                        repeat: false,
                        ..
                    } => match space {
                        SpaceActivation::Press => activations.push(order),
                        SpaceActivation::Release => armed = true,
                    },
                    egui::Event::Key {
                        key: egui::Key::Space,
                        pressed: false,
                        ..
                    } => {
                        if armed {
                            activations.push(order);
                        }
                        armed = false;
                    }
                    egui::Event::Key {
                        key: egui::Key::Enter,
                        pressed: true,
                        ..
                    } => activations.push(order),
                    _ => {}
                }
                if matches!(
                    event,
                    egui::Event::Key {
                        key: egui::Key::Space | egui::Key::Enter,
                        ..
                    }
                ) {
                    used.insert(order);
                }
            }
            let mut position = 0;
            input.events.retain(|_| {
                let keep = !positions[position].is_some_and(|index| used.contains(&index));
                position += 1;
                keep
            });
            self.consumed.extend(used);
            if armed {
                spaces.armed.insert(viewport, response.id);
            } else if spaces.armed.get(&viewport) == Some(&response.id) {
                spaces.armed.remove(&viewport);
            }
            activations
        });
        if response.clicked_by(egui::PointerButton::Primary) {
            activations.push(
                self.events
                    .iter()
                    .rposition(|event| {
                        matches!(
                            event,
                            egui::Event::PointerButton {
                                button: egui::PointerButton::Primary,
                                pressed: false,
                                ..
                            }
                        )
                    })
                    .unwrap_or(self.events.len()),
            );
            activations.sort_unstable();
        }
        activations
    }
}

#[cfg(test)]
#[path = "problems-tests.rs"]
mod tests;

use egui::emath::GuiRounding as _;
use egui::{
    Align, Color32, Id, Layout, Rect, RichText, Sense, Stroke, Ui, UiBuilder, Vec2, pos2, vec2,
};
use taide_model::ids::{PaneId, ProjectId, ShellSlotId, TabId};
use taide_model::layout::{DropEdge, PaneNode, SplitDir, Tab, TabKind};
use taide_model::project::{ProjectRef, ShellSlotTree};

use crate::commands::{ShellIntent, ShellMutation, request_close_tab};
use crate::snapshot::{
    ShellSnapshot, active_tab, project_group_sections, slot_count, slot_project,
};
use crate::split::{
    KEYBOARD_RESIZE_STEP, RESIZE_HIT_SIZE, child_rects, normalized_sizes, resized_pair,
};

pub const TITLE_HEIGHT: f32 = 28.0;
pub const STATUS_HEIGHT: f32 = 24.0;
pub const PROJECT_RAIL_WIDTH: f32 = 56.0;
pub const SLOT_HEADER_HEIGHT: f32 = 24.0;
pub const TAB_HEIGHT: f32 = 36.0;
pub const TAB_MIN_WIDTH: f32 = 96.0;
pub const TAB_MAX_WIDTH: f32 = 208.0;
pub const EXPLORER_DEFAULT_WIDTH: f32 = 240.0;
pub const EXPLORER_MIN_WIDTH: f32 = 180.0;
pub const EXPLORER_MAX_FRACTION: f32 = 0.4;
pub const TRAFFIC_LIGHT_INSET: f32 = 78.0;
const ICON_SIZE: f32 = 16.0;
const ICON_INSET: f32 = 4.0;
const RAIL_BUTTON_SIZE: f32 = 40.0;
const PADDING: f32 = 8.0;
const WELCOME_WIDTH: f32 = 384.0;
const WELCOME_GAP: f32 = 24.0;
const TAB_BAR_FILLER_MIN_WIDTH: f32 = 32.0;
const RAIL_ADD_ICON_SIZE: f32 = 20.0;
const RAIL_ADD_ICON_STROKE: f32 = 1.5;
const RAIL_RECENT_MAX_WIDTH: f32 = 288.0;
const WELCOME_SHORTCUT_COLUMNS: usize = 2;
const BORDER_WIDTH: f32 = 1.0;
const DIRTY_RADIUS: f32 = 3.0;
const TAB_ICON_SIZE: f32 = 14.0;
const TAB_ICON_GAP: f32 = 6.0;
const ADD_MENU_SIZE: f32 = 24.0;
const ADD_MENU_ICON_SIZE: f32 = 14.0;
const PROBLEMS_DEFAULT_HEIGHT: f32 = 220.0;
const PROBLEMS_MIN_HEIGHT: f32 = 120.0;
const PROBLEMS_MAX_FRACTION: f32 = 0.7;
const PROBLEMS_KEYBOARD_STEP: f32 = 0.05;
const PANEL_PERCENT_PRECISION: f32 = 1000.0;
const EMPTY_EDITOR_FONT_SIZE: f32 = 14.0;
const EMPTY_EDITOR_TEXT_OPACITY: f32 = 0.4;

#[derive(Clone, Copy)]
pub struct ShellColors {
    pub background: Color32,
    pub foreground: Color32,
    pub sidebar: Color32,
    pub muted: Color32,
    pub border: Color32,
    pub focus_border: Color32,
    pub tab_border: Color32,
    pub active_tab: Color32,
    pub inactive_tab: Color32,
    pub active_indicator: Color32,
    pub editor_background: Color32,
    pub editor_foreground: Color32,
}

/// A recent project row of the Welcome surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WelcomeRecent {
    pub id: ProjectId,
    pub name: String,
    pub root: String,
    pub root_missing: bool,
}

/// A shortcut row of the Welcome card with its localized description.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WelcomeShortcutRow {
    pub description: String,
    pub label: String,
}

/// Welcome data owned by the host; `recent` is `None` when the recent list failed to load.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WelcomeContent {
    pub recent: Option<Vec<WelcomeRecent>>,
    pub shortcuts: Vec<WelcomeShortcutRow>,
}

impl Default for WelcomeContent {
    fn default() -> Self {
        Self {
            recent: Some(Vec::new()),
            shortcuts: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub enum WindowScope {
    Main,
    Auxiliary { project: ProjectId, slot: u32 },
}

pub trait ShellSurfaces {
    fn text(&self, key: &str) -> String;
    fn text_with_args(&self, key: &str, arguments: &[(&str, &str)]) -> String;
    fn branch(&self, project: &ProjectId) -> Option<&str>;
    fn problems_open(&self, slot: &ShellSlotId) -> bool;
    fn problems_panel(&mut self, ui: &mut Ui, project: &ProjectId, slot: &ShellSlotId);
    fn explorer(
        &mut self,
        ui: &mut Ui,
        project: &ProjectId,
        slot: &ShellSlotId,
        intents: &mut Vec<ShellIntent>,
    );
    fn tab_content(
        &mut self,
        ui: &mut Ui,
        project: &ProjectId,
        pane: &PaneId,
        tab: &Tab,
        intents: &mut Vec<ShellIntent>,
    );
    fn tab_icon(&mut self, ui: &Ui, rect: Rect, tab: &Tab, title_color: Color32);
    fn status_bar(
        &mut self,
        ui: &mut Ui,
        project: Option<&ProjectId>,
        intents: &mut Vec<ShellIntent>,
    );
    /// Recent projects (already excluding `project` and limited) and shortcut rows to show.
    fn welcome_content(&self, _project: Option<&ProjectId>) -> WelcomeContent {
        WelcomeContent::default()
    }
    /// Recent projects for the rail `+` menu, already excluding open ones and limited.
    fn rail_recent_projects(&self) -> Vec<WelcomeRecent> {
        Vec::new()
    }
}

pub struct NativeShell {
    pub scope: WindowScope,
    pub colors: ShellColors,
    pub has_title_bar: bool,
}

#[derive(Clone, Copy)]
struct PaneViewContext<'a> {
    project: &'a ProjectId,
    root: Option<&'a str>,
    has_closed_tabs: bool,
    focused_pane: &'a PaneId,
    zen: bool,
    resizer_thickness: f32,
    welcome_on_empty: bool,
}

#[derive(Clone, Copy)]
struct SplitStyle {
    thickness: f32,
    border: Color32,
}

#[derive(Clone, Copy)]
struct ProblemsSplit {
    fraction: f32,
}

struct ProblemsRegions {
    editor: Rect,
    panel: Rect,
    divider: Rect,
    extent: f32,
    minimum: f32,
    maximum: f32,
}

impl NativeShell {
    pub fn show(
        &self,
        ui: &mut Ui,
        snapshot: &ShellSnapshot,
        surfaces: &mut impl ShellSurfaces,
    ) -> Vec<ShellIntent> {
        let mut intents = Vec::new();
        let rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(rect, 0.0, self.colors.background);
        ui.visuals_mut().override_text_color = Some(self.colors.foreground);
        match &self.scope {
            WindowScope::Main => self.main_window(ui, rect, snapshot, surfaces, &mut intents),
            WindowScope::Auxiliary { project, slot } => {
                if let Some(layout) = snapshot.layouts.get(project)
                    && let Some(window) = layout
                        .auxiliary_windows
                        .iter()
                        .find(|window| &window.slot == slot)
                {
                    let body = self.title_bar(
                        ui,
                        rect,
                        snapshot.project(project),
                        active_tab(&window.root, &window.focused_pane),
                        surfaces.branch(project),
                        surfaces,
                    );
                    self.pane_tree(
                        ui,
                        body,
                        &window.root,
                        PaneViewContext {
                            project,
                            root: snapshot
                                .project(project)
                                .map(|project| project.root.as_str()),
                            has_closed_tabs: !layout.closed_tabs.is_empty(),
                            focused_pane: &window.focused_pane,
                            zen: false,
                            resizer_thickness: snapshot.resizer_thickness,
                            welcome_on_empty: false,
                        },
                        surfaces,
                        &mut intents,
                    );
                }
            }
        }
        ui.allocate_rect(rect, Sense::hover());
        intents
    }

    fn main_window(
        &self,
        ui: &mut Ui,
        rect: Rect,
        snapshot: &ShellSnapshot,
        surfaces: &mut impl ShellSurfaces,
        intents: &mut Vec<ShellIntent>,
    ) {
        let focused = snapshot.focused_project();
        let project = focused.and_then(|project| snapshot.project(project));
        let branch = focused.and_then(|project| surfaces.branch(project));
        let mut body = self.title_bar(ui, rect, project, snapshot.focused_tab(), branch, surfaces);
        let zen = snapshot.shell.window_chrome.zen;
        if !(zen && snapshot.hide_status_in_zen) {
            let status = Rect::from_min_max(
                pos2(body.left(), (body.bottom() - STATUS_HEIGHT).max(body.top())),
                body.max,
            );
            ui.painter().rect_filled(status, 0.0, self.colors.sidebar);
            ui.painter().hline(
                status.x_range(),
                status.top(),
                Stroke::new(BORDER_WIDTH, self.colors.border),
            );
            child(ui, status.shrink2(vec2(PADDING, 0.0)), "status", |ui| {
                surfaces.status_bar(ui, focused, intents)
            });
            body.max.y = status.top();
        }
        if snapshot.projects.is_empty() {
            self.welcome(ui, body, None, surfaces, intents);
            return;
        }
        if !zen && !snapshot.shell.window_chrome.sidebar_rail_collapsed {
            let rail = Rect::from_min_max(
                body.min,
                pos2(
                    (body.left() + PROJECT_RAIL_WIDTH).min(body.right()),
                    body.bottom(),
                ),
            );
            self.project_rail(ui, rail, snapshot, surfaces, intents);
            body.min.x = rail.right();
        }
        let Some(tree) = snapshot.shell.tree.as_ref() else {
            child(ui, body, "select-project", |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label(surfaces.text("app.selectProject"));
                });
            });
            return;
        };
        self.slot_tree(ui, body, (tree, &[]), snapshot, surfaces, intents);
    }

    fn title_bar(
        &self,
        ui: &mut Ui,
        rect: Rect,
        project: Option<&ProjectRef>,
        tab: Option<&Tab>,
        branch: Option<&str>,
        surfaces: &impl ShellSurfaces,
    ) -> Rect {
        if !self.has_title_bar {
            return rect;
        }
        let title = Rect::from_min_max(
            rect.min,
            pos2(rect.right(), (rect.top() + TITLE_HEIGHT).min(rect.bottom())),
        );
        ui.painter().hline(
            title.x_range(),
            title.bottom(),
            Stroke::new(BORDER_WIDTH, self.colors.border),
        );
        let drag_region = ui.interact(
            title,
            ui.id().with("title-bar-drag-region"),
            Sense::CLICK | Sense::DRAG,
        );
        if drag_region.double_clicked() {
            let is_maximized = ui.input(|input| input.viewport().maximized.unwrap_or(false));
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
        } else if drag_region.drag_started_by(egui::PointerButton::Primary) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        let text_rect = title.shrink2(vec2(TRAFFIC_LIGHT_INSET.min(title.width() / 2.0), 0.0));
        child(ui, text_rect, "title", |ui| {
            ui.with_layout(
                Layout::centered_and_justified(egui::Direction::LeftToRight),
                |ui| {
                    let mut text = tab
                        .map(|tab| tab.title.clone())
                        .or_else(|| project.map(|project| project.name.clone()))
                        .unwrap_or_default();
                    if tab.is_some()
                        && let Some(project) = project
                    {
                        text.push_str(&format!(
                            " {} {}",
                            surfaces.text("window.titleSeparator"),
                            project.name
                        ));
                    }
                    if let Some(branch) = branch {
                        text.push_str(&format!("  {branch}"));
                    }
                    ui.add(egui::Label::new(text).truncate().selectable(false));
                },
            );
        });
        Rect::from_min_max(pos2(rect.left(), title.bottom()), rect.max)
    }

    fn project_rail(
        &self,
        ui: &mut Ui,
        rect: Rect,
        snapshot: &ShellSnapshot,
        surfaces: &impl ShellSurfaces,
        intents: &mut Vec<ShellIntent>,
    ) {
        ui.painter().rect_filled(rect, 0.0, self.colors.sidebar);
        ui.painter().vline(
            rect.right(),
            rect.y_range(),
            Stroke::new(BORDER_WIDTH, self.colors.border),
        );
        let footer = Rect::from_min_max(
            pos2(
                rect.left(),
                (rect.bottom() - RAIL_BUTTON_SIZE - PADDING).max(rect.top()),
            ),
            rect.max,
        );
        child(ui, footer, "rail-settings", |ui| {
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                if ui
                    .add_sized(
                        vec2(RAIL_BUTTON_SIZE, RAIL_BUTTON_SIZE),
                        egui::Button::new(surfaces.text("sidebar.settingsAriaLabel")),
                    )
                    .clicked()
                {
                    intents.push(ShellIntent::OpenSettings);
                }
            });
        });
        child(
            ui,
            Rect::from_min_max(rect.min, pos2(rect.right(), footer.top())).shrink(PADDING),
            "project-rail",
            |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let rail = project_group_sections(&snapshot.projects, &snapshot.groups);
                    for (group, members) in rail.sections {
                        if ui
                            .add(
                                egui::Button::new(&group.name)
                                    .wrap_mode(egui::TextWrapMode::Truncate),
                            )
                            .clicked()
                        {
                            intents.push(ShellIntent::Mutate(ShellMutation::SetGroupCollapsed {
                                group: group.id.clone(),
                                collapsed: !group.collapsed,
                            }));
                        }
                        if !group.collapsed {
                            for project in members {
                                self.project_button(
                                    ui,
                                    project,
                                    snapshot.focused_project(),
                                    surfaces,
                                    intents,
                                );
                            }
                        }
                    }
                    for project in rail.ungrouped {
                        self.project_button(
                            ui,
                            project,
                            snapshot.focused_project(),
                            surfaces,
                            intents,
                        );
                    }
                    self.add_project_menu(ui, surfaces, intents);
                });
            },
        );
    }

    /// The rail `+` menu (`sidebar-add-project-menu.tsx`).
    fn add_project_menu(
        &self,
        ui: &mut Ui,
        surfaces: &impl ShellSurfaces,
        intents: &mut Vec<ShellIntent>,
    ) {
        let label = surfaces.text("sidebar.addProjectMenu");
        let response = ui.add_sized(
            Vec2::splat(RAIL_BUTTON_SIZE),
            egui::Button::new("").frame(false),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &label)
        });
        let icon = Rect::from_center_size(response.rect.center(), Vec2::splat(RAIL_ADD_ICON_SIZE));
        let stroke = Stroke::new(RAIL_ADD_ICON_STROKE, self.colors.muted);
        ui.painter().hline(icon.x_range(), icon.center().y, stroke);
        ui.painter().vline(icon.center().x, icon.y_range(), stroke);
        let response = response.on_hover_text(label);
        egui::Popup::menu(&response)
            .align(egui::RectAlign::RIGHT_START)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                if menu_item(ui, surfaces.text("sidebar.openByPath")) {
                    intents.push(ShellIntent::OpenProjectByPath);
                }
                if menu_item(ui, surfaces.text("sidebar.openViaFinder")) {
                    intents.push(ShellIntent::OpenFolder);
                }
                let recent = surfaces.rail_recent_projects();
                if !recent.is_empty() {
                    ui.separator();
                    ui.label(
                        RichText::new(surfaces.text("app.recentItems"))
                            .small()
                            .color(self.colors.muted),
                    );
                    ui.set_max_width(RAIL_RECENT_MAX_WIDTH);
                    for project in &recent {
                        if welcome_recent_row(ui, project, surfaces, self.colors, intents) {
                            ui.close();
                        }
                    }
                }
            });
    }

    fn project_button(
        &self,
        ui: &mut Ui,
        project: &ProjectRef,
        focused: Option<&ProjectId>,
        surfaces: &impl ShellSurfaces,
        intents: &mut Vec<ShellIntent>,
    ) {
        let label = project
            .display
            .label
            .clone()
            .unwrap_or_else(|| project.name.chars().take(1).collect());
        ui.push_id(&project.id, |ui| {
            let response = ui.add_sized(
                vec2(RAIL_BUTTON_SIZE, RAIL_BUTTON_SIZE),
                egui::Button::selectable(focused == Some(&project.id), label),
            );
            response.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::WidgetType::SelectableLabel,
                    true,
                    focused == Some(&project.id),
                    &project.name,
                )
            });
            if response.clicked() {
                intents.push(ShellIntent::Mutate(ShellMutation::ActivateProject(
                    project.id.clone(),
                )));
            }
            let tooltip = if project.root_missing {
                format!(
                    "{}: {}",
                    project.name,
                    surfaces.text("app.recentProjectRootMissing")
                )
            } else {
                project.name.clone()
            };
            response.on_hover_text(tooltip);
        });
    }

    fn slot_tree(
        &self,
        ui: &mut Ui,
        rect: Rect,
        position: (&ShellSlotTree, &[u32]),
        snapshot: &ShellSnapshot,
        surfaces: &mut impl ShellSurfaces,
        intents: &mut Vec<ShellIntent>,
    ) {
        let (tree, path) = position;
        if snapshot.shell.window_chrome.zen {
            let focused = snapshot.shell.focused.as_ref();
            if focused.is_none_or(|focused| slot_project(tree, focused).is_none()) {
                return;
            }
            if let ShellSlotTree::Split { children, .. } = tree {
                for (index, child_node) in children.iter().enumerate() {
                    let mut child_path = path.to_vec();
                    child_path.push(index as u32);
                    self.slot_tree(
                        ui,
                        rect,
                        (child_node, &child_path),
                        snapshot,
                        surfaces,
                        intents,
                    );
                }
                return;
            }
        }
        match tree {
            ShellSlotTree::Leaf {
                slot_id,
                project_id,
            } => {
                self.project_shell(ui, rect, (slot_id, project_id), snapshot, surfaces, intents);
            }
            ShellSlotTree::Split {
                dir,
                children,
                sizes,
            } => {
                let id = Id::new(("shell-split", path));
                let (rects, resized) = split_regions(
                    ui,
                    rect,
                    id,
                    *dir,
                    sizes,
                    children.len(),
                    SplitStyle {
                        thickness: snapshot.resizer_thickness,
                        border: self.colors.border,
                    },
                );
                if let Some(sizes) = resized {
                    intents.push(ShellIntent::Mutate(ShellMutation::ResizeSlots {
                        path: path.to_vec(),
                        sizes,
                    }));
                }
                for (index, (node, child_rect)) in children.iter().zip(rects).enumerate() {
                    let mut child_path = path.to_vec();
                    child_path.push(index as u32);
                    self.slot_tree(
                        ui,
                        child_rect,
                        (node, &child_path),
                        snapshot,
                        surfaces,
                        intents,
                    );
                }
            }
        }
    }

    fn project_shell(
        &self,
        ui: &mut Ui,
        mut rect: Rect,
        target: (&ShellSlotId, &ProjectId),
        snapshot: &ShellSnapshot,
        surfaces: &mut impl ShellSurfaces,
        intents: &mut Vec<ShellIntent>,
    ) {
        let (slot, project) = target;
        let zen = snapshot.shell.window_chrome.zen;
        if !zen
            && snapshot
                .shell
                .tree
                .as_ref()
                .is_some_and(|tree| slot_count(tree) > 1)
        {
            let header = Rect::from_min_max(
                rect.min,
                pos2(
                    rect.right(),
                    (rect.top() + SLOT_HEADER_HEIGHT).min(rect.bottom()),
                ),
            );
            ui.painter().rect_filled(header, 0.0, self.colors.sidebar);
            child(
                ui,
                header.shrink2(vec2(PADDING, 0.0)),
                ("slot-header", slot),
                |ui| {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Label::new(
                                snapshot
                                    .project(project)
                                    .map(|project| project.name.as_str())
                                    .unwrap_or(project.as_str()),
                            )
                            .truncate(),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if close_button(
                                ui,
                                &surfaces.text("shellSlot.close"),
                                self.colors.foreground,
                            )
                            .clicked()
                            {
                                intents.push(ShellIntent::Mutate(ShellMutation::CloseSlot(
                                    slot.clone(),
                                )));
                            }
                        });
                    });
                },
            );
            rect.min.y = header.bottom();
        }
        let Some(layout) = snapshot.layouts.get(project) else {
            return;
        };
        let body = rect;
        let start = intents.len();
        if !zen && !layout.shell_view.sidebar_collapsed {
            child(ui, rect, ("project-explorer", slot), |ui| {
                egui::Panel::left(Id::new(("explorer", slot)))
                    .default_size(EXPLORER_DEFAULT_WIDTH)
                    .min_size(EXPLORER_MIN_WIDTH.min(rect.width() * EXPLORER_MAX_FRACTION))
                    .max_size(rect.width() * EXPLORER_MAX_FRACTION)
                    .frame(egui::Frame::NONE.fill(self.colors.sidebar))
                    .show(ui, |ui| surfaces.explorer(ui, project, slot, intents));
                rect = ui.available_rect_before_wrap();
            });
        }
        let problems_id = Id::new(("problems-divider", slot));
        let problems = if surfaces.problems_open(slot) {
            let regions = problems_regions(ui, rect, problems_id, snapshot.resizer_thickness);
            child(ui, regions.panel, ("project-problems", slot), |ui| {
                surfaces.problems_panel(ui, project, slot);
            });
            rect = regions.editor;
            Some(regions)
        } else {
            ui.data_mut(|data| data.remove::<ProblemsSplit>(problems_id));
            None
        };
        let mut editor_ui = problems.as_ref().map(|_| {
            let mut editor_ui = ui.new_child(
                UiBuilder::new()
                    .id_salt(("problems-editor", slot))
                    .max_rect(rect)
                    .layout(Layout::top_down(Align::Min)),
            );
            editor_ui.set_clip_rect(rect.intersect(ui.clip_rect()));
            editor_ui.set_min_size(rect.size());
            editor_ui
                .ctx()
                .accesskit_node_builder(editor_ui.unique_id(), |node| {
                    node.set_role(egui::accesskit::Role::Group);
                    node.set_bounds(egui::accesskit::Rect {
                        x0: f64::from(rect.left()),
                        y0: f64::from(rect.top()),
                        x1: f64::from(rect.right()),
                        y1: f64::from(rect.bottom()),
                    });
                });
            editor_ui
        });
        let editor_control = editor_ui.as_ref().map(Ui::unique_id);
        self.pane_tree(
            editor_ui.as_mut().unwrap_or(ui),
            rect,
            &layout.root,
            PaneViewContext {
                project,
                root: snapshot
                    .project(project)
                    .map(|project| project.root.as_str()),
                has_closed_tabs: !layout.closed_tabs.is_empty(),
                focused_pane: &layout.focused_pane,
                zen,
                resizer_thickness: snapshot.resizer_thickness,
                welcome_on_empty: snapshot.welcome_on_empty_editor,
            },
            surfaces,
            intents,
        );
        drop(editor_ui);
        if let (Some(regions), Some(editor_control)) = (problems, editor_control) {
            problems_resize(
                ui,
                problems_id,
                regions,
                (self.colors.border, self.colors.focus_border),
                editor_control,
            );
        }
        let pointer_in_body = ui.input(|input| {
            input.pointer.any_pressed()
                && input
                    .pointer
                    .interact_pos()
                    .is_some_and(|position| body.contains(position))
        });
        let widget_activated = !intents[start..].is_empty();
        if (pointer_in_body || widget_activated) && snapshot.shell.focused.as_ref() != Some(slot) {
            intents.insert(
                start,
                ShellIntent::Mutate(ShellMutation::FocusSlot(slot.clone())),
            );
        }
    }

    fn pane_tree(
        &self,
        ui: &mut Ui,
        rect: Rect,
        node: &PaneNode,
        context: PaneViewContext<'_>,
        surfaces: &mut impl ShellSurfaces,
        intents: &mut Vec<ShellIntent>,
    ) {
        let PaneViewContext {
            project,
            root,
            has_closed_tabs,
            focused_pane,
            zen,
            resizer_thickness,
            welcome_on_empty,
        } = context;
        match node {
            PaneNode::Split {
                id,
                dir,
                children,
                sizes,
            } => {
                let (rects, resized) = split_regions(
                    ui,
                    rect,
                    Id::new(("pane-split", project, id)),
                    *dir,
                    sizes,
                    children.len(),
                    SplitStyle {
                        thickness: resizer_thickness,
                        border: self.colors.border,
                    },
                );
                if let Some(sizes) = resized {
                    intents.push(ShellIntent::Mutate(ShellMutation::ResizePane {
                        pane: id.clone(),
                        sizes,
                    }));
                }
                for (child_node, child_rect) in children.iter().zip(rects) {
                    self.pane_tree(ui, child_rect, child_node, context, surfaces, intents);
                }
            }
            PaneNode::Leaf { id, tabs, active } => {
                let mut content = rect;
                if !zen {
                    let bar = Rect::from_min_max(
                        rect.min,
                        pos2(rect.right(), (rect.top() + TAB_HEIGHT).min(rect.bottom())),
                    );
                    ui.painter().rect_filled(bar, 0.0, self.colors.inactive_tab);
                    let add_width = ADD_MENU_SIZE.min(bar.width());
                    let add_right = (bar.right() - PADDING)
                        .max(bar.left() + add_width)
                        .min(bar.right());
                    let add_rect = Rect::from_center_size(
                        pos2(add_right - add_width / 2.0, bar.center().y),
                        vec2(add_width, ADD_MENU_SIZE.min(bar.height())),
                    );
                    let tabs_rect = Rect::from_min_max(
                        bar.min,
                        pos2((add_rect.left() - PADDING).max(bar.left()), bar.bottom()),
                    );
                    child(ui, tabs_rect, ("tab-bar", project, id), |ui| {
                        ui.style_mut().always_scroll_the_only_direction = true;
                        egui::ScrollArea::horizontal().show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let menu = TabMenuContext {
                                    project,
                                    pane: id,
                                    root,
                                    tabs,
                                    active: active.as_ref(),
                                    has_closed_tabs,
                                };
                                for tab in tabs {
                                    self.tab(
                                        ui,
                                        tab,
                                        active.as_ref() == Some(&tab.id),
                                        &menu,
                                        surfaces,
                                        intents,
                                    );
                                }
                                let filler = ui.allocate_response(
                                    vec2(
                                        ui.available_width().max(TAB_BAR_FILLER_MIN_WIDTH),
                                        TAB_HEIGHT,
                                    ),
                                    Sense::click(),
                                );
                                if filler.double_clicked() {
                                    intents.push(ShellIntent::NewUntitled {
                                        project: project.clone(),
                                        pane: id.clone(),
                                    });
                                }
                                filler
                                    .context_menu(|ui| tab_bar_menu(ui, &menu, surfaces, intents));
                            });
                        });
                    });
                    child(ui, add_rect, ("tab-add-menu", project, id), |ui| {
                        let response = ui.add_sized(
                            Vec2::splat(ADD_MENU_SIZE),
                            egui::Button::new("").frame(false),
                        );
                        let label = surfaces.text("tab.newTabMenu");
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                ui.is_enabled(),
                                &label,
                            )
                        });
                        let icon = Rect::from_center_size(
                            response.rect.center(),
                            Vec2::splat(ADD_MENU_ICON_SIZE),
                        );
                        let stroke = Stroke::new(BORDER_WIDTH, self.colors.muted);
                        ui.painter().line_segment(
                            [
                                pos2(icon.left(), icon.center().y),
                                pos2(icon.right(), icon.center().y),
                            ],
                            stroke,
                        );
                        ui.painter().line_segment(
                            [
                                pos2(icon.center().x, icon.top()),
                                pos2(icon.center().x, icon.bottom()),
                            ],
                            stroke,
                        );
                        let response = response.on_hover_text(label);
                        response.context_menu(|ui| {
                            tab_bar_menu(
                                ui,
                                &TabMenuContext {
                                    project,
                                    pane: id,
                                    root,
                                    tabs,
                                    active: active.as_ref(),
                                    has_closed_tabs,
                                },
                                surfaces,
                                intents,
                            )
                        });
                        egui::Popup::menu(&response).show(|ui| {
                            if ui.button(surfaces.text("tab.newUntitledFile")).clicked() {
                                intents.push(ShellIntent::NewUntitled {
                                    project: project.clone(),
                                    pane: id.clone(),
                                });
                                ui.close();
                            }
                            if ui.button(surfaces.text("tab.newTerminal")).clicked() {
                                intents.push(ShellIntent::NewTerminal {
                                    project: project.clone(),
                                    pane: id.clone(),
                                });
                                ui.close();
                            }
                        });
                    });
                    let border = if id == focused_pane {
                        self.colors.focus_border
                    } else {
                        self.colors.tab_border
                    };
                    ui.painter().hline(
                        bar.x_range(),
                        bar.bottom() - BORDER_WIDTH / 2.0,
                        Stroke::new(BORDER_WIDTH, border),
                    );
                    content.min.y = bar.bottom();
                }
                child(ui, content, ("pane-content", project, id), |ui| {
                    let shown_tab = tabs.iter().find(|tab| Some(&tab.id) == active.as_ref());
                    let shows_welcome = match shown_tab {
                        Some(tab) => matches!(tab.kind, TabKind::Welcome),
                        None => welcome_on_empty,
                    };
                    if shows_welcome {
                        self.welcome(ui, content, Some((project, id)), surfaces, intents);
                    } else if let Some(tab) = shown_tab {
                        surfaces.tab_content(ui, project, id, tab, intents);
                    } else {
                        ui.painter()
                            .rect_filled(content, 0.0, self.colors.editor_background);
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                RichText::new(surfaces.text("editor.noFileOpen"))
                                    .size(EMPTY_EDITOR_FONT_SIZE)
                                    .color(
                                        self.colors
                                            .editor_foreground
                                            .gamma_multiply(EMPTY_EDITOR_TEXT_OPACITY),
                                    ),
                            );
                        });
                    }
                });
                let focus_requested = ui.input(|input| {
                    input.pointer.any_pressed()
                        && input
                            .pointer
                            .interact_pos()
                            .is_some_and(|position| rect.contains(position))
                });
                if focus_requested {
                    intents.push(ShellIntent::Mutate(ShellMutation::FocusPane(id.clone())));
                }
            }
        }
    }

    fn tab(
        &self,
        ui: &mut Ui,
        tab: &Tab,
        active: bool,
        menu: &TabMenuContext<'_>,
        surfaces: &mut impl ShellSurfaces,
        intents: &mut Vec<ShellIntent>,
    ) {
        ui.push_id(&tab.id, |ui| {
            egui::Frame::NONE
                .fill(if active {
                    self.colors.active_tab
                } else {
                    self.colors.inactive_tab
                })
                .show(ui, |ui| {
                    ui.set_min_height(TAB_HEIGHT);
                    ui.set_max_width(TAB_MAX_WIDTH);
                    ui.set_min_width(TAB_MIN_WIDTH);
                    ui.horizontal(|ui| {
                        let icon_left = ui.cursor().left();
                        ui.add_space(TAB_ICON_SIZE + TAB_ICON_GAP);
                        let mut text = RichText::new(&tab.title);
                        if tab.preview {
                            text = text.italics();
                        }
                        let response = ui.add(
                            egui::Button::selectable(active, text)
                                .frame(false)
                                .wrap_mode(egui::TextWrapMode::Truncate),
                        );
                        let title_color = if active {
                            ui.visuals().selection.stroke.color
                        } else {
                            ui.visuals().text_color()
                        };
                        let icon_top = response.rect.center().y - TAB_ICON_SIZE / 2.0;
                        surfaces.tab_icon(
                            ui,
                            Rect::from_min_size(
                                pos2(icon_left, icon_top).round_to_pixels(ui.pixels_per_point()),
                                Vec2::splat(TAB_ICON_SIZE),
                            ),
                            tab,
                            title_color,
                        );
                        if response.clicked() {
                            intents.push(ShellIntent::Mutate(ShellMutation::ActivateTab(
                                tab.id.clone(),
                            )));
                        }
                        response.context_menu(|ui| tab_menu(ui, tab, menu, surfaces, intents));
                        if response.double_clicked() && tab.preview {
                            intents
                                .push(ShellIntent::Mutate(ShellMutation::KeepTab(tab.id.clone())));
                        }
                        if response.clicked_by(egui::PointerButton::Middle)
                            && let Ok(intent) = request_close_tab(tab)
                        {
                            intents.push(intent);
                        }
                        let label_key = if tab.pinned {
                            "tab.unpinAriaLabel"
                        } else {
                            "tab.closeAriaLabel"
                        };
                        let label = surfaces.text_with_args(label_key, &[("title", &tab.title)]);
                        let trailing = tab_action_button(ui, &label, tab, self.colors.foreground);
                        if trailing.clicked() {
                            if tab.pinned {
                                intents.push(ShellIntent::Mutate(ShellMutation::PinTab {
                                    tab: tab.id.clone(),
                                    pinned: false,
                                }));
                            } else if let Ok(intent) = request_close_tab(tab) {
                                intents.push(intent);
                            }
                        }
                    });
                    if active {
                        let rect = ui.min_rect();
                        ui.painter().hline(
                            rect.x_range(),
                            rect.top(),
                            Stroke::new(BORDER_WIDTH, self.colors.active_indicator),
                        );
                    }
                });
        });
    }

    fn welcome(
        &self,
        ui: &mut Ui,
        rect: Rect,
        target: Option<(&ProjectId, &PaneId)>,
        surfaces: &impl ShellSurfaces,
        intents: &mut Vec<ShellIntent>,
    ) {
        let width = WELCOME_WIDTH.min(rect.width());
        let left = rect.center().x - width / 2.0;
        let welcome = Rect::from_min_max(pos2(left, rect.top()), pos2(left + width, rect.bottom()));
        child(ui, welcome, "welcome", |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("TAIDE");
                ui.label(surfaces.text("app.openFolderHint"));
                ui.horizontal_wrapped(|ui| {
                    if ui.button(surfaces.text("app.openFolder")).clicked() {
                        intents.push(ShellIntent::OpenFolder);
                    }
                    if ui
                        .add_enabled(
                            target.is_some(),
                            egui::Button::new(surfaces.text("app.openFile")),
                        )
                        .clicked()
                        && let Some((project, pane)) = target
                    {
                        intents.push(ShellIntent::OpenFile {
                            project: project.clone(),
                            pane: pane.clone(),
                        });
                    }
                    if ui
                        .add_enabled(
                            target.is_some(),
                            egui::Button::new(surfaces.text("keymap.newTerminal")),
                        )
                        .clicked()
                        && let Some((project, pane)) = target
                    {
                        intents.push(ShellIntent::NewTerminal {
                            project: project.clone(),
                            pane: pane.clone(),
                        });
                    }
                });
                if target.is_none() {
                    ui.label(surfaces.text("app.openFileHint"));
                }
                let content = surfaces.welcome_content(target.map(|(project, _)| project));
                ui.add_space(WELCOME_GAP);
                match &content.recent {
                    None => {
                        ui.label(
                            RichText::new(surfaces.text("app.recentProjectsUnavailable"))
                                .small()
                                .color(self.colors.muted),
                        );
                    }
                    Some(recent) if !recent.is_empty() => {
                        ui.label(
                            RichText::new(surfaces.text("app.recentItems"))
                                .small()
                                .strong(),
                        );
                        for project in recent {
                            let _ = welcome_recent_row(ui, project, surfaces, self.colors, intents);
                        }
                        ui.add_space(WELCOME_GAP);
                    }
                    Some(_) => {}
                }
                if !content.shortcuts.is_empty() {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.label(
                            RichText::new(surfaces.text("app.keyboardShortcutsTitle"))
                                .small()
                                .strong(),
                        );
                        egui::Grid::new("welcome-shortcuts")
                            .num_columns(WELCOME_SHORTCUT_COLUMNS * 2)
                            .show(ui, |ui| {
                                for (index, shortcut) in content.shortcuts.iter().enumerate() {
                                    ui.label(
                                        RichText::new(&shortcut.description)
                                            .small()
                                            .color(self.colors.muted),
                                    );
                                    ui.label(RichText::new(&shortcut.label).small().monospace());
                                    if index % WELCOME_SHORTCUT_COLUMNS
                                        == WELCOME_SHORTCUT_COLUMNS - 1
                                    {
                                        ui.end_row();
                                    }
                                }
                            });
                    });
                }
            });
        });
    }
}

#[derive(Clone, Copy)]
struct TabMenuContext<'a> {
    project: &'a ProjectId,
    pane: &'a PaneId,
    root: Option<&'a str>,
    tabs: &'a [Tab],
    active: Option<&'a TabId>,
    has_closed_tabs: bool,
}

const SPLIT_EDGES: [(DropEdge, &str); 4] = [
    (DropEdge::Left, "editorArea.splitLeft"),
    (DropEdge::Right, "editorArea.splitRight"),
    (DropEdge::Top, "editorArea.splitTop"),
    (DropEdge::Bottom, "editorArea.splitBottom"),
];

fn menu_item(ui: &mut Ui, label: String) -> bool {
    let clicked = ui.button(label).clicked();
    if clicked {
        ui.close();
    }
    clicked
}

fn close_requests(tabs: impl Iterator<Item = Tab>) -> ShellIntent {
    ShellIntent::RequestCloseTabs(tabs.filter(|tab| !tab.pinned).map(|tab| tab.id).collect())
}

fn close_saved_and_all(
    ui: &mut Ui,
    menu: &TabMenuContext<'_>,
    surfaces: &impl ShellSurfaces,
    intents: &mut Vec<ShellIntent>,
) {
    if menu_item(ui, surfaces.text("tab.closeSaved")) {
        intents.push(close_requests(
            menu.tabs.iter().filter(|tab| !tab.dirty).cloned(),
        ));
    }
    if menu_item(ui, surfaces.text("tab.closeAll")) {
        intents.push(close_requests(menu.tabs.iter().cloned()));
    }
}

fn split_menu(
    ui: &mut Ui,
    pane: &PaneId,
    tab: &TabId,
    surfaces: &impl ShellSurfaces,
    intents: &mut Vec<ShellIntent>,
) {
    ui.menu_button(surfaces.text("tab.split"), |ui| {
        for (edge, key) in SPLIT_EDGES {
            if menu_item(ui, surfaces.text(key)) {
                intents.push(ShellIntent::Mutate(ShellMutation::SplitTab {
                    pane: pane.clone(),
                    edge,
                    tab: tab.clone(),
                }));
            }
        }
    });
}

/// The tab's own context menu (`tab-context-menu.tsx`). Entries whose native consumer does not
/// exist yet (explorer reveal/rename, Git changes/history, reopen with, window moves) are not shown.
fn tab_menu(
    ui: &mut Ui,
    tab: &Tab,
    menu: &TabMenuContext<'_>,
    surfaces: &impl ShellSurfaces,
    intents: &mut Vec<ShellIntent>,
) {
    if menu_item(ui, surfaces.text("tab.close")) {
        intents.push(ShellIntent::RequestCloseTab(tab.id.clone()));
    }
    if menu_item(ui, surfaces.text("tab.closeOthers")) {
        intents.push(close_requests(
            menu.tabs.iter().filter(|other| other.id != tab.id).cloned(),
        ));
    }
    if menu_item(ui, surfaces.text("tab.closeToRight")) {
        if let Some(index) = menu.tabs.iter().position(|other| other.id == tab.id) {
            intents.push(close_requests(menu.tabs[index + 1..].iter().cloned()));
        }
    }
    close_saved_and_all(ui, menu, surfaces, intents);
    ui.separator();
    let pin_key = if tab.pinned { "tab.unpin" } else { "tab.pin" };
    if menu_item(ui, surfaces.text(pin_key)) {
        intents.push(ShellIntent::Mutate(ShellMutation::PinTab {
            tab: tab.id.clone(),
            pinned: !tab.pinned,
        }));
    }
    if tab.preview && menu_item(ui, surfaces.text("tab.keepOpen")) {
        intents.push(ShellIntent::Mutate(ShellMutation::KeepTab(tab.id.clone())));
    }
    if let TabKind::File { path } = &tab.kind {
        ui.separator();
        if menu_item(ui, surfaces.text("explorer.copyPath")) {
            intents.push(ShellIntent::CopyText(path.clone()));
        }
        if let Some(root) = menu.root
            && menu_item(ui, surfaces.text("tab.copyRelativePath"))
        {
            intents.push(ShellIntent::CopyText(relative_path(root, path).to_owned()));
        }
        ui.separator();
        if menu_item(ui, surfaces.text("explorer.revealInFinder")) {
            intents.push(ShellIntent::RevealPath(path.clone()));
        }
    }
    ui.separator();
    split_menu(ui, menu.pane, &tab.id, surfaces, intents);
}

/// The tab bar's empty-space menu (`tab-bar-context-menu.tsx`, `buildTabBarMenuItems`).
fn tab_bar_menu(
    ui: &mut Ui,
    menu: &TabMenuContext<'_>,
    surfaces: &impl ShellSurfaces,
    intents: &mut Vec<ShellIntent>,
) {
    if menu_item(ui, surfaces.text("tab.newUntitledFile")) {
        intents.push(ShellIntent::NewUntitled {
            project: menu.project.clone(),
            pane: menu.pane.clone(),
        });
    }
    if menu_item(ui, surfaces.text("tab.newTerminal")) {
        intents.push(ShellIntent::NewTerminal {
            project: menu.project.clone(),
            pane: menu.pane.clone(),
        });
    }
    if menu.has_closed_tabs {
        ui.separator();
        if menu_item(ui, surfaces.text("keymap.reopenClosedTab")) {
            intents.push(ShellIntent::Mutate(ShellMutation::ReopenClosed(
                menu.project.clone(),
            )));
        }
    }
    if !menu.tabs.is_empty() {
        ui.separator();
        close_saved_and_all(ui, menu, surfaces, intents);
    }
    if let Some(active) = menu.active {
        ui.separator();
        split_menu(ui, menu.pane, active, surfaces, intents);
    }
    ui.separator();
    if menu_item(ui, surfaces.text("tab.openWelcome")) {
        intents.push(ShellIntent::OpenWelcome {
            project: menu.project.clone(),
            pane: menu.pane.clone(),
        });
    }
}

/// Original `toRelativePath`: strips `root/` when the path is inside it, otherwise keeps it.
fn relative_path<'a>(root: &str, path: &'a str) -> &'a str {
    let trimmed = root.strip_suffix('/').unwrap_or(root);
    path.strip_prefix(trimmed)
        .and_then(|rest| rest.strip_prefix('/'))
        .unwrap_or(path)
}

fn welcome_recent_row(
    ui: &mut Ui,
    project: &WelcomeRecent,
    surfaces: &impl ShellSurfaces,
    colors: ShellColors,
    intents: &mut Vec<ShellIntent>,
) -> bool {
    let mut detail = String::new();
    if project.root_missing {
        detail.push_str(&surfaces.text("app.recentProjectRootMissing"));
        detail.push(' ');
    }
    detail.push_str(&project.root);
    let mut job = egui::text::LayoutJob::default();
    job.append(
        &project.name,
        0.0,
        egui::TextFormat::simple(egui::TextStyle::Body.resolve(ui.style()), colors.foreground),
    );
    job.append(
        &format!("\n{detail}"),
        0.0,
        egui::TextFormat::simple(egui::TextStyle::Small.resolve(ui.style()), colors.muted),
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width(ui.available_width());
    let response = ui
        .add_enabled(
            !project.root_missing,
            egui::Button::new(job)
                .frame(false)
                .min_size(vec2(ui.available_width(), 0.0)),
        )
        .on_hover_text(&project.root)
        .on_disabled_hover_text(&project.root);
    let clicked = response.clicked();
    if clicked {
        intents.push(ShellIntent::SelectRecentProject {
            project: project.id.clone(),
            root: project.root.clone(),
        });
    }
    clicked
}

fn child<R>(
    ui: &mut Ui,
    rect: Rect,
    id: impl std::hash::Hash + std::fmt::Debug,
    contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    ui.scope_builder(
        UiBuilder::new()
            .id_salt(Id::new(id))
            .max_rect(rect)
            .layout(Layout::top_down(Align::Min)),
        |ui| {
            ui.set_clip_rect(rect.intersect(ui.clip_rect()));
            contents(ui)
        },
    )
    .inner
}

fn close_button(ui: &mut Ui, label: &str, color: Color32) -> egui::Response {
    let response = ui.add_sized(Vec2::splat(ICON_SIZE), egui::Button::new(""));
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    let rect = response.rect.shrink(ICON_INSET);
    ui.painter().line_segment(
        [rect.left_top(), rect.right_bottom()],
        Stroke::new(BORDER_WIDTH, color),
    );
    ui.painter().line_segment(
        [rect.right_top(), rect.left_bottom()],
        Stroke::new(BORDER_WIDTH, color),
    );
    response.on_hover_text(label)
}

fn tab_action_button(ui: &mut Ui, label: &str, tab: &Tab, color: Color32) -> egui::Response {
    let response = ui.add_sized(Vec2::splat(ICON_SIZE), egui::Button::new(""));
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    let stroke = Stroke::new(BORDER_WIDTH, color);
    let rect = response.rect.shrink(ICON_INSET);
    if tab.dirty && !response.hovered() {
        ui.painter()
            .circle_filled(response.rect.center(), DIRTY_RADIUS, color);
    } else if tab.pinned {
        ui.painter().hline(rect.x_range(), rect.top(), stroke);
        ui.painter().vline(rect.center().x, rect.y_range(), stroke);
        ui.painter().hline(rect.x_range(), rect.center().y, stroke);
    } else {
        ui.painter()
            .line_segment([rect.left_top(), rect.right_bottom()], stroke);
        ui.painter()
            .line_segment([rect.right_top(), rect.left_bottom()], stroke);
    }
    response.on_hover_text(label)
}

fn problems_regions(ui: &mut Ui, rect: Rect, id: Id, thickness: f32) -> ProblemsRegions {
    let thickness = thickness.max(0.0).min(rect.height().max(0.0));
    let extent = (rect.height() - thickness).max(0.0);
    if extent <= 0.0 {
        return ProblemsRegions {
            editor: rect,
            panel: Rect::from_min_max(rect.max, rect.max),
            divider: Rect::from_min_max(rect.max, rect.max),
            extent,
            minimum: 0.0,
            maximum: 0.0,
        };
    }
    let precision = crate::split::PERCENT_TOTAL * PANEL_PERCENT_PRECISION;
    let minimum_weight = ((PROBLEMS_MIN_HEIGHT / extent * precision).round() / precision).min(1.0);
    let (minimum, maximum) = if minimum_weight > PROBLEMS_MAX_FRACTION {
        let fraction = minimum_weight / (1.0 - PROBLEMS_MAX_FRACTION + minimum_weight);
        (fraction, fraction)
    } else {
        (minimum_weight, PROBLEMS_MAX_FRACTION)
    };
    let mut state = ui
        .data_mut(|data| data.get_temp::<ProblemsSplit>(id))
        .unwrap_or(ProblemsSplit {
            fraction: PROBLEMS_DEFAULT_HEIGHT / extent,
        });
    state.fraction = state.fraction.clamp(minimum, maximum);
    ui.data_mut(|data| data.insert_temp(id, state));
    let panel_top = rect.bottom() - extent * state.fraction;
    ProblemsRegions {
        editor: Rect::from_min_max(rect.min, pos2(rect.right(), panel_top - thickness)),
        panel: Rect::from_min_max(pos2(rect.left(), panel_top), rect.max),
        divider: Rect::from_min_max(
            pos2(rect.left(), panel_top - thickness),
            pos2(rect.right(), panel_top),
        ),
        extent,
        minimum,
        maximum,
    }
}

fn problems_resize(
    ui: &mut Ui,
    id: Id,
    regions: ProblemsRegions,
    colors: (Color32, Color32),
    editor_control: Id,
) {
    let ProblemsRegions {
        divider,
        extent,
        minimum,
        maximum,
        ..
    } = regions;
    if extent <= 0.0 {
        return;
    }
    let Some(mut state) = ui.data_mut(|data| data.get_temp::<ProblemsSplit>(id)) else {
        return;
    };
    let previous_fraction = state.fraction;
    let response = ui
        .interact(
            Rect::from_center_size(
                divider.center(),
                vec2(divider.width(), RESIZE_HIT_SIZE.max(divider.height())),
            ),
            id,
            Sense::drag(),
        )
        .on_hover_cursor(egui::CursorIcon::ResizeVertical);
    response.widget_info(|| egui::WidgetInfo::new(egui::WidgetType::ResizeHandle));
    if response.drag_started() {
        response.request_focus();
    }
    if response.dragged() {
        state.fraction =
            (state.fraction - response.drag_delta().y / extent).clamp(minimum, maximum);
    }
    if response.has_focus() {
        ui.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                id,
                egui::EventFilter {
                    vertical_arrows: true,
                    horizontal_arrows: true,
                    ..Default::default()
                },
            )
        });
        ui.input_mut(|input| {
            if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                state.fraction += PROBLEMS_KEYBOARD_STEP;
            }
            if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                state.fraction -= PROBLEMS_KEYBOARD_STEP;
            }
            if input.consume_key(egui::Modifiers::NONE, egui::Key::Home) {
                state.fraction = maximum;
            }
            if input.consume_key(egui::Modifiers::NONE, egui::Key::End) {
                state.fraction = minimum;
            }
            input.consume_key(egui::Modifiers::NONE, egui::Key::Enter);
            input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft);
            input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight);
            input.events.retain(|event| {
                !matches!(
                    event,
                    egui::Event::Key {
                        key: egui::Key::F6,
                        pressed: true,
                        ..
                    }
                )
            });
        });
        state.fraction = state.fraction.clamp(minimum, maximum);
    }
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_controls(vec![editor_control.accesskit_id()]);
        node.set_orientation(egui::accesskit::Orientation::Horizontal);
        let (value, min_value, max_value) = if maximum > PROBLEMS_MAX_FRACTION {
            let editor_weight = (1.0 - PROBLEMS_MAX_FRACTION) * crate::split::PERCENT_TOTAL;
            (editor_weight, editor_weight, editor_weight)
        } else {
            (
                (1.0 - state.fraction) * crate::split::PERCENT_TOTAL,
                (1.0 - maximum) * crate::split::PERCENT_TOTAL,
                (1.0 - minimum) * crate::split::PERCENT_TOTAL,
            )
        };
        node.set_numeric_value(f64::from(value));
        node.set_min_numeric_value(f64::from(min_value));
        node.set_max_numeric_value(f64::from(max_value));
    });
    if state.fraction != previous_fraction {
        ui.ctx().request_repaint();
    }
    ui.data_mut(|data| data.insert_temp(id, state));
    ui.painter().rect_filled(
        divider,
        0.0,
        if response.hovered() || response.dragged() {
            colors.1
        } else {
            colors.0
        },
    );
}

fn split_regions(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    dir: SplitDir,
    sizes: &[f32],
    count: usize,
    style: SplitStyle,
) -> (Vec<Rect>, Option<Vec<f32>>) {
    let SplitStyle { thickness, border } = style;
    let normalized = normalized_sizes(sizes, count);
    let mut live = ui
        .data_mut(|data| data.get_temp::<Vec<f32>>(id))
        .filter(|sizes| sizes.len() == count)
        .unwrap_or(normalized);
    let mut committed = None;
    let rects = child_rects(rect, dir, &live, thickness);
    let extent = match dir {
        SplitDir::Horizontal => rect.width(),
        SplitDir::Vertical => rect.height(),
    } - thickness * count.saturating_sub(1) as f32;
    for (index, pair) in rects.windows(2).enumerate() {
        let center = match dir {
            SplitDir::Horizontal => pos2((pair[0].right() + pair[1].left()) / 2.0, rect.center().y),
            SplitDir::Vertical => pos2(rect.center().x, (pair[0].bottom() + pair[1].top()) / 2.0),
        };
        let hit_size = match dir {
            SplitDir::Horizontal => vec2(RESIZE_HIT_SIZE, rect.height()),
            SplitDir::Vertical => vec2(rect.width(), RESIZE_HIT_SIZE),
        };
        let response = ui.interact(
            Rect::from_center_size(center, hit_size),
            id.with(index),
            Sense::drag(),
        );
        match dir {
            SplitDir::Horizontal => {
                ui.painter()
                    .vline(center.x, rect.y_range(), Stroke::new(thickness, border));
            }
            SplitDir::Vertical => {
                ui.painter()
                    .hline(rect.x_range(), center.y, Stroke::new(thickness, border));
            }
        }
        let delta = match dir {
            SplitDir::Horizontal => response.drag_delta().x,
            SplitDir::Vertical => response.drag_delta().y,
        };
        if response.dragged()
            && let Some(resized) = resized_pair(&live, index, delta, extent)
        {
            live = resized;
        }
        if response.drag_stopped() {
            committed = Some(live.clone());
            ui.data_mut(|data| data.remove::<Vec<f32>>(id));
        } else if response.dragged() {
            ui.data_mut(|data| data.insert_temp(id, live.clone()));
        }
        if response.has_focus() {
            let (decrease, increase) = match dir {
                SplitDir::Horizontal => (egui::Key::ArrowLeft, egui::Key::ArrowRight),
                SplitDir::Vertical => (egui::Key::ArrowUp, egui::Key::ArrowDown),
            };
            let delta = ui.input_mut(|input| {
                if input.consume_key(egui::Modifiers::NONE, decrease) {
                    -KEYBOARD_RESIZE_STEP
                } else if input.consume_key(egui::Modifiers::NONE, increase) {
                    KEYBOARD_RESIZE_STEP
                } else {
                    0.0
                }
            });
            if delta != 0.0 {
                committed = resized_pair(&live, index, delta, extent);
            }
        }
    }
    (child_rects(rect, dir, &live, thickness), committed)
}

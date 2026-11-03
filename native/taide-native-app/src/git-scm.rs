use std::collections::{HashMap, hash_map::Entry};
use std::f32::consts::{FRAC_PI_2, TAU};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Color32, FontFamily, FontId, Id, Key, Rect, Sense, Stroke, Ui, pos2, vec2,
};
use taide_model::error::{AppError, AppResult};
use taide_model::git::{
    CommitFile, GitBranch, GitChangeKind, GitRemote, GitStashEntry, GitStatus, LogEntry, StatusRow,
    TagInfo,
};
use taide_model::ids::{ProjectId, ShellSlotId};
use taide_model::locale::ResolvedLocale;
use taide_model::theme::ResolvedTheme;
use taide_native_ui::commands::ShellIntent;
use taide_native_ui::intents::git::{Intent, RowKey, Section};
use taide_native_ui::modal::Composition;

use crate::git_scm_model::{self as model, ChangeRow, Collapsed, ListRow, RowAction};
use crate::navigation_icons::{Icon, PaintIcon};
use crate::presentation;

const ROW_HEIGHT: f32 = 24.0;
const ROW_INDENT: f32 = 16.0;
const PADDING_X: f32 = 8.0;
const GAP: f32 = 6.0;
const TEXT_SIZE: f32 = 12.0;
const HEADER_TEXT_SIZE: f32 = 11.0;
const CHEVRON_SIZE: f32 = 12.0;
const EXPANDED_ANGLE: f32 = FRAC_PI_2;
const BORDER_WIDTH: f32 = 1.0;
const BADGE_HEIGHT: f32 = 16.0;
const BADGE_PADDING_X: f32 = 4.0;
const BADGE_TEXT_SIZE: f32 = 10.0;
const LETTER_WIDTH: f32 = 12.0;
const ACTION_SIZE: f32 = 16.0;
const ACTION_ICON_SIZE: f32 = 12.0;
const ACTION_GAP: f32 = 2.0;
const ACTION_RADIUS: u8 = 2;
const DIRECTORY_OPACITY: f32 = 0.7;
const EMPTY_PADDING_Y: f32 = 6.0;
const NOTICE_TEXT_SIZE: f32 = 14.0;
const NOTICE_GAP: f32 = 12.0;
const NOTICE_PADDING: f32 = 16.0;
const BUTTON_HEIGHT: f32 = 32.0;
const BUTTON_PADDING_X: f32 = 12.0;
const BUTTON_TEXT_SIZE: f32 = 14.0;
const BUTTON_RADIUS: u8 = 6;
const DISABLED_OPACITY: f32 = 0.5;
const ELLIPSIS: char = '…';
const LIST_ID: &str = "git-change-list";
const PENDING_FOCUS: &str = "pending-focus";
const STICKY_PADDING: f32 = ROW_HEIGHT;
const DISCARD_ID: &str = "git-discard-confirmation";
const DIALOG_MAX_WIDTH: f32 = 512.0;
const HEADER_HEIGHT: f32 = 32.0;
const HEADER_ICON_SIZE: f32 = 14.0;
const HEADER_BUTTON_SIZE: f32 = 20.0;
const COUNT_ICON_SIZE: f32 = 12.0;
const COUNT_GAP: f32 = 2.0;
const REMOTE_TEXT_SIZE: f32 = 11.0;
const REMOTE_MARGIN: f32 = 4.0;
const SPIN_SECONDS: f64 = 1.0;
const NO_CHANGES_HEIGHT: f32 = 28.0;
const STASH_ROW_HEIGHT: f32 = 30.0;
const STASH_APPLY_PADDING_X: f32 = 6.0;
const STASH_APPLY_HEIGHT: f32 = 20.0;
const STASH_DROP_SIZE: f32 = 22.0;
const COMMIT_PADDING_Y: f32 = 6.0;
const COMMIT_GAP: f32 = 6.0;
const COMMIT_ROWS: usize = 3;
const COMMIT_LINE_HEIGHT: f32 = 16.0;
const COMMIT_INPUT_PADDING_LEFT: f32 = 8.0;
const COMMIT_INPUT_PADDING_RIGHT: f32 = 28.0;
const COMMIT_INPUT_PADDING_Y: f32 = 6.0;
const INPUT_RADIUS: u8 = 4;
const AI_BUTTON_INSET: f32 = 6.0;
const REASON_TEXT_SIZE: f32 = 11.0;
const PRIMARY_HOVER_OPACITY: f32 = 0.9;
const STAGE_ALL_ID: &str = "git-stage-all-confirmation";
const GRAPH_MIN_HEIGHT: f32 = 120.0;
const GRAPH_RESIZE_COMMIT_DELAY: Duration = Duration::from_millis(120);
const GRAPH_SEPARATOR_ID: &str = "git-graph-separator";
pub(crate) const ARROWS: egui::EventFilter = egui::EventFilter {
    tab: false,
    horizontal_arrows: true,
    vertical_arrows: true,
    escape: false,
};
const STATUS_KINDS: [GitChangeKind; 6] = [
    GitChangeKind::Modified,
    GitChangeKind::Added,
    GitChangeKind::Deleted,
    GitChangeKind::Renamed,
    GitChangeKind::Untracked,
    GitChangeKind::Conflicted,
];

pub(crate) struct Appearance {
    pub(crate) panel: Color32,
    pub(crate) sidebar: Color32,
    pub(crate) foreground: Color32,
    pub(crate) muted: Color32,
    pub(crate) header: Color32,
    pub(crate) border: Color32,
    pub(crate) hover: Color32,
    focused: Color32,
    pub(crate) selected: Color32,
    pub(crate) badge: Color32,
    button: Color32,
    button_hover: Color32,
    pub(crate) button_hover_foreground: Color32,
    status: [(&'static str, Color32); STATUS_KINDS.len()],
    pub(crate) accent: Color32,
    pub(crate) list_active: Color32,
    pub(crate) popover_border: Color32,
    pub(crate) popover_separator: Color32,
    pub(crate) shadow: Color32,
    pub(crate) error: Color32,
    input: Color32,
    input_border: Color32,
    focus_border: Color32,
    primary: Color32,
    primary_foreground: Color32,
    pub(crate) lanes: [Color32; crate::git_graph::LANE_COLOR_COUNT],
    pub(crate) ref_branch: Color32,
}

impl Appearance {
    pub(crate) fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        let color = |key| presentation::color(theme, key);
        let mut status =
            STATUS_KINDS.map(|kind| (model::status_color_key(kind), Color32::TRANSPARENT));
        for (key, value) in status.iter_mut() {
            *value = color(key)?;
        }
        let mut lanes = [Color32::TRANSPARENT; crate::git_graph::LANE_COLOR_COUNT];
        for (lane, value) in lanes.iter_mut().enumerate() {
            *value = presentation::color(theme, &format!("graph.lane{}", lane + 1))?;
        }
        Ok(Self {
            panel: color("panel.background")?,
            sidebar: color("explorer.background")?,
            foreground: color("app.foreground")?,
            muted: color("appSidebar.iconDefault")?,
            header: color("panel.sectionHeader")?,
            border: color("app.border")?,
            hover: color("explorer.itemHover")?,
            focused: color("explorer.itemFocused")?,
            selected: color("explorer.itemSelected")?,
            badge: color("appSidebar.itemActive")?,
            button: color("app.background")?,
            button_hover: color("list.hoverBackground")?,
            button_hover_foreground: color("list.foreground")?,
            status,
            accent: color("app.accent")?,
            list_active: color("list.activeBackground")?,
            popover_border: color("popover.border")?,
            popover_separator: color("popover.separator")?,
            shadow: color("app.shadow")?,
            error: color("statusIndicator.error")?,
            input: color("panel.inputBackground")?,
            input_border: color("panel.inputBorder")?,
            focus_border: color("app.focusBorder")?,
            primary: color("button.primaryBackground")?,
            primary_foreground: color("button.primaryForeground")?,
            lanes,
            ref_branch: color("graph.refBranch")?,
        })
    }

    pub(crate) fn status(&self, kind: GitChangeKind) -> Color32 {
        let key = model::status_color_key(kind);
        self.status
            .iter()
            .find(|(status, _)| *status == key)
            .map_or(self.foreground, |(_, color)| *color)
    }
}

/// The create tag dialog of one panel (`commit-graph.tsx` `tagDialogTarget`): `opened` tells one
/// opening's input drafts from the next.
pub(crate) struct TagDialog {
    pub(crate) target: String,
    pub(crate) opened: u64,
    pub(crate) is_pending: bool,
}

pub(crate) struct Panel {
    project: ProjectId,
    mount: u64,
    collapsed: Collapsed,
    is_shown: bool,
    selected: Vec<RowKey>,
    anchor: Option<RowKey>,
    menu: Option<RowKey>,
    discard: Option<Vec<String>>,
    stage_all: bool,
    selected_commit: Option<String>,
    graph_menu: Option<String>,
    tag: Option<TagDialog>,
    graph_height: Option<f32>,
}

impl Panel {
    pub(crate) fn selected_commit(&self) -> Option<&str> {
        self.selected_commit.as_deref()
    }

    pub(crate) fn graph_menu(&self) -> Option<&str> {
        self.graph_menu.as_deref()
    }

    pub(crate) fn tag(&self) -> Option<&TagDialog> {
        self.tag.as_ref()
    }
}

/// The source control panels mounted this frame, one per sidebar slot: each reads the stored
/// collapse list when it mounts and keeps its own copy until it unmounts (`git-panel.tsx`). The
/// graph height saves wait per slot and outlive the panel (`pane-resize-commit.ts`).
#[derive(Default)]
pub(crate) struct Panels {
    entries: HashMap<ShellSlotId, Panel>,
    mounts: u64,
    tag_opens: u64,
    resizes: HashMap<ShellSlotId, (u32, Instant)>,
}

impl Panels {
    pub(crate) fn collapsed(&self, project: &ProjectId, slot: &ShellSlotId) -> Option<Collapsed> {
        self.entries
            .get(slot)
            .filter(|panel| panel.project == *project)
            .map(|panel| panel.collapsed)
    }

    fn mount(
        &mut self,
        project: &ProjectId,
        slot: &ShellSlotId,
        stored: impl FnOnce() -> Collapsed,
    ) -> &mut Panel {
        let mount = self.mounts + 1;
        let mounted = |collapsed| Panel {
            project: project.clone(),
            mount,
            collapsed,
            is_shown: false,
            selected: Vec::new(),
            anchor: None,
            menu: None,
            discard: None,
            stage_all: false,
            selected_commit: None,
            graph_menu: None,
            tag: None,
            graph_height: None,
        };
        match self.entries.entry(slot.clone()) {
            Entry::Occupied(entry) if entry.get().project == *project => entry.into_mut(),
            Entry::Occupied(mut entry) => {
                self.mounts = mount;
                entry.insert(mounted(stored()));
                entry.into_mut()
            }
            Entry::Vacant(entry) => {
                self.mounts = mount;
                entry.insert(mounted(stored()))
            }
        }
    }

    pub(crate) fn shown(
        &mut self,
        project: &ProjectId,
        slot: &ShellSlotId,
        stored: impl FnOnce() -> Collapsed,
    ) {
        self.mount(project, slot, stored).is_shown = true;
    }

    pub(crate) fn toggle(
        &mut self,
        project: &ProjectId,
        slot: &ShellSlotId,
        section: Section,
        stored: impl FnOnce() -> Collapsed,
    ) -> Collapsed {
        let panel = self.mount(project, slot, stored);
        panel.collapsed = panel.collapsed.toggled(section);
        if section == Section::Graph {
            panel.graph_height = None;
        }
        panel.collapsed
    }

    pub(crate) fn panel(&self, project: &ProjectId, slot: &ShellSlotId) -> Option<&Panel> {
        self.entries
            .get(slot)
            .filter(|panel| panel.project == *project)
    }

    fn panel_mut(&mut self, project: &ProjectId, slot: &ShellSlotId) -> Option<&mut Panel> {
        self.entries
            .get_mut(slot)
            .filter(|panel| panel.project == *project)
    }

    pub(crate) fn select(
        &mut self,
        project: &ProjectId,
        slot: &ShellSlotId,
        selected: Vec<RowKey>,
        anchor: Option<RowKey>,
    ) {
        if let Some(panel) = self.panel_mut(project, slot) {
            panel.selected = selected;
            panel.anchor = anchor;
        }
    }

    pub(crate) fn open_menu(&mut self, project: &ProjectId, slot: &ShellSlotId, target: RowKey) {
        if let Some(panel) = self.panel_mut(project, slot) {
            panel.menu = Some(target);
        }
    }

    pub(crate) fn request_discard(
        &mut self,
        project: &ProjectId,
        slot: &ShellSlotId,
        paths: Vec<String>,
    ) {
        if let Some(panel) = self.panel_mut(project, slot) {
            panel.discard = Some(paths);
        }
    }

    pub(crate) fn take_discard(
        &mut self,
        project: &ProjectId,
        slot: &ShellSlotId,
    ) -> Option<Vec<String>> {
        self.panel_mut(project, slot)
            .and_then(|panel| panel.discard.take())
    }

    pub(crate) fn request_stage_all(&mut self, project: &ProjectId, slot: &ShellSlotId) {
        if let Some(panel) = self.panel_mut(project, slot) {
            panel.stage_all = true;
        }
    }

    /// Closes the stage-all confirmation, reporting whether it was open.
    pub(crate) fn take_stage_all(&mut self, project: &ProjectId, slot: &ShellSlotId) -> bool {
        self.panel_mut(project, slot)
            .is_some_and(|panel| std::mem::take(&mut panel.stage_all))
    }

    pub(crate) fn select_commit(
        &mut self,
        project: &ProjectId,
        slot: &ShellSlotId,
        commit: Option<String>,
    ) {
        if let Some(panel) = self.panel_mut(project, slot) {
            panel.selected_commit =
                commit.filter(|commit| panel.selected_commit.as_ref() != Some(commit));
        }
    }

    pub(crate) fn open_graph_menu(
        &mut self,
        project: &ProjectId,
        slot: &ShellSlotId,
        commit: String,
    ) {
        if let Some(panel) = self.panel_mut(project, slot) {
            panel.graph_menu = Some(commit);
        }
    }

    /// Opens the slot's create tag dialog on `target`, mounting the panel when the request comes
    /// before the Git view shows (`git.createTagOnHead`). An open dialog keeps its inputs and only
    /// takes the new target, as the mounted dialog of the original does.
    pub(crate) fn request_tag(
        &mut self,
        project: &ProjectId,
        slot: &ShellSlotId,
        target: String,
        stored: impl FnOnce() -> Collapsed,
    ) {
        let opened = self.tag_opens + 1;
        let panel = self.mount(project, slot, stored);
        panel.is_shown = true;
        match &mut panel.tag {
            Some(dialog) => dialog.target = target,
            None => {
                panel.tag = Some(TagDialog {
                    target,
                    opened,
                    is_pending: false,
                });
                self.tag_opens = opened;
            }
        }
    }

    pub(crate) fn close_tag(&mut self, project: &ProjectId, slot: &ShellSlotId) {
        if let Some(panel) = self.panel_mut(project, slot) {
            panel.tag = None;
        }
    }

    /// Marks the slot's open tag dialog as creating and returns its target, unless it is already.
    pub(crate) fn begin_tag(&mut self, project: &ProjectId, slot: &ShellSlotId) -> Option<String> {
        let dialog = self.panel_mut(project, slot)?.tag.as_mut()?;
        if dialog.is_pending {
            return None;
        }
        dialog.is_pending = true;
        Some(dialog.target.clone())
    }

    /// Ends a tag creation: success closes the dialog, failure leaves it open for another try.
    pub(crate) fn finish_tag(&mut self, project: &ProjectId, slot: &ShellSlotId, is_created: bool) {
        let Some(panel) = self.panel_mut(project, slot) else {
            return;
        };
        if is_created {
            panel.tag = None;
        } else if let Some(dialog) = &mut panel.tag {
            dialog.is_pending = false;
        }
    }

    /// Re-enables every tag dialog whose creation will not answer (the host connection restarted).
    pub(crate) fn cancel_pending_tags(&mut self) {
        for dialog in self
            .entries
            .values_mut()
            .filter_map(|panel| panel.tag.as_mut())
        {
            dialog.is_pending = false;
        }
    }

    /// `handleGraphLayoutChanged`: a drag that crosses the collapse threshold records the collapse
    /// and stores no height, returning the new collapse flags to persist; any other drag keeps the
    /// pane at its new height and saves that height after the resize delay.
    pub(crate) fn resize_graph(
        &mut self,
        project: &ProjectId,
        slot: &ShellSlotId,
        height: Option<f32>,
        now: Instant,
    ) -> Option<Collapsed> {
        let panel = self.panel_mut(project, slot)?;
        if height.is_some() {
            panel.graph_height = height;
        }
        if panel.collapsed.is_collapsed(Section::Graph) != height.is_none() {
            panel.collapsed = panel.collapsed.toggled(Section::Graph);
            return Some(panel.collapsed);
        }
        let size = height?.round() as u32;
        self.resizes
            .insert(slot.clone(), (size, now + GRAPH_RESIZE_COMMIT_DELAY));
        None
    }

    /// The graph heights whose save delay ran out, removed from the waiting saves.
    pub(crate) fn take_due_resizes(&mut self, now: Instant) -> Vec<u32> {
        let mut due = Vec::new();
        self.resizes.retain(|_, (size, at)| {
            let is_due = *at <= now;
            if is_due {
                due.push(*size);
            }
            !is_due
        });
        due
    }

    /// How long until the next waiting graph height save is due.
    pub(crate) fn next_resize(&self, now: Instant) -> Option<std::time::Duration> {
        self.resizes
            .values()
            .map(|(_, at)| at.saturating_duration_since(now))
            .min()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn finish_frame(&mut self) {
        self.entries
            .retain(|_, panel| std::mem::take(&mut panel.is_shown));
    }
}

/// What the commit box shows for this panel's project (`commit-box.tsx` props).
#[derive(Default)]
pub(crate) struct Commit<'a> {
    pub message: &'a str,
    pub is_committing: bool,
    pub is_generating: bool,
}

/// What the graph pane shows for this panel's project (`git-panel.tsx` graph props and settings).
#[derive(Default)]
pub(crate) struct Graph<'a> {
    pub log: &'a [LogEntry],
    pub tags: &'a [TagInfo],
    pub commit_files: Option<Result<&'a [CommitFile], &'a AppError>>,
    pub size: f32,
    pub resizer_thickness: f32,
    pub now_unix: f64,
}

pub(crate) struct View<'a> {
    pub project: &'a ProjectId,
    pub slot: &'a ShellSlotId,
    pub status: Option<Result<&'a GitStatus, &'a AppError>>,
    pub collapsed: Collapsed,
    pub panel: Option<&'a Panel>,
    pub is_initializing: bool,
    pub remotes: &'a [GitRemote],
    pub branches: &'a [GitBranch],
    pub stashes: &'a [GitStashEntry],
    pub is_stashing: bool,
    pub is_syncing: bool,
    pub commit: Commit<'a>,
    pub graph: Graph<'a>,
    pub appearance: &'a Appearance,
    pub locale: &'a ResolvedLocale,
    pub tooltips: &'a crate::tooltips::Provider,
    pub tooltip_appearance: &'a crate::tooltips::Appearance,
}

impl View<'_> {
    pub(crate) fn text(&self, key: &str) -> String {
        presentation::message(self.locale, key, &[])
    }

    /// Widget state that lives as long as this slot's mounted panel (`git-panel.tsx` local state).
    pub(crate) fn mounted_id(&self, name: &str) -> Id {
        Id::new((name, self.slot, self.panel.map(|panel| panel.mount)))
    }

    fn icon_button(
        &self,
        ui: &mut Ui,
        (bounds, id): (Rect, Id),
        (icon, angle): (Icon, f32),
        (label, side): (&str, egui::RectAlign),
        is_enabled: bool,
        icons: &mut impl PaintIcon<Icon>,
    ) -> AppResult<bool> {
        let response = ui.interact(
            bounds,
            id,
            if is_enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, is_enabled, label));
        self.tooltips
            .show(&response, label, side, self.tooltip_appearance);
        if is_enabled && response.hovered() {
            ui.painter()
                .rect_filled(bounds, ACTION_RADIUS, self.appearance.hover);
        }
        let opacity = if is_enabled { 1.0 } else { DISABLED_OPACITY };
        icons.paint(
            ui,
            Rect::from_center_size(bounds.center(), vec2(HEADER_ICON_SIZE, HEADER_ICON_SIZE)),
            icon,
            self.appearance.foreground.gamma_multiply(opacity),
            angle,
        )?;
        if response.has_focus() {
            ui.memory_mut(|memory| memory.set_focus_lock_filter(id, ARROWS));
        }
        Ok(is_enabled && response.clicked())
    }

    fn selected(&self) -> &[RowKey] {
        self.panel.map_or(&[], |panel| &panel.selected)
    }

    fn row_intent(&self, section: Section, row: &StatusRow, action: RowAction) -> ShellIntent {
        let project = self.project.clone();
        ShellIntent::Git(match action {
            RowAction::OpenFile => Intent::OpenFile {
                project,
                path: row.abs_path.clone(),
            },
            RowAction::OpenChanges => Intent::OpenChanges {
                project,
                path: row.abs_path.clone(),
                staged: section == Section::Staged,
                before_path: row.orig_abs_path.clone(),
            },
            RowAction::Stage => Intent::Stage {
                project,
                paths: vec![row.path.clone()],
            },
            RowAction::Unstage => Intent::Unstage {
                project,
                paths: vec![row.path.clone()],
            },
            RowAction::Discard => Intent::RequestDiscard {
                project,
                slot: self.slot.clone(),
                paths: vec![row.path.clone()],
            },
            RowAction::CopyPath => return ShellIntent::CopyText(row.abs_path.clone()),
            RowAction::Reveal => return ShellIntent::RevealPath(row.abs_path.clone()),
        })
    }
}

pub(crate) fn show(
    ui: &mut Ui,
    view: &View<'_>,
    icons: &mut impl PaintIcon<Icon>,
    intents: &mut Vec<ShellIntent>,
) -> AppResult<()> {
    let result = match view.status {
        Some(Err(_)) => {
            not_a_repository(ui, view, intents);
            Ok(())
        }
        Some(Ok(status)) => header(ui, view, Some(status), icons, intents)
            .and_then(|()| commit_box(ui, view, &status.rows, icons, intents))
            .and_then(|()| change_list(ui, view, &status.rows, icons, intents)),
        None => header(ui, view, None, icons, intents)
            .and_then(|()| commit_box(ui, view, &[], icons, intents))
            .and_then(|()| change_list(ui, view, &[], icons, intents)),
    };
    if let Some(paths) = view.panel.and_then(|panel| panel.discard.as_deref()) {
        discard_dialog(ui.ctx(), view, paths, intents);
    }
    if view.panel.is_some_and(|panel| panel.stage_all) {
        stage_all_dialog(ui.ctx(), view, intents);
    }
    if let Some(dialog) = view.panel.and_then(Panel::tag) {
        crate::git_graph::tag_dialog(ui.ctx(), view, dialog, intents);
    }
    result
}

/// `Loader2 animate-spin`: one turn per second, repainting while it shows.
fn spinner(ui: &Ui) -> (Icon, f32) {
    ui.ctx().request_repaint();
    let turn = ui.input(|input| input.time).rem_euclid(SPIN_SECONDS) / SPIN_SECONDS;
    (Icon::Loader, turn as f32 * TAU)
}

/// The commit box under the header (`commit-box.tsx`): a three-row message input with the AI
/// button in its corner, the conflict notice, and the full-width commit button. Mod+Enter in the
/// input commits when the button could, and never inserts a line break.
fn commit_box(
    ui: &mut Ui,
    view: &View<'_>,
    rows: &[StatusRow],
    icons: &mut impl PaintIcon<Icon>,
    intents: &mut Vec<ShellIntent>,
) -> AppResult<()> {
    let appearance = view.appearance;
    let available = ui.available_rect_before_wrap();
    let width = (available.width() - 2.0 * PADDING_X).max(0.0);
    let gate = model::commit_gate(rows);
    let reason = (gate == model::CommitGate::BlockedByConflicts).then(|| {
        ui.painter().layout(
            view.text("git.commitBlockedByConflicts"),
            FontId::proportional(REASON_TEXT_SIZE),
            appearance.error,
            width,
        )
    });
    let input_height =
        COMMIT_ROWS as f32 * COMMIT_LINE_HEIGHT + 2.0 * (COMMIT_INPUT_PADDING_Y + BORDER_WIDTH);
    let reason_height = reason
        .as_ref()
        .map_or(0.0, |galley| galley.size().y + COMMIT_GAP);
    let rect = Rect::from_min_size(
        available.min,
        vec2(
            available.width(),
            2.0 * COMMIT_PADDING_Y
                + input_height
                + COMMIT_GAP
                + reason_height
                + BUTTON_HEIGHT
                + BORDER_WIDTH,
        ),
    );
    ui.painter().rect_filled(rect, 0.0, appearance.sidebar);
    let can_commit = !view.commit.message.trim().is_empty()
        && !view.commit.is_committing
        && gate != model::CommitGate::BlockedByConflicts;
    let request = || {
        ShellIntent::Git(Intent::RequestCommit {
            project: view.project.clone(),
            slot: view.slot.clone(),
        })
    };

    let input = Rect::from_min_size(
        pos2(rect.left() + PADDING_X, rect.top() + COMMIT_PADDING_Y),
        vec2(width, input_height),
    );
    let input_id = view.mounted_id("git-commit-message");
    let has_focus = ui.memory(|memory| memory.has_focus(input_id));
    if has_focus {
        let events = ui.input(|input| input.events.clone());
        let mut composition: Composition =
            ui.data(|data| data.get_temp(input_id)).unwrap_or_default();
        let is_composing = composition.observe(&events);
        ui.data_mut(|data| data.insert_temp(input_id, composition));
        if !is_composing
            && ui.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, Key::Enter))
            && can_commit
        {
            intents.push(request());
        }
    }
    ui.painter().rect(
        input,
        INPUT_RADIUS,
        appearance.input,
        Stroke::new(
            BORDER_WIDTH,
            if has_focus {
                appearance.focus_border
            } else {
                appearance.input_border
            },
        ),
        egui::StrokeKind::Inside,
    );
    let text_rect = Rect::from_min_max(
        pos2(
            input.left() + BORDER_WIDTH + COMMIT_INPUT_PADDING_LEFT,
            input.top() + BORDER_WIDTH + COMMIT_INPUT_PADDING_Y,
        ),
        pos2(
            input.right() - BORDER_WIDTH - COMMIT_INPUT_PADDING_RIGHT,
            input.bottom() - BORDER_WIDTH - COMMIT_INPUT_PADDING_Y,
        ),
    );
    let mut message = view.commit.message.to_owned();
    let mut field = ui.new_child(egui::UiBuilder::new().max_rect(text_rect));
    let edited = egui::ScrollArea::vertical()
        .id_salt(input_id.with("scroll"))
        .auto_shrink([false, false])
        .max_height(text_rect.height())
        .show(&mut field, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut message)
                    .id(input_id)
                    .frame(egui::Frame::NONE)
                    .margin(egui::Margin::ZERO)
                    .desired_width(f32::INFINITY)
                    .desired_rows(COMMIT_ROWS)
                    .font(FontId::proportional(TEXT_SIZE))
                    .text_color(appearance.foreground)
                    .hint_text(
                        egui::RichText::new(view.text("git.commitMessagePlaceholder"))
                            .color(appearance.muted),
                    ),
            )
        })
        .inner;
    if edited.changed() {
        intents.push(ShellIntent::Git(Intent::SetCommitMessage {
            project: view.project.clone(),
            slot: view.slot.clone(),
            message,
        }));
    }

    let can_generate = !model::build_sections(rows, 0, 0, view.collapsed).shows_no_changes;
    let generate_label = view.text(if view.commit.is_generating {
        "git.generatingCommitMessage"
    } else if can_generate {
        "git.generateCommitMessage"
    } else {
        "git.noChangesForCommitMessage"
    });
    let glyph = if view.commit.is_generating {
        spinner(ui)
    } else {
        (Icon::Sparkles, 0.0)
    };
    if view.icon_button(
        ui,
        (
            Rect::from_min_size(
                pos2(
                    input.right() - AI_BUTTON_INSET - HEADER_BUTTON_SIZE,
                    input.top() + AI_BUTTON_INSET,
                ),
                vec2(HEADER_BUTTON_SIZE, HEADER_BUTTON_SIZE),
            ),
            view.mounted_id("git-generate-commit-message"),
        ),
        glyph,
        (&generate_label, egui::RectAlign::LEFT),
        view.commit.is_generating || can_generate,
        icons,
    )? {
        intents.push(ShellIntent::Git(Intent::GenerateCommitMessage {
            project: view.project.clone(),
            slot: view.slot.clone(),
        }));
    }

    let mut top = input.bottom() + COMMIT_GAP;
    if let Some(galley) = reason {
        let height = galley.size().y;
        ui.painter()
            .galley(pos2(input.left(), top), galley, appearance.error);
        top += height + COMMIT_GAP;
    }
    let button = Rect::from_min_size(pos2(input.left(), top), vec2(width, BUTTON_HEIGHT));
    let label = view.text(if view.commit.is_committing {
        "git.committing"
    } else {
        "git.commit"
    });
    let response = ui.interact(
        button,
        view.mounted_id("git-commit"),
        if can_commit {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, can_commit, &label));
    let opacity = if can_commit { 1.0 } else { DISABLED_OPACITY };
    let background = if can_commit && response.hovered() {
        appearance.primary.gamma_multiply(PRIMARY_HOVER_OPACITY)
    } else {
        appearance.primary
    };
    ui.painter()
        .rect_filled(button, BUTTON_RADIUS, background.gamma_multiply(opacity));
    ui.painter().text(
        button.center(),
        egui::Align2::CENTER_CENTER,
        label,
        FontId::new(BUTTON_TEXT_SIZE, taide_native_ui::font_families::medium(ui)),
        appearance.primary_foreground.gamma_multiply(opacity),
    );
    if response.clicked() {
        intents.push(request());
    }

    let border = rect.bottom() - BORDER_WIDTH / 2.0;
    ui.painter().line_segment(
        [pos2(rect.left(), border), pos2(rect.right(), border)],
        Stroke::new(BORDER_WIDTH, appearance.border),
    );
    ui.advance_cursor_after_rect(rect);
    Ok(())
}

/// The stage-all confirmation (`git-panel.tsx` `AlertDialog`) shown when committing with nothing
/// staged.
fn stage_all_dialog(context: &egui::Context, view: &View<'_>, intents: &mut Vec<ShellIntent>) {
    if let Some(confirmed) = confirmation(
        context,
        Id::new((STAGE_ALL_ID, view.slot)),
        view,
        (
            view.text("git.stageAllTitle"),
            view.text("git.stageAllDescription"),
        ),
        (view.text("git.commit"), false),
    ) {
        intents.push(ShellIntent::Git(Intent::ResolveStageAll {
            project: view.project.clone(),
            slot: view.slot.clone(),
            confirmed,
        }));
    }
}

fn not_a_repository(ui: &mut Ui, view: &View<'_>, intents: &mut Vec<ShellIntent>) {
    let appearance = view.appearance;
    let rect = ui.available_rect_before_wrap();
    ui.painter().rect_filled(rect, 0.0, appearance.panel);
    let mut job = egui::text::LayoutJob::simple(
        view.text("git.notARepository"),
        FontId::proportional(NOTICE_TEXT_SIZE),
        appearance.muted,
        (rect.width() - 2.0 * NOTICE_PADDING).max(0.0),
    );
    job.halign = egui::Align::Center;
    let message = ui.painter().layout_job(job);
    let label = view.text("git.initRepository");
    let font = FontId::new(BUTTON_TEXT_SIZE, taide_native_ui::font_families::medium(ui));
    let width = ui
        .painter()
        .layout_no_wrap(label.clone(), font.clone(), appearance.muted)
        .size()
        .x;
    let message_height = message.size().y;
    let top = rect.center().y - (message_height + NOTICE_GAP + BUTTON_HEIGHT) / 2.0;
    ui.painter()
        .galley(pos2(rect.center().x, top), message, appearance.muted);
    let button = Rect::from_center_size(
        pos2(
            rect.center().x,
            top + message_height + NOTICE_GAP + BUTTON_HEIGHT / 2.0,
        ),
        vec2(
            width + 2.0 * (BUTTON_PADDING_X + BORDER_WIDTH),
            BUTTON_HEIGHT,
        ),
    );
    let is_enabled = !view.is_initializing;
    let response = ui.interact(
        button,
        ui.id().with(("git-init", view.slot)),
        if is_enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, is_enabled, &label));
    let is_hovered = is_enabled && response.hovered();
    let opacity = if is_enabled { 1.0 } else { DISABLED_OPACITY };
    let (background, foreground) = if is_hovered {
        (appearance.button_hover, appearance.button_hover_foreground)
    } else {
        (appearance.button, appearance.muted)
    };
    ui.painter().rect(
        button,
        BUTTON_RADIUS,
        background.gamma_multiply(opacity),
        Stroke::new(BORDER_WIDTH, appearance.border.gamma_multiply(opacity)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        button.center(),
        egui::Align2::CENTER_CENTER,
        label,
        font,
        foreground.gamma_multiply(opacity),
    );
    if response.clicked() {
        intents.push(ShellIntent::Git(Intent::Init(view.project.clone())));
    }
    ui.advance_cursor_after_rect(rect);
}

/// The panel's top bar (`git-panel.tsx` header): branch switcher, ahead/behind counts, then the
/// stash and sync buttons pushed right and the first remote's name. Only the switcher shrinks.
fn header(
    ui: &mut Ui,
    view: &View<'_>,
    status: Option<&GitStatus>,
    icons: &mut impl PaintIcon<Icon>,
    intents: &mut Vec<ShellIntent>,
) -> AppResult<()> {
    let appearance = view.appearance;
    let available = ui.available_rect_before_wrap();
    let rect = Rect::from_min_size(available.min, vec2(available.width(), HEADER_HEIGHT));
    ui.painter().rect_filled(rect, 0.0, appearance.sidebar);
    let center = rect.center().y;
    let has_remote = status.is_some_and(|status| status.has_remote);
    let remote = view.remotes.first().map(|remote| {
        ui.painter().layout_no_wrap(
            remote.name.clone(),
            FontId::proportional(REMOTE_TEXT_SIZE),
            appearance.muted,
        )
    });
    let counts: Vec<_> = status
        .filter(|_| has_remote)
        .map(|status| {
            [
                (Icon::ArrowUp, status.ahead),
                (Icon::ArrowDown, status.behind),
            ]
        })
        .into_iter()
        .flatten()
        .filter(|(_, count)| *count > 0)
        .map(|(icon, count)| {
            let text = ui.painter().layout_no_wrap(
                count.to_string(),
                FontId::proportional(TEXT_SIZE),
                appearance.foreground,
            );
            (icon, text)
        })
        .collect();
    let mut right = rect.right() - PADDING_X;
    let remote_left = remote.as_ref().map(|galley| {
        right -= galley.size().x;
        let left = right;
        right -= REMOTE_MARGIN + GAP;
        left
    });
    let sync = has_remote.then(|| {
        right -= HEADER_BUTTON_SIZE;
        let left = right;
        right -= GAP;
        left
    });
    right -= HEADER_BUTTON_SIZE;
    let stash = right;
    let button = |left: f32| {
        Rect::from_min_size(
            pos2(left, center - HEADER_BUTTON_SIZE / 2.0),
            vec2(HEADER_BUTTON_SIZE, HEADER_BUTTON_SIZE),
        )
    };
    let counts_width: f32 = counts
        .iter()
        .map(|(_, text)| COUNT_ICON_SIZE + COUNT_GAP + text.size().x + GAP)
        .sum();
    let mut left = rect.left() + PADDING_X;
    let branch = status.and_then(|status| status.branch.as_deref());
    let switcher_width = crate::git_branch::trigger_width(ui, view, branch)
        .min((stash - GAP - counts_width - left).max(0.0));
    crate::git_branch::show(
        ui,
        Rect::from_min_size(
            pos2(left, center - crate::git_branch::TRIGGER_HEIGHT / 2.0),
            vec2(switcher_width, crate::git_branch::TRIGGER_HEIGHT),
        ),
        view,
        branch,
        icons,
        intents,
    )?;
    left += switcher_width + GAP;
    for (icon, text) in counts {
        icons.paint(
            ui,
            Rect::from_min_size(
                pos2(left, center - COUNT_ICON_SIZE / 2.0),
                vec2(COUNT_ICON_SIZE, COUNT_ICON_SIZE),
            ),
            icon,
            appearance.foreground,
            0.0,
        )?;
        left += COUNT_ICON_SIZE + COUNT_GAP;
        let width = text.size().x;
        ui.painter().galley(
            pos2(left, center - text.size().y / 2.0),
            text,
            appearance.foreground,
        );
        left += width + GAP;
    }
    let can_stash = status.is_some_and(|status| !status.rows.is_empty());
    if view.icon_button(
        ui,
        (button(stash), view.mounted_id("git-stash-push")),
        (Icon::Archive, 0.0),
        (&view.text("git.stashPush"), egui::RectAlign::BOTTOM),
        can_stash,
        icons,
    )? {
        intents.push(ShellIntent::Git(Intent::StashPush(view.project.clone())));
    }
    if let Some(sync) = sync {
        let glyph = if view.is_syncing {
            spinner(ui)
        } else {
            (Icon::RefreshCw, 0.0)
        };
        if view.icon_button(
            ui,
            (button(sync), view.mounted_id("git-sync")),
            glyph,
            (&view.text("git.sync"), egui::RectAlign::BOTTOM),
            !view.is_syncing,
            icons,
        )? {
            intents.push(ShellIntent::Git(Intent::Sync(view.project.clone())));
        }
    }
    if let (Some(galley), Some(left)) = (remote, remote_left) {
        ui.painter().galley(
            pos2(left, center - galley.size().y / 2.0),
            galley,
            appearance.muted,
        );
    }
    let border = rect.bottom() - BORDER_WIDTH / 2.0;
    ui.painter().line_segment(
        [pos2(rect.left(), border), pos2(rect.right(), border)],
        Stroke::new(BORDER_WIDTH, appearance.border),
    );
    ui.spacing_mut().item_spacing.y = 0.0;
    ui.advance_cursor_after_rect(rect);
    Ok(())
}

fn change_list(
    ui: &mut Ui,
    view: &View<'_>,
    rows: &[taide_model::git::StatusRow],
    icons: &mut impl PaintIcon<Icon>,
    intents: &mut Vec<ShellIntent>,
) -> AppResult<()> {
    let appearance = view.appearance;
    let area = ui.available_rect_before_wrap();
    let sections = model::build_sections(
        rows,
        view.stashes.len(),
        view.graph.log.len(),
        view.collapsed,
    );
    let graph = sections.state(Section::Graph);
    let separator_id = view.mounted_id(GRAPH_SEPARATOR_ID);
    let graph_layout = graph
        .is_visible
        .then(|| graph_layout(ui, view, area, separator_id, graph.is_collapsed));
    let rect = graph_layout.map_or(area, |layout| {
        Rect::from_min_max(area.min, pos2(area.right(), layout.separator.top()))
    });
    ui.painter().rect_filled(rect, 0.0, appearance.sidebar);
    let items = model::list_rows(&sections);
    let keys: Vec<_> = items
        .iter()
        .map(|item| model::row_key(&sections, *item))
        .collect();
    let id = Id::new((LIST_ID, view.slot));
    let ids = items
        .iter()
        .zip(&keys)
        .map(|(item, key)| match item {
            ListRow::Header(section) => id.with(("header", *section)),
            ListRow::Row(..) => id.with(("row", key)),
        })
        .collect();
    let stash = sections.state(Section::Stashes);
    let list = List {
        view,
        sections: &sections,
        items,
        keys,
        ids,
        id,
        stash_header: stash
            .is_visible
            .then(|| id.with(("header", Section::Stashes))),
        graph_header: graph
            .is_visible
            .then(|| id.with(("header", Section::Graph))),
    };
    let scroll_to = list.rove(ui, intents);
    let headers = model::header_indexes(&list.items);
    let mut opened_menu = None;
    ui.spacing_mut().item_spacing.y = 0.0;
    let mut changes = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    let count = list.items.len();
    let list_height = count as f32 * ROW_HEIGHT;
    let notice_height = if sections.shows_no_changes {
        NO_CHANGES_HEIGHT
    } else {
        0.0
    };
    let stash_rows = if stash.is_collapsed {
        &[][..]
    } else {
        view.stashes
    };
    let stash_height = if stash.is_visible {
        ROW_HEIGHT + stash_rows.len() as f32 * STASH_ROW_HEIGHT
    } else {
        0.0
    };
    egui::ScrollArea::vertical()
        .id_salt(("git-changes", view.slot))
        .auto_shrink([false, false])
        .show_viewport(&mut changes, |ui, viewport| -> AppResult<()> {
            ui.set_height(list_height + notice_height + stash_height);
            let x = ui.max_rect().x_range();
            let top = ui.max_rect().top();
            let natural = |index: usize| top + index as f32 * ROW_HEIGHT;
            let at = |y: f32| Rect::from_x_y_ranges(x, y..=y + ROW_HEIGHT);
            let first = ((viewport.min.y / ROW_HEIGHT).floor().max(0.0) as usize).min(count);
            let end = ((viewport.max.y / ROW_HEIGHT).ceil().max(0.0) as usize + 1).min(count);
            let range = first..end.max(first);
            let stash_top = top + list_height + notice_height;
            if let Some(target) = scroll_to {
                let target = if target < count {
                    Rect::from_x_y_ranges(
                        x,
                        natural(target) - STICKY_PADDING..=natural(target) + ROW_HEIGHT,
                    )
                } else {
                    at(stash_top)
                };
                ui.scroll_to_rect(target, None);
            }
            let sticky =
                model::sticky_header_index(&headers, range.start).filter(|_| !range.is_empty());
            let clip_top = ui.clip_rect().top();
            let is_row = |index: &usize| matches!(list.items[*index], ListRow::Row(..));
            for index in range.clone().filter(is_row) {
                list.item(
                    ui,
                    index,
                    at(natural(index)),
                    icons,
                    intents,
                    &mut opened_menu,
                )?;
            }
            if let Some(index) = sticky {
                let y = natural(index)
                    .max(clip_top)
                    .min(top + list_height - ROW_HEIGHT);
                list.item(ui, index, at(y), icons, intents, &mut opened_menu)?;
            }
            for index in range.filter(|index| !is_row(index) && Some(*index) != sticky) {
                list.item(
                    ui,
                    index,
                    at(natural(index)),
                    icons,
                    intents,
                    &mut opened_menu,
                )?;
            }
            if sections.shows_no_changes {
                ui.painter().text(
                    pos2(x.min + PADDING_X, top + EMPTY_PADDING_Y),
                    egui::Align2::LEFT_TOP,
                    view.text("git.noChanges"),
                    FontId::proportional(TEXT_SIZE),
                    appearance.muted,
                );
            }
            if let Some(stash_id) = list.stash_header {
                list.take_pending_focus(ui, stash_id);
                list.header(
                    ui,
                    at(stash_top),
                    stash_id,
                    Section::Stashes,
                    icons,
                    intents,
                )?;
                for (row, entry) in stash_rows.iter().enumerate() {
                    let row_top = stash_top + ROW_HEIGHT + row as f32 * STASH_ROW_HEIGHT;
                    list.stash_row(
                        ui,
                        Rect::from_x_y_ranges(
                            x.min + ROW_INDENT..=x.max,
                            row_top..=row_top + STASH_ROW_HEIGHT,
                        ),
                        entry,
                        icons,
                        intents,
                    )?;
                }
            }
            Ok(())
        })
        .inner?;
    list.menu(ui, opened_menu, intents);
    if let (Some(layout), Some(header)) = (graph_layout, list.graph_header) {
        list.take_pending_focus(ui, header);
        list.header(ui, layout.header, header, Section::Graph, icons, intents)?;
        if let Some(body) = layout.body {
            crate::git_graph::show_pane(ui, body, view, icons, intents)?;
        }
        graph_separator(ui, view, area, layout, separator_id, intents);
    }
    ui.advance_cursor_after_rect(area);
    Ok(())
}

const GRAPH_COLLAPSED_HEIGHT: f32 = ROW_HEIGHT;

/// The graph pane's place below the change pane (`git-panel.tsx` vertical `Group`): its stored or
/// dragged height, never under the minimum unless collapsed to its header, never over the panel.
#[derive(Clone, Copy, Debug)]
struct GraphLayout {
    separator: Rect,
    header: Rect,
    body: Option<Rect>,
    height: f32,
}

/// `react-resizable-panels` collapsible clamping: below the minimum a size snaps to the collapsed
/// header once it is under halfway between the two, otherwise to the minimum.
fn snap_graph_height(height: f32, max: f32) -> f32 {
    let snapped = if height >= GRAPH_MIN_HEIGHT {
        height
    } else if height < (GRAPH_COLLAPSED_HEIGHT + GRAPH_MIN_HEIGHT) / 2.0 {
        GRAPH_COLLAPSED_HEIGHT
    } else {
        GRAPH_MIN_HEIGHT
    };
    snapped.min(max)
}

fn graph_layout(
    ui: &Ui,
    view: &View<'_>,
    area: Rect,
    separator_id: Id,
    is_collapsed: bool,
) -> GraphLayout {
    let thickness = view.graph.resizer_thickness;
    let max = (area.height() - thickness).max(0.0);
    let dragged = ui.data(|data| data.get_temp::<f32>(separator_id));
    let height = match dragged {
        Some(height) => snap_graph_height(height, max),
        None if is_collapsed => GRAPH_COLLAPSED_HEIGHT.min(max),
        None => view
            .panel
            .and_then(|panel| panel.graph_height)
            .unwrap_or(view.graph.size)
            .max(GRAPH_MIN_HEIGHT)
            .min(max),
    };
    let top = area.bottom() - height;
    let header = Rect::from_x_y_ranges(area.x_range(), top..=top + ROW_HEIGHT);
    let body = (!is_collapsed && height > ROW_HEIGHT)
        .then(|| Rect::from_x_y_ranges(area.x_range(), header.bottom()..=area.bottom()));
    GraphLayout {
        separator: Rect::from_x_y_ranges(area.x_range(), top - thickness..=top),
        header,
        body,
        height,
    }
}

/// The separator between the two panes (`PaneSeparator`): app border, focus border while hovered
/// or dragged, an 8px drag target. Releasing reports the new height, or the collapse.
fn graph_separator(
    ui: &Ui,
    view: &View<'_>,
    area: Rect,
    layout: GraphLayout,
    id: Id,
    intents: &mut Vec<ShellIntent>,
) {
    let response = ui
        .interact(
            Rect::from_center_size(
                layout.separator.center(),
                vec2(
                    area.width(),
                    taide_native_ui::split::RESIZE_HIT_SIZE.max(layout.separator.height()),
                ),
            ),
            id,
            Sense::drag(),
        )
        .on_hover_cursor(egui::CursorIcon::ResizeVertical);
    response.widget_info(|| egui::WidgetInfo::new(egui::WidgetType::ResizeHandle));
    let max = (area.height() - layout.separator.height()).max(0.0);
    let dragged = ui.data(|data| data.get_temp::<f32>(id));
    let color = if response.hovered() || response.dragged() {
        view.appearance.focus_border
    } else {
        view.appearance.border
    };
    ui.painter().rect_filled(layout.separator, 0.0, color);
    if response.dragged() {
        let height = dragged.unwrap_or(layout.height) - response.drag_delta().y;
        ui.data_mut(|data| data.insert_temp(id, height));
        ui.ctx().request_repaint();
    }
    if response.drag_stopped() {
        ui.data_mut(|data| data.remove::<f32>(id));
        let height = snap_graph_height(
            dragged.unwrap_or(layout.height) - response.drag_delta().y,
            max,
        );
        intents.push(ShellIntent::Git(Intent::ResizeGraph {
            project: view.project.clone(),
            slot: view.slot.clone(),
            height: (height > GRAPH_COLLAPSED_HEIGHT).then_some(height),
        }));
    }
}

/// The change pane's flattened list for one frame: items, their row keys and stable widget ids
/// (`git-panel.tsx` single virtualized list).
struct List<'a, 'v> {
    view: &'a View<'v>,
    sections: &'a model::Sections<'a>,
    items: Vec<ListRow>,
    keys: Vec<Option<RowKey>>,
    ids: Vec<Id>,
    id: Id,
    stash_header: Option<Id>,
    graph_header: Option<Id>,
}

impl List<'_, '_> {
    fn toggle(&self, section: Section) -> ShellIntent {
        ShellIntent::Git(Intent::ToggleSection {
            project: self.view.project.clone(),
            slot: self.view.slot.clone(),
            section,
        })
    }

    fn select(&self, selected: Vec<RowKey>, anchor: RowKey) -> ShellIntent {
        ShellIntent::Git(Intent::Select {
            project: self.view.project.clone(),
            slot: self.view.slot.clone(),
            selected,
            anchor: Some(anchor),
        })
    }

    fn pending_focus(&self) -> Id {
        self.id.with(PENDING_FOCUS)
    }

    /// The roving focus stops in order: the list items, then the stash header when it is drawn
    /// (`git-panel.tsx` `stashesRovingIndex`).
    fn stops(&self) -> impl Iterator<Item = Id> + '_ {
        self.ids.iter().copied().chain(self.stash_header)
    }

    fn take_pending_focus(&self, ui: &Ui, id: Id) {
        if ui.data(|data| data.get_temp::<Id>(self.pending_focus())) == Some(id) {
            ui.data_mut(|data| data.remove::<Id>(self.pending_focus()));
            ui.memory_mut(|memory| memory.request_focus_with_filter(id, ARROWS));
        }
    }

    /// Arrow keys on a focused header or row (`handleSectionsKeyDown`, `handleGraphPaneKeyDown`,
    /// `GitSectionHeader`): ↑/↓ move along the stops, the graph pane's header after the scrolled
    /// ones, and return the scrolled stop to bring into view; ←/→ collapse or expand a header.
    fn rove(&self, ui: &mut Ui, intents: &mut Vec<ShellIntent>) -> Option<usize> {
        let focused = ui.memory(|memory| memory.focused())?;
        let origin = if Some(focused) == self.graph_header {
            model::FocusTarget::Graph
        } else {
            model::FocusTarget::Item(self.stops().position(|id| id == focused)?)
        };
        ui.memory_mut(|memory| memory.set_focus_lock_filter(focused, ARROWS));
        let pressed = |key| ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key));
        let direction = if pressed(Key::ArrowDown) {
            model::Direction::Down
        } else if pressed(Key::ArrowUp) {
            model::Direction::Up
        } else {
            let header = match origin {
                model::FocusTarget::Graph => Some(Section::Graph),
                model::FocusTarget::Item(index) => match self.items.get(index) {
                    Some(ListRow::Header(section)) => Some(*section),
                    Some(ListRow::Row(..)) => None,
                    None => Some(Section::Stashes),
                },
            };
            if let Some(section) = header {
                let toggle_key = if self.sections.state(section).is_collapsed {
                    Key::ArrowRight
                } else {
                    Key::ArrowLeft
                };
                if pressed(toggle_key) {
                    intents.push(self.toggle(section));
                }
            }
            return None;
        };
        let target = model::focus_target(
            direction,
            Some(origin),
            self.stops().count(),
            self.graph_header.is_some(),
        )?;
        let (target_id, scroll_to) = match target {
            model::FocusTarget::Item(target) => (self.stops().nth(target)?, Some(target)),
            model::FocusTarget::Graph => (self.graph_header?, None),
        };
        ui.data_mut(|data| data.insert_temp(self.pending_focus(), target_id));
        scroll_to
    }

    fn item(
        &self,
        ui: &mut Ui,
        index: usize,
        rect: Rect,
        icons: &mut impl PaintIcon<Icon>,
        intents: &mut Vec<ShellIntent>,
        opened_menu: &mut Option<RowKey>,
    ) -> AppResult<()> {
        let id = self.ids[index];
        self.take_pending_focus(ui, id);
        match self.items[index] {
            ListRow::Row(..) => self.change_row(ui, rect, index, icons, intents, opened_menu),
            ListRow::Header(section) => self.header(ui, rect, id, section, icons, intents),
        }
    }

    /// The one row menu the whole list shares, kept by path so a status refresh that drops the
    /// row closes it (`git-panel.tsx` `contextMenuTarget`).
    fn menu(&self, ui: &Ui, opened: Option<RowKey>, intents: &mut Vec<ShellIntent>) {
        let is_opening = opened.is_some();
        let Some(target) = opened.or_else(|| self.view.panel.and_then(|panel| panel.menu.clone()))
        else {
            return;
        };
        let Some(change) = self
            .sections
            .rows(target.section)
            .iter()
            .find(|change| change.row.path == target.path)
        else {
            return;
        };
        egui::Popup::new(
            self.id.with(("menu", &target)),
            ui.ctx().clone(),
            egui::PopupAnchor::PointerFixed,
            ui.layer_id(),
        )
        .kind(egui::PopupKind::Menu)
        .layout(egui::Layout::top_down_justified(egui::Align::Min))
        .style(egui::menu::menu_style)
        .open_memory(is_opening.then_some(egui::SetOpenCommand::Bool(true)))
        .show(|ui| {
            for entry in model::row_menu(target.section) {
                let Some(action) = *entry else {
                    ui.separator();
                    continue;
                };
                let text = egui::RichText::new(self.view.text(action.label()));
                let text = if action == RowAction::Discard {
                    text.color(ui.visuals().error_fg_color)
                } else {
                    text
                };
                if ui.button(text).clicked() {
                    intents.push(self.view.row_intent(target.section, change.row, action));
                    ui.close();
                }
            }
        });
    }
}

/// A confirmation dialog (`AlertDialog`): focus starts and stays on Cancel until the user moves
/// it, Escape cancels and an outside click is ignored. Answers once the user decides.
fn confirmation(
    context: &egui::Context,
    id: Id,
    view: &View<'_>,
    (title, description): (String, String),
    (confirm, is_destructive): (String, bool),
) -> Option<bool> {
    let response = egui::Modal::new(id).show(context, |ui| {
        ui.set_max_width(DIALOG_MAX_WIDTH);
        ui.heading(title);
        ui.label(description);
        ui.horizontal(|ui| {
            let cancel = ui.button(view.text("common.cancel"));
            let confirm = egui::RichText::new(confirm);
            let confirm = ui.button(if is_destructive {
                confirm.color(ui.visuals().error_fg_color)
            } else {
                confirm
            });
            if !cancel.has_focus() && !confirm.has_focus() {
                cancel.request_focus();
            }
            if cancel.clicked() {
                Some(false)
            } else if confirm.clicked() {
                Some(true)
            } else {
                None
            }
        })
        .inner
    });
    let is_escaped = response.is_top_modal
        && !response.any_popup_open
        && context.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::Escape));
    response.inner.or(is_escaped.then_some(false))
}

/// The discard confirmation (`git-panel.tsx` `AlertDialog`).
fn discard_dialog(
    context: &egui::Context,
    view: &View<'_>,
    paths: &[String],
    intents: &mut Vec<ShellIntent>,
) {
    if let Some(confirmed) = confirmation(
        context,
        Id::new((DISCARD_ID, view.slot)),
        view,
        (
            view.text("git.discardTitle"),
            model::discard_description(view.locale, paths),
        ),
        (view.text("git.discardConfirm"), true),
    ) {
        intents.push(ShellIntent::Git(Intent::ResolveDiscard {
            project: view.project.clone(),
            slot: view.slot.clone(),
            confirmed,
        }));
    }
}

fn truncated(ui: &Ui, text: String, font: FontId, color: Color32, width: f32) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(text, font, color, width.max(0.0));
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some(ELLIPSIS);
    ui.painter().layout_job(job)
}

fn header_action(section: Section) -> Option<(Icon, &'static str)> {
    match section {
        Section::Staged => Some((Icon::Minus, "git.unstageAll")),
        Section::Changes => Some((Icon::Plus, "git.stageAll")),
        Section::Merge | Section::Stashes | Section::Graph => None,
    }
}

impl List<'_, '_> {
    fn header(
        &self,
        ui: &mut Ui,
        rect: Rect,
        id: Id,
        section: Section,
        icons: &mut impl PaintIcon<Icon>,
        intents: &mut Vec<ShellIntent>,
    ) -> AppResult<()> {
        let view = self.view;
        let response = ui.interact(rect, id, Sense::click());
        let action_id = id.with("group-action");
        let is_focus_within =
            response.has_focus() || ui.memory(|memory| memory.has_focus(action_id));
        let action = header_action(section)
            .filter(|_| is_focus_within || ui.rect_contains_pointer(rect))
            .map(|(icon, key)| {
                let button = Rect::from_min_size(
                    pos2(
                        rect.right() - PADDING_X - ACTION_SIZE,
                        rect.center().y - ACTION_SIZE / 2.0,
                    ),
                    vec2(ACTION_SIZE, ACTION_SIZE),
                );
                (icon, key, button)
            });
        let trailing = action.map_or(0.0, |_| ACTION_SIZE + GAP);
        let appearance = view.appearance;
        let state = self.sections.state(section);
        let title = view.text(model::section_title(section));
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &title));
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_expanded(!state.is_collapsed);
        });
        let background = if is_focus_within {
            appearance.focused
        } else if ui.rect_contains_pointer(rect) {
            appearance.hover
        } else {
            appearance.sidebar
        };
        ui.painter().rect_filled(rect, 0.0, background);
        ui.painter().line_segment(
            [rect.left_top(), rect.right_top()],
            Stroke::new(BORDER_WIDTH, appearance.border),
        );
        let chevron = Rect::from_center_size(
            pos2(
                rect.left() + PADDING_X + CHEVRON_SIZE / 2.0,
                rect.center().y,
            ),
            vec2(CHEVRON_SIZE, CHEVRON_SIZE),
        );
        icons.paint(
            ui,
            chevron,
            Icon::ChevronRight,
            appearance.header,
            if state.is_collapsed {
                0.0
            } else {
                EXPANDED_ANGLE
            },
        )?;
        let count = ui.painter().layout_no_wrap(
            state.count.to_string(),
            FontId::proportional(BADGE_TEXT_SIZE),
            appearance.header,
        );
        let badge_width = (count.size().x + 2.0 * BADGE_PADDING_X).max(BADGE_HEIGHT);
        let left = chevron.right() + GAP;
        let title = truncated(
            ui,
            title.to_uppercase(),
            FontId::new(
                HEADER_TEXT_SIZE,
                taide_native_ui::font_families::semibold(ui),
            ),
            appearance.header,
            rect.right() - PADDING_X - trailing - badge_width - GAP - left,
        );
        let title_width = title.size().x;
        ui.painter().galley(
            pos2(left, rect.center().y - title.size().y / 2.0),
            title,
            appearance.header,
        );
        let badge = Rect::from_min_size(
            pos2(
                left + title_width + GAP,
                rect.center().y - BADGE_HEIGHT / 2.0,
            ),
            vec2(badge_width, BADGE_HEIGHT),
        );
        ui.painter()
            .rect_filled(badge, BADGE_HEIGHT / 2.0, appearance.badge);
        ui.painter().galley(
            badge.center() - count.size() / 2.0,
            count,
            appearance.header,
        );
        if let Some((icon, key, button)) = action {
            let label = view.text(key);
            let pressed = ui.interact(button, action_id, Sense::click());
            pressed
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
            view.tooltips.show(
                &pressed,
                &label,
                egui::RectAlign::BOTTOM,
                view.tooltip_appearance,
            );
            if pressed.hovered() {
                ui.painter()
                    .rect_filled(button, ACTION_RADIUS, view.appearance.selected);
            }
            icons.paint(
                ui,
                Rect::from_center_size(button.center(), vec2(ACTION_ICON_SIZE, ACTION_ICON_SIZE)),
                icon,
                view.appearance.header,
                0.0,
            )?;
            if pressed.has_focus() {
                ui.memory_mut(|memory| memory.set_focus_lock_filter(action_id, ARROWS));
            }
            if pressed.clicked() {
                let project = view.project.clone();
                let paths = self
                    .sections
                    .rows(section)
                    .iter()
                    .map(|change| change.row.path.clone())
                    .collect();
                intents.push(ShellIntent::Git(match section {
                    Section::Staged => Intent::Unstage { project, paths },
                    _ => Intent::Stage { project, paths },
                }));
            }
        }
        if response.clicked() {
            if response.clicked_by(egui::PointerButton::Primary) {
                ui.memory_mut(|memory| memory.request_focus_with_filter(id, ARROWS));
            }
            intents.push(self.toggle(section));
        }
        Ok(())
    }
}

fn row_actions(section: Section) -> &'static [(Icon, RowAction)] {
    match section {
        Section::Merge => &[(Icon::File, RowAction::OpenFile)],
        Section::Staged => &[
            (Icon::Minus, RowAction::Unstage),
            (Icon::File, RowAction::OpenFile),
        ],
        Section::Changes => &[
            (Icon::Plus, RowAction::Stage),
            (Icon::Undo2, RowAction::Discard),
            (Icon::File, RowAction::OpenFile),
        ],
        Section::Stashes | Section::Graph => &[],
    }
}

impl List<'_, '_> {
    /// One change row (`status-row-item.tsx`): the indented row reacts to the menu, its name part
    /// is the focusable button, and hover or focus inside swaps the status letter for the actions.
    fn change_row(
        &self,
        ui: &mut Ui,
        rect: Rect,
        index: usize,
        icons: &mut impl PaintIcon<Icon>,
        intents: &mut Vec<ShellIntent>,
        opened_menu: &mut Option<RowKey>,
    ) -> AppResult<()> {
        let (ListRow::Row(section, row), Some(key)) = (self.items[index], &self.keys[index]) else {
            return Ok(());
        };
        let change: ChangeRow<'_> = self.sections.rows(section)[row];
        let id = self.ids[index];
        let view = self.view;
        let appearance = view.appearance;
        let item = ui.interact(rect, id.with("item"), Sense::CLICK);
        let content = Rect::from_min_max(pos2(rect.left() + ROW_INDENT, rect.top()), rect.max);
        let actions = row_actions(section);
        let action_ids: Vec<_> = actions
            .iter()
            .map(|(_, action)| id.with(action.label()))
            .collect();
        let is_focus_within = ui.memory(|memory| {
            memory.has_focus(id) || action_ids.iter().any(|action| memory.has_focus(*action))
        });
        let is_hovered = ui.rect_contains_pointer(content);
        let is_selected = view.selected().contains(key);
        let background = if is_focus_within {
            Some(appearance.focused)
        } else if is_hovered {
            Some(appearance.hover)
        } else {
            is_selected.then_some(appearance.selected)
        };
        if let Some(background) = background {
            ui.painter().rect_filled(content, 0.0, background);
        }
        let right = content.right() - PADDING_X;
        let shows_actions = is_hovered || is_focus_within;
        let trailing = if shows_actions {
            actions.len() as f32 * (ACTION_SIZE + ACTION_GAP) - ACTION_GAP
        } else {
            LETTER_WIDTH
        };
        let (name, directory) = model::split_path(&change.row.path);
        let label = directory.map_or_else(
            || name.to_owned(),
            |directory| format!("{name} {directory}"),
        );
        let button = ui.interact(
            Rect::from_min_max(content.min, pos2(right - trailing - GAP, content.bottom())),
            id,
            Sense::click(),
        );
        button.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, true, is_selected, &label)
        });
        view.tooltips.show(
            &button,
            &model::row_tooltip(change.row),
            egui::RectAlign::BOTTOM,
            view.tooltip_appearance,
        );
        let mut is_menu_requested = item.secondary_clicked() || button.secondary_clicked();
        if shows_actions {
            let mut left = right - trailing;
            for ((icon, action), action_id) in actions.iter().zip(&action_ids) {
                let bounds = Rect::from_min_size(
                    pos2(left, rect.center().y - ACTION_SIZE / 2.0),
                    vec2(ACTION_SIZE, ACTION_SIZE),
                );
                let text = view.text(action.label());
                let pressed = ui.interact(bounds, *action_id, Sense::click());
                pressed.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &text)
                });
                view.tooltips.show(
                    &pressed,
                    &text,
                    egui::RectAlign::BOTTOM,
                    view.tooltip_appearance,
                );
                let color = if pressed.hovered() {
                    ui.painter()
                        .rect_filled(bounds, ACTION_RADIUS, appearance.button_hover);
                    appearance.foreground
                } else {
                    appearance.muted
                };
                icons.paint(
                    ui,
                    Rect::from_center_size(
                        bounds.center(),
                        vec2(ACTION_ICON_SIZE, ACTION_ICON_SIZE),
                    ),
                    *icon,
                    color,
                    0.0,
                )?;
                if pressed.has_focus() {
                    ui.memory_mut(|memory| memory.set_focus_lock_filter(*action_id, ARROWS));
                }
                if pressed.clicked() {
                    intents.push(view.row_intent(section, change.row, *action));
                }
                is_menu_requested |= pressed.secondary_clicked();
                left += ACTION_SIZE + ACTION_GAP;
            }
        } else {
            ui.painter().text(
                pos2(right - LETTER_WIDTH / 2.0, rect.center().y),
                egui::Align2::CENTER_CENTER,
                model::status_letter(change.kind),
                FontId::new(TEXT_SIZE, taide_native_ui::font_families::semibold(ui)),
                appearance.status(change.kind),
            );
        }
        paint_path(
            ui,
            rect,
            content.left() + PADDING_X,
            right - trailing - GAP,
            (name, directory),
            appearance,
        );
        if is_menu_requested {
            intents.push(self.select(vec![key.clone()], key.clone()));
            intents.push(ShellIntent::Git(Intent::RowMenu {
                project: view.project.clone(),
                slot: view.slot.clone(),
                target: key.clone(),
            }));
            *opened_menu = Some(key.clone());
        }
        if button.clicked() {
            let is_pointer = button.clicked_by(egui::PointerButton::Primary);
            let modifiers = ui.input(|input| input.modifiers);
            let shift = is_pointer && modifiers.shift;
            let additive = is_pointer && (modifiers.command || modifiers.ctrl || modifiers.mac_cmd);
            let ordered: Vec<RowKey> = self.keys.iter().flatten().cloned().collect();
            let selection = model::resolve_selection(
                &ordered,
                view.selected(),
                view.panel.and_then(|panel| panel.anchor.as_ref()),
                key,
                shift,
                additive,
            );
            intents.push(self.select(selection.selected, selection.anchor));
            if !shift && !additive {
                intents.push(view.row_intent(section, change.row, RowAction::OpenChanges));
            }
            if is_pointer {
                ui.memory_mut(|memory| memory.request_focus_with_filter(id, ARROWS));
            }
        }
        Ok(())
    }
}

impl List<'_, '_> {
    /// One stash entry (`stash-list.tsx`): archive icon, message, an Apply text button and a Drop
    /// icon button, both disabled while a stash push, apply or drop runs. Neither asks first.
    fn stash_row(
        &self,
        ui: &mut Ui,
        rect: Rect,
        entry: &GitStashEntry,
        icons: &mut impl PaintIcon<Icon>,
        intents: &mut Vec<ShellIntent>,
    ) -> AppResult<()> {
        let view = self.view;
        let appearance = view.appearance;
        let id = self.id.with(("stash", entry.index));
        let center = rect.center().y;
        if ui.rect_contains_pointer(rect) {
            ui.painter().rect_filled(rect, 0.0, appearance.hover);
        }
        let is_enabled = !view.is_stashing;
        let opacity = if is_enabled { 1.0 } else { DISABLED_OPACITY };
        let sense = if is_enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let drop = Rect::from_center_size(
            pos2(rect.right() - PADDING_X - STASH_DROP_SIZE / 2.0, center),
            vec2(STASH_DROP_SIZE, STASH_DROP_SIZE),
        );
        let apply_label = view.text("git.stashApply");
        let apply_text = ui.painter().layout_no_wrap(
            apply_label.clone(),
            FontId::proportional(TEXT_SIZE),
            appearance.foreground.gamma_multiply(opacity),
        );
        let apply = Rect::from_min_max(
            pos2(
                drop.left() - GAP - apply_text.size().x - 2.0 * STASH_APPLY_PADDING_X,
                center - STASH_APPLY_HEIGHT / 2.0,
            ),
            pos2(drop.left() - GAP, center + STASH_APPLY_HEIGHT / 2.0),
        );
        let archive = Rect::from_min_size(
            pos2(rect.left() + PADDING_X, center - HEADER_ICON_SIZE / 2.0),
            vec2(HEADER_ICON_SIZE, HEADER_ICON_SIZE),
        );
        icons.paint(ui, archive, Icon::Archive, appearance.foreground, 0.0)?;
        let message = truncated(
            ui,
            entry.message.clone(),
            FontId::proportional(TEXT_SIZE),
            appearance.foreground,
            apply.left() - GAP - archive.right() - GAP,
        );
        ui.painter().galley(
            pos2(archive.right() + GAP, center - message.size().y / 2.0),
            message,
            appearance.foreground,
        );
        let applied = ui.interact(apply, id.with("apply"), sense);
        applied.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, is_enabled, &apply_label)
        });
        if is_enabled && applied.hovered() {
            ui.painter()
                .rect_filled(apply, ACTION_RADIUS, appearance.badge);
        }
        ui.painter().galley(
            apply.center() - apply_text.size() / 2.0,
            apply_text,
            appearance.foreground,
        );
        let drop_label = view.text("git.stashDrop");
        let dropped = ui.interact(drop, id.with("drop"), sense);
        dropped.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, is_enabled, &drop_label)
        });
        view.tooltips.show(
            &dropped,
            &drop_label,
            egui::RectAlign::BOTTOM,
            view.tooltip_appearance,
        );
        if is_enabled && dropped.hovered() {
            ui.painter()
                .rect_filled(drop, ACTION_RADIUS, appearance.badge);
        }
        icons.paint(
            ui,
            Rect::from_center_size(drop.center(), vec2(HEADER_ICON_SIZE, HEADER_ICON_SIZE)),
            Icon::Trash2,
            appearance.error.gamma_multiply(opacity),
            0.0,
        )?;
        for button in [&applied, &dropped] {
            if button.has_focus() {
                ui.memory_mut(|memory| memory.set_focus_lock_filter(button.id, ARROWS));
            }
        }
        let project = view.project.clone();
        let index = entry.index;
        if is_enabled && applied.clicked() {
            intents.push(ShellIntent::Git(Intent::StashApply { project, index }));
        } else if is_enabled && dropped.clicked() {
            intents.push(ShellIntent::Git(Intent::StashDrop { project, index }));
        }
        Ok(())
    }
}

fn paint_path(
    ui: &Ui,
    rect: Rect,
    left: f32,
    right: f32,
    (name, directory): (&str, Option<&str>),
    appearance: &Appearance,
) {
    let font = FontId::new(TEXT_SIZE, FontFamily::Proportional);
    let directory_color = appearance.muted.gamma_multiply(DIRECTORY_OPACITY);
    let natural = |text: &str, color| {
        ui.painter()
            .layout_no_wrap(text.into(), font.clone(), color)
            .size()
            .x
    };
    let available = (right - left).max(0.0);
    let name_natural = natural(name, appearance.foreground);
    let Some(directory) = directory else {
        let name = truncated(ui, name.into(), font, appearance.foreground, available);
        ui.painter().galley(
            pos2(left, rect.center().y - name.size().y / 2.0),
            name,
            appearance.foreground,
        );
        return;
    };
    let directory_natural = natural(directory, directory_color);
    let shared = (available - GAP).max(0.0);
    let total = name_natural + directory_natural;
    let name_width = if total <= shared || total == 0.0 {
        name_natural
    } else {
        shared * name_natural / total
    };
    let name = truncated(
        ui,
        name.into(),
        font.clone(),
        appearance.foreground,
        name_width,
    );
    let directory = truncated(
        ui,
        directory.into(),
        font,
        directory_color,
        (shared - name_width).max(0.0),
    );
    ui.painter().galley(
        pos2(left, rect.center().y - name.size().y / 2.0),
        name,
        appearance.foreground,
    );
    ui.painter().galley(
        pos2(
            left + name_width + GAP,
            rect.center().y - directory.size().y / 2.0,
        ),
        directory,
        directory_color,
    );
}

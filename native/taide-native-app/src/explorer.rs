use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use eframe::egui::{self, Color32, Id, Key, Modifiers, Response, Ui};
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::locale::ResolvedLocale;
use taide_model::theme::ResolvedTheme;
use taide_model::tree::{TreeEntryKind, TreeRow, TreeRowPage};

use crate::problems_icons::{FileColor, Glyph, Icons, file, folder};

const ROW_HEIGHT: f32 = 22.0;
const ROW_INDENT: f32 = 12.0;
const ROW_FONT_SIZE: f32 = 12.0;
const CHEVRON_SLOT_SIZE: f32 = 16.0;
const CHEVRON_SIZE: f32 = 12.0;
const ROW_ICON_SIZE: f32 = 14.0;
const ROW_SLOT_GAP: f32 = 4.0;
const COLLAPSED_CHEVRON_ANGLE: f32 = 0.0;
const EXPANDED_CHEVRON_ANGLE: f32 = std::f32::consts::FRAC_PI_2;
const HEADER_HEIGHT: f32 = 32.0;
const HEADER_PADDING: f32 = 8.0;
const HEADER_GAP: f32 = 4.0;
const HEADER_FONT_SIZE: f32 = 12.0;
const HEADER_BORDER: f32 = 1.0;
const TOOLBAR_TRANSITION_SECONDS: f32 = 0.15;
const TYPEAHEAD_RESET_SECONDS: f64 = 0.7;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenameRequest {
    pub token: u64,
    pub from: String,
    pub to: String,
}

pub struct RenameReply {
    pub project: ProjectId,
    pub request: RenameRequest,
    pub result: AppResult<TreeRowPage>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateRequest {
    pub token: u64,
    pub path: String,
    pub parent: String,
    pub kind: TreeEntryKind,
}

pub struct CreatedEntry {
    pub page: TreeRowPage,
    pub layout: Option<taide_model::layout::ProjectLayout>,
}

pub struct CreateReply {
    pub project: ProjectId,
    pub request: CreateRequest,
    pub result: AppResult<CreatedEntry>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Toggle(String),
    Open {
        path: String,
        preview: bool,
    },
    OpenToSide(TreeRow),
    OpenWith {
        row: TreeRow,
        mode: crate::open_with::Mode,
    },
    Rename(RenameRequest),
    Create(CreateRequest),
    RequestDelete(TreeRow),
    Paste(crate::explorer_clipboard::Request),
    CopyText(String),
    RevealPath(String),
    OpenInBrowser(String),
    Refresh,
    Collapse,
}

pub fn open_to_side_request(
    project: &ProjectId,
    row: &TreeRow,
    layout: Option<&taide_model::layout::ProjectLayout>,
    scope: &taide_native_ui::shell::WindowScope,
) -> Option<taide_model::layout::OpenTabInSplitRequest> {
    if row.kind != TreeEntryKind::File {
        return None;
    }
    let layout = layout?;
    let pane = match scope {
        taide_native_ui::shell::WindowScope::Main => &layout.focused_pane,
        taide_native_ui::shell::WindowScope::Auxiliary {
            project: owner,
            slot,
        } => {
            if owner != project {
                return None;
            }
            &layout
                .auxiliary_windows
                .iter()
                .find(|window| window.slot == *slot)?
                .focused_pane
        }
    };
    Some(taide_model::layout::OpenTabInSplitRequest {
        project_id: project.clone(),
        target_pane: pane.clone(),
        edge: taide_model::layout::DropEdge::Right,
        kind: taide_model::layout::TabKind::File {
            path: row.path.clone(),
        },
        title: row.name.clone(),
        preview: false,
    })
}

pub struct Output {
    pub actions: Vec<Action>,
    pub rows: HashMap<String, Response>,
    pub icons: HashMap<String, RowIcon>,
    pub draft_icon: Option<RowIcon>,
    pub icon_error: Option<AppError>,
    pub input: Option<Response>,
    pub(crate) validation_error: Option<String>,
    pub toolbar: HashMap<&'static str, Response>,
    pub menu: HashMap<&'static str, Response>,
    pub blank: Option<Response>,
}

pub struct Appearance {
    files: HashMap<FileColor, Color32>,
}

impl Appearance {
    pub fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        Ok(Self {
            files: FileColor::ALL
                .into_iter()
                .map(|color| Ok((color, crate::presentation::color(theme, color.theme_key())?)))
                .collect::<AppResult<_>>()?,
        })
    }

    pub fn file_color(&self, color: FileColor) -> Color32 {
        self.files[&color]
    }
}

pub struct RowIcons<'a> {
    pub glyphs: &'a mut Icons,
    pub appearance: &'a Appearance,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowIcon {
    pub chevron: Option<(egui::Rect, f32)>,
    pub glyph: Glyph,
    pub color: FileColor,
    pub rect: egui::Rect,
    pub label_left: f32,
}

impl RowIcon {
    fn new(
        row_rect: egui::Rect,
        depth: u32,
        chevron_angle: Option<f32>,
        (glyph, color): (Glyph, FileColor),
    ) -> Self {
        let slot_left = row_rect.left() + depth as f32 * ROW_INDENT;
        let center_y = row_rect.center().y;
        let chevron = chevron_angle.map(|angle| {
            (
                egui::Rect::from_center_size(
                    egui::pos2(slot_left + CHEVRON_SLOT_SIZE / 2.0, center_y),
                    egui::Vec2::splat(CHEVRON_SIZE),
                ),
                angle,
            )
        });
        let rect = egui::Rect::from_center_size(
            egui::pos2(
                slot_left + CHEVRON_SLOT_SIZE + ROW_SLOT_GAP + ROW_ICON_SIZE / 2.0,
                center_y,
            ),
            egui::Vec2::splat(ROW_ICON_SIZE),
        );
        Self {
            chevron,
            glyph,
            color,
            rect,
            label_left: rect.right() + ROW_SLOT_GAP,
        }
    }

    fn of_row(row_rect: egui::Rect, row: &TreeRow) -> Self {
        match row.kind {
            TreeEntryKind::Directory => Self::new(
                row_rect,
                row.depth,
                Some(if row.expanded {
                    EXPANDED_CHEVRON_ANGLE
                } else {
                    COLLAPSED_CHEVRON_ANGLE
                }),
                folder(&row.name, row.expanded),
            ),
            TreeEntryKind::File => Self::new(row_rect, row.depth, None, file(&row.name)),
        }
    }

    fn of_draft(row_rect: egui::Rect, draft: &RenameDraft) -> Self {
        let icon = match draft.row.kind {
            TreeEntryKind::Directory => folder(&draft.name, false),
            TreeEntryKind::File => file(&draft.name),
        };
        Self::new(row_rect, draft.row.depth, None, icon)
    }

    fn paint(&self, ui: &Ui, icons: &mut RowIcons<'_>) -> AppResult<()> {
        if let Some((rect, angle)) = self.chevron {
            icons.glyphs.paint(
                ui,
                rect,
                Glyph::ChevronRight,
                ui.visuals().text_color(),
                angle,
            )?;
        }
        icons.glyphs.paint(
            ui,
            self.rect,
            self.glyph,
            icons.appearance.file_color(self.color),
            0.0,
        )
    }
}

fn paint_row_icon(
    ui: &Ui,
    icon: &RowIcon,
    icons: &mut Option<RowIcons<'_>>,
    error: &mut Option<AppError>,
) {
    if let Some(icons) = icons
        && let Err(failure) = icon.paint(ui, icons)
    {
        error.get_or_insert(failure);
    }
}

pub struct RenameDraft {
    pub row: TreeRow,
    pub name: String,
    pub error: Option<String>,
    focus: bool,
    composing: bool,
}

pub struct CreateDraft {
    pub parent: String,
    pub input: RenameDraft,
}

struct DraftOutput {
    response: Response,
    commit: bool,
    cancel: bool,
}

impl RenameDraft {
    fn show(&mut self, ui: &mut Ui, locale: &ResolvedLocale, id: Id, pending: bool) -> DraftOutput {
        let owns_focus = ui.memory(|memory| memory.has_focus(id));
        let mut ime_frame = false;
        let mut enter_pressed = false;
        let mut escape_pressed = false;
        if owns_focus && ui.is_enabled() {
            ui.input_mut(|input| {
                for event in &input.events {
                    if let egui::Event::Ime(event) = event {
                        ime_frame = true;
                        match event {
                            egui::ImeEvent::Preedit { text, .. } => {
                                self.composing = !text.is_empty()
                            }
                            egui::ImeEvent::Commit(_) => self.composing = false,
                            _ => {}
                        }
                    }
                }
                input.events.retain(|event| match event {
                    egui::Event::Key {
                        key: Key::Enter,
                        pressed: true,
                        ..
                    } => {
                        enter_pressed = true;
                        false
                    }
                    egui::Event::Key {
                        key: Key::Escape,
                        pressed: true,
                        ..
                    } => {
                        escape_pressed = true;
                        false
                    }
                    _ => true,
                });
            });
        }
        let commit = enter_pressed && !self.composing && !ime_frame;
        let escape = escape_pressed && !self.composing && !ime_frame;
        let mut output = egui::TextEdit::singleline(&mut self.name)
            .id(id)
            .desired_width(ui.available_width())
            .hint_text(crate::presentation::message(
                locale,
                "explorer.entryNamePlaceholder",
                &[],
            ))
            .show(ui);
        let response = output.response.response;
        if self.focus && ui.is_enabled() {
            self.focus = false;
            response.request_focus();
            output
                .state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(self.name.chars().count()),
                )));
            output.state.store(ui.ctx(), id);
        }
        let cancel = escape || (ui.is_enabled() && !pending && response.lost_focus() && !commit);
        DraftOutput {
            response,
            commit,
            cancel,
        }
    }
}

fn name_error(
    name: &str,
    rows: &[TreeRow],
    destination: &str,
    previous: Option<&str>,
) -> Option<&'static str> {
    let Some(last) = name.split('/').rfind(|segment| !segment.is_empty()) else {
        return Some("explorer.entryNameInvalidChar");
    };
    if last == "." || last == ".." {
        return Some("explorer.entryNameReserved");
    }
    if last.chars().any(|value| "\\:*?\"<>|".contains(value)) || last.ends_with('.') {
        return Some("explorer.entryNameInvalidChar");
    }
    let parent = Path::new(destination.trim_end_matches('/')).parent();
    if rows.iter().any(|row| {
        Some(row.path.as_str()) != previous
            && Path::new(&row.path).parent() == parent
            && row.name == last
    }) {
        return Some("explorer.entryNameDuplicate");
    }
    None
}

#[derive(Default)]
pub struct Explorer {
    pub selected: Option<String>,
    pub rename: Option<RenameDraft>,
    pub create: Option<CreateDraft>,
    pub root: Option<String>,
    pub title: Option<String>,
    pending_create: Option<CreateRequest>,
    pending_parent: Option<(String, TreeEntryKind)>,
    pending: Option<RenameRequest>,
    serial: u64,
    reveal: Option<String>,
    selected_paths: BTreeSet<String>,
    selection_anchor: Option<String>,
    selection_primary: Option<String>,
    typeahead_buffer: String,
    typeahead_deadline: Option<f64>,
    composing: bool,
    clipboard: Option<crate::explorer_clipboard::Entry>,
    scroll_offset: f32,
}

fn revealing_scroll_offset(current: f32, viewport_height: f32, index: usize) -> Option<f32> {
    let row_top = index as f32 * ROW_HEIGHT;
    let row_bottom = row_top + ROW_HEIGHT;
    if row_bottom > current + viewport_height {
        return Some(row_bottom - viewport_height);
    }
    (row_top < current).then_some(row_top)
}

impl Explorer {
    pub fn clipboard(&self) -> Option<&crate::explorer_clipboard::Entry> {
        self.clipboard.as_ref()
    }

    pub fn replace_clipboard(&mut self, entry: Option<crate::explorer_clipboard::Entry>) {
        self.clipboard = entry;
    }

    pub fn paste_finished(&mut self, clear_cut: bool, path: Option<String>) {
        if clear_cut {
            self.clipboard = None;
        }
        if let Some(path) = path {
            self.select_only(Some(path.clone()));
            self.reveal = Some(path);
        }
    }

    fn capture_clipboard(&mut self, row: &TreeRow, mode: crate::explorer_clipboard::Mode) {
        self.clipboard = Some(crate::explorer_clipboard::Entry {
            mode,
            path: row.path.clone(),
            kind: row.kind,
        });
    }

    fn paste_request(
        &mut self,
        row: Option<&TreeRow>,
        page: &TreeRowPage,
        suffix: &str,
    ) -> Option<Action> {
        let entry = self.clipboard.clone()?;
        let target =
            match row.filter(|row| page.rows.iter().any(|candidate| candidate.path == row.path)) {
                Some(row) if row.kind == TreeEntryKind::Directory => row.path.clone(),
                Some(row) => crate::explorer_clipboard::parent_dir(&row.path).into(),
                None => self.root.clone()?,
            };
        if entry.mode == crate::explorer_clipboard::Mode::Cut
            && crate::explorer_clipboard::parent_dir(&entry.path) == target
        {
            self.clipboard = None;
            return None;
        }
        self.serial = self.serial.checked_add(1)?;
        Some(Action::Paste(crate::explorer_clipboard::Request {
            token: self.serial,
            owner: None,
            entry,
            sibling_names: page
                .rows
                .iter()
                .filter(|row| crate::explorer_clipboard::parent_dir(&row.path) == target)
                .map(|row| row.name.clone())
                .collect(),
            target,
            conflict_suffix: suffix.into(),
        }))
    }
    pub fn selected_paths(&self) -> &BTreeSet<String> {
        &self.selected_paths
    }

    fn select_only(&mut self, path: Option<String>) {
        self.selected_paths = path.iter().cloned().collect();
        self.selection_anchor = path.clone();
        self.selection_primary = path.clone();
        self.selected = path;
    }

    fn synchronize_selection(&mut self) {
        if self.selected != self.selection_primary {
            self.select_only(self.selected.clone());
        }
    }

    fn select_row(&mut self, row: &TreeRow, rows: &[TreeRow], modifiers: Modifiers) {
        self.synchronize_selection();
        self.selected_paths = rows
            .iter()
            .filter(|row| self.selected_paths.contains(&row.path))
            .map(|row| row.path.clone())
            .collect();
        let clicked = rows.iter().position(|entry| entry.path == row.path);
        let anchor = self
            .selection_anchor
            .as_ref()
            .and_then(|path| rows.iter().position(|entry| &entry.path == path));
        let additive = modifiers.ctrl || modifiers.command || modifiers.mac_cmd;
        if modifiers.shift
            && let (Some(clicked), Some(anchor)) = (clicked, anchor)
        {
            if !additive {
                self.selected_paths.clear();
            }
            self.selected_paths.extend(
                rows[clicked.min(anchor)..=clicked.max(anchor)]
                    .iter()
                    .map(|entry| entry.path.clone()),
            );
            self.selected = Some(row.path.clone());
            self.selection_primary = self.selected.clone();
            return;
        }
        if additive {
            if !self.selected_paths.remove(&row.path) {
                self.selected_paths.insert(row.path.clone());
            }
            self.selection_anchor = Some(row.path.clone());
            self.selected = if self.selected_paths.contains(&row.path) {
                Some(row.path.clone())
            } else {
                rows.iter()
                    .rfind(|entry| self.selected_paths.contains(&entry.path))
                    .map(|entry| entry.path.clone())
            };
            self.selection_primary = self.selected.clone();
            return;
        }
        self.select_only(Some(row.path.clone()));
    }

    fn select_index(&mut self, rows: &[TreeRow], index: Option<usize>) {
        if let Some(row) = index.and_then(|index| rows.get(index)) {
            self.select_only(Some(row.path.clone()));
            self.reveal = self.selected.clone();
        }
    }

    fn relative_path(&self, path: &str) -> Option<String> {
        let root = self.root.as_ref()?;
        let prefix = if root.ends_with('/') {
            root.clone()
        } else {
            format!("{root}/")
        };
        Some(path.strip_prefix(&prefix).unwrap_or(path).into())
    }

    pub fn start_create(&mut self, kind: TreeEntryKind, page: &TreeRowPage) -> Option<Action> {
        self.synchronize_selection();
        if self.pending.is_some() || self.pending_create.is_some() || self.pending_parent.is_some()
        {
            return None;
        }
        let selected = self
            .selected
            .as_ref()
            .and_then(|path| page.rows.iter().find(|row| &row.path == path));
        let parent = match selected {
            Some(row) if row.kind == TreeEntryKind::Directory => row.path.clone(),
            Some(row) => Path::new(&row.path).parent()?.to_str()?.into(),
            None => self.root.clone()?,
        };
        if self.root.as_ref() != Some(&parent)
            && !page
                .rows
                .iter()
                .any(|row| row.path == parent && row.kind == TreeEntryKind::Directory)
        {
            return None;
        }
        if let Some(row) = page.rows.iter().find(|row| row.path == parent)
            && !row.expanded
        {
            self.rename = None;
            self.create = None;
            self.pending_parent = Some((parent.clone(), kind));
            return Some(Action::Toggle(parent));
        }
        self.install_create(parent, kind, page);
        None
    }

    fn install_create(&mut self, parent: String, kind: TreeEntryKind, page: &TreeRowPage) {
        let depth = page
            .rows
            .iter()
            .find(|row| row.path == parent)
            .map_or(0, |row| row.depth + 1);
        self.rename = None;
        self.create = Some(CreateDraft {
            parent,
            input: RenameDraft {
                row: TreeRow {
                    path: String::new(),
                    name: String::new(),
                    kind,
                    depth,
                    expanded: false,
                    has_children: false,
                },
                name: String::new(),
                error: None,
                focus: true,
                composing: false,
            },
        });
    }

    pub fn toggle_finished(&mut self, path: &str, page: Option<&TreeRowPage>) {
        if !self
            .pending_parent
            .as_ref()
            .is_some_and(|(parent, _)| parent == path)
        {
            return;
        }
        let Some((parent, kind)) = self.pending_parent.take() else {
            return;
        };
        let Some(page) = page else {
            return;
        };
        if self.root.as_ref() == Some(&parent)
            || page.rows.iter().any(|row| {
                row.path == parent && row.kind == TreeEntryKind::Directory && row.expanded
            })
        {
            self.install_create(parent, kind, page);
        }
    }

    pub fn commit_create(
        &mut self,
        rows: &[TreeRow],
        locale: &ResolvedLocale,
    ) -> Option<CreateRequest> {
        if self.pending_create.is_some() {
            return None;
        }
        let draft = self.create.as_mut()?;
        let name = draft.input.name.trim();
        if name.is_empty() {
            self.create = None;
            return None;
        }
        let path = format!("{}/{}", draft.parent.trim_end_matches('/'), name);
        if let Some(error) = name_error(name, rows, &path, None) {
            draft.input.error = Some(crate::presentation::message(
                locale,
                error,
                &[("name", name)],
            ));
            return None;
        }
        self.serial = self.serial.checked_add(1)?;
        let request = CreateRequest {
            token: self.serial,
            path,
            parent: draft.parent.clone(),
            kind: draft.input.row.kind,
        };
        draft.input.error = None;
        self.pending_create = Some(request.clone());
        Some(request)
    }

    pub fn create_finished(&mut self, request: &CreateRequest, result: Result<(), String>) {
        if self.pending_create.as_ref() != Some(request) {
            return;
        }
        self.pending_create = None;
        match result {
            Ok(()) => {
                self.create = None;
                self.select_only(Some(request.path.clone()));
                self.reveal = self.selected.clone();
            }
            Err(error) => {
                if let Some(draft) = &mut self.create {
                    draft.input.error = Some(error);
                    draft.input.focus = true;
                }
            }
        }
    }

    pub fn retry_create(
        &mut self,
        failed: &CreateRequest,
        rows: &[TreeRow],
        locale: &ResolvedLocale,
    ) -> Option<CreateRequest> {
        if self.pending_create.is_some() {
            return None;
        }
        let name = Path::new(&failed.path)
            .strip_prefix(&failed.parent)
            .ok()?
            .to_str()?;
        if let Some(error) = name_error(name, rows, &failed.path, None) {
            if let Some(draft) = &mut self.create {
                draft.input.error = Some(crate::presentation::message(
                    locale,
                    error,
                    &[("name", name)],
                ));
            }
            return None;
        }
        self.serial = self.serial.checked_add(1)?;
        let request = CreateRequest {
            token: self.serial,
            ..failed.clone()
        };
        if let Some(draft) = &mut self.create {
            draft.input.error = None;
        }
        self.pending_create = Some(request.clone());
        Some(request)
    }

    pub fn start_rename(&mut self, row: &TreeRow) {
        if self.pending.is_some() || self.pending_create.is_some() || self.pending_parent.is_some()
        {
            return;
        }
        self.select_only(Some(row.path.clone()));
        self.create = None;
        self.rename = Some(RenameDraft {
            row: row.clone(),
            name: row.name.clone(),
            error: None,
            focus: true,
            composing: false,
        });
        self.reveal = Some(row.path.clone());
    }

    pub fn cancel_rename(&mut self) {
        self.rename = None;
    }

    pub fn commit_rename(
        &mut self,
        rows: &[TreeRow],
        locale: &ResolvedLocale,
    ) -> Option<RenameRequest> {
        if self.pending.is_some() {
            return None;
        }
        let draft = self.rename.as_mut()?;
        let name = draft.name.trim();
        if name.is_empty() || name == draft.row.name {
            self.cancel_rename();
            return None;
        }
        let parent = Path::new(&draft.row.path).parent()?.to_str()?;
        let destination = format!("{}/{}", parent.trim_end_matches('/'), name);
        let error = name_error(name, rows, &destination, Some(&draft.row.path));
        if let Some(error) = error {
            draft.error = Some(crate::presentation::message(
                locale,
                error,
                &[("name", name)],
            ));
            return None;
        }
        self.serial = self.serial.checked_add(1)?;
        let request = RenameRequest {
            token: self.serial,
            from: draft.row.path.clone(),
            to: destination,
        };
        draft.error = None;
        self.pending = Some(request.clone());
        Some(request)
    }

    pub fn rename_finished(&mut self, request: &RenameRequest, result: Result<(), String>) {
        if self.pending.as_ref() != Some(request) {
            return;
        }
        self.pending = None;
        match result {
            Ok(()) => {
                self.select_only(Some(request.to.clone()));
                self.reveal = Some(request.to.clone());
                self.rename = None;
            }
            Err(error) => {
                if let Some(draft) = self.rename.as_mut() {
                    draft.error = Some(error);
                    draft.focus = true;
                }
            }
        }
    }

    pub fn retry_rename(
        &mut self,
        failed: &RenameRequest,
        rows: &[TreeRow],
        locale: &ResolvedLocale,
    ) -> Option<RenameRequest> {
        if self.pending.is_some() {
            return None;
        }
        let name = Path::new(&failed.to)
            .strip_prefix(Path::new(&failed.from).parent()?)
            .ok()?
            .to_str()?;
        if let Some(error) = name_error(name, rows, &failed.to, Some(&failed.from)) {
            if let Some(draft) = self.rename.as_mut() {
                draft.error = Some(crate::presentation::message(
                    locale,
                    error,
                    &[("name", name)],
                ));
            }
            return None;
        }
        self.serial = self.serial.checked_add(1)?;
        let request = RenameRequest {
            token: self.serial,
            ..failed.clone()
        };
        if let Some(draft) = self.rename.as_mut() {
            draft.error = None;
        }
        self.pending = Some(request.clone());
        Some(request)
    }

    pub fn moved(&mut self, from: &Path, to: &Path) {
        self.synchronize_selection();
        if let Some(selected) = &self.selected
            && let Some(next) = crate::workspace_rename::moved_path(Path::new(selected), from, to)
        {
            self.selected = Some(next.to_string_lossy().into_owned());
        }
        self.selected_paths = self
            .selected_paths
            .iter()
            .map(|path| {
                crate::workspace_rename::moved_path(Path::new(path), from, to)
                    .map_or_else(|| path.clone(), |next| next.to_string_lossy().into_owned())
            })
            .collect();
        if let Some(anchor) = &self.selection_anchor
            && let Some(next) = crate::workspace_rename::moved_path(Path::new(anchor), from, to)
        {
            self.selection_anchor = Some(next.to_string_lossy().into_owned());
        }
        self.selection_primary = self.selected.clone();
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        project: &ProjectId,
        page: &TreeRowPage,
        locale: &ResolvedLocale,
    ) -> Output {
        self.render(ui, project, page, locale, None)
    }

    pub fn show_with_icons(
        &mut self,
        ui: &mut Ui,
        project: &ProjectId,
        page: &TreeRowPage,
        locale: &ResolvedLocale,
        icons: RowIcons<'_>,
    ) -> Output {
        self.render(ui, project, page, locale, Some(icons))
    }

    fn render(
        &mut self,
        ui: &mut Ui,
        project: &ProjectId,
        page: &TreeRowPage,
        locale: &ResolvedLocale,
        mut icons: Option<RowIcons<'_>>,
    ) -> Output {
        self.synchronize_selection();
        let time = ui.input(|input| input.time);
        if self
            .typeahead_deadline
            .is_some_and(|deadline| time >= deadline)
        {
            self.typeahead_buffer.clear();
            self.typeahead_deadline = None;
        }
        let mut output = Output {
            actions: Vec::new(),
            rows: HashMap::new(),
            icons: HashMap::new(),
            draft_icon: None,
            icon_error: None,
            input: None,
            validation_error: None,
            toolbar: HashMap::new(),
            menu: HashMap::new(),
            blank: None,
        };
        let tree_id = Id::new(("native-tree-focus", project));
        let is_toolbar_visible = ui.rect_contains_pointer(ui.max_rect())
            || self.rename.is_some()
            || self.create.is_some()
            || ui.memory(|memory| {
                memory.has_focus(tree_id)
                    || crate::explorer_toolbar::KEYS.iter().any(|key| {
                        memory.has_focus(Id::new(("native-explorer-toolbar", project, key)))
                    })
            });
        let toolbar_opacity = ui.ctx().animate_bool_with_time(
            Id::new(("native-explorer-toolbar-opacity", project)),
            is_toolbar_visible,
            TOOLBAR_TRANSITION_SECONDS,
        );
        ui.spacing_mut().item_spacing.y = 0.0;
        let header = ui.horizontal(|ui| {
            let width = ui.available_width();
            ui.set_min_size(egui::vec2(width, HEADER_HEIGHT));
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(HEADER_PADDING);
            let toolbar_width = crate::explorer_toolbar::WIDTH;
            let caption_width =
                (width - HEADER_PADDING * 2.0 - toolbar_width - HEADER_GAP).max(0.0);
            let caption = self
                .title
                .clone()
                .unwrap_or_else(|| crate::presentation::message(locale, "explorer.title", &[]));
            ui.add_sized(
                egui::vec2(caption_width, HEADER_HEIGHT),
                egui::Label::new(
                    egui::RichText::new(caption.to_uppercase()).size(HEADER_FONT_SIZE),
                )
                .truncate(),
            );
            ui.add_space(HEADER_GAP);
            ui.spacing_mut().item_spacing.x = crate::explorer_toolbar::GAP;
            for (key, kind) in [
                ("explorer.newFile", Some(TreeEntryKind::File)),
                ("explorer.newFolder", Some(TreeEntryKind::Directory)),
                ("explorer.refresh", None),
                ("explorer.collapseAll", None),
            ] {
                let response = crate::explorer_toolbar::button(
                    ui,
                    key,
                    Id::new(("native-explorer-toolbar", project, key)),
                    toolbar_opacity,
                    locale,
                );
                if response.clicked() {
                    if let Some(kind) = kind {
                        if let Some(action) = self.start_create(kind, page) {
                            output.actions.push(action);
                        }
                    } else if key == "explorer.refresh" {
                        output.actions.push(Action::Refresh);
                    } else {
                        output.actions.push(Action::Collapse);
                    }
                }
                output.toolbar.insert(key, response);
            }
        });
        ui.painter().line_segment(
            [
                header.response.rect.left_bottom(),
                header.response.rect.right_bottom(),
            ],
            egui::Stroke::new(
                HEADER_BORDER,
                ui.visuals().widgets.noninteractive.bg_stroke.color,
            ),
        );
        let tree = ui.interact(
            ui.max_rect(),
            tree_id,
            egui::Sense::focusable_noninteractive(),
        );
        let tree_has_focus = tree.has_focus();
        if tree_has_focus && ui.is_enabled() && !egui::Popup::is_any_open(ui.ctx()) {
            let has_arrow = ui.input(|input| {
                input.events.iter().any(|event| {
                    matches!(
                        event,
                        egui::Event::Key {
                            key: Key::ArrowUp | Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight,
                            pressed: true,
                            ..
                        }
                    )
                })
            });
            ui.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    tree_id,
                    egui::EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        ..Default::default()
                    },
                );
                if has_arrow {
                    memory.move_focus(egui::FocusDirection::None);
                }
            });
        }
        let ime_frame = ui.input(|input| {
            let mut frame = false;
            for event in &input.events {
                if let egui::Event::Ime(event) = event {
                    frame = true;
                    if tree_has_focus {
                        match event {
                            egui::ImeEvent::Preedit { text, .. } => {
                                self.composing = !text.is_empty();
                            }
                            egui::ImeEvent::Commit(_) => self.composing = false,
                            _ => {}
                        }
                    }
                }
            }
            frame
        });
        if !tree_has_focus {
            self.composing = false;
        }
        if self.rename.is_none()
            && self.create.is_none()
            && tree_has_focus
            && ui.is_enabled()
            && !egui::Popup::is_any_open(ui.ctx())
            && !ime_frame
            && !self.composing
            && self.pending_parent.is_none()
        {
            self.keyboard(ui, page, locale, &mut output.actions);
        }
        let draft_row = self
            .rename
            .as_ref()
            .filter(|draft| !page.rows.iter().any(|row| row.path == draft.row.path))
            .map(|draft| draft.row.clone());
        let create_index = self.create.as_ref().map(|draft| {
            page.rows
                .iter()
                .position(|row| row.path == draft.parent)
                .map_or(0, |index| index + 1)
        });
        let row_count = page.rows.len()
            + usize::from(draft_row.is_some())
            + usize::from(create_index.is_some());
        let reveal_index = self
            .create
            .as_ref()
            .filter(|draft| draft.input.focus)
            .and(create_index)
            .or_else(|| {
                self.reveal.as_ref().and_then(|path| {
                    page.rows
                        .iter()
                        .chain(draft_row.iter())
                        .position(|row| &row.path == path)
                })
            });
        let mut area = egui::ScrollArea::vertical().id_salt(("native-tree", project));
        if let Some(offset) = reveal_index.and_then(|index| {
            revealing_scroll_offset(self.scroll_offset, ui.available_height(), index)
        }) {
            area = area.vertical_scroll_offset(offset);
        }
        let mut start_rename = None;
        let mut context_create = None;
        let mut commit = false;
        let mut cancel = false;
        let area =
            area.auto_shrink([false, false])
                .show_rows(ui, ROW_HEIGHT, row_count, |ui, range| {
                    for index in range {
                        if create_index == Some(index) {
                            let draft = self.create.as_mut().expect("create row exists");
                            ui.horizontal(|ui| {
                                ui.set_min_height(ROW_HEIGHT);
                                let row_rect = egui::Rect::from_min_size(
                                    ui.max_rect().left_top(),
                                    egui::vec2(ui.available_width(), ROW_HEIGHT),
                                );
                                let icon = RowIcon::of_draft(row_rect, &draft.input);
                                paint_row_icon(ui, &icon, &mut icons, &mut output.icon_error);
                                output.draft_icon = Some(icon);
                                ui.add_space(icon.label_left - row_rect.left());
                                let result = draft.input.show(
                                    ui,
                                    locale,
                                    Id::new(("native-create-input", project)),
                                    self.pending_create.is_some(),
                                );
                                commit |= result.commit;
                                cancel |= result.cancel;
                                output.input = Some(result.response);
                            });
                            continue;
                        }
                        let index = index
                            - usize::from(
                                create_index.is_some_and(|draft_index| index > draft_index),
                            );
                        let Some(row) = page.rows.get(index).or(draft_row.as_ref()) else {
                            continue;
                        };
                        ui.push_id(&row.path, |ui| {
                            ui.horizontal(|ui| {
                                ui.set_min_height(ROW_HEIGHT);
                                let row_rect = egui::Rect::from_min_size(
                                    ui.max_rect().left_top(),
                                    egui::vec2(ui.available_width(), ROW_HEIGHT),
                                );
                                ui.set_min_width(row_rect.width());
                                if self
                                    .rename
                                    .as_ref()
                                    .is_some_and(|draft| draft.row.path == row.path)
                                {
                                    let draft =
                                        self.rename.as_mut().expect("rename row is present");
                                    let icon = RowIcon::of_draft(row_rect, draft);
                                    paint_row_icon(ui, &icon, &mut icons, &mut output.icon_error);
                                    output.draft_icon = Some(icon);
                                    ui.add_space(icon.label_left - row_rect.left());
                                    let result = draft.show(
                                        ui,
                                        locale,
                                        Id::new(("native-rename-input", project, &row.path)),
                                        self.pending.is_some(),
                                    );
                                    commit |= result.commit;
                                    cancel |= result.cancel;
                                    output.input = Some(result.response);
                                    return;
                                }
                                let icon = RowIcon::of_row(row_rect, row);
                                ui.add_space(icon.label_left - row_rect.left());
                                let selected = self.selected_paths.contains(&row.path);
                                let response = ui.interact(
                                    row_rect,
                                    ui.id().with("full-row"),
                                    egui::Sense::click(),
                                );
                                if selected || response.hovered() {
                                    let fill = if selected {
                                        ui.visuals().selection.bg_fill
                                    } else {
                                        ui.visuals().widgets.hovered.weak_bg_fill
                                    };
                                    ui.painter().rect_filled(row_rect, 0.0, fill);
                                }
                                paint_row_icon(ui, &icon, &mut icons, &mut output.icon_error);
                                output.icons.insert(row.path.clone(), icon);
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&row.name).size(ROW_FONT_SIZE),
                                    )
                                    .truncate(),
                                );
                                response.widget_info(|| {
                                    egui::WidgetInfo::selected(
                                        egui::WidgetType::Button,
                                        ui.is_enabled(),
                                        selected,
                                        &row.name,
                                    )
                                });
                                if response.clicked() {
                                    let modifiers = ui.input(|input| input.modifiers);
                                    self.select_row(row, &page.rows, modifiers);
                                    ui.memory_mut(|memory| memory.request_focus(tree_id));
                                    if !ui.input(|input| {
                                        let modifiers = input.modifiers;
                                        modifiers.shift
                                            || modifiers.ctrl
                                            || modifiers.command
                                            || modifiers.mac_cmd
                                    }) {
                                        if row.kind == TreeEntryKind::Directory {
                                            output.actions.push(Action::Toggle(row.path.clone()));
                                        } else {
                                            output.actions.push(Action::Open {
                                                path: row.path.clone(),
                                                preview: false,
                                            });
                                        }
                                    }
                                }
                                if response.secondary_clicked() {
                                    self.select_only(Some(row.path.clone()));
                                    ui.memory_mut(|memory| memory.request_focus(tree_id));
                                }
                                response.context_menu(|ui| {
                                    for (key, kind) in [
                                        ("explorer.newFile", TreeEntryKind::File),
                                        ("explorer.newFolder", TreeEntryKind::Directory),
                                    ] {
                                        let response = ui.button(crate::presentation::message(
                                            locale,
                                            key,
                                            &[],
                                        ));
                                        if response.clicked() {
                                            context_create = Some((row.path.clone(), kind));
                                            ui.close();
                                        }
                                        output.menu.insert(key, response);
                                    }
                                    ui.separator();
                                    if row.kind == TreeEntryKind::File {
                                        let response = ui.button(crate::presentation::message(
                                            locale,
                                            "explorer.openToTheSide",
                                            &[],
                                        ));
                                        if response.clicked() {
                                            output.actions.push(Action::OpenToSide(row.clone()));
                                            ui.close();
                                        }
                                        output.menu.insert("explorer.openToTheSide", response);
                                    }
                                    if row.kind == TreeEntryKind::File
                                        && crate::open_with::preview_kind(&row.name).is_some()
                                    {
                                        let response = ui.menu_button(
                                            crate::presentation::message(
                                                locale,
                                                "explorer.openWith",
                                                &[],
                                            ),
                                            |ui| {
                                                for (key, mode) in [
                                                    (
                                                        "explorer.openWithEditor",
                                                        crate::open_with::Mode::Editor,
                                                    ),
                                                    (
                                                        "explorer.openWithPreview",
                                                        crate::open_with::Mode::Preview,
                                                    ),
                                                ] {
                                                    let response =
                                                        ui.button(crate::presentation::message(
                                                            locale,
                                                            key,
                                                            &[],
                                                        ));
                                                    if response.clicked() {
                                                        output.actions.push(Action::OpenWith {
                                                            row: row.clone(),
                                                            mode,
                                                        });
                                                        ui.close();
                                                    }
                                                    output.menu.insert(key, response);
                                                }
                                            },
                                        );
                                        output.menu.insert("explorer.openWith", response.response);
                                    }
                                    if row.kind == TreeEntryKind::File
                                        && row.name.rsplit_once('.').is_some_and(
                                            |(stem, extension)| {
                                                !stem.is_empty()
                                                    && matches!(
                                                        extension.to_lowercase().as_str(),
                                                        "html" | "htm"
                                                    )
                                            },
                                        )
                                    {
                                        let browser = ui.button(crate::presentation::message(
                                            locale,
                                            "explorer.openInBrowser",
                                            &[],
                                        ));
                                        if browser.clicked() {
                                            output
                                                .actions
                                                .push(Action::OpenInBrowser(row.path.clone()));
                                            ui.close();
                                        }
                                        output.menu.insert("explorer.openInBrowser", browser);
                                        ui.separator();
                                    }
                                    let reveal = ui.add(
                                        egui::Button::new(crate::presentation::message(
                                            locale,
                                            "explorer.revealInFinder",
                                            &[],
                                        ))
                                        .shortcut_text("⌥⌘R"),
                                    );
                                    if reveal.clicked() {
                                        output.actions.push(Action::RevealPath(row.path.clone()));
                                        ui.close();
                                    }
                                    output.menu.insert("explorer.revealInFinder", reveal);
                                    ui.separator();
                                    for (key, mode, shortcut) in [
                                        (
                                            "explorer.cut",
                                            crate::explorer_clipboard::Mode::Cut,
                                            "⌘X",
                                        ),
                                        (
                                            "explorer.copy",
                                            crate::explorer_clipboard::Mode::Copy,
                                            "⌘C",
                                        ),
                                    ] {
                                        let response = ui.add(
                                            egui::Button::new(crate::presentation::message(
                                                locale,
                                                key,
                                                &[],
                                            ))
                                            .shortcut_text(shortcut),
                                        );
                                        if response.clicked() {
                                            self.capture_clipboard(row, mode);
                                            ui.close();
                                        }
                                        output.menu.insert(key, response);
                                    }
                                    self.paste_button(ui, Some(row), page, locale, &mut output);
                                    let copy_path = ui.add(
                                        egui::Button::new(crate::presentation::message(
                                            locale,
                                            "explorer.copyPath",
                                            &[],
                                        ))
                                        .shortcut_text("⌥⌘C"),
                                    );
                                    if copy_path.clicked() {
                                        output.actions.push(Action::CopyText(row.path.clone()));
                                        ui.close();
                                    }
                                    output.menu.insert("explorer.copyPath", copy_path);
                                    let copy_relative = ui.add(
                                        egui::Button::new(crate::presentation::message(
                                            locale,
                                            "explorer.copyRelativePath",
                                            &[],
                                        ))
                                        .shortcut_text("⌥⇧⌘C"),
                                    );
                                    if copy_relative.clicked() {
                                        if let Some(path) = self.relative_path(&row.path) {
                                            output.actions.push(Action::CopyText(path));
                                        }
                                        ui.close();
                                    }
                                    output
                                        .menu
                                        .insert("explorer.copyRelativePath", copy_relative);
                                    ui.separator();
                                    if ui
                                        .button(crate::presentation::message(
                                            locale,
                                            "explorer.rename",
                                            &[],
                                        ))
                                        .clicked()
                                    {
                                        start_rename = Some(row.clone());
                                        ui.close();
                                    }
                                    let delete = ui.button(
                                        egui::RichText::new(crate::presentation::message(
                                            locale,
                                            "explorer.delete",
                                            &[],
                                        ))
                                        .color(ui.visuals().error_fg_color),
                                    );
                                    if delete.clicked() {
                                        output.actions.push(Action::RequestDelete(row.clone()));
                                        ui.close();
                                    }
                                    output.menu.insert("explorer.delete", delete);
                                });
                                output.rows.insert(row.path.clone(), response);
                            });
                        });
                    }
                });
        self.scroll_offset = area.state.offset.y;
        if self.rename.is_none() && self.create.is_none() {
            let content_bottom = area.inner_rect.top() + area.content_size.y - area.state.offset.y;
            let blank_top = content_bottom.clamp(area.inner_rect.top(), area.inner_rect.bottom());
            let rect = egui::Rect::from_min_max(
                egui::pos2(area.inner_rect.left(), blank_top),
                area.inner_rect.right_bottom(),
            );
            let response = ui.interact(
                rect,
                Id::new(("native-tree-blank", project)),
                egui::Sense::click(),
            );
            if response.secondary_clicked() || response.double_clicked() {
                self.select_only(None);
                ui.memory_mut(|memory| memory.request_focus(tree_id));
            }
            if response.double_clicked()
                && let Some(action) = self.start_create(TreeEntryKind::File, page)
            {
                output.actions.push(action);
            }
            response.context_menu(|ui| {
                for (key, kind) in [
                    ("explorer.newFile", TreeEntryKind::File),
                    ("explorer.newFolder", TreeEntryKind::Directory),
                ] {
                    let button = ui.button(crate::presentation::message(locale, key, &[]));
                    if button.clicked() {
                        self.select_only(None);
                        if let Some(action) = self.start_create(kind, page) {
                            output.actions.push(action);
                        }
                        ui.close();
                    }
                    output.menu.insert(key, button);
                }
                ui.separator();
                self.paste_button(ui, None, page, locale, &mut output);
            });
            output.blank = Some(response);
        }
        self.reveal = None;
        if let Some((path, kind)) = context_create {
            self.select_only(Some(path));
            if let Some(action) = self.start_create(kind, page) {
                output.actions.push(action);
            }
        }
        if let Some(row) = start_rename {
            self.start_rename(&row);
        }
        if cancel {
            self.cancel_rename();
            self.create = None;
            output.input = None;
            ui.memory_mut(|memory| memory.request_focus(tree_id));
            ui.ctx().request_repaint();
        } else if commit {
            if self.create.is_some() {
                if let Some(request) = self.commit_create(&page.rows, locale) {
                    output.actions.push(Action::Create(request));
                }
            } else if let Some(request) = self.commit_rename(&page.rows, locale) {
                output.actions.push(Action::Rename(request));
            }
            if self.rename.is_none() && self.create.is_none() {
                ui.memory_mut(|memory| memory.request_focus(tree_id));
            }
        }
        if let Some(input) = &output.input {
            output.validation_error = self
                .create
                .as_ref()
                .filter(|_| input.id == Id::new(("native-create-input", project)))
                .and_then(|draft| draft.input.error.clone())
                .or_else(|| {
                    self.rename
                        .as_ref()
                        .filter(|draft| {
                            input.id == Id::new(("native-rename-input", project, &draft.row.path))
                        })
                        .and_then(|draft| draft.error.clone())
                });
        }
        output
    }

    fn paste_button(
        &mut self,
        ui: &mut Ui,
        row: Option<&TreeRow>,
        page: &TreeRowPage,
        locale: &ResolvedLocale,
        output: &mut Output,
    ) {
        let response = ui.add_enabled(
            self.clipboard.is_some(),
            egui::Button::new(crate::presentation::message(locale, "explorer.paste", &[]))
                .shortcut_text("⌘V"),
        );
        if response.clicked() {
            let suffix = crate::presentation::message(locale, "explorer.pasteConflictSuffix", &[]);
            if let Some(action) = self.paste_request(row, page, &suffix) {
                output.actions.push(action);
            }
            ui.close();
        }
        output.menu.insert("explorer.paste", response);
    }

    fn keyboard(
        &mut self,
        ui: &mut Ui,
        page: &TreeRowPage,
        locale: &ResolvedLocale,
        actions: &mut Vec<Action>,
    ) {
        let (events, time, mut modifiers) =
            ui.input(|input| (input.events.clone(), input.time, input.modifiers));
        let suffix = crate::presentation::message(locale, "explorer.pasteConflictSuffix", &[]);
        let mut consumed = BTreeSet::new();
        let mut skip_space_text = false;
        for (index, event) in events.iter().enumerate() {
            match event {
                egui::Event::ModifiersChanged(next) => modifiers = *next,
                egui::Event::Copy => {
                    if self.key_press(Key::C, modifiers, page, &suffix, actions) {
                        consumed.insert(index);
                    }
                }
                egui::Event::Cut => {
                    if self.key_press(Key::X, modifiers, page, &suffix, actions) {
                        consumed.insert(index);
                    }
                }
                egui::Event::Paste(_) => {
                    if self.key_press(Key::V, modifiers, page, &suffix, actions) {
                        consumed.insert(index);
                    }
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers: next,
                    ..
                } => {
                    modifiers = *next;
                    let shortcut_key = if *key == Key::Paste { Key::V } else { *key };
                    let handled = self.key_press(shortcut_key, modifiers, page, &suffix, actions);
                    skip_space_text = handled && *key == Key::Space;
                    if handled {
                        consumed.insert(index);
                    }
                }
                egui::Event::Text(value) => {
                    if skip_space_text && value == " " {
                        skip_space_text = false;
                        consumed.insert(index);
                        continue;
                    }
                    skip_space_text = false;
                    if value.encode_utf16().count() == 1
                        && !modifiers.alt
                        && !modifiers.ctrl
                        && !modifiers.command
                        && !modifiers.mac_cmd
                        && self.typeahead(&page.rows, value, time)
                    {
                        consumed.insert(index);
                    }
                }
                _ => {}
            }
            if self.rename.is_some() || self.create.is_some() || self.pending_parent.is_some() {
                break;
            }
        }
        ui.input_mut(|input| {
            let mut index = 0;
            input.events.retain(|_| {
                let keep = !consumed.contains(&index);
                index += 1;
                keep
            });
        });
    }

    fn key_press(
        &mut self,
        key: Key,
        modifiers: Modifiers,
        page: &TreeRowPage,
        suffix: &str,
        actions: &mut Vec<Action>,
    ) -> bool {
        let matches_shortcut = |expected: Modifiers| {
            modifiers.matches_exact(expected)
                && (!cfg!(target_os = "macos") || modifiers.ctrl == expected.ctrl)
        };
        if key == Key::N {
            let kind = if matches_shortcut(Modifiers::COMMAND | Modifiers::SHIFT) {
                Some(TreeEntryKind::Directory)
            } else if matches_shortcut(Modifiers::COMMAND) {
                Some(TreeEntryKind::File)
            } else {
                None
            };
            if let Some(kind) = kind {
                if let Some(action) = self.start_create(kind, page) {
                    actions.push(action);
                }
                return true;
            }
        }
        let rows = &page.rows;
        let selected = self
            .selected
            .as_ref()
            .and_then(|path| rows.iter().position(|row| &row.path == path));
        if matches!(key, Key::X | Key::C) && matches_shortcut(Modifiers::COMMAND) {
            if let Some(index) = selected {
                let mode = if key == Key::X {
                    crate::explorer_clipboard::Mode::Cut
                } else {
                    crate::explorer_clipboard::Mode::Copy
                };
                self.capture_clipboard(&rows[index], mode);
            }
            return true;
        }
        if key == Key::V && matches_shortcut(Modifiers::COMMAND) {
            if let Some(action) =
                self.paste_request(selected.map(|index| &rows[index]), page, suffix)
            {
                actions.push(action);
            }
            return true;
        }
        if key == Key::C && matches_shortcut(Modifiers::COMMAND | Modifiers::ALT | Modifiers::SHIFT)
        {
            if let Some(index) = selected
                && let Some(path) = self.relative_path(&rows[index].path)
            {
                actions.push(Action::CopyText(path));
            }
            return true;
        }
        if key == Key::C && matches_shortcut(Modifiers::COMMAND | Modifiers::ALT) {
            if let Some(index) = selected {
                actions.push(Action::CopyText(rows[index].path.clone()));
            }
            return true;
        }
        if key == Key::R && matches_shortcut(Modifiers::COMMAND | Modifiers::ALT) {
            if let Some(index) = selected {
                actions.push(Action::RevealPath(rows[index].path.clone()));
            }
            return true;
        }
        if key == Key::Backspace && matches_shortcut(Modifiers::COMMAND) {
            if let Some(index) = selected {
                actions.push(Action::RequestDelete(rows[index].clone()));
            }
            return true;
        }
        if (key == Key::Space && matches_shortcut(Modifiers::NONE))
            || (key == Key::ArrowDown && matches_shortcut(Modifiers::COMMAND))
        {
            if let Some(index) = selected
                && rows[index].kind == TreeEntryKind::File
            {
                actions.push(Action::Open {
                    path: rows[index].path.clone(),
                    preview: false,
                });
            }
            return true;
        }
        if matches!(key, Key::F2 | Key::Enter) && matches_shortcut(Modifiers::NONE) {
            if let Some(index) = selected {
                self.start_rename(&rows[index]);
            }
            return true;
        }
        if key == Key::ArrowDown {
            self.select_index(rows, selected.map_or(Some(0), |index| index.checked_add(1)));
            return true;
        }
        if key == Key::ArrowUp {
            let index = match selected {
                Some(index) => index.checked_sub(1),
                None => rows.len().checked_sub(1),
            };
            self.select_index(rows, index);
            return true;
        }
        let Some(index) = selected else {
            return false;
        };
        let row = &rows[index];
        if key == Key::ArrowRight {
            if row.kind != TreeEntryKind::Directory {
                return true;
            }
            if !row.expanded {
                actions.push(Action::Toggle(row.path.clone()));
            } else if let Some(child) = rows.get(index + 1)
                && child.depth == row.depth + 1
            {
                self.select_index(rows, Some(index + 1));
            }
            return true;
        }
        if key == Key::ArrowLeft {
            if row.kind == TreeEntryKind::Directory && row.expanded {
                actions.push(Action::Toggle(row.path.clone()));
            } else {
                let parent = rows[..index]
                    .iter()
                    .rposition(|parent| parent.depth < row.depth);
                self.select_index(rows, parent);
            }
            return true;
        }
        false
    }

    fn typeahead(&mut self, rows: &[TreeRow], text: &str, time: f64) -> bool {
        let Some(selected) = self
            .selected
            .as_ref()
            .and_then(|path| rows.iter().position(|row| &row.path == path))
        else {
            return false;
        };
        self.typeahead_buffer.push_str(text);
        self.typeahead_deadline = Some(time + TYPEAHEAD_RESET_SECONDS);
        let prefix = self.typeahead_buffer.to_lowercase();
        let matched = (selected..rows.len())
            .chain(0..selected)
            .find(|index| rows[*index].name.to_lowercase().starts_with(&prefix));
        self.select_index(rows, matched);
        true
    }
}

pub async fn create_entry(
    services: &std::sync::Arc<taide_runtime::AppServices>,
    project: &ProjectId,
    request: &CreateRequest,
) -> AppResult<CreatedEntry> {
    let _operation = services
        .tasks
        .begin_operation("native-explorer-create")
        .ok_or_else(|| AppError::Forbidden("native explorer creation is stopping".into()))?;
    let guard = services.state.begin_owned_mutation().await;
    let state = services.state.clone();
    let project_for_check = project.clone();
    let entry = request.clone();
    services
        .tasks
        .run_blocking_result("native-explorer-create-file", move || {
            let _guard = guard;
            if state.is_shutting_down() {
                return Err(AppError::Forbidden(
                    "native explorer creation is stopping".into(),
                ));
            }
            let projects = state.projects.read();
            let root = taide_infra::root_guard::project_root(&projects, &project_for_check)?;
            if !Path::new(&entry.path).is_absolute() || !Path::new(&entry.parent).is_absolute() {
                return Err(AppError::InvalidArgument(
                    "native explorer creation path is invalid".into(),
                ));
            }
            let canonical =
                taide_infra::root_guard::ensure_within_root(&root, Path::new(&entry.path))?;
            taide_infra::root_guard::ensure_within_root(&root, Path::new(&entry.parent))?;
            taide_file::service::create_entry(&canonical, entry.kind == TreeEntryKind::Directory)?;
            state.self_writes.mark(&canonical);
            Ok(())
        })
        .await?;
    taide_runtime::tree_actions::tree_refresh(
        &services.state,
        &services.tree,
        &services.tasks,
        project.clone(),
        request.parent.clone(),
    )
    .await?;
    let page = taide_runtime::tree_actions::tree_reveal(
        &services.state,
        &services.tree,
        &services.tasks,
        project.clone(),
        request.path.clone(),
    )
    .await?;
    let layout = if request.kind == TreeEntryKind::File {
        let title = Path::new(&request.path)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                AppError::InvalidArgument("native explorer creation filename is invalid".into())
            })?;
        Some(
            taide_runtime::layout_actions::layout_open_tab(
                services.events.as_ref(),
                &services.state,
                project.clone(),
                taide_model::layout::TabKind::File {
                    path: request.path.clone(),
                },
                title.into(),
                None,
                false,
            )
            .await?,
        )
    } else {
        None
    };
    Ok(CreatedEntry { page, layout })
}

pub async fn rename_entry(
    services: &std::sync::Arc<taide_runtime::AppServices>,
    project: &ProjectId,
    request: &RenameRequest,
    replies: &tokio::sync::mpsc::Sender<crate::lsp::Reply>,
    repaint: &std::sync::Arc<dyn Fn() + Send + Sync>,
) -> AppResult<TreeRowPage> {
    move_entry(services, project, request, replies, repaint).await?;
    let parent = Path::new(&request.from)
        .parent()
        .and_then(Path::to_str)
        .ok_or_else(|| {
            AppError::InvalidArgument("native explorer rename parent is unavailable".into())
        })?;
    taide_runtime::tree_actions::tree_refresh(
        &services.state,
        &services.tree,
        &services.tasks,
        project.clone(),
        parent.into(),
    )
    .await?;
    taide_runtime::tree_actions::tree_reveal(
        &services.state,
        &services.tree,
        &services.tasks,
        project.clone(),
        request.to.clone(),
    )
    .await
}

pub async fn move_entry(
    services: &std::sync::Arc<taide_runtime::AppServices>,
    project: &ProjectId,
    request: &RenameRequest,
    replies: &tokio::sync::mpsc::Sender<crate::lsp::Reply>,
    repaint: &std::sync::Arc<dyn Fn() + Send + Sync>,
) -> AppResult<()> {
    crate::explorer_move::move_selected(services, project, request, replies, repaint).await
}

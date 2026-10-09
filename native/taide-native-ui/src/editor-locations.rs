use std::collections::HashSet;
use std::sync::Arc;

use egui::{
    Align2, Color32, Event, FontFamily, FontId, Id, Key, PointerButton, Rect, Sense, Stroke,
    StrokeKind, Ui, UiBuilder, pos2, vec2,
};
use taide_native_editor::document::{DocumentId, EditorError};
use taide_native_editor::store::EditorStore;
use taide_native_editor::symbol_locations::{Command, Locations, Target};
use taide_native_editor::view::ViewId;

pub const DEFAULT_LINES: f32 = 18.0;
const DEFAULT_RATIO: f32 = 0.7;
const MINIMUM_RATIO: f32 = 0.2;
const MAXIMUM_RATIO: f32 = 0.8;
const ROW_HEIGHT: f32 = 23.0;
const HEADER_LINES: f32 = 1.2;
const BORDER_WIDTH: f32 = 1.0;
const BUTTON_SIZE: f32 = 22.0;
const MARGIN: f32 = 8.0;
const INDENT: f32 = 16.0;
const CONTEXT_UNITS: u32 = 8;
const SASH_WIDTH: f32 = 4.0;
const MINIMUM_LINES: f32 = 5.0;
const ICON_SIZE: f32 = 16.0;
const ARROW_RATIO: f32 = 1.0 / 3.0;
const CLOSE_ICON: char = '\u{ea76}';
const TREE_EVENT_FILTER: egui::EventFilter = egui::EventFilter {
    tab: false,
    horizontal_arrows: true,
    vertical_arrows: true,
    escape: true,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Body,
    Tree,
    Preview,
}

#[derive(Clone)]
pub struct Widget {
    pub token: String,
    pub position: usize,
    pub title: String,
    pub model: Arc<Locations>,
    pub selected: Option<usize>,
    pub focus: Option<(u64, Focus)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colors {
    pub border: Color32,
    pub background: Color32,
    pub heading_background: Color32,
    pub heading: Color32,
    pub detail: Color32,
    pub tree_background: Color32,
    pub selection_background: Color32,
    pub selection_foreground: Color32,
    pub highlight: Color32,
    pub highlight_border: Color32,
    pub link: Color32,
}

pub trait Provider {
    fn definition_available(&self, _store: &EditorStore, _view: ViewId) -> bool {
        false
    }
    fn source_word(
        &self,
        _store: &EditorStore,
        _view: ViewId,
        _byte: usize,
    ) -> Option<std::ops::Range<usize>> {
        None
    }
    fn request_at(
        &mut self,
        _store: &mut EditorStore,
        _view: ViewId,
        _byte: usize,
        _mode: taide_native_editor::symbol_locations::Mode,
    ) -> Result<bool, EditorError> {
        Ok(false)
    }
    fn link_preview(
        &mut self,
        _store: &EditorStore,
        _view: ViewId,
        _byte: usize,
    ) -> Option<LinkPreview> {
        None
    }
    fn clear_link(&mut self, _view: ViewId) {}
    fn keyboard_link(&self, _store: &EditorStore, _view: ViewId) -> Option<(String, usize)> {
        None
    }
    fn preserve_focus(&mut self, _view: ViewId, _focus: Focus) {}
    fn clear_preview_chord(&mut self, _view: ViewId) {}
    fn preview_find_visible(&self, _view: ViewId) -> bool {
        false
    }
    fn current(&mut self, store: &EditorStore, view: ViewId) -> Option<Widget>;
    fn execute(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
        command: Command,
    ) -> Result<bool, EditorError>;
    fn render_preview(
        &mut self,
        ui: &mut Ui,
        store: &mut EditorStore,
        view: ViewId,
        rect: Rect,
        focus: bool,
    ) -> Result<Vec<Id>, EditorError>;
    fn preview_document(&self, store: &EditorStore, target: &Target) -> Option<DocumentId>;
    fn file_label(&self, target: &Target) -> String;
    fn file_description(&self, _target: &Target) -> String {
        String::new()
    }
    fn word_start(&self, store: &EditorStore, document: DocumentId, byte: usize) -> usize;
}

pub struct LinkPreview {
    pub bytes: std::ops::Range<usize>,
    pub text: String,
    pub code: Option<egui::text::LayoutJob>,
}

#[derive(Clone, PartialEq, Eq)]
enum Action {
    Command(Command),
    Group(usize),
}

#[derive(Clone)]
struct Hit {
    id: Id,
    rect: Rect,
    action: Action,
}

#[derive(Clone)]
struct Scene {
    document: DocumentId,
    revision: u64,
    token: String,
    rect: Rect,
    scroll: egui::Vec2,
    word_wrap: bool,
    folds: Vec<std::ops::Range<usize>>,
    tree: Id,
    preview: Vec<Id>,
    hits: Vec<Hit>,
}

#[derive(Clone)]
pub(crate) struct State {
    scene: Option<Scene>,
    pressed: Option<Id>,
    collapsed: HashSet<usize>,
    ratio: f32,
    lines: f32,
    scroll: f32,
    chord: bool,
    focus_preview: bool,
    selected: Option<usize>,
    applied_focus: Option<(u64, Focus)>,
    tree_focus: Option<Action>,
    painted_focus: Option<Action>,
}

#[derive(Default)]
pub(crate) struct Input {
    pub(crate) consumed: HashSet<usize>,
    pub(crate) release_at: Option<usize>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            scene: None,
            pressed: None,
            collapsed: HashSet::new(),
            ratio: DEFAULT_RATIO,
            lines: DEFAULT_LINES,
            scroll: 0.0,
            chord: false,
            focus_preview: false,
            selected: None,
            applied_focus: None,
            tree_focus: None,
            painted_focus: None,
        }
    }
}

pub(crate) fn shortcut(event: &Event, composing: bool, visible: bool) -> Option<Command> {
    if composing || !visible {
        return None;
    }
    let Event::Key {
        key,
        pressed: true,
        modifiers,
        ..
    } = event
    else {
        return None;
    };
    if *key == Key::Escape && modifiers.is_none() {
        return Some(Command::Close);
    }
    if matches!(key, Key::F4 | Key::F12)
        && !modifiers.alt
        && !modifiers.command
        && !modifiers.ctrl
        && !modifiers.mac_cmd
    {
        return Some(if modifiers.shift {
            Command::Previous
        } else {
            Command::Next
        });
    }
    None
}

fn owns_event(context: &egui::Context, id: Id, index: usize) -> bool {
    let before = context.keyboard_focus_before_events() == Some(id);
    context.keyboard_input_route(id).map_or(before, |route| {
        route
            .0
            .get(index)
            .and_then(|entry| entry.0)
            .unwrap_or(before)
    })
}

impl State {
    pub(crate) fn height(&self, line_height: f32, viewport_height: f32) -> f32 {
        (self.lines * line_height).min(viewport_height.max(line_height * MINIMUM_LINES))
    }

    pub(crate) fn focus_ids(&self) -> Vec<Id> {
        self.scene.as_ref().map_or_else(Vec::new, |scene| {
            std::iter::once(scene.tree)
                .chain(scene.preview.iter().copied())
                .chain(scene.hits.iter().map(|hit| hit.id))
                .collect()
        })
    }

    pub(crate) fn clear(&mut self) {
        self.scene = None;
        self.pressed = None;
        self.collapsed.clear();
        self.scroll = 0.0;
        self.chord = false;
        self.applied_focus = None;
        self.tree_focus = None;
        self.painted_focus = None;
    }

    pub(crate) fn input(
        &mut self,
        ui: &mut Ui,
        store: &mut EditorStore,
        view: ViewId,
        body: Id,
        rect: Rect,
        word_wrap: bool,
        widget: Option<&Widget>,
        provider: &mut Option<&mut dyn Provider>,
    ) -> Result<Input, EditorError> {
        let Some(widget) = widget else {
            self.clear();
            return Ok(Input::default());
        };
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        let document = store.documents().snapshot(current.document)?;
        let valid = self.scene.as_ref().is_some_and(|scene| {
            scene.token == widget.token
                && scene.document == document.id
                && scene.revision == document.revision
                && scene.rect == rect
                && scene.scroll == vec2(current.scroll.x, current.scroll.y)
                && scene.word_wrap == word_wrap
                && scene.folds == current.folds
        });
        if !valid {
            self.pressed = None;
        }
        if !ui.is_enabled() {
            self.pressed = None;
            self.chord = false;
            return Ok(Input::default());
        }
        let ids = self.focus_ids();
        let raw = ui.input(|input| input.raw.events.clone());
        let mut next = 0;
        let mut tree_owned = false;
        let events = ui.input(|input| input.events.clone());
        let mut input = Input::default();
        let mut released = false;
        for event in events {
            let index = egui::Context::raw_event_index(&raw, &event, &mut next);
            let owns =
                !released && (tree_owned || ids.iter().any(|id| owns_event(ui.ctx(), *id, index)));
            let body_owns = released || owns_event(ui.ctx(), body, index);
            let preview_owned = !tree_owned
                && self.scene.as_ref().is_some_and(|scene| {
                    scene
                        .preview
                        .iter()
                        .any(|id| owns_event(ui.ctx(), *id, index))
                });
            let widget = provider
                .as_deref_mut()
                .and_then(|provider| provider.current(store, view))
                .unwrap_or_else(|| widget.clone());
            let composing = store
                .views()
                .get(view)
                .is_some_and(|view| view.composition.is_some());
            let close_key = owns
                && self.scene.as_ref().is_some_and(|scene| {
                    scene.hits.iter().any(|hit| {
                        matches!(hit.action, Action::Command(Command::Close))
                            && owns_event(ui.ctx(), hit.id, index)
                    })
                })
                && matches!(&event, Event::Key { key: Key::Enter | Key::Space, pressed: true, modifiers, .. } if modifiers.is_none());
            let find_escape = preview_owned
                && provider
                    .as_deref()
                    .is_some_and(|provider| provider.preview_find_visible(view))
                && matches!(
                    &event,
                    Event::Key {
                        key: Key::Escape,
                        pressed: true,
                        ..
                    }
                );
            if !find_escape
                && (owns || body_owns)
                && let Some(command) = shortcut(&event, composing, true)
                    .or_else(|| close_key.then_some(Command::Close))
            {
                if let Some(provider) = provider.as_deref_mut() {
                    if matches!(command, Command::Next | Command::Previous) {
                        provider.preserve_focus(
                            view,
                            if preview_owned {
                                Focus::Preview
                            } else if body_owns {
                                Focus::Body
                            } else {
                                Focus::Tree
                            },
                        );
                    }
                    provider.execute(store, view, command)?;
                    if matches!(command, Command::Next | Command::Previous)
                        && let Some(current) = provider.current(store, view)
                    {
                        self.tree_focus = current.selected.map(|index| {
                            Action::Command(Command::Select {
                                index,
                                focus_preview: false,
                            })
                        });
                        if let Some(selected) = current.selected
                            && let Some(group) = current
                                .model
                                .groups()
                                .iter()
                                .position(|group| group.references.contains(&selected))
                        {
                            self.collapsed.remove(&group);
                        }
                    }
                }
                if command == Command::Close {
                    ui.memory_mut(|memory| memory.request_focus(body));
                    self.clear();
                    input.release_at = Some(index);
                    released = true;
                    tree_owned = false;
                }
                input.consumed.insert(index);
                continue;
            }
            if owns
                && !composing
                && let Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } = &event
            {
                if self.chord && *key == Key::F2 && modifiers.is_none() {
                    self.chord = false;
                    if let Some(provider) = provider.as_deref_mut() {
                        provider.clear_preview_chord(view);
                    }
                    let tree = self.scene.as_ref().map(|scene| scene.tree);
                    let in_tree =
                        tree_owned || tree.is_some_and(|id| owns_event(ui.ctx(), id, index));
                    if in_tree {
                        self.focus_preview = true;
                    } else if let Some(tree) = tree {
                        ui.memory_mut(|memory| {
                            memory.request_focus_with_filter(tree, TREE_EVENT_FILTER)
                        });
                        tree_owned = true;
                    }
                    input.consumed.insert(index);
                    continue;
                }
                self.chord =
                    *key == Key::K && modifiers.command && !modifiers.alt && !modifiers.shift;
                if self.chord && !preview_owned {
                    input.consumed.insert(index);
                    continue;
                }
                let in_tree = tree_owned
                    || self
                        .scene
                        .as_ref()
                        .is_some_and(|scene| owns_event(ui.ctx(), scene.tree, index));
                if in_tree {
                    if self.tree_focus.is_none() {
                        self.tree_focus = widget.selected.map(|index| {
                            Action::Command(Command::Select {
                                index,
                                focus_preview: false,
                            })
                        });
                    }
                    if modifiers.is_none()
                        && matches!(key, Key::ArrowDown | Key::ArrowUp | Key::Home | Key::End)
                    {
                        let mut rows = Vec::new();
                        for (group_index, group) in widget.model.groups().iter().enumerate() {
                            if widget.model.groups().len() > 1 {
                                rows.push(Action::Group(group_index));
                                if self.collapsed.contains(&group_index) {
                                    continue;
                                }
                            }
                            rows.extend(group.references.clone().map(|index| {
                                Action::Command(Command::Select {
                                    index,
                                    focus_preview: false,
                                })
                            }));
                        }
                        let current = rows
                            .iter()
                            .position(|row| Some(row) == self.tree_focus.as_ref())
                            .unwrap_or(0);
                        let next = match key {
                            Key::ArrowUp => current.saturating_sub(1),
                            Key::ArrowDown => (current + 1).min(rows.len().saturating_sub(1)),
                            Key::Home => 0,
                            _ => rows.len().saturating_sub(1),
                        };
                        self.tree_focus = rows.get(next).cloned();
                        if let Some(Action::Command(command)) = self.tree_focus
                            && let Some(provider) = provider.as_deref_mut()
                        {
                            provider.execute(store, view, command)?;
                        }
                        input.consumed.insert(index);
                        continue;
                    }
                    let command = match key {
                        Key::Enter
                            if modifiers.is_none()
                                && matches!(self.tree_focus, Some(Action::Group(_))) =>
                        {
                            if let Some(Action::Group(group)) = self.tree_focus
                                && !self.collapsed.remove(&group)
                            {
                                self.collapsed.insert(group);
                            }
                            None
                        }
                        Key::Enter if !modifiers.alt && !modifiers.shift => {
                            Some(if modifiers.command || modifiers.ctrl {
                                Command::OpenSelected { side: true }
                            } else {
                                Command::GotoSelected
                            })
                        }
                        Key::ArrowRight | Key::ArrowLeft if modifiers.is_none() => {
                            match self.tree_focus {
                                Some(Action::Group(group)) if *key == Key::ArrowLeft => {
                                    self.collapsed.insert(group);
                                }
                                Some(Action::Group(group)) => {
                                    if !self.collapsed.remove(&group)
                                        && let Some(group) = widget.model.groups().get(group)
                                    {
                                        let command = Command::Select {
                                            index: group.references.start,
                                            focus_preview: false,
                                        };
                                        self.tree_focus = Some(Action::Command(command));
                                        if let Some(provider) = provider.as_deref_mut() {
                                            provider.execute(store, view, command)?;
                                        }
                                    }
                                }
                                Some(Action::Command(Command::Select { index, .. }))
                                    if *key == Key::ArrowLeft
                                        && widget.model.groups().len() > 1 =>
                                {
                                    self.tree_focus = widget
                                        .model
                                        .groups()
                                        .iter()
                                        .position(|group| group.references.contains(&index))
                                        .map(Action::Group);
                                }
                                _ => {}
                            }
                            None
                        }
                        _ => None,
                    };
                    if let Some(command) = command
                        && let Some(provider) = provider.as_deref_mut()
                    {
                        provider.execute(store, view, command)?;
                    }
                    input.consumed.insert(index);
                    continue;
                }
            }
            if valid
                && let Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers,
                    ..
                } = &event
            {
                let hit = self
                    .scene
                    .as_ref()
                    .and_then(|scene| {
                        scene.hits.iter().find(|hit| {
                            hit.rect.contains(*pos)
                                && ui.ctx().pointer_focus_preserving_trigger_at(index)
                                    == Some(hit.id)
                        })
                    })
                    .cloned();
                if *pressed {
                    self.pressed = hit.as_ref().map(|hit| hit.id);
                    if hit.is_some() {
                        tree_owned = true;
                    }
                } else if let Some(hit) = hit
                    && self.pressed.take() == Some(hit.id)
                {
                    match hit.action {
                        Action::Group(index) => {
                            self.tree_focus = Some(Action::Group(index));
                            if !self.collapsed.remove(&index) {
                                self.collapsed.insert(index);
                            }
                        }
                        Action::Command(command) => {
                            if matches!(command, Command::Select { .. }) {
                                self.tree_focus = Some(Action::Command(command));
                            }
                            if let Some(provider) = provider.as_deref_mut() {
                                let command = if matches!(command, Command::OpenSelected { .. }) {
                                    Command::OpenSelected {
                                        side: modifiers.command || modifiers.ctrl || modifiers.alt,
                                    }
                                } else {
                                    command
                                };
                                provider.execute(store, view, command)?;
                                if matches!(command, Command::Select { .. })
                                    && (modifiers.command
                                        || modifiers.ctrl
                                        || modifiers.alt
                                        || ui.input(|input| {
                                            input
                                                .pointer
                                                .button_double_clicked(PointerButton::Primary)
                                        }))
                                {
                                    provider.execute(
                                        store,
                                        view,
                                        if modifiers.command || modifiers.ctrl || modifiers.alt {
                                            Command::OpenSelected { side: true }
                                        } else {
                                            Command::GotoSelected
                                        },
                                    )?;
                                }
                            }
                            if command == Command::Close {
                                ui.memory_mut(|memory| memory.request_focus(body));
                                self.clear();
                                input.release_at = Some(index);
                                released = true;
                            }
                        }
                    }
                    tree_owned = !released;
                    if let Some(scene) = &self.scene {
                        ui.memory_mut(|memory| {
                            memory.request_focus_with_filter(scene.tree, TREE_EVENT_FILTER)
                        });
                    }
                    input.consumed.insert(index);
                    continue;
                }
            }
            if !released
                && owns
                && !preview_owned
                && matches!(event, Event::Text(_) | Event::Paste(_) | Event::Ime(_))
            {
                input.consumed.insert(index);
                continue;
            }
        }
        Ok(input)
    }

    pub(crate) fn paint(
        &mut self,
        ui: &mut Ui,
        store: &mut EditorStore,
        view: ViewId,
        provider: &mut dyn Provider,
        frame: Frame<'_>,
    ) -> Result<(), EditorError> {
        let fresh = self
            .scene
            .as_ref()
            .is_none_or(|scene| scene.token != frame.widget.token);
        if fresh {
            self.collapsed.clear();
            self.scroll = 0.0;
            self.focus_preview = false;
            self.tree_focus = None;
        }
        let mut ui = ui.new_child(
            UiBuilder::new()
                .id_salt(frame.id.with("location-widget"))
                .max_rect(frame.rect),
        );
        ui.set_clip_rect(frame.rect.intersect(frame.clip));
        let colors = frame.colors;
        ui.painter().rect_filled(frame.rect, 0.0, colors.background);
        ui.painter().rect_stroke(
            frame.rect,
            0.0,
            Stroke::new(BORDER_WIDTH, colors.border),
            StrokeKind::Inside,
        );
        let arrow = frame.line_height * ARROW_RATIO;
        let anchor = frame
            .anchor_x
            .clamp(frame.rect.left() + arrow, frame.rect.right() - arrow);
        ui.painter()
            .with_clip_rect(frame.clip)
            .add(egui::epaint::Shape::convex_polygon(
                vec![
                    pos2(anchor, frame.rect.top() - arrow),
                    pos2(anchor - arrow, frame.rect.top()),
                    pos2(anchor + arrow, frame.rect.top()),
                ],
                colors.border,
                Stroke::NONE,
            ));
        let header = Rect::from_min_max(
            frame.rect.min + vec2(BORDER_WIDTH, BORDER_WIDTH),
            pos2(
                frame.rect.right() - BORDER_WIDTH,
                frame.rect.top() + frame.line_height * HEADER_LINES,
            ),
        );
        ui.painter()
            .rect_filled(header, 0.0, colors.heading_background);
        let close = Rect::from_center_size(
            pos2(header.right() - BUTTON_SIZE / 2.0, header.center().y),
            vec2(BUTTON_SIZE, BUTTON_SIZE),
        );
        let close_id = frame.id.with("location-close");
        let response = ui.interact(close, close_id, Sense::click());
        ui.ctx()
            .register_pointer_preserves_keyboard_focus(response.id);
        let mut hits = vec![Hit {
            id: close_id,
            rect: close.intersect(ui.clip_rect()),
            action: Action::Command(Command::Close),
        }];
        let icon_family = FontFamily::Name(crate::editor_find_widget::ICON_FAMILY.into());
        let icon_font = if ui.fonts(|fonts| fonts.definitions().families.contains_key(&icon_family))
        {
            FontId::new(ICON_SIZE, icon_family)
        } else {
            FontId::proportional(ICON_SIZE)
        };
        ui.painter().text(
            close.center(),
            Align2::CENTER_CENTER,
            CLOSE_ICON,
            icon_font,
            colors.heading,
        );
        let target = frame
            .widget
            .selected
            .and_then(|index| frame.widget.model.targets().get(index));
        let title = target
            .map(|target| provider.file_label(target))
            .unwrap_or_default();
        let title_rect = Rect::from_min_max(header.min, pos2(close.left(), header.bottom()))
            .intersect(ui.clip_rect());
        if target.is_some() {
            let title_id = frame.id.with("location-title");
            let title_response = ui.interact(title_rect, title_id, Sense::click());
            ui.ctx()
                .register_pointer_preserves_keyboard_focus(title_response.id);
            hits.push(Hit {
                id: title_id,
                rect: title_rect,
                action: Action::Command(Command::OpenSelected { side: false }),
            });
        }
        let description = target
            .map(|target| provider.file_description(target))
            .unwrap_or_default();
        let mut job = egui::text::LayoutJob::default();
        job.append(
            &title,
            0.0,
            egui::TextFormat {
                font_id: FontId::proportional(frame.font.size),
                color: colors.heading,
                ..Default::default()
            },
        );
        job.append(
            &format!(
                "  {description}    {} ({})",
                frame.widget.title,
                frame.widget.model.targets().len()
            ),
            0.0,
            egui::TextFormat {
                font_id: FontId::proportional(frame.font.size),
                color: colors.detail,
                ..Default::default()
            },
        );
        let galley = ui.painter().layout_job(job);
        ui.painter().with_clip_rect(title_rect).galley(
            pos2(
                header.left() + MARGIN,
                header.center().y - galley.size().y / 2.0,
            ),
            galley,
            colors.heading,
        );
        let body = Rect::from_min_max(
            pos2(frame.rect.left() + BORDER_WIDTH, header.bottom()),
            frame.rect.max - vec2(BORDER_WIDTH, BORDER_WIDTH),
        );
        if frame.widget.model.targets().is_empty() {
            let tree = frame.id.with("location-tree");
            if fresh {
                ui.memory_mut(|memory| memory.request_focus(frame.id));
            }
            ui.painter().text(
                body.center(),
                Align2::CENTER_CENTER,
                "No results",
                frame.font.clone(),
                colors.detail,
            );
            let current = store.views().get(view).ok_or(EditorError::NotFound)?;
            let document = store.documents().snapshot(current.document)?;
            self.scene = Some(Scene {
                document: document.id,
                revision: document.revision,
                token: frame.widget.token.clone(),
                rect: frame.owner_rect,
                scroll: vec2(current.scroll.x, current.scroll.y),
                word_wrap: frame.word_wrap,
                folds: current.folds.clone(),
                tree,
                preview: Vec::new(),
                hits,
            });
            return Ok(());
        }
        let sash_x = body.left() + body.width() * self.ratio;
        let sash = Rect::from_min_max(
            pos2(sash_x - SASH_WIDTH / 2.0, body.top()),
            pos2(sash_x + SASH_WIDTH / 2.0, body.bottom()),
        );
        let split = ui.interact(sash, frame.id.with("location-sash"), Sense::drag());
        if split.dragged()
            && let Some(pointer) = split.interact_pointer_pos()
        {
            self.ratio =
                ((pointer.x - body.left()) / body.width()).clamp(MINIMUM_RATIO, MAXIMUM_RATIO);
            ui.ctx().request_repaint();
        }
        let preview_rect = Rect::from_min_max(body.min, pos2(sash_x - BORDER_WIDTH, body.bottom()));
        let tree_rect = Rect::from_min_max(pos2(sash_x + BORDER_WIDTH, body.top()), body.max);
        ui.painter()
            .rect_filled(tree_rect, 0.0, colors.tree_background);
        ui.painter().line_segment(
            [pos2(sash_x, body.top()), pos2(sash_x, body.bottom())],
            Stroke::new(BORDER_WIDTH, colors.border),
        );
        let tree = frame.id.with("location-tree");
        ui.interact(tree_rect, tree, Sense::focusable_noninteractive());
        if fresh {
            ui.memory_mut(|memory| memory.request_focus_with_filter(tree, TREE_EVENT_FILTER));
        }
        if fresh || self.applied_focus != frame.widget.focus {
            if let Some((_, focus)) = frame.widget.focus {
                match focus {
                    Focus::Body => ui.memory_mut(|memory| memory.request_focus(frame.id)),
                    Focus::Tree => ui.memory_mut(|memory| {
                        memory.request_focus_with_filter(tree, TREE_EVENT_FILTER)
                    }),
                    Focus::Preview => self.focus_preview = true,
                }
            }
            self.applied_focus = frame.widget.focus;
        }
        if ui.memory(|memory| memory.has_focus(tree)) {
            ui.memory_mut(|memory| memory.set_focus_lock_filter(tree, TREE_EVENT_FILTER));
        }
        if fresh || self.selected != frame.widget.selected {
            self.tree_focus = frame.widget.selected.map(|index| {
                Action::Command(Command::Select {
                    index,
                    focus_preview: false,
                })
            });
            if let Some(selected) = frame.widget.selected
                && let Some(group) = frame
                    .widget
                    .model
                    .groups()
                    .iter()
                    .position(|group| group.references.contains(&selected))
            {
                self.collapsed.remove(&group);
            }
        }
        let mut rows = Vec::new();
        for (group_index, group) in frame.widget.model.groups().iter().enumerate() {
            if frame.widget.model.groups().len() > 1 {
                let name =
                    provider.file_label(&frame.widget.model.targets()[group.references.start]);
                rows.push((
                    Some(format!("{name} ({})", group.references.len())),
                    Action::Group(group_index),
                    self.tree_focus == Some(Action::Group(group_index)),
                    0.0,
                ));
                if self.collapsed.contains(&group_index) {
                    continue;
                }
            }
            for index in group.references.clone() {
                rows.push((
                    None,
                    Action::Command(Command::Select {
                        index,
                        focus_preview: false,
                    }),
                    !matches!(self.tree_focus, Some(Action::Group(_)))
                        && frame.widget.selected == Some(index),
                    if frame.widget.model.groups().len() > 1 {
                        INDENT
                    } else {
                        0.0
                    },
                ));
            }
        }
        if fresh || self.selected != frame.widget.selected || self.painted_focus != self.tree_focus
        {
            if let Some(index) = rows.iter().position(|(_, _, selected, _)| *selected) {
                let top = index as f32 * ROW_HEIGHT;
                if top < self.scroll {
                    self.scroll = top;
                }
                if top + ROW_HEIGHT > self.scroll + tree_rect.height() {
                    self.scroll = (top + ROW_HEIGHT - tree_rect.height()).max(0.0);
                }
            }
            self.selected = frame.widget.selected;
            self.painted_focus = self.tree_focus.clone();
        }
        if ui.input(|input| {
            input
                .pointer
                .hover_pos()
                .is_some_and(|pointer| tree_rect.contains(pointer))
        }) {
            self.scroll = (self.scroll - ui.input(|input| input.smooth_scroll_delta.y)).clamp(
                0.0,
                (rows.len() as f32 * ROW_HEIGHT - tree_rect.height()).max(0.0),
            );
        }
        if rows.is_empty() {
            ui.painter().text(
                tree_rect.center(),
                Align2::CENTER_CENTER,
                "No results",
                frame.font.clone(),
                colors.detail,
            );
        }
        for (index, (text, action, selected, indent)) in rows.into_iter().enumerate() {
            let row = Rect::from_min_size(
                pos2(
                    tree_rect.left(),
                    tree_rect.top() + index as f32 * ROW_HEIGHT - self.scroll,
                ),
                vec2(tree_rect.width(), ROW_HEIGHT),
            );
            if !row.intersects(tree_rect.intersect(ui.clip_rect())) {
                continue;
            }
            let (text, highlight) = if let Some(text) = text {
                (text, None)
            } else if let Action::Command(Command::Select { index, .. }) = &action {
                let target = &frame.widget.model.targets()[*index];
                let preview = provider
                    .preview_document(store, target)
                    .and_then(|document| {
                        let snapshot = store.documents().snapshot(document).ok()?;
                        taide_native_editor::symbol_locations::preview(
                            &snapshot,
                            target.selection,
                            CONTEXT_UNITS,
                            |byte| provider.word_start(store, document, byte),
                        )
                        .ok()
                    });
                preview.map_or_else(
                    || {
                        (
                            format!(
                                "{}:{}",
                                u64::from(target.selection.start.line) + 1,
                                u64::from(target.selection.start.character) + 1
                            ),
                            None,
                        )
                    },
                    |preview| (preview.text, Some(preview.highlight)),
                )
            } else {
                (String::new(), None)
            };
            let id = frame.id.with(("location-row", index));
            let response = ui.interact(row.intersect(tree_rect), id, Sense::click());
            ui.ctx().register_pointer_preserves_keyboard_focus(id);
            hits.push(Hit {
                id,
                rect: row.intersect(tree_rect).intersect(ui.clip_rect()),
                action,
            });
            let painter = ui
                .painter()
                .with_clip_rect(row.intersect(tree_rect).intersect(ui.clip_rect()));
            if selected {
                painter.rect_filled(row, 0.0, colors.selection_background);
            }
            let format = egui::TextFormat {
                font_id: frame.font.clone(),
                color: if selected {
                    colors.selection_foreground
                } else {
                    colors.heading
                },
                ..Default::default()
            };
            let mut job = egui::text::LayoutJob::default();
            if let Some(highlight) = highlight.filter(|range| {
                range.start <= range.end
                    && text.is_char_boundary(range.start)
                    && text.is_char_boundary(range.end)
            }) {
                job.append(&text[..highlight.start], 0.0, format.clone());
                job.append(
                    &text[highlight.clone()],
                    0.0,
                    egui::TextFormat {
                        background: colors.highlight,
                        ..format.clone()
                    },
                );
                job.append(&text[highlight.end..], 0.0, format);
            } else {
                job.append(&text, 0.0, format);
            }
            let galley = painter.layout_job(job);
            painter.galley(
                pos2(
                    row.left() + MARGIN + indent,
                    row.center().y - galley.size().y / 2.0,
                ),
                galley,
                colors.heading,
            );
            response.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::SelectableLabel,
                    ui.is_enabled(),
                    "Reference",
                )
            });
        }
        let preview = provider.render_preview(
            &mut ui,
            store,
            view,
            preview_rect,
            std::mem::take(&mut self.focus_preview),
        )?;
        let resize = ui.interact(
            Rect::from_min_max(
                pos2(frame.rect.left(), frame.rect.bottom() - SASH_WIDTH),
                frame.rect.max,
            ),
            frame.id.with("location-height"),
            Sense::drag(),
        );
        if resize.dragged() {
            self.lines =
                (self.lines + resize.drag_delta().y / frame.line_height).max(MINIMUM_LINES);
            ui.ctx().request_repaint();
        }
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        let document = store.documents().snapshot(current.document)?;
        self.scene = Some(Scene {
            document: document.id,
            revision: document.revision,
            token: frame.widget.token.clone(),
            rect: frame.owner_rect,
            scroll: vec2(current.scroll.x, current.scroll.y),
            word_wrap: frame.word_wrap,
            folds: current.folds.clone(),
            tree,
            preview,
            hits,
        });
        Ok(())
    }
}

pub(crate) struct Frame<'a> {
    pub(crate) id: Id,
    pub(crate) widget: &'a Widget,
    pub(crate) rect: Rect,
    pub(crate) owner_rect: Rect,
    pub(crate) anchor_x: f32,
    pub(crate) word_wrap: bool,
    pub(crate) clip: Rect,
    pub(crate) line_height: f32,
    pub(crate) font: &'a FontId,
    pub(crate) colors: Colors,
}

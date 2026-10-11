use std::sync::Arc;

use egui::{Color32, Event, EventFilter, FontId, Id, Key, Rect, Sense, Stroke, Ui, pos2, vec2};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::ViewId;

use crate::editor_geometry::EditorGeometry;

const ROW_HEIGHT: f32 = 24.0;
const TEXT_SIZE: f32 = 13.0;
const HEADER_SIZE: f32 = 11.0;
const ICON_SIZE: f32 = 16.0;
const ICON_GAP: f32 = 6.0;
const ROW_PADDING: f32 = 6.0;
const MIN_WIDTH: f32 = 100.0;
const MAX_WIDTH_RATIO: f32 = 0.8;
const PADDING: i8 = 4;
const CORNER_RADIUS: u8 = 8;
const ROW_RADIUS: u8 = 4;
const BORDER_WIDTH: f32 = 1.0;
const SHADOW_OFFSET: [i8; 2] = [0, 4];
const SHADOW_BLUR: u8 = 8;
const MESSAGE_PADDING_X: i8 = 4;
const MESSAGE_PADDING_Y: i8 = 2;
const MESSAGE_RADIUS: u8 = 3;
const MESSAGE_ANCHOR_SIZE: f32 = 8.0;
const MESSAGE_LEFT_OFFSET: f32 = 6.0;
const MESSAGE_MOUSE_LINES: usize = 3;
const MENU_FILTER: EventFilter = EventFilter {
    tab: true,
    horizontal_arrows: true,
    vertical_arrows: true,
    escape: true,
};

pub(crate) fn bulb_rect(anchor: Rect, gutter_left: Option<f32>) -> Rect {
    Rect::from_min_size(
        pos2(
            gutter_left.unwrap_or(anchor.left()),
            anchor.center().y - ICON_SIZE / 2.0,
        ),
        vec2(ICON_SIZE, ICON_SIZE),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    QuickFix,
    Refactor,
    Source,
    OrganizeImports,
    FixAll,
    AutoFix,
}

impl Command {
    pub fn unavailable_message(self) -> &'static str {
        match self {
            Self::QuickFix => "No code actions available",
            Self::Refactor => "No refactorings available",
            Self::Source => "No source actions available",
            Self::OrganizeImports => "No organize imports action available",
            Self::FixAll => "No fix all action available",
            Self::AutoFix => "No auto fixes available",
        }
    }

    pub fn from_action(action: &str) -> Option<Self> {
        Some(match action.strip_prefix("monaco.").unwrap_or(action) {
            "editor.action.quickFix" => Self::QuickFix,
            "editor.action.refactor" => Self::Refactor,
            "editor.action.sourceAction" => Self::Source,
            "editor.action.organizeImports" => Self::OrganizeImports,
            "editor.action.fixAll" => Self::FixAll,
            "editor.action.autoFix" => Self::AutoFix,
            _ => return None,
        })
    }

    pub fn kind(self) -> Option<&'static str> {
        match self {
            Self::QuickFix => None,
            Self::Refactor => Some("refactor"),
            Self::Source => Some("source"),
            Self::OrganizeImports => Some("source.organizeImports"),
            Self::FixAll => Some("source.fixAll"),
            Self::AutoFix => Some("quickfix"),
        }
    }

    pub fn auto_apply_single(self) -> bool {
        matches!(self, Self::OrganizeImports | Self::FixAll | Self::AutoFix)
    }

    pub fn accepts(self, action: &Action) -> bool {
        if let Some(kind) = self.kind() {
            if !action
                .kind
                .as_deref()
                .is_some_and(|actual| kind_contains(kind, actual))
            {
                return false;
            }
        } else if action
            .kind
            .as_deref()
            .is_some_and(|kind| kind_contains("source", kind))
        {
            return false;
        }
        self != Self::AutoFix || action.preferred
    }

    pub fn intersects(self, kind: &str) -> bool {
        self.kind().map_or_else(
            || !kind_contains("source", kind),
            |requested| kind_contains(requested, kind) || kind_contains(kind, requested),
        )
    }
}

pub fn kind_contains(parent: &str, child: &str) -> bool {
    parent.is_empty()
        || parent == child
        || child
            .strip_prefix(parent)
            .is_some_and(|suffix| suffix.starts_with('.'))
}

#[derive(Clone, Debug)]
pub struct Action {
    pub title: String,
    pub kind: Option<String>,
    pub preferred: bool,
    pub disabled: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    QuickFix,
    Extract,
    Inline,
    Rewrite,
    Move,
    Surround,
    Source,
    Other,
}

impl Group {
    pub const ALL: [Self; 8] = [
        Self::QuickFix,
        Self::Extract,
        Self::Inline,
        Self::Rewrite,
        Self::Move,
        Self::Surround,
        Self::Source,
        Self::Other,
    ];

    pub fn of(action: &Action) -> Self {
        let Some(kind) = action.kind.as_deref() else {
            return Self::Other;
        };
        for (prefix, group) in [
            ("quickfix", Self::QuickFix),
            ("refactor.extract", Self::Extract),
            ("refactor.inline", Self::Inline),
            ("refactor.rewrite", Self::Rewrite),
            ("refactor.move", Self::Move),
            ("refactor.surround", Self::Surround),
            ("source", Self::Source),
        ] {
            if kind_contains(prefix, kind) {
                return group;
            }
        }
        Self::Other
    }

    fn title(self) -> &'static str {
        match self {
            Self::QuickFix => "Quick Fix",
            Self::Extract => "Extract",
            Self::Inline => "Inline",
            Self::Rewrite => "Rewrite",
            Self::Move => "Move",
            Self::Surround => "Surround With",
            Self::Source => "Source Action",
            Self::Other => "More Actions...",
        }
    }

    fn glyph(self) -> Option<char> {
        match self {
            Self::Extract | Self::Inline | Self::Rewrite | Self::Move => Some('\u{eb65}'),
            Self::Surround => Some('\u{ec24}'),
            Self::Source => Some('\u{eb60}'),
            Self::QuickFix | Self::Other => None,
        }
    }
}

#[derive(Clone)]
pub struct Labels {
    pub groups: [(Group, String); 8],
    pub show_disabled: String,
    pub hide_disabled: String,
    pub lightbulb: String,
}

impl Default for Labels {
    fn default() -> Self {
        Self {
            groups: Group::ALL.map(|group| (group, group.title().to_owned())),
            show_disabled: "Show Disabled".into(),
            hide_disabled: "Hide Disabled".into(),
            lightbulb: "Show Code Actions".into(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Bulb {
    pub anchor: usize,
    pub gutter: bool,
}

#[derive(Clone)]
pub struct Session {
    pub token: u128,
    pub anchor: usize,
    pub menu_open: bool,
    pub include_disabled: bool,
    pub show_headers: bool,
    pub actions: Arc<[Action]>,
    pub bulb: Option<Bulb>,
    pub labels: Labels,
}

#[derive(Clone)]
pub struct Message {
    pub token: u128,
    pub anchor: usize,
    pub text: String,
}

pub trait Provider {
    fn message(&self, _store: &EditorStore, _view: ViewId) -> Option<Message> {
        None
    }
    fn current(&self, store: &EditorStore, view: ViewId) -> Option<Session>;
    fn open(&mut self, store: &EditorStore, view: ViewId, token: u128);
    fn select(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        token: u128,
        index: usize,
        preview: bool,
    );
    fn close(&mut self, view: ViewId, token: u128);
}

#[derive(Clone, Copy)]
pub struct Colors {
    pub background: Color32,
    pub foreground: Color32,
    pub disabled: Color32,
    pub selected_background: Color32,
    pub selected_foreground: Color32,
    pub hover_background: Color32,
    pub border: Color32,
    pub shadow: Color32,
    pub lightbulb: Color32,
    pub lightbulb_auto_fix: Color32,
    pub message_background: Color32,
    pub message_foreground: Color32,
    pub message_border: Color32,
}

#[derive(Clone, Default)]
pub(crate) struct State {
    token: Option<u128>,
    selected: Option<usize>,
    show_disabled: bool,
    composing: bool,
    reveal: bool,
    message_token: Option<u128>,
}

#[derive(Default)]
pub(crate) struct Output {
    pub focus_ids: Vec<Id>,
    pub released: Vec<(usize, Event)>,
}

pub fn focus_id(editor: Id, token: u128) -> Id {
    editor.with(("code-action-menu", token))
}
pub fn action_id(editor: Id, token: u128, index: usize) -> Id {
    focus_id(editor, token).with(("action", index))
}
pub fn bulb_id(editor: Id, token: u128) -> Id {
    editor.with(("code-action-lightbulb", token))
}

pub fn message_id(editor: Id, token: u128) -> Id {
    editor.with(("code-action-message", token))
}

pub fn disabled_toggle_id(editor: Id, token: u128) -> Id {
    focus_id(editor, token).with("disabled-toggle")
}

#[derive(Clone, Copy)]
enum Row {
    Header(Group),
    Action(usize),
}

impl State {
    pub(crate) fn reveal_message(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
        message: Option<&Message>,
    ) -> Result<(), taide_native_editor::document::EditorError> {
        let Some(message) = message else {
            self.message_token = None;
            return Ok(());
        };
        if self.message_token != Some(message.token) {
            store.request_selection_reveal(view, message.anchor..message.anchor, true)?;
            self.message_token = Some(message.token);
        }
        Ok(())
    }

    fn show_message(
        &mut self,
        ui: &mut Ui,
        store: &EditorStore,
        view: ViewId,
        editor: Id,
        geometry: &EditorGeometry,
        font: &FontId,
        colors: Colors,
        provider: &mut dyn Provider,
        message: Message,
    ) {
        let Some(anchor) = geometry.caret_rect(message.anchor) else {
            return;
        };
        let id = message_id(editor, message.token);
        let pointer = ui.input(|input| input.pointer.hover_pos());
        let hovered = ui.ctx().read_response(id).is_some_and(|response| {
            pointer.is_some_and(|position| response.rect.contains(position))
        });
        if !ui.is_enabled()
            || !ui.input(|input| input.raw.focused)
            || (!ui.memory(|memory| memory.has_focus(editor)) && !hovered)
            || ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::Escape))
        {
            provider.close(view, message.token);
            return;
        }
        let screen = ui.ctx().content_rect();
        let width_limit = screen.width() * MAX_WIDTH_RATIO;
        let galley = ui.fonts_mut(|fonts| {
            fonts.layout(
                message.text.clone(),
                font.clone(),
                colors.message_foreground,
                (width_limit - f32::from(MESSAGE_PADDING_X) * 2.0).max(0.0),
            )
        });
        let width = galley.size().x + f32::from(MESSAGE_PADDING_X) * 2.0;
        let height = galley.size().y + f32::from(MESSAGE_PADDING_Y) * 2.0;
        let above = anchor.top() - height - MESSAGE_ANCHOR_SIZE >= screen.top();
        let top = if above {
            anchor.top() - height - MESSAGE_ANCHOR_SIZE
        } else {
            anchor.bottom() + MESSAGE_ANCHOR_SIZE
        };
        let left = (anchor.left() - MESSAGE_LEFT_OFFSET)
            .min((screen.right() - width).max(screen.left()))
            .max(screen.left());
        let response = egui::Area::new(id.with("area"))
            .order(egui::Order::Tooltip)
            .fixed_pos(pos2(left, top))
            .interactable(false)
            .show(ui.ctx(), |ui| {
                let response = egui::Frame::new()
                    .fill(colors.message_background)
                    .stroke(Stroke::new(BORDER_WIDTH, colors.message_border))
                    .corner_radius(MESSAGE_RADIUS)
                    .inner_margin(egui::Margin::symmetric(
                        MESSAGE_PADDING_X,
                        MESSAGE_PADDING_Y,
                    ))
                    .show(ui, |ui| {
                        ui.set_max_width(galley.size().x);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&message.text)
                                    .font(font.clone())
                                    .color(colors.message_foreground),
                            )
                            .wrap(),
                        );
                    });
                ui.interact(response.response.rect, id, Sense::hover())
            })
            .inner;
        let rect = response.rect;
        let x = anchor.left().clamp(rect.left(), rect.right());
        let (base, tip) = if above {
            (rect.bottom(), rect.bottom() + MESSAGE_ANCHOR_SIZE)
        } else {
            (rect.top(), rect.top() - MESSAGE_ANCHOR_SIZE)
        };
        ui.ctx()
            .layer_painter(egui::LayerId::new(egui::Order::Tooltip, id.with("area")))
            .add(egui::Shape::convex_polygon(
                vec![
                    pos2(x - MESSAGE_ANCHOR_SIZE, base),
                    pos2(x + MESSAGE_ANCHOR_SIZE, base),
                    pos2(x, tip),
                ],
                colors.message_border,
                Stroke::NONE,
            ));
        if !hovered
            && ui.input(|input| {
                input
                    .events
                    .iter()
                    .any(|event| matches!(event, Event::PointerMoved(_)))
            })
            && let Some(position) =
                pointer.filter(|position| geometry.content_rect.contains(*position))
            && let Some(row) = geometry.rows.iter().find(|row| {
                row.origin.y <= position.y && position.y < row.origin.y + geometry.line_height
            })
            && let Some(snapshot) = store
                .views()
                .get(view)
                .and_then(|view| store.documents().snapshot(view.document).ok())
            && snapshot
                .rope
                .byte_to_line(row.segment.bytes.start)
                .abs_diff(snapshot.rope.byte_to_line(message.anchor))
                > MESSAGE_MOUSE_LINES
        {
            provider.close(view, message.token);
        }
    }

    fn reset_menu(&mut self, ui: &Ui, editor: Id) {
        if self
            .token
            .is_some_and(|token| ui.memory(|memory| memory.has_focus(focus_id(editor, token))))
        {
            ui.memory_mut(|memory| memory.request_focus_with_filter(editor, MENU_FILTER));
        }
        self.token = None;
        self.selected = None;
        self.composing = false;
        self.reveal = false;
    }

    fn rows(&self, session: &Session) -> Vec<Row> {
        let has_valid = session
            .actions
            .iter()
            .any(|action| action.disabled.is_none());
        let include_disabled = session.include_disabled && (self.show_disabled || !has_valid);
        let indices = session
            .actions
            .iter()
            .enumerate()
            .filter(|(_, action)| include_disabled || action.disabled.is_none())
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if !session.show_headers {
            return indices.into_iter().map(Row::Action).collect();
        }
        Group::ALL
            .into_iter()
            .flat_map(|group| {
                let matching = indices
                    .iter()
                    .copied()
                    .filter(|index| Group::of(&session.actions[*index]) == group)
                    .collect::<Vec<_>>();
                if matching.is_empty() {
                    return Vec::new();
                }
                std::iter::once(Row::Header(group))
                    .chain(matching.into_iter().map(Row::Action))
                    .collect()
            })
            .collect()
    }

    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        store: &EditorStore,
        view: ViewId,
        editor: Id,
        geometry: &EditorGeometry,
        font: &FontId,
        colors: Colors,
        provider: &mut dyn Provider,
    ) -> Output {
        if let Some(message) = provider.message(store, view) {
            self.reset_menu(ui, editor);
            self.show_message(
                ui, store, view, editor, geometry, font, colors, provider, message,
            );
            return Output::default();
        }
        let Some(session) = provider.current(store, view) else {
            self.reset_menu(ui, editor);
            return Output::default();
        };
        if !ui.is_enabled() || !ui.input(|input| input.raw.focused) {
            provider.close(view, session.token);
            self.reset_menu(ui, editor);
            return Output::default();
        }
        if !session.menu_open {
            self.reset_menu(ui, editor);
            if ui.memory(|memory| memory.has_focus(editor))
                && session
                    .actions
                    .iter()
                    .any(|action| action.disabled.is_none())
                && let Some(bulb) = session.bulb
                && let Some(anchor) = geometry.caret_rect(bulb.anchor)
            {
                let rect = bulb_rect(anchor, bulb.gutter.then_some(geometry.gutter_rect.left()));
                let preferred = session.actions.iter().any(|action| {
                    action.disabled.is_none()
                        && action.preferred
                        && Group::of(action) == Group::QuickFix
                });
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    if preferred { '\u{eb13}' } else { '\u{ea61}' },
                    FontId::new(ICON_SIZE, crate::font_families::codicons(ui)),
                    if preferred {
                        colors.lightbulb_auto_fix
                    } else {
                        colors.lightbulb
                    },
                );
                let response = ui
                    .interact(rect, bulb_id(editor, session.token), Sense::CLICK)
                    .on_hover_text(&session.labels.lightbulb);
                ui.ctx()
                    .register_pointer_preserves_keyboard_focus(response.id);
                if response.clicked() {
                    provider.open(store, view, session.token);
                    ui.ctx().request_repaint();
                }
            }
            return Output::default();
        }
        let fresh = self.token != Some(session.token);
        let id = focus_id(editor, session.token);
        if !fresh && !ui.memory(|memory| memory.has_focus(id)) {
            provider.close(view, session.token);
            self.reset_menu(ui, editor);
            return Output::default();
        }
        let rows = self.rows(&session);
        let enabled = rows
            .iter()
            .filter_map(|row| match row {
                Row::Action(index) if session.actions[*index].disabled.is_none() => Some(*index),
                _ => None,
            })
            .collect::<Vec<_>>();
        if fresh {
            self.token = Some(session.token);
            self.selected = enabled.first().copied();
            self.composing = false;
            self.reveal = true;
            ui.memory_mut(|memory| memory.request_focus_with_filter(id, MENU_FILTER));
        }
        if rows.is_empty() {
            provider.close(view, session.token);
            self.reset_menu(ui, editor);
            return Output::default();
        }
        let mut selection = None;
        let mut close = false;
        let mut released = Vec::new();
        let pointer_moved = ui.input(|input| {
            input
                .raw
                .events
                .iter()
                .any(|event| matches!(event, Event::PointerMoved(_)))
        });
        for (index, event) in ui
            .input(|input| input.raw.events.clone())
            .into_iter()
            .enumerate()
        {
            if !is_keyboard(&event) {
                continue;
            }
            if selection.is_some() || close {
                released.push((index, event));
                continue;
            }
            match &event {
                Event::Ime(egui::ImeEvent::Preedit { text, .. }) => {
                    self.composing = !text.is_empty()
                }
                Event::Ime(egui::ImeEvent::Commit(_)) => self.composing = false,
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } if !modifiers.alt => {
                    let is_mac = ui.ctx().os().is_mac();
                    let previous =
                        *key == Key::ArrowUp || (is_mac && modifiers.ctrl && *key == Key::P);
                    let next =
                        *key == Key::ArrowDown || (is_mac && modifiers.ctrl && *key == Key::N);
                    if (previous || next) && !modifiers.shift && !enabled.is_empty() {
                        let position = self
                            .selected
                            .and_then(|selected| {
                                enabled.iter().position(|index| *index == selected)
                            })
                            .unwrap_or(0);
                        let target = if previous {
                            (position + enabled.len() - 1) % enabled.len()
                        } else {
                            (position + 1) % enabled.len()
                        };
                        self.selected = Some(enabled[target]);
                        self.reveal = true;
                    } else if *key == Key::Escape
                        && !modifiers.ctrl
                        && !modifiers.command
                        && !modifiers.mac_cmd
                    {
                        close = true;
                    } else if !self.composing
                        && !modifiers.shift
                        && (*key == Key::Enter
                            || (*key == Key::Space
                                && !modifiers.ctrl
                                && !modifiers.command
                                && !modifiers.mac_cmd)
                            || (*key == Key::Period && modifiers.command))
                    {
                        if let Some(selected) =
                            self.selected.filter(|selected| enabled.contains(selected))
                        {
                            selection = Some((
                                selected,
                                *key == Key::Enter && (modifiers.command || modifiers.mac_cmd),
                            ));
                        }
                    }
                }
                _ => (),
            }
        }
        ui.input_mut(|input| input.events.retain(|event| !is_keyboard(event)));
        let Some(anchor) = geometry.caret_rect(session.anchor) else {
            provider.close(view, session.token);
            self.reset_menu(ui, editor);
            return Output {
                focus_ids: Vec::new(),
                released,
            };
        };
        let screen = ui.ctx().content_rect();
        let font = FontId::new(TEXT_SIZE, crate::font_families::medium(ui));
        let width = session
            .actions
            .iter()
            .map(|action| {
                ui.fonts_mut(|fonts| {
                    fonts
                        .layout_no_wrap(action.title.clone(), font.clone(), colors.foreground)
                        .size()
                        .x
                })
            })
            .fold(MIN_WIDTH, f32::max)
            + ICON_SIZE
            + ICON_GAP
            + ROW_PADDING * 2.0;
        let width = width.min(screen.width() * MAX_WIDTH_RATIO);
        let can_toggle = session.include_disabled
            && !enabled.is_empty()
            && session
                .actions
                .iter()
                .any(|action| action.disabled.is_some());
        let height =
            (rows.len() + usize::from(can_toggle)) as f32 * ROW_HEIGHT + f32::from(PADDING) * 2.0;
        let below = (screen.bottom() - anchor.bottom()).max(0.0);
        let above = (anchor.top() - screen.top()).max(0.0);
        let available = if height <= below || below >= above {
            below
        } else {
            above
        };
        let top = if height <= below || below >= above {
            anchor.bottom()
        } else {
            anchor.top() - height.min(above)
        };
        let left = anchor
            .left()
            .min((screen.right() - width).max(screen.left()))
            .max(screen.left());
        let mut menu_rect = Rect::NOTHING;
        egui::Area::new(editor.with("code-action-box"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos2(left, top))
            .movable(false)
            .show(ui.ctx(), |ui| {
                egui::Frame::NONE
                    .fill(colors.background)
                    .stroke(Stroke::new(BORDER_WIDTH, colors.border))
                    .corner_radius(CORNER_RADIUS)
                    .inner_margin(PADDING)
                    .shadow(egui::epaint::Shadow {
                        offset: SHADOW_OFFSET,
                        blur: SHADOW_BLUR,
                        spread: 0,
                        color: colors.shadow,
                    })
                    .show(ui, |ui| {
                        ui.set_width((width - f32::from(PADDING) * 2.0).max(MIN_WIDTH));
                        ui.spacing_mut().item_spacing.y = 0.0;
                        egui::ScrollArea::vertical()
                            .id_salt(("code-action-scroll", session.token))
                            .max_height(
                                (available
                                    - f32::from(PADDING) * 2.0
                                    - if can_toggle { ROW_HEIGHT } else { 0.0 })
                                .max(ROW_HEIGHT),
                            )
                            .show(ui, |ui| {
                                for row in &rows {
                                    let (_, rect) =
                                        ui.allocate_space(vec2(ui.available_width(), ROW_HEIGHT));
                                    match *row {
                                        Row::Header(group) => {
                                            let label = session
                                                .labels
                                                .groups
                                                .iter()
                                                .find(|(current, _)| *current == group)
                                                .map_or(group.title(), |(_, label)| label.as_str());
                                            ui.painter().text(
                                                rect.left_center() + vec2(ROW_PADDING, 0.0),
                                                egui::Align2::LEFT_CENTER,
                                                label,
                                                FontId::new(
                                                    HEADER_SIZE,
                                                    crate::font_families::semibold(ui),
                                                ),
                                                colors.foreground,
                                            );
                                        }
                                        Row::Action(index) => {
                                            let action = &session.actions[index];
                                            let response = ui.interact(
                                                rect,
                                                action_id(editor, session.token, index),
                                                Sense::CLICK,
                                            );
                                            ui.ctx().register_pointer_preserves_keyboard_focus(
                                                response.id,
                                            );
                                            if pointer_moved
                                                && response.hovered()
                                                && action.disabled.is_none()
                                            {
                                                self.selected = Some(index);
                                            }
                                            let selected = self.selected == Some(index)
                                                && action.disabled.is_none();
                                            if selected || response.hovered() {
                                                ui.painter().rect_filled(
                                                    rect,
                                                    ROW_RADIUS,
                                                    if selected {
                                                        colors.selected_background
                                                    } else {
                                                        colors.hover_background
                                                    },
                                                );
                                            }
                                            let foreground = if action.disabled.is_some() {
                                                colors.disabled
                                            } else if selected {
                                                colors.selected_foreground
                                            } else {
                                                colors.foreground
                                            };
                                            if let Some(glyph) = Group::of(action).glyph() {
                                                ui.painter().text(
                                                    rect.left_center()
                                                        + vec2(ROW_PADDING + ICON_SIZE / 2.0, 0.0),
                                                    egui::Align2::CENTER_CENTER,
                                                    glyph,
                                                    FontId::new(
                                                        ICON_SIZE,
                                                        crate::font_families::codicons(ui),
                                                    ),
                                                    foreground,
                                                );
                                            }
                                            ui.painter().with_clip_rect(rect).text(
                                                rect.left_center()
                                                    + vec2(ROW_PADDING + ICON_SIZE + ICON_GAP, 0.0),
                                                egui::Align2::LEFT_CENTER,
                                                &action.title,
                                                font.clone(),
                                                foreground,
                                            );
                                            if self.reveal && selected {
                                                response.scroll_to_me(Some(egui::Align::Center));
                                            }
                                            let clicked = response.clicked();
                                            if let Some(reason) = &action.disabled {
                                                response.on_hover_text(reason);
                                            }
                                            if clicked && action.disabled.is_none() {
                                                selection = Some((index, false));
                                            }
                                        }
                                    }
                                }
                            });
                        if can_toggle {
                            let label = if self.show_disabled {
                                &session.labels.hide_disabled
                            } else {
                                &session.labels.show_disabled
                            };
                            let (_, rect) =
                                ui.allocate_space(vec2(ui.available_width(), ROW_HEIGHT));
                            let response = ui.interact(
                                rect,
                                disabled_toggle_id(editor, session.token),
                                Sense::CLICK,
                            );
                            ui.ctx()
                                .register_pointer_preserves_keyboard_focus(response.id);
                            if response.hovered() {
                                ui.painter()
                                    .rect_filled(rect, ROW_RADIUS, colors.hover_background);
                            }
                            ui.painter().text(
                                rect.left_center() + vec2(ROW_PADDING, 0.0),
                                egui::Align2::LEFT_CENTER,
                                label,
                                font.clone(),
                                colors.foreground,
                            );
                            if response.clicked() {
                                self.show_disabled = !self.show_disabled;
                                ui.memory_mut(|memory| {
                                    memory.request_focus_with_filter(id, MENU_FILTER)
                                });
                                ui.ctx().request_repaint();
                            }
                        }
                        ui.interact(ui.min_rect(), id, Sense::focusable_noninteractive());
                        menu_rect = ui.min_rect().expand(f32::from(PADDING));
                        if fresh {
                            ui.memory_mut(|memory| {
                                memory.request_focus_with_filter(id, MENU_FILTER)
                            });
                        }
                    });
            });
        self.reveal = false;
        if !fresh
            && ui.input(|input| {
                input.pointer.any_pressed()
                    && input
                        .pointer
                        .interact_pos()
                        .is_some_and(|position| !menu_rect.contains(position))
            })
        {
            close = true;
        }
        if let Some((index, preview)) = selection {
            provider.select(store, view, session.token, index, preview);
        }
        if close {
            provider.close(view, session.token);
        }
        if selection.is_some() || close {
            self.reset_menu(ui, editor);
            ui.memory_mut(|memory| memory.request_focus_with_filter(editor, MENU_FILTER));
            return Output {
                focus_ids: Vec::new(),
                released,
            };
        }
        Output {
            focus_ids: vec![id],
            released,
        }
    }
}

fn is_keyboard(event: &Event) -> bool {
    matches!(
        event,
        Event::Key { .. }
            | Event::Text(_)
            | Event::Paste(_)
            | Event::Copy
            | Event::Cut
            | Event::Ime(_)
    )
}

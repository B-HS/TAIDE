use std::{collections::HashMap, time::Duration};

use super::Instant;
use super::egui::{self, Color32, Event, Id, Key, RichText, Stroke};
use taide_model::{error::AppResult, locale::ResolvedLocale, theme::ResolvedTheme};

use crate::keybinding_search::{Search, label};
use crate::keymap::{
    Context,
    catalog::{
        self, ConflictIndex, Overrides, Row,
        capture::{Capture, Effect, Target},
    },
};
use crate::modal::{self, Chrome, Composition, FocusReturn, Layer};
use crate::presentation::{color, message};

use crate::ui_icons::{Icon, Icons};

const CONTEXT_POLL: Duration = Duration::from_millis(500);
const MAX_WIDTH: f32 = 768.0;
const SCREEN_MARGIN: f32 = 16.0;
const HEIGHT_RATIO: f32 = 0.7;
const PADDING: i8 = 24;
const SECTION_GAP: f32 = 16.0;
const ROW_GAP: f32 = 4.0;
const ROW_PADDING_X: i8 = 12;
const ROW_PADDING_Y: i8 = 6;
const SMALL_FONT: f32 = 12.0;
const DETAIL_FONT: f32 = 10.0;
const TITLE_FONT: f32 = 18.0;
const ROW_RADIUS: u8 = 2;
const BORDER_WIDTH: f32 = 1.0;
const ROW_CONTROL_GAP: f32 = 12.0;
const ICON_BUTTON_SIZE: f32 = 24.0;
const CLOSE_INSET: f32 = 16.0;
const CLOSE_OPACITY: f32 = 0.7;
const CONTROL_GAP: f32 = 4.0;
const BINDING_GAP: f32 = 6.0;
const LABEL_GAP: f32 = 2.0;
const BUTTON_PADDING: f32 = 8.0;
const LINE_HEIGHT_RATIO: f32 = 1.5;
const BUTTON_RADIUS: u8 = 6;
const CAPTURE_HEIGHT: f32 = 28.0;
const BADGE_PADDING_X: i8 = 4;
const BADGE_PADDING_Y: i8 = 2;
const BADGE_OPACITY: f32 = 0.15;
const CLOSE_FOCUS_RING: f32 = 2.0;
const CLOSE_FOCUS_OFFSET: f32 = 2.0;
const BODY_FONT: f32 = 13.0;
const SEARCH_GAP: f32 = 8.0;
const PILL_RADIUS: u8 = u8::MAX;
const ACTIVE_WARNING_BORDER_OPACITY: f32 = 0.4;

fn reveal_focused_control(response: &egui::Response) {
    if response.has_focus() && !response.interact_rect.contains_rect(response.rect) {
        response.scroll_to_me_animation(None, egui::style::ScrollAnimation::none());
    }
}

fn register_focus_control(order: &mut Vec<Id>, response: &egui::Response) {
    if response.enabled() && response.sense.is_focusable() {
        order.push(response.id);
    }
}

fn can_unbind(row: &Row) -> bool {
    !row.binding.key().is_empty()
        || row
            .default_binding_label
            .as_ref()
            .is_some_and(|label| !label.is_empty())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum RowControlKind {
    ResolveConflict,
    Capture,
    Confirm,
    Change,
    Reset,
    Unbind,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct RowControl {
    row: String,
    kind: RowControlKind,
}

impl RowControl {
    fn new(row: &Row, kind: RowControlKind) -> Self {
        Self {
            row: row.id.clone(),
            kind,
        }
    }

    fn scope(&self, context: &egui::Context) -> egui::UiBuilder {
        egui::UiBuilder::new().id(Id::new((
            "native-keybinding-control",
            context.viewport_id(),
            self,
        )))
    }
}

#[derive(Clone, PartialEq, Eq)]
enum TabTarget {
    Widget(Id),
    Row(RowControl),
}

pub struct Appearance {
    background: Color32,
    modal_border: Color32,
    foreground: Color32,
    muted: Color32,
    border: Color32,
    active: Color32,
    accent: Color32,
    warning: Color32,
    focus: Color32,
    input: Color32,
    input_border: Color32,
    shadow: Color32,
    button_background: Color32,
    button_hover: Color32,
    button_hover_foreground: Color32,
    primary: Color32,
    primary_foreground: Color32,
}

impl Appearance {
    pub fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        Ok(Self {
            background: color(theme, "modal.background")?,
            modal_border: color(theme, "modal.border")?,
            foreground: color(theme, "app.foreground")?,
            muted: color(theme, "appSidebar.iconDefault")?,
            border: color(theme, "app.border")?,
            active: color(theme, "appSidebar.itemActive")?,
            accent: color(theme, "app.accent")?,
            warning: color(theme, "statusIndicator.warning")?,
            focus: color(theme, "app.focusBorder")?,
            input: color(theme, "panel.inputBackground")?,
            input_border: color(theme, "panel.inputBorder")?,
            shadow: color(theme, "app.shadow")?,
            button_background: color(theme, "app.background")?,
            button_hover: color(theme, "list.hoverBackground")?,
            button_hover_foreground: color(theme, "list.foreground")?,
            primary: color(theme, "button.primaryBackground")?,
            primary_foreground: color(theme, "button.primaryForeground")?,
        })
    }

    fn buttons(&self, ui: &mut egui::Ui, outline: bool) {
        ui.visuals_mut().override_text_color = None;
        ui.spacing_mut().interact_size.y = ICON_BUTTON_SIZE;
        let padding = BUTTON_PADDING + if outline { BORDER_WIDTH } else { 0.0 };
        ui.spacing_mut().button_padding = egui::vec2(padding, 0.0);
        let widgets = &mut ui.visuals_mut().widgets;
        for (visuals, fill, foreground) in [
            (
                &mut widgets.inactive,
                if outline {
                    self.button_background
                } else {
                    Color32::TRANSPARENT
                },
                self.foreground,
            ),
            (
                &mut widgets.hovered,
                self.button_hover,
                self.button_hover_foreground,
            ),
            (
                &mut widgets.active,
                self.button_hover,
                self.button_hover_foreground,
            ),
        ] {
            visuals.weak_bg_fill = fill;
            visuals.bg_fill = fill;
            visuals.bg_stroke = if outline {
                Stroke::new(BORDER_WIDTH, self.border)
            } else {
                Stroke::NONE
            };
            visuals.fg_stroke.color = foreground;
            visuals.corner_radius = BUTTON_RADIUS.into();
            visuals.expansion = 0.0;
        }
    }
}

#[derive(Default)]
pub struct Output {
    pub saves: Vec<Overrides>,
    pub warnings: Vec<String>,
    pub started_capture: bool,
    tooltip_triggers: Vec<(&'static str, egui::Response)>,
}

impl Output {
    pub fn show_tooltips(
        &self,
        locale: &ResolvedLocale,
        provider: &crate::tooltips::Provider,
        appearance: &crate::tooltips::Appearance,
    ) {
        for (key, response) in &self.tooltip_triggers {
            provider.show(
                response,
                &message(locale, key, &[]),
                egui::RectAlign::BOTTOM,
                appearance,
            );
        }
    }
}

pub struct Editor {
    open: bool,
    mounted: bool,
    query: String,
    capture: Capture,
    conflicts_only: bool,
    unassigned_only: bool,
    context_keys: Vec<&'static str>,
    polled_at: Option<Instant>,
    previous_focus: FocusReturn,
    capture_focus: Option<Id>,
    search_focus: Option<Id>,
    close_focus: Option<Id>,
    focus_order: Vec<Id>,
    focus_rows: HashMap<Id, RowControl>,
    focus_target: Option<RowControl>,
    rows_scroll: Option<(Id, Vec<String>)>,
    focus_next: bool,
    search_focus_next: bool,
    raw_overrides: Option<Option<String>>,
    overrides: Overrides,
    rows: Vec<Row>,
    search: Search,
    appearance: Appearance,
    icons: Icons,
    composition: Composition,
    is_mac: bool,
}

#[cfg(feature = "inspection")]
pub struct KeybindingInspection {
    pub open: bool,
    pub capturing: bool,
    pub query: String,
    pub focused: Option<String>,
    pub targets: std::collections::BTreeMap<String, egui::Rect>,
    pub hit_targets: Vec<String>,
}

impl Editor {
    #[cfg(feature = "inspection")]
    pub fn inspection(&self, context: &egui::Context) -> KeybindingInspection {
        let focused = context.memory(|memory| memory.focused());
        let controls = [
            ("search".to_owned(), self.search_focus),
            ("close".to_owned(), self.close_focus),
            ("capture".to_owned(), self.capture_focus),
        ]
        .into_iter()
        .filter_map(|(label, id)| id.map(|id| (label, id)))
        .chain(
            self.focus_rows
                .iter()
                .map(|(id, control)| (format!("row:{}:{:?}", control.row, control.kind), *id)),
        );
        let mut inspection = KeybindingInspection {
            open: self.open,
            capturing: self.is_capturing(),
            query: self.query.clone(),
            focused: None,
            targets: std::collections::BTreeMap::new(),
            hit_targets: Vec::new(),
        };
        if !self.open {
            return inspection;
        }
        for (label, id) in controls {
            let Some(response) = context.read_response(id) else {
                continue;
            };
            if !response.enabled() || !response.interact_rect.is_positive() {
                continue;
            }
            if focused == Some(id) {
                inspection.focused = Some(label.clone());
            }
            if response.contains_pointer() {
                inspection.hit_targets.push(label.clone());
            }
            inspection.targets.insert(label, response.interact_rect);
        }
        inspection
    }

    fn prepare_row_focus(&self, ui: &egui::Ui, control: &RowControl) {
        if self.focus_target.as_ref() == Some(control) {
            ui.ctx()
                .memory_mut(|memory| memory.request_focus(ui.next_auto_id()));
        }
    }

    fn register_row_control(&mut self, row: &Row, kind: RowControlKind, response: &egui::Response) {
        register_focus_control(&mut self.focus_order, response);
        if response.enabled() && response.sense.is_focusable() {
            self.focus_rows
                .insert(response.id, RowControl::new(row, kind));
        }
    }

    fn tab_targets(&self, locale: &ResolvedLocale) -> Vec<TabTarget> {
        let mut targets = self
            .focus_order
            .iter()
            .filter(|id| !self.focus_rows.contains_key(id) && Some(**id) != self.close_focus)
            .map(|id| TabTarget::Widget(*id))
            .collect::<Vec<_>>();
        let conflicts = ConflictIndex::new(&self.rows, self.is_mac);
        for index in self.visible_rows(locale, &conflicts) {
            let row = &self.rows[index];
            let mut kinds = Vec::new();
            if conflicts.find(row).is_some() {
                kinds.push(RowControlKind::ResolveConflict);
            }
            if let Some(Target::Row { id, first }) = &self.capture.target
                && *id == row.id
            {
                kinds.push(RowControlKind::Capture);
                if first.is_some() {
                    kinds.push(RowControlKind::Confirm);
                }
            }
            kinds.push(RowControlKind::Change);
            if row.is_overridden {
                kinds.push(RowControlKind::Reset);
            }
            if can_unbind(row) {
                kinds.push(RowControlKind::Unbind);
            }
            targets.extend(
                kinds
                    .into_iter()
                    .map(|kind| TabTarget::Row(RowControl::new(row, kind))),
            );
        }
        if let Some(close) = self.close_focus {
            targets.push(TabTarget::Widget(close));
        }
        targets
    }

    fn filter_button(
        &self,
        ui: &mut egui::Ui,
        text: String,
        selected: bool,
        warning: bool,
    ) -> egui::Response {
        ui.scope(|ui| {
            self.appearance.buttons(ui, true);
            let (fill, foreground, border) = match (selected, warning) {
                (true, true) => (
                    self.appearance.warning.gamma_multiply(BADGE_OPACITY),
                    self.appearance.warning,
                    self.appearance
                        .warning
                        .gamma_multiply(ACTIVE_WARNING_BORDER_OPACITY),
                ),
                (true, false) => (
                    self.appearance.active,
                    self.appearance.foreground,
                    self.appearance.border,
                ),
                (false, _) => (
                    Color32::TRANSPARENT,
                    self.appearance.muted,
                    self.appearance.border,
                ),
            };
            ui.add(
                egui::Button::new(
                    RichText::new(text)
                        .size(SMALL_FONT)
                        .line_height(Some(SMALL_FONT * LINE_HEIGHT_RATIO))
                        .color(foreground),
                )
                .small()
                .min_size(egui::vec2(0.0, ICON_BUTTON_SIZE))
                .corner_radius(PILL_RADIUS)
                .fill(fill)
                .stroke(Stroke::new(BORDER_WIDTH, border)),
            )
        })
        .inner
    }

    fn text_button(
        &self,
        ui: &mut egui::Ui,
        row: &Row,
        kind: RowControlKind,
        text: String,
        outline: bool,
    ) -> egui::Response {
        let control = RowControl::new(row, kind);
        let response = ui
            .scope_builder(control.scope(ui.ctx()), |ui| {
                self.appearance.buttons(ui, outline);
                self.prepare_row_focus(ui, &control);
                ui.add(
                    egui::Button::new(
                        RichText::new(text)
                            .size(SMALL_FONT)
                            .line_height(Some(SMALL_FONT * LINE_HEIGHT_RATIO)),
                    )
                    .small()
                    .min_size(egui::vec2(0.0, ICON_BUTTON_SIZE)),
                )
            })
            .inner;
        reveal_focused_control(&response);
        response
    }

    fn badge(&self, ui: &mut egui::Ui, text: String, color: Color32) {
        egui::Frame::NONE
            .fill(color.gamma_multiply(BADGE_OPACITY))
            .corner_radius(ROW_RADIUS)
            .inner_margin(egui::Margin::symmetric(BADGE_PADDING_X, BADGE_PADDING_Y))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(text)
                        .size(DETAIL_FONT)
                        .line_height(Some(DETAIL_FONT * LINE_HEIGHT_RATIO))
                        .color(color),
                );
            });
    }

    fn icon_button(
        &self,
        ui: &mut egui::Ui,
        row: &Row,
        kind: RowControlKind,
        icon: Icon,
    ) -> egui::Response {
        let control = RowControl::new(row, kind);
        let response = ui
            .scope_builder(control.scope(ui.ctx()), |ui| {
                self.appearance.buttons(ui, false);
                ui.spacing_mut().button_padding = egui::Vec2::ZERO;
                self.prepare_row_focus(ui, &control);
                ui.add(
                    egui::Button::opt_image_and_text(
                        self.icons.image(icon, self.appearance.foreground),
                        None,
                    )
                    .min_size(egui::Vec2::splat(ICON_BUTTON_SIZE)),
                )
            })
            .inner;
        reveal_focused_control(&response);
        response
    }

    pub fn new(theme: &ResolvedTheme, collation_locale: &str, is_mac: bool) -> AppResult<Self> {
        Self::with_appearance(Appearance::new(theme)?, collation_locale, is_mac)
    }

    pub fn with_appearance(
        appearance: Appearance,
        collation_locale: &str,
        is_mac: bool,
    ) -> AppResult<Self> {
        Ok(Self {
            open: false,
            mounted: false,
            query: String::new(),
            capture: Capture::default(),
            conflicts_only: false,
            unassigned_only: false,
            context_keys: Vec::new(),
            polled_at: None,
            previous_focus: FocusReturn::default(),
            capture_focus: None,
            search_focus: None,
            close_focus: None,
            focus_order: Vec::new(),
            focus_rows: HashMap::new(),
            focus_target: None,
            rows_scroll: None,
            focus_next: false,
            search_focus_next: false,
            raw_overrides: None,
            overrides: Overrides::default(),
            rows: Vec::new(),
            search: Search::new(collation_locale)?,
            appearance,
            icons: Icons::new()?,
            composition: Composition::default(),
            is_mac,
        })
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn set_appearance(&mut self, appearance: Appearance) {
        self.appearance = appearance;
    }
    pub fn is_capturing(&self) -> bool {
        self.open && self.capture.target.is_some()
    }

    pub fn open(&mut self, context: &egui::Context) {
        if self.open {
            return;
        }
        self.previous_focus.capture(context);
        self.open = true;
        self.mounted = true;
        self.search_focus_next = true;
        context.request_repaint();
    }

    pub fn observe_closed(&mut self, scope: Context, now: Instant) {
        if !self.mounted
            || self.open
            || self
                .polled_at
                .is_some_and(|last| now.saturating_duration_since(last) < CONTEXT_POLL)
        {
            return;
        }
        self.context_keys.clear();
        if scope.editor {
            self.context_keys.push("editorTextFocus");
        }
        if scope.terminal {
            self.context_keys.push("terminalFocus");
        }
        self.polled_at = Some(now);
    }

    fn close(&mut self, context: &egui::Context) {
        self.open = false;
        self.query.clear();
        self.capture = Capture::default();
        self.conflicts_only = false;
        self.unassigned_only = false;
        self.capture_focus = None;
        self.search_focus = None;
        self.close_focus = None;
        self.focus_order.clear();
        self.focus_rows.clear();
        self.focus_target = None;
        self.rows_scroll = None;
        self.focus_next = false;
        self.search_focus_next = false;
        self.polled_at = None;
        self.composition.reset();
        self.previous_focus.release(context, true);
        context.request_repaint();
    }

    fn visible_rows(&self, locale: &ResolvedLocale, conflicts: &ConflictIndex<'_>) -> Vec<usize> {
        let ordered = self.search.ordered_indices(&self.rows, locale);
        let ordered = if self.query.is_empty() {
            ordered
        } else {
            let labels = ordered
                .iter()
                .map(|index| {
                    format!(
                        "{} {}",
                        label(&self.rows[*index], locale),
                        self.rows[*index].id
                    )
                })
                .collect::<Vec<_>>();
            self.search
                .filter(&self.query, &labels)
                .into_iter()
                .map(|ranked| ordered[ranked.index])
                .collect()
        };
        ordered
            .into_iter()
            .filter(|index| {
                let row = &self.rows[*index];
                if let Some(binding) = &self.capture.searched_key
                    && (row.binding.key().is_empty()
                        || !row
                            .binding
                            .matches(&binding.event(self.is_mac), self.is_mac))
                {
                    return false;
                }
                (!self.conflicts_only || conflicts.find(row).is_some())
                    && (!self.unassigned_only || row.is_unassigned())
            })
            .collect()
    }

    fn apply_effect(&mut self, effect: Effect, locale: &ResolvedLocale, output: &mut Output) {
        match effect {
            Effect::None => (),
            Effect::Warning(key) => output.warnings.push(message(locale, key, &[])),
            Effect::Assign { id, binding } => {
                if let Some(row) = self.rows.iter().find(|row| row.id == id)
                    && let Some(conflict) = ConflictIndex::new(&self.rows, self.is_mac)
                        .find(&row.with_binding(binding.clone()))
                {
                    output.warnings.push(message(
                        locale,
                        "settings.keymapConflictWarning",
                        &[("action", &label(conflict, locale))],
                    ));
                }
                output.saves.push(self.overrides.assign(&id, &binding));
            }
        }
    }

    pub fn show(
        &mut self,
        context: &egui::Context,
        locale: &ResolvedLocale,
        raw_overrides: Option<&str>,
        enabled: bool,
    ) -> AppResult<Output> {
        let mut output = Output::default();
        if !self.open {
            self.previous_focus.settle(context);
            return Ok(output);
        }
        self.icons.prepare(context)?;
        if self
            .raw_overrides
            .as_ref()
            .is_none_or(|previous| previous.as_deref() != raw_overrides)
        {
            let overrides = Overrides::parse(raw_overrides);
            let rows = catalog::rows(&overrides, self.is_mac)?;
            self.raw_overrides = Some(raw_overrides.map(str::to_owned));
            self.overrides = overrides;
            self.rows = rows;
        }
        let was_capturing = self.is_capturing();
        let container_focus = Id::new(("native-keybinding-focus-container", context.viewport_id()));
        let previous_focus = context.memory(|memory| memory.focused());
        let previous_row_focus = previous_focus.filter(|id| self.focus_rows.contains_key(id));
        let targets = self.tab_targets(locale);
        if enabled
            && previous_row_focus
                .is_some_and(|id| !targets.contains(&TabTarget::Row(self.focus_rows[&id].clone())))
        {
            context.memory_mut(|memory| {
                memory.move_focus(egui::FocusDirection::None);
                memory.request_focus(container_focus);
            });
        }
        let is_composing = context.input(|input| self.composition.observe(&input.raw.events));
        if !context.input(|input| input.raw.focused) {
            self.capture.blur();
        }
        let mut captured = Vec::new();
        let mut escape = false;
        let has_popup = egui::Popup::is_any_open(context);
        if enabled && !has_popup {
            if was_capturing {
                context.memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
            }
            context.input_mut(|input| {
                input.events.retain(|event| {
                    if matches!(
                        event,
                        Event::Key {
                            key: Key::Escape,
                            pressed: true,
                            ..
                        }
                    ) {
                        escape = true;
                        return false;
                    }
                    if was_capturing
                        && matches!(
                            event,
                            Event::Key { .. }
                                | Event::Text(_)
                                | Event::Copy
                                | Event::Cut
                                | Event::Paste(_)
                        )
                    {
                        captured.push(event.clone());
                        return false;
                    }
                    true
                })
            });
        }
        let should_escape = escape && !is_composing && !has_popup;
        if enabled && !was_capturing && !has_popup && !is_composing {
            let focused = context.memory(|memory| memory.focused());
            let target = context.input(|input| {
                input.events.iter().find_map(|event| {
                    let Event::Key {
                        key: Key::Tab,
                        pressed: true,
                        modifiers,
                        ..
                    } = event
                    else {
                        return None;
                    };
                    if modifiers.alt || modifiers.ctrl || modifiers.command || modifiers.mac_cmd {
                        return None;
                    }
                    if focused == Some(container_focus) {
                        if modifiers.shift {
                            return Some(TabTarget::Widget(container_focus));
                        }
                        return targets.first().cloned();
                    }
                    let focused = focused.map(|id| {
                        self.focus_rows
                            .get(&id)
                            .map(|control| TabTarget::Row(control.clone()))
                            .unwrap_or(TabTarget::Widget(id))
                    })?;
                    let index = targets.iter().position(|target| *target == focused)?;
                    if modifiers.shift {
                        let previous = index.checked_sub(1).unwrap_or(targets.len() - 1);
                        return Some(targets[previous].clone());
                    }
                    Some(targets[(index + 1) % targets.len()].clone())
                })
            });
            if let Some(target) = target {
                context.memory_mut(|memory| {
                    memory.move_focus(egui::FocusDirection::None);
                });
                match target {
                    TabTarget::Widget(id) => context.memory_mut(|memory| memory.request_focus(id)),
                    TabTarget::Row(control) => self.focus_target = Some(control),
                }
                context.input_mut(|input| {
                    input.events.retain(|event| {
                        !matches!(
                            event,
                            Event::Key {
                                key: Key::Tab,
                                pressed: true,
                                modifiers,
                                ..
                            } if !modifiers.alt && !modifiers.ctrl && !modifiers.command && !modifiers.mac_cmd
                        )
                    })
                });
            }
        }
        self.focus_order.clear();
        self.focus_rows.clear();
        let dialog = Layer {
            id: Id::new("native-keybindings-editor"),
            chrome: Chrome {
                background: self.appearance.background,
                border: self.appearance.modal_border,
                shadow: self.appearance.shadow,
            },
            padding: PADDING,
            transition: None,
            is_modal: true,
        };
        let response = dialog.show(context, |ui| {
            let available = context.content_rect().size();
            let width = (available.x - SCREEN_MARGIN * 2.0).min(MAX_WIDTH)
                - (f32::from(PADDING) + BORDER_WIDTH) * 2.0;
            let height = available.y * HEIGHT_RATIO - (f32::from(PADDING) + BORDER_WIDTH) * 2.0;
            ui.set_width(width.max(0.0));
            ui.set_height(height.max(0.0));
            ui.spacing_mut().item_spacing.y = SECTION_GAP;
            ui.visuals_mut().override_text_color = Some(self.appearance.foreground);
            ui.add_enabled_ui(enabled, |ui| self.content(ui, locale, &mut output))
                .inner
        });
        if enabled && response.is_top_modal {
            if response.inner
                || response.backdrop_response.clicked()
                || (should_escape && !was_capturing)
            {
                self.close(context);
            } else if should_escape {
                self.capture.cancel();
            } else {
                if previous_row_focus.is_some_and(|id| !self.focus_order.contains(&id))
                    && context.memory(|memory| memory.focused()).is_none()
                {
                    context.memory_mut(|memory| memory.request_focus(container_focus));
                }
                let owns_focus = self
                    .capture_focus
                    .is_some_and(|focus| context.memory(|memory| memory.focused() == Some(focus)));
                if self.is_capturing() && !owns_focus && !self.focus_next {
                    self.capture.blur();
                }
                if owns_focus && context.input(|input| input.raw.focused) {
                    for event in &captured {
                        let effect = self.capture.egui_key(event, self.is_mac);
                        self.apply_effect(effect, locale, &mut output);
                    }
                }
            }
        }
        if !self.open {
            output.tooltip_triggers.clear();
        }
        self.focus_target = None;
        output.started_capture = !was_capturing && self.is_capturing();
        Ok(output)
    }

    fn content(&mut self, ui: &mut egui::Ui, locale: &ResolvedLocale, output: &mut Output) -> bool {
        ui.interact(
            ui.max_rect(),
            Id::new(("native-keybinding-focus-container", ui.ctx().viewport_id())),
            egui::Sense::focusable_noninteractive(),
        );
        ui.visuals_mut().extreme_bg_color = self.appearance.input;
        ui.visuals_mut().widgets.inactive.bg_stroke =
            Stroke::new(1.0, self.appearance.input_border);
        ui.visuals_mut().widgets.inactive.corner_radius = ROW_RADIUS.into();
        let mut closed = false;
        ui.horizontal(|ui| {
            ui.add(egui::Label::new(
                RichText::new(message(locale, "settings.keymapEditorTitle", &[]))
                    .size(TITLE_FONT)
                    .strong(),
            ));
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = SEARCH_GAP;
            let search_button_width = ui.fonts_mut(|fonts| {
                fonts
                    .layout_no_wrap(
                        message(locale, "settings.keymapSearchByKey", &[]),
                        egui::FontId::proportional(SMALL_FONT),
                        self.appearance.foreground,
                    )
                    .size()
                    .x
            }) + Icon::Keyboard.size()
                + CONTROL_GAP
                + (BINDING_GAP + BORDER_WIDTH) * 2.0;
            let width = (ui.available_width() - search_button_width - SEARCH_GAP).max(0.0);
            if matches!(self.capture.target, Some(Target::Search)) {
                let text = self
                    .capture
                    .searched_key
                    .as_ref()
                    .map(|binding| binding.label(self.is_mac))
                    .unwrap_or_else(|| message(locale, "settings.keymapCapturePrompt", &[]));
                let capture = ui.add_sized(
                    [width, 0.0],
                    egui::Button::new(RichText::new(text).monospace())
                        .fill(self.appearance.active)
                        .stroke(Stroke::new(1.0, self.appearance.focus)),
                );
                self.capture_focus = Some(capture.id);
                register_focus_control(&mut self.focus_order, &capture);
                if self.focus_next {
                    capture.request_focus();
                    self.focus_next = false;
                }
                capture.ctx.memory_mut(|memory| {
                    memory.set_focus_lock_filter(capture.id, modal::FOCUS_FILTER)
                });
            } else {
                let response = ui.add_sized(
                    [width, 0.0],
                    egui::TextEdit::singleline(&mut self.query)
                        .id_salt("native-keybinding-search")
                        .font(egui::FontId::proportional(BODY_FONT))
                        .margin(egui::Margin::symmetric(
                            BUTTON_PADDING as i8,
                            BADGE_PADDING_Y * 2,
                        ))
                        .hint_text(message(locale, "settings.keymapSearchPlaceholder", &[])),
                );
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::TextEdit,
                        ui.is_enabled(),
                        message(locale, "settings.keymapSearchAriaLabel", &[]),
                    )
                });
                self.search_focus = Some(response.id);
                register_focus_control(&mut self.focus_order, &response);
                if self.search_focus_next {
                    response.request_focus();
                    self.search_focus_next = false;
                }
            }
            let search = ui
                .scope(|ui| {
                    self.appearance.buttons(ui, true);
                    ui.spacing_mut().item_spacing.x = CONTROL_GAP;
                    ui.spacing_mut().button_padding = egui::vec2(BINDING_GAP + BORDER_WIDTH, 0.0);
                    let selected = matches!(self.capture.target, Some(Target::Search));
                    let foreground = if selected {
                        self.appearance.primary_foreground
                    } else {
                        self.appearance.foreground
                    };
                    let fill = if selected {
                        self.appearance.primary
                    } else {
                        self.appearance.button_background
                    };
                    ui.add(
                        egui::Button::opt_image_and_text(
                            self.icons.image(Icon::Keyboard, foreground),
                            Some(
                                RichText::new(message(locale, "settings.keymapSearchByKey", &[]))
                                    .size(SMALL_FONT)
                                    .line_height(Some(SMALL_FONT * LINE_HEIGHT_RATIO))
                                    .color(foreground)
                                    .into(),
                            ),
                        )
                        .small()
                        .min_size(egui::vec2(0.0, ICON_BUTTON_SIZE))
                        .fill(fill),
                    )
                })
                .inner;
            register_focus_control(&mut self.focus_order, &search);
            if search.clicked() {
                if self.capture.toggle_search() {
                    self.query.clear();
                    self.focus_next = true;
                }
                ui.ctx().request_repaint();
            }
        });
        let conflicts = ConflictIndex::new(&self.rows, self.is_mac);
        let conflict_count = self
            .rows
            .iter()
            .filter(|row| conflicts.find(row).is_some())
            .count();
        let unassigned_count = self.rows.iter().filter(|row| row.is_unassigned()).count();
        let mut conflicts_only = self.conflicts_only;
        let mut unassigned_only = self.unassigned_only;
        let filters = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = SEARCH_GAP;
                let conflict_filter = self.filter_button(
                    ui,
                    format!(
                        "{} ({conflict_count})",
                        message(locale, "settings.keymapConflictFilter", &[])
                    ),
                    conflicts_only,
                    true,
                );
                if conflict_filter.clicked() {
                    conflicts_only = !conflicts_only;
                }
                let unassigned_filter = self.filter_button(
                    ui,
                    format!(
                        "{} ({unassigned_count})",
                        message(locale, "settings.keymapUnassignedFilter", &[])
                    ),
                    unassigned_only,
                    false,
                );
                if unassigned_filter.clicked() {
                    unassigned_only = !unassigned_only;
                }
                (conflict_filter, unassigned_filter)
            })
            .inner;
        register_focus_control(&mut self.focus_order, &filters.0);
        register_focus_control(&mut self.focus_order, &filters.1);
        self.conflicts_only = conflicts_only;
        self.unassigned_only = unassigned_only;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = SEARCH_GAP;
            ui.label(
                RichText::new(message(locale, "settings.keymapInspectorTitle", &[]))
                    .size(DETAIL_FONT)
                    .color(self.appearance.muted),
            );
            if self.context_keys.is_empty() {
                ui.label(
                    RichText::new(message(locale, "settings.keymapInspectorEmpty", &[]))
                        .size(DETAIL_FONT)
                        .color(self.appearance.muted),
                );
            }
            for key in &self.context_keys {
                egui::Frame::NONE
                    .fill(self.appearance.active)
                    .corner_radius(PILL_RADIUS)
                    .inner_margin(egui::Margin::symmetric(BINDING_GAP as i8, BADGE_PADDING_Y))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(*key)
                                .monospace()
                                .size(DETAIL_FONT)
                                .line_height(Some(DETAIL_FONT * LINE_HEIGHT_RATIO))
                                .color(self.appearance.foreground),
                        );
                    });
            }
        });
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(ROW_PADDING_X, 0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = ROW_CONTROL_GAP;
                    let key = message(locale, "settings.keymapKeyColumn", &[]).to_uppercase();
                    let source = message(locale, "settings.keymapSourceColumn", &[]).to_uppercase();
                    let widths = ui.fonts_mut(|fonts| {
                        [key.clone(), source.clone()].map(|text| {
                            fonts
                                .layout_no_wrap(
                                    text,
                                    egui::FontId::proportional(DETAIL_FONT),
                                    self.appearance.muted,
                                )
                                .size()
                                .x
                        })
                    });
                    let trailing_width = widths.iter().sum::<f32>() + ROW_CONTROL_GAP * 2.0;
                    ui.add_sized(
                        [
                            (ui.available_width() - trailing_width).max(0.0),
                            DETAIL_FONT * LINE_HEIGHT_RATIO,
                        ],
                        egui::Label::new(
                            RichText::new(
                                message(locale, "settings.keymapCommandColumn", &[]).to_uppercase(),
                            )
                            .size(DETAIL_FONT)
                            .color(self.appearance.muted),
                        )
                        .truncate(),
                    );
                    for (text, width) in [key, source].into_iter().zip(widths) {
                        ui.add_sized(
                            [width, DETAIL_FONT * LINE_HEIGHT_RATIO],
                            egui::Label::new(
                                RichText::new(text)
                                    .size(DETAIL_FONT)
                                    .color(self.appearance.muted),
                            ),
                        );
                    }
                });
            });
        let visible = self.visible_rows(locale, &conflicts);
        let displayed = visible
            .iter()
            .map(|index| {
                let row = &self.rows[*index];
                (
                    row.clone(),
                    conflicts
                        .find(row)
                        .map(|conflict| (conflict.id.clone(), label(conflict, locale))),
                )
            })
            .collect::<Vec<_>>();
        let displayed_ids = displayed
            .iter()
            .map(|(row, _)| row.id.clone())
            .collect::<Vec<_>>();
        let changed_scroll_offset = self.rows_scroll.as_ref().and_then(|(id, previous_rows)| {
            if *previous_rows == displayed_ids {
                return None;
            }
            egui::scroll_area::State::load(ui.ctx(), *id).map(|state| state.offset)
        });
        let scroll = egui::ScrollArea::vertical()
            .id_salt("native-keybinding-rows")
            .animated(false)
            .auto_shrink([false, false])
            .max_height(ui.available_height())
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = ROW_GAP;
                if displayed.is_empty() {
                    ui.label(
                        RichText::new(message(locale, "settings.keymapNoResults", &[]))
                            .size(SMALL_FONT)
                            .color(self.appearance.muted),
                    );
                }
                for (row, conflict) in &displayed {
                    ui.scope_builder(
                        egui::UiBuilder::new().id(Id::new((
                            "native-keybinding-row",
                            ui.ctx().viewport_id(),
                            &row.id,
                        ))),
                        |ui| self.row(ui, locale, row, conflict.as_ref(), output),
                    );
                }
            });
        self.rows_scroll = Some((scroll.id, displayed_ids));
        if changed_scroll_offset.is_some_and(|offset| offset != scroll.state.offset) {
            ui.ctx()
                .request_discard("keybinding row filtering changed the scroll layout");
        }
        let close_rect = egui::Rect::from_min_size(
            egui::pos2(
                ui.max_rect().right() + f32::from(PADDING) + BORDER_WIDTH
                    - CLOSE_INSET
                    - Icon::Close.size(),
                ui.max_rect().top() - f32::from(PADDING) - BORDER_WIDTH + CLOSE_INSET,
            ),
            egui::Vec2::splat(Icon::Close.size()),
        );
        let close = ui.interact(
            close_rect,
            ui.make_persistent_id("native-keybinding-close"),
            egui::Sense::click(),
        );
        self.close_focus = Some(close.id);
        register_focus_control(&mut self.focus_order, &close);
        if close.has_focus() {
            ui.painter().rect_stroke(
                close_rect.expand(CLOSE_FOCUS_OFFSET),
                ROW_RADIUS,
                Stroke::new(CLOSE_FOCUS_OFFSET, self.appearance.button_background),
                egui::StrokeKind::Inside,
            );
            ui.painter().rect_stroke(
                close_rect.expand(CLOSE_FOCUS_OFFSET + CLOSE_FOCUS_RING),
                ROW_RADIUS,
                Stroke::new(CLOSE_FOCUS_RING, self.appearance.focus),
                egui::StrokeKind::Inside,
            );
        }
        let opacity = if close.hovered() { 1.0 } else { CLOSE_OPACITY };
        if let Some(image) = self.icons.image(
            Icon::Close,
            self.appearance.foreground.gamma_multiply(opacity),
        ) {
            image.paint_at(ui, close_rect);
        }
        close.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                ui.is_enabled(),
                message(locale, "common.close", &[]),
            )
        });
        closed |= close.clicked();
        closed
    }

    fn row(
        &mut self,
        ui: &mut egui::Ui,
        locale: &ResolvedLocale,
        row: &Row,
        conflict: Option<&(String, String)>,
        output: &mut Output,
    ) {
        let pending = match &self.capture.target {
            Some(Target::Row { id, first }) if id == &row.id => Some(first.clone()),
            _ => None,
        };
        let assigned = if row.is_unassigned() {
            message(locale, "settings.keymapUnassigned", &[])
        } else if row.binding.key().is_empty() {
            row.default_binding_label.clone().unwrap_or_default()
        } else {
            row.binding.label(self.is_mac)
        };
        let binding_text = match &pending {
            Some(Some(first)) => message(
                locale,
                "settings.keymapChordCapturePrompt",
                &[("shortcut", &first.label(self.is_mac))],
            ),
            Some(None) => message(locale, "settings.keymapCapturePrompt", &[]),
            None => assigned.clone(),
        };
        let source = if row.is_overridden {
            "settings.keymapSourceUser"
        } else {
            "settings.keymapSourceDefault"
        };
        let can_unbind = can_unbind(row);
        let icon_controls = usize::from(row.is_overridden) + usize::from(can_unbind);
        let width = |text: String, font| {
            ui.fonts_mut(|fonts| {
                fonts
                    .layout_no_wrap(text, font, self.appearance.foreground)
                    .size()
                    .x
            })
        };
        let mut binding_width = width(binding_text, egui::FontId::monospace(SMALL_FONT));
        if pending.is_some() {
            binding_width += (BUTTON_PADDING + BORDER_WIDTH) * 2.0;
        }
        if matches!(pending, Some(Some(_))) {
            binding_width += BINDING_GAP * 2.0
                + width(
                    message(locale, "settings.keymapChordWaitingBadge", &[]),
                    egui::FontId::proportional(DETAIL_FONT),
                )
                + CONTROL_GAP * 2.0
                + width(
                    message(locale, "settings.keymapChordConfirmSingle", &[]),
                    egui::FontId::proportional(SMALL_FONT),
                )
                + BUTTON_PADDING * 2.0;
        } else if conflict.is_some() && pending.is_none() {
            binding_width += BINDING_GAP
                + width(
                    message(locale, "settings.keymapConflictBadge", &[]),
                    egui::FontId::proportional(DETAIL_FONT),
                )
                + CONTROL_GAP * 2.0;
        }
        let source_width = width(
            message(locale, source, &[]),
            egui::FontId::proportional(SMALL_FONT),
        );
        let controls_width = width(
            message(locale, "settings.keymapChange", &[]),
            egui::FontId::proportional(SMALL_FONT),
        ) + (BUTTON_PADDING + BORDER_WIDTH) * 2.0
            + icon_controls as f32 * (ICON_BUTTON_SIZE + CONTROL_GAP);
        let label_width = (ui.available_width()
            - (f32::from(ROW_PADDING_X) + BORDER_WIDTH) * 2.0
            - binding_width
            - source_width
            - controls_width
            - ROW_CONTROL_GAP * 3.0)
            .max(0.0);
        egui::Frame::NONE
            .stroke(Stroke::new(1.0, self.appearance.border))
            .corner_radius(ROW_RADIUS)
            .inner_margin(egui::Margin::symmetric(ROW_PADDING_X, ROW_PADDING_Y))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = ROW_CONTROL_GAP;
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(label_width, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_max_width(label_width);
                            ui.spacing_mut().item_spacing.y = LABEL_GAP;
                            ui.add(
                                egui::Label::new(
                                    RichText::new(label(row, locale))
                                        .size(SMALL_FONT)
                                        .line_height(Some(SMALL_FONT * LINE_HEIGHT_RATIO)),
                                )
                                .truncate(),
                            );
                            if let Some(when) = &row.when {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(when)
                                            .monospace()
                                            .size(DETAIL_FONT)
                                            .line_height(Some(DETAIL_FONT * LINE_HEIGHT_RATIO))
                                            .color(self.appearance.muted),
                                    )
                                    .truncate(),
                                );
                            }
                            if let Some((id, text)) = conflict {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = CONTROL_GAP;
                                    let unbind_label =
                                        message(locale, "settings.keymapUnbind", &[]);
                                    let button_width = ui.fonts_mut(|fonts| {
                                        fonts
                                            .layout_no_wrap(
                                                unbind_label.clone(),
                                                egui::FontId::proportional(SMALL_FONT),
                                                self.appearance.foreground,
                                            )
                                            .size()
                                            .x
                                    }) + BUTTON_PADDING * 2.0;
                                    let warning_width = (ui.available_width()
                                        - Icon::Warning.size()
                                        - button_width
                                        - CONTROL_GAP * 2.0)
                                        .max(0.0);
                                    if let Some(image) =
                                        self.icons.image(Icon::Warning, self.appearance.warning)
                                    {
                                        ui.add(image);
                                    }
                                    ui.add_sized(
                                        [warning_width, 0.0],
                                        egui::Label::new(
                                            RichText::new(message(
                                                locale,
                                                "settings.keymapConflictWarning",
                                                &[("action", text)],
                                            ))
                                            .size(SMALL_FONT)
                                            .color(self.appearance.warning),
                                        )
                                        .truncate(),
                                    );
                                    let unbind = self.text_button(
                                        ui,
                                        row,
                                        RowControlKind::ResolveConflict,
                                        unbind_label,
                                        false,
                                    );
                                    self.register_row_control(
                                        row,
                                        RowControlKind::ResolveConflict,
                                        &unbind,
                                    );
                                    if unbind.clicked() {
                                        output.saves.push(self.overrides.unbind(id));
                                    }
                                });
                            }
                        },
                    );
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = BINDING_GAP;
                        ui.spacing_mut().button_padding = egui::vec2(
                            BUTTON_PADDING + BORDER_WIDTH,
                            f32::from(BADGE_PADDING_Y) * 2.0 + BORDER_WIDTH,
                        );
                        if let Some(first) = pending {
                            let text = first
                                .as_ref()
                                .map(|binding| {
                                    message(
                                        locale,
                                        "settings.keymapChordCapturePrompt",
                                        &[("shortcut", &binding.label(self.is_mac))],
                                    )
                                })
                                .unwrap_or_else(|| {
                                    message(locale, "settings.keymapCapturePrompt", &[])
                                });
                            let control = RowControl::new(row, RowControlKind::Capture);
                            let capture = ui
                                .scope_builder(control.scope(ui.ctx()), |ui| {
                                    self.prepare_row_focus(ui, &control);
                                    ui.add(
                                        egui::Button::new(
                                            RichText::new(text)
                                                .monospace()
                                                .size(SMALL_FONT)
                                                .line_height(Some(SMALL_FONT * LINE_HEIGHT_RATIO)),
                                        )
                                        .fill(self.appearance.active)
                                        .stroke(Stroke::new(BORDER_WIDTH, self.appearance.focus))
                                        .corner_radius(ROW_RADIUS)
                                        .min_size(egui::vec2(0.0, CAPTURE_HEIGHT)),
                                    )
                                })
                                .inner;
                            self.capture_focus = Some(capture.id);
                            self.register_row_control(row, RowControlKind::Capture, &capture);
                            if self.focus_next {
                                capture.request_focus();
                                self.focus_next = false;
                            }
                            reveal_focused_control(&capture);
                            capture.ctx.memory_mut(|memory| {
                                memory.set_focus_lock_filter(capture.id, modal::FOCUS_FILTER)
                            });
                            if first.is_some() {
                                self.badge(
                                    ui,
                                    message(locale, "settings.keymapChordWaitingBadge", &[]),
                                    self.appearance.accent,
                                );
                                let confirm = self.text_button(
                                    ui,
                                    row,
                                    RowControlKind::Confirm,
                                    message(locale, "settings.keymapChordConfirmSingle", &[]),
                                    false,
                                );
                                self.register_row_control(row, RowControlKind::Confirm, &confirm);
                                if confirm.is_pointer_button_down_on() {
                                    capture.request_focus();
                                }
                                if confirm.clicked() {
                                    let effect = self.capture.confirm_single();
                                    self.apply_effect(effect, locale, output);
                                }
                            }
                        } else {
                            let color = if row.is_unassigned() {
                                self.appearance.muted
                            } else if row.is_overridden {
                                self.appearance.accent
                            } else {
                                self.appearance.foreground
                            };
                            ui.label(
                                RichText::new(assigned)
                                    .monospace()
                                    .size(SMALL_FONT)
                                    .line_height(Some(SMALL_FONT * LINE_HEIGHT_RATIO))
                                    .color(color),
                            );
                            if conflict.is_some() {
                                self.badge(
                                    ui,
                                    message(locale, "settings.keymapConflictBadge", &[]),
                                    self.appearance.warning,
                                );
                            }
                        }
                    });
                    ui.label(
                        RichText::new(message(locale, source, &[]))
                            .size(SMALL_FONT)
                            .line_height(Some(SMALL_FONT * LINE_HEIGHT_RATIO))
                            .color(self.appearance.muted),
                    );
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = CONTROL_GAP;
                        let change = self.text_button(
                            ui,
                            row,
                            RowControlKind::Change,
                            message(locale, "settings.keymapChange", &[]),
                            true,
                        );
                        self.register_row_control(row, RowControlKind::Change, &change);
                        if change.clicked() {
                            self.capture.start_row(row.id.clone());
                            self.focus_next = true;
                            ui.ctx().request_repaint();
                        }
                        if row.is_overridden {
                            let reset =
                                self.icon_button(ui, row, RowControlKind::Reset, Icon::Reset);
                            self.register_row_control(row, RowControlKind::Reset, &reset);
                            reset.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    ui.is_enabled(),
                                    message(locale, "settings.keymapResetOne", &[]),
                                )
                            });
                            if reset.clicked() {
                                output.saves.push(self.overrides.reset(&row.id));
                            }
                            output
                                .tooltip_triggers
                                .push(("settings.keymapResetOne", reset));
                        }
                        if can_unbind {
                            let unbind =
                                self.icon_button(ui, row, RowControlKind::Unbind, Icon::Unbind);
                            self.register_row_control(row, RowControlKind::Unbind, &unbind);
                            unbind.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    ui.is_enabled(),
                                    message(locale, "settings.keymapUnbind", &[]),
                                )
                            });
                            if unbind.clicked() {
                                output.saves.push(self.overrides.unbind(&row.id));
                            }
                            output
                                .tooltip_triggers
                                .push(("settings.keymapUnbind", unbind));
                        }
                    });
                });
            });
    }
}

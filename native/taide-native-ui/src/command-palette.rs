use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat, TextWrapping};
use egui::{Color32, Event, FontFamily, FontId, Galley, Id, Key, Modifiers, Sense, Stroke, Ui};
use taide_model::error::AppResult;
use taide_model::locale::ResolvedLocale;
use taide_model::theme::ResolvedTheme;

use crate::command_palette_file_match::{relative_path, split_for_display};
use crate::command_palette_query::{
    LineTarget, Mode, Query, entry_query, parse, parse_line_target,
};
use crate::command_registry::{CommandContext, PaletteEntry, Run, registry};
use crate::fuzzy_match::{Matcher, Ranked, highlight_segments};
use crate::icons::{Icon, Icons};
use crate::keymap::catalog::{Overrides, rows};
use crate::modal::{self, BORDER_WIDTH, Chrome, Composition, FocusReturn, Layer, Presence};
use crate::presentation::{color, message};

const MODAL_ID: &str = "native-command-palette";
const INPUT_ID: &str = "native-command-palette-input";
const LIST_ID: &str = "native-command-palette-list";
const LINE_ITEM_KEY: &str = "line";
const FILE_RESULT_LIMIT: usize = 200;
const MAX_WIDTH: f32 = 512.0;
const RESPONSIVE_BREAKPOINT: f32 = 640.0;
const SCREEN_MARGIN: f32 = 16.0;
const SURFACE_RADIUS: u8 = 6;
const DIALOG_PADDING: i8 = 0;
const EXITING_CONTENT_ALPHA: f32 = 1.0;
const INPUT_ROW_HEIGHT: f32 = 36.0;
const INPUT_PADDING_X: f32 = 12.0;
const INPUT_ICON_GAP: f32 = 8.0;
const INPUT_ICON_OPACITY: f32 = 0.5;
const LIST_MAX_HEIGHT: f32 = 300.0;
const LIST_SCROLL_PADDING: f32 = 4.0;
const LIST_TOP: f32 = 0.0;
const EMPTY_PADDING_Y: f32 = 24.0;
const GROUP_PADDING: f32 = 4.0;
const HEADING_PADDING_X: f32 = 8.0;
const HEADING_PADDING_Y: f32 = 6.0;
const HEADING_GAP: f32 = 6.0;
const REFRESHING_GAP: f32 = 4.0;
const SPIN_SECONDS: f64 = 1.0;
const ROTATION_ORIGIN: f32 = 0.5;
const ITEM_PADDING_X: f32 = 8.0;
const ITEM_PADDING_Y: f32 = 6.0;
const ITEM_GAP: f32 = 8.0;
const ITEM_RADIUS: u8 = 4;
const BODY_FONT: f32 = 14.0;
const BODY_LINE_HEIGHT: f32 = 20.0;
const DETAIL_FONT: f32 = 12.0;
const DETAIL_LINE_HEIGHT: f32 = 16.0;
const SHORTCUT_LETTER_SPACING: f32 = 1.2;
const DISABLED_OPACITY: f32 = 0.5;
const ELLIPSIS: char = '…';

pub struct Appearance {
    modal_background: Color32,
    modal_border: Color32,
    shadow: Color32,
    surface: Color32,
    foreground: Color32,
    muted: Color32,
    separator: Color32,
    selected_background: Color32,
    selected_foreground: Color32,
    selection_ring: Color32,
    match_highlight: Color32,
}

impl Appearance {
    pub fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        Ok(Self {
            modal_background: color(theme, "modal.background")?,
            modal_border: color(theme, "modal.border")?,
            shadow: color(theme, "app.shadow")?,
            surface: color(theme, "panel.background")?,
            foreground: color(theme, "app.foreground")?,
            muted: color(theme, "appSidebar.iconDefault")?,
            separator: color(theme, "app.border")?,
            selected_background: color(theme, "list.activeBackground")?,
            selected_foreground: color(theme, "list.foreground")?,
            selection_ring: color(theme, "app.accent")?,
            match_highlight: color(theme, "panel.matchHighlight")?,
        })
    }

    fn chrome(&self) -> Chrome {
        Chrome {
            background: self.modal_background,
            border: self.modal_border,
            shadow: self.shadow,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct FileIndex<'a> {
    pub root: Option<&'a str>,
    pub paths: Option<&'a [String]>,
    pub revision: u64,
    pub is_pending: bool,
    pub is_refreshing: bool,
}

#[cfg(feature = "native-host")]
#[derive(Clone, Copy, Default)]
pub struct SymbolIndex<'a> {
    pub entries: Option<&'a [taide_native_editor::document_symbols::Symbol]>,
    pub generation: u64,
    pub is_pending: bool,
}

#[cfg(feature = "native-host")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceSymbol {
    pub name: String,
    pub kind: taide_native_editor::document_symbols::SymbolKind,
    pub container_name: String,
    pub path: String,
    pub line: u32,
    pub column: u32,
}

#[cfg(feature = "native-host")]
#[derive(Clone, Copy, Default)]
pub struct WorkspaceSymbolIndex<'a> {
    pub entries: Option<&'a [WorkspaceSymbol]>,
    pub query: Option<&'a str>,
    pub generation: u64,
    pub is_pending: bool,
}

#[cfg(feature = "native-host")]
pub fn trim_workspace_query(query: &str) -> &str {
    query.trim_matches(crate::keybinding_search::js_whitespace)
}

#[derive(Clone, Copy)]
pub struct Scope<'a> {
    pub locale: &'a ResolvedLocale,
    pub commands: &'a CommandContext,
    pub keymap_overrides: Option<&'a str>,
    pub files: FileIndex<'a>,
    pub active_file: Option<&'a str>,
    #[cfg(feature = "native-host")]
    pub symbols: SymbolIndex<'a>,
    #[cfg(feature = "native-host")]
    pub workspace_symbols: WorkspaceSymbolIndex<'a>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    RunCommand(String),
    OpenFile(String),
    RevealLine(LineTarget),
    #[cfg(feature = "native-host")]
    RevealSymbol {
        generation: u64,
        index: usize,
    },
    #[cfg(feature = "native-host")]
    OpenWorkspaceSymbol {
        generation: u64,
        index: usize,
    },
}

#[derive(Debug, Default, PartialEq)]
pub struct Output {
    pub action: Option<Action>,
}

#[cfg(any(test, feature = "inspection"))]
#[derive(Clone, Debug, PartialEq)]
pub struct RowInspection {
    pub key: String,
    pub label: String,
    pub detail: Option<String>,
    pub shortcut: Option<String>,
    pub is_enabled: bool,
    pub is_selected: bool,
    pub rect: egui::Rect,
    pub icon: Icon,
    pub icon_rect: egui::Rect,
    pub label_left: f32,
}

#[cfg(any(test, feature = "inspection"))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Inspection {
    pub is_open: bool,
    pub query: String,
    pub placeholder: String,
    pub heading: Option<String>,
    pub refreshing: Option<String>,
    pub empty_message: Option<String>,
    pub rows: Vec<RowInspection>,
    pub dialog: Option<egui::Rect>,
    pub input: Option<egui::Rect>,
    pub input_icon: Option<egui::Rect>,
    pub input_field: Option<egui::Rect>,
    pub refreshing_icon: Option<(egui::Rect, f32)>,
    pub list: Option<egui::Rect>,
    pub has_input_focus: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Navigation {
    Next,
    Previous,
    First,
    Last,
    Activate,
    Dismiss,
}

struct Item {
    key: String,
    action: Action,
    icon: Icon,
    label: String,
    indices: Vec<usize>,
    detail: Option<(String, Vec<usize>)>,
    shortcut: Option<String>,
    is_enabled: bool,
    is_truncated: bool,
}

struct Group {
    heading: Option<String>,
    refreshing: Option<String>,
    items: Vec<Item>,
}

struct Listing {
    group: Option<Group>,
    empty_message: String,
}

impl Listing {
    fn items(&self) -> &[Item] {
        self.group
            .as_ref()
            .map_or(&[][..], |group| group.items.as_slice())
    }
}

struct FileRanking {
    search_term: String,
    root: String,
    revision: u64,
    entries: Vec<Ranked<String>>,
}

struct Shortcuts {
    source: Option<String>,
    labels: HashMap<String, Option<String>>,
}

#[derive(Default)]
struct Interaction {
    activated: Option<Action>,
    is_dismissed: bool,
}

struct Typography {
    size: f32,
    line_height: f32,
    color: Color32,
    highlight: Color32,
}

struct RowLayout {
    response: egui::Response,
    icon: egui::Rect,
    label_left: f32,
}

pub struct Palette {
    is_open: bool,
    query: String,
    listed_query: Option<String>,
    selected: Option<String>,
    focus_return: FocusReturn,
    presence: Option<Presence>,
    is_reduced_motion: bool,
    should_place_caret_at_end: bool,
    should_reveal_selected: bool,
    should_reset_scroll: bool,
    composition: Composition,
    is_mac: bool,
    matcher: Matcher,
    appearance: Appearance,
    icons: Icons,
    shortcuts: Option<Shortcuts>,
    file_ranking: Option<FileRanking>,
    #[cfg(any(test, feature = "inspection"))]
    inspection: Inspection,
}

impl Palette {
    pub fn new(theme: &ResolvedTheme, collation_locale: &str, is_mac: bool) -> AppResult<Self> {
        Self::with_appearance(Appearance::new(theme)?, collation_locale, is_mac)
    }

    pub fn with_appearance(
        appearance: Appearance,
        collation_locale: &str,
        is_mac: bool,
    ) -> AppResult<Self> {
        Ok(Self {
            is_open: false,
            query: String::new(),
            listed_query: None,
            selected: None,
            focus_return: FocusReturn::default(),
            presence: None,
            is_reduced_motion: false,
            should_place_caret_at_end: false,
            should_reveal_selected: false,
            should_reset_scroll: false,
            composition: Composition::default(),
            is_mac,
            matcher: Matcher::new(collation_locale)?,
            appearance,
            icons: Icons::new()?,
            shortcuts: None,
            file_ranking: None,
            #[cfg(any(test, feature = "inspection"))]
            inspection: Inspection::default(),
        })
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn observes_files(&self) -> bool {
        self.is_open && parse(&self.query).mode == Mode::Files
    }

    #[cfg(feature = "native-host")]
    pub fn observes_symbols(&self) -> bool {
        self.is_open && parse(&self.query).mode == Mode::Symbol
    }

    #[cfg(feature = "native-host")]
    pub fn workspace_query(&self) -> Option<&str> {
        let query = parse(&self.query);
        (self.is_open && query.mode == Mode::WorkspaceSymbol).then_some(query.search_term)
    }

    pub fn set_appearance(&mut self, appearance: Appearance) {
        self.appearance = appearance;
    }

    #[cfg(any(test, feature = "inspection"))]
    pub fn inspection(&self) -> &Inspection {
        &self.inspection
    }

    pub fn set_reduced_motion(&mut self, is_reduced_motion: bool) {
        self.is_reduced_motion = is_reduced_motion;
    }

    pub fn open(&mut self, context: &egui::Context, entry: PaletteEntry) {
        if !self.is_open {
            self.focus_return.capture(context);
            self.is_open = true;
            self.selected = None;
            self.listed_query = None;
            self.should_reset_scroll = true;
        }
        self.set_query(entry_query(entry));
        context.request_repaint();
    }

    fn set_query(&mut self, query: String) {
        self.query = query;
        self.should_place_caret_at_end = true;
    }

    fn close(&mut self, context: &egui::Context, should_restore_focus: bool) {
        self.is_open = false;
        self.query.clear();
        self.composition.reset();
        self.clear_listing();
        let now = context.input(|input| input.time);
        if let Some(presence) = &mut self.presence {
            presence.target(false, now);
        }
        let input = input_id(context);
        context.memory_mut(|memory| memory.surrender_focus(input));
        self.focus_return.release(context, should_restore_focus);
        context.request_repaint();
    }

    fn clear_listing(&mut self) {
        self.listed_query = None;
        self.selected = None;
        self.should_place_caret_at_end = false;
        self.should_reveal_selected = false;
        self.should_reset_scroll = false;
        self.file_ranking = None;
    }

    fn sync_presence(&mut self, context: &egui::Context) -> Option<Presence> {
        let now = context.input(|input| input.time);
        match &mut self.presence {
            Some(presence) => presence.target(self.is_open, now),
            None if self.is_open => self.presence = Some(Presence::enter(now)),
            None => (),
        }
        let is_reduced_motion = self.is_reduced_motion;
        let presence = self.presence.as_mut()?;
        presence.set_reduced_motion(is_reduced_motion);
        if presence.is_present(now) {
            return Some(*presence);
        }
        modal::unmount(context, modal::layer_id(Id::new(MODAL_ID)));
        self.presence = None;
        self.clear_listing();
        None
    }

    pub fn show(
        &mut self,
        context: &egui::Context,
        scope: Scope<'_>,
        enabled: bool,
    ) -> AppResult<Output> {
        let mut output = Output::default();
        self.reset_inspection();
        let Some(presence) = self.sync_presence(context) else {
            self.focus_return.settle(context);
            return Ok(output);
        };
        self.icons.prepare(context)?;
        let transition = presence.sample(context.input(|input| input.time));
        if transition.is_active {
            context.request_repaint();
        }
        let is_open = self.is_open;
        let dialog = Layer {
            id: Id::new(MODAL_ID),
            chrome: self.appearance.chrome(),
            padding: DIALOG_PADDING,
            transition: Some(transition),
            is_modal: is_open,
        };
        let layer = dialog.layer_id();
        let is_covered =
            context.memory(|memory| memory.top_modal_layer().is_some_and(|top| top != layer));
        let accepts_input = enabled && is_open && !is_covered;
        let is_composing = context.input(|input| self.composition.observe(&input.raw.events));
        let keys = if accepts_input && !is_composing {
            take_navigation(context)
        } else {
            Vec::new()
        };
        let response = dialog.show(context, |ui| {
            if !is_open {
                ui.visuals_mut().disabled_alpha = EXITING_CONTENT_ALPHA;
            }
            ui.add_enabled_ui(enabled && is_open, |ui| {
                self.content(ui, &scope, &keys, accepts_input)
            })
            .inner
        });
        let interaction = response.inner?;
        if !is_open {
            self.reset_inspection();
            self.focus_return.settle(context);
            return Ok(output);
        }
        self.trace_dialog(response.response.rect);
        if !accepts_input {
            return Ok(output);
        }
        match interaction.activated {
            Some(Action::RunCommand(id)) => {
                let run = registry()?
                    .command(&id)
                    .and_then(|command| command.runnable(scope.commands));
                match run {
                    Some(Run::OpenPalette(entry)) => self.set_query(entry_query(entry)),
                    Some(_) => {
                        output.action = Some(Action::RunCommand(id));
                        self.close(context, false);
                    }
                    None => (),
                }
            }
            Some(action) => {
                output.action = Some(action);
                self.close(context, false);
            }
            None if interaction.is_dismissed || response.backdrop_response.clicked() => {
                self.close(context, true)
            }
            None => (),
        }
        Ok(output)
    }

    fn content(
        &mut self,
        ui: &mut Ui,
        scope: &Scope<'_>,
        keys: &[Navigation],
        accepts_input: bool,
    ) -> AppResult<Interaction> {
        let width = dialog_width(ui.ctx().content_rect().width()) - BORDER_WIDTH * 2.0;
        ui.set_width(width.max(0.0));
        #[cfg(feature = "native-host")]
        ui.set_max_height(INPUT_ROW_HEIGHT + LIST_MAX_HEIGHT);
        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
        egui::Frame::NONE
            .fill(self.appearance.surface)
            .corner_radius(SURFACE_RADIUS)
            .show(ui, |ui| -> AppResult<Interaction> {
                self.input(ui, scope);
                let listing = self.listing(scope)?;
                self.sync_selection(listing.items());
                let mut interaction = Interaction::default();
                for key in keys {
                    match key {
                        Navigation::Dismiss => interaction.is_dismissed = true,
                        Navigation::Activate => {
                            if interaction.activated.is_none() {
                                interaction.activated = self.selected_action(listing.items());
                            }
                        }
                        step => self.navigate(listing.items(), *step),
                    }
                }
                let clicked = self.list(ui, &listing, accepts_input);
                if interaction.activated.is_none() {
                    interaction.activated = clicked;
                }
                Ok(interaction)
            })
            .inner
    }

    fn input(&mut self, ui: &mut Ui, scope: &Scope<'_>) {
        let (row, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), INPUT_ROW_HEIGHT),
            Sense::hover(),
        );
        ui.painter().hline(
            row.x_range(),
            row.bottom() - BORDER_WIDTH / 2.0,
            Stroke::new(BORDER_WIDTH, self.appearance.separator),
        );
        let icon = egui::Rect::from_center_size(
            egui::pos2(
                row.left() + INPUT_PADDING_X + Icon::Search.size() / 2.0,
                row.center().y,
            ),
            egui::Vec2::splat(Icon::Search.size()),
        );
        if let Some(image) = self.icons.image(
            Icon::Search,
            self.appearance
                .foreground
                .gamma_multiply(INPUT_ICON_OPACITY),
        ) {
            image.paint_at(ui, icon);
        }
        let field = egui::Rect::from_min_max(
            egui::pos2(icon.right() + INPUT_ICON_GAP, row.top()),
            egui::pos2(row.right() - INPUT_PADDING_X, row.bottom() - BORDER_WIDTH),
        );
        self.trace_input_layout(icon, field);
        let id = input_id(ui.ctx());
        if std::mem::take(&mut self.should_place_caret_at_end) {
            let mut state = egui::TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
            state
                .cursor
                .set_char_range(Some(CCursorRange::one(CCursor::new(
                    self.query.chars().count(),
                ))));
            egui::TextEdit::store_state(ui.ctx(), id, state);
        }
        let placeholder = message(scope.locale, parse(&self.query).mode.placeholder_key(), &[]);
        self.trace_input(&placeholder, row);
        let is_enabled = ui.is_enabled();
        let mut field_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(field)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let response = egui::TextEdit::singleline(&mut self.query)
            .id(id)
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::ZERO)
            .desired_width(field.width())
            .font(FontId::proportional(BODY_FONT))
            .text_color(self.appearance.foreground)
            .hint_text(placeholder)
            .return_key(None)
            .event_filter(modal::FOCUS_FILTER)
            .show(&mut field_ui)
            .response;
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::TextEdit,
                is_enabled,
                message(scope.locale, "palette.title", &[]),
            )
        });
        if is_enabled && !response.has_focus() {
            ui.memory_mut(|memory| memory.request_focus_with_filter(id, modal::FOCUS_FILTER));
        }
        self.trace_focus(ui.memory(|memory| memory.has_focus(id)));
    }

    fn listing(&mut self, scope: &Scope<'_>) -> AppResult<Listing> {
        let Query { mode, search_term } = parse(&self.query);
        let search_term = search_term.to_owned();
        let text = |key: &str| message(scope.locale, key, &[]);
        let has_project = scope.commands.active_project.is_some();
        let without_active_file = || {
            text(if scope.active_file.is_none() {
                "palette.noActiveFile"
            } else {
                "palette.noResults"
            })
        };
        let group = |heading: &str, refreshing: Option<String>, items: Vec<Item>| {
            Some(Group {
                heading: Some(text(heading)),
                refreshing,
                items,
            })
        };
        Ok(match mode {
            Mode::Commands => Listing {
                group: group(
                    "palette.commands",
                    None,
                    self.command_items(&search_term, scope)?,
                ),
                empty_message: text("palette.noResults"),
            },
            Mode::Files => Listing {
                group: group(
                    "palette.files",
                    scope
                        .files
                        .is_refreshing
                        .then(|| text("palette.filesRefreshing")),
                    self.file_items(&search_term, &scope.files),
                ),
                empty_message: text(
                    if has_project && (scope.files.root.is_none() || scope.files.is_pending) {
                        "common.loading"
                    } else {
                        "palette.noResults"
                    },
                ),
            },
            Mode::Symbol => Listing {
                #[cfg(feature = "native-host")]
                group: group(
                    "palette.symbols",
                    None,
                    if scope.active_file.is_some() {
                        self.symbol_items(&search_term, scope.symbols)
                    } else {
                        Vec::new()
                    },
                ),
                #[cfg(not(feature = "native-host"))]
                group: group("palette.symbols", None, Vec::new()),
                #[cfg(feature = "native-host")]
                empty_message: if scope.active_file.is_some() && scope.symbols.is_pending {
                    text("common.loading")
                } else {
                    without_active_file()
                },
                #[cfg(not(feature = "native-host"))]
                empty_message: without_active_file(),
            },
            Mode::Line => Listing {
                group: parse_line_target(&search_term)
                    .filter(|_| scope.active_file.is_some())
                    .map(|target| Group {
                        heading: None,
                        refreshing: None,
                        items: vec![Item {
                            key: LINE_ITEM_KEY.into(),
                            action: Action::RevealLine(target),
                            icon: Icon::CornerDownLeft,
                            label: target.label(),
                            indices: Vec::new(),
                            detail: None,
                            shortcut: None,
                            is_enabled: true,
                            is_truncated: false,
                        }],
                    }),
                empty_message: without_active_file(),
            },
            Mode::WorkspaceSymbol => Listing {
                #[cfg(feature = "native-host")]
                group: group(
                    "palette.workspaceSymbols",
                    None,
                    if !scope.workspace_symbols.is_pending
                        && scope.workspace_symbols.query == Some(search_term.as_str())
                    {
                        workspace_symbol_items(&search_term, scope.workspace_symbols)
                    } else {
                        Vec::new()
                    },
                ),
                #[cfg(not(feature = "native-host"))]
                group: group("palette.workspaceSymbols", None, Vec::new()),
                empty_message: text(if !has_project {
                    "app.openProjectFirst"
                } else {
                    #[cfg(feature = "native-host")]
                    if !trim_workspace_query(&search_term).is_empty()
                        && (scope.workspace_symbols.is_pending
                            || scope.workspace_symbols.query != Some(search_term.as_str()))
                    {
                        "common.loading"
                    } else {
                        "palette.noResults"
                    }
                    #[cfg(not(feature = "native-host"))]
                    "palette.noResults"
                }),
            },
        })
    }

    fn command_items(&mut self, search_term: &str, scope: &Scope<'_>) -> AppResult<Vec<Item>> {
        let is_mac = self.is_mac;
        let shortcuts = match self.shortcuts.take() {
            Some(shortcuts) if shortcuts.source.as_deref() == scope.keymap_overrides => shortcuts,
            _ => {
                let mut labels = HashMap::new();
                for row in rows(&Overrides::parse(scope.keymap_overrides), is_mac)? {
                    let label = if row.binding.key().is_empty() {
                        row.default_binding_label
                            .filter(|default_label| !default_label.is_empty())
                    } else {
                        Some(row.binding.label(is_mac))
                    };
                    labels.entry(row.id).or_insert(label);
                }
                Shortcuts {
                    source: scope.keymap_overrides.map(str::to_owned),
                    labels,
                }
            }
        };
        let shortcuts = self.shortcuts.insert(shortcuts);
        let commands = registry()?
            .commands()
            .iter()
            .filter(|command| command.is_registered(is_mac));
        Ok(self
            .matcher
            .filter(
                search_term,
                commands,
                |command| Cow::Owned(command.label(scope.locale)),
                None,
            )
            .into_iter()
            .map(|ranked| Item {
                key: ranked.item.id.clone(),
                action: Action::RunCommand(ranked.item.id.clone()),
                icon: Icon::Terminal,
                label: ranked.label,
                indices: ranked.matched.indices,
                detail: None,
                shortcut: shortcuts
                    .labels
                    .get(ranked.item.keymap_id.as_ref().unwrap_or(&ranked.item.id))
                    .cloned()
                    .flatten(),
                is_enabled: ranked.item.is_runnable(scope.commands),
                is_truncated: false,
            })
            .collect())
    }

    #[cfg(feature = "native-host")]
    fn symbol_items(&mut self, search_term: &str, symbols: SymbolIndex<'_>) -> Vec<Item> {
        let entries = symbols.entries.unwrap_or_default();
        let indexed = entries.iter().enumerate().collect::<Vec<_>>();
        self.matcher
            .filter(
                search_term,
                &indexed,
                |(_, symbol)| Cow::Borrowed(&symbol.name),
                None,
            )
            .into_iter()
            .map(|ranked| {
                let (index, symbol) = *ranked.item;
                Item {
                    key: format!("symbol/{}/{index}", symbols.generation),
                    action: Action::RevealSymbol {
                        generation: symbols.generation,
                        index,
                    },
                    icon: Icon::Braces,
                    label: ranked.label,
                    indices: ranked.matched.indices,
                    detail: (!symbol.container_label.is_empty())
                        .then(|| (symbol.container_label.clone(), Vec::new())),
                    shortcut: None,
                    is_enabled: true,
                    is_truncated: true,
                }
            })
            .collect()
    }

    fn file_items(&mut self, search_term: &str, files: &FileIndex<'_>) -> Vec<Item> {
        let (Some(root), Some(paths)) = (files.root, files.paths) else {
            return Vec::new();
        };
        let ranking = match self.file_ranking.take() {
            Some(ranking)
                if ranking.revision == files.revision
                    && ranking.root == root
                    && ranking.search_term == search_term =>
            {
                ranking
            }
            _ => FileRanking {
                search_term: search_term.to_owned(),
                root: root.to_owned(),
                revision: files.revision,
                entries: self
                    .matcher
                    .filter(
                        search_term,
                        paths,
                        |path| Cow::Borrowed(relative_path(root, path)),
                        Some(FILE_RESULT_LIMIT),
                    )
                    .into_iter()
                    .map(|ranked| Ranked {
                        item: ranked.item.clone(),
                        label: ranked.label,
                        matched: ranked.matched,
                    })
                    .collect(),
            },
        };
        self.file_ranking
            .insert(ranking)
            .entries
            .iter()
            .map(|ranked| {
                let display = split_for_display(&ranked.label, &ranked.matched.indices);
                Item {
                    key: ranked.item.clone(),
                    action: Action::OpenFile(ranked.item.clone()),
                    icon: Icon::File,
                    label: display.file_name.to_owned(),
                    indices: display.file_name_indices,
                    detail: display
                        .dir_path
                        .map(|dir_path| (dir_path.to_owned(), display.dir_path_indices)),
                    shortcut: None,
                    is_enabled: true,
                    is_truncated: true,
                }
            })
            .collect()
    }

    fn sync_selection(&mut self, items: &[Item]) {
        let is_query_listed = self.listed_query.as_deref() == Some(self.query.as_str());
        let is_selected_listed = self
            .selected
            .as_ref()
            .is_some_and(|key| items.iter().any(|item| &item.key == key));
        if is_query_listed && is_selected_listed {
            return;
        }
        self.listed_query = Some(self.query.clone());
        self.select(
            items
                .iter()
                .find(|item| item.is_enabled)
                .map(|item| item.key.clone()),
        );
    }

    fn select(&mut self, key: Option<String>) {
        if self.selected != key {
            self.selected = key;
            self.should_reveal_selected = true;
        }
    }

    fn selected_action(&self, items: &[Item]) -> Option<Action> {
        let selected = self.selected.as_ref()?;
        items
            .iter()
            .find(|item| item.is_enabled && &item.key == selected)
            .map(|item| item.action.clone())
    }

    fn navigate(&mut self, items: &[Item], step: Navigation) {
        let candidates = items
            .iter()
            .filter(|item| item.is_enabled)
            .map(|item| &item.key)
            .collect::<Vec<_>>();
        let current = self
            .selected
            .as_ref()
            .and_then(|key| candidates.iter().position(|candidate| *candidate == key));
        let target = match step {
            Navigation::Next => candidates.get(current.map_or(0, |index| index + 1)),
            Navigation::Previous => current
                .and_then(|index| index.checked_sub(1))
                .and_then(|index| candidates.get(index)),
            Navigation::First => candidates.first(),
            Navigation::Last => candidates.last(),
            Navigation::Activate | Navigation::Dismiss => None,
        };
        if let Some(key) = target {
            self.select(Some((*key).clone()));
        }
    }

    fn list(&mut self, ui: &mut Ui, listing: &Listing, accepts_input: bool) -> Option<Action> {
        let should_reveal = std::mem::take(&mut self.should_reveal_selected);
        let is_pointer_moving = ui.input(|input| input.pointer.delta() != egui::Vec2::ZERO);
        let mut activated = None;
        let mut area = egui::ScrollArea::vertical()
            .id_salt(LIST_ID)
            .max_height(LIST_MAX_HEIGHT)
            .auto_shrink([false, true])
            .animated(false)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
        if std::mem::take(&mut self.should_reset_scroll) {
            area = area.vertical_scroll_offset(LIST_TOP);
        }
        let viewport = area
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                if listing.items().is_empty() {
                    self.empty(ui, &listing.empty_message);
                }
                let Some(group) = &listing.group else {
                    return;
                };
                egui::Frame::NONE
                    .inner_margin(GROUP_PADDING)
                    .show(ui, |ui| {
                        let heading = group
                            .heading
                            .as_deref()
                            .map(|heading| self.heading(ui, heading, group.refreshing.as_deref()));
                        for (index, item) in group.items.iter().enumerate() {
                            let is_selected = self.selected.as_ref() == Some(&item.key);
                            let RowLayout {
                                response,
                                icon,
                                label_left,
                            } = self.row(ui, item, is_selected);
                            if is_selected && should_reveal {
                                let target = match heading {
                                    Some(heading) if index == 0 => heading.union(response.rect),
                                    _ => response.rect,
                                };
                                ui.scroll_to_rect_animation(
                                    target.expand2(egui::vec2(0.0, LIST_SCROLL_PADDING)),
                                    None,
                                    egui::style::ScrollAnimation::none(),
                                );
                            }
                            if accepts_input && item.is_enabled {
                                if response.clicked() {
                                    self.selected = Some(item.key.clone());
                                    activated = Some(item.action.clone());
                                } else if is_pointer_moving && response.hovered() {
                                    self.selected = Some(item.key.clone());
                                }
                            }
                            self.trace_row(item, is_selected, response.rect, icon, label_left);
                        }
                    });
            })
            .inner_rect;
        self.trace_list(listing, viewport);
        activated
    }

    fn empty(&self, ui: &mut Ui, text: &str) {
        let mut job = LayoutJob::single_section(
            text.to_owned(),
            TextFormat {
                font_id: FontId::proportional(BODY_FONT),
                line_height: Some(BODY_LINE_HEIGHT),
                color: self.appearance.foreground,
                ..Default::default()
            },
        );
        job.wrap.max_width = ui.available_width();
        job.halign = egui::Align::Center;
        let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(
                ui.available_width(),
                galley.size().y.max(BODY_LINE_HEIGHT) + EMPTY_PADDING_Y * 2.0,
            ),
            Sense::hover(),
        );
        ui.painter().galley(
            egui::pos2(rect.center().x, rect.top() + EMPTY_PADDING_Y),
            galley,
            self.appearance.foreground,
        );
    }

    fn heading(&mut self, ui: &mut Ui, text: &str, refreshing: Option<&str>) -> egui::Rect {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(
                ui.available_width(),
                DETAIL_LINE_HEIGHT + HEADING_PADDING_Y * 2.0,
            ),
            Sense::hover(),
        );
        let muted = self.appearance.muted;
        let family = crate::font_families::medium(ui);
        let caption = |ui: &Ui, text: &str, max_width: f32| {
            let mut job = LayoutJob::single_section(
                text.to_owned(),
                TextFormat {
                    font_id: FontId::new(DETAIL_FONT, family.clone()),
                    line_height: Some(DETAIL_LINE_HEIGHT),
                    color: muted,
                    ..Default::default()
                },
            );
            job.wrap = truncation(max_width.max(0.0));
            ui.fonts_mut(|fonts| fonts.layout_job(job))
        };
        let left = rect.left() + HEADING_PADDING_X;
        let right = rect.right() - HEADING_PADDING_X;
        let top = rect.top() + HEADING_PADDING_Y;
        let title = caption(ui, text, right - left);
        let title_right = left + title.size().x;
        ui.painter().galley(egui::pos2(left, top), title, muted);
        let Some(refreshing) = refreshing else {
            return rect;
        };
        let spinner = egui::Rect::from_center_size(
            egui::pos2(
                title_right + HEADING_GAP + Icon::Loader.size() / 2.0,
                rect.center().y,
            ),
            egui::Vec2::splat(Icon::Loader.size()),
        );
        let turn = ui.input(|input| input.time).rem_euclid(SPIN_SECONDS) / SPIN_SECONDS;
        let angle = turn as f32 * std::f32::consts::TAU;
        if let Some(image) = self.icons.image(Icon::Loader, muted) {
            image
                .rotate(angle, egui::Vec2::splat(ROTATION_ORIGIN))
                .paint_at(ui, spinner);
        }
        ui.ctx().request_repaint();
        let label_left = spinner.right() + REFRESHING_GAP;
        let label = caption(ui, refreshing, right - label_left);
        ui.painter()
            .galley(egui::pos2(label_left, top), label, muted);
        self.trace_refreshing(spinner, angle);
        rect
    }

    fn row(&self, ui: &mut Ui, item: &Item, is_selected: bool) -> RowLayout {
        #[cfg(feature = "native-host")]
        let inline_detail = matches!(
            item.action,
            Action::RevealSymbol { .. } | Action::OpenWorkspaceSymbol { .. }
        );
        #[cfg(not(feature = "native-host"))]
        let inline_detail = false;
        let opacity = if item.is_enabled {
            1.0
        } else {
            DISABLED_OPACITY
        };
        let muted = self.appearance.muted.gamma_multiply(opacity);
        let highlight = self.appearance.match_highlight.gamma_multiply(opacity);
        let width = ui.available_width();
        let icon_side = item.icon.size();
        let text_width = (width - ITEM_PADDING_X * 2.0 - icon_side - ITEM_GAP).max(0.0);
        let shortcut = item.shortcut.as_deref().map(|shortcut| {
            let mut job = LayoutJob::single_section(
                shortcut.to_owned(),
                TextFormat {
                    font_id: FontId::proportional(DETAIL_FONT),
                    extra_letter_spacing: SHORTCUT_LETTER_SPACING,
                    line_height: Some(DETAIL_LINE_HEIGHT),
                    color: muted,
                    ..Default::default()
                },
            );
            job.wrap = truncation(text_width);
            ui.fonts_mut(|fonts| fonts.layout_job(job))
        });
        let label_width = shortcut.as_ref().map_or(text_width, |shortcut| {
            (text_width - shortcut.size().x - ITEM_GAP).max(0.0)
        });
        let label_color = if is_selected {
            self.appearance.selected_foreground
        } else {
            self.appearance.foreground
        }
        .gamma_multiply(opacity);
        let mut label = highlighted(
            ui,
            &item.label,
            &item.indices,
            &Typography {
                size: BODY_FONT,
                line_height: BODY_LINE_HEIGHT,
                color: label_color,
                highlight,
            },
            label_width,
            item.is_truncated,
        );
        let mut detail = item.detail.as_ref().map(|(detail, indices)| {
            highlighted(
                ui,
                detail,
                indices,
                &Typography {
                    size: DETAIL_FONT,
                    line_height: DETAIL_LINE_HEIGHT,
                    color: muted,
                    highlight,
                },
                text_width,
                true,
            )
        });
        if inline_detail
            && let (Some(caption), Some((text, indices))) = (detail.as_ref(), item.detail.as_ref())
        {
            let natural = label.size().x + caption.size().x;
            if natural + ITEM_GAP > text_width && natural > 0.0 {
                let available = (text_width - ITEM_GAP).max(0.0);
                let label_width = available * label.size().x / natural;
                label = highlighted(
                    ui,
                    &item.label,
                    &item.indices,
                    &Typography {
                        size: BODY_FONT,
                        line_height: BODY_LINE_HEIGHT,
                        color: label_color,
                        highlight,
                    },
                    label_width,
                    true,
                );
                detail = Some(highlighted(
                    ui,
                    text,
                    indices,
                    &Typography {
                        size: DETAIL_FONT,
                        line_height: DETAIL_LINE_HEIGHT,
                        color: muted,
                        highlight,
                    },
                    available - label_width,
                    true,
                ));
            }
        }
        let text_height = if inline_detail {
            BODY_LINE_HEIGHT
        } else {
            label.size().y + detail.as_ref().map_or(0.0, |detail| detail.size().y)
        };
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(
                width,
                text_height.max(BODY_LINE_HEIGHT) + ITEM_PADDING_Y * 2.0,
            ),
            Sense::CLICK,
        );
        let content = rect.shrink2(egui::vec2(ITEM_PADDING_X, ITEM_PADDING_Y));
        let icon = egui::Rect::from_center_size(
            egui::pos2(content.left() + icon_side / 2.0, rect.center().y),
            egui::Vec2::splat(icon_side),
        );
        let label_left = icon.right() + ITEM_GAP;
        let layout = RowLayout {
            response,
            icon,
            label_left,
        };
        if !ui.is_rect_visible(rect) {
            return layout;
        }
        if is_selected {
            ui.painter().rect(
                rect,
                ITEM_RADIUS,
                self.appearance.selected_background.gamma_multiply(opacity),
                Stroke::new(
                    BORDER_WIDTH,
                    self.appearance.selection_ring.gamma_multiply(opacity),
                ),
                egui::StrokeKind::Inside,
            );
        }
        if let Some(image) = self.icons.image(item.icon, muted) {
            image.paint_at(ui, icon);
        }
        let detail_top = content.top() + label.size().y;
        let detail_left = if inline_detail {
            label_left + label.size().x + ITEM_GAP
        } else {
            label_left
        };
        let detail_top = if inline_detail {
            content.center().y - DETAIL_LINE_HEIGHT / 2.0
        } else {
            detail_top
        };
        ui.painter()
            .galley(egui::pos2(label_left, content.top()), label, label_color);
        if let Some(detail) = detail {
            ui.painter()
                .galley(egui::pos2(detail_left, detail_top), detail, muted);
        }
        if let Some(shortcut) = shortcut {
            ui.painter().galley(
                egui::pos2(
                    content.right() - shortcut.size().x,
                    content.center().y - shortcut.size().y / 2.0,
                ),
                shortcut,
                muted,
            );
        }
        layout
    }

    fn reset_inspection(&mut self) {
        #[cfg(any(test, feature = "inspection"))]
        {
            self.inspection = Inspection::default();
        }
    }

    fn trace_dialog(&mut self, rect: egui::Rect) {
        #[cfg(any(test, feature = "inspection"))]
        {
            self.inspection.is_open = true;
            self.inspection.query = self.query.clone();
            self.inspection.dialog = Some(rect);
        }
        #[cfg(not(any(test, feature = "inspection")))]
        let _ = rect;
    }

    fn trace_input(&mut self, placeholder: &str, rect: egui::Rect) {
        #[cfg(any(test, feature = "inspection"))]
        {
            self.inspection.placeholder = placeholder.to_owned();
            self.inspection.input = Some(rect);
        }
        #[cfg(not(any(test, feature = "inspection")))]
        let _ = (placeholder, rect);
    }

    fn trace_input_layout(&mut self, icon: egui::Rect, field: egui::Rect) {
        #[cfg(any(test, feature = "inspection"))]
        {
            self.inspection.input_icon = Some(icon);
            self.inspection.input_field = Some(field);
        }
        #[cfg(not(any(test, feature = "inspection")))]
        let _ = (icon, field);
    }

    fn trace_refreshing(&mut self, spinner: egui::Rect, angle: f32) {
        #[cfg(any(test, feature = "inspection"))]
        {
            self.inspection.refreshing_icon = Some((spinner, angle));
        }
        #[cfg(not(any(test, feature = "inspection")))]
        let _ = (spinner, angle);
    }

    fn trace_focus(&mut self, has_input_focus: bool) {
        #[cfg(any(test, feature = "inspection"))]
        {
            self.inspection.has_input_focus = has_input_focus;
        }
        #[cfg(not(any(test, feature = "inspection")))]
        let _ = has_input_focus;
    }

    fn trace_row(
        &mut self,
        item: &Item,
        is_selected: bool,
        rect: egui::Rect,
        icon_rect: egui::Rect,
        label_left: f32,
    ) {
        #[cfg(any(test, feature = "inspection"))]
        self.inspection.rows.push(RowInspection {
            key: item.key.clone(),
            label: item.label.clone(),
            detail: item.detail.as_ref().map(|(detail, _)| detail.clone()),
            shortcut: item.shortcut.clone(),
            is_enabled: item.is_enabled,
            is_selected,
            rect,
            icon: item.icon,
            icon_rect,
            label_left,
        });
        #[cfg(not(any(test, feature = "inspection")))]
        let _ = (item, is_selected, rect, icon_rect, label_left);
    }

    fn trace_list(&mut self, listing: &Listing, rect: egui::Rect) {
        #[cfg(any(test, feature = "inspection"))]
        {
            let group = listing.group.as_ref();
            self.inspection.list = Some(rect);
            self.inspection.heading = group.and_then(|group| group.heading.clone());
            self.inspection.refreshing = group.and_then(|group| group.refreshing.clone());
            self.inspection.empty_message = listing
                .items()
                .is_empty()
                .then(|| listing.empty_message.clone());
        }
        #[cfg(not(any(test, feature = "inspection")))]
        let _ = (listing, rect);
    }
}

fn highlighted(
    ui: &Ui,
    text: &str,
    indices: &[usize],
    typography: &Typography,
    max_width: f32,
    is_truncated: bool,
) -> Arc<Galley> {
    let mut job = LayoutJob::default();
    for segment in highlight_segments(text, indices) {
        let (family, color) = if segment.is_matched {
            (crate::font_families::semibold(ui), typography.highlight)
        } else {
            (FontFamily::Proportional, typography.color)
        };
        job.append(
            segment.text,
            0.0,
            TextFormat {
                font_id: FontId::new(typography.size, family),
                line_height: Some(typography.line_height),
                color,
                ..Default::default()
            },
        );
    }
    job.wrap = if is_truncated {
        truncation(max_width)
    } else {
        TextWrapping {
            max_width,
            ..Default::default()
        }
    };
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

fn input_id(context: &egui::Context) -> Id {
    Id::new((INPUT_ID, context.viewport_id()))
}

fn dialog_width(viewport_width: f32) -> f32 {
    if viewport_width >= RESPONSIVE_BREAKPOINT {
        return MAX_WIDTH;
    }
    (viewport_width - SCREEN_MARGIN * 2.0).max(0.0)
}

fn truncation(max_width: f32) -> TextWrapping {
    TextWrapping {
        max_width,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some(ELLIPSIS),
    }
}

fn navigation(key: Key, modifiers: Modifiers) -> Option<Navigation> {
    let is_vim_binding = modifiers.ctrl && !modifiers.shift;
    let forward = if modifiers.mac_cmd {
        Navigation::Last
    } else {
        Navigation::Next
    };
    let backward = if modifiers.mac_cmd {
        Navigation::First
    } else {
        Navigation::Previous
    };
    match key {
        Key::ArrowDown => Some(forward),
        Key::N | Key::J if is_vim_binding => Some(forward),
        Key::ArrowUp => Some(backward),
        Key::P | Key::K if is_vim_binding => Some(backward),
        Key::Home => Some(Navigation::First),
        Key::End => Some(Navigation::Last),
        Key::Enter => Some(Navigation::Activate),
        Key::Escape => Some(Navigation::Dismiss),
        _ => None,
    }
}

fn take_navigation(context: &egui::Context) -> Vec<Navigation> {
    let mut keys = Vec::new();
    context.input_mut(|input| {
        input.events.retain(|event| {
            let Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = event
            else {
                return true;
            };
            match navigation(*key, *modifiers) {
                Some(step) => {
                    keys.push(step);
                    false
                }
                None => true,
            }
        })
    });
    keys
}

#[cfg(feature = "native-host")]
fn workspace_symbol_items(search_term: &str, symbols: WorkspaceSymbolIndex<'_>) -> Vec<Item> {
    symbols
        .entries
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(index, symbol)| Item {
            key: format!("workspace-symbol/{}/{index}", symbols.generation),
            action: Action::OpenWorkspaceSymbol {
                generation: symbols.generation,
                index,
            },
            icon: Icon::Hash,
            label: symbol.name.clone(),
            indices: crate::fuzzy_match::fuzzy_match(
                trim_workspace_query(search_term),
                &symbol.name,
            )
            .map_or_else(Vec::new, |matched| matched.indices),
            detail: (!symbol.container_name.is_empty())
                .then(|| (symbol.container_name.clone(), Vec::new())),
            shortcut: None,
            is_enabled: true,
            is_truncated: true,
        })
        .collect()
}

#[cfg(all(test, feature = "native-host"))]
#[path = "command-palette-tests.rs"]
mod tests;

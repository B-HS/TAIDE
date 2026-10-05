use std::collections::{HashMap, HashSet};

use egui::{self, Align, Color32, FontId, Id, Rect, RichText, Sense, Stroke, Ui};
use taide_model::{
    error::{AppError, AppResult},
    locale::{LocaleSummary, ResolvedLocale},
    settings::Settings,
    theme::{ResolvedTheme, ThemeSummary, ThemeType},
};
#[cfg(feature = "native-host")]
use taide_runtime::{AppState, locale_actions, theme_actions};

use crate::{
    icons::{Icon, Icons},
    presentation::{color, message},
    settings_controls::{Change, Numeric, NumericDraft, Position, Section, Switch},
    theme_draft::Mode,
    theme_editor::{Editor, icon_button},
};

#[path = "settings-code-view.rs"]
pub(crate) mod code_view;

pub const PAGE_X: i8 = 16;
const PAGE_Y: i8 = 32;
pub const TOC_WIDTH: f32 = 192.0;
const BODY_GAP: f32 = 32.0;
const SECTION_GAP: f32 = 24.0;
const CARD_PADDING: i8 = 20;
const CARD_GAP: f32 = 16.0;
const CONTROL_GAP: f32 = 12.0;
const PICKER_GAP: f32 = 8.0;
const POSITION_GAP: f32 = 4.0;
pub const POSITION_WIDTH: f32 = 160.0;
pub const POSITION_HEIGHT: f32 = 36.0;
const POSITION_COLUMNS: usize = 3;
const NUMERIC_WIDTH: f32 = 80.0;
const COMBO_HEIGHT: f32 = 192.0;
const COMBO_TRIGGER_HEIGHT: f32 = 32.0;
const TEXT_SIZE: f32 = 12.0;
const TITLE_SIZE: f32 = 18.0;
const SECTION_TITLE_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 16.0;
const THEME_BREAKPOINT: f32 = 640.0;
const THEME_COLUMNS_SMALL: usize = 2;
const THEME_COLUMNS_LARGE: usize = 3;
const THEME_HEIGHT: f32 = 50.0;
const ICON_BUTTON_SIZE: f32 = 24.0;
pub const SETTINGS_FILE_HEIGHT: f32 = 24.0;
const SETTINGS_FILE_PADDING: f32 = 6.0;
const SETTINGS_FILE_GAP: f32 = 4.0;
const CARD_DESCRIPTION_GAP: f32 = 6.0;
const ACTION_BUTTON_HEIGHT: f32 = 32.0;
const ACTION_BUTTON_PADDING_X: f32 = 12.0;
const ACTION_BUTTON_FONT: f32 = 14.0;
const SNIPPETS_FOLDER_HEIGHT: f32 = 24.0;
const SNIPPETS_FOLDER_PADDING_X: f32 = 6.0;
const CUSTOM_ROW_X: i8 = 8;
const CUSTOM_ROW_Y: i8 = 6;
const ROW_RADIUS: u8 = 6;
const CARD_RADIUS: u8 = 8;
const TOC_ROW_HEIGHT: f32 = 28.0;
const TOC_ROW_GAP: f32 = 2.0;
const SWITCH_WIDTH: f32 = 32.0;
const SWITCH_HEIGHT: f32 = 18.4;
const SWITCH_THUMB: f32 = 16.0;
const SWITCH_INSET: f32 = 1.0;
const CHECK_SIZE: f32 = 16.0;
const CHECK_STROKE: f32 = 2.0;
const HALF: f32 = 0.5;
const BORDER_WIDTH: f32 = 1.0;
const SELECTED_POSITION_OPACITY: f32 = 0.2;
const CHECK_POINTS: [[f32; 2]; 3] = [[0.2, 0.5], [0.4, 0.7], [0.8, 0.3]];
const CHEVRON_POINTS: [[f32; 2]; 3] = [[0.25, 0.375], [0.5, 0.625], [0.75, 0.375]];
const TAIL_VIEWPORT_FRACTION: f32 = 0.5;

enum ThemeGroup {
    Taide,
    Bundled,
    Custom,
}

pub use crate::settings_owner::Owner;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    owner: Owner,
    mount: u64,
    generation: u64,
}

pub struct Catalog {
    pub themes: AppResult<Vec<ThemeSummary>>,
    pub locales: AppResult<Vec<LocaleSummary>>,
}

#[cfg(feature = "native-host")]
impl Catalog {
    pub fn load(state: &AppState) -> Self {
        Self {
            themes: theme_actions::theme_list(state),
            locales: locale_actions::locale_list(state),
        }
    }
}

#[cfg(feature = "native-host")]
impl Request {
    pub fn is_active(&self, state: &AppState) -> bool {
        !state.is_shutting_down() && self.owner.is_active(state)
    }

    pub fn load(&self, state: &AppState) -> AppResult<Catalog> {
        if !self.is_active(state) {
            return Err(AppError::Forbidden(
                "native Settings owner is no longer active".into(),
            ));
        }
        Ok(Catalog::load(state))
    }
}

pub struct Appearance {
    background: Color32,
    card: Color32,
    foreground: Color32,
    muted: Color32,
    border: Color32,
    hover: Color32,
    active: Color32,
    focus: Color32,
    accent: Color32,
    input: Color32,
    input_background: Color32,
    primary: Color32,
    editor: crate::theme_editor::Appearance,
    code: code_view::Appearance,
    snippets: crate::snippet_editor::Appearance,
}

impl Appearance {
    #[cfg(any(test, feature = "inspection"))]
    pub fn inspection_background(&self) -> Color32 {
        self.background
    }

    pub fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        Ok(Self {
            background: color(theme, "app.background")?,
            card: color(theme, "panel.background")?,
            foreground: color(theme, "app.foreground")?,
            muted: color(theme, "appSidebar.iconDefault")?,
            border: color(theme, "app.border")?,
            hover: color(theme, "appSidebar.itemHover")?,
            active: color(theme, "appSidebar.itemActive")?,
            focus: color(theme, "app.focusBorder")?,
            accent: color(theme, "app.accent")?,
            input: color(theme, "input.border")?,
            input_background: color(theme, "input.background")?,
            primary: color(theme, "button.primaryBackground")?,
            editor: crate::theme_editor::Appearance::new(theme)?,
            code: code_view::Appearance::new(theme)?,
            snippets: crate::snippet_editor::Appearance::new(theme)?,
        })
    }
}

#[derive(Default)]
pub struct Views {
    entries: HashMap<Owner, View>,
    seen: HashSet<Owner>,
    resource_seen: HashSet<Owner>,
    mount: u64,
    icons: Option<Icons>,
    theme_revision: u64,
    preview_sequence: u64,
    resource_cache: crate::settings_resources::Cache,
    font_previews: crate::font_preview::Previews,
    snippet_catalog: crate::snippet_catalog::Catalog,
}

mod retained {
    use super::*;
    pub struct View {
        pub mount: u64,
        pub generation: u64,
        pub attempted: bool,
        pub pending: Option<Request>,
        pub catalog: Option<Catalog>,
        pub resources: crate::settings_resources::Resources,
        pub resources_mounted: bool,
        pub code: code_view::State,
        pub active: Section,
        pub resizer: NumericDraft,
        pub debounce: NumericDraft,
        pub language_open: bool,
        pub language_cursor: usize,
        pub editor: Option<Editor>,
        pub snippets: Option<crate::snippet_editor::Editor>,
        pub reset_scroll: bool,
        pub viewport: egui::ViewportId,
        pub preview: Option<ResolvedTheme>,
        pub preview_sequence: u64,
    }
}
use retained::View;
#[cfg(feature = "inspection")]
pub use retained::View as InspectionView;

impl Views {
    #[cfg(feature = "native-host")]
    pub fn preview(&self, viewport: egui::ViewportId, state: &AppState) -> Option<&ResolvedTheme> {
        self.preview_where(viewport, |owner| owner.is_active(state))
    }

    pub fn preview_where(
        &self,
        viewport: egui::ViewportId,
        mut is_active: impl FnMut(&Owner) -> bool,
    ) -> Option<&ResolvedTheme> {
        self.entries
            .iter()
            .filter(|(owner, view)| {
                view.viewport == viewport
                    && view.editor.is_some()
                    && view.preview.is_some()
                    && is_active(owner)
            })
            .max_by_key(|(_, view)| view.preview_sequence)
            .and_then(|(_, view)| view.preview.as_ref())
    }

    #[cfg(feature = "inspection")]
    pub fn inspection(&self) -> &HashMap<Owner, InspectionView> {
        &self.entries
    }

    #[cfg(feature = "inspection")]
    pub fn inspection_mut(&mut self) -> &mut HashMap<Owner, InspectionView> {
        &mut self.entries
    }

    pub fn begin_frame(&mut self) {
        self.seen.clear();
        self.resource_seen.clear();
    }

    pub fn finish_frame(&mut self) {
        self.entries.retain(|owner, _| self.seen.contains(owner));
        if self.entries.is_empty() {
            self.font_previews.clear();
        }
        if self.resource_seen.is_empty() {
            self.resource_cache.unobserve(web_time::Instant::now());
        }
    }

    pub fn clear(&mut self) {
        self.font_previews.clear();
        self.entries.clear();
        self.seen.clear();
        self.resource_seen.clear();
        self.resource_cache = crate::settings_resources::Cache::default();
        self.snippet_catalog = crate::snippet_catalog::Catalog::default();
    }

    pub fn observe_theme_revision(&mut self, revision: u64) {
        if self.theme_revision == revision {
            return;
        }
        self.theme_revision = revision;
        self.invalidate();
    }

    fn invalidate(&mut self) {
        for view in self.entries.values_mut() {
            view.attempted = false;
            view.pending = None;
            view.catalog = None;
        }
    }

    pub fn invalidate_catalog(&mut self) {
        self.invalidate();
    }

    #[cfg(target_arch = "wasm32")]
    pub fn set_font_painter(&mut self, painter: impl crate::font_preview::Painter + 'static) {
        self.font_previews.set_painter(painter);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_font_painter(
        &mut self,
        painter: impl crate::font_preview::Painter + Send + 'static,
    ) {
        self.font_previews.set_painter(painter);
    }

    pub fn refresh_resources(&mut self) {
        self.resource_cache.refresh();
        for view in self.entries.values_mut() {
            view.resources.refresh();
        }
    }

    pub fn accept_resource(&mut self, reply: crate::settings_resources::Reply) -> bool {
        if !self.resource_cache.accept(reply, web_time::Instant::now()) {
            return false;
        }
        for view in self.entries.values_mut() {
            view.resources = self.resource_cache.snapshot();
        }
        true
    }

    pub fn accept_theme(&mut self, reply: crate::theme_edit::Reply) -> Option<AppError> {
        let owner = reply.owner().clone();
        let view = self.entries.get_mut(&owner)?;
        let accepted = view.editor.as_mut()?.accept(reply)?;
        if accepted.close {
            view.editor = None;
            view.preview = None;
        }
        if accepted.mutated {
            self.invalidate();
        }
        accepted.error
    }

    pub fn accept_snippet(&mut self, reply: crate::snippet_edit::Reply) -> bool {
        if matches!(
            &reply.result,
            Ok(crate::snippet_edit::Outcome::Saved(_) | crate::snippet_edit::Outcome::Deleted)
        ) {
            self.snippet_catalog.invalidate();
            for view in self.entries.values_mut() {
                if let Some(editor) = view.snippets.as_mut() {
                    editor.reload();
                }
            }
        }
        let Some(view) = self.entries.get_mut(reply.request.owner()) else {
            return false;
        };
        view.snippets
            .as_mut()
            .is_some_and(|editor| editor.accept(reply))
    }

    pub fn snippet_catalog(&self) -> &crate::snippet_catalog::Catalog {
        &self.snippet_catalog
    }

    pub fn take_snippet_notices(&mut self) -> Vec<crate::snippet_editor::Notice> {
        self.entries
            .values_mut()
            .filter_map(|view| view.snippets.as_mut())
            .flat_map(|editor| editor.take_notices())
            .collect()
    }

    pub fn observe_snippet_list(
        &mut self,
        request: crate::snippet_edit::Request,
        now: web_time::Instant,
    ) {
        self.snippet_catalog.observe(request, now);
        self.accept_snippet_observers();
    }

    pub fn next_snippet_read(&mut self) -> Option<crate::snippet_catalog::Request> {
        self.snippet_catalog.next_read()
    }

    pub fn accept_snippet_catalog(
        &mut self,
        reply: crate::snippet_catalog::Reply,
        now: web_time::Instant,
    ) -> bool {
        let successful = reply.result.is_ok();
        if !self.snippet_catalog.accept(reply, now) {
            return false;
        }
        self.accept_snippet_observers();
        if successful {
            for view in self.entries.values_mut() {
                if let Some(editor) = view.snippets.as_mut() {
                    editor.update_catalog(self.snippet_catalog.files());
                }
            }
        }
        true
    }

    fn accept_snippet_observers(&mut self) {
        for reply in self.snippet_catalog.take_finished() {
            self.accept_snippet(reply);
        }
    }

    pub fn reconnect_snippets(&mut self, now: web_time::Instant) {
        self.snippet_catalog.reconnect(now);
    }

    pub fn disconnect_snippets(&mut self) {
        self.snippet_catalog.disconnected();
        self.accept_snippet_observers();
    }

    pub fn refresh_snippets(&mut self) {
        self.snippet_catalog.invalidate();
    }

    pub fn accept(&mut self, request: &Request, result: AppResult<Catalog>) -> bool {
        let Some(view) = self.entries.get_mut(&request.owner) else {
            return false;
        };
        if view.pending.as_ref() != Some(request) {
            return false;
        }
        view.pending = None;
        view.catalog = Some(match result {
            Ok(catalog) => catalog,
            Err(error) => Catalog {
                themes: Err(AppError::Internal(error.to_string())),
                locales: Err(error),
            },
        });
        true
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        owner: Owner,
        settings: &Settings,
        locale: &ResolvedLocale,
        appearance: &Appearance,
    ) -> Output {
        self.seen.insert(owner.clone());
        if self.icons.is_none() {
            match Icons::new() {
                Ok(icons) => self.icons = Some(icons),
                Err(error) => {
                    return Output {
                        error: Some(error),
                        ..Default::default()
                    };
                }
            }
        }
        let icons = self
            .icons
            .as_mut()
            .expect("native Settings icons are initialized");
        if let Err(error) = icons.prepare(ui.ctx()) {
            return Output {
                error: Some(error),
                ..Default::default()
            };
        }
        let new_mount = !self.entries.contains_key(&owner);
        if new_mount {
            let Some(mount) = self.mount.checked_add(1) else {
                return Output {
                    error: Some(AppError::Internal("native Settings mount exhausted".into())),
                    ..Default::default()
                };
            };
            self.mount = mount;
            self.entries.insert(
                owner.clone(),
                View {
                    mount,
                    generation: 0,
                    attempted: false,
                    pending: None,
                    catalog: None,
                    resources: self.resource_cache.snapshot(),
                    resources_mounted: false,
                    code: code_view::State::new(settings),
                    active: Section::Appearance,
                    resizer: NumericDraft::new(settings.resizer_thickness),
                    debounce: NumericDraft::new(settings.search_on_type_debounce_ms),
                    language_open: false,
                    language_cursor: 0,
                    editor: None,
                    snippets: None,
                    reset_scroll: false,
                    viewport: ui.ctx().viewport_id(),
                    preview: None,
                    preview_sequence: 0,
                },
            );
        }
        let view = self.entries.get_mut(&owner).expect("mounted Settings view");
        view.viewport = ui.ctx().viewport_id();
        let mut output = Output::default();
        if view.editor.is_none() && view.snippets.is_none() {
            self.resource_seen.insert(owner.clone());
            self.resource_cache
                .observe(web_time::Instant::now(), !view.resources_mounted);
            view.resources_mounted = true;
            if ui.is_enabled() {
                output.resources = self.resource_cache.requests(&owner);
            }
        } else {
            view.resources_mounted = false;
        }
        view.resources = self.resource_cache.snapshot();
        if !view.attempted && ui.is_enabled() {
            let Some(generation) = view.generation.checked_add(1) else {
                return Output {
                    error: Some(AppError::Internal(
                        "native Settings catalog generation exhausted".into(),
                    )),
                    ..Default::default()
                };
            };
            view.generation = generation;
            let request = Request {
                owner: owner.clone(),
                mount: view.mount,
                generation,
            };
            view.attempted = true;
            view.pending = Some(request.clone());
            output.load = Some(request);
        }
        ui.push_id((&owner, view.mount), |ui| {
            if let Some(editor) = view.editor.as_mut() {
                let result = ui
                    .push_id("theme-editor", |ui| {
                        editor.show(ui, locale, &appearance.editor, icons)
                    })
                    .inner;
                output.themes.extend(result.commands);
                #[cfg(any(test, feature = "inspection"))]
                output.editor_traces.extend(result.traces);
                output.theme_error = result.error;
                if result.close {
                    view.editor = None;
                    view.preview = None;
                } else {
                    output.tooltips.extend(result.tooltips);
                    let preview = editor.preview();
                    if preview != view.preview {
                        if let Some(sequence) = self.preview_sequence.checked_add(1) {
                            self.preview_sequence = sequence;
                            view.preview_sequence = sequence;
                            view.preview = preview;
                        } else {
                            output.error = Some(AppError::Internal(
                                "native theme preview sequence exhausted".into(),
                            ));
                        }
                    }
                }
            } else if let Some(editor) = view.snippets.as_mut() {
                let result = ui
                    .push_id("snippet-editor", |ui| {
                        editor.show(ui, locale, appearance, &appearance.snippets, icons)
                    })
                    .inner;
                output.snippets.extend(result.requests);
                output.snippet_notices.extend(result.notices);
                output.tooltips.extend(result.tooltips);
                #[cfg(any(test, feature = "inspection"))]
                {
                    output.editor_traces.extend(result.traces);
                    output.snippet_interactions.extend(result.interactions);
                }
                if result.close {
                    view.snippets = None;
                    view.code = code_view::State::new(settings);
                    view.reset_scroll = true;
                }
            } else {
                view.show(
                    ui,
                    settings,
                    locale,
                    appearance,
                    icons,
                    &mut self.font_previews,
                    &mut output,
                );
            }
        });
        if output.open_snippets {
            view.snippets = Some(crate::snippet_editor::Editor::new(owner.clone()));
            view.code = code_view::State::new(settings);
            view.reset_scroll = true;
            view.language_open = false;
        }
        if let Some((source, mode, name)) = output.open.take() {
            match Editor::new(owner, source, mode, name) {
                Ok(editor) => {
                    view.editor = Some(editor);
                    view.code = code_view::State::new(settings);
                    view.reset_scroll = true;
                    view.language_open = false;
                }
                Err(error) => output.theme_error = Some(error),
            }
        }
        output
    }
}

#[derive(Default)]
pub struct Output {
    pub load: Option<Request>,
    pub resources: Vec<crate::settings_resources::Request>,
    pub changes: Vec<Change>,
    pub folders: Vec<taide_model::system::AppDataPathKind>,
    pub error: Option<AppError>,
    pub themes: Vec<crate::theme_edit::Command>,
    pub snippets: Vec<crate::snippet_edit::Request>,
    pub snippet_notices: Vec<crate::snippet_editor::Notice>,
    pub theme_error: Option<AppError>,
    pub open_settings_file: bool,
    pub open_keybindings: bool,
    pub tooltips: Vec<crate::tooltip_trigger::Trigger>,
    open: Option<(String, Mode, String)>,
    open_snippets: bool,
    #[cfg(any(test, feature = "inspection"))]
    pub traces: Vec<Trace>,
    #[cfg(any(test, feature = "inspection"))]
    pub popup_traces: Vec<PopupTrace>,
    #[cfg(any(test, feature = "inspection"))]
    pub editor_traces: Vec<(String, Id, Rect)>,
    #[cfg(any(test, feature = "inspection"))]
    pub snippet_interactions: HashMap<Id, Rect>,
    #[cfg(any(test, feature = "inspection"))]
    pub tooltip_traces: Vec<(String, Id, Rect)>,
    #[cfg(any(test, feature = "inspection"))]
    pub scroll: Option<(Id, egui::Vec2, Rect)>,
}

impl Output {
    pub fn take_tooltips(&mut self) -> Vec<crate::tooltip_trigger::Trigger> {
        #[cfg(any(test, feature = "inspection"))]
        {
            self.tooltip_traces = self
                .tooltips
                .iter()
                .map(|trigger| {
                    (
                        trigger.label.clone(),
                        trigger.response.id,
                        trigger.response.rect,
                    )
                })
                .collect();
        }
        std::mem::take(&mut self.tooltips)
    }
}

#[cfg(any(test, feature = "inspection"))]
pub struct PopupTrace {
    pub field: &'static str,
    pub open: bool,
    pub opacity: f32,
    pub scale: f32,
    pub active: bool,
}

#[cfg(any(test, feature = "inspection"))]
pub struct Trace {
    pub field: &'static str,
    pub id: Id,
    pub rect: Rect,
    pub has_focus: bool,
    pub lost_focus: bool,
    pub clicked: bool,
    pub interact_rect: Rect,
    pub enabled: bool,
}

#[cfg(any(test, feature = "inspection"))]
impl std::fmt::Debug for Trace {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple(self.field)
            .field(&self.id.value())
            .field(&self.rect)
            .field(&self.has_focus)
            .field(&self.lost_focus)
            .field(&self.clicked)
            .finish()
    }
}

#[cfg(any(test, feature = "inspection"))]
impl Trace {
    fn capture(field: &'static str, response: &egui::Response) -> Self {
        Self {
            field,
            id: response.id,
            rect: response.rect,
            has_focus: response.has_focus(),
            lost_focus: response.lost_focus(),
            clicked: response.clicked(),
            interact_rect: response.interact_rect,
            enabled: response.enabled(),
        }
    }
}

impl View {
    fn show(
        &mut self,
        ui: &mut Ui,
        settings: &Settings,
        locale: &ResolvedLocale,
        appearance: &Appearance,
        icons: &Icons,
        font_previews: &mut crate::font_preview::Previews,
        output: &mut Output,
    ) {
        let container = ui.available_rect_before_wrap();
        egui::Frame::NONE
            .fill(appearance.background)
            .show(ui, |ui| {
                ui.set_min_size(container.size());
                ui.visuals_mut().override_text_color = Some(appearance.foreground);
                ui.visuals_mut().extreme_bg_color = appearance.input_background;
                ui.visuals_mut().selection.bg_fill = appearance.active;
                ui.visuals_mut().selection.stroke = Stroke::new(BORDER_WIDTH, appearance.accent);
                ui.spacing_mut().item_spacing.x = CONTROL_GAP;
                ui.visuals_mut().widgets.inactive.bg_fill = appearance.background;
                ui.visuals_mut().widgets.inactive.weak_bg_fill = appearance.background;
                ui.visuals_mut().widgets.inactive.bg_stroke =
                    Stroke::new(BORDER_WIDTH, appearance.border);
                ui.visuals_mut().widgets.hovered.bg_fill = appearance.hover;
                ui.visuals_mut().widgets.hovered.weak_bg_fill = appearance.hover;
                ui.visuals_mut().widgets.active.bg_fill = appearance.active;
                ui.visuals_mut().widgets.active.weak_bg_fill = appearance.active;
                let mut scroll = egui::ScrollArea::vertical()
                    .id_salt("settings-scroll")
                    .auto_shrink([false, false]);
                if self.reset_scroll {
                    scroll = scroll.vertical_scroll_offset(0.0);
                    self.reset_scroll = false;
                }
                let _scroll = scroll.show_viewport(ui, |ui, viewport| {
                    egui::Frame::NONE
                        .inner_margin(egui::Margin::symmetric(PAGE_X, PAGE_Y))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(BODY_GAP, SECTION_GAP);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(message(locale, "settings.title", &[]))
                                        .size(TITLE_SIZE)
                                        .strong()
                                        .color(appearance.foreground),
                                );
                                ui.allocate_ui_with_layout(
                                    egui::vec2(ui.available_width(), SETTINGS_FILE_HEIGHT),
                                    egui::Layout::right_to_left(Align::Center),
                                    |ui| {
                                        ui.spacing_mut().button_padding =
                                            egui::vec2(SETTINGS_FILE_PADDING, 0.0);
                                        ui.spacing_mut().icon_spacing = SETTINGS_FILE_GAP;
                                        let text = RichText::new(message(
                                            locale,
                                            "app.openSettingsFile",
                                            &[],
                                        ))
                                        .size(TEXT_SIZE)
                                        .color(appearance.foreground);
                                        let button = if let Some(image) =
                                            icons.image(Icon::FileJson, appearance.foreground)
                                        {
                                            egui::Button::image_and_text(image, text)
                                        } else {
                                            egui::Button::new(text)
                                        }
                                        .stroke(Stroke::new(BORDER_WIDTH, appearance.border))
                                        .corner_radius(ROW_RADIUS)
                                        .min_size(egui::vec2(0.0, SETTINGS_FILE_HEIGHT));
                                        let response = ui.add(button);
                                        output.open_settings_file = response.clicked();
                                        #[cfg(any(test, feature = "inspection"))]
                                        output
                                            .traces
                                            .push(Trace::capture("open-settings-file", &response));
                                    },
                                );
                            });
                            let origin = ui.next_widget_position();
                            let toc_top = origin.y.max(ui.clip_rect().top() + f32::from(PAGE_Y));
                            let toc_rect = Rect::from_min_size(
                                egui::pos2(origin.x, toc_top),
                                egui::vec2(TOC_WIDTH, 0.0),
                            );
                            let mut toc = ui.new_child(
                                egui::UiBuilder::new()
                                    .id_salt("toc")
                                    .max_rect(toc_rect)
                                    .layout(egui::Layout::top_down(Align::Min)),
                            );
                            toc.spacing_mut().item_spacing.y = TOC_ROW_GAP;
                            let mut scroll = None;
                            for section in Section::BASIC {
                                let selected = self.active == section;
                                let text = message(locale, section.title(), &[]);
                                let button = egui::Button::new(
                                    RichText::new(&text).size(TEXT_SIZE).color(if selected {
                                        appearance.foreground
                                    } else {
                                        appearance.muted
                                    }),
                                )
                                .fill(if selected {
                                    appearance.active
                                } else {
                                    Color32::TRANSPARENT
                                })
                                .stroke(Stroke::NONE)
                                .corner_radius(ROW_RADIUS);
                                let response = toc.add_sized([TOC_WIDTH, TOC_ROW_HEIGHT], button);
                                #[cfg(any(test, feature = "inspection"))]
                                output
                                    .traces
                                    .push(Trace::capture(section.title(), &response));
                                if response.clicked() {
                                    self.active = section;
                                    scroll = Some(section);
                                }
                            }
                            let content_left = origin.x + TOC_WIDTH + BODY_GAP;
                            let content_rect = Rect::from_min_max(
                                egui::pos2(content_left, origin.y),
                                egui::pos2(ui.max_rect().right().max(content_left), f32::INFINITY),
                            );
                            let content = ui.scope_builder(
                                egui::UiBuilder::new()
                                    .id_salt("sections")
                                    .max_rect(content_rect)
                                    .layout(egui::Layout::top_down(Align::Min)),
                                |ui| {
                                    ui.spacing_mut().item_spacing.y = SECTION_GAP;
                                    let width = ui.available_width();
                                    for section in Section::BASIC {
                                        let card = egui::Frame::NONE
                                            .fill(appearance.card)
                                            .stroke(Stroke::new(1.0, appearance.border))
                                            .corner_radius(CARD_RADIUS)
                                            .inner_margin(CARD_PADDING)
                                            .show(ui, |ui| {
                                                ui.set_min_width(
                                                    (width
                                                        - f32::from(CARD_PADDING) / HALF
                                                        - BORDER_WIDTH / HALF)
                                                        .max(0.0),
                                                );
                                                ui.spacing_mut().item_spacing.y = CARD_GAP;
                                                let title = RichText::new(message(
                                                    locale,
                                                    section.title(),
                                                    &[],
                                                ))
                                                .size(SECTION_TITLE_SIZE)
                                                .strong()
                                                .color(appearance.foreground);
                                                if section == Section::Keymap {
                                                    ui.scope(|ui| {
                                                        ui.spacing_mut().item_spacing.y =
                                                            CARD_DESCRIPTION_GAP;
                                                        ui.label(title);
                                                        ui.label(
                                                            RichText::new(message(
                                                                locale,
                                                                "settings.keymapDescription",
                                                                &[],
                                                            ))
                                                            .size(TEXT_SIZE)
                                                            .color(appearance.muted),
                                                        );
                                                    });
                                                } else {
                                                    ui.label(title);
                                                }
                                                ui.scope(|ui| {
                                                    ui.spacing_mut().item_spacing =
                                                        egui::Vec2::splat(CONTROL_GAP);
                                                    match section {
                                                        Section::Appearance => self.appearance(
                                                            ui, settings, locale, appearance,
                                                            icons, output,
                                                        ),
                                                        Section::Language => self.language(
                                                            ui, settings, locale, appearance,
                                                            output,
                                                        ),
                                                        Section::Interface => self.interface(
                                                            ui, settings, locale, appearance,
                                                            output,
                                                        ),
                                                        Section::Notifications => {
                                                            for field in Switch::ALL
                                                                .into_iter()
                                                                .filter(|field| {
                                                                    field.section() == section
                                                                })
                                                            {
                                                                switch(
                                                                    ui, field, settings, locale,
                                                                    appearance, output,
                                                                );
                                                            }
                                                        }
                                                        Section::Editor | Section::Terminal => {
                                                            self.code.show(
                                                                ui,
                                                                section,
                                                                settings,
                                                                locale,
                                                                appearance,
                                                                &self.resources,
                                                                font_previews,
                                                                output,
                                                            );
                                                        }
                                                        Section::Keymap => {
                                                            keymap(ui, locale, appearance, output);
                                                        }
                                                        Section::Snippets => {
                                                            snippets(
                                                                ui, locale, appearance, icons,
                                                                output,
                                                            );
                                                        }
                                                    }
                                                });
                                            });
                                        if scroll == Some(section) {
                                            ui.scroll_to_rect(
                                                Rect::from_min_size(
                                                    card.response.rect.left_top()
                                                        - egui::vec2(0.0, f32::from(PAGE_Y)),
                                                    egui::Vec2::ZERO,
                                                ),
                                                Some(Align::TOP),
                                            );
                                        }
                                    }
                                    ui.add_space(viewport.height() * TAIL_VIEWPORT_FRACTION);
                                },
                            );
                            ui.expand_to_include_rect(content.response.rect);
                        });
                });
                #[cfg(any(test, feature = "inspection"))]
                {
                    output.scroll = Some((_scroll.id, _scroll.state.offset, _scroll.inner_rect));
                }
            });
    }

    fn appearance(
        &self,
        ui: &mut Ui,
        settings: &Settings,
        locale: &ResolvedLocale,
        appearance: &Appearance,
        icons: &Icons,
        output: &mut Output,
    ) {
        match self.catalog.as_ref().map(|catalog| &catalog.themes) {
            None => {
                ui.label(
                    RichText::new(message(locale, "settings.loading", &[]))
                        .size(TEXT_SIZE)
                        .color(appearance.muted),
                );
            }
            Some(Err(error)) => {
                ui.label(
                    RichText::new(error.to_string())
                        .size(TEXT_SIZE)
                        .color(appearance.muted),
                );
            }
            Some(Ok(themes)) => {
                let columns = if ui.ctx().viewport_rect().width() >= THEME_BREAKPOINT {
                    THEME_COLUMNS_LARGE
                } else {
                    THEME_COLUMNS_SMALL
                };
                for (group, title) in [
                    (ThemeGroup::Taide, "settings.builtinThemesSection"),
                    (ThemeGroup::Bundled, "settings.bundledThemesSection"),
                    (ThemeGroup::Custom, "themeEditor.customThemes"),
                ] {
                    let themes = themes
                        .iter()
                        .filter(|theme| {
                            let builtin = matches!(theme.id.as_str(), "taide-dark" | "taide-light");
                            match group {
                                ThemeGroup::Taide => theme.builtin && builtin,
                                ThemeGroup::Bundled => theme.builtin && !builtin,
                                ThemeGroup::Custom => !theme.builtin,
                            }
                        })
                        .collect::<Vec<_>>();
                    if themes.is_empty() {
                        continue;
                    }
                    ui.scope(|ui| {
                        ui.spacing_mut().item_spacing = egui::Vec2::splat(PICKER_GAP);
                        ui.label(
                            RichText::new(message(locale, title, &[]))
                                .size(TEXT_SIZE)
                                .color(appearance.muted),
                        );
                        let width = ((ui.available_width() - PICKER_GAP * (columns - 1) as f32)
                            / columns as f32)
                            .max(0.0);
                        for row in themes.chunks(columns) {
                            ui.horizontal(|ui| {
                                for theme in row {
                                    ui.push_id(&theme.id, |ui| {
                                        let selected = theme.id == settings.theme_id;
                                        let (rect, wrapper) = ui.allocate_exact_size(
                                            egui::vec2(width, THEME_HEIGHT),
                                            Sense::hover(),
                                        );
                                        let duplicate = !matches!(group, ThemeGroup::Taide);
                                        let action_width = if duplicate {
                                            ICON_BUTTON_SIZE + POSITION_GAP
                                        } else {
                                            0.0
                                        } + if selected {
                                            CHECK_SIZE + POSITION_GAP
                                        } else {
                                            0.0
                                        };
                                        let select_rect = Rect::from_min_max(
                                            rect.min + egui::vec2(CONTROL_GAP, PICKER_GAP),
                                            egui::pos2(
                                                (rect.right() - CONTROL_GAP - action_width)
                                                    .max(rect.left()),
                                                rect.bottom() - PICKER_GAP,
                                            ),
                                        );
                                        let response = ui.interact(
                                            select_rect,
                                            ui.make_persistent_id("select"),
                                            Sense::click(),
                                        );
                                        response.widget_info(|| {
                                            egui::WidgetInfo::selected(
                                                egui::WidgetType::SelectableLabel,
                                                ui.is_enabled(),
                                                selected,
                                                &theme.name,
                                            )
                                        });
                                        ui.painter().rect(
                                            rect,
                                            ROW_RADIUS,
                                            if selected {
                                                appearance.active
                                            } else if wrapper.hovered() {
                                                appearance.hover
                                            } else {
                                                Color32::TRANSPARENT
                                            },
                                            Stroke::new(
                                                1.0,
                                                if selected || response.has_focus() {
                                                    appearance.focus
                                                } else {
                                                    appearance.border
                                                },
                                            ),
                                            egui::StrokeKind::Inside,
                                        );
                                        let inset = egui::vec2(CONTROL_GAP, PICKER_GAP);
                                        let text_width =
                                            (width - CONTROL_GAP / HALF - action_width).max(0.0);
                                        text(
                                            ui,
                                            &theme.name,
                                            rect.min + inset,
                                            text_width,
                                            appearance.foreground,
                                        );
                                        let kind = match theme.theme_type {
                                            ThemeType::Dark => "settings.themeDark",
                                            ThemeType::Light => "settings.themeLight",
                                        };
                                        text(
                                            ui,
                                            &message(locale, kind, &[]),
                                            rect.min + inset + egui::vec2(0.0, LINE_HEIGHT),
                                            text_width,
                                            appearance.muted,
                                        );
                                        if selected {
                                            check(
                                                ui,
                                                Rect::from_center_size(
                                                    egui::pos2(
                                                        rect.right()
                                                            - CONTROL_GAP
                                                            - CHECK_SIZE * HALF,
                                                        rect.center().y,
                                                    ),
                                                    egui::Vec2::splat(CHECK_SIZE),
                                                ),
                                                appearance.accent,
                                            );
                                        }
                                        if response.clicked() {
                                            response.request_focus();
                                            output.changes.push(Change::Theme(theme.id.clone()));
                                        }
                                        if duplicate {
                                            let left = rect.right() - CONTROL_GAP - action_width;
                                            let button = Rect::from_center_size(
                                                egui::pos2(
                                                    left + ICON_BUTTON_SIZE * HALF,
                                                    rect.center().y,
                                                ),
                                                egui::Vec2::splat(ICON_BUTTON_SIZE),
                                            );
                                            ui.scope_builder(
                                                egui::UiBuilder::new()
                                                    .id_salt("duplicate")
                                                    .max_rect(button),
                                                |ui| {
                                                    let response = icon_button(
                                                        ui,
                                                        icons,
                                                        Icon::Copy,
                                                        appearance.muted,
                                                        &message(
                                                            locale,
                                                            "themeEditor.duplicateTheme",
                                                            &[],
                                                        ),
                                                        false,
                                                        &mut output.tooltips,
                                                    );
                                                    if response.clicked() {
                                                        output.open = Some((
                                                            theme.id.clone(),
                                                            Mode::Create,
                                                            message(
                                                                locale,
                                                                "themeEditor.duplicateNameTemplate",
                                                                &[("name", &theme.name)],
                                                            ),
                                                        ));
                                                    }
                                                },
                                            );
                                        }
                                    });
                                }
                            });
                        }
                    });
                }
            }
        }
        switch(
            ui,
            Switch::FollowSystemTheme,
            settings,
            locale,
            appearance,
            output,
        );
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(message(locale, "themeEditor.customThemes", &[]))
                    .size(TEXT_SIZE)
                    .color(appearance.muted),
            );
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                let is_loaded = matches!(
                    self.catalog.as_ref().map(|catalog| &catalog.themes),
                    Some(Ok(_))
                );
                let response = ui.add_enabled(
                    is_loaded,
                    egui::Button::new(
                        RichText::new(message(locale, "themeEditor.createNew", &[]))
                            .size(TEXT_SIZE),
                    ),
                );
                #[cfg(any(test, feature = "inspection"))]
                output
                    .traces
                    .push(Trace::capture("create-theme", &response));
                if response.clicked()
                    && let Some(Ok(themes)) = self.catalog.as_ref().map(|catalog| &catalog.themes)
                {
                    let name = themes
                        .iter()
                        .find(|theme| theme.id == settings.theme_id)
                        .map_or(settings.theme_id.as_str(), |theme| theme.name.as_str());
                    output.open = Some((
                        settings.theme_id.clone(),
                        Mode::Create,
                        message(
                            locale,
                            "themeEditor.duplicateNameTemplate",
                            &[("name", name)],
                        ),
                    ));
                }
                let response = ui.button(
                    RichText::new(message(locale, "settings.themesOpenFolder", &[]))
                        .size(TEXT_SIZE),
                );
                #[cfg(any(test, feature = "inspection"))]
                output
                    .traces
                    .push(Trace::capture("themes-folder", &response));
                if response.clicked() {
                    output
                        .folders
                        .push(taide_model::system::AppDataPathKind::Themes);
                }
            });
        });
        if let Some(Ok(themes)) = self.catalog.as_ref().map(|catalog| &catalog.themes) {
            let custom = themes
                .iter()
                .filter(|theme| !theme.builtin)
                .collect::<Vec<_>>();
            if custom.is_empty() {
                ui.label(
                    RichText::new(message(locale, "themeEditor.noCustomThemes", &[]))
                        .size(TEXT_SIZE)
                        .color(appearance.muted),
                );
            }
            for theme in custom {
                ui.push_id(("custom-theme", &theme.id), |ui| {
                    egui::Frame::NONE
                        .stroke(Stroke::new(BORDER_WIDTH, appearance.border))
                        .corner_radius(ROW_RADIUS)
                        .inner_margin(egui::Margin::symmetric(CUSTOM_ROW_X, CUSTOM_ROW_Y))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = PICKER_GAP;
                                let name_width = (ui.available_width()
                                    - ICON_BUTTON_SIZE / HALF
                                    - PICKER_GAP / HALF)
                                    .max(0.0);
                                ui.add_sized(
                                    [name_width, ICON_BUTTON_SIZE],
                                    egui::Label::new(RichText::new(&theme.name).size(TEXT_SIZE))
                                        .truncate(),
                                );
                                let response = icon_button(
                                    ui,
                                    icons,
                                    Icon::Copy,
                                    appearance.muted,
                                    &message(locale, "themeEditor.duplicateTheme", &[]),
                                    false,
                                    &mut output.tooltips,
                                );
                                #[cfg(any(test, feature = "inspection"))]
                                output
                                    .traces
                                    .push(Trace::capture("duplicate-custom-theme", &response));
                                if response.clicked() {
                                    output.open = Some((
                                        theme.id.clone(),
                                        Mode::Create,
                                        message(
                                            locale,
                                            "themeEditor.duplicateNameTemplate",
                                            &[("name", &theme.name)],
                                        ),
                                    ));
                                }
                                let response = icon_button(
                                    ui,
                                    icons,
                                    Icon::Pencil,
                                    appearance.muted,
                                    &message(locale, "themeEditor.editTheme", &[]),
                                    false,
                                    &mut output.tooltips,
                                );
                                #[cfg(any(test, feature = "inspection"))]
                                output
                                    .traces
                                    .push(Trace::capture("edit-custom-theme", &response));
                                if response.clicked() {
                                    output.open =
                                        Some((theme.id.clone(), Mode::Edit, String::new()));
                                }
                            });
                        });
                });
            }
        }
    }

    fn language(
        &mut self,
        ui: &mut Ui,
        settings: &Settings,
        locale: &ResolvedLocale,
        appearance: &Appearance,
        output: &mut Output,
    ) {
        match self.catalog.as_ref().map(|catalog| &catalog.locales) {
            None => {
                ui.label(
                    RichText::new(message(locale, "settings.loading", &[]))
                        .size(TEXT_SIZE)
                        .color(appearance.muted),
                );
            }
            Some(Err(error)) => {
                ui.label(
                    RichText::new(error.to_string())
                        .size(TEXT_SIZE)
                        .color(appearance.muted),
                );
            }
            Some(Ok(locales)) => {
                let system = message(locale, "settings.systemLanguage", &[]);
                let active = locales
                    .iter()
                    .find(|entry| entry.id == settings.language)
                    .map_or(system.as_str(), |entry| entry.name.as_str());
                let combo_id = ui.make_persistent_id("language");
                let enter_to_select = self.language_open
                    && ui.is_enabled()
                    && ui.input_mut(|input| {
                        input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                    });
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), COMBO_TRIGGER_HEIGHT),
                    Sense::hover(),
                );
                let trigger = ui.interact(rect, combo_id, Sense::click());
                #[cfg(any(test, feature = "inspection"))]
                output.traces.push(Trace::capture("language", &trigger));
                let label = message(locale, "settings.languageSelectPlaceholder", &[]);
                trigger.widget_info(|| {
                    let mut info = egui::WidgetInfo::labeled(
                        egui::WidgetType::ComboBox,
                        ui.is_enabled(),
                        &label,
                    );
                    info.current_text_value = Some(active.into());
                    info.selected = Some(self.language_open);
                    info
                });
                ui.painter().rect(
                    rect,
                    ROW_RADIUS,
                    if trigger.hovered() {
                        appearance.hover
                    } else {
                        appearance.background
                    },
                    Stroke::new(
                        BORDER_WIDTH,
                        if trigger.has_focus() {
                            appearance.focus
                        } else {
                            appearance.border
                        },
                    ),
                    egui::StrokeKind::Inside,
                );
                text(
                    ui,
                    active,
                    egui::pos2(
                        rect.left() + CONTROL_GAP,
                        rect.center().y - LINE_HEIGHT * HALF,
                    ),
                    (rect.width() - CONTROL_GAP / HALF - CHECK_SIZE).max(0.0),
                    appearance.foreground,
                );
                let icon = Rect::from_center_size(
                    egui::pos2(
                        rect.right() - CONTROL_GAP - CHECK_SIZE * HALF,
                        rect.center().y,
                    ),
                    egui::Vec2::splat(CHECK_SIZE),
                );
                ui.painter().add(egui::Shape::line(
                    CHEVRON_POINTS
                        .map(|[x, y]| icon.min + egui::vec2(icon.width() * x, icon.height() * y))
                        .to_vec(),
                    Stroke::new(CHECK_STROKE, appearance.muted),
                ));
                let opened_with_arrow = !self.language_open
                    && trigger.has_focus()
                    && ui.input_mut(|input| {
                        input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
                    });
                if trigger.clicked() || opened_with_arrow {
                    self.language_open = opened_with_arrow || !self.language_open;
                    self.language_cursor = 0;
                    trigger.request_focus();
                }
                let mut options = vec![("system", system.as_str())];
                options.extend(
                    locales
                        .iter()
                        .map(|entry| (entry.id.as_str(), entry.name.as_str())),
                );
                self.language_cursor = self.language_cursor.min(options.len() - 1);
                let mut selected = enter_to_select.then(|| options[self.language_cursor].0);
                if self.language_open && ui.is_enabled() {
                    if !opened_with_arrow
                        && ui.input_mut(|input| {
                            input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
                        })
                    {
                        self.language_cursor = (self.language_cursor + 1) % options.len();
                    }
                    if ui.input_mut(|input| {
                        input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)
                    }) {
                        self.language_cursor = self
                            .language_cursor
                            .checked_sub(1)
                            .unwrap_or(options.len() - 1);
                    }
                    if ui.input_mut(|input| {
                        input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                    }) {
                        selected = Some(options[self.language_cursor].0);
                    }
                }
                egui::Popup::from_response(&trigger)
                    .open_bool(&mut self.language_open)
                    .width(rect.width())
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                    .show(|ui| {
                        egui::ScrollArea::vertical()
                            .max_height(COMBO_HEIGHT)
                            .show(ui, |ui| {
                                for (index, (id, name)) in options.iter().enumerate() {
                                    let response = ui.add_sized(
                                        [ui.available_width(), TOC_ROW_HEIGHT],
                                        egui::Button::new(RichText::new(*name).size(TEXT_SIZE))
                                            .fill(if index == self.language_cursor {
                                                appearance.hover
                                            } else {
                                                Color32::TRANSPARENT
                                            })
                                            .stroke(Stroke::NONE),
                                    );
                                    if *id == settings.language {
                                        check(
                                            ui,
                                            Rect::from_center_size(
                                                egui::pos2(
                                                    response.rect.right() - CONTROL_GAP,
                                                    response.rect.center().y,
                                                ),
                                                egui::Vec2::splat(CHECK_SIZE),
                                            ),
                                            appearance.accent,
                                        );
                                    }
                                    if response.clicked() {
                                        selected = Some(*id);
                                    }
                                }
                            });
                    });
                if let Some(id) = selected {
                    output.changes.push(Change::Language(id.into()));
                    self.language_open = false;
                    trigger.request_focus();
                }
                ui.memory_mut(|memory| {
                    memory.set_focus_lock_filter(
                        combo_id,
                        egui::EventFilter {
                            vertical_arrows: true,
                            escape: self.language_open,
                            ..Default::default()
                        },
                    )
                });
            }
        }
        if ui
            .button(
                RichText::new(message(locale, "settings.localesOpenFolder", &[])).size(TEXT_SIZE),
            )
            .clicked()
        {
            output
                .folders
                .push(taide_model::system::AppDataPathKind::Locales);
        }
    }

    fn interface(
        &mut self,
        ui: &mut Ui,
        settings: &Settings,
        locale: &ResolvedLocale,
        appearance: &Appearance,
        output: &mut Output,
    ) {
        ui.label(
            RichText::new(message(locale, "settings.toastPosition", &[]))
                .size(TEXT_SIZE)
                .color(appearance.muted),
        );
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::splat(POSITION_GAP);
            let width = (POSITION_WIDTH - POSITION_GAP * (POSITION_COLUMNS - 1) as f32)
                / POSITION_COLUMNS as f32;
            for row in Position::ALL.chunks(POSITION_COLUMNS) {
                ui.horizontal(|ui| {
                    for position in row {
                        ui.push_id(position.value(), |ui| {
                            let label = message(locale, position.label(), &[]);
                            let selected = settings.toast_position == position.value();
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(width, POSITION_HEIGHT),
                                Sense::click(),
                            );
                            response.widget_info(|| {
                                egui::WidgetInfo::selected(
                                    egui::WidgetType::SelectableLabel,
                                    ui.is_enabled(),
                                    selected,
                                    &label,
                                )
                            });
                            ui.painter().rect(
                                rect,
                                ROW_RADIUS,
                                if selected {
                                    appearance.accent.gamma_multiply(SELECTED_POSITION_OPACITY)
                                } else if response.hovered() {
                                    appearance.hover
                                } else {
                                    Color32::TRANSPARENT
                                },
                                Stroke::new(
                                    1.0,
                                    if selected || response.has_focus() {
                                        appearance.focus
                                    } else {
                                        appearance.border
                                    },
                                ),
                                egui::StrokeKind::Inside,
                            );
                            if response.clicked() {
                                output.changes.push(Change::Position(*position));
                            }
                            output.tooltips.push(crate::tooltip_trigger::Trigger {
                                response,
                                label,
                                align: egui::RectAlign::TOP,
                                focus_target: None,
                            });
                        });
                    }
                });
            }
        });
        numeric(
            ui,
            Numeric::ResizerThickness,
            &mut self.resizer,
            settings,
            locale,
            appearance,
            output,
        );
        for field in Switch::ALL
            .into_iter()
            .filter(|field| field.section() == Section::Interface && *field != Switch::SearchOnType)
        {
            switch(ui, field, settings, locale, appearance, output);
        }
        ui.separator();
        ui.label(
            RichText::new(message(locale, "search.title", &[]))
                .size(TEXT_SIZE)
                .color(appearance.foreground),
        );
        switch(
            ui,
            Switch::SearchOnType,
            settings,
            locale,
            appearance,
            output,
        );
        numeric(
            ui,
            Numeric::SearchOnTypeDebounce,
            &mut self.debounce,
            settings,
            locale,
            appearance,
            output,
        );
        ui.label(
            RichText::new(message(
                locale,
                "settings.searchOnTypeDebounceMsDescription",
                &[],
            ))
            .size(TEXT_SIZE)
            .color(appearance.muted),
        );
    }
}

fn outline_buttons(ui: &mut Ui, appearance: &Appearance) {
    let visuals = ui.visuals_mut();
    visuals.override_text_color = None;
    visuals.widgets.inactive.weak_bg_fill = appearance.background;
    visuals.widgets.inactive.fg_stroke.color = appearance.foreground;
    for visual in [&mut visuals.widgets.hovered, &mut visuals.widgets.active] {
        visual.weak_bg_fill = appearance.code.trigger_hover;
        visual.fg_stroke.color = appearance.code.list_foreground;
    }
    for visual in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
    ] {
        visual.bg_stroke = Stroke::new(BORDER_WIDTH, appearance.border);
        visual.expansion = 0.0;
    }
}

fn snippets(
    ui: &mut Ui,
    locale: &ResolvedLocale,
    appearance: &Appearance,
    icons: &Icons,
    output: &mut Output,
) {
    outline_buttons(ui, appearance);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = PICKER_GAP;
        ui.spacing_mut().button_padding = egui::vec2(ACTION_BUTTON_PADDING_X, 0.0);
        let response = ui.add(
            egui::Button::new(
                RichText::new(message(locale, "settings.snippetsManage", &[])).font(FontId::new(
                    ACTION_BUTTON_FONT,
                    crate::font_families::medium(ui),
                )),
            )
            .corner_radius(ROW_RADIUS)
            .min_size(egui::vec2(0.0, ACTION_BUTTON_HEIGHT)),
        );
        output.open_snippets = response.clicked();
        #[cfg(any(test, feature = "inspection"))]
        output
            .traces
            .push(Trace::capture("settings.snippetsManage", &response));
        ui.spacing_mut().button_padding = egui::vec2(SNIPPETS_FOLDER_PADDING_X, 0.0);
        ui.spacing_mut().icon_spacing = SETTINGS_FILE_GAP;
        let response = ui.add(
            egui::Button::opt_image_and_text(
                icons.image(Icon::FolderOpen, appearance.foreground),
                Some(
                    RichText::new(message(locale, "settings.snippetsOpenFolder", &[]))
                        .font(FontId::new(TEXT_SIZE, crate::font_families::medium(ui)))
                        .into(),
                ),
            )
            .corner_radius(ROW_RADIUS)
            .min_size(egui::vec2(0.0, SNIPPETS_FOLDER_HEIGHT)),
        );
        if response.clicked() {
            output
                .folders
                .push(taide_model::system::AppDataPathKind::Snippets);
        }
        #[cfg(any(test, feature = "inspection"))]
        output
            .traces
            .push(Trace::capture("settings.snippetsOpenFolder", &response));
    });
}

fn keymap(ui: &mut Ui, locale: &ResolvedLocale, appearance: &Appearance, output: &mut Output) {
    ui.spacing_mut().button_padding = egui::vec2(ACTION_BUTTON_PADDING_X, 0.0);
    outline_buttons(ui, appearance);
    let response = ui.add(
        egui::Button::new(
            RichText::new(message(locale, "settings.keymapOpenEditor", &[])).font(FontId::new(
                ACTION_BUTTON_FONT,
                crate::font_families::medium(ui),
            )),
        )
        .corner_radius(ROW_RADIUS)
        .min_size(egui::vec2(0.0, ACTION_BUTTON_HEIGHT)),
    );
    output.open_keybindings = response.clicked();
    #[cfg(any(test, feature = "inspection"))]
    output
        .traces
        .push(Trace::capture("settings.keymapOpenEditor", &response));
}

fn text(ui: &Ui, value: &str, position: egui::Pos2, width: f32, foreground: Color32) {
    let mut job = egui::text::LayoutJob::simple(
        value.into(),
        FontId::proportional(TEXT_SIZE),
        foreground,
        width,
    );
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    ui.painter().galley(position, galley, foreground);
}

fn check(ui: &Ui, rect: Rect, foreground: Color32) {
    let points =
        CHECK_POINTS.map(|[x, y]| rect.min + egui::vec2(rect.width() * x, rect.height() * y));
    ui.painter().add(egui::Shape::line(
        points.to_vec(),
        Stroke::new(CHECK_STROKE, foreground),
    ));
}

fn switch(
    ui: &mut Ui,
    field: Switch,
    settings: &Settings,
    locale: &ResolvedLocale,
    appearance: &Appearance,
    output: &mut Output,
) {
    if switch_control(
        ui,
        field.label(),
        field.description(),
        field.value(settings),
        locale,
        appearance,
        output,
    ) {
        output
            .changes
            .push(Change::Switch(field, !field.value(settings)));
    }
}

fn switch_control(
    ui: &mut Ui,
    label_key: &'static str,
    description: Option<&'static str>,
    checked: bool,
    locale: &ResolvedLocale,
    appearance: &Appearance,
    _output: &mut Output,
) -> bool {
    let mut changed = false;
    ui.push_id(label_key, |ui| {
        let label = message(locale, label_key, &[]);
        let width = ui.available_width();
        let row = ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2((width - SWITCH_WIDTH - CONTROL_GAP).max(0.0), 0.0),
                egui::Layout::top_down(Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing.y = TOC_ROW_GAP;
                    ui.set_min_width((width - SWITCH_WIDTH - CONTROL_GAP).max(0.0));
                    ui.label(
                        RichText::new(&label)
                            .size(TEXT_SIZE)
                            .color(appearance.foreground),
                    );
                    if let Some(description) = description {
                        ui.label(
                            RichText::new(message(locale, description, &[]))
                                .size(TEXT_SIZE)
                                .color(appearance.muted),
                        );
                    }
                },
            );
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(SWITCH_WIDTH, SWITCH_HEIGHT), Sense::hover());
            ui.painter().rect_filled(
                rect,
                SWITCH_HEIGHT * HALF,
                if checked {
                    appearance.primary
                } else {
                    appearance.input
                },
            );
            let x = if checked {
                rect.right() - SWITCH_THUMB * HALF - SWITCH_INSET
            } else {
                rect.left() + SWITCH_THUMB * HALF + SWITCH_INSET
            };
            ui.painter().circle_filled(
                egui::pos2(x, rect.center().y),
                SWITCH_THUMB * HALF,
                appearance.background,
            );
            rect
        });
        let label_response = ui.interact(
            row.response.rect,
            Id::new((ui.id(), "switch-label")),
            Sense::click(),
        );
        label_response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), checked, &label)
        });
        #[cfg(any(test, feature = "inspection"))]
        _output
            .traces
            .push(Trace::capture(label_key, &label_response));
        if label_response.has_focus() {
            ui.painter().rect_stroke(
                row.inner.expand(SWITCH_INSET),
                SWITCH_HEIGHT * HALF,
                Stroke::new(1.0, appearance.focus),
                egui::StrokeKind::Outside,
            );
        }
        if label_response.clicked() {
            changed = true;
        }
    });
    changed
}

fn numeric(
    ui: &mut Ui,
    field: Numeric,
    draft: &mut NumericDraft,
    settings: &Settings,
    locale: &ResolvedLocale,
    appearance: &Appearance,
    output: &mut Output,
) {
    let stored = field.value(settings);
    draft.sync(stored);
    ui.push_id(field.label(), |ui| {
        let width = ui.available_width();
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2((width - NUMERIC_WIDTH - CONTROL_GAP).max(0.0), LINE_HEIGHT),
                egui::Layout::top_down(Align::Min),
                |ui| {
                    ui.set_min_width((width - NUMERIC_WIDTH - CONTROL_GAP).max(0.0));
                    ui.add(
                        egui::Label::new(
                            RichText::new(message(locale, field.label(), &[]))
                                .size(TEXT_SIZE)
                                .color(appearance.foreground),
                        )
                        .truncate(),
                    );
                },
            );
            let response = ui.add_sized(
                [NUMERIC_WIDTH, LINE_HEIGHT],
                egui::TextEdit::singleline(&mut draft.text)
                    .id_salt((field.label(), stored))
                    .return_key(None)
                    .font(FontId::proportional(TEXT_SIZE))
                    .horizontal_align(Align::Max),
            );
            #[cfg(any(test, feature = "inspection"))]
            output.traces.push(Trace::capture(field.label(), &response));
            if response.lost_focus() {
                match draft.commit(field, stored) {
                    Ok(Some(change)) => output.changes.push(change),
                    Ok(None) => (),
                    Err(error) => output.error = Some(error),
                }
            }
        });
    });
}

impl Request {
    pub fn owner(&self) -> &Owner {
        &self.owner
    }
    pub fn mount(&self) -> u64 {
        self.mount
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

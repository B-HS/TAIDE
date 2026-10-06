#[cfg(any(test, feature = "inspection"))]
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(any(test, feature = "inspection"))]
use egui::Id;
use egui::{Align, AtomExt, Color32, FontId, Rect, Response, RichText, Stroke, Ui};
use taide_model::{
    error::{AppError, AppResult},
    locale::ResolvedLocale,
    theme::ResolvedTheme,
};

use crate::{
    icons::{Icon, Icons},
    modal::{self, Chrome, Composition, Layer, Presence},
    presentation::{color, message},
    settings_owner::Owner,
    settings_view::{
        self,
        code_view::{Choice, Picker},
    },
    snippet_draft::{Draft, LANGUAGE_IDS, Validation},
    snippet_edit::{Kind, Outcome, Reply, Request},
    snippet_editor_state::{Navigation, State},
    tooltip_trigger::Trigger,
};

const HEADER_X: i8 = 24;
const HEADER_Y: i8 = 16;
const SIDEBAR_WIDTH: f32 = 256.0;
const SIDEBAR_PADDING: i8 = 16;
const BODY_X: i8 = 24;
const BODY_Y: i8 = 16;
const CARD_PADDING: i8 = 12;
const CARD_RADIUS: u8 = 6;
const INPUT_RADIUS: u8 = 4;
const BORDER: f32 = 1.0;
const SIDES: f32 = 2.0;
const FONT: f32 = 12.0;
const TITLE_FONT: f32 = 14.0;
const HINT_FONT: f32 = 11.0;
const LINE_HEIGHT: f32 = 16.0;
const FIELD_GAP: f32 = 4.0;
const ROW_GAP: f32 = 8.0;
const CARD_GAP: f32 = 12.0;
const HEADER_GAP: f32 = 16.0;
const BUTTON_HEIGHT: f32 = 32.0;
const SMALL_BUTTON_HEIGHT: f32 = 24.0;
const FILE_ROW_HEIGHT: f32 = 30.0;
const FILE_ROW_Y: f32 = 6.0;
const CLOSE_SIZE: f32 = 16.0;
const CLOSE_INSET: f32 = 16.0;
const BUTTON_X: f32 = 12.0;
const SMALL_ICON_BUTTON_X: f32 = 6.0;
const SMALL_ICON_SIZE: f32 = 14.0;
const INPUT_X: i8 = 8;
const INPUT_Y: i8 = 4;
const BODY_ROWS: usize = 4;
const BODY_HEIGHT: f32 = 64.0;
const TRASH_TOP: f32 = 20.0;
const MODAL_WIDTH: f32 = 512.0;
const MODAL_BREAKPOINT: f32 = 640.0;
const SMALL_MODAL_WIDTH: f32 = 320.0;
const MODAL_PADDING: i8 = 24;
const MODAL_VIEWPORT_MARGIN: f32 = 32.0;
const MODAL_GAP: f32 = 16.0;
const MODAL_HEADER_GAP: f32 = 6.0;
const DIALOG_HEADER_GAP: f32 = 8.0;
const MODAL_BUTTON_HEIGHT: f32 = 36.0;
const MODAL_BUTTON_X: f32 = 16.0;
const TITLE_LINE_HEIGHT: f32 = 20.0;
const ALERT_TITLE_LINE_HEIGHT: f32 = 28.0;
const MODAL_TITLE_FONT: f32 = 18.0;
const BUTTON_HOVER_OPACITY: f32 = 0.9;
const DISABLED_OPACITY: f32 = 0.5;
const CLOSE_OPACITY: f32 = 0.7;
const CLOSE_RING_WIDTH: f32 = 2.0;
const CLOSE_RADIUS: u8 = 2;
const OUTLINE_SHADOW_OFFSET: [i8; 2] = [0, 1];
const OUTLINE_SHADOW_BLUR: u8 = 2;
const OUTLINE_SHADOW_ALPHA: u8 = 13;
const BUTTON_RING_WIDTH: f32 = 3.0;
const BUTTON_RING_OPACITY: f32 = 0.5;
const DESTRUCTIVE_RING_OPACITY: f32 = 0.2;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ButtonStyle {
    Primary,
    Outline,
    Ghost,
    Destructive,
}

#[derive(Clone)]
struct FocusModality {
    frame: Option<u64>,
    keyboard: bool,
    initial_pointer: bool,
    focused: Option<egui::Id>,
    visible: bool,
}

impl Default for FocusModality {
    fn default() -> Self {
        Self {
            frame: None,
            keyboard: true,
            initial_pointer: true,
            focused: None,
            visible: false,
        }
    }
}

fn focus_modality_id(context: &egui::Context) -> egui::Id {
    egui::Id::new(("snippet-button-focus-visible", context.viewport_id()))
}

fn update_focus_modality(context: &egui::Context) {
    let id = focus_modality_id(context);
    let frame = context.cumulative_frame_nr();
    let events = context.input(|input| input.raw.events.clone());
    let focused = context.memory(|memory| memory.focused());
    context.data_mut(|data| {
        let state = data.get_temp_mut_or_default::<FocusModality>(id);
        if state.frame == Some(frame) {
            return;
        }
        state.frame = Some(frame);
        for event in events {
            match event {
                egui::Event::Key {
                    pressed: true,
                    modifiers,
                    ..
                } if !modifiers.alt
                    && !modifiers.ctrl
                    && !modifiers.mac_cmd
                    && !modifiers.command =>
                {
                    state.keyboard = true;
                    state.visible = true;
                }
                egui::Event::PointerButton { pressed: true, .. }
                | egui::Event::Touch {
                    phase: egui::TouchPhase::Start,
                    ..
                } => {
                    state.keyboard = false;
                    state.initial_pointer = false;
                }
                egui::Event::PointerMoved(_) if state.initial_pointer => {
                    state.keyboard = false;
                    state.initial_pointer = false;
                }
                egui::Event::WindowFocused(false) => state.initial_pointer = true,
                egui::Event::AccessKitActionRequest(request)
                    if request.action == egui::accesskit::Action::Focus =>
                {
                    state.keyboard = true;
                    state.visible = true;
                }
                _ => {}
            }
        }
        if state.focused != focused {
            state.focused = focused;
            state.visible = state.keyboard;
        }
    });
}

fn paint_button_focus(ui: &Ui, response: &Response, appearance: &Appearance, style: ButtonStyle) {
    let id = focus_modality_id(ui.ctx());
    let visible = response.enabled()
        && response.has_focus()
        && ui.ctx().data_mut(|data| {
            let state = data.get_temp_mut_or_default::<FocusModality>(id);
            if state.focused != Some(response.id) {
                state.focused = Some(response.id);
                state.visible = state.keyboard;
            }
            state.visible
        });
    if style == ButtonStyle::Outline {
        let color = crate::button_color_motion::animate(
            ui.ctx(),
            response.id.with("snippet-focus-border"),
            if visible {
                appearance.focus
            } else {
                appearance.border
            },
        );
        ui.painter().rect_stroke(
            response.rect,
            CARD_RADIUS,
            Stroke::new(BORDER, color),
            egui::StrokeKind::Inside,
        );
    }
    let color = if style == ButtonStyle::Destructive {
        appearance.error.gamma_multiply(DESTRUCTIVE_RING_OPACITY)
    } else {
        appearance.focus.gamma_multiply(BUTTON_RING_OPACITY)
    };
    let color = crate::button_color_motion::animate(
        ui.ctx(),
        response.id.with("snippet-focus-ring-color"),
        if visible { color } else { Color32::TRANSPARENT },
    );
    let width = BUTTON_RING_WIDTH
        * crate::button_color_motion::animate_amount(
            ui.ctx(),
            response.id.with("snippet-focus-ring-width"),
            if visible { 1.0 } else { 0.0 },
        );
    if width <= 0.0 {
        return;
    }
    ui.painter().rect_stroke(
        response.rect,
        CARD_RADIUS,
        Stroke::new(width, color),
        egui::StrokeKind::Outside,
    );
}

pub struct Appearance {
    background: Color32,
    foreground: Color32,
    muted: Color32,
    border: Color32,
    input: Color32,
    input_border: Color32,
    active: Color32,
    hover: Color32,
    button_hover: Color32,
    button_hover_foreground: Color32,
    focus: Color32,
    primary: Color32,
    primary_foreground: Color32,
    error: Color32,
    modal: Color32,
    modal_border: Color32,
    shadow: Color32,
}

impl Appearance {
    pub fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        Ok(Self {
            background: color(theme, "app.background")?,
            foreground: color(theme, "app.foreground")?,
            muted: color(theme, "appSidebar.iconDefault")?,
            border: color(theme, "app.border")?,
            input: color(theme, "panel.inputBackground")?,
            input_border: color(theme, "panel.inputBorder")?,
            active: color(theme, "appSidebar.itemActive")?,
            hover: color(theme, "appSidebar.itemHover")?,
            button_hover: color(theme, "list.hoverBackground")?,
            button_hover_foreground: color(theme, "list.foreground")?,
            focus: color(theme, "app.focusBorder")?,
            primary: color(theme, "button.primaryBackground")?,
            primary_foreground: color(theme, "button.primaryForeground")?,
            error: color(theme, "statusIndicator.error")?,
            modal: color(theme, "modal.background")?,
            modal_border: color(theme, "modal.border")?,
            shadow: color(theme, "app.shadow")?,
        })
    }

    fn apply(&self, ui: &mut Ui) {
        ui.visuals_mut().override_text_color = Some(self.foreground);
        ui.visuals_mut().extreme_bg_color = self.input;
        ui.visuals_mut().disabled_alpha = DISABLED_OPACITY;
        ui.visuals_mut().widgets.inactive.bg_fill = self.background;
        ui.visuals_mut().widgets.inactive.weak_bg_fill = self.background;
        ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::new(BORDER, self.border);
        ui.visuals_mut().widgets.hovered.bg_fill = self.hover;
        ui.visuals_mut().widgets.hovered.weak_bg_fill = self.hover;
        ui.visuals_mut().widgets.active.bg_fill = self.active;
        ui.visuals_mut().widgets.active.weak_bg_fill = self.active;
        ui.spacing_mut().item_spacing = egui::vec2(ROW_GAP, ROW_GAP);
        ui.spacing_mut().button_padding = egui::vec2(BUTTON_X, 0.0);
    }
}

pub enum Notice {
    Saved,
    Incomplete(usize),
    DuplicateNames,
    SaveFailed { create: bool, error: AppError },
    DeleteFailed(AppError),
}

#[derive(Default)]
pub struct Output {
    pub requests: Vec<Request>,
    pub notices: Vec<Notice>,
    pub close: bool,
    pub tooltips: Vec<Trigger>,
    #[cfg(any(test, feature = "inspection"))]
    pub traces: Vec<(String, Id, Rect)>,
    #[cfg(any(test, feature = "inspection"))]
    pub interactions: HashMap<Id, Rect>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dialog {
    New,
    DeleteFile,
    Discard,
    DeleteEntry,
}

pub struct Editor {
    owner: Owner,
    lifetime: Arc<()>,
    state: State,
    needs_list: bool,
    list: Option<Request>,
    save: Option<Request>,
    delete: Option<Request>,
    notices: Vec<Notice>,
    load_error: Option<AppError>,
    picker: Picker,
    focused_dialog: Option<Dialog>,
    dialog_presence: Option<Presence>,
    dialog_layer: Option<(egui::Context, egui::LayerId)>,
    focus_pending: bool,
    global_input_present: bool,
    composition: Composition,
    button_motion_owner: Option<crate::button_color_motion::Owner>,
}

impl Editor {
    pub fn new(owner: Owner) -> Self {
        Self {
            owner,
            lifetime: Arc::new(()),
            state: State::default(),
            needs_list: true,
            list: None,
            save: None,
            delete: None,
            notices: Vec::new(),
            load_error: None,
            picker: Picker::default(),
            focused_dialog: None,
            dialog_presence: None,
            dialog_layer: None,
            focus_pending: false,
            global_input_present: false,
            composition: Composition::default(),
            button_motion_owner: None,
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    #[cfg(feature = "inspection")]
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    pub fn load_error(&self) -> Option<&AppError> {
        self.load_error.as_ref()
    }

    pub(crate) fn take_notices(&mut self) -> Vec<Notice> {
        std::mem::take(&mut self.notices)
    }

    pub(crate) fn update_catalog(&mut self, files: &[taide_model::snippet::SnippetFile]) {
        self.state.set_files(files.to_vec());
    }

    fn request(&self, kind: Kind) -> Request {
        Request::new(self.owner.clone(), &self.lifetime, kind)
    }

    pub fn accept(&mut self, reply: Reply) -> bool {
        if !reply.request.is_active() || reply.request.owner() != &self.owner {
            return false;
        }
        let kind = reply.request.kind().clone();
        let pending = match &kind {
            Kind::List => &self.list,
            Kind::Save { .. } => &self.save,
            Kind::Delete { .. } => &self.delete,
        };
        if !pending
            .as_ref()
            .is_some_and(|request| request.same_request(&reply.request))
        {
            return false;
        }
        let result = match (&kind, reply.result) {
            (Kind::List, Ok(Outcome::Listed(files))) => Ok(Outcome::Listed(files)),
            (Kind::Save { file_name, .. }, Ok(Outcome::Saved(file)))
                if *file_name == file.file_name =>
            {
                Ok(Outcome::Saved(file))
            }
            (Kind::Delete { .. }, Ok(Outcome::Deleted)) => Ok(Outcome::Deleted),
            (_, Err(error)) => Err(error),
            _ => Err(AppError::InvalidArgument(
                "snippet response does not match its request".into(),
            )),
        };
        match kind {
            Kind::List => {
                self.list = None;
                match result {
                    Ok(Outcome::Listed(files)) => {
                        self.load_error = None;
                        self.state.set_files(files);
                    }
                    Err(error) => self.load_error = Some(error),
                    _ => unreachable!(),
                }
            }
            Kind::Save {
                file_name, create, ..
            } => {
                self.save = None;
                match result {
                    Ok(Outcome::Saved(_)) => {
                        self.reload();
                        if create {
                            self.state.request_select(file_name);
                        } else {
                            self.notices.push(Notice::Saved);
                        }
                    }
                    Err(error) => self.notices.push(Notice::SaveFailed { create, error }),
                    _ => unreachable!(),
                }
            }
            Kind::Delete { .. } => {
                self.delete = None;
                match result {
                    Ok(Outcome::Deleted) => {
                        self.state.deleted_file();
                        self.reload();
                    }
                    Err(error) => self.notices.push(Notice::DeleteFailed(error)),
                    _ => unreachable!(),
                }
            }
        }
        true
    }

    pub(crate) fn reload(&mut self) {
        self.list = None;
        self.needs_list = true;
    }

    fn request_save(&mut self, output: &mut Output) {
        if self.save.is_some() {
            return;
        }
        match self.state.validate_save() {
            Err(Validation::Incomplete(count)) => {
                self.notices.push(Notice::Incomplete(count));
                return;
            }
            Err(Validation::DuplicateNames) => {
                self.notices.push(Notice::DuplicateNames);
                return;
            }
            Ok(()) => {}
        }
        match self.state.save_content() {
            Ok(Some((file_name, content))) => self.submit_save(file_name, content, false, output),
            Ok(None) => {}
            Err(error) => self.notices.push(Notice::SaveFailed {
                create: false,
                error,
            }),
        }
    }

    fn submit_save(
        &mut self,
        file_name: String,
        content: String,
        create: bool,
        output: &mut Output,
    ) {
        let request = self.request(Kind::Save {
            file_name,
            content,
            create,
        });
        self.save = Some(request.clone());
        output.requests.push(request);
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        locale: &ResolvedLocale,
        settings_appearance: &settings_view::Appearance,
        appearance: &Appearance,
        icons: &Icons,
    ) -> Output {
        update_focus_modality(ui.ctx());
        if self
            .button_motion_owner
            .as_ref()
            .is_some_and(|owner| !owner.belongs_to(ui.ctx()))
        {
            self.button_motion_owner = None;
        }
        let owner = self.button_motion_owner.get_or_insert_with(|| {
            crate::button_color_motion::Owner::new(ui.ctx().clone(), &self.lifetime)
        });
        owner.activate();
        let mut output = Output::default();
        if self.needs_list && ui.is_enabled() {
            self.needs_list = false;
            let request = self.request(Kind::List);
            self.list = Some(request.clone());
            output.requests.push(request);
        }
        let container = ui.available_rect_before_wrap();
        #[cfg(any(test, feature = "inspection"))]
        output
            .traces
            .push(("editor-container".into(), ui.id(), container));
        ui.painter()
            .rect_filled(container, 0.0, appearance.background);
        appearance.apply(ui);
        self.sync_dialog(ui.ctx());
        ui.scope(|ui| {
            if self.focused_dialog.is_some() {
                ui.visuals_mut().disabled_alpha = 1.0;
                ui.disable();
            }
            let header = egui::Frame::NONE
                .inner_margin(egui::Margin::symmetric(HEADER_X, HEADER_Y))
                .show(ui, |ui| {
                    ui.set_width((container.width() - f32::from(HEADER_X) * SIDES).max(0.0));
                    ui.spacing_mut().item_spacing.x = HEADER_GAP;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = CARD_GAP;
                        let back = add_button(
                            button(
                                ui,
                                message(locale, "snippetEditor.backToSettings", &[]),
                                appearance,
                                ButtonStyle::Ghost,
                                true,
                                None,
                            ),
                            true,
                            ui,
                            appearance,
                            ButtonStyle::Ghost,
                        );
                        trace(&mut output, "back", &back);
                        if back.clicked() {
                            output.close = self.state.request_close() == Navigation::Closed;
                        }
                        if let Some(name) = self.state.selected_file_name() {
                            ui.label(
                                RichText::new(name)
                                    .size(TITLE_FONT)
                                    .family(crate::font_families::medium(ui)),
                            );
                            ui.allocate_ui_with_layout(
                                egui::vec2(ui.available_width(), BUTTON_HEIGHT),
                                egui::Layout::right_to_left(Align::Center),
                                |ui| {
                                    ui.spacing_mut().item_spacing.x = ROW_GAP;
                                    let save = add_button(
                                        button(
                                            ui,
                                            message(locale, "snippetEditor.save", &[]),
                                            appearance,
                                            ButtonStyle::Primary,
                                            self.save.is_none(),
                                            None,
                                        ),
                                        self.save.is_none(),
                                        ui,
                                        appearance,
                                        ButtonStyle::Primary,
                                    );
                                    trace(&mut output, "save", &save);
                                    if save.clicked() {
                                        self.request_save(&mut output);
                                    }
                                    let delete = add_button(
                                        button(
                                            ui,
                                            message(locale, "snippetEditor.deleteFileButton", &[]),
                                            appearance,
                                            ButtonStyle::Outline,
                                            self.delete.is_none(),
                                            None,
                                        ),
                                        self.delete.is_none(),
                                        ui,
                                        appearance,
                                        ButtonStyle::Outline,
                                    );
                                    trace(&mut output, "delete-file", &delete);
                                    if delete.clicked() {
                                        self.state.delete_file_open = true;
                                    }
                                },
                            );
                        }
                    });
                });
            let header_rect = header.response.rect;
            ui.painter().line_segment(
                [header_rect.left_bottom(), header_rect.right_bottom()],
                Stroke::new(BORDER, appearance.border),
            );
            let body = Rect::from_min_max(
                egui::pos2(container.left(), header_rect.bottom() + BORDER),
                container.right_bottom(),
            );
            let sidebar = Rect::from_min_max(
                body.left_top(),
                egui::pos2(
                    (body.left() + SIDEBAR_WIDTH).min(body.right()),
                    body.bottom(),
                ),
            );
            let entries =
                Rect::from_min_max(egui::pos2(sidebar.right(), body.top()), body.right_bottom());
            ui.painter().line_segment(
                [
                    sidebar.right_top() - egui::vec2(BORDER / SIDES, 0.0),
                    sidebar.right_bottom() - egui::vec2(BORDER / SIDES, 0.0),
                ],
                Stroke::new(BORDER, appearance.border),
            );
            self.show_files(
                ui,
                Rect::from_min_max(sidebar.min, sidebar.max - egui::vec2(BORDER, 0.0)),
                locale,
                appearance,
                icons,
                &mut output,
            );
            self.show_entries(ui, entries, locale, appearance, icons, &mut output);
        });
        self.show_dialog(
            ui,
            locale,
            settings_appearance,
            appearance,
            icons,
            &mut output,
        );
        output.notices.append(&mut self.notices);
        ui.expand_to_include_rect(container);
        output
    }

    fn show_files(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        locale: &ResolvedLocale,
        appearance: &Appearance,
        icons: &Icons,
        output: &mut Output,
    ) {
        ui.scope_builder(
            egui::UiBuilder::new()
                .id_salt("snippet-files-pane")
                .max_rect(rect)
                .layout(egui::Layout::top_down(Align::Min)),
            |ui| {
                ui.set_width(rect.width());
                egui::ScrollArea::vertical()
                    .id_salt("snippet-files-scroll")
                    .max_height(rect.height().max(0.0))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Frame::NONE
                            .inner_margin(SIDEBAR_PADDING)
                            .show(ui, |ui| {
                                ui.set_width(
                                    (rect.width() - f32::from(SIDEBAR_PADDING) * SIDES).max(0.0),
                                );
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(message(
                                            locale,
                                            "snippetEditor.fileListTitle",
                                            &[],
                                        ))
                                        .size(FONT)
                                        .family(crate::font_families::medium(ui))
                                        .color(appearance.muted),
                                    );
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(ui.available_width(), SMALL_BUTTON_HEIGHT),
                                        egui::Layout::right_to_left(Align::Center),
                                        |ui| {
                                            let new = small_button(
                                                ui,
                                                Icon::Plus,
                                                "snippetEditor.newFileButton",
                                                locale,
                                                appearance,
                                                icons,
                                            );
                                            trace(output, "new-file", &new);
                                            if new.clicked() {
                                                self.state.new_file.open = true;
                                            }
                                        },
                                    );
                                });
                                let names = self
                                    .state
                                    .files()
                                    .iter()
                                    .map(|file| file.file_name.clone())
                                    .collect::<Vec<_>>();
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = FIELD_GAP;
                                    ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
                                        node.set_role(egui::accesskit::Role::List);
                                    });
                                    if names.is_empty() {
                                        ui.label(
                                            RichText::new(message(
                                                locale,
                                                "snippetEditor.noFiles",
                                                &[],
                                            ))
                                            .size(FONT)
                                            .color(appearance.muted),
                                        );
                                    }
                                    for name in names {
                                        let selected =
                                            self.state.selected_file_name() == Some(&name);
                                        ui.push_id(&name, |ui| {
                                            ui.spacing_mut().button_padding =
                                                egui::vec2(BUTTON_X, FILE_ROW_Y);
                                            ui.ctx().accesskit_node_builder(
                                                ui.unique_id(),
                                                |node| {
                                                    node.set_role(egui::accesskit::Role::ListItem);
                                                },
                                            );
                                            ui.visuals_mut().widgets.inactive.weak_bg_fill =
                                                if selected {
                                                    appearance.active
                                                } else {
                                                    Color32::TRANSPARENT
                                                };
                                            ui.visuals_mut().widgets.active.weak_bg_fill =
                                                if selected {
                                                    appearance.active
                                                } else {
                                                    appearance.hover
                                                };
                                            let response = ui.add(
                                                egui::Button::new(
                                                    RichText::new(&name)
                                                        .size(FONT)
                                                        .line_height(Some(LINE_HEIGHT))
                                                        .atom_grow(true)
                                                        .atom_align(egui::Align2::LEFT_CENTER),
                                                )
                                                .truncate()
                                                .min_size(egui::vec2(
                                                    ui.available_width(),
                                                    FILE_ROW_HEIGHT,
                                                ))
                                                .stroke(Stroke::new(
                                                    BORDER,
                                                    if selected {
                                                        appearance.focus
                                                    } else {
                                                        appearance.border
                                                    },
                                                ))
                                                .corner_radius(CARD_RADIUS),
                                            );
                                            response.widget_info(|| {
                                                egui::WidgetInfo::selected(
                                                    egui::WidgetType::Button,
                                                    response.enabled(),
                                                    selected,
                                                    &name,
                                                )
                                            });
                                            trace(output, &format!("file:{name}"), &response);
                                            if response.clicked() {
                                                self.state.request_select(name.clone());
                                            }
                                        });
                                    }
                                });
                            });
                    });
            },
        );
    }

    fn show_entries(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        locale: &ResolvedLocale,
        appearance: &Appearance,
        icons: &Icons,
        output: &mut Output,
    ) {
        ui.scope_builder(
            egui::UiBuilder::new()
                .id_salt("snippet-entries-pane")
                .max_rect(rect)
                .layout(egui::Layout::top_down(Align::Min)),
            |ui| {
                ui.set_width(rect.width());
                egui::ScrollArea::vertical()
                    .id_salt("snippet-entries-scroll")
                    .max_height(rect.height().max(0.0))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(BODY_X, BODY_Y))
                            .show(ui, |ui| {
                                ui.set_width((rect.width() - f32::from(BODY_X) * SIDES).max(0.0));
                                ui.spacing_mut().item_spacing.y = CARD_GAP;
                                if self.state.selected_file_name().is_none()
                                    || self.state.drafts().is_none()
                                {
                                    ui.label(
                                        RichText::new(message(
                                            locale,
                                            "snippetEditor.noFiles",
                                            &[],
                                        ))
                                        .size(FONT)
                                        .color(appearance.muted),
                                    );
                                    return;
                                }
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(message(
                                            locale,
                                            "snippetEditor.snippetListTitle",
                                            &[],
                                        ))
                                        .size(FONT)
                                        .family(crate::font_families::medium(ui))
                                        .color(appearance.muted),
                                    );
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(ui.available_width(), SMALL_BUTTON_HEIGHT),
                                        egui::Layout::right_to_left(Align::Center),
                                        |ui| {
                                            let add = small_button(
                                                ui,
                                                Icon::Plus,
                                                "snippetEditor.addSnippetButton",
                                                locale,
                                                appearance,
                                                icons,
                                            );
                                            trace(output, "add-entry", &add);
                                            if add.clicked() {
                                                self.state.append_entry();
                                            }
                                        },
                                    );
                                });
                                let show_scope = self.state.show_scope();
                                if let Some(drafts) = self.state.drafts_mut() {
                                    if drafts.is_empty() {
                                        ui.label(
                                            RichText::new(message(
                                                locale,
                                                "snippetEditor.noSnippets",
                                                &[],
                                            ))
                                            .size(FONT)
                                            .color(appearance.muted),
                                        );
                                    }
                                    let mut delete = None;
                                    for draft in drafts {
                                        let id = draft.id.clone();
                                        if ui
                                            .push_id(&id, |ui| {
                                                entry(
                                                    ui, draft, show_scope, locale, appearance,
                                                    icons, output,
                                                )
                                            })
                                            .inner
                                        {
                                            delete = Some(id);
                                        }
                                    }
                                    if delete.is_some() {
                                        self.state.delete_entry = delete;
                                    }
                                }
                            });
                    });
            },
        );
    }

    fn dialog(&self) -> Option<Dialog> {
        if self.state.delete_entry.is_some() {
            return Some(Dialog::DeleteEntry);
        }
        if self.state.pending_discard().is_some() {
            return Some(Dialog::Discard);
        }
        if self.state.delete_file_open {
            return Some(Dialog::DeleteFile);
        }
        self.state.new_file.open.then_some(Dialog::New)
    }

    fn cancel_dialog(&mut self, dialog: Dialog) {
        self.composition.reset();
        if dialog == Dialog::New {
            self.picker = Picker::default();
        }
        match dialog {
            Dialog::New => self.state.new_file.open = false,
            Dialog::DeleteFile => self.state.delete_file_open = false,
            Dialog::Discard => self.state.cancel_discard(),
            Dialog::DeleteEntry => self.state.delete_entry = None,
        }
    }

    fn clear_dialog_layer(&mut self) {
        if let Some((context, layer)) = self.dialog_layer.take() {
            modal::unmount(&context, layer);
        }
    }

    fn sync_dialog(&mut self, context: &egui::Context) {
        let now = context.input(|input| input.time);
        let desired = self.dialog();
        if let Some(dialog) = desired {
            if self.focused_dialog != Some(dialog) {
                self.clear_dialog_layer();
                self.focused_dialog = Some(dialog);
                self.dialog_presence = Some(Presence::enter(now));
                self.focus_pending = true;
            }
        }
        if let Some(presence) = &mut self.dialog_presence {
            presence.target(desired.is_some(), now);
        }
        if self
            .dialog_presence
            .is_some_and(|presence| !presence.is_present(now))
        {
            self.clear_dialog_layer();
            self.focused_dialog = None;
            self.dialog_presence = None;
            self.global_input_present = false;
            self.composition.reset();
        }
    }

    fn show_dialog(
        &mut self,
        ui: &Ui,
        locale: &ResolvedLocale,
        settings_appearance: &settings_view::Appearance,
        appearance: &Appearance,
        icons: &Icons,
        output: &mut Output,
    ) {
        self.sync_dialog(ui.ctx());
        let Some(dialog) = self.focused_dialog else {
            return;
        };
        let open = self.dialog() == Some(dialog);
        let now = ui.input(|input| input.time);
        let transition = self.dialog_presence.unwrap().sample(now);
        if transition.is_active {
            ui.ctx().request_repaint();
        }
        let id = ui.make_persistent_id("snippet-dialog").with(match dialog {
            Dialog::New => "new",
            Dialog::DeleteFile => "delete-file",
            Dialog::Discard => "discard",
            Dialog::DeleteEntry => "delete-entry",
        });
        let viewport_width = ui.ctx().content_rect().width();
        let mobile = viewport_width < MODAL_BREAKPOINT;
        let width = if dialog == Dialog::Discard {
            SMALL_MODAL_WIDTH
        } else if mobile {
            viewport_width - MODAL_VIEWPORT_MARGIN
        } else {
            MODAL_WIDTH
        };
        let maximum_width = if dialog == Dialog::Discard {
            viewport_width
        } else {
            viewport_width - MODAL_VIEWPORT_MARGIN
        };
        let width = width.min(maximum_width.max(0.0));
        let mut confirm = false;
        let mut cancel = false;
        let is_composing = ui.input(|input| self.composition.observe(&input.events));
        let enabled = ui.is_enabled() && open;
        let layer = Layer {
            id,
            chrome: Chrome {
                background: appearance.modal,
                border: appearance.modal_border,
                shadow: appearance.shadow,
            },
            padding: MODAL_PADDING,
            transition: Some(transition),
            is_modal: true,
        };
        #[cfg(any(test, feature = "inspection"))]
        let transform = transition.transform(ui.ctx().content_rect().center());
        self.dialog_layer = Some((ui.ctx().clone(), layer.layer_id()));
        let mut focus_nodes = Vec::new();
        let response = layer.show(ui.ctx(), |ui| {
            appearance.apply(ui);
            if !enabled {
                ui.disable();
            }
            ui.set_width((width - f32::from(MODAL_PADDING) * SIDES - BORDER * SIDES).max(0.0));
            ui.spacing_mut().item_spacing.y = MODAL_GAP;
            let (title, description, action) = match dialog {
                Dialog::New => ("snippetEditor.newFileDialogTitle", None, "common.confirm"),
                Dialog::DeleteFile => (
                    "snippetEditor.deleteFileConfirmTitle",
                    Some((
                        "snippetEditor.deleteFileConfirmDescription",
                        "fileName",
                        self.state
                            .selected_file_name()
                            .unwrap_or_default()
                            .to_owned(),
                    )),
                    "snippetEditor.deleteFileButton",
                ),
                Dialog::Discard => (
                    "common.unsavedChangesTitle",
                    Some(("common.unsavedChangesDescription", "", String::new())),
                    "common.discardChanges",
                ),
                Dialog::DeleteEntry => (
                    "snippetEditor.deleteConfirmTitle",
                    Some((
                        "snippetEditor.deleteConfirmDescription",
                        "name",
                        self.state.pending_delete_entry_name().to_owned(),
                    )),
                    "snippetEditor.deleteSnippetButton",
                ),
            };
            let dialog_node = ui.unique_id();
            ui.ctx().accesskit_node_builder(dialog_node, |node| {
                node.set_role(if dialog == Dialog::New {
                    egui::accesskit::Role::Dialog
                } else {
                    egui::accesskit::Role::AlertDialog
                });
            });
            let header_align = if mobile || dialog == Dialog::Discard {
                Align::Center
            } else {
                Align::Min
            };
            ui.with_layout(egui::Layout::top_down_justified(header_align), |ui| {
                ui.spacing_mut().item_spacing.y = if dialog == Dialog::New {
                    DIALOG_HEADER_GAP
                } else {
                    MODAL_HEADER_GAP
                };
                let label = ui.label(
                    RichText::new(message(locale, title, &[]))
                        .size(MODAL_TITLE_FONT)
                        .line_height(Some(if dialog == Dialog::New {
                            MODAL_TITLE_FONT
                        } else {
                            ALERT_TITLE_LINE_HEIGHT
                        }))
                        .family(crate::font_families::semibold(ui))
                        .color(appearance.foreground),
                );
                ui.ctx().accesskit_node_builder(dialog_node, |node| {
                    node.push_labelled_by(label.id.accesskit_id());
                });
                if let Some((key, param, value)) = description {
                    let description = ui.label(
                        RichText::new(message(locale, key, &[(param, &value)]))
                            .size(TITLE_FONT)
                            .line_height(Some(TITLE_LINE_HEIGHT))
                            .color(appearance.muted),
                    );
                    ui.ctx().accesskit_node_builder(dialog_node, |node| {
                        node.push_described_by(description.id.accesskit_id());
                    });
                    trace(output, "dialog-description", &description);
                }
            });
            if dialog == Dialog::New {
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.y = ROW_GAP;
                    let mut picker_output = settings_view::Output::default();
                    let mut choices = vec![Choice {
                        identity: "global".into(),
                        value: "global".to_owned(),
                        label: message(locale, "snippetEditor.newFileGlobalOption", &[]),
                        search: String::new(),
                        font: None,
                    }];
                    choices.extend(LANGUAGE_IDS.iter().map(|language| Choice {
                        identity: (*language).into(),
                        value: (*language).to_owned(),
                        label: (*language).into(),
                        search: String::new(),
                        font: None,
                    }));
                    let active = self.state.new_file.option().to_owned();
                    let text = choices
                        .iter()
                        .find(|choice| choice.value == active)
                        .map(|choice| choice.label.as_str())
                        .unwrap_or_default();
                    if let Some(option) = self.picker.show(
                        ui,
                        "snippetEditor.newFileLanguagePlaceholder",
                        text,
                        &active,
                        &choices,
                        None,
                        None,
                        false,
                        egui::emath::RectAlign::BOTTOM_END,
                        locale,
                        settings_appearance,
                        &mut picker_output,
                    ) {
                        self.state.new_file.select(&option);
                    }
                    if self.focus_pending
                        && !self.state.new_file.is_global()
                        && !ui.is_sizing_pass()
                    {
                        let trigger = ui.make_persistent_id((
                            "snippetEditor.newFileLanguagePlaceholder",
                            "picker",
                        ));
                        if let Some(response) = ui.ctx().read_response(trigger) {
                            ui.memory_mut(|memory| {
                                memory.request_focus_with_filter(response.id, modal::FOCUS_FILTER)
                            });
                            self.focus_pending = false;
                        }
                    }
                    focus_nodes.push(ui.make_persistent_id((
                        "snippetEditor.newFileLanguagePlaceholder",
                        "picker",
                    )));
                    #[cfg(any(test, feature = "inspection"))]
                    for trace in picker_output.traces.drain(..) {
                        if trace.enabled {
                            output.interactions.insert(trace.id, trace.interact_rect);
                        }
                        output
                            .traces
                            .push((trace.field.into(), trace.id, trace.rect));
                    }
                    if self.state.new_file.is_global() {
                        let mut layouter = |ui: &Ui, buffer: &dyn egui::TextBuffer, _: f32| {
                            let mut job = egui::text::LayoutJob::simple(
                                buffer.as_str().into(),
                                FontId::proportional(TITLE_FONT),
                                appearance.foreground,
                                f32::INFINITY,
                            );
                            job.keep_trailing_whitespace = true;
                            for section in &mut job.sections {
                                section.format.line_height = Some(TITLE_LINE_HEIGHT);
                            }
                            ui.fonts_mut(|fonts| fonts.layout_job(job))
                        };
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.state.new_file.global_name)
                                .id_salt("snippet-global-name")
                                .desired_width(ui.available_width())
                                .font(FontId::proportional(TITLE_FONT))
                                .layouter(&mut layouter)
                                .hint_text(
                                    RichText::new(message(
                                        locale,
                                        "snippetEditor.newFileGlobalNamePlaceholder",
                                        &[],
                                    ))
                                    .size(TITLE_FONT)
                                    .line_height(Some(TITLE_LINE_HEIGHT)),
                                )
                                .margin(egui::Margin::symmetric(INPUT_X, INPUT_Y))
                                .frame(
                                    egui::Frame::NONE
                                        .fill(appearance.input)
                                        .inner_margin(egui::Margin::symmetric(INPUT_X, INPUT_Y))
                                        .stroke(Stroke::new(BORDER, appearance.input_border))
                                        .corner_radius(INPUT_RADIUS),
                                ),
                        );
                        response.widget_info(|| {
                            egui::WidgetInfo::text_edit(
                                response.enabled(),
                                "",
                                &self.state.new_file.global_name,
                                message(locale, "snippetEditor.newFileGlobalNamePlaceholder", &[]),
                            )
                        });
                        if (!self.global_input_present || self.focus_pending)
                            && !ui.is_sizing_pass()
                        {
                            ui.memory_mut(|memory| {
                                memory.request_focus_with_filter(response.id, modal::FOCUS_FILTER)
                            });
                            self.focus_pending = false;
                        }
                        trace(output, "new-global-name", &response);
                        focus_nodes.push(response.id);
                        self.global_input_present = true;
                    } else {
                        self.global_input_present = false;
                    }
                });
                let close_rect = Rect::from_min_size(
                    egui::pos2(
                        ui.max_rect().right() + f32::from(MODAL_PADDING) - CLOSE_INSET - CLOSE_SIZE,
                        ui.max_rect().top() - f32::from(MODAL_PADDING) + CLOSE_INSET,
                    ),
                    egui::Vec2::splat(CLOSE_SIZE),
                );
                let close = ui.place(
                    close_rect,
                    egui::Button::new(())
                        .frame(false)
                        .small()
                        .min_size(egui::Vec2::splat(CLOSE_SIZE)),
                );
                let close_opacity = crate::button_color_motion::animate_amount(
                    ui.ctx(),
                    close.id.with("snippet-dialog-close-opacity"),
                    if close.hovered() { 1.0 } else { CLOSE_OPACITY },
                );
                let parent_opacity = ui.opacity();
                ui.multiply_opacity(close_opacity);
                if let Some(image) = icons.image(Icon::Close, appearance.foreground) {
                    image.paint_at(ui, close.rect);
                }
                if close.has_focus() {
                    ui.painter().rect_stroke(
                        close.rect,
                        CLOSE_RADIUS,
                        Stroke::new(CLOSE_RING_WIDTH, appearance.background),
                        egui::StrokeKind::Outside,
                    );
                    ui.painter().rect_stroke(
                        close.rect.expand(CLOSE_RING_WIDTH),
                        f32::from(CLOSE_RADIUS) + CLOSE_RING_WIDTH,
                        Stroke::new(CLOSE_RING_WIDTH, appearance.focus),
                        egui::StrokeKind::Outside,
                    );
                }
                ui.set_opacity(parent_opacity);
                close.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        close.enabled(),
                        message(locale, "common.close", &[]),
                    )
                });
                trace(output, "dialog-close", &close);
                cancel |= close.clicked();
                focus_nodes.push(close.id);
            }
            let footer_layout = if mobile && dialog != Dialog::Discard {
                egui::Layout::top_down_justified(Align::Center)
            } else {
                egui::Layout::right_to_left(Align::Center)
            };
            ui.with_layout(footer_layout, |ui| {
                ui.spacing_mut().item_spacing.x = ROW_GAP;
                ui.spacing_mut().item_spacing.y = ROW_GAP;
                ui.spacing_mut().button_padding.x = MODAL_BUTTON_X;
                let button_width = if dialog == Dialog::Discard {
                    (ui.available_width() - ROW_GAP) / SIDES
                } else if mobile {
                    ui.available_width()
                } else {
                    0.0
                };
                let enabled =
                    dialog != Dialog::New || self.state.new_file.can_create(self.state.files());
                let confirm_button = button(
                    ui,
                    message(locale, action, &[]),
                    appearance,
                    if dialog == Dialog::New {
                        ButtonStyle::Primary
                    } else {
                        ButtonStyle::Destructive
                    },
                    enabled,
                    (button_width > 0.0).then_some(button_width),
                )
                .min_size(egui::vec2(button_width, MODAL_BUTTON_HEIGHT));
                let response = add_button(
                    confirm_button,
                    enabled,
                    ui,
                    appearance,
                    if dialog == Dialog::New {
                        ButtonStyle::Primary
                    } else {
                        ButtonStyle::Destructive
                    },
                );
                trace(output, "dialog-confirm", &response);
                let confirm_id = response.enabled().then_some(response.id);
                confirm = response.clicked();
                let response = add_button(
                    button(
                        ui,
                        message(locale, "common.cancel", &[]),
                        appearance,
                        ButtonStyle::Outline,
                        true,
                        (button_width > 0.0).then_some(button_width),
                    )
                    .min_size(egui::vec2(button_width, MODAL_BUTTON_HEIGHT)),
                    true,
                    ui,
                    appearance,
                    ButtonStyle::Outline,
                );
                if self.focus_pending && !ui.is_sizing_pass() {
                    ui.memory_mut(|memory| {
                        memory.request_focus_with_filter(response.id, modal::FOCUS_FILTER)
                    });
                    self.focus_pending = false;
                }
                trace(output, "dialog-cancel", &response);
                let footer_start = if dialog == Dialog::New {
                    focus_nodes.len().saturating_sub(1)
                } else {
                    focus_nodes.len()
                };
                focus_nodes.insert(footer_start, response.id);
                if let Some(confirm_id) = confirm_id {
                    focus_nodes.insert(footer_start + 1, confirm_id);
                }
                cancel |= response.clicked();
            });
        });
        #[cfg(any(test, feature = "inspection"))]
        {
            for (name, node, rect) in &mut output.traces {
                if name.starts_with("dialog-")
                    || name == "new-global-name"
                    || name == "snippetEditor.newFileLanguagePlaceholder"
                {
                    *rect = transform * *rect;
                    if let Some(interaction) = output.interactions.get_mut(node) {
                        *interaction = transform * *interaction;
                    }
                }
            }
            output.traces.push((
                "dialog-content".into(),
                id,
                transform * response.response.rect,
            ));
            output.traces.push((
                "dialog-backdrop".into(),
                id.with("backdrop"),
                transform * response.backdrop_response.rect,
            ));
        }
        if enabled && response.is_top_modal && !response.any_popup_open {
            modal::trap_focus(ui.ctx(), &focus_nodes);
        }
        if enabled && !is_composing && modal::takes_escape(ui.ctx(), &response) {
            cancel = true;
        }
        if enabled && dialog == Dialog::New && response.backdrop_response.clicked() {
            cancel = true;
        }
        if cancel {
            self.cancel_dialog(dialog);
        }
        if confirm {
            match dialog {
                Dialog::New => {
                    let file_name = self.state.new_file.file_name();
                    self.state.new_file.open = false;
                    self.picker = Picker::default();
                    self.submit_save(file_name, "{}".into(), true, output);
                }
                Dialog::DeleteFile => {
                    if let Some(file_name) = self.state.selected_file_name() {
                        let request = self.request(Kind::Delete {
                            file_name: file_name.into(),
                        });
                        self.delete = Some(request.clone());
                        output.requests.push(request);
                    }
                    self.state.delete_file_open = false;
                }
                Dialog::Discard => {
                    output.close = self.state.confirm_discard() == Navigation::Closed
                }
                Dialog::DeleteEntry => self.state.confirm_delete_entry(),
            }
        }
        self.sync_dialog(ui.ctx());
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        self.clear_dialog_layer();
    }
}

fn button(
    ui: &Ui,
    text: String,
    appearance: &Appearance,
    style: ButtonStyle,
    enabled: bool,
    width: Option<f32>,
) -> egui::Button<'static> {
    let hovered = button_hovered(ui, enabled);
    let foreground = animated_button_color(
        ui,
        "foreground",
        match style {
            ButtonStyle::Primary => appearance.primary_foreground,
            ButtonStyle::Destructive => Color32::WHITE,
            ButtonStyle::Outline | ButtonStyle::Ghost if hovered => {
                appearance.button_hover_foreground
            }
            ButtonStyle::Outline | ButtonStyle::Ghost => appearance.foreground,
        },
    );
    let fill = animated_button_color(
        ui,
        "background",
        match style {
            ButtonStyle::Primary if hovered => {
                appearance.primary.gamma_multiply(BUTTON_HOVER_OPACITY)
            }
            ButtonStyle::Primary => appearance.primary,
            ButtonStyle::Destructive if hovered => {
                appearance.error.gamma_multiply(BUTTON_HOVER_OPACITY)
            }
            ButtonStyle::Destructive => appearance.error,
            ButtonStyle::Outline | ButtonStyle::Ghost if hovered => appearance.button_hover,
            ButtonStyle::Outline => appearance.background,
            ButtonStyle::Ghost => Color32::TRANSPARENT,
        },
    );
    egui::Button::new(button_text(ui, text, foreground, width))
        .wrap_mode(egui::TextWrapMode::Extend)
        .fill(fill)
        .stroke(if style == ButtonStyle::Outline {
            Stroke::new(BORDER, Color32::TRANSPARENT)
        } else {
            Stroke::NONE
        })
        .corner_radius(CARD_RADIUS)
        .min_size(egui::vec2(0.0, BUTTON_HEIGHT))
}

fn button_hovered(ui: &Ui, enabled: bool) -> bool {
    enabled
        && ui.is_enabled()
        && ui
            .ctx()
            .read_response(ui.next_auto_id())
            .is_some_and(|response| response.hovered())
}

fn animated_button_color(ui: &Ui, property: &str, target: Color32) -> Color32 {
    crate::button_color_motion::animate(
        ui.ctx(),
        ui.next_auto_id().with(("snippet-button-color", property)),
        target,
    )
}

fn button_text(
    ui: &Ui,
    text: String,
    foreground: Color32,
    width: Option<f32>,
) -> egui::Atom<'static> {
    let text = RichText::new(text)
        .size(TITLE_FONT)
        .line_height(Some(TITLE_LINE_HEIGHT))
        .family(crate::font_families::medium(ui))
        .color(foreground);
    match width {
        Some(width) => text.atom_size(egui::vec2(
            (width - ui.spacing().button_padding.x * SIDES).max(0.0),
            TITLE_LINE_HEIGHT,
        )),
        None => text.into(),
    }
}

fn small_button(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    locale: &ResolvedLocale,
    appearance: &Appearance,
    icons: &Icons,
) -> Response {
    ui.spacing_mut().button_padding = egui::vec2(SMALL_ICON_BUTTON_X, 0.0);
    let hovered = button_hovered(ui, true);
    let foreground = animated_button_color(
        ui,
        "foreground",
        if hovered {
            appearance.button_hover_foreground
        } else {
            appearance.foreground
        },
    );
    let fill = animated_button_color(
        ui,
        "background",
        if hovered {
            appearance.button_hover
        } else {
            appearance.background
        },
    );
    let button = egui::Button::opt_image_and_text(
        icons
            .image(icon, foreground)
            .map(|image| image.fit_to_exact_size(egui::Vec2::splat(SMALL_ICON_SIZE))),
        Some(
            RichText::new(message(locale, label, &[]))
                .size(FONT)
                .line_height(Some(LINE_HEIGHT))
                .family(crate::font_families::medium(ui))
                .color(foreground)
                .into(),
        ),
    )
    .fill(fill)
    .gap(FIELD_GAP)
    .stroke(Stroke::new(BORDER, Color32::TRANSPARENT))
    .corner_radius(CARD_RADIUS)
    .min_size(egui::vec2(0.0, SMALL_BUTTON_HEIGHT));
    add_button(button, true, ui, appearance, ButtonStyle::Outline)
}

fn add_button(
    button: egui::Button<'_>,
    enabled: bool,
    ui: &mut Ui,
    appearance: &Appearance,
    style: ButtonStyle,
) -> Response {
    let old_opacity = ui.opacity();
    let old_disabled_alpha = ui.visuals().disabled_alpha;
    let parent_opacity = if !ui.is_enabled() && old_disabled_alpha > 0.0 {
        old_opacity / old_disabled_alpha
    } else {
        old_opacity
    };
    let target_opacity = if !enabled {
        DISABLED_OPACITY
    } else if !ui.is_enabled() {
        old_disabled_alpha
    } else {
        1.0
    };
    let opacity = if ui.is_sizing_pass() {
        target_opacity
    } else {
        crate::button_color_motion::animate_amount(
            ui.ctx(),
            ui.next_auto_id().with("snippet-button-opacity"),
            target_opacity,
        )
    };
    ui.visuals_mut().disabled_alpha = 1.0;
    ui.set_opacity(parent_opacity * opacity);
    let shadow_index = (style == ButtonStyle::Outline).then(|| ui.painter().add(egui::Shape::Noop));
    let response = ui.add_enabled(enabled, button);
    if let Some(shadow_index) = shadow_index {
        let shadow = egui::epaint::Shadow {
            offset: OUTLINE_SHADOW_OFFSET,
            blur: OUTLINE_SHADOW_BLUR,
            spread: 0,
            color: Color32::from_black_alpha(OUTLINE_SHADOW_ALPHA),
        };
        ui.painter()
            .set(shadow_index, shadow.as_shape(response.rect, CARD_RADIUS));
    }
    paint_button_focus(ui, &response, appearance, style);
    ui.set_opacity(old_opacity);
    ui.visuals_mut().disabled_alpha = old_disabled_alpha;
    response
}

fn entry(
    ui: &mut Ui,
    draft: &mut Draft,
    show_scope: bool,
    locale: &ResolvedLocale,
    appearance: &Appearance,
    icons: &Icons,
    output: &mut Output,
) -> bool {
    let mut delete = false;
    let card = egui::Frame::NONE
        .stroke(Stroke::new(BORDER, appearance.border))
        .corner_radius(CARD_RADIUS)
        .inner_margin(CARD_PADDING)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = ROW_GAP;
            ui.horizontal_top(|ui| {
                let width = (ui.available_width() - SMALL_BUTTON_HEIGHT - ROW_GAP).max(0.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(width, 0.0),
                    egui::Layout::top_down(Align::Min),
                    |ui| {
                        field(
                            ui,
                            &mut draft.name,
                            "name",
                            "snippetEditor.nameLabel",
                            "snippetEditor.namePlaceholder",
                            None,
                            false,
                            locale,
                            appearance,
                            output,
                        );
                    },
                );
                ui.vertical(|ui| {
                    ui.add_space(TRASH_TOP);
                    let label = message(locale, "snippetEditor.deleteSnippetButton", &[]);
                    let (response, trigger) =
                        crate::tooltip_trigger::wrap_button(ui, &label, false, |ui| {
                            let response = ui.add(
                                egui::Button::new(())
                                    .frame(false)
                                    .min_size(egui::Vec2::splat(SMALL_BUTTON_HEIGHT)),
                            );
                            let foreground = if response.hovered() {
                                appearance.error
                            } else {
                                appearance.muted
                            };
                            if let Some(image) = icons.image(Icon::Trash, foreground) {
                                image.paint_at(
                                    ui,
                                    Rect::from_center_size(
                                        response.rect.center(),
                                        egui::Vec2::splat(SMALL_ICON_SIZE),
                                    ),
                                );
                            }
                            response.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    response.enabled(),
                                    &label,
                                )
                            });
                            response
                        });
                    output.tooltips.push(trigger);
                    trace(output, "delete-entry", &response);
                    delete = response.clicked();
                });
            });
            field(
                ui,
                &mut draft.prefix,
                "prefix",
                "snippetEditor.prefixLabel",
                "snippetEditor.prefixPlaceholder",
                Some("snippetEditor.prefixHint"),
                false,
                locale,
                appearance,
                output,
            );
            field(
                ui,
                &mut draft.body,
                "body",
                "snippetEditor.bodyLabel",
                "snippetEditor.bodyPlaceholder",
                Some("snippetEditor.bodyHint"),
                true,
                locale,
                appearance,
                output,
            );
            field(
                ui,
                &mut draft.description,
                "description",
                "snippetEditor.descriptionLabel",
                "snippetEditor.descriptionPlaceholder",
                None,
                false,
                locale,
                appearance,
                output,
            );
            if show_scope {
                field(
                    ui,
                    &mut draft.scope,
                    "scope",
                    "snippetEditor.scopeLabel",
                    "snippetEditor.scopePlaceholder",
                    Some("snippetEditor.scopeHint"),
                    false,
                    locale,
                    appearance,
                    output,
                );
            }
        });
    trace(output, "entry-card", &card.response);
    delete
}

fn field(
    ui: &mut Ui,
    value: &mut String,
    identity: &'static str,
    label: &'static str,
    placeholder: &'static str,
    hint: Option<&str>,
    multiline: bool,
    locale: &ResolvedLocale,
    appearance: &Appearance,
    output: &mut Output,
) {
    ui.push_id(identity, |ui| {
        ui.spacing_mut().item_spacing.y = FIELD_GAP;
        let label_text = message(locale, label, &[]);
        let label = ui.label(
            RichText::new(&label_text)
                .size(FONT)
                .line_height(Some(LINE_HEIGHT))
                .color(appearance.muted),
        );
        let old = value.clone();
        let font = if multiline {
            FontId::monospace(FONT)
        } else {
            FontId::proportional(FONT)
        };
        let mut layouter = |ui: &Ui, buffer: &dyn egui::TextBuffer, width: f32| {
            let mut job = egui::text::LayoutJob::simple(
                buffer.as_str().into(),
                font.clone(),
                appearance.foreground,
                width,
            );
            job.keep_trailing_whitespace = true;
            for section in &mut job.sections {
                section.format.line_height = Some(LINE_HEIGHT);
            }
            ui.fonts_mut(|fonts| fonts.layout_job(job))
        };
        let input_frame = egui::Frame::NONE
            .fill(appearance.input)
            .stroke(Stroke::new(BORDER, appearance.input_border))
            .corner_radius(INPUT_RADIUS)
            .inner_margin(egui::Margin::symmetric(INPUT_X, INPUT_Y));
        let response = input_frame
            .show(ui, |ui| {
                let edit = if multiline {
                    egui::TextEdit::multiline(value).desired_rows(BODY_ROWS)
                } else {
                    egui::TextEdit::singleline(value)
                };
                let edit = edit
                    .id_salt("input")
                    .desired_width(ui.available_width())
                    .font(font.clone())
                    .hint_text(message(locale, placeholder, &[]))
                    .margin(egui::Margin::ZERO)
                    .frame(egui::Frame::NONE)
                    .layouter(&mut layouter);
                if multiline {
                    return egui::ScrollArea::vertical()
                        .id_salt("body-scroll")
                        .min_scrolled_height(BODY_HEIGHT)
                        .max_height(BODY_HEIGHT)
                        .auto_shrink([false, false])
                        .show(ui, |ui| ui.add(edit))
                        .inner;
                }
                ui.add(edit)
            })
            .inner
            .labelled_by(label.id);
        response.widget_info(|| {
            egui::WidgetInfo::text_edit(
                response.enabled(),
                old.as_str(),
                value.as_str(),
                label_text.as_str(),
            )
        });
        trace(output, identity, &response);
        if let Some(hint) = hint {
            ui.label(
                RichText::new(message(locale, hint, &[]))
                    .size(HINT_FONT)
                    .color(appearance.muted),
            );
        }
    });
}

fn trace(_output: &mut Output, _name: &str, _response: &Response) {
    #[cfg(any(test, feature = "inspection"))]
    {
        _output
            .traces
            .push((_name.into(), _response.id, _response.rect));
        if _response.enabled() {
            _output
                .interactions
                .insert(_response.id, _response.interact_rect);
        }
    }
}

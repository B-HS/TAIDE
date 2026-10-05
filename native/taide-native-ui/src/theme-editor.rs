use std::collections::HashMap;

#[cfg(any(test, feature = "inspection"))]
use egui::Id;
use egui::{self, Align, Color32, Rect, Response, RichText, Stroke, Ui};
use taide_model::{
    error::{AppError, AppResult},
    locale::ResolvedLocale,
    theme::ResolvedTheme,
};

use crate::{
    icons::{Icon, Icons},
    presentation::{color, message, parse_color},
    settings_owner::Owner,
    theme_color_picker::{self, Picker},
    theme_draft::{ColorDomain, Draft, Mode, SyntaxPatch},
    theme_edit::{Command, DeleteRequest, LoadRequest, Reply, SaveRequest, Session},
    theme_editor_tokens::{COLORS, SYNTAX, TERMINAL},
};

const HEADER_X: i8 = 24;
const HEADER_Y: i8 = 16;
const BODY_X: i8 = 24;
const BODY_Y: i8 = 16;
const PREVIEW_X: i8 = 16;
const PREVIEW_WIDTH: f32 = 384.0;
const BODY_GAP: f32 = 16.0;
const ROW_GAP: f32 = 8.0;
const ACTION_GAP: f32 = 4.0;
const ROW_Y: i8 = 4;
const CARD_PADDING: i8 = 20;
const CARD_RADIUS: u8 = 8;
const ROW_FONT: f32 = 12.0;
const TITLE_FONT: f32 = 14.0;
const NAME_WIDTH: f32 = 192.0;
const BUTTON_HEIGHT: f32 = 32.0;
const ICON_BUTTON_SIZE: f32 = 24.0;
const BORDER_WIDTH: f32 = 1.0;
const MARGIN_SIDES: f32 = 2.0;
const MODAL_WIDTH: f32 = 512.0;

pub struct Appearance {
    background: Color32,
    foreground: Color32,
    muted: Color32,
    card: Color32,
    border: Color32,
    input: Color32,
    input_border: Color32,
    active: Color32,
    hover: Color32,
    primary: Color32,
    primary_foreground: Color32,
    error: Color32,
    picker: theme_color_picker::Appearance,
}

impl Appearance {
    pub fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        Ok(Self {
            background: color(theme, "app.background")?,
            foreground: color(theme, "app.foreground")?,
            muted: color(theme, "appSidebar.iconDefault")?,
            card: color(theme, "panel.background")?,
            border: color(theme, "app.border")?,
            input: color(theme, "panel.inputBackground")?,
            input_border: color(theme, "panel.inputBorder")?,
            active: color(theme, "appSidebar.itemActive")?,
            hover: color(theme, "appSidebar.itemHover")?,
            primary: color(theme, "button.primaryBackground")?,
            primary_foreground: color(theme, "button.primaryForeground")?,
            error: color(theme, "statusIndicator.error")?,
            picker: theme_color_picker::Appearance::new(theme)?,
        })
    }

    fn apply(&self, ui: &mut Ui) {
        ui.visuals_mut().override_text_color = Some(self.foreground);
        ui.visuals_mut().extreme_bg_color = self.input;
        ui.visuals_mut().widgets.inactive.bg_fill = self.background;
        ui.visuals_mut().widgets.inactive.weak_bg_fill = self.background;
        ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::new(BORDER_WIDTH, self.input_border);
        ui.visuals_mut().widgets.hovered.bg_fill = self.hover;
        ui.visuals_mut().widgets.hovered.weak_bg_fill = self.hover;
        ui.visuals_mut().widgets.active.bg_fill = self.active;
        ui.visuals_mut().widgets.active.weak_bg_fill = self.active;
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum PickerKey {
    Color(ColorDomain, String),
    Syntax(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dialog {
    Discard,
    Delete,
}

struct RowContext<'a> {
    locale: &'a ResolvedLocale,
    appearance: &'a Appearance,
    icons: &'a Icons,
}

pub struct Editor {
    session: Session,
    mode: Mode,
    draft: Option<Box<Draft>>,
    load: Option<LoadRequest>,
    initial_load: bool,
    save: Option<Box<SaveRequest>>,
    delete: Option<DeleteRequest>,
    search: String,
    pickers: HashMap<PickerKey, Picker>,
    dialog: Option<Dialog>,
    focus_cancel: bool,
}

#[derive(Default)]
pub struct Output {
    pub commands: Vec<Command>,
    pub close: bool,
    pub error: Option<AppError>,
    pub tooltips: Vec<crate::tooltip_trigger::Trigger>,
    #[cfg(any(test, feature = "inspection"))]
    pub traces: Vec<(String, Id, Rect)>,
}

pub struct Accepted {
    pub close: bool,
    pub mutated: bool,
    pub error: Option<AppError>,
}

impl Editor {
    pub fn preview(&self) -> Option<ResolvedTheme> {
        self.draft.as_ref().map(|draft| draft.preview())
    }

    pub fn new(owner: Owner, source: String, mode: Mode, name: String) -> AppResult<Self> {
        let session = Session::new(owner, source, mode)?;
        let load = session.load_request(name);
        Ok(Self {
            session,
            mode,
            draft: None,
            load: Some(load),
            initial_load: true,
            save: None,
            delete: None,
            search: String::new(),
            pickers: HashMap::new(),
            dialog: None,
            focus_cancel: false,
        })
    }

    pub fn accept(&mut self, reply: Reply) -> Option<Accepted> {
        let result = match reply {
            Reply::Loaded { request, result } => {
                if !self.session.owns_load(&request)
                    || !self
                        .load
                        .as_ref()
                        .is_some_and(|pending| pending.same_request(&request))
                {
                    return None;
                }
                self.load = None;
                match result {
                    Ok(draft) => {
                        self.draft = Some(draft);
                        Ok((false, false))
                    }
                    Err(error) => Err(error),
                }
            }
            Reply::Saved { request, result } => {
                if !self.session.owns_save(&request)
                    || !self
                        .save
                        .as_ref()
                        .is_some_and(|pending| pending.same_request(&request))
                {
                    return None;
                }
                self.save = None;
                result.map(|_| (true, true))
            }
            Reply::Deleted { request, result } => {
                if !self.session.owns_delete(&request)
                    || !self
                        .delete
                        .as_ref()
                        .is_some_and(|pending| pending.same_request(&request))
                {
                    return None;
                }
                self.delete = None;
                result.map(|_| (true, true))
            }
        };
        Some(match result {
            Ok((close, mutated)) => Accepted {
                close,
                mutated,
                error: None,
            },
            Err(error) => Accepted {
                close: false,
                mutated: false,
                error: Some(error),
            },
        })
    }

    fn request_close(&mut self, output: &mut Output) {
        if self
            .draft
            .as_ref()
            .is_some_and(|draft| draft.has_unsaved_changes())
        {
            self.dialog = Some(Dialog::Discard);
            self.focus_cancel = true;
        } else {
            output.close = true;
        }
    }

    fn request_save(&mut self, output: &mut Output) {
        let Some(draft) = self.draft.as_ref().filter(|draft| draft.is_valid()) else {
            return;
        };
        if self.save.is_some() {
            return;
        }
        match self.session.save_request(draft) {
            Ok(request) => {
                let request = Box::new(request);
                self.save = Some(request.clone());
                output.commands.push(Command::Save(request));
            }
            Err(error) => output.error = Some(error),
        }
    }

    fn request_delete(&mut self, output: &mut Output) {
        if self.delete.is_some() {
            return;
        }
        match self.session.delete_request() {
            Ok(request) => {
                self.delete = Some(request.clone());
                output.commands.push(Command::Delete(request));
            }
            Err(error) => output.error = Some(error),
        }
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        locale: &ResolvedLocale,
        appearance: &Appearance,
        icons: &Icons,
    ) -> Output {
        let mut output = Output::default();
        if self.initial_load && ui.is_enabled() {
            self.initial_load = false;
            if let Some(request) = &self.load {
                output.commands.push(Command::Load(request.clone()));
            }
        }
        let Some(draft) = self.draft.as_mut() else {
            ui.painter()
                .rect_filled(ui.available_rect_before_wrap(), 0, appearance.background);
            return output;
        };
        let container = ui.available_rect_before_wrap();
        let palette = match Appearance::new(draft.current()) {
            Ok(palette) => palette,
            Err(error) => {
                output.error = Some(error);
                return output;
            }
        };
        palette.apply(ui);
        let row_context = RowContext {
            locale,
            appearance: &palette,
            icons,
        };
        let mut back = false;
        let mut save = false;
        let mut delete = false;
        egui::Frame::NONE
            .fill(palette.background)
            .inner_margin(egui::Margin::symmetric(HEADER_X, HEADER_Y))
            .show(ui, |ui| {
                ui.set_min_width((container.width() - f32::from(HEADER_X) * MARGIN_SIDES).max(0.0));
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = ROW_GAP;
                    let response = ui.add_sized(
                        [0.0, BUTTON_HEIGHT],
                        egui::Button::new(message(locale, "themeEditor.backToSettings", &[]))
                            .frame(false),
                    );
                    back = response.clicked();
                    trace(&mut output, "back", &response);
                    let mut name = draft.current().name.clone();
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut name)
                            .id_salt("name")
                            .desired_width(NAME_WIDTH)
                            .font(egui::FontId::proportional(TITLE_FONT))
                            .hint_text(message(locale, "themeEditor.themeNamePlaceholder", &[])),
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::text_edit(
                            ui.is_enabled(),
                            draft.current().name.as_str(),
                            name.as_str(),
                            message(locale, "themeEditor.themeNamePlaceholder", &[]),
                        )
                    });
                    if response.changed() {
                        draft.rename(name);
                    }
                    trace(&mut output, "name", &response);
                    ui.label(
                        RichText::new(message(
                            locale,
                            "themeEditor.changedCount",
                            &[("count", &draft.changed_count().to_string())],
                        ))
                        .size(ROW_FONT)
                        .color(palette.muted),
                    );
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        let response = ui.add_enabled(
                            self.save.is_none() && draft.is_valid(),
                            egui::Button::new(
                                RichText::new(message(locale, "themeEditor.save", &[]))
                                    .color(palette.primary_foreground),
                            )
                            .fill(palette.primary)
                            .min_size(egui::vec2(0.0, BUTTON_HEIGHT)),
                        );
                        save = response.clicked();
                        trace(&mut output, "save", &response);
                        if self.mode == Mode::Edit {
                            let response = ui.add_enabled(
                                self.delete.is_none(),
                                egui::Button::new(message(locale, "themeEditor.deleteTheme", &[]))
                                    .min_size(egui::vec2(0.0, BUTTON_HEIGHT)),
                            );
                            delete = response.clicked();
                            trace(&mut output, "delete", &response);
                        }
                    });
                });
            });
        let body = Rect::from_min_max(
            egui::pos2(container.left(), ui.cursor().top()),
            container.right_bottom(),
        );
        ui.painter().hline(
            container.x_range(),
            body.top(),
            Stroke::new(BORDER_WIDTH, palette.border),
        );
        let split = (body.right() - PREVIEW_WIDTH).max(body.left());
        let left = Rect::from_min_max(body.min, egui::pos2(split, body.bottom()));
        let right = Rect::from_min_max(egui::pos2(split, body.top()), body.max);
        ui.scope_builder(
            egui::UiBuilder::new()
                .id_salt("tokens")
                .max_rect(left)
                .layout(egui::Layout::top_down(Align::Min)),
            |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("token-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(BODY_X, BODY_Y))
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = BODY_GAP;
                                let width =
                                    (left.width() - f32::from(BODY_X) * MARGIN_SIDES).max(0.0);
                                ui.set_width(width);
                                let response = ui.add(
                                    egui::TextEdit::singleline(&mut self.search)
                                        .id_salt("search")
                                        .desired_width(width)
                                        .font(egui::FontId::proportional(ROW_FONT))
                                        .hint_text(message(
                                            locale,
                                            "themeEditor.searchTokensPlaceholder",
                                            &[],
                                        )),
                                );
                                response.widget_info(|| {
                                    egui::WidgetInfo::text_edit(
                                        ui.is_enabled(),
                                        "",
                                        &self.search,
                                        message(locale, "themeEditor.searchTokensAriaLabel", &[]),
                                    )
                                });
                                trace(&mut output, "search", &response);
                                let query = self.search.trim().to_lowercase();
                                for (namespace, tokens) in COLORS {
                                    let tokens = tokens
                                        .iter()
                                        .filter(|token| {
                                            matches_query(&query, namespace)
                                                || matches_query(&query, token)
                                        })
                                        .collect::<Vec<_>>();
                                    if tokens.is_empty() {
                                        continue;
                                    }
                                    card(
                                        ui,
                                        &message(
                                            locale,
                                            &format!("themeEditor.ns.{namespace}"),
                                            &[],
                                        ),
                                        &palette,
                                        |ui| {
                                            for token in tokens {
                                                let key = format!("{namespace}.{token}");
                                                color_row(
                                                    ui,
                                                    draft,
                                                    &mut self.pickers,
                                                    (ColorDomain::Colors, &key, token),
                                                    &row_context,
                                                    &mut output,
                                                );
                                            }
                                        },
                                    );
                                }
                                let syntax = SYNTAX
                                    .iter()
                                    .filter(|token| matches_query(&query, token))
                                    .collect::<Vec<_>>();
                                if !syntax.is_empty() {
                                    card(
                                        ui,
                                        &message(locale, "themeEditor.syntaxSectionTitle", &[]),
                                        &palette,
                                        |ui| {
                                            for token in syntax {
                                                syntax_row(
                                                    ui,
                                                    draft,
                                                    &mut self.pickers,
                                                    token,
                                                    &row_context,
                                                    &mut output,
                                                );
                                            }
                                        },
                                    );
                                }
                                let terminal = TERMINAL
                                    .iter()
                                    .filter(|token| matches_query(&query, token))
                                    .collect::<Vec<_>>();
                                if !terminal.is_empty() {
                                    card(
                                        ui,
                                        &message(locale, "themeEditor.terminalSectionTitle", &[]),
                                        &palette,
                                        |ui| {
                                            for token in terminal {
                                                color_row(
                                                    ui,
                                                    draft,
                                                    &mut self.pickers,
                                                    (ColorDomain::Terminal, token, token),
                                                    &row_context,
                                                    &mut output,
                                                );
                                            }
                                        },
                                    );
                                }
                            });
                    });
            },
        );
        ui.painter().vline(
            split,
            body.y_range(),
            Stroke::new(BORDER_WIDTH, palette.border),
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .id_salt("preview")
                .max_rect(right)
                .layout(egui::Layout::top_down(Align::Min)),
            |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("preview-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(PREVIEW_X, BODY_Y))
                            .show(ui, |ui| {
                                ui.set_width(
                                    (right.width() - f32::from(PREVIEW_X) * MARGIN_SIDES).max(0.0),
                                );
                                ui.label(
                                    RichText::new(message(locale, "themeEditor.previewTitle", &[]))
                                        .size(ROW_FONT)
                                        .strong()
                                        .color(palette.muted),
                                );
                                ui.label(
                                    RichText::new(message(
                                        locale,
                                        "themeEditor.livePreviewHint",
                                        &[],
                                    ))
                                    .size(ROW_FONT)
                                    .color(palette.muted),
                                );
                                match crate::theme_live_preview::show(ui, draft.current(), locale) {
                                    Ok(tooltips) => output.tooltips.extend(tooltips),
                                    Err(error) => output.error = Some(error),
                                }
                            });
                    });
            },
        );
        ui.advance_cursor_after_rect(body);
        let query = self.search.trim().to_lowercase();
        self.pickers.retain(|key, _| match key {
            PickerKey::Color(ColorDomain::Colors, key) => {
                key.split_once('.').is_some_and(|(namespace, token)| {
                    matches_query(&query, namespace) || matches_query(&query, token)
                })
            }
            PickerKey::Color(ColorDomain::Terminal, token) | PickerKey::Syntax(token) => {
                matches_query(&query, token)
            }
        });
        if back {
            self.request_close(&mut output);
        }
        if save {
            self.request_save(&mut output);
        }
        if delete {
            self.dialog = Some(Dialog::Delete);
            self.focus_cancel = true;
        }
        self.show_dialog(ui, locale, &palette, &mut output);
        output
    }

    fn show_dialog(
        &mut self,
        ui: &Ui,
        locale: &ResolvedLocale,
        appearance: &Appearance,
        output: &mut Output,
    ) {
        let Some(dialog) = self.dialog else {
            return;
        };
        let id = ui.make_persistent_id("theme-confirmation");
        let mut cancel = false;
        let mut confirm = false;
        let response = egui::Modal::new(id).show(ui.ctx(), |ui| {
            appearance.apply(ui);
            ui.set_max_width(MODAL_WIDTH);
            let (title, description, action) = match dialog {
                Dialog::Discard => (
                    "common.unsavedChangesTitle",
                    "common.unsavedChangesDescription",
                    "common.discardChanges",
                ),
                Dialog::Delete => (
                    "themeEditor.deleteConfirmTitle",
                    "themeEditor.deleteConfirmDescription",
                    "themeEditor.deleteTheme",
                ),
            };
            ui.heading(message(locale, title, &[]));
            let name = self
                .draft
                .as_ref()
                .map_or("", |draft| draft.current().name.as_str());
            ui.label(message(locale, description, &[("name", name)]));
            ui.horizontal(|ui| {
                let response = ui.button(message(locale, "common.cancel", &[]));
                if self.focus_cancel {
                    response.request_focus();
                    self.focus_cancel = false;
                }
                cancel = response.clicked();
                trace(output, "dialog-cancel", &response);
                let response =
                    ui.button(RichText::new(message(locale, action, &[])).color(appearance.error));
                confirm = response.clicked();
                trace(output, "dialog-confirm", &response);
            });
        });
        if response.is_top_modal
            && !response.any_popup_open
            && ui
                .ctx()
                .input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            cancel = true;
        }
        if cancel {
            self.dialog = None;
        }
        if confirm {
            self.dialog = None;
            match dialog {
                Dialog::Discard => output.close = true,
                Dialog::Delete => self.request_delete(output),
            }
        }
    }
}

fn matches_query(query: &str, label: &str) -> bool {
    query.is_empty() || label.to_lowercase().contains(query)
}

fn card(ui: &mut Ui, title: &str, appearance: &Appearance, contents: impl FnOnce(&mut Ui)) {
    let width = ui.available_width();
    egui::Frame::NONE
        .fill(appearance.card)
        .stroke(Stroke::new(BORDER_WIDTH, appearance.border))
        .corner_radius(CARD_RADIUS)
        .inner_margin(CARD_PADDING)
        .show(ui, |ui| {
            ui.set_width(
                (width - f32::from(CARD_PADDING) * MARGIN_SIDES - BORDER_WIDTH * MARGIN_SIDES)
                    .max(0.0),
            );
            ui.spacing_mut().item_spacing.y = ROW_GAP;
            ui.label(RichText::new(title).size(TITLE_FONT).strong());
            contents(ui);
        });
}

pub fn icon_button(
    ui: &mut Ui,
    icons: &Icons,
    icon: Icon,
    color: Color32,
    label: &str,
    disabled: bool,
    tooltips: &mut Vec<crate::tooltip_trigger::Trigger>,
) -> Response {
    let (response, trigger) = crate::tooltip_trigger::wrap_button(ui, label, disabled, |ui| {
        ui.add_enabled(
            !disabled,
            egui::Button::opt_image_and_text(icons.image(icon, color), None)
                .frame(false)
                .min_size(egui::Vec2::splat(ICON_BUTTON_SIZE)),
        )
    });
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, response.enabled(), label)
    });
    tooltips.push(trigger);
    response
}

fn color_row(
    ui: &mut Ui,
    draft: &mut Draft,
    pickers: &mut HashMap<PickerKey, Picker>,
    token: (ColorDomain, &str, &str),
    context: &RowContext<'_>,
    output: &mut Output,
) {
    let RowContext {
        locale,
        appearance,
        icons,
    } = context;
    let (domain, key, label) = token;
    let changed = draft.color_changed(domain, key);
    let value = match domain {
        ColorDomain::Colors => draft.current().colors.get(key),
        ColorDomain::Terminal => draft.current().terminal.get(key),
    }
    .cloned();
    let Some(value) = value else {
        output.error = Some(AppError::Internal(format!(
            "native theme token is unavailable: {key}"
        )));
        return;
    };
    ui.push_id((domain, key), |ui| {
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(0, ROW_Y))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = ACTION_GAP;
                    let control = Picker::trigger_width(ui, &value, locale)
                        + if changed {
                            ICON_BUTTON_SIZE + ACTION_GAP
                        } else {
                            0.0
                        };
                    let width = (ui.available_width() - control - ROW_GAP).max(0.0);
                    let mut text = RichText::new(label)
                        .size(ROW_FONT)
                        .color(appearance.foreground);
                    if changed {
                        text = text.strong();
                    }
                    ui.add_sized([width, ICON_BUTTON_SIZE], egui::Label::new(text).truncate());
                    let mut reset = false;
                    if changed {
                        let response = icon_button(
                            ui,
                            icons,
                            Icon::ThemeReset,
                            appearance.muted,
                            &message(locale, "themeEditor.resetToken", &[]),
                            false,
                            &mut output.tooltips,
                        );
                        trace(output, &format!("reset-{domain:?}-{key}"), &response);
                        reset = response.clicked();
                    }
                    let picker_key = PickerKey::Color(domain, key.into());
                    let picker = pickers
                        .entry(picker_key)
                        .or_insert_with(|| Picker::new(value.clone()));
                    if let Some(value) = picker.show(
                        ui,
                        ui.make_persistent_id("picker"),
                        &value,
                        locale,
                        &appearance.picker,
                    ) && let Err(error) = draft.set_color(domain, key, value)
                    {
                        output.error = Some(error);
                    }
                    output.tooltips.extend(picker.take_tooltip());
                    #[cfg(any(test, feature = "inspection"))]
                    output
                        .traces
                        .extend(picker.input_traces().iter().enumerate().filter_map(
                            |(index, trace)| {
                                trace.map(|(id, rect)| {
                                    (format!("picker-{domain:?}-{key}-{index}"), id, rect)
                                })
                            },
                        ));
                    if reset && let Err(error) = draft.reset_color(domain, key) {
                        output.error = Some(error);
                    }
                });
            });
    });
}

fn syntax_row(
    ui: &mut Ui,
    draft: &mut Draft,
    pickers: &mut HashMap<PickerKey, Picker>,
    key: &str,
    context: &RowContext<'_>,
    output: &mut Output,
) {
    let RowContext {
        locale,
        appearance,
        icons,
    } = context;
    let Some(style) = draft.current().syntax.get(key).cloned() else {
        output.error = Some(AppError::Internal(format!(
            "native syntax token is unavailable: {key}"
        )));
        return;
    };
    let changed = draft.syntax_changed(key);
    let foreground = match parse_color(&style.fg, key) {
        Ok(color) => color,
        Err(error) => {
            output.error = Some(error);
            return;
        }
    };
    ui.push_id(("syntax", key), |ui| {
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(0, ROW_Y))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = ACTION_GAP;
                    let control = Picker::trigger_width(ui, &style.fg, locale)
                        + (ICON_BUTTON_SIZE + ACTION_GAP) * MARGIN_SIDES
                        + if changed {
                            ICON_BUTTON_SIZE + ACTION_GAP
                        } else {
                            0.0
                        };
                    let width = (ui.available_width() - control - ROW_GAP).max(0.0);
                    let mut text = RichText::new(key).size(ROW_FONT).color(foreground);
                    if style.bold || changed {
                        text = text.strong();
                    }
                    if style.italic {
                        text = text.italics();
                    }
                    ui.add_sized([width, ICON_BUTTON_SIZE], egui::Label::new(text).truncate());
                    for (field, selected, abbreviation, label) in [
                        (
                            "bold",
                            style.bold,
                            "themeEditor.boldAbbreviation",
                            "themeEditor.boldToggle",
                        ),
                        (
                            "italic",
                            style.italic,
                            "themeEditor.italicAbbreviation",
                            "themeEditor.italicToggle",
                        ),
                    ] {
                        let label = message(locale, label, &[]);
                        let abbreviation =
                            RichText::new(message(locale, abbreviation, &[])).size(ROW_FONT);
                        let abbreviation = if field == "bold" {
                            abbreviation.strong()
                        } else {
                            abbreviation.italics()
                        };
                        let (response, trigger) =
                            crate::tooltip_trigger::wrap_button(ui, &label, false, |ui| {
                                ui.add(
                                    egui::Button::new(abbreviation)
                                        .selected(selected)
                                        .min_size(egui::Vec2::splat(ICON_BUTTON_SIZE)),
                                )
                            });
                        response.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::SelectableLabel,
                                ui.is_enabled(),
                                selected,
                                &label,
                            )
                        });
                        trace(output, &format!("{field}-{key}"), &response);
                        output.tooltips.push(trigger);
                        if response.clicked() {
                            let patch = if field == "bold" {
                                SyntaxPatch {
                                    bold: Some(!selected),
                                    ..Default::default()
                                }
                            } else {
                                SyntaxPatch {
                                    italic: Some(!selected),
                                    ..Default::default()
                                }
                            };
                            if let Err(error) = draft.set_syntax(key, patch) {
                                output.error = Some(error);
                            }
                        }
                    }
                    let mut reset = false;
                    if changed {
                        let response = icon_button(
                            ui,
                            icons,
                            Icon::ThemeReset,
                            appearance.muted,
                            &message(locale, "themeEditor.resetToken", &[]),
                            false,
                            &mut output.tooltips,
                        );
                        trace(output, &format!("reset-Syntax-{key}"), &response);
                        reset = response.clicked();
                    }
                    let picker = pickers
                        .entry(PickerKey::Syntax(key.into()))
                        .or_insert_with(|| Picker::new(style.fg.clone()));
                    if let Some(value) = picker.show(
                        ui,
                        ui.make_persistent_id("picker"),
                        &style.fg,
                        locale,
                        &appearance.picker,
                    ) && let Err(error) = draft.set_syntax(
                        key,
                        SyntaxPatch {
                            fg: Some(value),
                            ..Default::default()
                        },
                    ) {
                        output.error = Some(error);
                    }
                    output.tooltips.extend(picker.take_tooltip());
                    if reset && let Err(error) = draft.reset_syntax(key) {
                        output.error = Some(error);
                    }
                });
            });
    });
}

fn trace(output: &mut Output, key: &str, response: &Response) {
    #[cfg(any(test, feature = "inspection"))]
    output.traces.push((key.into(), response.id, response.rect));
    #[cfg(not(any(test, feature = "inspection")))]
    let _ = (output, key, response);
}

#[cfg(all(test, feature = "native-host"))]
#[path = "theme-editor-tests.rs"]
mod tests;

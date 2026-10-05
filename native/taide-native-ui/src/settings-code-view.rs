use super::*;
use crate::settings_code_controls::{
    Change as CodeChange, Font, Selection, Switch as CodeSwitch, Text, TextDraft,
};
use crate::settings_resources::Resources;

const TEXT_WIDTH: f32 = 224.0;
const OPTION_WIDTH: f32 = 128.0;
const COMBO_INSET_X: f32 = 11.0;
const COMBO_ICON_GAP: f32 = 6.0;
const INPUT_PADDING_X: i8 = 8;
const INPUT_PADDING_Y: i8 = 4;
const LIST_PADDING: i8 = 4;
const LIST_ROW_HEIGHT: f32 = 28.0;
const SHELL_INSET_X: f32 = 13.0;
const SHELL_INSET_Y: f32 = 7.0;
const SHELL_CHECK_GAP: f32 = 8.0;
const SHELL_GAP: f32 = 4.0;
const FONT_SEARCH_HEIGHT: f32 = 36.0;
const SEARCH_PADDING: i8 = 12;
const SEARCH_ICON_SIZE: f32 = 16.0;
const SEARCH_ICON_VIEWBOX: f32 = 24.0;
const SEARCH_ICON_CENTER: f32 = 11.0;
const SEARCH_ICON_RADIUS: f32 = 8.0;
const SEARCH_ICON_STROKE: f32 = 2.0;
const SEARCH_ICON_LINE: [[f32; 2]; 2] = [[16.66, 16.66], [21.0, 21.0]];
const LIST_ROW_RADIUS: u8 = 4;
const EMPTY_PADDING: f32 = 16.0;
const HINT_GAP: f32 = 4.0;
const SYSTEM_FONT_VALUE: &str = "system-default";
const POPUP_OFFSET: f32 = 4.0;
const POPUP_SHADOW_OFFSET: [i8; 2] = [0, 2];
const POPUP_SHADOW_BLUR: u8 = 8;
const PICKER_EVENTS: egui::EventFilter = egui::EventFilter {
    tab: false,
    horizontal_arrows: true,
    vertical_arrows: true,
    escape: true,
};
const POPUP_EVENTS: egui::EventFilter = egui::EventFilter {
    tab: true,
    ..PICKER_EVENTS
};

pub(super) struct Appearance {
    input_background: Color32,
    input_border: Color32,
    popup_background: Color32,
    popup_border: Color32,
    popup_separator: Color32,
    popup_shadow: Color32,
    pub(super) trigger_hover: Color32,
    list_active: Color32,
    pub(super) list_foreground: Color32,
}

impl Appearance {
    pub(super) fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        Ok(Self {
            input_background: color(theme, "panel.inputBackground")?,
            input_border: color(theme, "panel.inputBorder")?,
            popup_background: color(theme, "popover.background")?,
            popup_border: color(theme, "popover.border")?,
            popup_separator: color(theme, "popover.separator")?,
            popup_shadow: color(theme, "app.shadow")?,
            trigger_hover: color(theme, "list.hoverBackground")?,
            list_active: color(theme, "list.activeBackground")?,
            list_foreground: color(theme, "list.foreground")?,
        })
    }
}

pub struct State {
    editor_font_size: NumericDraft,
    terminal_font_size: NumericDraft,
    auto_save: NumericDraft,
    tab_size: NumericDraft,
    scrollback: NumericDraft,
    rulers: TextDraft,
    shell: TextDraft,
    editor_font: FontPicker,
    terminal_font: FontPicker,
    whitespace: Picker,
    editor_cursor: Picker,
    editor_blinking: Picker,
    terminal_cursor: Picker,
}

impl State {
    pub(super) fn new(settings: &Settings) -> Self {
        Self {
            editor_font_size: NumericDraft::new(settings.editor_font_size),
            terminal_font_size: NumericDraft::new(settings.terminal_font_size),
            auto_save: NumericDraft::new(settings.auto_save_delay_ms),
            tab_size: NumericDraft::new(settings.editor_tab_size),
            scrollback: NumericDraft::new(settings.terminal_scrollback),
            rulers: TextDraft::new(Text::Rulers.value(settings)),
            shell: TextDraft::new(Text::Shell.value(settings)),
            editor_font: FontPicker::default(),
            terminal_font: FontPicker::default(),
            whitespace: Picker::default(),
            editor_cursor: Picker::default(),
            editor_blinking: Picker::default(),
            terminal_cursor: Picker::default(),
        }
    }

    pub(super) fn show(
        &mut self,
        ui: &mut Ui,
        section: Section,
        settings: &Settings,
        locale: &ResolvedLocale,
        appearance: &super::Appearance,
        resources: &Resources,
        font_previews: &mut crate::font_preview::Previews,
        output: &mut Output,
    ) {
        ui.scope(|ui| {
            ui.visuals_mut().extreme_bg_color = appearance.code.input_background;
            ui.visuals_mut().widgets.inactive.bg_stroke =
                Stroke::new(BORDER_WIDTH, appearance.code.input_border);
            if section == Section::Terminal {
                numeric(
                    ui,
                    Numeric::TerminalFontSize,
                    &mut self.terminal_font_size,
                    settings,
                    locale,
                    appearance,
                    output,
                );
                self.terminal_font.show(
                    ui,
                    Font::Terminal,
                    settings,
                    resources,
                    locale,
                    appearance,
                    font_previews,
                    output,
                );
                text_field(
                    ui,
                    Text::Shell,
                    &mut self.shell,
                    settings,
                    locale,
                    appearance,
                    output,
                );
                shell_profiles(ui, settings, resources, locale, appearance, output);
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.y = HINT_GAP;
                    numeric(
                        ui,
                        Numeric::TerminalScrollback,
                        &mut self.scrollback,
                        settings,
                        locale,
                        appearance,
                        output,
                    );
                    hint(ui, "settings.terminalScrollbackHint", locale, appearance);
                });
                option(
                    ui,
                    Selection::TerminalCursor(settings.terminal_cursor_style),
                    &mut self.terminal_cursor,
                    locale,
                    appearance,
                    output,
                );
                code_switch(
                    ui,
                    CodeSwitch::TerminalCursorBlink,
                    settings,
                    locale,
                    appearance,
                    output,
                );
                return;
            }
            numeric(
                ui,
                Numeric::EditorFontSize,
                &mut self.editor_font_size,
                settings,
                locale,
                appearance,
                output,
            );
            self.editor_font.show(
                ui,
                Font::Editor,
                settings,
                resources,
                locale,
                appearance,
                font_previews,
                output,
            );
            for field in [
                CodeSwitch::FormatOnSave,
                CodeSwitch::OrganizeImportsOnSave,
                CodeSwitch::FixAllOnSave,
                CodeSwitch::TrimTrailingWhitespaceOnSave,
                CodeSwitch::InsertFinalNewlineOnSave,
                CodeSwitch::EditorConfigEnabled,
                CodeSwitch::EditorCodeLens,
            ] {
                code_switch(ui, field, settings, locale, appearance, output);
            }
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.y = HINT_GAP;
                numeric(
                    ui,
                    Numeric::AutoSaveDelay,
                    &mut self.auto_save,
                    settings,
                    locale,
                    appearance,
                    output,
                );
                hint(ui, "settings.autoSaveDelayHint", locale, appearance);
            });
            for field in [CodeSwitch::EditorWordWrap, CodeSwitch::EditorLineNumbers] {
                code_switch(ui, field, settings, locale, appearance, output);
            }
            numeric(
                ui,
                Numeric::EditorTabSize,
                &mut self.tab_size,
                settings,
                locale,
                appearance,
                output,
            );
            for field in [
                CodeSwitch::EditorInsertSpaces,
                CodeSwitch::EditorDetectIndentation,
            ] {
                code_switch(ui, field, settings, locale, appearance, output);
            }
            option(
                ui,
                Selection::Whitespace(settings.editor_render_whitespace),
                &mut self.whitespace,
                locale,
                appearance,
                output,
            );
            for field in [
                CodeSwitch::EditorBracketPairColorization,
                CodeSwitch::EditorBracketPairGuides,
            ] {
                code_switch(ui, field, settings, locale, appearance, output);
            }
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.y = HINT_GAP;
                text_field(
                    ui,
                    Text::Rulers,
                    &mut self.rulers,
                    settings,
                    locale,
                    appearance,
                    output,
                );
                hint(ui, "settings.editorRulersHint", locale, appearance);
            });
            code_switch(
                ui,
                CodeSwitch::EditorFontLigatures,
                settings,
                locale,
                appearance,
                output,
            );
            option(
                ui,
                Selection::EditorCursor(settings.editor_cursor_style),
                &mut self.editor_cursor,
                locale,
                appearance,
                output,
            );
            option(
                ui,
                Selection::EditorBlinking(settings.editor_cursor_blinking),
                &mut self.editor_blinking,
                locale,
                appearance,
                output,
            );
            for field in [
                CodeSwitch::EditorCursorSmoothCaretAnimation,
                CodeSwitch::EditorScrollBeyondLastLine,
                CodeSwitch::EditorSmoothScrolling,
                CodeSwitch::EditorStickyScroll,
                CodeSwitch::EditorSemanticHighlighting,
                CodeSwitch::EditorFormatOnType,
                CodeSwitch::EditorFormatOnPaste,
                CodeSwitch::EditorSuggestPreview,
                CodeSwitch::EmmetEnabled,
                CodeSwitch::EditorDiffHideUnchangedRegions,
                CodeSwitch::EditorDiffShowMoves,
            ] {
                code_switch(ui, field, settings, locale, appearance, output);
            }
        });
    }
}

fn code_switch(
    ui: &mut Ui,
    field: CodeSwitch,
    settings: &Settings,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    output: &mut Output,
) {
    let checked = field.value(settings);
    if switch_control(
        ui,
        field.label(),
        field.description(),
        checked,
        locale,
        appearance,
        output,
    ) {
        output
            .changes
            .push(Change::Code(CodeChange::Switch(field, !checked)));
    }
}

fn hint(ui: &mut Ui, key: &str, locale: &ResolvedLocale, appearance: &super::Appearance) {
    ui.label(
        RichText::new(message(locale, key, &[]))
            .size(TEXT_SIZE)
            .color(appearance.muted),
    );
}

fn left_label(
    ui: &mut Ui,
    width: f32,
    key: &str,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
) {
    ui.allocate_ui_with_layout(
        egui::vec2(width, LINE_HEIGHT),
        egui::Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_min_width(width);
            ui.add(
                egui::Label::new(
                    RichText::new(message(locale, key, &[]))
                        .size(TEXT_SIZE)
                        .color(appearance.foreground),
                )
                .halign(Align::Min)
                .truncate(),
            );
        },
    );
}

fn text_field(
    ui: &mut Ui,
    field: Text,
    draft: &mut TextDraft,
    settings: &Settings,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    output: &mut Output,
) {
    let stored = field.value(settings);
    draft.sync(stored.clone());
    ui.push_id(field.label(), |ui| {
        let width = ui.available_width();
        ui.horizontal(|ui| {
            left_label(
                ui,
                (width - TEXT_WIDTH - CONTROL_GAP).max(0.0),
                field.label(),
                locale,
                appearance,
            );
            let placeholder = match field {
                Text::Rulers => message(locale, "settings.editorRulersPlaceholder", &[]),
                Text::Shell => "/bin/zsh".into(),
            };
            let response = ui.add_sized(
                [TEXT_WIDTH, LINE_HEIGHT],
                egui::TextEdit::singleline(&mut draft.text)
                    .id_salt((field.label(), &stored))
                    .return_key(None)
                    .font(FontId::proportional(TEXT_SIZE))
                    .hint_text(placeholder)
                    .margin(egui::Margin::symmetric(INPUT_PADDING_X, INPUT_PADDING_Y)),
            );
            #[cfg(any(test, feature = "inspection"))]
            output.traces.push(Trace::capture(field.label(), &response));
            if response.lost_focus() {
                output.changes.push(Change::Code(draft.commit(field)));
            }
        });
    });
}

fn shell_profiles(
    ui: &mut Ui,
    settings: &Settings,
    resources: &Resources,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    output: &mut Output,
) {
    let Some(profiles) = resources.shells() else {
        hint(ui, "settings.loading", locale, appearance);
        return;
    };
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = SHELL_GAP;
        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
            node.set_role(egui::accesskit::Role::List);
        });
        for profile in profiles {
            ui.push_id(("shell-profile", &profile.id), |ui| {
                ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
                    node.set_role(egui::accesskit::Role::ListItem);
                });
                let active = settings.shell_override.as_deref() == Some(profile.path.as_str());
                let width = ui.available_width();
                let check_width = if active {
                    CHECK_SIZE + SHELL_CHECK_GAP
                } else {
                    0.0
                };
                let text_width = (width - SHELL_INSET_X / HALF - check_width).max(0.0);
                let mut name = egui::text::LayoutJob::simple(
                    profile.name.clone(),
                    FontId::new(TEXT_SIZE, crate::font_families::medium(ui)),
                    appearance.foreground,
                    text_width,
                );
                name.wrap.max_rows = 1;
                name.wrap.break_anywhere = true;
                for section in &mut name.sections {
                    section.format.line_height = Some(LINE_HEIGHT);
                }
                let mut path = egui::text::LayoutJob::simple(
                    profile.path.clone(),
                    FontId::monospace(TEXT_SIZE),
                    appearance.muted,
                    text_width,
                );
                path.wrap.break_anywhere = true;
                for section in &mut path.sections {
                    section.format.line_height = Some(LINE_HEIGHT);
                }
                let name = ui.fonts_mut(|fonts| fonts.layout_job(name));
                let path = ui.fonts_mut(|fonts| fonts.layout_job(path));
                let height = SHELL_INSET_Y / HALF + name.size().y + path.size().y;
                let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());
                let response = ui.interact(rect, ui.make_persistent_id("select"), Sense::click());
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::Button,
                        ui.is_enabled(),
                        active,
                        format!("{} {}", profile.name, profile.path),
                    )
                });
                ui.painter().rect(
                    rect,
                    ROW_RADIUS,
                    if active {
                        appearance.active
                    } else if response.hovered() {
                        appearance.hover
                    } else {
                        Color32::TRANSPARENT
                    },
                    Stroke::new(
                        BORDER_WIDTH,
                        if active || response.has_focus() {
                            appearance.focus
                        } else {
                            appearance.border
                        },
                    ),
                    egui::StrokeKind::Inside,
                );
                let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
                let position = rect.min + egui::vec2(SHELL_INSET_X, SHELL_INSET_Y);
                let path_position = position + egui::vec2(0.0, name.size().y);
                painter.galley(position, name, appearance.foreground);
                painter.galley(path_position, path, appearance.muted);
                if active {
                    check(
                        ui,
                        Rect::from_center_size(
                            egui::pos2(
                                rect.right() - SHELL_INSET_X - CHECK_SIZE * HALF,
                                rect.center().y,
                            ),
                            egui::Vec2::splat(CHECK_SIZE),
                        ),
                        appearance.accent,
                    );
                }
                #[cfg(any(test, feature = "inspection"))]
                output
                    .traces
                    .push(Trace::capture("shell-profile", &response));
                if response.clicked() {
                    output
                        .changes
                        .push(Change::Code(CodeChange::Shell(profile.path.clone())));
                }
            });
        }
    });
}

pub(crate) struct Choice<T> {
    pub(crate) identity: String,
    pub(crate) value: T,
    pub(crate) label: String,
    pub(crate) search: String,
    pub(crate) font: Option<String>,
}

#[derive(Default)]
pub(crate) struct Picker {
    open: bool,
    cursor: usize,
    search: String,
    focus_content: bool,
    composing: bool,
    motion: Option<crate::tooltips::motion::Motion>,
    layer: Option<(egui::Context, egui::LayerId)>,
    restore_focus: bool,
    content_focused: bool,
}

impl Drop for Picker {
    fn drop(&mut self) {
        self.clear_layer();
    }
}

impl Picker {
    fn clear_layer(&mut self) {
        if let Some((context, layer)) = self.layer.take() {
            context.set_transform_layer(layer, egui::emath::TSTransform::IDENTITY);
            context.unregister_dismissal_layer(layer.id);
        }
    }

    pub(crate) fn show<T: Clone + PartialEq>(
        &mut self,
        ui: &mut Ui,
        label_key: &'static str,
        active_text: &str,
        active: &T,
        choices: &[Choice<T>],
        active_font: Option<&str>,
        mut font_previews: Option<&mut crate::font_preview::Previews>,
        searchable: bool,
        align: egui::emath::RectAlign,
        locale: &ResolvedLocale,
        appearance: &super::Appearance,
        _output: &mut Output,
    ) -> Option<T> {
        let id = ui.make_persistent_id((label_key, "picker"));
        let popup_area = id.with(("popup", ui.ctx().viewport_id()));
        let popup_layer = egui::LayerId::new(egui::Order::Foreground, popup_area);
        let now = ui.input(|input| input.time);
        let was_present = self.motion.is_some_and(|motion| motion.is_present(now));
        if self.open || was_present {
            ui.ctx().register_dismissal_layer(popup_area);
        }
        let focused = ui.memory(|memory| memory.focused());
        let owns_focus = focused
            .and_then(|id| ui.ctx().read_response(id))
            .is_some_and(|response| response.layer_id == popup_layer);
        if (self.open || self.motion.is_some())
            && self.content_focused
            && focused.is_some()
            && focused != Some(id)
            && !owns_focus
        {
            self.open = false;
            self.restore_focus = false;
        }
        let mut ime_frame = false;
        if ui.is_enabled() && ui.memory(|memory| memory.has_focus(id.with("search"))) {
            ui.input(|input| {
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
            });
        } else {
            self.composing = false;
        }
        let keyboard_enabled = ui.is_enabled() && !self.composing && !ime_frame;
        let popup_keyboard = keyboard_enabled
            && owns_focus
            && ui.memory(|memory| memory.allows_interaction(popup_layer));
        let guarded_escape =
            !keyboard_enabled && ui.input(|input| input.key_pressed(egui::Key::Escape));
        let enter = (self.open || was_present)
            && popup_keyboard
            && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        let escape = (self.open || was_present)
            && popup_keyboard
            && ui.ctx().dismissal_layers().last() == Some(&popup_area)
            && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
        let [down, up, home, end] = [
            egui::Key::ArrowDown,
            egui::Key::ArrowUp,
            egui::Key::Home,
            egui::Key::End,
        ]
        .map(|key| {
            (self.open || was_present)
                && popup_keyboard
                && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key))
        });
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), COMBO_TRIGGER_HEIGHT),
            Sense::hover(),
        );
        let trigger = ui.interact(rect, id, Sense::click());
        trigger.widget_info(|| {
            let mut info = egui::WidgetInfo::labeled(
                egui::WidgetType::ComboBox,
                ui.is_enabled(),
                message(
                    locale,
                    if searchable {
                        "settings.fontFamilySelectPlaceholder"
                    } else {
                        label_key
                    },
                    &[],
                ),
            );
            info.current_text_value = Some(active_text.into());
            info
        });
        #[cfg(any(test, feature = "inspection"))]
        _output.traces.push(Trace::capture(label_key, &trigger));
        ui.painter().rect(
            rect,
            ROW_RADIUS,
            if trigger.hovered() {
                appearance.code.trigger_hover
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
        let label_rect = Rect::from_min_size(
            rect.min + egui::vec2(COMBO_INSET_X, (COMBO_TRIGGER_HEIGHT - LINE_HEIGHT) * HALF),
            egui::vec2(
                (rect.width() - COMBO_INSET_X / HALF - CHECK_SIZE - COMBO_ICON_GAP).max(0.0),
                LINE_HEIGHT,
            ),
        );
        font_text(
            ui,
            active_text,
            active_font,
            label_rect,
            if trigger.hovered() {
                appearance.code.list_foreground
            } else {
                appearance.foreground
            },
            font_previews.as_deref_mut(),
            _output,
        );
        let icon = Rect::from_center_size(
            egui::pos2(
                rect.right() - COMBO_INSET_X - CHECK_SIZE * HALF,
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
        let opened_with_arrow = !self.open
            && keyboard_enabled
            && trigger.has_focus()
            && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));
        if trigger.clicked() || opened_with_arrow {
            self.open = opened_with_arrow || !self.open;
            if self.open && !was_present {
                self.search.clear();
                self.cursor = 0;
                self.focus_content = true;
                self.composing = false;
                self.restore_focus = true;
                self.content_focused = false;
            }
            ui.memory_mut(|memory| memory.request_focus_with_filter(id, PICKER_EVENTS));
        }
        if escape {
            self.open = false;
        }
        if self.open && self.motion.is_none() {
            self.motion = Some(crate::tooltips::motion::Motion::new(true, now));
        }
        if let Some(motion) = &mut self.motion {
            motion.target(self.open, now);
        }
        let present = self.motion.is_some_and(|motion| motion.is_present(now));
        if !present {
            if self.motion.take().is_some() && self.restore_focus && ui.is_enabled() {
                ui.memory_mut(|memory| memory.request_focus_with_filter(id, PICKER_EVENTS));
            }
            self.clear_layer();
            self.focus_content = false;
            self.content_focused = false;
        }
        let mut selected = None;
        let mut rows = Vec::new();
        let mut popup_id = None;
        let enabled = ui.is_enabled();
        let popup = egui::Popup::from_response(&trigger)
            .id(popup_area)
            .open(present)
            .align(align)
            .gap(POPUP_OFFSET)
            .fade_in(false)
            .width(rect.width())
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .frame(egui::Frame::NONE);
        let placement = popup.get_best_align();
        let mut bounds = Vec::new();
        let shown = popup.show(|ui| {
            let layer = ui.layer_id();
            let start = ui
                .ctx()
                .graphics_mut(|graphics| graphics.entry(layer).next_idx());
            if self
                .layer
                .as_ref()
                .is_some_and(|(_, previous)| *previous != layer)
            {
                self.clear_layer();
            }
            self.layer = Some((ui.ctx().clone(), layer));
            ui.ctx().register_dismissal_layer(popup_area);
            let frame = egui::Frame::NONE
                .fill(appearance.code.popup_background)
                .stroke(Stroke::new(BORDER_WIDTH, appearance.code.popup_border))
                .shadow(egui::Shadow {
                    offset: POPUP_SHADOW_OFFSET,
                    blur: POPUP_SHADOW_BLUR,
                    spread: 0,
                    color: appearance.code.popup_shadow,
                })
                .corner_radius(ROW_RADIUS)
                .inner_margin(0)
                .show(ui, |ui| {
                    ui.scope_builder(
                        egui::UiBuilder::new()
                            .id_salt("dialog")
                            .sense(Sense::focusable_noninteractive()),
                        |ui| {
                            if !enabled {
                                ui.disable();
                            }
                            popup_id = Some(ui.unique_id());
                            ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
                                node.set_role(egui::accesskit::Role::Dialog);
                            });
                            ui.set_min_width((rect.width() - BORDER_WIDTH / HALF).max(0.0));
                            ui.spacing_mut().item_spacing.y = 0.0;
                            let mut input_id = None;
                            if searchable {
                                let row = egui::Frame::NONE
                                    .inner_margin(egui::Margin::symmetric(SEARCH_PADDING, 0))
                                    .show(ui, |ui| {
                                        ui.allocate_ui_with_layout(
                                            egui::vec2(ui.available_width(), FONT_SEARCH_HEIGHT),
                                            egui::Layout::left_to_right(Align::Center),
                                            |ui| {
                                                ui.spacing_mut().item_spacing.x = CONTROL_GAP;
                                                let (icon, _) = ui.allocate_exact_size(
                                                    egui::Vec2::splat(SEARCH_ICON_SIZE),
                                                    Sense::hover(),
                                                );
                                                let scale = SEARCH_ICON_SIZE / SEARCH_ICON_VIEWBOX;
                                                let stroke = Stroke::new(
                                                    SEARCH_ICON_STROKE * scale,
                                                    appearance.foreground.gamma_multiply(HALF),
                                                );
                                                ui.painter().circle_stroke(
                                                    icon.min
                                                        + egui::Vec2::splat(
                                                            SEARCH_ICON_CENTER * scale,
                                                        ),
                                                    SEARCH_ICON_RADIUS * scale,
                                                    stroke,
                                                );
                                                ui.painter().line_segment(
                                                    SEARCH_ICON_LINE.map(|[x, y]| {
                                                        icon.min + egui::vec2(x * scale, y * scale)
                                                    }),
                                                    stroke,
                                                );
                                                ui.add(
                                        egui::TextEdit::singleline(&mut self.search)
                                            .id(id.with("search"))
                                            .return_key(None)
                                            .frame(egui::Frame::NONE)
                                            .margin(egui::Margin::ZERO)
                                            .min_size(egui::vec2(0.0, FONT_SEARCH_HEIGHT))
                                            .vertical_align(Align::Center)
                                            .desired_width(ui.available_width())
                                            .text_color(appearance.foreground)
                                            .hint_text(
                                                RichText::new(message(
                                                    locale,
                                                    "settings.fontFamilySearchPlaceholder",
                                                    &[],
                                                ))
                                                .color(appearance.muted),
                                            )
                                            .font(FontId::proportional(TEXT_SIZE)),
                                    )
                                            },
                                        )
                                        .inner
                                    });
                                ui.painter().line_segment(
                                    [
                                        row.response.rect.left_bottom(),
                                        row.response.rect.right_bottom(),
                                    ],
                                    Stroke::new(BORDER_WIDTH, appearance.code.popup_separator),
                                );
                                let response = row.inner;
                                input_id = Some(response.id);
                                bounds.push((response.id, response.rect));
                                #[cfg(any(test, feature = "inspection"))]
                                _output
                                    .traces
                                    .push(Trace::capture("font-search", &response));
                                if self.focus_content
                                    && ui.is_visible()
                                    && !ui.is_sizing_pass()
                                    && ui.is_enabled()
                                {
                                    ui.memory_mut(|memory| {
                                        memory.request_focus_with_filter(response.id, POPUP_EVENTS)
                                    });
                                    ui.ctx().request_repaint();
                                    self.focus_content = false;
                                    self.content_focused = true;
                                }
                                if response.changed() {
                                    self.cursor = 0;
                                }
                                ui.memory_mut(|memory| {
                                    memory.set_focus_lock_filter(response.id, POPUP_EVENTS)
                                });
                            }
                            for (index, choice) in choices.iter().enumerate() {
                                let score = if self.search.is_empty() {
                                    1.0
                                } else {
                                    crate::command_score::score(
                                        crate::settings_code_controls::trim_text(&choice.search),
                                        &self.search,
                                    )
                                };
                                if score > 0.0 {
                                    rows.push((index, score));
                                }
                            }
                            if !self.search.is_empty() {
                                rows.sort_by(|left, right| right.1.total_cmp(&left.1));
                            }
                            self.cursor = self.cursor.min(rows.len().saturating_sub(1));
                            if ui.is_enabled() {
                                if down {
                                    self.cursor =
                                        (self.cursor + 1).min(rows.len().saturating_sub(1));
                                }
                                if up {
                                    self.cursor = self.cursor.saturating_sub(1);
                                }
                                if home {
                                    self.cursor = 0;
                                }
                                if end {
                                    self.cursor = rows.len().saturating_sub(1);
                                }
                                if enter && let Some((index, _)) = rows.get(self.cursor) {
                                    selected = Some(choices[*index].value.clone());
                                }
                            }
                            let mut list_id = None;
                            let mut option_ids = Vec::new();
                            egui::ScrollArea::vertical()
                                .id_salt((label_key, "options"))
                                .max_height(COMBO_HEIGHT)
                                .show(ui, |ui| {
                                    egui::Frame::NONE.inner_margin(LIST_PADDING).show(ui, |ui| {
                                        list_id = Some(ui.unique_id());
                                        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
                                            node.set_role(egui::accesskit::Role::ListBox);
                                            node.set_label("Suggestions");
                                        });
                                        ui.spacing_mut().item_spacing.y = 0.0;
                                        if rows.is_empty() && searchable {
                                            ui.add_space(EMPTY_PADDING);
                                            hint(
                                                ui,
                                                "settings.fontFamilyNoResults",
                                                locale,
                                                appearance,
                                            );
                                            ui.add_space(EMPTY_PADDING);
                                        }
                                        for (row, (index, _)) in rows.iter().enumerate() {
                                            let choice = &choices[*index];
                                            ui.push_id((label_key, &choice.identity), |ui| {
                                                let (rect, _) = ui.allocate_exact_size(
                                                    egui::vec2(
                                                        ui.available_width(),
                                                        LIST_ROW_HEIGHT,
                                                    ),
                                                    Sense::hover(),
                                                );
                                                let response = ui.interact(
                                                    rect,
                                                    id.with(("option", &choice.identity)),
                                                    Sense::CLICK,
                                                );
                                                bounds.push((response.id, response.rect));
                                                option_ids.push(response.id);
                                                response.widget_info(|| {
                                                    egui::WidgetInfo::labeled(
                                                        egui::WidgetType::SelectableLabel,
                                                        ui.is_enabled(),
                                                        &choice.label,
                                                    )
                                                });
                                                ui.painter().rect(
                                                    rect,
                                                    LIST_ROW_RADIUS,
                                                    if row == self.cursor {
                                                        appearance.code.list_active
                                                    } else {
                                                        Color32::TRANSPARENT
                                                    },
                                                    if row == self.cursor {
                                                        Stroke::new(BORDER_WIDTH, appearance.accent)
                                                    } else {
                                                        Stroke::NONE
                                                    },
                                                    egui::StrokeKind::Inside,
                                                );
                                                let check_width = if &choice.value == active {
                                                    CHECK_SIZE + f32::from(INPUT_PADDING_X)
                                                } else {
                                                    0.0
                                                };
                                                let label_rect = Rect::from_min_size(
                                                    rect.min
                                                        + egui::vec2(
                                                            f32::from(INPUT_PADDING_X),
                                                            (LIST_ROW_HEIGHT - LINE_HEIGHT) * HALF,
                                                        ),
                                                    egui::vec2(
                                                        (rect.width()
                                                            - f32::from(INPUT_PADDING_X) / HALF
                                                            - check_width)
                                                            .max(0.0),
                                                        LINE_HEIGHT,
                                                    ),
                                                );
                                                font_text(
                                                    ui,
                                                    &choice.label,
                                                    choice.font.as_deref(),
                                                    label_rect,
                                                    if row == self.cursor {
                                                        appearance.code.list_foreground
                                                    } else {
                                                        appearance.foreground
                                                    },
                                                    font_previews.as_deref_mut(),
                                                    _output,
                                                );
                                                if &choice.value == active {
                                                    check(
                                                        ui,
                                                        Rect::from_center_size(
                                                            egui::pos2(
                                                                response.rect.right()
                                                                    - f32::from(INPUT_PADDING_X)
                                                                    - CHECK_SIZE * HALF,
                                                                response.rect.center().y,
                                                            ),
                                                            egui::Vec2::splat(CHECK_SIZE),
                                                        ),
                                                        appearance.accent,
                                                    );
                                                }
                                                #[cfg(any(test, feature = "inspection"))]
                                                _output.traces.push(Trace::capture(
                                                    "picker-option",
                                                    &response,
                                                ));
                                                if response.hovered()
                                                    && ui.input(|input| {
                                                        input.pointer.delta() != egui::Vec2::ZERO
                                                    })
                                                {
                                                    self.cursor = row;
                                                }
                                                if response.clicked() {
                                                    selected = Some(choice.value.clone());
                                                }
                                                if row == self.cursor && (down || up || home || end)
                                                {
                                                    response.scroll_to_me(None);
                                                }
                                            });
                                        }
                                    });
                                });
                            for (row, option) in option_ids.iter().enumerate() {
                                ui.ctx().accesskit_node_builder(*option, |node| {
                                    node.set_role(egui::accesskit::Role::ListBoxOption);
                                    node.clear_toggled();
                                    node.set_selected(row == self.cursor);
                                });
                            }
                            let selected_id =
                                option_ids.get(self.cursor).map(|id| id.accesskit_id());
                            if let Some(list) = list_id {
                                ui.ctx().accesskit_node_builder(list, |node| {
                                    if let Some(selected) = selected_id {
                                        node.set_active_descendant(selected);
                                    } else {
                                        node.clear_active_descendant();
                                    }
                                });
                                if let Some(input) = input_id {
                                    ui.ctx().accesskit_node_builder(input, |node| {
                                        node.set_role(egui::accesskit::Role::EditableComboBox);
                                        node.set_auto_complete(egui::accesskit::AutoComplete::List);
                                        node.set_expanded(true);
                                        node.set_controls(vec![list.accesskit_id()]);
                                        node.set_label(message(
                                            locale,
                                            "settings.fontFamilySearchPlaceholder",
                                            &[],
                                        ));
                                        if let Some(selected) = selected_id {
                                            node.set_active_descendant(selected);
                                        } else {
                                            node.clear_active_descendant();
                                        }
                                    });
                                }
                            }
                            if let Some(list) = list_id {
                                if let Some(response) = ui.ctx().read_response(list) {
                                    bounds.push((list, response.rect));
                                }
                            }
                            let dialog = ui.response();
                            dialog.widget_info(|| {
                                let mut info = egui::WidgetInfo::new(egui::WidgetType::Other);
                                info.enabled = ui.is_enabled();
                                info
                            });
                            ui.ctx().accesskit_node_builder(dialog.id, |node| {
                                node.set_role(egui::accesskit::Role::Dialog);
                            });
                            if !searchable
                                && ui.is_visible()
                                && !ui.is_sizing_pass()
                                && ui.is_enabled()
                            {
                                if self.focus_content {
                                    ui.memory_mut(|memory| {
                                        memory.request_focus_with_filter(dialog.id, POPUP_EVENTS)
                                    });
                                    self.focus_content = false;
                                    self.content_focused = true;
                                    ui.ctx().request_repaint();
                                }
                                ui.memory_mut(|memory| {
                                    memory.set_focus_lock_filter(dialog.id, POPUP_EVENTS)
                                });
                            }
                        },
                    );
                });
            let end = ui
                .ctx()
                .graphics_mut(|graphics| graphics.entry(layer).next_idx());
            (layer, start, end, frame.response.rect)
        });
        if let Some(popup) = &shown
            && popup.response.should_close()
        {
            let outside_click =
                popup.response.clicked_elsewhere() && ui.input(|input| input.pointer.any_click());
            if !guarded_escape || outside_click {
                self.open = false;
                if outside_click {
                    self.restore_focus = false;
                }
            }
        }
        if selected.is_some() {
            self.open = false;
        }
        if let Some(motion) = &mut self.motion {
            motion.target(self.open, now);
        }
        if let Some(shown) = &shown
            && let Some(motion) = self.motion
        {
            let sample = motion.sample_popup(now);
            #[cfg(any(test, feature = "inspection"))]
            _output.popup_traces.push(super::PopupTrace {
                field: label_key,
                open: self.open,
                opacity: sample.opacity,
                scale: sample.scale,
                active: sample.active,
            });
            if sample.active {
                ui.ctx().request_repaint();
            }
            let (layer, start, end, rect) = shown.inner;
            let (pivot, _) = placement.pivot_pos(&trigger.rect, POPUP_OFFSET);
            let transform = sample.transform(pivot.pos_in_rect(&rect));
            ui.ctx().set_transform_layer(layer, transform);
            if let Some(dialog) = popup_id {
                bounds.push((dialog, rect));
                #[cfg(any(test, feature = "inspection"))]
                if let Some(mut response) = ui.ctx().read_response(dialog) {
                    response.rect = transform * rect;
                    _output
                        .traces
                        .push(Trace::capture("picker-dialog", &response));
                }
            }
            for (node_id, rect) in bounds {
                let rect = transform * rect;
                ui.ctx().accesskit_node_builder(node_id, |node| {
                    node.set_bounds(egui::accesskit::Rect {
                        x0: rect.left().into(),
                        y0: rect.top().into(),
                        x1: rect.right().into(),
                        y1: rect.bottom().into(),
                    })
                });
            }
            let opacity = sample.opacity;
            ui.ctx().graphics_mut(|graphics| {
                let list = graphics.entry(layer);
                for index in start.0..end.0 {
                    list.mutate_shape(egui::layers::ShapeIdx(index), |shape| {
                        if opacity < 1.0 {
                            egui::epaint::shape_transform::adjust_colors(
                                &mut shape.shape,
                                move |color| {
                                    if *color != Color32::PLACEHOLDER {
                                        *color = color.gamma_multiply(opacity);
                                    }
                                },
                            );
                        }
                    });
                }
            });
        }
        ui.ctx().accesskit_node_builder(id, |node| {
            node.clear_toggled();
            node.set_expanded(self.open);
            node.set_has_popup(egui::accesskit::HasPopup::Dialog);
            if let Some(popup) = popup_id {
                node.set_controls(vec![popup.accesskit_id()]);
            } else {
                node.clear_controls();
            }
        });
        if !searchable {
            ui.memory_mut(|memory| memory.set_focus_lock_filter(id, PICKER_EVENTS));
        }
        selected
    }
}

struct FontPicker {
    picker: Picker,
    monospace_only: bool,
}

impl Default for FontPicker {
    fn default() -> Self {
        Self {
            picker: Picker::default(),
            monospace_only: true,
        }
    }
}

impl FontPicker {
    fn show(
        &mut self,
        ui: &mut Ui,
        field: Font,
        settings: &Settings,
        resources: &Resources,
        locale: &ResolvedLocale,
        appearance: &super::Appearance,
        font_previews: &mut crate::font_preview::Previews,
        output: &mut Output,
    ) {
        let Some(fonts) = resources.fonts() else {
            hint(ui, "settings.loading", locale, appearance);
            return;
        };
        ui.push_id(field.label(), |ui| {
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.y = PICKER_GAP;
                ui.horizontal(|ui| {
                    let filter_label = message(locale, "settings.fontFamilyMonospaceOnly", &[]);
                    let filter_width =
                        ui.fonts_mut(|fonts| {
                            fonts.layout_no_wrap(
                                filter_label,
                                FontId::proportional(TEXT_SIZE),
                                appearance.muted,
                            )
                        })
                        .size()
                        .x + SWITCH_WIDTH
                            + CONTROL_GAP;
                    let label_width = (ui.available_width() - filter_width - CONTROL_GAP).max(0.0);
                    left_label(ui, label_width, field.label(), locale, appearance);
                    ui.allocate_ui_with_layout(
                        egui::vec2(filter_width, SWITCH_HEIGHT),
                        egui::Layout::top_down(Align::Min),
                        |ui| {
                            ui.set_width(filter_width);
                            if switch_control(
                                ui,
                                "settings.fontFamilyMonospaceOnly",
                                None,
                                self.monospace_only,
                                locale,
                                appearance,
                                output,
                            ) {
                                self.monospace_only = !self.monospace_only;
                            }
                            #[cfg(any(test, feature = "inspection"))]
                            if let Some(trace) = output.traces.last_mut() {
                                trace.field = match field {
                                    Font::Editor => "editor-monospace-filter",
                                    Font::Terminal => "terminal-monospace-filter",
                                };
                            }
                        },
                    );
                });
                let mut choices = vec![Choice {
                    identity: SYSTEM_FONT_VALUE.into(),
                    value: None,
                    label: message(locale, "settings.fontFamilySystemDefault", &[]),
                    search: SYSTEM_FONT_VALUE.into(),
                    font: None,
                }];
                choices.extend(
                    fonts
                        .iter()
                        .filter(|font| !self.monospace_only || font.monospaced)
                        .map(|font| Choice {
                            identity: font.name.clone(),
                            value: Some(font.name.clone()),
                            label: font.name.clone(),
                            search: font.name.clone(),
                            font: Some(font.name.clone()),
                        }),
                );
                let active = field.value(settings).map(str::to_owned);
                let display = active
                    .clone()
                    .unwrap_or_else(|| message(locale, "settings.fontFamilySystemDefault", &[]));
                if let Some(value) = self.picker.show(
                    ui,
                    field.label(),
                    &display,
                    &active,
                    &choices,
                    active.as_deref(),
                    Some(font_previews),
                    true,
                    egui::emath::RectAlign::BOTTOM_START,
                    locale,
                    appearance,
                    output,
                ) {
                    output
                        .changes
                        .push(Change::Code(CodeChange::Font(field, value)));
                }
            });
        });
    }
}

fn font_text(
    ui: &Ui,
    value: &str,
    family: Option<&str>,
    rect: Rect,
    foreground: Color32,
    previews: Option<&mut crate::font_preview::Previews>,
    output: &mut Output,
) {
    if let Some(family) = family
        && let Some(previews) = previews
    {
        if let Err(error) = previews.paint(
            ui,
            &crate::font_preview::Text {
                family,
                value,
                rect,
                size: TEXT_SIZE,
                foreground,
            },
        ) {
            output.error = Some(error);
        }
        return;
    }
    text(ui, value, rect.min, rect.width(), foreground);
}

fn option(
    ui: &mut Ui,
    current: Selection,
    picker: &mut Picker,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    output: &mut Output,
) {
    ui.push_id(current.label(), |ui| {
        let width = ui.available_width();
        ui.horizontal(|ui| {
            left_label(
                ui,
                (width - OPTION_WIDTH - CONTROL_GAP).max(0.0),
                current.label(),
                locale,
                appearance,
            );
            ui.allocate_ui_with_layout(
                egui::vec2(OPTION_WIDTH, COMBO_TRIGGER_HEIGHT),
                egui::Layout::top_down(Align::Min),
                |ui| {
                    ui.set_width(OPTION_WIDTH);
                    let choices = current
                        .choices()
                        .iter()
                        .map(|value| Choice {
                            identity: value.option_label().into(),
                            value: *value,
                            label: message(locale, value.option_label(), &[]),
                            search: String::new(),
                            font: None,
                        })
                        .collect::<Vec<_>>();
                    if let Some(value) = picker.show(
                        ui,
                        current.label(),
                        &message(locale, current.option_label(), &[]),
                        &current,
                        &choices,
                        None,
                        None,
                        false,
                        egui::emath::RectAlign::BOTTOM_END,
                        locale,
                        appearance,
                        output,
                    ) {
                        output
                            .changes
                            .push(Change::Code(CodeChange::Selection(value)));
                    }
                },
            );
        });
    });
}

use egui::{
    Align, Color32, FontId, Id, Key, Layout, Modifiers, Rect, RichText, Sense, Stroke, StrokeKind,
    Ui, Vec2, vec2,
};
use taide_model::ids::ProjectId;
use taide_model::project::{ProjectDisplay, ProjectDisplayPatch, ProjectRef};

use crate::icons::glyphs::PROJECT_ICONS;
use crate::modal::Composition;
use crate::project_rail::{
    PROJECT_COLOR_TOKENS, PROJECT_LABEL_MAX_CODEPOINTS, ProjectDisplayResolution, ProjectGlyph,
    RailIcon, clamp_project_label, normalize_project_label, paint_project_glyph,
    resolve_project_color_lane, resolve_project_display, resolve_project_icon_name,
};
use crate::shell::ShellColors;

pub const DIALOG_ID: &str = "native-project-display";
const SEARCH_ID: &str = "native-project-display-icon-search";
const LABEL_ID: &str = "native-project-display-label";
const ICON_LIST_ID: &str = "native-project-display-icons";
const DIALOG_CONTENT_WIDTH: f32 = 464.0;
const DIALOG_VIEWPORT_INSET: f32 = 80.0;
pub(crate) const SECTION_GAP: f32 = 16.0;
pub(crate) const ROW_GAP: f32 = 12.0;
const MODE_GAP: f32 = 16.0;
const COLOR_LABEL_GAP: f32 = 8.0;
pub(crate) const SMALL_TEXT_SIZE: f32 = 12.0;
const ICON_COLUMNS: usize = 8;
const ICON_GAP: f32 = 4.0;
const ICON_LIST_PADDING: f32 = 4.0;
const ICON_LIST_MAX_HEIGHT: f32 = 176.0;
const ICON_GLYPH_SIZE: f32 = 16.0;
const ICON_EMPTY_PADDING: f32 = 16.0;
const LABEL_INPUT_WIDTH: f32 = 96.0;
const SWATCH_SIZE: f32 = 24.0;
const SWATCH_GAP: f32 = 6.0;
const SWATCH_RING_WIDTH: f32 = 2.0;
const SWATCH_RING_OFFSET: f32 = 1.0;
const PREVIEW_SIZE: f32 = 40.0;
const BOX_RADIUS: f32 = 6.0;
const ITEM_RADIUS: f32 = 4.0;
const BORDER_WIDTH: f32 = 1.0;
const NAVIGATION_KEYS: [Key; 5] = [
    Key::ArrowDown,
    Key::ArrowUp,
    Key::Home,
    Key::End,
    Key::Enter,
];

/// `ProjectDisplayMode`; the three presentations compete for the same 40px rail button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DisplayMode {
    Icon,
    Label,
    #[default]
    Default,
}

/// Original `DISPLAY_MODE_ORDER` with `DISPLAY_MODE_LABEL_KEY`.
const DISPLAY_MODES: [(DisplayMode, &str); 3] = [
    (DisplayMode::Icon, "project.displayModeIcon"),
    (DisplayMode::Label, "project.displayModeLabel"),
    (DisplayMode::Default, "project.displayModeDefault"),
];

/// `project-display-dialog.tsx`: every field is reset from the stored display on open, and Save
/// sends all three axes with `""` for the ones the chosen mode does not use.
#[derive(Default)]
pub struct ProjectDisplayDialog {
    project: Option<ProjectId>,
    name: String,
    mode: DisplayMode,
    label: String,
    icon: &'static str,
    lane: Option<usize>,
    search: String,
    cursor: usize,
    is_pending: bool,
    focus_label: bool,
    scroll_to_cursor: bool,
    composition: Composition,
}

impl ProjectDisplayDialog {
    pub fn open(&mut self, project: &ProjectRef) {
        let display = &project.display;
        self.project = Some(project.id.clone());
        self.name = project.name.clone();
        self.mode = match resolve_project_display(display).glyph {
            ProjectGlyph::Label(_) => DisplayMode::Label,
            ProjectGlyph::Icon(_) => DisplayMode::Icon,
            ProjectGlyph::Default => DisplayMode::Default,
        };
        self.label = clamp_project_label(display.label.as_deref().unwrap_or_default());
        self.icon = resolve_project_icon_name(display.icon.as_deref());
        self.lane = resolve_project_color_lane(display.color.as_deref());
        self.is_pending = false;
        self.select_mode(self.mode);
    }

    pub fn is_open(&self) -> bool {
        self.project.is_some()
    }

    pub fn project(&self) -> Option<&ProjectId> {
        self.project.as_ref()
    }

    pub fn close(&mut self) {
        self.project = None;
        self.is_pending = false;
    }

    /// Settles a submitted save: success closes the dialog, failure keeps every field.
    pub fn finish(&mut self, is_success: bool) {
        if !self.is_pending {
            return;
        }
        self.is_pending = false;
        if is_success {
            self.project = None;
        }
    }

    /// Returns the project and the full patch to write when the user saves.
    pub fn show(
        &mut self,
        context: &egui::Context,
        colors: &ShellColors,
        text: &dyn Fn(&str) -> String,
        icon: &mut dyn FnMut(&Ui, Rect, RailIcon<'_>, Color32),
    ) -> Option<(ProjectId, ProjectDisplayPatch)> {
        let project = self.project.clone()?;
        let mut is_saved = false;
        let mut is_cancelled = false;
        let width = dialog_width(context);
        let response = egui::Modal::new(Id::new(DIALOG_ID)).show(context, |ui| {
            ui.set_width(width);
            ui.heading(text("project.displayDialogTitle"));
            ui.add_space(SECTION_GAP);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = MODE_GAP;
                for (mode, key) in DISPLAY_MODES {
                    let label = RichText::new(text(key))
                        .size(SMALL_TEXT_SIZE)
                        .color(colors.foreground);
                    if ui.radio(self.mode == mode, label).clicked() && self.mode != mode {
                        self.select_mode(mode);
                    }
                }
            });
            match self.mode {
                DisplayMode::Icon => {
                    ui.add_space(SECTION_GAP);
                    self.icon_picker(ui, colors, text, icon);
                }
                DisplayMode::Label => {
                    ui.add_space(SECTION_GAP);
                    self.label_field(ui, colors, text);
                }
                DisplayMode::Default => {}
            }
            ui.add_space(SECTION_GAP);
            color_picker(ui, colors, &text("project.displayColor"), &mut self.lane);
            ui.add_space(SECTION_GAP);
            self.preview_row(ui, colors, text, icon);
            ui.add_space(SECTION_GAP);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let can_save = self.can_save() && !self.is_pending;
                if ui
                    .add_enabled(can_save, egui::Button::new(text("common.save")))
                    .clicked()
                {
                    is_saved = true;
                }
                if ui.button(text("common.cancel")).clicked() {
                    is_cancelled = true;
                }
            });
        });
        if is_cancelled || response.should_close() {
            self.close();
            return None;
        }
        if is_saved && self.can_save() && !self.is_pending {
            self.is_pending = true;
            return Some((project, self.patch()));
        }
        None
    }

    fn select_mode(&mut self, mode: DisplayMode) {
        self.mode = mode;
        self.search.clear();
        self.cursor = 0;
        self.scroll_to_cursor = false;
        self.composition.reset();
        self.focus_label = mode == DisplayMode::Label;
    }

    fn can_save(&self) -> bool {
        self.mode != DisplayMode::Label || !normalize_project_label(&self.label).is_empty()
    }

    fn color_token(&self) -> Option<&'static str> {
        self.lane.map(|lane| PROJECT_COLOR_TOKENS[lane])
    }

    fn preview(&self) -> ProjectDisplayResolution {
        resolve_project_display(&ProjectDisplay {
            icon: (self.mode == DisplayMode::Icon).then(|| self.icon.to_owned()),
            label: (self.mode == DisplayMode::Label).then(|| normalize_project_label(&self.label)),
            color: self.color_token().map(str::to_owned),
        })
    }

    fn patch(&self) -> ProjectDisplayPatch {
        ProjectDisplayPatch {
            icon: Some(if self.mode == DisplayMode::Icon {
                self.icon.to_owned()
            } else {
                String::new()
            }),
            label: Some(if self.mode == DisplayMode::Label {
                normalize_project_label(&self.label)
            } else {
                String::new()
            }),
            color: Some(self.color_token().unwrap_or_default().to_owned()),
        }
    }

    /// The cmdk icon picker: fuzzy-filtered, score-sorted, first match selected after each search,
    /// arrows/Home/End move the selection and Enter or a click chooses it.
    fn icon_picker(
        &mut self,
        ui: &mut Ui,
        colors: &ShellColors,
        text: &dyn Fn(&str) -> String,
        icon: &mut dyn FnMut(&Ui, Rect, RailIcon<'_>, Color32),
    ) {
        egui::Frame::NONE
            .stroke(Stroke::new(BORDER_WIDTH, colors.border))
            .corner_radius(BOX_RADIUS)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let search_id = Id::new(SEARCH_ID);
                let events = ui.input(|input| input.events.clone());
                let is_composing = self.composition.observe(&events);
                let has_focus = ui.memory(|memory| memory.has_focus(search_id));
                let [down, up, home, end, enter] = NAVIGATION_KEYS.map(|key| {
                    has_focus
                        && !is_composing
                        && ui.input_mut(|input| input.consume_key(Modifiers::NONE, key))
                });
                let search = ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .id(search_id)
                        .hint_text(text("project.displayIconSearch"))
                        .font(FontId::proportional(SMALL_TEXT_SIZE))
                        .return_key(None)
                        .desired_width(f32::INFINITY),
                );
                if search.changed() {
                    self.cursor = 0;
                }
                let matches = icon_matches(&self.search);
                let last = matches.len().saturating_sub(1);
                let previous = self.cursor;
                if down {
                    self.cursor = (self.cursor + 1).min(last);
                }
                if up {
                    self.cursor = self.cursor.saturating_sub(1);
                }
                if home {
                    self.cursor = 0;
                }
                if end {
                    self.cursor = last;
                }
                self.cursor = self.cursor.min(last);
                self.scroll_to_cursor |= self.cursor != previous;
                if enter && let Some(name) = matches.get(self.cursor) {
                    self.icon = name;
                }
                ui.separator();
                if matches.is_empty() {
                    ui.add_space(ICON_EMPTY_PADDING);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(text("palette.noResults"))
                                .size(SMALL_TEXT_SIZE)
                                .color(colors.muted),
                        );
                    });
                    ui.add_space(ICON_EMPTY_PADDING);
                    return;
                }
                egui::ScrollArea::vertical()
                    .id_salt(ICON_LIST_ID)
                    .max_height(ICON_LIST_MAX_HEIGHT)
                    .show(ui, |ui| self.icon_grid(ui, &matches, colors, icon));
            });
    }

    fn icon_grid(
        &mut self,
        ui: &mut Ui,
        matches: &[&'static str],
        colors: &ShellColors,
        icon: &mut dyn FnMut(&Ui, Rect, RailIcon<'_>, Color32),
    ) {
        let columns = ICON_COLUMNS as f32;
        let width = ui.available_width();
        let cell = (width - 2.0 * ICON_LIST_PADDING - ICON_GAP * (columns - 1.0)) / columns;
        let rows = matches.len().div_ceil(ICON_COLUMNS) as f32;
        let height = rows * cell + (rows - 1.0).max(0.0) * ICON_GAP + 2.0 * ICON_LIST_PADDING;
        let (area, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let is_pointer_moving = ui.input(|input| input.pointer.is_moving());
        for (index, name) in matches.iter().copied().enumerate() {
            let offset = vec2(
                (index % ICON_COLUMNS) as f32 * (cell + ICON_GAP),
                (index / ICON_COLUMNS) as f32 * (cell + ICON_GAP),
            );
            let rect = Rect::from_min_size(
                area.min + Vec2::splat(ICON_LIST_PADDING) + offset,
                Vec2::splat(cell),
            );
            let response = ui.interact(rect, Id::new(ICON_LIST_ID).with(name), Sense::click());
            let is_chosen = self.icon == name;
            response.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, is_chosen, name)
            });
            if response.hovered() && is_pointer_moving {
                self.cursor = index;
            }
            if response.clicked() {
                self.cursor = index;
                self.icon = name;
            }
            let is_selected = self.cursor == index;
            let fill = if is_selected {
                Some(ui.visuals().selection.bg_fill)
            } else {
                (self.icon == name).then_some(colors.rail.item_active)
            };
            if let Some(fill) = fill {
                ui.painter().rect_filled(rect, ITEM_RADIUS, fill);
                ui.painter().rect_stroke(
                    rect,
                    ITEM_RADIUS,
                    Stroke::new(BORDER_WIDTH, colors.rail.accent),
                    StrokeKind::Inside,
                );
            }
            icon(
                ui,
                Rect::from_center_size(rect.center(), Vec2::splat(ICON_GLYPH_SIZE)),
                RailIcon::Project(Some(name)),
                colors.foreground,
            );
            if is_selected && std::mem::take(&mut self.scroll_to_cursor) {
                ui.scroll_to_rect(rect, None);
            }
        }
    }

    fn label_field(&mut self, ui: &mut Ui, colors: &ShellColors, text: &dyn Fn(&str) -> String) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = ROW_GAP;
            ui.label(
                RichText::new(text("project.displayLabelField"))
                    .size(SMALL_TEXT_SIZE)
                    .color(colors.foreground),
            );
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.label)
                    .id(Id::new(LABEL_ID))
                    .char_limit(PROJECT_LABEL_MAX_CODEPOINTS)
                    .horizontal_align(Align::Center)
                    .font(FontId::proportional(SMALL_TEXT_SIZE))
                    .desired_width(LABEL_INPUT_WIDTH),
            );
            if response.changed() {
                self.label = clamp_project_label(&self.label);
            }
            if std::mem::take(&mut self.focus_label) {
                response.request_focus();
            }
            ui.label(
                RichText::new(text("project.displayLabelHint"))
                    .size(SMALL_TEXT_SIZE)
                    .color(colors.muted),
            );
        });
    }

    fn preview_row(
        &self,
        ui: &mut Ui,
        colors: &ShellColors,
        text: &dyn Fn(&str) -> String,
        icon: &mut dyn FnMut(&Ui, Rect, RailIcon<'_>, Color32),
    ) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = ROW_GAP;
            ui.label(
                RichText::new(text("project.displayPreview"))
                    .size(SMALL_TEXT_SIZE)
                    .color(colors.muted),
            );
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(PREVIEW_SIZE), Sense::hover());
            let painter = ui.painter().clone();
            painter.rect_filled(rect, BOX_RADIUS, colors.sidebar);
            let preview = self.preview();
            let color = preview
                .lane
                .map_or(colors.muted, |lane| colors.rail.lanes[lane]);
            paint_project_glyph(ui, &painter, rect, &preview, color, icon);
            ui.add(
                egui::Label::new(
                    RichText::new(&self.name)
                        .size(SMALL_TEXT_SIZE)
                        .color(colors.foreground),
                )
                .truncate(),
            );
        });
    }
}

/// Original `DialogContent` width: `sm:max-w-lg` less `p-6`, narrowing like `calc(100%-2rem)`.
pub(crate) fn dialog_width(context: &egui::Context) -> f32 {
    DIALOG_CONTENT_WIDTH.min(context.content_rect().width() - DIALOG_VIEWPORT_INSET)
}

/// The twelve `PROJECT_COLOR_TOKENS` swatches the display and group dialogs share: a press toggles
/// the lane and the pressed one wears the accent ring.
pub(crate) fn color_picker(
    ui: &mut Ui,
    colors: &ShellColors,
    title: &str,
    selected: &mut Option<usize>,
) {
    ui.label(
        RichText::new(title)
            .size(SMALL_TEXT_SIZE)
            .color(colors.foreground),
    );
    ui.add_space(COLOR_LABEL_GAP);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(SWATCH_GAP);
        for (lane, fill) in colors.rail.lanes.iter().copied().enumerate() {
            let (rect, response) = ui.allocate_exact_size(Vec2::splat(SWATCH_SIZE), Sense::click());
            let is_pressed = *selected == Some(lane);
            let label = format!("{title} {}", lane + 1);
            response.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::Button, true, is_pressed, &label)
            });
            if response.clicked() {
                *selected = (!is_pressed).then_some(lane);
            }
            let radius = SWATCH_SIZE / 2.0;
            let painter = ui.painter();
            painter.circle_filled(rect.center(), radius, fill);
            painter.circle_stroke(
                rect.center(),
                radius - BORDER_WIDTH / 2.0,
                Stroke::new(BORDER_WIDTH, colors.border),
            );
            if *selected == Some(lane) {
                painter.circle_stroke(
                    rect.center(),
                    radius + SWATCH_RING_OFFSET + SWATCH_RING_WIDTH / 2.0,
                    Stroke::new(SWATCH_RING_WIDTH, colors.rail.accent),
                );
            }
        }
    });
}

/// cmdk's default filter over the icon ids: positive `commandScore` only, best first, catalog order
/// kept for ties and for an empty search.
fn icon_matches(search: &str) -> Vec<&'static str> {
    if search.is_empty() {
        return PROJECT_ICONS.iter().map(|(id, _)| *id).collect();
    }
    let mut scored = PROJECT_ICONS
        .iter()
        .map(|(id, _)| (*id, crate::command_score::score(id, search)))
        .filter(|(_, score)| *score > 0.0)
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| right.1.total_cmp(&left.1));
    scored.into_iter().map(|(id, _)| id).collect()
}

#[cfg(test)]
#[path = "project-display-dialog-tests.rs"]
mod tests;

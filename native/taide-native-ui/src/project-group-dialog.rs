use egui::{Align, FontId, Id, Key, Layout, Modifiers, RichText};
use taide_model::ids::{ProjectGroupId, ProjectId};
use taide_model::project::ProjectGroup;

use crate::modal::Composition;
use crate::project_display_dialog::{
    ROW_GAP, SECTION_GAP, SMALL_TEXT_SIZE, color_picker, dialog_width,
};
use crate::project_rail::{
    PROJECT_COLOR_TOKENS, PROJECT_GROUP_NAME_MAX_CODEPOINTS, clamp_project_group_name,
    normalize_project_group_name, resolve_project_color_lane,
};
use crate::shell::ShellColors;

pub const DIALOG_ID: &str = "native-project-group";
pub const DELETE_DIALOG_ID: &str = "native-project-group-delete";
const NAME_ID: &str = "native-project-group-name";

/// Original `ProjectGroupDialogMode`: the same form creates a group for the project whose menu
/// asked for it, or edits an existing group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Create(ProjectId),
    Edit(ProjectGroupId),
}

impl Target {
    /// Original `TITLE_KEY` and `CONFIRM_KEY`.
    fn keys(&self) -> (&'static str, &'static str) {
        match self {
            Self::Create(_) => ("projectGroup.createTitle", "projectGroup.create"),
            Self::Edit(_) => ("projectGroup.editTitle", "common.save"),
        }
    }
}

/// Original `onSubmit({ name, color })`: the normalized name and the chosen palette token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Submission {
    pub target: Target,
    pub name: String,
    pub color: Option<&'static str>,
}

/// The writes original `handleSubmitEdit` makes: a rename only when the name changed and a recolor
/// only when the resolved token changed, in that order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GroupEdit {
    pub name: Option<String>,
    pub color: Option<Option<String>>,
}

impl GroupEdit {
    pub fn new(group: &ProjectGroup, name: &str, color: Option<&str>) -> Self {
        let stored = resolve_project_color_lane(group.color.as_deref())
            .map(|lane| PROJECT_COLOR_TOKENS[lane]);
        Self {
            name: (name != group.name).then(|| name.to_owned()),
            color: (color != stored).then(|| color.map(str::to_owned)),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.color.is_none()
    }
}

/// `project-group-dialog.tsx`: both fields are reset on every open, the name is clamped while
/// typing and normalized on confirm, and the swatches toggle the group's tint.
#[derive(Default)]
pub struct ProjectGroupDialog {
    target: Option<Target>,
    name: String,
    lane: Option<usize>,
    is_pending: bool,
    focus_name: bool,
    composition: Composition,
}

impl ProjectGroupDialog {
    pub fn create(&mut self, project: ProjectId) {
        self.open(Target::Create(project), "", None);
    }

    pub fn edit(&mut self, group: &ProjectGroup) {
        self.open(
            Target::Edit(group.id.clone()),
            &group.name,
            group.color.as_deref(),
        );
    }

    fn open(&mut self, target: Target, name: &str, color: Option<&str>) {
        self.target = Some(target);
        self.name = clamp_project_group_name(name);
        self.lane = resolve_project_color_lane(color);
        self.is_pending = false;
        self.focus_name = true;
        self.composition.reset();
    }

    pub fn is_open(&self) -> bool {
        self.target.is_some()
    }

    pub fn target(&self) -> Option<&Target> {
        self.target.as_ref()
    }

    pub fn close(&mut self) {
        self.target = None;
        self.is_pending = false;
    }

    /// Settles a submitted write: success closes the dialog, failure keeps both fields.
    pub fn finish(&mut self, is_success: bool) {
        if !self.is_pending {
            return;
        }
        self.is_pending = false;
        if is_success {
            self.target = None;
        }
    }

    /// Returns the normalized name and color to write when the user confirms.
    pub fn show(
        &mut self,
        context: &egui::Context,
        colors: &ShellColors,
        text: &dyn Fn(&str) -> String,
    ) -> Option<Submission> {
        let target = self.target.clone()?;
        let (title, confirm) = target.keys();
        let mut is_confirmed = false;
        let mut is_cancelled = false;
        let width = dialog_width(context);
        let response = egui::Modal::new(Id::new(DIALOG_ID)).show(context, |ui| {
            ui.set_width(width);
            ui.heading(text(title));
            ui.add_space(SECTION_GAP);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = ROW_GAP;
                ui.label(
                    RichText::new(text("projectGroup.nameField"))
                        .size(SMALL_TEXT_SIZE)
                        .color(colors.foreground),
                );
                let name_id = Id::new(NAME_ID);
                let events = ui.input(|input| input.events.clone());
                let is_composing = self.composition.observe(&events);
                let has_focus = ui.memory(|memory| memory.has_focus(name_id));
                is_confirmed = has_focus
                    && !is_composing
                    && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter));
                let input = ui.add(
                    egui::TextEdit::singleline(&mut self.name)
                        .id(name_id)
                        .char_limit(PROJECT_GROUP_NAME_MAX_CODEPOINTS)
                        .font(FontId::proportional(SMALL_TEXT_SIZE))
                        .return_key(None)
                        .desired_width(f32::INFINITY),
                );
                if input.changed() {
                    self.name = clamp_project_group_name(&self.name);
                }
                if std::mem::take(&mut self.focus_name) {
                    input.request_focus();
                }
            });
            ui.add_space(SECTION_GAP);
            color_picker(ui, colors, &text("projectGroup.color"), &mut self.lane);
            ui.add_space(SECTION_GAP);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(self.can_confirm(), egui::Button::new(text(confirm)))
                    .clicked()
                {
                    is_confirmed = true;
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
        if !is_confirmed || !self.can_confirm() {
            return None;
        }
        self.is_pending = true;
        Some(Submission {
            target,
            name: normalize_project_group_name(&self.name),
            color: self.lane.map(|lane| PROJECT_COLOR_TOKENS[lane]),
        })
    }

    fn can_confirm(&self) -> bool {
        !self.is_pending && !normalize_project_group_name(&self.name).is_empty()
    }
}

/// The group menu's delete confirmation (`sortable-project-group-header.tsx` 129-144): an alert
/// dialog that starts on Cancel, closes on Escape but not on an outside click, and deletes only the
/// group.
#[derive(Default)]
pub struct ProjectGroupDeleteDialog {
    group: Option<(ProjectGroupId, String)>,
    focus_cancel: bool,
}

impl ProjectGroupDeleteDialog {
    pub fn open(&mut self, group: &ProjectGroup) {
        self.group = Some((group.id.clone(), group.name.clone()));
        self.focus_cancel = true;
    }

    pub fn is_open(&self) -> bool {
        self.group.is_some()
    }

    pub fn group(&self) -> Option<&ProjectGroupId> {
        self.group.as_ref().map(|(id, _)| id)
    }

    pub fn close(&mut self) {
        self.group = None;
    }

    /// Returns the group to delete when the destructive action is chosen; either choice closes.
    pub fn show(
        &mut self,
        context: &egui::Context,
        text: &dyn Fn(&str, &[(&str, &str)]) -> String,
    ) -> Option<ProjectGroupId> {
        let (group, name) = self.group.clone()?;
        let width = dialog_width(context);
        let response = egui::Modal::new(Id::new(DELETE_DIALOG_ID)).show(context, |ui| {
            ui.set_width(width);
            ui.heading(text(
                "projectGroup.deleteConfirmTitle",
                &[("name", name.as_str())],
            ));
            ui.label(
                RichText::new(text("projectGroup.deleteConfirmDescription", &[]))
                    .color(ui.visuals().weak_text_color()),
            );
            ui.add_space(SECTION_GAP);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let confirm = ui.button(
                    RichText::new(text("projectGroup.delete", &[]))
                        .color(ui.visuals().error_fg_color),
                );
                let cancel = ui.button(text("common.cancel", &[]));
                if std::mem::take(&mut self.focus_cancel) {
                    cancel.request_focus();
                }
                (confirm.clicked(), cancel.clicked())
            })
            .inner
        });
        let (is_confirmed, is_cancelled) = response.inner;
        let is_escaped = response.is_top_modal
            && !response.any_popup_open
            && context.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
        if is_confirmed || is_cancelled || is_escaped {
            self.close();
        }
        is_confirmed.then_some(group)
    }
}

#[cfg(test)]
#[path = "project-group-dialog-tests.rs"]
mod tests;

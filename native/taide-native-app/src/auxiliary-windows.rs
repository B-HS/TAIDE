use std::collections::{HashMap, HashSet};

use eframe::egui;
use taide_model::ids::{PaneId, ProjectId, ShellSlotId, TabId};
use taide_model::layout::{AuxWindowLayout, ProjectLayout};
use taide_model::project::Project;
use taide_native_ui::shell::WindowScope;
use taide_runtime::WindowRegistry;

const VIEWPORT_SALT: &str = "taide-auxiliary-window";
const TITLE_SEPARATOR: &str = " — ";
pub(crate) const DEFAULT_SIZE: [f32; 2] = [1_000.0, 700.0];
pub(crate) const MINIMUM_SIZE: [f32; 2] = [640.0, 420.0];

#[derive(Clone, Debug)]
pub(crate) struct Window {
    pub(crate) project: ProjectId,
    pub(crate) slot: u32,
    pub(crate) label: String,
    pub(crate) viewport: egui::ViewportId,
    pub(crate) explorer: ShellSlotId,
    pub(crate) is_explorer_open: bool,
    focused_editor: Option<(PaneId, TabId)>,
}

impl Window {
    pub(crate) fn scope(&self) -> WindowScope {
        WindowScope::Auxiliary {
            project: self.project.clone(),
            slot: self.slot,
        }
    }

    pub(crate) fn layout<'a>(
        &self,
        layouts: &'a HashMap<ProjectId, ProjectLayout>,
    ) -> Option<&'a AuxWindowLayout> {
        layouts
            .get(&self.project)?
            .auxiliary_windows
            .iter()
            .find(|window| window.slot == self.slot)
    }

    pub(crate) fn is_alive(&self, layouts: &HashMap<ProjectId, ProjectLayout>) -> bool {
        self.layout(layouts)
            .is_some_and(|window| !taide_layout::service::is_layout_tree_empty(&window.root))
    }
}

#[derive(Default)]
pub(crate) struct Windows {
    open: Vec<Window>,
    reserved: Vec<String>,
    returns: Vec<(ProjectId, u32)>,
}

impl Windows {
    pub(crate) fn restore(
        order: &[ProjectId],
        projects: &HashMap<ProjectId, Project>,
        layouts: &HashMap<ProjectId, ProjectLayout>,
        registry: &WindowRegistry,
    ) -> Self {
        let mut windows = Self::default();
        for (project, slot) in restoration_plan(order, projects, layouts) {
            windows.open(project, slot, registry);
        }
        windows
    }

    pub(crate) fn open(&mut self, project: ProjectId, slot: u32, registry: &WindowRegistry) {
        let label = self.next_label();
        registry.register(label.clone(), project.clone(), slot);
        self.adopt(project, slot, label);
    }

    pub(crate) fn reserve(&mut self) -> String {
        let label = self.next_label();
        self.reserved.push(label.clone());
        label
    }

    pub(crate) fn release(&mut self, label: &str) {
        self.reserved.retain(|reserved| reserved != label);
    }

    pub(crate) fn settle_reserved(
        &mut self,
        layouts: &HashMap<ProjectId, ProjectLayout>,
        registry: &WindowRegistry,
    ) {
        let moved: Vec<_> = layouts
            .iter()
            .flat_map(|(project, layout)| {
                layout
                    .auxiliary_windows
                    .iter()
                    .map(move |window| (project, window.slot))
            })
            .filter(|(project, slot)| self.find(project, *slot).is_none())
            .filter_map(|(project, slot)| {
                registry
                    .label_for(project, slot)
                    .filter(|label| self.reserved.contains(label))
                    .map(|label| (project.clone(), slot, label))
            })
            .collect();
        for (project, slot, label) in moved {
            self.adopt(project, slot, label);
        }
        for label in std::mem::take(&mut self.reserved) {
            registry.forget(&label);
        }
    }

    pub(crate) fn adopt(&mut self, project: ProjectId, slot: u32, label: String) {
        self.release(&label);
        self.returned(&project, slot);
        self.open.push(Window {
            project,
            slot,
            viewport: egui::ViewportId::from_hash_of((VIEWPORT_SALT, &label)),
            label,
            explorer: ShellSlotId::new(),
            is_explorer_open: true,
            focused_editor: None,
        });
    }

    pub(crate) fn close(&mut self, label: &str, registry: &WindowRegistry) -> Option<Window> {
        let index = self.open.iter().position(|window| window.label == label)?;
        registry.forget(label);
        let window = self.open.remove(index);
        self.returns.push((window.project.clone(), window.slot));
        Some(window)
    }

    pub(crate) fn pending_returns(&self) -> Vec<(ProjectId, u32)> {
        self.returns.clone()
    }

    pub(crate) fn returned(&mut self, project: &ProjectId, slot: u32) {
        self.returns
            .retain(|(pending, pending_slot)| pending != project || *pending_slot != slot);
    }

    pub(crate) fn windows(&self) -> &[Window] {
        &self.open
    }

    pub(crate) fn find(&self, project: &ProjectId, slot: u32) -> Option<&Window> {
        self.open
            .iter()
            .find(|window| &window.project == project && window.slot == slot)
    }

    fn next_label(&self) -> String {
        taide_window::service::next_auxiliary_label(
            self.open
                .iter()
                .map(|window| window.label.as_str())
                .chain(self.reserved.iter().map(String::as_str)),
        )
    }

    pub(crate) fn show_explorer(&mut self, project: &ProjectId, slot: u32) -> Option<ShellSlotId> {
        let window = self
            .open
            .iter_mut()
            .find(|window| &window.project == project && window.slot == slot)?;
        window.is_explorer_open = true;
        Some(window.explorer.clone())
    }

    pub(crate) fn toggle_explorer(&mut self, project: &ProjectId, slot: u32) {
        if let Some(window) = self
            .open
            .iter_mut()
            .find(|window| &window.project == project && window.slot == slot)
        {
            window.is_explorer_open = !window.is_explorer_open;
        }
    }

    pub(crate) fn focused_editor(&self, viewport: egui::ViewportId) -> Option<(PaneId, TabId)> {
        self.open
            .iter()
            .find(|window| window.viewport == viewport)
            .and_then(|window| window.focused_editor.clone())
    }

    pub(crate) fn set_focused_editor(
        &mut self,
        viewport: egui::ViewportId,
        focus: Option<(PaneId, TabId)>,
    ) {
        if let Some(window) = self
            .open
            .iter_mut()
            .find(|window| window.viewport == viewport)
        {
            window.focused_editor = focus;
        }
    }

    pub(crate) fn explorer_mounts(&self) -> HashMap<ShellSlotId, ProjectId> {
        self.open
            .iter()
            .map(|window| (window.explorer.clone(), window.project.clone()))
            .collect()
    }
}

pub(crate) fn restoration_plan(
    order: &[ProjectId],
    projects: &HashMap<ProjectId, Project>,
    layouts: &HashMap<ProjectId, ProjectLayout>,
) -> Vec<(ProjectId, u32)> {
    let restorable: HashSet<ProjectId> = taide_window::service::restorable_project_ids(projects)
        .into_iter()
        .collect();
    let ordered: Vec<ProjectId> = order
        .iter()
        .filter(|project| restorable.contains(*project))
        .cloned()
        .collect();
    taide_window::service::plan_auxiliary_window_restorations(&ordered, layouts)
}

pub(crate) fn title(project_name: &str, application_name: &str) -> String {
    format!("{project_name}{TITLE_SEPARATOR}{application_name}")
}

pub(crate) fn builder(title: String) -> egui::ViewportBuilder {
    egui::ViewportBuilder::default()
        .with_title(title)
        .with_inner_size(DEFAULT_SIZE)
        .with_min_inner_size(MINIMUM_SIZE)
        .with_fullsize_content_view(true)
        .with_title_shown(false)
        .with_titlebar_shown(false)
}

#[cfg(test)]
#[path = "auxiliary-windows-tests.rs"]
mod tests;

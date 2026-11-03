use std::collections::HashMap;

use taide_model::ids::{ProjectId, ShellSlotId};
use taide_model::project::ShellSlotTree;

use crate::explorer::Explorer;

/// Explorer state per mount (a main window slot or an auxiliary window's sidebar), like the
/// original `ExplorerContainer` each window and slot owns. The tree page stays per project.
#[derive(Default)]
pub(crate) struct Mounts {
    entries: HashMap<ShellSlotId, (ProjectId, Explorer)>,
}

impl Mounts {
    pub(crate) fn get(&self, mount: &ShellSlotId) -> Option<&Explorer> {
        self.entries.get(mount).map(|(_, explorer)| explorer)
    }

    pub(crate) fn get_mut(
        &mut self,
        project: &ProjectId,
        mount: &ShellSlotId,
    ) -> Option<&mut Explorer> {
        self.entries
            .get_mut(mount)
            .filter(|(owner, _)| owner == project)
            .map(|(_, explorer)| explorer)
    }

    pub(crate) fn entry(&mut self, project: &ProjectId, mount: &ShellSlotId) -> &mut Explorer {
        let entry = self
            .entries
            .entry(mount.clone())
            .or_insert_with(|| (project.clone(), Explorer::default()));
        if entry.0 != *project {
            *entry = (project.clone(), Explorer::default());
        }
        &mut entry.1
    }

    /// Takes the mount's explorer out for a render inside the sidebar panel's error boundary;
    /// [`Mounts::put`] returns it.
    pub(crate) fn take(&mut self, project: &ProjectId, mount: &ShellSlotId) -> Option<Explorer> {
        self.entries
            .remove(mount)
            .filter(|(owner, _)| owner == project)
            .map(|(_, explorer)| explorer)
    }

    pub(crate) fn put(&mut self, project: &ProjectId, mount: ShellSlotId, explorer: Explorer) {
        self.entries.insert(mount, (project.clone(), explorer));
    }

    pub(crate) fn of_project_mut<'a>(
        &'a mut self,
        project: &'a ProjectId,
    ) -> impl Iterator<Item = &'a mut Explorer> + 'a {
        self.entries
            .values_mut()
            .filter(move |(owner, _)| owner == project)
            .map(|(_, explorer)| explorer)
    }

    #[cfg(test)]
    pub(crate) fn of_project<'a>(
        &'a self,
        project: &'a ProjectId,
    ) -> impl Iterator<Item = &'a Explorer> + 'a {
        self.entries
            .values()
            .filter(move |(owner, _)| owner == project)
            .map(|(_, explorer)| explorer)
    }

    pub(crate) fn all_mut(&mut self) -> impl Iterator<Item = &mut Explorer> {
        self.entries.values_mut().map(|(_, explorer)| explorer)
    }

    pub(crate) fn reconcile(
        &mut self,
        tree: Option<&ShellSlotTree>,
        auxiliary: &HashMap<ShellSlotId, ProjectId>,
    ) {
        self.entries.retain(|mount, (project, _)| {
            tree.and_then(|tree| taide_native_ui::snapshot::slot_project(tree, mount))
                .or_else(|| auxiliary.get(mount))
                == Some(project)
        });
    }

    pub(crate) fn forget_project(&mut self, project: &ProjectId) {
        self.entries.retain(|_, (owner, _)| owner != project);
    }
}

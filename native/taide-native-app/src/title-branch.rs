use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use taide_model::app_event::AppEvent;
use taide_model::ids::ProjectId;

#[derive(Default)]
pub(crate) struct Changes {
    projects: Mutex<HashSet<ProjectId>>,
}

impl Changes {
    pub(crate) fn record(&self, event: &AppEvent) {
        let project = match event {
            AppEvent::GitStatusChanged { project_id }
            | AppEvent::GitRefsChanged { project_id }
            | AppEvent::FsChanged { project_id, .. } => project_id,
            _ => return,
        };
        self.projects
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(project.clone());
    }

    pub(crate) fn take(&self) -> HashSet<ProjectId> {
        std::mem::take(
            &mut *self
                .projects
                .lock()
                .unwrap_or_else(|error| error.into_inner()),
        )
    }
}

struct Entry {
    branch: Option<String>,
    is_fetching: bool,
    is_stale: bool,
}

#[derive(Default)]
pub(crate) struct Branches {
    entries: HashMap<ProjectId, Entry>,
}

impl Branches {
    pub(crate) fn branch(&self, project: &ProjectId) -> Option<&str> {
        self.entries.get(project)?.branch.as_deref()
    }

    pub(crate) fn invalidate(&mut self, projects: impl IntoIterator<Item = ProjectId>) {
        for project in projects {
            if let Some(entry) = self.entries.get_mut(&project) {
                entry.is_stale = true;
            }
        }
    }

    pub(crate) fn retain(&mut self, open: &HashSet<ProjectId>) {
        self.entries.retain(|project, _| open.contains(project));
    }

    pub(crate) fn requests(
        &mut self,
        shown: impl IntoIterator<Item = ProjectId>,
    ) -> Vec<ProjectId> {
        let mut requests = Vec::new();
        for project in shown {
            let entry = self.entries.entry(project.clone()).or_insert(Entry {
                branch: None,
                is_fetching: false,
                is_stale: true,
            });
            if entry.is_stale && !entry.is_fetching {
                entry.is_stale = false;
                entry.is_fetching = true;
                requests.push(project);
            }
        }
        requests
    }

    pub(crate) fn retry(&mut self, project: &ProjectId) {
        if let Some(entry) = self.entries.get_mut(project) {
            entry.is_fetching = false;
            entry.is_stale = true;
        }
    }

    pub(crate) fn cancel_fetches(&mut self) {
        for entry in self.entries.values_mut().filter(|entry| entry.is_fetching) {
            entry.is_fetching = false;
            entry.is_stale = true;
        }
    }

    pub(crate) fn finish(&mut self, project: &ProjectId, branch: Option<String>) {
        if let Some(entry) = self.entries.get_mut(project) {
            entry.branch = branch;
            entry.is_fetching = false;
        }
    }
}

#[cfg(test)]
#[path = "title-branch-tests.rs"]
mod tests;

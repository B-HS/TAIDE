use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::sync::Mutex;

use taide_model::app_event::AppEvent;
use taide_model::file::FsChangeKind;
use taide_model::ids::ProjectId;

const MAX_PENDING_PROJECTS: usize = 64;
const MAX_PENDING_DIRECTORIES: usize = 128;
pub const MAX_PENDING_PREVIEW_PATHS: usize = 512;

pub type TreeInvalidation = Option<BTreeSet<String>>;

#[derive(Default)]
pub struct TreeChanges {
    pending: Mutex<Pending>,
    previews: Mutex<PreviewInvalidation>,
}

#[derive(Default)]
pub struct PreviewInvalidation {
    pub paths: HashSet<String>,
    pub projects: HashSet<ProjectId>,
    pub all: bool,
}

#[derive(Default)]
struct Pending {
    projects: HashMap<ProjectId, TreeInvalidation>,
    all: bool,
}

impl TreeChanges {
    pub fn record(&self, event: &AppEvent) {
        self.record_preview(event);
        match event {
            AppEvent::FsRescanRequired { project_id } => self.merge(project_id.clone(), None),
            AppEvent::FsChanged { project_id, change } => {
                if change.from_app && change.kind == FsChangeKind::Modified {
                    return;
                }
                let mut dirs = BTreeSet::new();
                for path in &change.paths {
                    if let Some(parent) = Path::new(path).parent().and_then(Path::to_str) {
                        dirs.insert(parent.to_owned());
                        if dirs.len() > MAX_PENDING_DIRECTORIES {
                            break;
                        }
                    }
                }
                if dirs.len() > MAX_PENDING_DIRECTORIES {
                    self.merge(project_id.clone(), None);
                } else {
                    self.merge(project_id.clone(), Some(dirs));
                }
            }
            _ => {}
        }
    }

    fn record_preview(&self, event: &AppEvent) {
        if !matches!(
            event,
            AppEvent::FsChanged { .. } | AppEvent::FsRescanRequired { .. }
        ) {
            return;
        }
        let mut pending = self
            .previews
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if pending.all {
            return;
        }
        match event {
            AppEvent::FsChanged { change, .. } => {
                for path in &change.paths {
                    pending.paths.insert(path.clone());
                    if pending.paths.len() > MAX_PENDING_PREVIEW_PATHS {
                        pending.paths.clear();
                        pending.projects.clear();
                        pending.all = true;
                        return;
                    }
                }
            }
            AppEvent::FsRescanRequired { project_id } => {
                pending.projects.insert(project_id.clone());
                if pending.projects.len() > MAX_PENDING_PROJECTS {
                    pending.paths.clear();
                    pending.projects.clear();
                    pending.all = true;
                }
            }
            _ => {}
        }
    }

    pub fn take_previews(&self) -> PreviewInvalidation {
        let mut pending = self
            .previews
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        std::mem::take(&mut *pending)
    }

    pub fn merge(&self, project: ProjectId, dirs: TreeInvalidation) {
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if pending.all {
            return;
        }
        if !pending.projects.contains_key(&project)
            && pending.projects.len() == MAX_PENDING_PROJECTS
        {
            pending.projects.clear();
            pending.all = true;
            return;
        }
        let entry = pending
            .projects
            .entry(project)
            .or_insert_with(|| Some(BTreeSet::new()));
        match (entry.as_mut(), dirs) {
            (Some(current), Some(dirs)) => {
                for dir in dirs {
                    current.insert(dir);
                    if current.len() > MAX_PENDING_DIRECTORIES {
                        *entry = None;
                        break;
                    }
                }
            }
            (_, None) => *entry = None,
            (None, Some(_)) => {}
        }
    }

    pub fn take(
        &self,
        projects: impl IntoIterator<Item = ProjectId>,
    ) -> HashMap<ProjectId, TreeInvalidation> {
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let batch = std::mem::take(&mut *pending);
        if batch.all {
            return projects
                .into_iter()
                .map(|project| (project, None))
                .collect();
        }
        batch.projects
    }
}

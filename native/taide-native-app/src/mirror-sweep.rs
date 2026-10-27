use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use taide_infra::root_guard;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::{ProjectId, ShellSlotId, TabId};
use taide_model::layout::{ProjectLayout, Tab, TabKind};
use taide_model::project::ShellSlotTree;
use taide_runtime::AppServices;

#[derive(Default)]
pub(crate) struct MirrorSweeps {
    swept: HashMap<ShellSlotId, ProjectId>,
}

impl MirrorSweeps {
    pub(crate) fn due(
        &mut self,
        tree: Option<&ShellSlotTree>,
        windows: &HashMap<ShellSlotId, ProjectId>,
        layouts: &HashMap<ProjectId, ProjectLayout>,
    ) -> Vec<ProjectId> {
        let mut hosts = windows.clone();
        if let Some(tree) = tree {
            collect(tree, &mut hosts);
        }
        self.swept.retain(|host, _| hosts.contains_key(host));
        let mut due = Vec::new();
        for (host, project) in hosts {
            if !layouts.contains_key(&project) || self.swept.get(&host) == Some(&project) {
                continue;
            }
            self.swept.insert(host, project.clone());
            if !due.contains(&project) {
                due.push(project);
            }
        }
        due
    }

    pub(crate) fn retry(&mut self, project: &ProjectId) {
        self.swept.retain(|_, swept| swept != project);
    }
}

fn collect(tree: &ShellSlotTree, hosts: &mut HashMap<ShellSlotId, ProjectId>) {
    match tree {
        ShellSlotTree::Leaf {
            slot_id,
            project_id,
        } => {
            hosts.insert(slot_id.clone(), project_id.clone());
        }
        ShellSlotTree::Split { children, .. } => {
            for child in children {
                collect(child, hosts);
            }
        }
    }
}

fn keep(layout: &ProjectLayout) -> (Vec<String>, Vec<TabId>) {
    let open: Vec<&Tab> = taide_layout::service::all_roots(layout)
        .flat_map(crate::tabs::tabs_in)
        .collect();
    let paths = open
        .iter()
        .filter_map(|tab| match &tab.kind {
            TabKind::File { path } => Some(path),
            _ => None,
        })
        .flat_map(|path| {
            let canonical = root_guard::canonicalize_lenient(Path::new(path))
                .ok()
                .and_then(|canonical| canonical.to_str().map(str::to_owned));
            std::iter::once(path.clone()).chain(canonical)
        })
        .collect();
    let tab_ids = open
        .into_iter()
        .chain(layout.closed_tabs.iter().map(|closed| &closed.tab))
        .filter(|tab| matches!(tab.kind, TabKind::Untitled { .. }))
        .map(|tab| tab.id.clone())
        .collect();
    (paths, tab_ids)
}

pub async fn prune(services: &Arc<AppServices>, project: ProjectId) -> AppResult<()> {
    let stopping = || AppError::Forbidden("native mirror prune is stopping".into());
    let operation = services
        .tasks
        .begin_operation("native-mirror-prune")
        .ok_or_else(stopping)?;
    let guard = services.state.begin_owned_mutation().await;
    let services = services.clone();
    let tasks = services.tasks.clone();
    tasks
        .run_blocking_result("native-mirror-prune", move || {
            let _operation = operation;
            let _guard = guard;
            if services.state.is_shutting_down() {
                return Err(stopping());
            }
            root_guard::project_root(&services.state.projects.read(), &project)?;
            let (keep_paths, keep_tab_ids) = {
                let layouts = services.state.layouts.read();
                let layout = layouts
                    .get(&project)
                    .ok_or_else(|| AppError::NotFound("native mirror prune layout".into()))?;
                keep(layout)
            };
            let paths =
                taide_file::service::prune_mirrors(&services.state.paths, &project, &keep_paths);
            let untitled = taide_file::service::prune_untitled_mirrors(
                &services.state.paths,
                &project,
                &keep_tab_ids,
            );
            paths.and(untitled)
        })
        .await
}

#[cfg(test)]
#[path = "mirror-sweep-tests.rs"]
mod tests;

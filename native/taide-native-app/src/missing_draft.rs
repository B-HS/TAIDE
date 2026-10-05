use std::path::{Path, PathBuf};
use std::sync::Arc;

use taide_infra::root_guard;
use taide_model::error::{AppError, AppResult};
use taide_model::file::{MirrorEntry, OpenedFile, REFUSED_FILE_BYTES};
use taide_model::ids::{ProjectId, TabId};
use taide_model::layout::{Tab, TabKind};
use taide_runtime::{AppServices, AppState, TaskOperationLease, TaskSupervisor, layout_actions};
use tokio::sync::OwnedMutexGuard;

#[derive(Clone)]
pub struct MissingDraft {
    pub canonical: PathBuf,
    pub project: ProjectId,
    pub mirror: MirrorEntry,
}

pub struct PreparedMissingDraft {
    draft: MissingDraft,
    state: AppState,
    operation: TaskOperationLease,
    guard: OwnedMutexGuard<()>,
}

pub struct SavedMissingDraft {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub opened: OpenedFile,
}

impl PreparedMissingDraft {
    pub fn commit(self) -> AppResult<MissingDraft> {
        let _operation = self.operation;
        let _guard = self.guard;
        if self.state.is_shutting_down() {
            return Err(stopped());
        }
        Ok(self.draft)
    }
}

pub async fn prepare(
    state: &AppState,
    tasks: &TaskSupervisor,
    path: String,
) -> AppResult<Option<PreparedMissingDraft>> {
    let operation = tasks
        .begin_operation("native-missing-draft")
        .ok_or_else(stopped)?;
    let guard = state.begin_owned_mutation().await;
    let state = state.clone();
    tasks
        .run_blocking_result("native-missing-draft", move || {
            if state.is_shutting_down() {
                return Err(stopped());
            }
            let projects = state.projects.read().clone();
            let (project, canonical) = root_guard::resolve_owning_project_or_cli_opened(
                &projects,
                &state.cli_opened_paths.read(),
                Path::new(&path),
            )?;
            let Some(project) = project else {
                return Ok(None);
            };
            if !is_missing(&canonical)? {
                return Ok(None);
            }
            let root = root_guard::project_root(&projects, &project)?;
            let mirror = matching_mirror(&state, &project, &root, &canonical)?;
            let Some(mirror) = mirror.filter(|mirror| mirror.source_missing) else {
                return Ok(None);
            };
            if mirror.content.len() as u64 >= REFUSED_FILE_BYTES {
                return Err(AppError::Forbidden(
                    "native missing draft exceeds the file size policy".into(),
                ));
            }
            Ok(Some(PreparedMissingDraft {
                draft: MissingDraft {
                    canonical,
                    project,
                    mirror,
                },
                state,
                operation,
                guard,
            }))
        })
        .await
}

pub async fn save(
    services: &Arc<AppServices>,
    tab: TabId,
    draft: MissingDraft,
    destination: String,
) -> AppResult<SavedMissingDraft> {
    let operation = services
        .tasks
        .begin_operation("native-missing-draft-save")
        .ok_or_else(stopped)?;
    let guard = services.state.begin_owned_mutation().await;
    let services = services.clone();
    let tasks = services.tasks.clone();
    tasks.run_blocking_result("native-missing-draft-save", move || {
        let _operation = operation;
        let _guard = guard;
        if services.state.is_shutting_down() {
            return Err(stopped());
        }
        let state = &services.state;
        if draft.mirror.content.len() as u64 >= REFUSED_FILE_BYTES {
            return Err(AppError::Forbidden("native missing draft exceeds the file size policy".into()));
        }
        let projects = state.projects.read().clone();
        let source_owner = draft.project.clone();
        let root = root_guard::project_root(&projects, &source_owner)?;
        let source = root_guard::ensure_within_root(&root, &draft.canonical)?;
        if source != draft.canonical || !is_missing(&source)? {
            return Err(AppError::Forbidden("native missing draft source changed before save".into()));
        }
        if matching_mirror(state, &source_owner, &root, &source)?.as_ref() != Some(&draft.mirror) {
            return Err(AppError::Forbidden("native missing draft changed before save".into()));
        }
        let (target_owner, target) = root_guard::resolve_owning_project(&projects, Path::new(&destination))?;
        let mut layouts = state.layouts.read().clone();
        let layout_project = taide_layout::service::locate_project_with_tab(&layouts, &tab)?;
        let layout = taide_layout::service::get_layout_mut(&mut layouts, &layout_project)?;
        let candidate = taide_layout::service::all_roots(layout)
            .flat_map(crate::tabs::tabs_in)
            .find(|candidate| candidate.id == tab)
            .ok_or_else(|| AppError::NotFound("native draft tab is missing".into()))?;
        if !matches!(&candidate.kind, TabKind::File { path } if root_guard::ensure_within_root(&root, Path::new(path)).is_ok_and(|path| path == source)) {
            return Err(AppError::Forbidden("native draft tab no longer names its source".into()));
        }
        let open_destination = matches!(&candidate.kind, TabKind::File { path } if path != &destination);
        if target != source {
            let target_root = root_guard::project_root(&projects, &target_owner)?;
            if matching_mirror(state, &target_owner, &target_root, &target)?.is_some() {
                return Err(AppError::Forbidden("resolve the destination's hot-exit draft before replacing it".into()));
            }
        }
        for layout in layouts.values() {
            for candidate in taide_layout::service::all_roots(layout).flat_map(crate::tabs::tabs_in) {
                if candidate.dirty && let TabKind::File { path } = &candidate.kind
                    && root_guard::canonicalize_lenient(Path::new(path)).is_ok_and(|path| path == target && path != source)
                {
                    return Err(AppError::Forbidden("save the destination's unsaved draft before replacing it".into()));
                }
            }
        }
        let layout = taide_layout::service::get_layout_mut(&mut layouts, &layout_project)?;
        if open_destination {
            let pane = layout.focused_pane.clone();
            let title = Path::new(&destination).file_name().and_then(|name| name.to_str())
                .ok_or_else(|| AppError::InvalidArgument("native save destination has no UTF-8 file name".into()))?.to_owned();
            taide_layout::service::open_tab(layout, &pane, Tab {
                id: TabId::new(), kind: TabKind::File { path: destination }, title,
                pinned: false, preview: false, dirty: false, view_state: None,
            }, false)?;
        }
        let mut changed_projects = std::collections::HashSet::new();
        for (project, layout) in &mut layouts {
            let source_tabs: Vec<_> = taide_layout::service::all_roots(layout).flat_map(crate::tabs::tabs_in)
                .filter(|candidate| matches!(&candidate.kind, TabKind::File { path } if root_guard::canonicalize_lenient(Path::new(path)).is_ok_and(|path| path == source)))
                .map(|candidate| candidate.id.clone()).collect();
            let changed = !source_tabs.is_empty() || project == &layout_project;
            for tab in source_tabs {
                taide_layout::service::set_dirty(layout, &tab, false)?;
            }
            if changed {
                changed_projects.insert(project.clone());
            }
        }
        taide_infra::persist::write_atomic_preserving_mode(&target, draft.mirror.content.as_bytes())?;
        state.self_writes.mark(&target);
        let overlays = taide_plugin::service::language_overlays(&taide_plugin::service::ensure_loaded(&services.plugin, &state.paths.plugins_dir()));
        let opened = taide_file::service::open_file(&target, &overlays, state.settings.read().editor_config_enabled)?;
        taide_file::service::clear_mirror(&state.paths, &source_owner, &source)?;
        for project in changed_projects {
            if let Some(layout) = layouts.get_mut(&project) {
                layout_actions::finish_mutation(services.events.as_ref(), state, &project, layout);
            }
        }
        *state.layouts.write() = layouts;
        Ok(SavedMissingDraft { source, destination: target, opened })
    }).await
}

fn matching_mirror(
    state: &AppState,
    project: &ProjectId,
    root: &Path,
    source: &Path,
) -> AppResult<Option<MirrorEntry>> {
    let mut result = None;
    for mirror in taide_file::service::list_mirrors(&state.paths, project)? {
        if root_guard::ensure_within_root(root, Path::new(&mirror.path))
            .is_ok_and(|path| path == source)
        {
            if result.is_some() {
                return Err(AppError::Forbidden(
                    "multiple native mirrors name the missing source".into(),
                ));
            }
            result = Some(mirror);
        }
    }
    Ok(result)
}

fn is_missing(path: &Path) -> AppResult<bool> {
    match std::fs::metadata(path) {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(error.into()),
    }
}

fn stopped() -> AppError {
    AppError::Forbidden("native missing draft admission is stopping".into())
}

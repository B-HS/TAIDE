use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use taide_infra::root_guard;
use taide_model::error::{AppError, AppResult};
use taide_model::file::{OpenedFile, REFUSED_FILE_BYTES, UntitledMirrorEntry};
use taide_model::ids::{ProjectId, TabId};
use taide_model::layout::{ProjectLayout, TabKind};
use taide_native_editor::document::{DocumentId, DocumentKey};
use taide_native_editor::store::{EditorStore, SaveSnapshot};
use taide_runtime::{AppServices, TaskOperationLease, layout_actions};
use tokio::sync::OwnedMutexGuard;

pub struct PreparedUntitled {
    services: Arc<AppServices>,
    operation: TaskOperationLease,
    guard: OwnedMutexGuard<()>,
    pub project: ProjectId,
    pub tab: TabId,
    mirror: Option<UntitledMirrorEntry>,
}

pub struct PreparedUntitledSave {
    services: Arc<AppServices>,
    operation: TaskOperationLease,
    guard: OwnedMutexGuard<()>,
    layouts: HashMap<ProjectId, ProjectLayout>,
    project: ProjectId,
    source_tab: TabId,
    file_tab: TabId,
    canonical: PathBuf,
    opened: OpenedFile,
    snapshot: SaveSnapshot,
    mirror: Option<UntitledMirrorEntry>,
}

pub struct ConvertedUntitled {
    pub project: ProjectId,
    pub source_tab: TabId,
    pub file_tab: TabId,
    pub canonical: PathBuf,
    pub document: DocumentId,
    pub merged_document: Option<DocumentId>,
    pub mirror: Option<UntitledMirrorEntry>,
    pub clean: bool,
}

impl PreparedUntitled {
    pub fn commit(self, store: &mut EditorStore) -> AppResult<DocumentId> {
        let _operation = self.operation;
        let _guard = self.guard;
        if self.services.state.is_shutting_down() {
            return Err(stopped());
        }
        store
            .restore_untitled(
                self.tab,
                self.mirror.as_ref().map(|mirror| mirror.content.as_str()),
            )
            .map_err(editor_error)
    }
}

impl PreparedUntitledSave {
    pub fn commit(self, store: &mut EditorStore) -> AppResult<ConvertedUntitled> {
        let _operation = self.operation;
        let _guard = self.guard;
        let state = &self.services.state;
        if state.is_shutting_down() {
            return Err(stopped());
        }
        let document = self.snapshot.document();
        let live = store.documents().snapshot(document).map_err(editor_error)?;
        let clean = live.rope == *self.snapshot.rope();
        let mut layouts = self.layouts;
        let mut changed_projects = Vec::new();
        for (project, layout) in &mut layouts {
            let tabs = taide_layout::service::all_roots(layout).flat_map(crate::tabs::tabs_in)
                .filter(|tab| matches!(&tab.kind, TabKind::File { path } if root_guard::canonicalize_lenient(Path::new(path)).is_ok_and(|path| path == self.canonical)))
                .map(|tab| tab.id.clone()).collect::<Vec<_>>();
            let changed = !tabs.is_empty() || project == &self.project;
            for tab in tabs {
                taide_layout::service::set_dirty(layout, &tab, !clean)?;
            }
            if changed {
                changed_projects.push(project.clone());
            }
        }
        let merged_document = store
            .convert_untitled_save(
                self.snapshot,
                self.canonical.clone(),
                self.opened,
                self.file_tab.clone(),
            )
            .map_err(editor_error)?;
        for project in changed_projects {
            if let Some(layout) = layouts.get_mut(&project) {
                layout_actions::finish_mutation(
                    self.services.events.as_ref(),
                    state,
                    &project,
                    layout,
                );
            }
        }
        *state.layouts.write() = layouts;
        Ok(ConvertedUntitled {
            project: self.project,
            source_tab: self.source_tab,
            file_tab: self.file_tab,
            canonical: self.canonical,
            document,
            merged_document,
            mirror: self.mirror,
            clean,
        })
    }
}

pub async fn prepare(services: &Arc<AppServices>, tab: TabId) -> AppResult<PreparedUntitled> {
    let operation = services
        .tasks
        .begin_operation("native-untitled-admission")
        .ok_or_else(stopped)?;
    let guard = services.state.begin_owned_mutation().await;
    let services = services.clone();
    let tasks = services.tasks.clone();
    tasks
        .run_blocking_result("native-untitled-admission", move || {
            if services.state.is_shutting_down() {
                return Err(stopped());
            }
            root_guard::ensure_safe_component(tab.as_str())?;
            let layouts = services.state.layouts.read();
            let project = taide_layout::service::locate_project_with_tab(&layouts, &tab)?;
            ensure_untitled(&layouts[&project], &tab)?;
            root_guard::project_root(&services.state.projects.read(), &project)?;
            drop(layouts);
            let mirror = mirror(&services, &project, &tab)?;
            if mirror
                .as_ref()
                .is_some_and(|mirror| mirror.content.len() as u64 >= REFUSED_FILE_BYTES)
            {
                return Err(AppError::Forbidden(
                    "native untitled mirror exceeds the file size policy".into(),
                ));
            }
            Ok(PreparedUntitled {
                services,
                operation,
                guard,
                project,
                tab,
                mirror,
            })
        })
        .await
}

pub async fn prepare_save(
    services: &Arc<AppServices>,
    tab: TabId,
    destination: String,
    snapshot: SaveSnapshot,
) -> AppResult<PreparedUntitledSave> {
    let operation = services
        .tasks
        .begin_operation("native-untitled-save")
        .ok_or_else(stopped)?;
    let guard = services.state.begin_owned_mutation().await;
    let services = services.clone();
    let tasks = services.tasks.clone();
    tasks.run_blocking_result("native-untitled-save", move || {
        if services.state.is_shutting_down() { return Err(stopped()); }
        if snapshot.key() != &DocumentKey::Untitled(tab.clone()) { return Err(AppError::Forbidden("native untitled save identity mismatch".into())); }
        if snapshot.rope().len_bytes() as u64 >= REFUSED_FILE_BYTES { return Err(AppError::Forbidden("native untitled save exceeds the file size policy".into())); }
        root_guard::ensure_safe_component(tab.as_str())?;
        let state = &services.state;
        let projects = state.projects.read().clone();
        let (owner, canonical) = root_guard::resolve_owning_project(&projects, Path::new(&destination))?;
        let root = root_guard::project_root(&projects, &owner)?;
        if taide_file::service::list_mirrors(&state.paths, &owner)?.into_iter()
            .any(|mirror| root_guard::ensure_within_root(&root, Path::new(&mirror.path)).is_ok_and(|path| path == canonical))
        { return Err(AppError::Forbidden("resolve the destination's hot-exit draft before replacing it".into())); }
        let mut layouts = state.layouts.read().clone();
        let project = taide_layout::service::locate_project_with_tab(&layouts, &tab)?;
        root_guard::project_root(&projects, &project)?;
        ensure_untitled(&layouts[&project], &tab)?;
        for layout in layouts.values() {
            if taide_layout::service::all_roots(layout).flat_map(crate::tabs::tabs_in)
                .any(|candidate| candidate.dirty && matches!(&candidate.kind, TabKind::File { path } if root_guard::canonicalize_lenient(Path::new(path)).is_ok_and(|path| path == canonical)))
            { return Err(AppError::Forbidden("save the destination's unsaved draft before replacing it".into())); }
        }
        let layout = taide_layout::service::get_layout_mut(&mut layouts, &project)?;
        let path = canonical.to_str().ok_or_else(|| AppError::InvalidArgument("native untitled destination is not UTF-8".into()))?.to_owned();
        let title = canonical.file_name().and_then(|name| name.to_str()).ok_or_else(|| AppError::InvalidArgument("native untitled destination has no file name".into()))?.to_owned();
        let file_tab = taide_layout::service::convert_untitled_to_file(layout, &tab, path, title)?;
        let mirror = mirror(&services, &project, &tab)?;
        taide_infra::persist::write_atomic_preserving_mode(&canonical, snapshot.rope().to_string().as_bytes())?;
        state.self_writes.mark(&canonical);
        let overlays = taide_plugin::service::language_overlays(&taide_plugin::service::ensure_loaded(&services.plugin, &state.paths.plugins_dir()));
        let opened = taide_file::service::open_file(&canonical, &overlays, state.settings.read().editor_config_enabled)?;
        Ok(PreparedUntitledSave { services, operation, guard, layouts, project, source_tab: tab, file_tab, canonical, opened, snapshot, mirror })
    }).await
}

pub async fn cleanup_mirror(
    services: &Arc<AppServices>,
    project: ProjectId,
    tab: TabId,
    expected: Option<UntitledMirrorEntry>,
) -> AppResult<()> {
    let operation = services
        .tasks
        .begin_operation("native-untitled-mirror-cleanup")
        .ok_or_else(stopped)?;
    let guard = services.state.begin_owned_mutation().await;
    let services = services.clone();
    let tasks = services.tasks.clone();
    tasks
        .run_blocking_result("native-untitled-mirror-cleanup", move || {
            let _operation = operation;
            let _guard = guard;
            root_guard::ensure_safe_component(tab.as_str())?;
            root_guard::project_root(&services.state.projects.read(), &project)?;
            let layouts = services.state.layouts.read();
            if layouts
                .values()
                .any(|layout| ensure_untitled(layout, &tab).is_ok())
            {
                return Ok(());
            }
            drop(layouts);
            if mirror(&services, &project, &tab)? == expected {
                taide_file::service::clear_untitled_mirror(&services.state.paths, &project, &tab)?;
            }
            Ok(())
        })
        .await
}

fn mirror(
    services: &AppServices,
    project: &ProjectId,
    tab: &TabId,
) -> AppResult<Option<UntitledMirrorEntry>> {
    Ok(
        taide_file::service::list_untitled_mirrors(&services.state.paths, project)?
            .into_iter()
            .find(|mirror| &mirror.tab_id == tab),
    )
}

fn ensure_untitled(layout: &ProjectLayout, tab: &TabId) -> AppResult<()> {
    if taide_layout::service::all_roots(layout)
        .flat_map(crate::tabs::tabs_in)
        .any(|candidate| &candidate.id == tab && matches!(candidate.kind, TabKind::Untitled { .. }))
    {
        return Ok(());
    }
    Err(AppError::Forbidden(
        "native untitled tab no longer exists".into(),
    ))
}

fn editor_error(error: taide_native_editor::document::EditorError) -> AppError {
    AppError::Internal(format!("native untitled editor: {error:?}"))
}
fn stopped() -> AppError {
    AppError::Forbidden("native untitled admission is stopping".into())
}

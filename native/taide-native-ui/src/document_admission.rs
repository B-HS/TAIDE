use std::time::UNIX_EPOCH;

use taide_infra::root_guard;
use taide_model::error::{AppError, AppResult};
use taide_native_editor::document::{DiskChoice, DocumentId, EditorError};
use taide_native_editor::store::EditorStore;
use taide_runtime::native_file_actions::NativeOpenedFile;
use taide_runtime::{AppState, TaskOperationLease, TaskSupervisor};
use tokio::sync::OwnedMutexGuard;

pub struct PreparedDocument {
    opened: NativeOpenedFile,
    state: AppState,
    operation: TaskOperationLease,
    guard: OwnedMutexGuard<()>,
    mirror: Option<taide_model::file::MirrorEntry>,
}

pub struct CommittedDocument {
    pub document: DocumentId,
    pub restored_conflict: Option<bool>,
}

pub struct CommittedDiskChoice {
    pub document: DocumentId,
    pub canonical: std::path::PathBuf,
    pub project: Option<taide_model::ids::ProjectId>,
    pub mirror: Option<taide_model::file::MirrorEntry>,
    pub dirty: bool,
}

impl PreparedDocument {
    pub fn commit_observation(
        self,
        store: &mut EditorStore,
        document: DocumentId,
    ) -> AppResult<bool> {
        let _operation = self.operation;
        let _guard = self.guard;
        if self.state.is_shutting_down() {
            return Err(admission_stopped());
        }
        store
            .observe_file(document, &self.opened.canonical_path, self.opened.file)
            .map_err(|error| AppError::Internal(format!("native disk observation: {error:?}")))
    }

    pub fn commit_disk_choice(
        self,
        store: &mut EditorStore,
        document: DocumentId,
        expected_revision: u64,
        choice: DiskChoice,
        events: &dyn taide_runtime::EventSink,
    ) -> AppResult<CommittedDiskChoice> {
        let _operation = self.operation;
        let _guard = self.guard;
        if self.state.is_shutting_down() {
            return Err(admission_stopped());
        }
        let live = store
            .documents()
            .snapshot(document)
            .map_err(|error| AppError::Internal(format!("native disk choice: {error:?}")))?;
        let dirty = choice == DiskChoice::KeepMine && live.dirty;
        let mut layouts = self.state.layouts.read().clone();
        let mut changed = Vec::new();
        for (project, layout) in &mut layouts {
            let tabs = taide_layout::service::all_roots(layout).flat_map(tabs_in)
                .filter(|tab| matches!(&tab.kind, taide_model::layout::TabKind::File { path } if root_guard::canonicalize_lenient(std::path::Path::new(path)).is_ok_and(|path| path == self.opened.canonical_path)))
                .map(|tab| tab.id.clone()).collect::<Vec<_>>();
            if !tabs.is_empty() {
                changed.push(project.clone());
            }
            for tab in tabs {
                taide_layout::service::set_dirty(layout, &tab, dirty)?;
            }
        }
        let dirty = store
            .choose_disk(
                document,
                expected_revision,
                &self.opened.canonical_path,
                self.opened.file,
                choice,
            )
            .map_err(|error| AppError::Internal(format!("native disk choice: {error:?}")))?;
        for project in changed {
            if let Some(layout) = layouts.get_mut(&project) {
                taide_runtime::layout_actions::finish_mutation(
                    events,
                    &self.state,
                    &project,
                    layout,
                );
            }
        }
        *self.state.layouts.write() = layouts;
        Ok(CommittedDiskChoice {
            document,
            canonical: self.opened.canonical_path,
            project: self.opened.project_id,
            mirror: self.mirror,
            dirty,
        })
    }
    pub fn commit(self, store: &mut EditorStore) -> AppResult<DocumentId> {
        Ok(self.commit_with_notice(store)?.document)
    }

    pub fn commit_with_notice(self, store: &mut EditorStore) -> AppResult<CommittedDocument> {
        let _operation = self.operation;
        let _guard = self.guard;
        if self.state.is_shutting_down() {
            return Err(admission_stopped());
        }
        let existed = store
            .documents()
            .find(&taide_native_editor::document::DocumentKey::File(
                self.opened.canonical_path.clone(),
            ))
            .is_some();
        let restored_conflict = (!existed)
            .then_some(self.mirror.as_ref().map(|mirror| mirror.conflict))
            .flatten();
        let document = store
            .open_file_with_draft(
                self.opened.canonical_path,
                self.opened.file,
                self.mirror.as_ref().map(|mirror| mirror.content.as_str()),
            )
            .map_err(|error| match error {
                EditorError::Refused | EditorError::ReadOnly => {
                    AppError::Forbidden(format!("native document policy: {error:?}"))
                }
                _ => AppError::Internal(format!("native document admission: {error:?}")),
            })?;
        Ok(CommittedDocument {
            document,
            restored_conflict,
        })
    }
}

pub async fn prepare_opened_document(
    state: &AppState,
    tasks: &TaskSupervisor,
    opened: NativeOpenedFile,
) -> AppResult<PreparedDocument> {
    let operation = tasks
        .begin_operation("native-document-admission")
        .ok_or_else(admission_stopped)?;
    let guard = state.begin_owned_mutation().await;
    let state = state.clone();
    tasks
        .run_blocking_result("native-document-validate", move || {
            if state.is_shutting_down() {
                return Err(admission_stopped());
            }
            let projects = state.projects.read().clone();
            let (project, canonical) = root_guard::resolve_owning_project_or_cli_opened(
                &projects,
                &state.cli_opened_paths.read(),
                &opened.canonical_path,
            )?;
            if project != opened.project_id || canonical != opened.canonical_path {
                return Err(AppError::Forbidden(
                    "native file ownership changed before document admission".into(),
                ));
            }
            let metadata = std::fs::metadata(&canonical)?;
            let modified = metadata
                .modified()?
                .duration_since(UNIX_EPOCH)
                .map_err(|error| AppError::Io(error.to_string()))?
                .as_secs_f64()
                * taide_infra::clock::MS_PER_SECOND;
            if metadata.len() != u64::from(opened.file.byte_size)
                || modified != opened.file.modified_ms
            {
                return Err(AppError::Forbidden(
                    "file changed before native document admission".into(),
                ));
            }
            let mut mirror = None;
            if let Some(project) = &opened.project_id {
                let root = root_guard::project_root(&projects, project)?;
                for entry in taide_file::service::list_mirrors(&state.paths, project)? {
                    if root_guard::ensure_within_root(&root, std::path::Path::new(&entry.path))
                        .is_ok_and(|path| path == canonical)
                    {
                        if mirror.is_some() {
                            return Err(AppError::Forbidden(
                                "multiple native mirrors name the same canonical document".into(),
                            ));
                        }
                        mirror = Some(entry);
                    }
                }
            }
            Ok(PreparedDocument {
                opened,
                state,
                operation,
                guard,
                mirror,
            })
        })
        .await
}

fn admission_stopped() -> AppError {
    AppError::Forbidden("native document admission is shutting down".into())
}

fn tabs_in(node: &taide_model::layout::PaneNode) -> Vec<&taide_model::layout::Tab> {
    match node {
        taide_model::layout::PaneNode::Leaf { tabs, .. } => tabs.iter().collect(),
        taide_model::layout::PaneNode::Split { children, .. } => {
            children.iter().flat_map(tabs_in).collect()
        }
    }
}

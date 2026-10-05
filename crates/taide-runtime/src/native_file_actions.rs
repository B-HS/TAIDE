use std::path::{Path, PathBuf};

use taide_file::service;
use taide_infra::language::LanguageOverlay;
use taide_infra::perf::{self, SpanSlot};
use taide_infra::root_guard;
use taide_model::error::AppResult;
use taide_model::file::OpenedFile;
use taide_model::ids::ProjectId;

use crate::file_actions::run_guarded_file_worker;
use crate::{AppState, TaskSupervisor};

#[derive(Debug)]
pub struct NativeOpenedFile {
    pub canonical_path: PathBuf,
    pub display_path: String,
    pub project_id: Option<ProjectId>,
    pub file: OpenedFile,
}

/// Opens an authorized file for native document admission without changing its display path.
pub async fn open_document_file(
    state: &AppState,
    tasks: &TaskSupervisor,
    path: String,
    load_language_overlays: impl FnOnce() -> Vec<LanguageOverlay> + Send + 'static,
) -> AppResult<NativeOpenedFile> {
    run_guarded_file_worker(state, tasks, "native-file-open", |state| {
        let shared_state = state.clone();
        Ok(move || {
            let _span = perf::span(SpanSlot::FileOpen);
            let projects = shared_state.projects.read().clone();
            let (project_id, canonical_path) =
                root_guard::resolve_owning_project_or_cli_opened(&projects, &shared_state.cli_opened_paths.read(), Path::new(&path))?;
            let editor_config_enabled = shared_state.settings.read().editor_config_enabled;
            let overlays = load_language_overlays();
            let file = service::open_file(&canonical_path, &overlays, editor_config_enabled)?;
            Ok(NativeOpenedFile {
                canonical_path,
                display_path: path,
                project_id,
                file,
            })
        })
    })
    .await
}

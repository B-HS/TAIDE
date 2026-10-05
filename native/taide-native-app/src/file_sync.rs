use std::path::Path;
use std::sync::Arc;

use taide_infra::root_guard;
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::file::{OpenedFile, REFUSED_FILE_BYTES};
use taide_native_editor::document::DocumentKey;
use taide_native_editor::store::SaveSnapshot;
use taide_runtime::AppServices;

pub async fn prepare_document(
    services: &Arc<AppServices>,
    path: String,
) -> AppResult<taide_native_ui::document_admission::PreparedDocument> {
    let plugins = services.plugin.clone();
    let plugins_dir = services.state.paths.plugins_dir();
    let opened = taide_runtime::native_file_actions::open_document_file(
        &services.state,
        &services.tasks,
        path,
        move || {
            taide_plugin::service::language_overlays(&taide_plugin::service::ensure_loaded(
                &plugins,
                &plugins_dir,
            ))
        },
    )
    .await?;
    taide_native_ui::document_admission::prepare_opened_document(
        &services.state,
        &services.tasks,
        opened,
    )
    .await
}

pub async fn cleanup_mirror(
    services: &Arc<AppServices>,
    project: taide_model::ids::ProjectId,
    canonical: std::path::PathBuf,
    expected: Option<taide_model::file::MirrorEntry>,
) -> AppResult<()> {
    let operation = services
        .tasks
        .begin_operation("native-file-mirror-cleanup")
        .ok_or_else(|| AppError::Forbidden("native mirror cleanup is stopping".into()))?;
    let guard = services.state.begin_owned_mutation().await;
    let services = services.clone();
    let tasks = services.tasks.clone();
    tasks
        .run_blocking_result("native-file-mirror-cleanup", move || {
            let _operation = operation;
            let _guard = guard;
            let root = root_guard::project_root(&services.state.projects.read(), &project)?;
            if root_guard::ensure_within_root(&root, &canonical)? != canonical {
                return Err(AppError::Forbidden(
                    "native mirror cleanup identity changed".into(),
                ));
            }
            let mut current = None;
            for mirror in taide_file::service::list_mirrors(&services.state.paths, &project)? {
                if root_guard::ensure_within_root(&root, Path::new(&mirror.path))
                    .is_ok_and(|path| path == canonical)
                {
                    if current.is_some() {
                        return Err(AppError::Forbidden(
                            "multiple native mirrors name the disk choice".into(),
                        ));
                    }
                    current = Some(mirror);
                }
            }
            if current == expected {
                taide_file::service::clear_mirror_if_current(
                    &services.state.paths,
                    &project,
                    &canonical,
                    expected.as_ref(),
                )?;
            }
            Ok(())
        })
        .await
}

pub async fn save_snapshot(
    services: &Arc<AppServices>,
    path: String,
    snapshot: SaveSnapshot,
    epoch: Option<crate::persistence::DraftEpoch>,
) -> AppResult<OpenedFile> {
    let operation = services
        .tasks
        .begin_operation("native-file-save-snapshot")
        .ok_or_else(|| AppError::Forbidden("native file save is stopping".into()))?;
    let guard = services.state.begin_owned_mutation().await;
    let services = services.clone();
    let tasks = services.tasks.clone();
    tasks
        .run_blocking_result("native-file-save-snapshot", move || {
            let _operation = operation;
            let _guard = guard;
            let state = &services.state;
            if state.is_shutting_down() {
                return Err(AppError::Forbidden("native file save is stopping".into()));
            }
            if epoch.as_ref().is_some_and(|epoch| !epoch.is_current()) {
                return Err(AppError::Forbidden("native file save was cancelled".into()));
            }
            let (_, canonical) = root_guard::resolve_owning_project_or_cli_opened(
                &state.projects.read(),
                &state.cli_opened_paths.read(),
                Path::new(&path),
            )?;
            if snapshot.key() != &DocumentKey::File(canonical.clone()) {
                return Err(AppError::Forbidden(
                    "native save target changed its canonical document identity".into(),
                ));
            }
            if snapshot.rope().len_bytes() as u64 >= REFUSED_FILE_BYTES {
                return Err(AppError::Forbidden(
                    "native save snapshot exceeds the file size policy".into(),
                ));
            }
            let overlays = taide_plugin::service::language_overlays(
                &taide_plugin::service::ensure_loaded(&services.plugin, &state.paths.plugins_dir()),
            );
            let config = state.settings.read().editor_config_enabled;
            match taide_file::service::open_file(&canonical, &overlays, config) {
                Ok(file) if file.read_only => {
                    return Err(AppError::Forbidden(
                        "native save target is read-only".into(),
                    ));
                }
                Ok(_) => {}
                Err(error) if error.kind() == AppErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
            if epoch.as_ref().is_some_and(|epoch| !epoch.is_current()) {
                return Err(AppError::Forbidden("native file save was cancelled".into()));
            }
            taide_runtime::file_actions::save_file_within_open_projects(
                state,
                &canonical,
                &snapshot.rope().to_string(),
            )?;
            if let Some(epoch) = epoch {
                epoch.invalidate();
            }
            taide_file::service::open_file(&canonical, &overlays, config)
        })
        .await
}

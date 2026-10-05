use std::path::{Path, PathBuf};
use std::sync::Arc;

use taide_infra::root_guard;
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_runtime::AppServices;
use tokio::sync::{mpsc, oneshot};

use crate::explorer::RenameRequest;
use crate::explorer_source_missing::{Changed, Prepare, preserve_drafts};
use crate::lsp::Reply;
use crate::workspace_activity::ActivityOwner;
use crate::workspace_rename::{DocumentMove, Renamed};

pub async fn move_selected(
    services: &Arc<AppServices>,
    project: &ProjectId,
    request: &RenameRequest,
    replies: &mpsc::Sender<Reply>,
    repaint: &Arc<dyn Fn() + Send + Sync>,
) -> AppResult<()> {
    let operation = services
        .tasks
        .begin_operation("native-explorer-selected-move")
        .ok_or_else(|| AppError::Forbidden("native explorer move is stopping".into()))?;
    let guard = services.state.begin_owned_mutation().await;
    let checked_services = services.clone();
    let checked_project = project.clone();
    let checked_request = request.clone();
    let (root, from, to) = services
        .tasks
        .run_blocking_result("native-explorer-selected-move-path", move || {
            paths(&checked_services, &checked_project, &checked_request)
        })
        .await?;
    if !from.starts_with(&root) {
        return Err(AppError::Forbidden(
            "native explorer source is outside its project".into(),
        ));
    }
    let owner = ActivityOwner::new(repaint.clone());
    let (completion, receive) = oneshot::channel();
    replies
        .send(Reply::RenamePrepare(crate::workspace_rename::Prepare {
            from: from.clone(),
            to: to.clone(),
            activity: owner.activity.clone(),
            completion,
        }))
        .await
        .map_err(|_| AppError::Internal("native explorer editor is unavailable".into()))?;
    repaint();
    let sources = receive
        .await
        .map_err(|_| AppError::Internal("native explorer editor stopped before move".into()))??;
    let work_services = services.clone();
    let requested_project = project.clone();
    let requested = request.clone();
    let requested_from = from.clone();
    let requested_to = to.clone();
    let (documents, layouts, survivor_drafts, warnings, guard, operation, owner) = services
        .tasks
        .run_blocking_result("native-explorer-selected-move-file", move || {
            let (root, from, to) = paths(&work_services, &requested_project, &requested)?;
            if from != requested_from || to != requested_to || !from.starts_with(&root) {
                return Err(AppError::Forbidden(
                    "native explorer move identity changed".into(),
                ));
            }
            let mut layouts = work_services.state.layouts.read().clone();
            let selected = taide_layout::service::get_layout_mut(&mut layouts, &requested_project)?;
            let change = taide_model::layout::TabPathChange::Renamed {
                from: requested.from.clone(),
                to: requested.to.clone(),
            };
            let outcome = taide_layout::service::apply_tab_path_change(selected, &change);
            let mut mirrors = Vec::new();
            let existing =
                taide_file::service::list_mirrors(&work_services.state.paths, &requested_project)?;
            for moved in outcome.moved {
                let old = root_guard::canonicalize_lenient(Path::new(&moved.from))?;
                let new = root_guard::canonicalize_lenient(Path::new(&moved.to))?;
                root_guard::ensure_within_root(&root, &new)?;
                let cached = existing.iter().find(|mirror| mirror.path == moved.from);
                let source = sources.iter().find(|source| {
                    source.snapshot.key
                        == taide_native_editor::document::DocumentKey::File(old.clone())
                        || source
                            .display_paths
                            .iter()
                            .any(|path| path == Path::new(&moved.from))
                });
                let draft = if moved.dirty
                    || cached.is_some()
                    || source.is_some_and(|source| source.snapshot.dirty)
                {
                    source
                        .map(|source| source.snapshot.rope.to_string())
                        .or_else(|| cached.map(|mirror| mirror.content.clone()))
                } else {
                    None
                };
                mirrors.push((old, new, moved.to, draft));
            }
            let snapshots = sources
                .iter()
                .map(|source| source.snapshot.clone())
                .collect::<Vec<_>>();
            let survivor_drafts = preserve_drafts(&work_services, &from, &snapshots, &layouts)?;
            let projects = work_services.state.projects.read().clone();
            let affected = affected_projects(&work_services, &from, &to);
            let overlays =
                taide_plugin::service::language_overlays(&taide_plugin::service::ensure_loaded(
                    &work_services.plugin,
                    &work_services.state.paths.plugins_dir(),
                ));
            let config = work_services.state.settings.read().editor_config_enabled;
            taide_file::service::rename_entry(&from, &to)?;
            work_services.state.self_writes.mark(&from);
            work_services.state.self_writes.mark(&to);
            if let Some(selected) = layouts.get_mut(&requested_project) {
                taide_runtime::layout_actions::finish_mutation(
                    work_services.events.as_ref(),
                    &work_services.state,
                    &requested_project,
                    selected,
                );
            }
            *work_services.state.layouts.write() = layouts.clone();
            for project_id in affected {
                work_services
                    .events
                    .publish(AppEvent::FsRescanRequired { project_id });
            }
            let mut warnings = Vec::new();
            for (old, canonical, display, draft) in mirrors {
                let result = (|| {
                    if let Some(draft) = draft {
                        taide_file::service::mirror_dirty(
                            &work_services.state.paths,
                            &requested_project,
                            &canonical,
                            &display,
                            &draft,
                        )?;
                        if canonical == old {
                            return Ok(());
                        }
                    }
                    if survivor_drafts
                        .values()
                        .any(|draft| draft.project == requested_project && draft.canonical == old)
                    {
                        return Ok(());
                    }
                    taide_file::service::clear_mirror(
                        &work_services.state.paths,
                        &requested_project,
                        &old,
                    )
                })();
                if let Err(error) = result {
                    warnings.push(error);
                }
            }
            let mut documents = Vec::new();
            for source in sources {
                let taide_native_editor::document::DocumentKey::File(old) = &source.snapshot.key
                else {
                    continue;
                };
                let next = crate::workspace_rename::moved_path(old, &from, &to).or_else(|| {
                    source
                        .display_paths
                        .iter()
                        .find_map(|path| crate::workspace_rename::moved_path(path, &from, &to))
                });
                let Some(next) = next else { continue };
                let canonical = match root_guard::resolve_owning_project(&projects, &next) {
                    Ok((_, canonical)) => canonical,
                    Err(error) => {
                        warnings.push(error);
                        next
                    }
                };
                let metadata = match taide_file::service::open_file(&canonical, &overlays, config) {
                    Ok(file) => taide_native_editor::document::DocumentMetadata::from_opened(&file),
                    Err(error) => {
                        warnings.push(error);
                        source.snapshot.metadata.clone()
                    }
                };
                documents.push(DocumentMove {
                    requested: source.snapshot,
                    canonical,
                    metadata,
                });
            }
            Ok((
                documents,
                layouts,
                survivor_drafts,
                warnings,
                guard,
                operation,
                owner,
            ))
        })
        .await?;
    let (completion, receive) = oneshot::channel();
    replies
        .send(Reply::Renamed(Renamed {
            from,
            to,
            documents,
            layouts,
            survivor_drafts,
            warnings,
            completion,
        }))
        .await
        .map_err(|_| AppError::Internal("native explorer editor stopped after move".into()))?;
    repaint();
    let result = receive
        .await
        .map_err(|_| AppError::Internal("native explorer move acknowledgement stopped".into()))?;
    drop(guard);
    drop(operation);
    drop(owner);
    result?;
    Ok(())
}

fn paths(
    services: &AppServices,
    project: &ProjectId,
    request: &RenameRequest,
) -> AppResult<(PathBuf, PathBuf, PathBuf)> {
    if services.state.is_shutting_down() {
        return Err(AppError::Forbidden(
            "native explorer move is stopping".into(),
        ));
    }
    if !Path::new(&request.from).is_absolute() || !Path::new(&request.to).is_absolute() {
        return Err(AppError::InvalidArgument(
            "native explorer move paths must be absolute".into(),
        ));
    }
    let projects = services.state.projects.read();
    let root = root_guard::canonicalize_lenient(&root_guard::project_root(&projects, project)?)?;
    let (_, from) = root_guard::resolve_entry_owning_project(&projects, Path::new(&request.from))?;
    let (_, to) = root_guard::resolve_entry_owning_project(&projects, Path::new(&request.to))?;
    if to == root || !to.starts_with(&root) {
        return Err(AppError::Forbidden(
            "native explorer move destination is outside its project".into(),
        ));
    }
    Ok((root, from, to))
}

fn affected_projects(services: &AppServices, from: &Path, to: &Path) -> Vec<ProjectId> {
    services
        .state
        .projects
        .read()
        .values()
        .filter(|project| {
            root_guard::canonicalize_lenient(Path::new(&project.root))
                .is_ok_and(|root| from.starts_with(&root) || to.starts_with(&root))
        })
        .map(|project| project.id.clone())
        .collect()
}

pub async fn try_cross_project_move(
    services: &Arc<AppServices>,
    project: &ProjectId,
    request: &RenameRequest,
    replies: &mpsc::Sender<Reply>,
    repaint: &Arc<dyn Fn() + Send + Sync>,
) -> AppResult<bool> {
    let operation = services
        .tasks
        .begin_operation("native-explorer-move")
        .ok_or_else(|| AppError::Forbidden("native explorer move is stopping".into()))?;
    let guard = services.state.begin_owned_mutation().await;
    let check_services = services.clone();
    let checked_project = project.clone();
    let checked_request = request.clone();
    let (root, from, to) = services
        .tasks
        .run_blocking_result("native-explorer-move-path", move || {
            paths(&check_services, &checked_project, &checked_request)
        })
        .await?;
    if from.starts_with(&root) {
        return Ok(false);
    }
    let owner = ActivityOwner::new(repaint.clone());
    let (completion, receive) = oneshot::channel();
    replies
        .send(Reply::ExplorerMovePrepare(Prepare {
            path: from.clone(),
            activity: owner.activity.clone(),
            completion,
        }))
        .await
        .map_err(|_| AppError::Internal("native explorer editor is unavailable".into()))?;
    repaint();
    let documents = receive
        .await
        .map_err(|_| AppError::Internal("native explorer editor stopped before move".into()))??;
    let work_services = services.clone();
    let requested_project = project.clone();
    let requested = request.clone();
    let requested_from = from.clone();
    let requested_to = to.clone();
    let snapshots = documents.clone();
    let (layouts, drafts, guard, operation, owner) = services
        .tasks
        .run_blocking_result("native-explorer-move-file", move || {
            let (_, from, to) = paths(&work_services, &requested_project, &requested)?;
            if from != requested_from || to != requested_to {
                return Err(AppError::Forbidden(
                    "native explorer move identity changed".into(),
                ));
            }
            let layouts = work_services.state.layouts.read().clone();
            let drafts = preserve_drafts(&work_services, &from, &snapshots, &layouts)?;
            let affected = affected_projects(&work_services, &from, &to);
            taide_file::service::rename_entry(&from, &to)?;
            work_services.state.self_writes.mark(&from);
            work_services.state.self_writes.mark(&to);
            for project_id in affected {
                work_services
                    .events
                    .publish(AppEvent::FsRescanRequired { project_id });
            }
            Ok((layouts, drafts, guard, operation, owner))
        })
        .await?;
    let (completion, receive) = oneshot::channel();
    replies
        .send(Reply::ExplorerMoved(Changed {
            path: from,
            documents,
            layouts,
            drafts,
            completion,
        }))
        .await
        .map_err(|_| AppError::Internal("native explorer editor stopped after move".into()))?;
    repaint();
    let result = receive
        .await
        .map_err(|_| AppError::Internal("native explorer move acknowledgement stopped".into()))?;
    drop(guard);
    drop(operation);
    drop(owner);
    result?;
    Ok(true)
}

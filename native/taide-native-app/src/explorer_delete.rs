use std::path::{Path, PathBuf};
use std::sync::Arc;

use taide_infra::root_guard;
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::layout::TabPathChange;
use taide_native_editor::document::DocumentId;
use taide_runtime::{AppServices, layout_actions};
use tokio::sync::{mpsc, oneshot};

use crate::lsp::Reply;
use crate::workspace_activity::ActivityOwner;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub project: ProjectId,
    pub path: String,
    pub name: String,
}

pub use crate::explorer_source_missing::{
    Changed as Deleted, Prepare, commit_documents, prepare_documents,
};

fn scoped_path(services: &AppServices, request: &Request) -> AppResult<PathBuf> {
    if services.state.is_shutting_down() {
        return Err(AppError::Forbidden(
            "native explorer deletion is stopping".into(),
        ));
    }
    let path = Path::new(&request.path);
    if !path.is_absolute() {
        return Err(AppError::InvalidArgument(
            "explorer deletion requires an absolute path".into(),
        ));
    }
    let projects = services.state.projects.read();
    let root = root_guard::project_root(&projects, &request.project)?;
    let (_, resolved) = root_guard::resolve_entry_owning_project(&projects, path)?;
    if resolved == root || !resolved.starts_with(&root) {
        return Err(AppError::Forbidden(
            "explorer deletion is outside the selected project entries".into(),
        ));
    }
    Ok(resolved)
}

pub async fn apply(
    services: &Arc<AppServices>,
    request: Request,
    replies: &mpsc::Sender<Reply>,
    repaint: &Arc<dyn Fn() + Send + Sync>,
) -> AppResult<Vec<DocumentId>> {
    let operation = services
        .tasks
        .begin_operation("native-explorer-delete")
        .ok_or_else(|| AppError::Forbidden("native explorer deletion is stopping".into()))?;
    let guard = services.state.begin_owned_mutation().await;
    let check_services = services.clone();
    let checked = request.clone();
    let path = services
        .tasks
        .run_blocking_result("native-explorer-delete-path", move || {
            scoped_path(&check_services, &checked)
        })
        .await?;
    let owner = ActivityOwner::new(repaint.clone());
    let (completion, receive) = oneshot::channel();
    replies
        .send(Reply::ExplorerDeletePrepare(Prepare {
            path: path.clone(),
            activity: owner.activity.clone(),
            completion,
        }))
        .await
        .map_err(|_| AppError::Internal("native explorer editor is unavailable".into()))?;
    repaint();
    let documents = receive
        .await
        .map_err(|_| AppError::Internal("native explorer editor stopped".into()))??;
    let work_services = services.clone();
    let requested = path.clone();
    let snapshots = documents.clone();
    let (layouts, drafts, guard, operation, owner) = services
        .tasks
        .run_blocking_result("native-explorer-delete-file", move || {
            if scoped_path(&work_services, &request)? != requested {
                return Err(AppError::Forbidden(
                    "explorer deletion identity changed".into(),
                ));
            }
            let mut layouts = work_services.state.layouts.read().clone();
            let selected = taide_layout::service::get_layout_mut(&mut layouts, &request.project)?;
            taide_layout::service::apply_tab_path_change(
                selected,
                &TabPathChange::Deleted {
                    path: request.path.clone(),
                },
            );
            let drafts = crate::explorer_source_missing::preserve_drafts(
                &work_services,
                &requested,
                &snapshots,
                &layouts,
            )?;
            taide_file::service::delete_entry(&requested)?;
            work_services.state.self_writes.mark(&requested);
            let selected = taide_layout::service::get_layout_mut(&mut layouts, &request.project)?;
            layout_actions::finish_mutation(
                work_services.events.as_ref(),
                &work_services.state,
                &request.project,
                selected,
            );
            *work_services.state.layouts.write() = layouts.clone();
            work_services.events.publish(AppEvent::FsRescanRequired {
                project_id: request.project,
            });
            Ok((layouts, drafts, guard, operation, owner))
        })
        .await?;
    let (completion, receive) = oneshot::channel();
    replies
        .send(Reply::ExplorerDeleted(Deleted {
            path,
            documents,
            layouts,
            drafts,
            completion,
        }))
        .await
        .map_err(|_| AppError::Internal("native explorer editor stopped after deletion".into()))?;
    repaint();
    let result = receive.await.map_err(|_| {
        AppError::Internal("native explorer deletion acknowledgement stopped".into())
    })?;
    drop(guard);
    drop(operation);
    drop(owner);
    result
}

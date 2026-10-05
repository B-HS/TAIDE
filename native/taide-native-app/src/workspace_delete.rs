use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use taide_infra::root_guard;
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::{ProjectId, TabId};
use taide_model::layout::{ProjectLayout, TabKind, TabPathChange};
use taide_native_editor::document::{DocumentId, DocumentKey, DocumentSnapshot};
use taide_native_editor::store::EditorStore;
use taide_runtime::{AppServices, layout_actions};
use tokio::sync::{mpsc, oneshot};

use crate::lsp::Reply;
use crate::workspace_activity::{Activity, ActivityOwner, scoped_entry};

pub struct Prepare {
    pub path: PathBuf,
    pub activity: Activity,
    pub completion: oneshot::Sender<AppResult<Vec<DocumentSnapshot>>>,
}

pub struct Deleted {
    pub path: PathBuf,
    pub documents: Vec<DocumentSnapshot>,
    pub layouts: HashMap<ProjectId, ProjectLayout>,
    pub completion: oneshot::Sender<AppResult<Vec<DocumentId>>>,
}

pub fn prepare_documents(
    store: &EditorStore,
    path: &Path,
    additional: &HashSet<DocumentId>,
) -> AppResult<Vec<DocumentSnapshot>> {
    let documents = store
        .documents()
        .snapshots()
        .filter(|snapshot| {
            additional.contains(&snapshot.id)
                || matches!(&snapshot.key, DocumentKey::File(file) if file.starts_with(path))
        })
        .collect::<Vec<_>>();
    if documents.iter().any(|document| document.dirty) {
        return Err(AppError::Forbidden(
            "save or close unsaved files before deleting them".into(),
        ));
    }
    Ok(documents)
}

pub fn commit_documents(
    store: &mut EditorStore,
    event: &Deleted,
    retained: &HashSet<DocumentId>,
) -> AppResult<Vec<DocumentId>> {
    for document in &event.documents {
        let current = store
            .documents()
            .snapshot(document.id)
            .map_err(editor_error)?;
        if current.key != document.key || current.revision != document.revision || current.dirty {
            return Err(AppError::Forbidden(
                "deleted document changed before release".into(),
            ));
        }
    }
    let tabs = event
        .layouts
        .values()
        .flat_map(taide_layout::service::all_roots)
        .flat_map(crate::tabs::tabs_in)
        .map(|tab| tab.id.clone())
        .collect::<HashSet<TabId>>();
    let mut released = Vec::new();
    for document in &event.documents {
        let views = store
            .views()
            .for_document(document.id)
            .filter(|view| !tabs.contains(&view.key.tab))
            .map(|view| view.id)
            .collect::<Vec<_>>();
        for view in views {
            store.detach_view(view).map_err(editor_error)?;
        }
        if !retained.contains(&document.id)
            && store.views().for_document(document.id).next().is_none()
        {
            store.release_document(document.id).map_err(editor_error)?;
            released.push(document.id);
        }
    }
    Ok(released)
}

fn editor_error(error: taide_native_editor::document::EditorError) -> AppError {
    AppError::Forbidden(format!("workspace delete document rejected: {error:?}"))
}

pub async fn apply(
    services: &Arc<AppServices>,
    delete: taide_lsp::native::protocol::lsp_types::DeleteFile,
    roots: Option<Vec<String>>,
    replies: &mpsc::Sender<Reply>,
    repaint: &Arc<dyn Fn() + Send + Sync>,
) -> AppResult<Vec<DocumentId>> {
    let path = crate::lsp_workspace_worker::uri_path(&delete.uri)?;
    let ignore_missing = delete
        .options
        .is_some_and(|options| options.ignore_if_not_exists == Some(true));
    let operation = services
        .tasks
        .begin_operation("native-workspace-delete")
        .ok_or_else(|| AppError::Forbidden("workspace file operations are stopping".into()))?;
    let guard = services.state.begin_owned_mutation().await;
    let check_services = services.clone();
    let check_roots = roots.clone();
    let path = services
        .tasks
        .run_blocking_result("native-workspace-delete-path", move || {
            let path = scoped_entry(&check_services, &path, check_roots.as_deref())?;
            if ignore_missing
                && std::fs::metadata(&path)
                    .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
            {
                return Ok(None);
            }
            Ok(Some(path))
        })
        .await?;
    let Some(path) = path else {
        return Ok(Vec::new());
    };
    let owner = ActivityOwner::new(repaint.clone());
    let (completion, receive) = oneshot::channel();
    replies
        .send(Reply::DeletePrepare(Prepare {
            path: path.clone(),
            activity: owner.activity.clone(),
            completion,
        }))
        .await
        .map_err(|_| AppError::Internal("workspace editor is unavailable".into()))?;
    repaint();
    let documents = receive
        .await
        .map_err(|_| AppError::Internal("workspace editor stopped".into()))??;
    let work_services = services.clone();
    let requested = path.clone();
    let (layouts, guard, operation, owner) = services.tasks.run_blocking_result("native-workspace-delete-file", move || {
        let canonical = scoped_entry(&work_services, &requested, roots.as_deref())?;
        if canonical != requested {
            return Err(AppError::Forbidden("workspace delete identity changed".into()));
        }
        let projects = work_services.state.projects.read().clone();
        let mut matching = Vec::new();
        for project in projects.values() {
            let root = root_guard::project_root(&projects, &project.id)?;
            if !requested.starts_with(&root) { continue; }
            if taide_file::service::list_mirrors(&work_services.state.paths, &project.id)?.iter()
                .any(|mirror| Path::new(&mirror.path).starts_with(&requested))
            {
                return Err(AppError::Forbidden("save or close workspace mirrors before deleting them".into()));
            }
            matching.push(project.id.clone());
        }
        let mut layouts = work_services.state.layouts.read().clone();
        let path_string = requested.to_str().ok_or_else(|| AppError::InvalidArgument("workspace path must be UTF-8".into()))?.to_owned();
        let change = TabPathChange::Deleted {path: path_string};
        for project in &matching {
            let layout = taide_layout::service::get_layout_mut(&mut layouts, project)?;
            if taide_layout::service::all_roots(layout).flat_map(crate::tabs::tabs_in)
                .any(|tab| matches!(&tab.kind, TabKind::File {path} if Path::new(path).starts_with(&requested)) && tab.dirty)
            {
                return Err(AppError::Forbidden("save or close unsaved tabs before deleting them".into()));
            }
            taide_layout::service::apply_tab_path_change(layout, &change);
            if taide_layout::service::all_roots(layout).flat_map(crate::tabs::tabs_in)
                .any(|tab| matches!(&tab.kind, TabKind::File {path} if Path::new(path).starts_with(&requested)))
            {
                return Err(AppError::Forbidden("workspace file tabs could not close before deletion".into()));
            }
        }
        taide_file::service::delete_entry(&requested)?;
        work_services.state.self_writes.mark(&requested);
        for project in &matching {
            let layout = taide_layout::service::get_layout_mut(&mut layouts, project)?;
            layout_actions::finish_mutation(work_services.events.as_ref(), &work_services.state, project, layout);
        }
        *work_services.state.layouts.write() = layouts.clone();
        for project in matching {
            work_services.events.publish(AppEvent::FsRescanRequired {project_id: project});
        }
        Ok((layouts, guard, operation, owner))
    }).await?;
    let (completion, receive) = oneshot::channel();
    replies
        .send(Reply::Deleted(Deleted {
            path,
            documents,
            layouts,
            completion,
        }))
        .await
        .map_err(|_| AppError::Internal("workspace editor is unavailable after deletion".into()))?;
    repaint();
    let result = receive
        .await
        .map_err(|_| AppError::Internal("workspace editor stopped after deletion".into()))?;
    drop(guard);
    drop(operation);
    drop(owner);
    result
}

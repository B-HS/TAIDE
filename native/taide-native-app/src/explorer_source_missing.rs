use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use taide_infra::root_guard;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::{ProjectId, TabId};
use taide_model::layout::{ProjectLayout, TabKind};
use taide_native_editor::document::{DocumentId, DocumentKey, DocumentSnapshot};
use taide_native_editor::store::EditorStore;
use taide_runtime::AppServices;
use tokio::sync::oneshot;

use crate::missing_draft::MissingDraft;
use crate::workspace_activity::Activity;

pub struct Prepare {
    pub path: PathBuf,
    pub activity: Activity,
    pub completion: oneshot::Sender<AppResult<Vec<DocumentSnapshot>>>,
}

pub struct Changed {
    pub path: PathBuf,
    pub documents: Vec<DocumentSnapshot>,
    pub layouts: HashMap<ProjectId, ProjectLayout>,
    pub drafts: HashMap<String, MissingDraft>,
    pub completion: oneshot::Sender<AppResult<Vec<DocumentId>>>,
}

pub fn prepare_documents(
    store: &EditorStore,
    path: &Path,
    additional: &HashSet<DocumentId>,
) -> Vec<DocumentSnapshot> {
    store
        .documents()
        .snapshots()
        .filter(|snapshot| {
            additional.contains(&snapshot.id)
                || matches!(&snapshot.key, DocumentKey::File(file) if file.starts_with(path))
        })
        .collect()
}

pub fn commit_documents(
    store: &mut EditorStore,
    event: &Changed,
    retained: &HashSet<DocumentId>,
) -> AppResult<Vec<DocumentId>> {
    for document in &event.documents {
        let current = store
            .documents()
            .snapshot(document.id)
            .map_err(editor_error)?;
        if current.key != document.key || current.revision != document.revision {
            return Err(AppError::Forbidden(
                "missing source document changed before release".into(),
            ));
        }
    }
    let tabs = event
        .layouts
        .values()
        .flat_map(taide_layout::service::all_roots)
        .flat_map(crate::tabs::tabs_in)
        .filter(
            |tab| !matches!(&tab.kind, TabKind::File { path } if event.drafts.contains_key(path)),
        )
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
            store
                .discard_document(document.id, document.revision)
                .map_err(editor_error)?;
            released.push(document.id);
        }
    }
    Ok(released)
}

pub fn preserve_drafts(
    services: &AppServices,
    path: &Path,
    documents: &[DocumentSnapshot],
    layouts: &HashMap<ProjectId, ProjectLayout>,
) -> AppResult<HashMap<String, MissingDraft>> {
    let projects = services.state.projects.read().clone();
    let mut drafts = HashMap::new();
    let mut written = HashSet::new();
    for (project, layout) in layouts {
        for tab in taide_layout::service::all_roots(layout).flat_map(crate::tabs::tabs_in) {
            let TabKind::File { path: file } = &tab.kind else {
                continue;
            };
            if !Path::new(file).starts_with(path) {
                continue;
            }
            let (source_project, canonical) =
                root_guard::resolve_owning_project(&projects, Path::new(file))?;
            let root = root_guard::project_root(&projects, project)?;
            let storage_project = if root_guard::ensure_within_root(&root, &canonical).is_ok() {
                project.clone()
            } else {
                source_project
            };
            if !written.insert((storage_project.clone(), file.clone())) {
                continue;
            }
            let existing =
                taide_file::service::list_mirrors(&services.state.paths, &storage_project)?
                    .into_iter()
                    .find(|mirror| mirror.path == *file);
            let content = if let Some(document) = documents
                .iter()
                .find(|document| document.key == DocumentKey::File(canonical.clone()))
            {
                document.rope.to_string()
            } else if let Some(mirror) = existing {
                mirror.content
            } else {
                taide_file::service::open_file(&canonical, &[], false)?.content
            };
            taide_file::service::mirror_dirty(
                &services.state.paths,
                &storage_project,
                &canonical,
                file,
                &content,
            )?;
            let mut mirror =
                taide_file::service::list_mirrors(&services.state.paths, &storage_project)?
                    .into_iter()
                    .find(|mirror| mirror.path == *file)
                    .ok_or_else(|| {
                        AppError::Internal("surviving source draft is unavailable".into())
                    })?;
            mirror.source_missing = true;
            mirror.disk_modified_ms = None;
            mirror.conflict = false;
            drafts.insert(
                file.clone(),
                MissingDraft {
                    canonical,
                    project: storage_project,
                    mirror,
                },
            );
        }
    }
    Ok(drafts)
}

fn editor_error(error: taide_native_editor::document::EditorError) -> AppError {
    AppError::Forbidden(format!("missing source document rejected: {error:?}"))
}

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use taide_infra::root_guard;
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::layout::{ProjectLayout, TabPathChange};
use taide_native_editor::document::{DocumentId, DocumentKey, DocumentMetadata, DocumentSnapshot};
use taide_native_editor::store::EditorStore;
use taide_runtime::{AppServices, layout_actions};
use tokio::sync::{mpsc, oneshot};

use crate::lsp::Reply;
use crate::workspace_activity::{Activity, ActivityOwner, scoped_entry};

pub struct SourceDocument {
    pub snapshot: DocumentSnapshot,
    pub display_paths: Vec<PathBuf>,
}

pub struct Prepare {
    pub from: PathBuf,
    pub to: PathBuf,
    pub activity: Activity,
    pub completion: oneshot::Sender<AppResult<Vec<SourceDocument>>>,
}

pub struct DocumentMove {
    pub requested: DocumentSnapshot,
    pub canonical: PathBuf,
    pub metadata: DocumentMetadata,
}

pub struct Renamed {
    pub from: PathBuf,
    pub to: PathBuf,
    pub documents: Vec<DocumentMove>,
    pub layouts: HashMap<ProjectId, ProjectLayout>,
    pub survivor_drafts: HashMap<String, crate::missing_draft::MissingDraft>,
    pub warnings: Vec<AppError>,
    pub completion: oneshot::Sender<AppResult<Vec<DocumentSnapshot>>>,
}

pub fn moved_path(path: &Path, from: &Path, to: &Path) -> Option<PathBuf> {
    path.strip_prefix(from)
        .ok()
        .map(|relative| to.join(relative))
}

pub fn prepare_documents(
    store: &EditorStore,
    from: &Path,
    display_paths: &HashMap<String, DocumentId>,
) -> AppResult<Vec<SourceDocument>> {
    let documents = store
        .documents()
        .snapshots()
        .filter_map(|snapshot| {
            let paths = display_paths
                .iter()
                .filter(|(path, document)| {
                    **document == snapshot.id && Path::new(path).starts_with(from)
                })
                .map(|(path, _)| PathBuf::from(path))
                .collect::<Vec<_>>();
            let canonical_moves =
                matches!(&snapshot.key, DocumentKey::File(path) if path.starts_with(from));
            (canonical_moves || !paths.is_empty()).then_some(SourceDocument {
                snapshot,
                display_paths: paths,
            })
        })
        .collect::<Vec<_>>();
    if documents
        .iter()
        .any(|document| document.snapshot.revision.checked_add(1).is_none())
    {
        return Err(AppError::Forbidden(
            "renamed document revision is exhausted".into(),
        ));
    }
    Ok(documents)
}

pub fn commit_documents(
    store: &mut EditorStore,
    event: &Renamed,
) -> AppResult<Vec<DocumentSnapshot>> {
    for document in &event.documents {
        let current = store
            .documents()
            .snapshot(document.requested.id)
            .map_err(editor_error)?;
        if current.key != document.requested.key || current.revision != document.requested.revision
        {
            return Err(AppError::Forbidden(
                "renamed document changed before retarget".into(),
            ));
        }
        if current.revision.checked_add(1).is_none() {
            return Err(AppError::Forbidden(
                "renamed document revision is exhausted".into(),
            ));
        }
    }
    let mut snapshots = Vec::new();
    let missing_tabs = event
        .layouts
        .values()
        .flat_map(taide_layout::service::all_roots)
        .flat_map(crate::tabs::tabs_in)
        .filter(|tab| matches!(&tab.kind, taide_model::layout::TabKind::File { path } if event.survivor_drafts.contains_key(path)))
        .map(|tab| tab.id.clone())
        .collect::<HashSet<_>>();
    let missing_views = store
        .documents()
        .snapshots()
        .flat_map(|document| store.views().for_document(document.id))
        .filter(|view| missing_tabs.contains(&view.key.tab))
        .map(|view| view.id)
        .collect::<Vec<_>>();
    for view in missing_views {
        store.detach_view(view).map_err(editor_error)?;
    }
    for document in &event.documents {
        store
            .retarget_file(
                &document.requested,
                document.canonical.clone(),
                document.metadata.clone(),
            )
            .map_err(editor_error)?;
        snapshots.push(
            store
                .documents()
                .snapshot(document.requested.id)
                .map_err(editor_error)?,
        );
    }
    Ok(snapshots)
}

fn editor_error(error: taide_native_editor::document::EditorError) -> AppError {
    AppError::Forbidden(format!("workspace rename document rejected: {error:?}"))
}

struct MirrorMove {
    project: ProjectId,
    old_storage: PathBuf,
    to: PathBuf,
    draft: Option<String>,
}

pub async fn apply(
    services: &Arc<AppServices>,
    rename: taide_lsp::native::protocol::lsp_types::RenameFile,
    roots: Option<Vec<String>>,
    replies: &mpsc::Sender<Reply>,
    repaint: &Arc<dyn Fn() + Send + Sync>,
) -> AppResult<Vec<DocumentSnapshot>> {
    let from = crate::lsp_workspace_worker::uri_path(&rename.old_uri)?;
    let to = crate::lsp_workspace_worker::uri_path(&rename.new_uri)?;
    let overwrite = rename
        .options
        .as_ref()
        .is_some_and(|options| options.overwrite == Some(true));
    let ignore_exists = rename
        .options
        .is_some_and(|options| options.ignore_if_exists == Some(true));
    let operation = services
        .tasks
        .begin_operation("native-workspace-rename")
        .ok_or_else(|| AppError::Forbidden("workspace file operations are stopping".into()))?;
    let guard = services.state.begin_owned_mutation().await;
    let check_services = services.clone();
    let check_roots = roots.clone();
    let paths = services
        .tasks
        .run_blocking_result("native-workspace-rename-path", move || {
            let from = scoped_entry(&check_services, &from, check_roots.as_deref())?;
            let to = scoped_entry(&check_services, &to, check_roots.as_deref())?;
            if !overwrite && to.exists() {
                if ignore_exists {
                    return Ok(None);
                }
                return Err(AppError::InvalidArgument(
                    "workspace rename destination exists".into(),
                ));
            }
            let projects = check_services.state.projects.read();
            for project in projects.values() {
                let root = root_guard::project_root(&projects, &project.id)?;
                if from.starts_with(&root) && !to.starts_with(&root) {
                    return Err(AppError::Forbidden(
                        "moving files between project roots is not supported".into(),
                    ));
                }
            }
            Ok(Some((from, to)))
        })
        .await?;
    let Some((from, to)) = paths else {
        return Ok(Vec::new());
    };
    if from == to {
        return Ok(Vec::new());
    }
    let owner = ActivityOwner::new(repaint.clone());
    let (completion, receive) = oneshot::channel();
    replies
        .send(Reply::RenamePrepare(Prepare {
            from: from.clone(),
            to: to.clone(),
            activity: owner.activity.clone(),
            completion,
        }))
        .await
        .map_err(|_| AppError::Internal("workspace editor is unavailable".into()))?;
    repaint();
    let sources = receive
        .await
        .map_err(|_| AppError::Internal("workspace editor stopped".into()))??;
    let work_services = services.clone();
    let requested_from = from.clone();
    let requested_to = to.clone();
    let (documents, layouts, warnings, guard, operation, owner) = services
        .tasks
        .run_blocking_result("native-workspace-rename-file", move || {
            let from = scoped_entry(&work_services, &requested_from, roots.as_deref())?;
            let to = scoped_entry(&work_services, &requested_to, roots.as_deref())?;
            if from != requested_from || to != requested_to {
                return Err(AppError::Forbidden(
                    "workspace rename identity changed".into(),
                ));
            }
            let projects = work_services.state.projects.read().clone();
            let mut matching = Vec::new();
            let mut layouts = work_services.state.layouts.read().clone();
            let change = TabPathChange::Renamed {
                from: from
                    .to_str()
                    .ok_or_else(|| {
                        AppError::InvalidArgument("workspace path must be UTF-8".into())
                    })?
                    .into(),
                to: to
                    .to_str()
                    .ok_or_else(|| {
                        AppError::InvalidArgument("workspace path must be UTF-8".into())
                    })?
                    .into(),
            };
            let mut mirrors = Vec::new();
            let mut mirror_paths = HashSet::new();
            for project in projects.values() {
                let root = root_guard::project_root(&projects, &project.id)?;
                if !from.starts_with(&root) {
                    continue;
                }
                if !to.starts_with(&root) {
                    return Err(AppError::Forbidden(
                        "moving files between project roots is not supported".into(),
                    ));
                }
                let existing =
                    taide_file::service::list_mirrors(&work_services.state.paths, &project.id)?;
                let layout = taide_layout::service::get_layout_mut(&mut layouts, &project.id)?;
                let outcome = taide_layout::service::apply_tab_path_change(layout, &change);
                for moved in outcome.moved {
                    let old = Path::new(&moved.from);
                    let old_storage = root_guard::canonicalize_lenient(old)?;
                    let cached = existing.iter().find(|mirror| mirror.path == moved.from);
                    let source = sources.iter().find(|source| {
                        source.snapshot.key == DocumentKey::File(old_storage.clone())
                            || source.display_paths.iter().any(|path| path == old)
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
                    if mirror_paths.insert((project.id.clone(), old_storage.clone())) {
                        mirrors.push(MirrorMove {
                            project: project.id.clone(),
                            old_storage,
                            to: moved.to.into(),
                            draft,
                        });
                    }
                }
                matching.push(project.id.clone());
            }
            taide_file::service::rename_entry(&from, &to)?;
            work_services.state.self_writes.mark(&from);
            work_services.state.self_writes.mark(&to);
            for project in &matching {
                let layout = taide_layout::service::get_layout_mut(&mut layouts, project)?;
                layout_actions::finish_mutation(
                    work_services.events.as_ref(),
                    &work_services.state,
                    project,
                    layout,
                );
            }
            *work_services.state.layouts.write() = layouts.clone();
            for project in matching {
                work_services.events.publish(AppEvent::FsRescanRequired {
                    project_id: project,
                });
            }
            let mut warnings = Vec::new();
            for mirror in mirrors {
                let result = (|| {
                    let (_, canonical) = root_guard::resolve_owning_project(&projects, &mirror.to)?;
                    if let Some(draft) = mirror.draft {
                        taide_file::service::mirror_dirty(
                            &work_services.state.paths,
                            &mirror.project,
                            &canonical,
                            mirror.to.to_str().ok_or_else(|| {
                                AppError::InvalidArgument("workspace path must be UTF-8".into())
                            })?,
                            &draft,
                        )?;
                        if canonical == mirror.old_storage {
                            return Ok(());
                        }
                    }
                    taide_file::service::clear_mirror(
                        &work_services.state.paths,
                        &mirror.project,
                        &mirror.old_storage,
                    )
                })();
                if let Err(error) = result {
                    warnings.push(error);
                }
            }
            let overlays =
                taide_plugin::service::language_overlays(&taide_plugin::service::ensure_loaded(
                    &work_services.plugin,
                    &work_services.state.paths.plugins_dir(),
                ));
            let config = work_services.state.settings.read().editor_config_enabled;
            let mut documents = Vec::new();
            for source in sources {
                let DocumentKey::File(old_canonical) = &source.snapshot.key else {
                    continue;
                };
                let next = moved_path(old_canonical, &from, &to).or_else(|| {
                    source
                        .display_paths
                        .first()
                        .and_then(|path| moved_path(path, &from, &to))
                });
                let Some(next) = next else {
                    continue;
                };
                let canonical = match root_guard::resolve_owning_project(&projects, &next) {
                    Ok((_, canonical)) => canonical,
                    Err(error) => {
                        warnings.push(error);
                        moved_path(old_canonical, &from, &to)
                            .unwrap_or_else(|| old_canonical.clone())
                    }
                };
                let metadata = match taide_file::service::open_file(&canonical, &overlays, config) {
                    Ok(file) => DocumentMetadata::from_opened(&file),
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
            Ok((documents, layouts, warnings, guard, operation, owner))
        })
        .await?;
    let warning = warnings.first().map(ToString::to_string);
    let (completion, receive) = oneshot::channel();
    replies
        .send(Reply::Renamed(Renamed {
            from,
            to,
            documents,
            layouts,
            survivor_drafts: HashMap::new(),
            warnings,
            completion,
        }))
        .await
        .map_err(|_| AppError::Internal("workspace editor is unavailable after rename".into()))?;
    repaint();
    let result = receive
        .await
        .map_err(|_| AppError::Internal("workspace editor stopped after rename".into()))?;
    drop(guard);
    drop(operation);
    drop(owner);
    let snapshots = result?;
    if let Some(error) = warning {
        return Err(AppError::Internal(error));
    }
    Ok(snapshots)
}

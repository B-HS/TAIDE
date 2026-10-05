use std::path::{Path, PathBuf};
use std::sync::Arc;

use taide_infra::root_guard;
use taide_lsp::native::MAX_MIRROR_BYTES;
use taide_lsp::native::protocol::lsp_types::{
    self, DocumentChangeOperation, DocumentChanges, OneOf, ResourceOp,
};
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_native_editor::document::{DocumentKey, DocumentSnapshot};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_runtime::{AppServices, file_actions};
use tokio::sync::{mpsc, oneshot};

use crate::lsp::{ProtocolDocument, Reply, WorkspaceEditEvent};

pub fn uri_path(uri: &lsp_types::Uri) -> AppResult<PathBuf> {
    if uri
        .scheme()
        .is_none_or(|scheme| !scheme.eq_lowercase("file"))
        || uri.authority().is_some_and(|authority| {
            !authority.as_str().is_empty() && !authority.as_str().eq_ignore_ascii_case("localhost")
        })
        || uri.query().is_some()
        || uri.fragment().is_some()
    {
        return Err(AppError::InvalidArgument(
            "workspace edit requires a local file URI".into(),
        ));
    }
    let path = uri
        .path()
        .as_estr()
        .decode()
        .into_string()
        .map_err(|_| AppError::InvalidArgument("workspace file URI must be UTF-8".into()))?;
    let path = PathBuf::from(path.as_ref());
    if !path.is_absolute() || path.as_os_str().as_encoded_bytes().contains(&0) {
        return Err(AppError::InvalidArgument(
            "workspace file URI path is invalid".into(),
        ));
    }
    Ok(path)
}

async fn approved_path(
    services: &Arc<AppServices>,
    path: PathBuf,
    roots: Option<Vec<String>>,
) -> AppResult<PathBuf> {
    let state = services.state.clone();
    services
        .tasks
        .run_blocking_result("native-workspace-path", move || {
            if state.is_shutting_down() {
                return Err(AppError::Forbidden("workspace edits are stopping".into()));
            }
            let (_, canonical) = root_guard::resolve_owning_project_or_cli_opened(
                &state.projects.read(),
                &state.cli_opened_paths.read(),
                &path,
            )?;
            if let Some(roots) = roots {
                let allowed = roots.iter().any(|root| {
                    root_guard::canonicalize_lenient(Path::new(root))
                        .is_ok_and(|root| canonical.starts_with(root))
                });
                if !allowed {
                    return Err(AppError::Forbidden(
                        "workspace edit is outside its session roots".into(),
                    ));
                }
            }
            Ok(canonical)
        })
        .await
}

async fn open_document(
    path: PathBuf,
    replies: &mpsc::Sender<Reply>,
    repaint: &Arc<dyn Fn() + Send + Sync>,
) -> AppResult<Option<DocumentSnapshot>> {
    let (completion, receive) = oneshot::channel();
    replies
        .send(Reply::DocumentQuery { path, completion })
        .await
        .map_err(|_| AppError::Internal("workspace editor is unavailable".into()))?;
    repaint();
    receive
        .await
        .map_err(|_| AppError::Internal("workspace editor stopped".into()))
}

fn text_edits(edit: lsp_types::TextDocumentEdit) -> Vec<lsp_types::TextEdit> {
    edit.edits
        .into_iter()
        .map(|edit| match edit {
            OneOf::Left(edit) => edit,
            OneOf::Right(edit) => edit.text_edit,
        })
        .collect()
}

async fn edit_unopened(
    services: &Arc<AppServices>,
    path: PathBuf,
    edit: lsp_types::TextDocumentEdit,
) -> AppResult<()> {
    let path_string = path
        .to_str()
        .ok_or_else(|| AppError::InvalidArgument("workspace path must be UTF-8".into()))?
        .to_owned();
    let operation = services
        .tasks
        .begin_operation("native-workspace-unopened")
        .ok_or_else(|| AppError::Forbidden("workspace edits are stopping".into()))?;
    let guard = services.state.begin_owned_mutation().await;
    let state = services.state.clone();
    services
        .tasks
        .run_blocking_result("native-workspace-unopened", move || {
            let _operation = operation;
            let _guard = guard;
            if state.is_shutting_down() {
                return Err(AppError::Forbidden("workspace edits are stopping".into()));
            }
            let projects = state.projects.read().clone();
            let (_, canonical) = root_guard::resolve_owning_project_or_cli_opened(
                &projects,
                &state.cli_opened_paths.read(),
                &path,
            )?;
            if canonical != path {
                return Err(AppError::Forbidden(
                    "workspace document identity changed".into(),
                ));
            }
            for project in projects.values() {
                if taide_file::service::has_mirror(&state.paths, &project.id, &path)? {
                    return Err(AppError::Forbidden(
                        "open the restorable workspace draft before editing it".into(),
                    ));
                }
            }
            let file = taide_file::service::open_file(
                &path,
                &[],
                state.settings.read().editor_config_enabled,
            )?;
            let mut store = EditorStore::new(EditorLimits {
                max_documents: 1,
                max_views: 1,
                max_undo_groups: 1,
                max_document_bytes: MAX_MIRROR_BYTES,
            })
            .map_err(editor_error)?;
            let document = store.open_file(path, file).map_err(editor_error)?;
            let requested = store.documents().snapshot(document).map_err(editor_error)?;
            taide_native_editor::lsp::apply_text_edits(
                &mut store,
                &requested,
                None,
                text_edits(edit),
            )
            .map_err(editor_error)?;
            let content = store
                .documents()
                .snapshot(document)
                .map_err(editor_error)?
                .rope
                .to_string();
            file_actions::save_file_within_open_projects(&state, Path::new(&path_string), &content)
        })
        .await
}

fn editor_error(error: taide_native_editor::document::EditorError) -> AppError {
    AppError::Forbidden(format!("workspace document edit rejected: {error:?}"))
}

async fn create_file(
    services: &Arc<AppServices>,
    create: lsp_types::CreateFile,
    roots: Option<Vec<String>>,
) -> AppResult<()> {
    let path = approved_path(services, uri_path(&create.uri)?, roots).await?;
    let overwrite = create
        .options
        .as_ref()
        .is_some_and(|options| options.overwrite == Some(true));
    let ignore_if_exists = create
        .options
        .is_some_and(|options| options.ignore_if_exists == Some(true));
    let operation = services
        .tasks
        .begin_operation("native-workspace-create")
        .ok_or_else(|| AppError::Forbidden("workspace edits are stopping".into()))?;
    let guard = services.state.begin_owned_mutation().await;
    let state = services.state.clone();
    services
        .tasks
        .run_blocking_result("native-workspace-create", move || {
            let _operation = operation;
            let _guard = guard;
            if state.is_shutting_down() {
                return Err(AppError::Forbidden("workspace edits are stopping".into()));
            }
            let projects = state.projects.read().clone();
            if overwrite {
                let (_, canonical) = root_guard::resolve_owning_project_or_cli_opened(
                    &projects,
                    &state.cli_opened_paths.read(),
                    &path,
                )?;
                if canonical != path {
                    return Err(AppError::Forbidden(
                        "workspace file identity changed".into(),
                    ));
                }
                return file_actions::save_file_within_open_projects(&state, &canonical, "");
            }
            let (_, canonical) = root_guard::resolve_owning_project(&projects, &path)?;
            if canonical != path {
                return Err(AppError::Forbidden(
                    "workspace file identity changed".into(),
                ));
            }
            match taide_file::service::create_entry(&canonical, false) {
                Ok(()) => {
                    state.self_writes.mark(&canonical);
                    Ok(())
                }
                Err(error) if ignore_if_exists && error.kind() == AppErrorKind::InvalidArgument => {
                    Ok(())
                }
                Err(error) => Err(error),
            }
        })
        .await
}

pub async fn apply(
    services: &Arc<AppServices>,
    edit: lsp_types::WorkspaceEdit,
    mut documents: Vec<ProtocolDocument>,
    roots: Option<Vec<String>>,
    replies: &mpsc::Sender<Reply>,
    repaint: &Arc<dyn Fn() + Send + Sync>,
) -> AppResult<()> {
    let operations = match edit.document_changes {
        Some(DocumentChanges::Edits(edits)) => edits
            .into_iter()
            .map(DocumentChangeOperation::Edit)
            .collect::<Vec<_>>(),
        Some(DocumentChanges::Operations(operations)) => operations,
        None => edit
            .changes
            .unwrap_or_default()
            .into_iter()
            .map(|(uri, edits)| {
                DocumentChangeOperation::Edit(lsp_types::TextDocumentEdit {
                    text_document: lsp_types::OptionalVersionedTextDocumentIdentifier {
                        uri,
                        version: None,
                    },
                    edits: edits.into_iter().map(OneOf::Left).collect(),
                })
            })
            .collect(),
    };
    for operation in operations {
        let mut edit = match operation {
            DocumentChangeOperation::Edit(edit) => edit,
            DocumentChangeOperation::Op(ResourceOp::Create(create)) => {
                create_file(services, create, roots.clone()).await?;
                continue;
            }
            DocumentChangeOperation::Op(ResourceOp::Delete(delete)) => {
                let released = crate::workspace_delete::apply(
                    services,
                    delete,
                    roots.clone(),
                    replies,
                    repaint,
                )
                .await?;
                documents.retain(|document| !released.contains(&document.snapshot.id));
                continue;
            }
            DocumentChangeOperation::Op(ResourceOp::Rename(rename)) => {
                let moved = crate::workspace_rename::apply(
                    services,
                    rename,
                    roots.clone(),
                    replies,
                    repaint,
                )
                .await?;
                for snapshot in moved {
                    let DocumentKey::File(path) = &snapshot.key else {
                        continue;
                    };
                    documents.retain(|document| {
                        document.snapshot.id != snapshot.id && document.snapshot.key != snapshot.key
                    });
                    documents.push(ProtocolDocument {
                        uri: taide_lsp::service::workspace_folder_uri(
                            path.to_str().expect("URI path is UTF-8"),
                        ),
                        snapshot,
                        revision: None,
                    });
                }
                continue;
            }
        };
        let path =
            approved_path(services, uri_path(&edit.text_document.uri)?, roots.clone()).await?;
        if edit.edits.is_empty() {
            continue;
        }
        let known = documents
            .iter()
            .position(|document| document.snapshot.key == DocumentKey::File(path.clone()));
        let document = match known {
            Some(index) => Some(documents[index].clone()),
            None => open_document(path.clone(), replies, repaint)
                .await?
                .map(|snapshot| ProtocolDocument {
                    snapshot,
                    uri: taide_lsp::service::workspace_folder_uri(
                        path.to_str().expect("URI path is UTF-8"),
                    ),
                    revision: None,
                }),
        };
        let Some(document) = document else {
            edit_unopened(services, path, edit).await?;
            continue;
        };
        edit.text_document.uri = document
            .uri
            .parse()
            .map_err(|_| AppError::InvalidArgument("workspace document URI is invalid".into()))?;
        let (completion, receive) = oneshot::channel();
        replies
            .send(Reply::WorkspaceEdit(WorkspaceEditEvent {
                edit: lsp_types::WorkspaceEdit {
                    document_changes: Some(DocumentChanges::Edits(vec![edit])),
                    ..Default::default()
                },
                documents: vec![document.clone()],
                completion,
            }))
            .await
            .map_err(|_| AppError::Internal("workspace editor is unavailable".into()))?;
        repaint();
        if !receive.await.unwrap_or(false) {
            return Err(AppError::Forbidden(
                "workspace editor rejected an edit".into(),
            ));
        }
        if let Some(current) = open_document(path, replies, repaint).await? {
            if let Some(index) = known {
                documents[index].snapshot = current;
            } else {
                documents.push(ProtocolDocument {
                    snapshot: current,
                    ..document
                });
            }
        }
    }
    Ok(())
}

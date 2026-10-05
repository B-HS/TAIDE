use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use taide_infra::root_guard;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::tree::{TreeEntryKind, TreeRowPage};
use taide_runtime::{AppServices, tree_actions};
use tokio::sync::mpsc;

const DESTINATION_ATTEMPT_LIMIT: usize = 8;
const DESTINATION_EXISTS_KEY: &str = "error.file.destinationExists";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Cut,
    Copy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub mode: Mode,
    pub path: String,
    pub kind: TreeEntryKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub token: u64,
    pub owner: Option<crate::explorer_clipboard_owners::Owner>,
    pub entry: Entry,
    pub target: String,
    pub sibling_names: BTreeSet<String>,
    pub conflict_suffix: String,
}

pub struct Pasted {
    pub path: String,
    pub page: TreeRowPage,
}

pub struct Reply {
    pub project: ProjectId,
    pub request: Request,
    pub clear_cut: bool,
    pub result: AppResult<Pasted>,
}

pub fn parent_dir(path: &str) -> &str {
    path.rfind('/')
        .filter(|index| *index > 0)
        .map_or("/", |index| &path[..index])
}

pub fn unique_name(
    desired: &str,
    taken: &BTreeSet<String>,
    suffix: &str,
    kind: TreeEntryKind,
) -> String {
    if !taken.contains(desired) {
        return desired.into();
    }
    let (base, extension) = match desired.rfind('.') {
        Some(index) if index > 0 && kind == TreeEntryKind::File => {
            (&desired[..index], &desired[index..])
        }
        _ => (desired, ""),
    };
    let mut attempt = 1;
    let mut candidate = format!("{base} {suffix}{extension}");
    while taken.contains(&candidate) {
        attempt += 1;
        candidate = format!("{base} {suffix} {attempt}{extension}");
    }
    candidate
}

pub async fn apply(
    services: &Arc<AppServices>,
    project: ProjectId,
    request: Request,
    replies: &mpsc::Sender<crate::lsp::Reply>,
    repaint: &Arc<dyn Fn() + Send + Sync>,
) -> Reply {
    let mut clear_cut = false;
    let result = paste(
        services,
        &project,
        &request,
        replies,
        repaint,
        &mut clear_cut,
    )
    .await;
    Reply {
        project,
        request,
        clear_cut,
        result,
    }
}

async fn paste(
    services: &Arc<AppServices>,
    project: &ProjectId,
    request: &Request,
    replies: &mpsc::Sender<crate::lsp::Reply>,
    repaint: &Arc<dyn Fn() + Send + Sync>,
    clear_cut: &mut bool,
) -> AppResult<Pasted> {
    let _operation = services
        .tasks
        .begin_operation("native-explorer-paste")
        .ok_or_else(|| AppError::Forbidden("native explorer paste is stopping".into()))?;
    let desired = request.entry.path.rsplit('/').next().unwrap_or_default();
    if desired.is_empty() || matches!(desired, "." | "..") {
        return Err(AppError::InvalidArgument(
            "native clipboard entry name is invalid".into(),
        ));
    }
    let mut taken = request.sibling_names.clone();
    let mut destination = None;
    for attempt in 0..DESTINATION_ATTEMPT_LIMIT {
        let name = unique_name(
            desired,
            &taken,
            &request.conflict_suffix,
            request.entry.kind,
        );
        let path = format!(
            "{}/{name}",
            request.target.strip_suffix('/').unwrap_or(&request.target)
        );
        let result = match request.entry.mode {
            Mode::Copy => copy_entry(services, project, &request.entry.path, &path).await,
            Mode::Cut => {
                let rename = crate::explorer::RenameRequest {
                    token: request.token,
                    from: request.entry.path.clone(),
                    to: path.clone(),
                };
                if crate::explorer_move::try_cross_project_move(
                    services, project, &rename, replies, repaint,
                )
                .await?
                {
                    Ok(())
                } else {
                    crate::explorer_move::move_selected(
                        services, project, &rename, replies, repaint,
                    )
                    .await
                }
            }
        };
        match result {
            Ok(()) => {
                destination = Some(path);
                break;
            }
            Err(error) => {
                if attempt + 1 == DESTINATION_ATTEMPT_LIMIT
                    || !matches!(&error, AppError::Localized(value) if value.key == DESTINATION_EXISTS_KEY)
                {
                    return Err(error);
                }
                taken.insert(name);
            }
        }
    }
    let path =
        destination.ok_or_else(|| AppError::Internal("native paste has no destination".into()))?;
    if request.entry.mode == Mode::Cut {
        tree_actions::tree_refresh(
            &services.state,
            &services.tree,
            &services.tasks,
            project.clone(),
            parent_dir(&request.entry.path).into(),
        )
        .await?;
        *clear_cut = true;
    }
    tree_actions::tree_refresh(
        &services.state,
        &services.tree,
        &services.tasks,
        project.clone(),
        request.target.clone(),
    )
    .await?;
    let page = tree_actions::tree_reveal(
        &services.state,
        &services.tree,
        &services.tasks,
        project.clone(),
        path.clone(),
    )
    .await?;
    Ok(Pasted { path, page })
}

async fn copy_entry(
    services: &Arc<AppServices>,
    project: &ProjectId,
    from: &str,
    to: &str,
) -> AppResult<()> {
    let guard = services.state.begin_owned_mutation().await;
    let state = services.state.clone();
    let project = project.clone();
    let from = from.to_owned();
    let to = to.to_owned();
    services
        .tasks
        .run_blocking_result("native-explorer-copy", move || {
            let _guard = guard;
            if state.is_shutting_down() {
                return Err(AppError::Forbidden(
                    "native explorer copy is stopping".into(),
                ));
            }
            if !Path::new(&from).is_absolute() || !Path::new(&to).is_absolute() {
                return Err(AppError::InvalidArgument(
                    "native clipboard path must be absolute".into(),
                ));
            }
            let projects = state.projects.read();
            let root = root_guard::project_root(&projects, &project)?;
            let (_, from) = root_guard::resolve_owning_project(&projects, Path::new(&from))?;
            let to = root_guard::ensure_within_root(&root, Path::new(&to))?;
            taide_file::service::copy_entry(&from, &to)?;
            state.self_writes.mark(&to);
            Ok(())
        })
        .await
}

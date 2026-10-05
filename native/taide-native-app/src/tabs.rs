use std::path::{Path, PathBuf};
use std::sync::Arc;

use taide_model::error::{AppError, AppResult};
use taide_model::ide::IdeDiffOutcome;
use taide_model::ids::{ProjectId, TabId};
use taide_model::layout::{PaneNode, Tab, TabKind};
use taide_runtime::{AppServices, layout_actions};

pub struct ClosedNativeTab {
    pub project: ProjectId,
    pub tab: Tab,
    pub has_remaining_file: bool,
    pub discarded: bool,
}

pub fn tabs_in(node: &PaneNode) -> Vec<&Tab> {
    match node {
        PaneNode::Leaf { tabs, .. } => tabs.iter().collect(),
        PaneNode::Split { children, .. } => children.iter().flat_map(tabs_in).collect(),
    }
}

pub async fn close(
    services: &Arc<AppServices>,
    tab: TabId,
    discard: bool,
) -> AppResult<ClosedNativeTab> {
    let operation = services
        .tasks
        .begin_operation("native-tab-close")
        .ok_or_else(|| AppError::Forbidden("native tab close is stopping".into()))?;
    let guard = services.state.begin_owned_mutation().await;
    let services = services.clone();
    let tasks = services.tasks.clone();
    tasks
        .run_blocking_result("native-tab-close", move || {
            let _operation = operation;
            let _guard = guard;
            if services.state.is_shutting_down() {
                return Err(AppError::Forbidden("native tab close is stopping".into()));
            }
            let mut layouts = services.state.layouts.read().clone();
            let project = taide_layout::service::locate_project_with_tab(&layouts, &tab)?;
            let layout = taide_layout::service::get_layout_mut(&mut layouts, &project)?;
            let candidate = tabs_in(&layout.root)
                .into_iter()
                .chain(
                    layout
                        .auxiliary_windows
                        .iter()
                        .flat_map(|window| tabs_in(&window.root)),
                )
                .find(|candidate| candidate.id == tab)
                .ok_or_else(|| AppError::NotFound("native tab no longer exists".into()))?;
            if candidate.pinned {
                return Err(AppError::Forbidden("pinned tab cannot close".into()));
            }
            if candidate.dirty
                && matches!(
                    candidate.kind,
                    TabKind::File { .. } | TabKind::Untitled { .. }
                )
                && !discard
            {
                return Err(AppError::Forbidden(
                    "native dirty tab requires a completed save or explicit discard".into(),
                ));
            }
            if discard {
                taide_layout::service::set_dirty(layout, &tab, false)?;
            }
            let closed = taide_layout::service::close_tab(layout, &tab)?;
            let mut remaining = false;
            match &closed.tab.kind {
                TabKind::File { path } => {
                    let projects = services.state.projects.read().clone();
                    let approved = services.state.cli_opened_paths.read();
                    let (owner, canonical) =
                        taide_infra::root_guard::resolve_owning_project_or_cli_opened(
                            &projects,
                            &approved,
                            Path::new(path),
                        )?;
                    for layout in layouts.values() {
                        for candidate in tabs_in(&layout.root).into_iter().chain(
                            layout
                                .auxiliary_windows
                                .iter()
                                .flat_map(|window| tabs_in(&window.root)),
                        ) {
                            if let TabKind::File { path } = &candidate.kind {
                                let path =
                                    taide_infra::root_guard::canonicalize_lenient(Path::new(path));
                                if path.as_ref().is_ok_and(|path| path == &canonical) {
                                    remaining = true;
                                }
                            }
                        }
                    }
                    if !remaining && let Some(owner) = owner {
                        taide_file::service::clear_mirror(
                            &services.state.paths,
                            &owner,
                            &canonical,
                        )?;
                    }
                }
                TabKind::Untitled { .. } => {
                    taide_file::service::clear_untitled_mirror(
                        &services.state.paths,
                        &project,
                        &tab,
                    )?;
                }
                _ => {}
            }
            let layout = taide_layout::service::get_layout_mut(&mut layouts, &project)?;
            layout_actions::finish_mutation(
                services.events.as_ref(),
                &services.state,
                &project,
                layout,
            );
            *services.state.layouts.write() = layouts;
            match &closed.tab.kind {
                TabKind::Terminal { session_id, .. } => services.terminal.kill_session(session_id),
                TabKind::ClaudeDiff { request_id, .. } => {
                    if let Some(pending) = services.ide.take_pending_diff(request_id) {
                        drop(pending.responder.send((IdeDiffOutcome::TabClosed, None)));
                    }
                }
                _ => {}
            }
            Ok(ClosedNativeTab {
                project,
                tab: closed.tab,
                has_remaining_file: remaining,
                discarded: discard,
            })
        })
        .await
}

pub async fn save_mirrored(services: &Arc<AppServices>, tab: TabId) -> AppResult<()> {
    let candidate = {
        let layouts = services.state.layouts.read();
        layouts
            .values()
            .flat_map(|layout| {
                tabs_in(&layout.root).into_iter().chain(
                    layout
                        .auxiliary_windows
                        .iter()
                        .flat_map(|window| tabs_in(&window.root)),
                )
            })
            .find(|candidate| candidate.id == tab)
            .cloned()
            .ok_or_else(|| AppError::NotFound("native tab no longer exists".into()))?
    };
    let TabKind::File { path } = candidate.kind else {
        return Err(AppError::Forbidden(
            "native untitled Save As is not connected".into(),
        ));
    };
    let (owner, canonical) = taide_infra::root_guard::resolve_owning_project_or_cli_opened(
        &services.state.projects.read(),
        &services.state.cli_opened_paths.read(),
        Path::new(&path),
    )?;
    let Some(owner) = owner else { return Ok(()) };
    let mirrors = taide_runtime::file_actions::file_list_mirrors(&services.state, owner).await?;
    let mirror = mirrors.into_iter().find(|entry| {
        taide_infra::root_guard::canonicalize_lenient(Path::new(&entry.path))
            .is_ok_and(|path: PathBuf| path == canonical)
    });
    if let Some(mirror) = mirror {
        if mirror.source_missing || mirror.conflict {
            return Err(AppError::Forbidden(
                "resolve the native mirror conflict or Save As before closing".into(),
            ));
        }
        taide_runtime::file_actions::file_save(
            &services.state,
            &services.tasks,
            path,
            mirror.content,
        )
        .await?;
    }
    Ok(())
}

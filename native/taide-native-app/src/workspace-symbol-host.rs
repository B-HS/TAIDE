use taide_model::{
    error::{AppError, AppResult},
    ids::{PaneId, ProjectId},
    layout::{PaneNode, ProjectLayout, Tab, TabKind},
};
use taide_native_ui::shell::WindowScope;
use taide_runtime::{AppServices, layout_actions};

use crate::terminal_tabs::OpenedFileLink;

#[derive(Clone)]
pub struct Request {
    pub(crate) project: ProjectId,
    pub(crate) pane: PaneId,
    pub(crate) focused: PaneId,
    pub(crate) revision: u32,
    pub(crate) scope: WindowScope,
    pub(crate) path: String,
    pub(crate) line: u32,
    pub(crate) column: u32,
    pub(crate) viewport: eframe::egui::ViewportId,
}

pub(crate) fn destination(
    layout: &ProjectLayout,
    project: &ProjectId,
    scope: &WindowScope,
    path: &str,
) -> Option<PaneId> {
    let (root, focused) = crate::symbol_sidebar::window_tree(project, layout, scope)?;
    let holds_file = |leaf: &&PaneNode| matches!(leaf, PaneNode::Leaf { tabs, .. } if tabs.iter().any(|tab| matches!(&tab.kind, TabKind::File { path: candidate } if candidate == path)));
    let leaf = taide_layout::service::find_leaf(root, focused)
        .filter(holds_file)
        .or_else(|| {
            taide_layout::service::collect_leaves(root)
                .into_iter()
                .find(holds_file)
        });
    match leaf {
        Some(PaneNode::Leaf { id, .. }) => Some(id.clone()),
        _ => Some(focused.clone()),
    }
}

pub(crate) async fn open(services: &AppServices, request: Request) -> AppResult<OpenedFileLink> {
    let _guard = services.state.begin_mutation().await;
    let projects = services.state.projects.read().clone();
    if !projects.contains_key(&request.project) || request.line == 0 || request.column == 0 {
        return Err(AppError::NotFound(
            "native workspace symbol source is closed or invalid".into(),
        ));
    }
    if matches!(request.scope, WindowScope::Main)
        && !services
            .state
            .session
            .read()
            .shell_slots
            .as_ref()
            .is_some_and(|tree| crate::breadcrumbs::contains_project(tree, &request.project))
    {
        return Err(AppError::NotFound(
            "native workspace symbol slot is closed".into(),
        ));
    }
    let (_, resolved) = taide_infra::root_guard::resolve_owning_project_or_cli_opened(
        &projects,
        &services.state.cli_opened_paths.read(),
        std::path::Path::new(&request.path),
    )?;
    taide_infra::root_guard::ensure_existing_file(&resolved, &request.path)?;
    let mut layouts = services.state.layouts.read().clone();
    let layout = layouts
        .get_mut(&request.project)
        .filter(|layout| {
            layout.revision == request.revision
                && crate::symbol_sidebar::window_tree(&request.project, layout, &request.scope)
                    .is_some_and(|(root, focused)| {
                        *focused == request.focused
                            && taide_layout::service::find_leaf(root, &request.pane).is_some()
                    })
        })
        .ok_or_else(|| {
            AppError::NotFound("native workspace symbol pane is closed or changed".into())
        })?;
    let title = std::path::Path::new(&request.path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&request.path)
        .to_owned();
    let preview = services.state.settings.read().enable_preview_tabs;
    let tab = taide_layout::service::open_tab(
        layout,
        &request.pane,
        Tab {
            id: taide_model::ids::TabId::new(),
            kind: TabKind::File {
                path: request.path.clone(),
            },
            title,
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        },
        preview,
    )?;
    taide_layout::service::focus_pane(layout, &request.pane)?;
    let updated = layout_actions::finish_mutation(
        services.events.as_ref(),
        &services.state,
        &request.project,
        layout,
    );
    *services.state.layouts.write() = layouts;
    Ok(OpenedFileLink {
        project: request.project,
        pane: request.pane,
        tab,
        path: request.path,
        line: f64::from(request.line),
        column: f64::from(request.column),
        viewport: request.viewport,
        layout: updated,
    })
}

#[cfg(test)]
#[path = "workspace-symbol-host-tests.rs"]
mod tests;

use taide_model::{
    error::{AppError, AppResult},
    layout::{DropEdge, Tab, TabKind},
};
use taide_native_editor::symbol_locations::Target;
use taide_native_ui::shell::WindowScope;
use taide_runtime::{AppServices, layout_actions};

#[derive(Clone)]
pub struct Request {
    pub(crate) source: crate::editor_locations::Request,
    pub(crate) token: uuid::Uuid,
    pub(crate) cancelled: tokio::sync::watch::Receiver<bool>,
    pub(crate) target: Target,
    pub(crate) side: bool,
    pub(crate) keep_peek: bool,
    pub(crate) revision: u32,
    pub(crate) scope: WindowScope,
    pub(crate) viewport: eframe::egui::ViewportId,
}

impl Request {
    pub(crate) fn is_cancelled(&self) -> bool {
        self.source.is_cancelled()
            || *self.cancelled.borrow()
            || self.cancelled.has_changed().is_err()
    }
}

pub(crate) async fn open(
    services: &AppServices,
    request: &Request,
) -> AppResult<crate::terminal_tabs::OpenedFileLink> {
    let _guard = services.state.begin_mutation().await;
    if request.is_cancelled() || !request.target.is_valid() {
        return Err(AppError::NotFound("native location request expired".into()));
    }
    let path = crate::editor_locations::file_path(&request.target.uri)
        .ok_or_else(|| AppError::InvalidArgument("native location URI is invalid".into()))?;
    let path = path
        .to_str()
        .ok_or_else(|| AppError::InvalidArgument("native location path is invalid".into()))?
        .to_owned();
    let projects = services.state.projects.read().clone();
    if !projects.contains_key(&request.source.project)
        || matches!(request.scope, WindowScope::Main)
            && !services
                .state
                .session
                .read()
                .shell_slots
                .as_ref()
                .is_some_and(|tree| {
                    crate::breadcrumbs::contains_project(tree, &request.source.project)
                })
    {
        return Err(AppError::NotFound(
            "native location project is closed".into(),
        ));
    }
    let (_, resolved) = taide_infra::root_guard::resolve_owning_project_or_cli_opened(
        &projects,
        &services.state.cli_opened_paths.read(),
        std::path::Path::new(&path),
    )?;
    taide_infra::root_guard::ensure_existing_file(&resolved, &path)?;
    let mut layouts = services.state.layouts.read().clone();
    let layout = layouts
        .get_mut(&request.source.project)
        .filter(|layout| {
            layout.revision == request.revision
                && crate::symbol_sidebar::window_tree(
                    &request.source.project,
                    layout,
                    &request.scope,
                )
                .is_some_and(|(root, focused)| {
                    *focused == request.source.source_key.pane
                        && taide_native_ui::snapshot::active_tab(root, focused)
                            .is_some_and(|tab| tab.id == request.source.source_key.tab)
                })
        })
        .ok_or_else(|| AppError::NotFound("native location source pane changed".into()))?;
    let title = std::path::Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&path)
        .to_owned();
    let preview = services.state.settings.read().enable_preview_tabs;
    let tab = Tab {
        id: taide_model::ids::TabId::new(),
        kind: TabKind::File { path: path.clone() },
        title,
        pinned: false,
        preview,
        dirty: false,
        view_state: None,
    };
    let tab = if request.side {
        taide_layout::service::open_tab_in_split(
            layout,
            &request.source.source_key.pane,
            DropEdge::Right,
            tab,
        )?
    } else {
        taide_layout::service::open_tab(layout, &request.source.source_key.pane, tab, preview)?
    };
    let pane = taide_layout::service::all_roots(layout)
        .find_map(|root| {
            taide_layout::service::collect_leaves(root)
                .into_iter()
                .find_map(|leaf| match leaf {
                    taide_model::layout::PaneNode::Leaf { id, tabs, .. }
                        if tabs.iter().any(|entry| entry.id == tab) =>
                    {
                        Some(id.clone())
                    }
                    _ => None,
                })
        })
        .ok_or_else(|| AppError::Internal("native location destination disappeared".into()))?;
    taide_layout::service::focus_pane(layout, &pane)?;
    let updated = layout_actions::finish_mutation(
        services.events.as_ref(),
        &services.state,
        &request.source.project,
        layout,
    );
    *services.state.layouts.write() = layouts;
    Ok(crate::terminal_tabs::OpenedFileLink {
        project: request.source.project.clone(),
        pane,
        tab,
        path,
        line: f64::from(request.target.selection.start.line) + 1.0,
        column: f64::from(request.target.selection.start.character) + 1.0,
        viewport: request.viewport,
        layout: updated,
    })
}

#[cfg(test)]
#[path = "symbol-location-host-tests.rs"]
mod tests;

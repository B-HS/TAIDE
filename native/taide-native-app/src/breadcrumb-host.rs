use taide_model::{
    error::{AppError, AppResult},
    layout::{Tab, TabKind},
};
use taide_runtime::{AppServices, layout_actions};

use crate::{breadcrumbs::Source, terminal_tabs::OpenedFileLink};

pub(crate) async fn open(
    services: &AppServices,
    source: &Source,
    path: String,
    viewport: eframe::egui::ViewportId,
) -> AppResult<OpenedFileLink> {
    let _guard = services.state.begin_mutation().await;
    let projects = services.state.projects.read().clone();
    let (_, resolved) = taide_infra::root_guard::resolve_owning_project_or_cli_opened(
        &projects,
        &services.state.cli_opened_paths.read(),
        std::path::Path::new(&path),
    )?;
    taide_infra::root_guard::ensure_existing_file(&resolved, &path)?;
    let mut layouts = services.state.layouts.read().clone();
    let layout = layouts
        .get_mut(&source.project)
        .filter(|layout| source.is_active(layout))
        .ok_or_else(|| {
            AppError::NotFound("native breadcrumb source is closed or inactive".into())
        })?;
    let title = std::path::Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&path)
        .to_owned();
    let preview = services.state.settings.read().enable_preview_tabs;
    let tab = taide_layout::service::open_tab(
        layout,
        &source.pane,
        Tab {
            id: taide_model::ids::TabId::new(),
            kind: TabKind::File { path: path.clone() },
            title,
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        },
        preview,
    )?;
    taide_layout::service::focus_pane(layout, &source.pane)?;
    let updated = layout_actions::finish_mutation(
        services.events.as_ref(),
        &services.state,
        &source.project,
        layout,
    );
    *services.state.layouts.write() = layouts;
    Ok(OpenedFileLink {
        project: source.project.clone(),
        pane: source.pane.clone(),
        tab,
        path,
        line: 1.0,
        column: 1.0,
        viewport,
        layout: updated,
    })
}

#[cfg(test)]
#[path = "breadcrumb-host-tests.rs"]
mod tests;

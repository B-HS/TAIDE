use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use eframe::egui::{self, Ui};
use taide_model::error::{AppError, AppResult};
use taide_model::ids::{PaneId, ProjectId, ShellSlotId, TabId};
use taide_model::layout::{Tab, TabKind};
use taide_model::tree::TreeRowPage;
use taide_native_editor::view::{ViewId, ViewKey};
use taide_native_ui::commands::{ShellIntent, ShellMutation};
use taide_native_ui::intents::explorer_search::Intent;
use taide_native_ui::shell::WindowScope;
use taide_native_ui::surfaces::explorer_search::ExplorerSearchSurfaces;
use taide_runtime::tree_actions;

use crate::application::{AppSurfaces, NativeApplication, WINDOW_LABEL, editor_error};
use crate::host::{HostCommand, HostReply, HostWorker, stopped};
use crate::presentation;

pub enum Command {
    RestoreWatchers,
    RefreshTree {
        project: ProjectId,
        dirs: crate::events::TreeInvalidation,
    },
    OpenWorkspaceSymbol(crate::workspace_symbol_host::Request),
    RevealBreadcrumbTree(crate::breadcrumbs::Source),
    TreeRows {
        project: ProjectId,
        offset: u32,
    },
    TreeToggle {
        project: ProjectId,
        path: String,
    },
    TreeCollapse(ProjectId),
    RevealExplorerPath {
        project: ProjectId,
        mount: taide_model::ids::ShellSlotId,
        path: String,
        rename: bool,
    },
}

pub enum Reply {
    WorkspaceSymbolOpened {
        result: AppResult<crate::terminal_tabs::OpenedFileLink>,
    },
    BreadcrumbTree {
        source: crate::breadcrumbs::Source,
        result: AppResult<TreeRowPage>,
    },
    TreeSynced {
        project: ProjectId,
        result: AppResult<TreeRowPage>,
        errors: Vec<AppError>,
    },
    Tree {
        project: ProjectId,
        offset: u32,
        result: AppResult<TreeRowPage>,
    },
    TreeToggled {
        project: ProjectId,
        path: String,
        result: AppResult<TreeRowPage>,
    },
    ExplorerPathRevealed {
        project: ProjectId,
        mount: taide_model::ids::ShellSlotId,
        path: String,
        rename: bool,
        result: AppResult<TreeRowPage>,
    },
}

pub(crate) struct State {
    pub(crate) workspace_symbols: crate::workspace_symbols::State,
    pub(crate) explorer_appearance: crate::explorer::Appearance,
    pub(crate) explorer_icons: crate::problems_icons::Icons,
    pub(crate) navigation_appearance: crate::symbol_outline::Appearance,
    pub(crate) navigation_icons: crate::navigation_icons::Icons,
    pub(crate) symbol_sidebars: crate::symbol_sidebar::Views,
    pub(crate) breadcrumbs: crate::breadcrumbs::Views,
    pub(crate) trees: HashMap<ProjectId, TreeRowPage>,
    pub(crate) explorers: HashMap<ProjectId, crate::explorer::Explorer>,
    pub(crate) explorer_clipboards: crate::explorer_clipboard_owners::ClipboardOwners,
    pub(crate) pending_entry_delete: Option<crate::delete_dialog::Confirmation>,
    pub(crate) deleting_entry: bool,
    pub(crate) tree_loading: HashSet<ProjectId>,
    pub(crate) tree_changes: Arc<crate::events::TreeChanges>,
}

#[derive(Default)]
pub(crate) struct FrameOutput {
    pub(crate) rename_entries: Vec<(ProjectId, crate::explorer::RenameRequest)>,
    pub(crate) paste_entries: Vec<(ProjectId, crate::explorer_clipboard::Request)>,
    pub(crate) create_entries: Vec<(ProjectId, crate::explorer::CreateRequest)>,
    pub(crate) delete_entries: Vec<crate::explorer_delete::Request>,
    pub(crate) discarded_explorers: Vec<ProjectId>,
    pub(crate) load_trees: Vec<ProjectId>,
}

pub(crate) async fn dispatch(worker: &HostWorker<'_>, command: Command) -> Option<HostReply> {
    let services = worker.services;
    let sender = worker.sender;
    let repaint = worker.repaint;
    match command {
        Command::RefreshTree { project, dirs } => {
            let dirs = match dirs {
                Some(dirs) => dirs,
                None => {
                    let root = services
                        .state
                        .projects
                        .read()
                        .get(&project)
                        .map(|project| project.root.clone());
                    let mut dirs = std::collections::BTreeSet::new();
                    if let Some(root) = root {
                        dirs.insert(root);
                    }
                    if let Ok(page) = tree_actions::tree_rows(
                        &services.state,
                        &services.tree,
                        &services.tasks,
                        project.clone(),
                        0,
                        None,
                    )
                    .await
                    {
                        dirs.extend(
                            page.rows
                                .into_iter()
                                .filter(|row| row.expanded)
                                .map(|row| row.path),
                        );
                    }
                    dirs
                }
            };
            let mut errors = Vec::new();
            for dir in dirs {
                if let Err(error) = tree_actions::tree_refresh(
                    &services.state,
                    &services.tree,
                    &services.tasks,
                    project.clone(),
                    dir,
                )
                .await
                {
                    errors.push(error);
                }
            }
            let result = tree_actions::tree_rows(
                &services.state,
                &services.tree,
                &services.tasks,
                project.clone(),
                0,
                None,
            )
            .await;
            Some(HostReply::ExplorerSearch(Reply::TreeSynced {
                project,
                result,
                errors,
            }))
        }
        Command::RestoreWatchers => {
            let pending_services = services.clone();
            let sender = sender.clone();
            let repaint = repaint.clone();
            if services
                .tasks
                .spawn_transient("native-watcher-restore", async move {
                    if let Err(error) = crate::projects::NativeProjects::new(pending_services)
                        .restore_watchers()
                        .await
                    {
                        let _ = sender.send(HostReply::Failed(error)).await;
                    }
                    repaint();
                })
            {
                None
            } else {
                Some(HostReply::Failed(stopped()))
            }
        }
        Command::OpenWorkspaceSymbol(request) => {
            Some(HostReply::ExplorerSearch(Reply::WorkspaceSymbolOpened {
                result: crate::workspace_symbol_host::open(services, request).await,
            }))
        }
        Command::RevealBreadcrumbTree(source) => {
            let active = services
                .state
                .layouts
                .read()
                .get(&source.project)
                .is_some_and(|layout| source.is_active(layout));
            let result = if active {
                tree_actions::tree_reveal(
                    &services.state,
                    &services.tree,
                    &services.tasks,
                    source.project.clone(),
                    source.path.clone(),
                )
                .await
            } else {
                Err(AppError::NotFound(
                    "native breadcrumb source is closed or inactive".into(),
                ))
            };
            Some(HostReply::ExplorerSearch(Reply::BreadcrumbTree {
                source,
                result,
            }))
        }
        Command::TreeRows { project, offset } => {
            let result = tree_actions::tree_rows(
                &services.state,
                &services.tree,
                &services.tasks,
                project.clone(),
                offset,
                None,
            )
            .await;
            Some(HostReply::ExplorerSearch(Reply::Tree {
                project,
                offset,
                result,
            }))
        }
        Command::TreeToggle { project, path } => {
            let result = tree_actions::tree_toggle(
                &services.state,
                &services.tree,
                &services.tasks,
                project.clone(),
                path.clone(),
            )
            .await;
            Some(HostReply::ExplorerSearch(Reply::TreeToggled {
                project,
                path,
                result,
            }))
        }
        Command::TreeCollapse(project) => {
            let result = tree_actions::tree_collapse_all(
                &services.state,
                &services.tree,
                &services.tasks,
                project.clone(),
            )
            .await;
            Some(HostReply::ExplorerSearch(Reply::Tree {
                project,
                offset: 0,
                result,
            }))
        }
        Command::RevealExplorerPath {
            project,
            mount,
            path,
            rename,
        } => {
            let result = tree_actions::tree_reveal(
                &services.state,
                &services.tree,
                &services.tasks,
                project.clone(),
                path.clone(),
            )
            .await;
            Some(HostReply::ExplorerSearch(Reply::ExplorerPathRevealed {
                project,
                mount,
                path,
                rename,
                result,
            }))
        }
    }
}

impl NativeApplication {
    pub(crate) fn apply_explorer_search_reply(&mut self, context: &egui::Context, reply: Reply) {
        match reply {
            Reply::WorkspaceSymbolOpened { result } => {
                self.apply_current_file_link_reply(context, result)
            }
            Reply::BreadcrumbTree { source, result } => {
                let current = self.store.documents().snapshot(source.document).ok();
                let generation = self
                    .editor_symbols
                    .palette(Some(&source.project), current.as_ref())
                    .generation;
                if self.closing.is_none()
                    && !self.services.state.is_shutting_down()
                    && self
                        .services
                        .state
                        .layouts
                        .read()
                        .get(&source.project)
                        .is_some_and(|layout| source.is_active(layout))
                    && current
                        .as_ref()
                        .is_some_and(|current| source.describes(current, generation))
                {
                    match result {
                        Ok(page) => {
                            self.explorer_search_state
                                .trees
                                .insert(source.project, page);
                            context.request_repaint();
                        }
                        Err(error) => self.report(&error),
                    }
                }
            }
            Reply::TreeSynced {
                project,
                result,
                errors,
            } => {
                match result {
                    Ok(page) => {
                        self.explorer_search_state.trees.insert(project, page);
                    }
                    Err(error) => self.status = Some(error.to_string()),
                }
                if !errors.is_empty() {
                    self.status = Some(
                        errors
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join("\n"),
                    );
                }
            }
            Reply::Tree {
                project,
                offset,
                result,
            } => {
                self.explorer_search_state.tree_loading.remove(&project);
                match result {
                    Ok(page) if offset == 0 => {
                        self.explorer_search_state.trees.insert(project, page);
                    }
                    Ok(page) => {
                        if let Some(current) = self.explorer_search_state.trees.get_mut(&project) {
                            current.rows.extend(page.rows);
                            current.total = page.total;
                        }
                    }
                    Err(error) => self.status = Some(error.to_string()),
                }
            }
            Reply::TreeToggled {
                project,
                path,
                result,
            } => {
                if let Some(explorer) = self.explorer_search_state.explorers.get_mut(&project) {
                    explorer.toggle_finished(&path, result.as_ref().ok());
                }
                match result {
                    Ok(page) => {
                        self.explorer_search_state.trees.insert(project, page);
                    }
                    Err(error) => self.status = Some(error.to_string()),
                }
            }
            Reply::ExplorerPathRevealed {
                project,
                mount,
                path,
                rename,
                result,
            } => match result {
                Ok(page) => {
                    let explorer = self
                        .explorer_search_state
                        .explorers
                        .entry(project.clone())
                        .or_default();
                    explorer.address_requests(&mount);
                    explorer.select_path(&page.rows, &path);
                    if rename && let Some(row) = page.rows.iter().find(|row| row.path == path) {
                        explorer.start_rename(row);
                    }
                    self.explorer_search_state.trees.insert(project, page);
                    context.request_repaint();
                }
                Err(error) => self.status = Some(error.to_string()),
            },
        }
    }

    pub(crate) fn apply_explorer_search_intent(
        &mut self,
        intent: Intent,
        context: &egui::Context,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
    ) {
        match intent {
            Intent::ToggleAuxiliaryExplorer { project, slot } => {
                self.shell_state
                    .auxiliary_windows
                    .toggle_explorer(&project, slot);
                context.request_repaint();
            }
            Intent::ShowExplorer { project, slot } => {
                self.show_explorer_files(snapshot, &project, &slot);
                context.request_repaint();
            }
            Intent::ShowAuxiliaryExplorer { project, slot } => {
                self.show_auxiliary_explorer_files(&project, slot);
                context.request_repaint();
            }
            Intent::RevealInExplorer {
                project,
                slot,
                path,
                rename,
            } => {
                self.show_explorer_files(snapshot, &project, &slot);
                self.submit(HostCommand::ExplorerSearch(Command::RevealExplorerPath {
                    project,
                    mount: slot,
                    path,
                    rename,
                }));
            }
            Intent::RevealInAuxiliaryExplorer {
                project,
                slot,
                path,
                rename,
            } => {
                if let Some(explorer) = self.show_auxiliary_explorer_files(&project, slot) {
                    self.submit(HostCommand::ExplorerSearch(Command::RevealExplorerPath {
                        project,
                        mount: explorer,
                        path,
                        rename,
                    }));
                }
            }
        }
    }

    pub(crate) fn apply_explorer_search_lsp_reply(&mut self, reply: crate::lsp::Reply) {
        match reply {
            crate::lsp::Reply::WorkspaceSymbols { request, result } => {
                let providers = self.lsp.as_ref().map_or_else(HashSet::new, |lsp| {
                    lsp.workspace_providers(&request.project)
                });
                if self
                    .services
                    .state
                    .projects
                    .read()
                    .contains_key(&request.project)
                {
                    self.explorer_search_state
                        .workspace_symbols
                        .accept(&request, providers, result);
                }
            }
            crate::lsp::Reply::ExplorerPasted(event) => {
                let path = match event.result {
                    Ok(pasted) => {
                        if self
                            .services
                            .state
                            .projects
                            .read()
                            .contains_key(&event.project)
                        {
                            self.explorer_search_state
                                .trees
                                .insert(event.project.clone(), pasted.page);
                            Some(pasted.path)
                        } else {
                            None
                        }
                    }
                    Err(error) => {
                        self.report(&error);
                        None
                    }
                };
                self.explorer_search_state
                    .explorer_clipboards
                    .paste_finished(event.request.owner, event.clear_cut, path);
            }
            crate::lsp::Reply::ExplorerCreated(event) => match event.result {
                Ok(created) => {
                    if self
                        .services
                        .state
                        .projects
                        .read()
                        .contains_key(&event.project)
                    {
                        self.explorer_search_state
                            .trees
                            .insert(event.project.clone(), created.page);
                        if let Some(layout) = created.layout {
                            self.controller
                                .apply_layouts(HashMap::from([(event.project.clone(), layout)]));
                        }
                        if let Some(explorer) =
                            self.explorer_search_state.explorers.get_mut(&event.project)
                        {
                            explorer.create_finished(&event.request, Ok(()));
                        }
                    }
                }
                Err(error) => self.report_create_failure(&event.project, &event.request, &error),
            },
            crate::lsp::Reply::ExplorerRenamed(event) => match event.result {
                Ok(page) => {
                    self.explorer_search_state
                        .trees
                        .insert(event.project.clone(), page);
                    if let Some(explorer) =
                        self.explorer_search_state.explorers.get_mut(&event.project)
                    {
                        explorer.rename_finished(&event.request, Ok(()));
                    }
                }
                Err(error) => self.report_rename_failure(&event.project, &event.request, &error),
            },
            crate::lsp::Reply::ExplorerDeleteFinished { request, result } => {
                self.explorer_search_state.deleting_entry = false;
                match result {
                    Ok(_) => {
                        let parent = std::path::Path::new(&request.path)
                            .parent()
                            .and_then(|parent| parent.to_str())
                            .map(String::from);
                        if self
                            .services
                            .state
                            .projects
                            .read()
                            .contains_key(&request.project)
                        {
                            self.submit(HostCommand::ExplorerSearch(Command::RefreshTree {
                                project: request.project,
                                dirs: parent
                                    .map(|parent| std::collections::BTreeSet::from([parent])),
                            }));
                        }
                    }
                    Err(error) => self.report(&error),
                }
            }
            _ => {}
        }
    }

    pub(crate) fn move_explorer_search_entries(
        &mut self,
        from: &std::path::Path,
        to: &std::path::Path,
    ) {
        for explorer in self.explorer_search_state.explorers.values_mut() {
            explorer.moved(from, to);
        }
    }

    pub(crate) fn reconnect_explorer_search(&mut self) {
        self.explorer_search_state.tree_loading.clear();
    }

    pub(crate) fn tick_explorer_search_clipboards(
        &mut self,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
    ) {
        let auxiliary_explorers = self.shell_state.auxiliary_windows.explorer_mounts();
        self.explorer_search_state
            .explorer_clipboards
            .reconcile_windows(
                snapshot.shell.tree.as_ref(),
                &auxiliary_explorers.keys().cloned().collect(),
            );
    }

    pub(crate) fn tick_explorer_search_breadcrumbs(
        &mut self,
        context: &egui::Context,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
    ) {
        self.explorer_search_state.breadcrumbs.reconcile(
            context,
            &snapshot.layouts,
            snapshot.shell.tree.as_ref(),
            &self.shell.scope,
        );
    }

    pub(crate) fn tick_explorer_search_trees(
        &mut self,
        context: &egui::Context,
        open_projects: &HashSet<ProjectId>,
    ) {
        for (project, dirs) in self
            .explorer_search_state
            .tree_changes
            .take(open_projects.iter().cloned())
        {
            if !open_projects.contains(&project) {
                continue;
            }
            if self.submit(HostCommand::ExplorerSearch(Command::RefreshTree {
                project: project.clone(),
                dirs: dirs.clone(),
            })) {
                self.queue_project_observations(&project);
            } else {
                self.explorer_search_state.tree_changes.merge(project, dirs);
                context.request_repaint();
            }
        }
    }

    pub(crate) fn load_explorer_search_trees(
        &mut self,
        context: &egui::Context,
        load_trees: Vec<ProjectId>,
    ) {
        for project in load_trees {
            if self
                .explorer_search_state
                .tree_loading
                .insert(project.clone())
                && !self.submit(HostCommand::ExplorerSearch(Command::TreeRows {
                    project: project.clone(),
                    offset: 0,
                }))
            {
                self.explorer_search_state.tree_loading.remove(&project);
                context.request_repaint();
            }
        }
    }

    pub(crate) fn fail_explorer_search_entries(
        &mut self,
        create_entries: Vec<(ProjectId, crate::explorer::CreateRequest)>,
        rename_entries: Vec<(ProjectId, crate::explorer::RenameRequest)>,
    ) {
        for (project, request) in create_entries {
            if let Some(explorer) = self.explorer_search_state.explorers.get_mut(&project) {
                explorer.create_finished(
                    &request,
                    Err("native dirty tab update is unavailable".into()),
                );
            }
        }
        for (project, request) in rename_entries {
            if let Some(explorer) = self.explorer_search_state.explorers.get_mut(&project) {
                explorer.rename_finished(
                    &request,
                    Err("native dirty tab update is unavailable".into()),
                );
            }
        }
    }

    pub(crate) fn submit_explorer_search_entries(
        &mut self,
        create_entries: Vec<(ProjectId, crate::explorer::CreateRequest)>,
        rename_entries: Vec<(ProjectId, crate::explorer::RenameRequest)>,
        paste_entries: Vec<(ProjectId, crate::explorer_clipboard::Request)>,
    ) {
        for (project, request) in create_entries {
            self.create_entry(project, request);
        }
        for (project, request) in rename_entries {
            self.rename_entry(project, request);
        }
        for (project, request) in paste_entries {
            let result = self
                .lsp
                .as_ref()
                .ok_or_else(|| AppError::Internal("native workspace host is unavailable".into()))
                .and_then(|bridge| bridge.paste_entry(project, request));
            if let Err(error) = result {
                self.report(&error);
            }
        }
    }

    pub(crate) fn close_explorer_search_discarded_delete(
        &mut self,
        discarded_explorers: &[ProjectId],
    ) {
        if self
            .explorer_search_state
            .pending_entry_delete
            .as_ref()
            .is_some_and(|pending| discarded_explorers.contains(&pending.request.project))
        {
            self.explorer_search_state.pending_entry_delete = None;
        }
    }

    pub(crate) fn queue_explorer_search_delete(
        &mut self,
        delete_entries: Vec<crate::explorer_delete::Request>,
    ) {
        if let Some(request) = delete_entries.into_iter().next() {
            self.explorer_search_state.pending_entry_delete =
                Some(crate::delete_dialog::Confirmation::new(request));
        }
    }

    pub(crate) fn show_explorer_search_delete_dialog(&mut self, context: &egui::Context) {
        if let Some(pending) = self.explorer_search_state.pending_entry_delete.as_mut() {
            let choice = pending.show(context, &self.locale).choice;
            if let Some(confirmed) = choice {
                let request = self
                    .explorer_search_state
                    .pending_entry_delete
                    .take()
                    .expect("delete confirmation exists")
                    .request;
                context.memory_mut(|memory| {
                    memory.request_focus(egui::Id::new(("native-tree-focus", &request.project)))
                });
                if confirmed && self.flush_dirty() {
                    let result = self
                        .lsp
                        .as_ref()
                        .ok_or_else(|| {
                            AppError::Internal("native workspace host is unavailable".into())
                        })
                        .and_then(|bridge| bridge.delete_entry(request));
                    match result {
                        Ok(()) => self.explorer_search_state.deleting_entry = true,
                        Err(error) => self.report(&error),
                    }
                }
                context.request_repaint();
            }
        }
    }

    fn show_explorer_files(
        &mut self,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
        project: &ProjectId,
        slot: &ShellSlotId,
    ) {
        self.explorer_search_state
            .symbol_sidebars
            .entry(project, slot)
            .view = crate::symbol_sidebar::View::Files;
        if snapshot
            .layouts
            .get(project)
            .is_some_and(|layout| layout.shell_view.sidebar_collapsed)
            && let Err(error) = self.controller.submit(ShellMutation::SetSidebarCollapsed {
                project: project.clone(),
                collapsed: false,
            })
        {
            self.report(&error);
        }
    }

    fn show_auxiliary_explorer_files(
        &mut self,
        project: &ProjectId,
        slot: u32,
    ) -> Option<ShellSlotId> {
        let explorer = self
            .shell_state
            .auxiliary_windows
            .show_explorer(project, slot)?;
        self.explorer_search_state
            .symbol_sidebars
            .entry(project, &explorer)
            .view = crate::symbol_sidebar::View::Files;
        Some(explorer)
    }

    pub(crate) fn report_create_failure(
        &mut self,
        project: &ProjectId,
        request: &crate::explorer::CreateRequest,
        error: &AppError,
    ) {
        let message = crate::toast::describe_error(&self.locale, error);
        if let Some(explorer) = self.explorer_search_state.explorers.get_mut(project) {
            explorer.create_finished(request, Err(message.clone()));
        }
        crate::toast::entry_failed(
            &mut self.toasts,
            &self.locale,
            message,
            crate::toast::Action::RetryCreate {
                project: project.clone(),
                request: request.clone(),
            },
            Instant::now(),
        );
    }

    pub(crate) fn report_rename_failure(
        &mut self,
        project: &ProjectId,
        request: &crate::explorer::RenameRequest,
        error: &AppError,
    ) {
        let message = crate::toast::describe_error(&self.locale, error);
        if let Some(explorer) = self.explorer_search_state.explorers.get_mut(project) {
            explorer.rename_finished(request, Err(message.clone()));
        }
        crate::toast::entry_failed(
            &mut self.toasts,
            &self.locale,
            message,
            crate::toast::Action::RetryRename {
                project: project.clone(),
                request: request.clone(),
            },
            Instant::now(),
        );
    }

    pub(crate) fn create_entry(
        &mut self,
        project: ProjectId,
        request: crate::explorer::CreateRequest,
    ) {
        let result = self
            .lsp
            .as_ref()
            .ok_or_else(|| AppError::Internal("native workspace host is unavailable".into()))
            .and_then(|bridge| bridge.create_entry(project.clone(), request.clone()));
        if let Err(error) = result {
            self.report_create_failure(&project, &request, &error);
        }
    }

    pub(crate) fn rename_entry(
        &mut self,
        project: ProjectId,
        request: crate::explorer::RenameRequest,
    ) {
        let result = self
            .lsp
            .as_ref()
            .ok_or_else(|| AppError::Internal("native workspace host is unavailable".into()))
            .and_then(|bridge| bridge.rename_entry(project.clone(), request.clone()));
        if let Err(error) = result {
            self.report_rename_failure(&project, &request, &error);
        }
    }

    pub(crate) fn observe_workspace_symbols(
        &mut self,
        context: &egui::Context,
        project: Option<&ProjectId>,
        enabled: bool,
    ) {
        let providers = project
            .zip(self.lsp.as_ref())
            .map_or_else(HashSet::new, |(project, lsp)| {
                lsp.workspace_providers(project)
            });
        let query = self
            .commands_state
            .palette
            .workspace_query()
            .filter(|_| enabled);
        let now = Instant::now();
        if let Some(request) = self.explorer_search_state.workspace_symbols.observe(
            project,
            query,
            providers.clone(),
            now,
        ) {
            let sent = self
                .lsp
                .as_ref()
                .is_some_and(|lsp| lsp.workspace_symbols(request.clone()).is_ok());
            if !sent {
                self.explorer_search_state.workspace_symbols.accept(
                    &request,
                    providers,
                    Err(taide_lsp::native::Failure::TransportClosed),
                );
            }
        }
        if let Some(delay) = self.explorer_search_state.workspace_symbols.delay(now) {
            context.request_repaint_after(delay);
        }
    }
}

enum OutlineReveal {
    Untitled {
        key: ViewKey,
        document: taide_native_editor::document::DocumentSnapshot,
        selection: std::ops::Range<usize>,
    },
    File {
        tab: TabId,
        path: String,
        viewport: egui::ViewportId,
        position: crate::editor_reveal::Position,
    },
}

/// A sidebar panel's own state, taken out of the app for one render inside the panel's error
/// boundary, and the requests that render made (applied only if the render finished).
#[derive(Default)]
pub(crate) struct ExplorerRegion {
    mount: Option<ShellSlotId>,
    owner: Option<crate::explorer_clipboard_owners::Owner>,
    sidebar: Option<crate::symbol_sidebar::Sidebar>,
    explorer: Option<crate::explorer::Explorer>,
    icons: crate::navigation_icons::Deferred,
    outline_reveal: Option<OutlineReveal>,
    is_tree_requested: bool,
    is_reveal_taken: bool,
    is_files_shown: bool,
    window_pane: Option<PaneId>,
    actions: Vec<crate::explorer::Action>,
    errors: Vec<String>,
}

impl ExplorerSearchSurfaces for AppSurfaces<'_> {
    type Explorer = ExplorerRegion;
    fn is_auxiliary_explorer_open(&self, project: &ProjectId, slot: u32) -> bool {
        self.auxiliary_windows
            .find(project, slot)
            .is_some_and(|window| window.is_explorer_open)
    }
    fn begin_explorer(
        &mut self,
        project: &ProjectId,
        mount: taide_native_ui::shell::ExplorerMount<'_>,
    ) -> ExplorerRegion {
        let slot = match mount {
            taide_native_ui::shell::ExplorerMount::Slot(slot) => Some(slot.clone()),
            taide_native_ui::shell::ExplorerMount::Auxiliary(slot) => self
                .auxiliary_windows
                .find(project, slot)
                .map(|window| window.explorer.clone()),
        };
        let Some(slot) = slot else {
            return ExplorerRegion::default();
        };
        let mut region = ExplorerRegion {
            sidebar: Some(self.symbol_sidebars.take(project, &slot)),
            explorer: self.explorers.remove(project),
            ..Default::default()
        };
        match self.explorer_clipboards.mount(&slot) {
            Ok(owner) => region.owner = Some(owner),
            Err(error) => region.errors.push(error.to_string()),
        }
        region.mount = Some(slot);
        region
    }
    fn explorer(
        &self,
        ui: &mut Ui,
        project: &ProjectId,
        _: taide_native_ui::shell::ExplorerMount<'_>,
        region: &mut ExplorerRegion,
        intents: &mut Vec<ShellIntent>,
    ) {
        let ExplorerRegion {
            mount: Some(slot),
            owner: Some(owner),
            sidebar: Some(sidebar),
            explorer,
            icons,
            outline_reveal,
            is_tree_requested,
            is_reveal_taken,
            is_files_shown,
            window_pane,
            actions,
            errors,
        } = region
        else {
            return;
        };
        let (slot, owner) = (&*slot, *owner);
        match sidebar.switch(
            ui,
            egui::Id::new(("symbol-sidebar", project, slot)),
            self.locale,
            self.navigation_appearance,
            &mut *icons,
        ) {
            Ok(buttons) => {
                for (view, response) in buttons {
                    self.tooltips.show(
                        &response,
                        &presentation::message(self.locale, view.label(), &[]),
                        egui::RectAlign::BOTTOM,
                        self.tooltip_appearance,
                    );
                }
            }
            Err(error) => errors.push(error.to_string()),
        }
        if sidebar.view == crate::symbol_sidebar::View::Git {
            self.show_git_view(ui, project, slot, &mut *icons, errors, intents);
        }
        if matches!(
            sidebar.view,
            crate::symbol_sidebar::View::Search | crate::symbol_sidebar::View::Git
        ) {
            return;
        }
        if sidebar.view == crate::symbol_sidebar::View::Outline {
            let active = self
                .layouts
                .get(project)
                .and_then(|layout| crate::symbol_sidebar::window_tree(project, layout, self.scope))
                .and_then(|(root, focused)| {
                    taide_native_ui::snapshot::active_tab(root, focused).map(|tab| (focused, tab))
                });
            let document = active.and_then(|(_, tab)| {
                let id = match &tab.kind {
                    TabKind::File { path } => self.files.get(path).map(|file| file.id),
                    TabKind::Untitled { .. } => {
                        self.untitled.get(&tab.id).map(|document| document.id)
                    }
                    _ => None,
                }?;
                self.store.documents().snapshot(id).ok()
            });
            let key = active.and_then(|(_, tab)| match &tab.kind {
                TabKind::File { path } => Some(taide_native_editor::document::DocumentKey::File(
                    path.into(),
                )),
                TabKind::Untitled { .. } => Some(
                    taide_native_editor::document::DocumentKey::Untitled(tab.id.clone()),
                ),
                _ => None,
            });
            match sidebar.outline.show(
                ui,
                egui::Id::new(("symbol-outline", project, slot)),
                crate::symbol_outline::Scope {
                    key,
                    symbols: self
                        .editor_symbols
                        .palette(Some(project), document.as_ref()),
                },
                self.locale,
                self.navigation_appearance,
                &mut *icons,
            ) {
                Ok(output) => {
                    if let (Some((generation, index)), Some((pane, tab)), Some(document)) =
                        (output.reveal, active, document.as_ref())
                        && let Some(selection) = self
                            .editor_symbols
                            .selection(project, document, generation, index)
                    {
                        if matches!(tab.kind, TabKind::Untitled { .. }) {
                            *outline_reveal = Some(OutlineReveal::Untitled {
                                key: ViewKey {
                                    window: WINDOW_LABEL.into(),
                                    pane: pane.clone(),
                                    tab: tab.id.clone(),
                                },
                                document: document.clone(),
                                selection,
                            });
                        } else if let TabKind::File { path } = &tab.kind
                            && let Some(position) = self
                                .editor_symbols
                                .position(project, document, generation, index)
                        {
                            *outline_reveal = Some(OutlineReveal::File {
                                tab: tab.id.clone(),
                                path: path.clone(),
                                viewport: ui.ctx().viewport_id(),
                                position,
                            });
                        }
                        ui.ctx().request_repaint();
                    }
                }
                Err(error) => errors.push(error.to_string()),
            }
            return;
        }
        let Some(page) = self.trees.get(project) else {
            if !self.tree_loading.contains(project) {
                *is_tree_requested = true;
            }
            ui.spinner();
            return;
        };
        let explorer = explorer.get_or_insert_with(Default::default);
        explorer.replace_clipboard(self.explorer_clipboards.clipboard(owner).cloned());
        if let Some(path) = self.explorer_clipboards.reveal(owner) {
            explorer.address_requests(slot);
            explorer.paste_finished(false, Some(path.clone()));
            *is_reveal_taken = true;
        }
        if let Some(project) = self.projects.iter().find(|entry| &entry.id == project) {
            explorer.root = Some(project.root.clone());
            explorer.title = Some(project.name.clone());
        }
        let output = explorer.show_with_icons(
            ui,
            project,
            slot,
            page,
            self.locale,
            crate::explorer::RowIcons {
                glyphs: icons,
                appearance: self.explorer_appearance,
            },
        );
        if let Some(error) = &output.icon_error {
            errors.push(error.to_string());
        }
        crate::explorer_toolbar::show_tooltips(
            &output,
            self.locale,
            self.tooltips,
            self.tooltip_appearance,
        );
        *window_pane = match self.scope {
            WindowScope::Main => None,
            WindowScope::Auxiliary { .. } => self
                .layouts
                .get(project)
                .and_then(|layout| crate::symbol_sidebar::window_tree(project, layout, self.scope))
                .map(|(_, focused)| focused.clone()),
        };
        *actions = output.actions;
        actions.retain(|action| {
            let crate::explorer::Action::FileHistory(path) = action else {
                return true;
            };
            intents.push(ShellIntent::Git(
                taide_native_ui::intents::git::Intent::OpenFileHistory {
                    project: project.clone(),
                    path: path.clone(),
                },
            ));
            false
        });
        *is_files_shown = true;
    }
    fn finish_explorer(
        &mut self,
        context: &egui::Context,
        project: &ProjectId,
        _: taide_native_ui::shell::ExplorerMount<'_>,
        region: ExplorerRegion,
    ) {
        let ExplorerRegion {
            mount,
            owner,
            sidebar,
            explorer,
            icons,
            outline_reveal,
            is_tree_requested,
            is_reveal_taken,
            is_files_shown,
            window_pane,
            actions,
            errors,
        } = region;
        let Some(slot) = mount else {
            return;
        };
        if let Some(sidebar) = sidebar {
            self.symbol_sidebars.put(slot, sidebar);
        }
        if let Some(explorer) = explorer {
            self.explorers.insert(project.clone(), explorer);
        }
        if let Some(error) = errors.into_iter().last() {
            *self.status = Some(error);
        }
        if let Err(error) = icons.fill(&mut *self.explorer_icons, &mut *self.navigation_icons) {
            *self.status = Some(error.to_string());
        }
        if is_tree_requested {
            self.explorer_output.load_trees.push(project.clone());
        }
        match outline_reveal {
            Some(OutlineReveal::Untitled {
                key,
                document,
                selection,
            }) => {
                match crate::editor_reveal::reveal_document(self.store, &key, &document, selection)
                {
                    Ok(Some(view)) => {
                        if let Some(body) = crate::editor_context_menu::body(context, view) {
                            context.memory_mut(|memory| memory.request_focus(body));
                        }
                    }
                    Ok(None) => {}
                    Err(error) => *self.status = Some(editor_error(error).to_string()),
                }
            }
            Some(OutlineReveal::File {
                tab,
                path,
                viewport,
                position,
            }) => {
                self.reveals.queue_position(
                    crate::editor_reveal::Target {
                        project,
                        tab: &tab,
                        path: &path,
                        viewport,
                    },
                    position,
                    self.layouts,
                    Instant::now(),
                );
            }
            None => {}
        }
        let Some(owner) = owner else {
            return;
        };
        if is_reveal_taken {
            self.explorer_clipboards.take_reveal(owner);
        }
        for action in actions {
            match action {
                crate::explorer::Action::Toggle(path) => {
                    self.commands
                        .push(HostCommand::ExplorerSearch(Command::TreeToggle {
                            project: project.clone(),
                            path,
                        }))
                }
                crate::explorer::Action::Open { path, preview } => self.commands.push(
                    HostCommand::Editor(crate::domains::editor::Command::OpenFileTab {
                        project: project.clone(),
                        pane: window_pane.clone(),
                        path,
                        preview,
                    }),
                ),
                crate::explorer::Action::OpenToSide(row) => {
                    if let Some(request) = crate::explorer::open_to_side_request(
                        project,
                        &row,
                        self.layouts.get(project),
                        self.scope,
                    ) {
                        self.commands.push(HostCommand::Editor(
                            crate::domains::editor::Command::OpenFileToSide(request),
                        ));
                    }
                }
                crate::explorer::Action::OpenWith { row, mode } => {
                    self.open_with.set(row.path.clone(), mode);
                    self.commands.push(HostCommand::Editor(
                        crate::domains::editor::Command::OpenFileTab {
                            project: project.clone(),
                            pane: window_pane.clone(),
                            path: row.path,
                            preview: false,
                        },
                    ));
                }
                crate::explorer::Action::Rename(request) => self
                    .explorer_output
                    .rename_entries
                    .push((project.clone(), request)),
                crate::explorer::Action::Create(request) => self
                    .explorer_output
                    .create_entries
                    .push((project.clone(), request)),
                crate::explorer::Action::Paste(mut request) => {
                    request.owner = Some(owner);
                    self.explorer_output
                        .paste_entries
                        .push((project.clone(), request))
                }
                crate::explorer::Action::RequestDelete(row) => self
                    .explorer_output
                    .delete_entries
                    .push(crate::explorer_delete::Request {
                        project: project.clone(),
                        path: row.path,
                        name: row.name,
                    }),
                crate::explorer::Action::CopyText(text) => {
                    self.commands.push(HostCommand::Shell(
                        crate::domains::shell::Command::CopyText(text),
                    ));
                }
                crate::explorer::Action::RevealPath(path) => {
                    self.commands.push(HostCommand::Shell(
                        crate::domains::shell::Command::RevealPath(path),
                    ));
                }
                crate::explorer::Action::OpenInBrowser(path) => {
                    self.commands.push(HostCommand::Shell(
                        crate::domains::shell::Command::OpenInBrowser(path),
                    ));
                }
                crate::explorer::Action::Refresh => {
                    self.commands
                        .push(HostCommand::ExplorerSearch(Command::RefreshTree {
                            project: project.clone(),
                            dirs: None,
                        }))
                }
                crate::explorer::Action::Collapse => {
                    self.commands
                        .push(HostCommand::ExplorerSearch(Command::TreeCollapse(
                            project.clone(),
                        )))
                }
                crate::explorer::Action::SelectForCompare(path) => {
                    if let Some(explorer) = self.explorers.get_mut(project) {
                        explorer.compare_source = Some(path);
                    }
                }
                crate::explorer::Action::CompareWithSelected(path) => {
                    if let Some(command) = self
                        .explorers
                        .get_mut(project)
                        .and_then(|explorer| explorer.compare_source.take())
                        .map(|source| {
                            crate::domains::git::compare_tab(project.clone(), source, path)
                        })
                    {
                        self.commands.push(command);
                    }
                }
                crate::explorer::Action::FileHistory(_) => {}
            }
        }
        if is_files_shown && let Some(explorer) = self.explorers.get(project) {
            self.explorer_clipboards
                .replace(owner, explorer.clipboard().cloned());
        }
    }
    fn discard_explorer(
        &mut self,
        project: &ProjectId,
        _: taide_native_ui::shell::ExplorerMount<'_>,
        region: ExplorerRegion,
    ) {
        let Some(slot) = region.mount else {
            return;
        };
        self.explorer_output
            .discarded_explorers
            .push(project.clone());
        self.explorer_clipboards.unmount(&slot);
        if let Some(explorer) = region.explorer {
            self.explorers.insert(project.clone(), explorer.remounted());
        }
    }
}

impl AppSurfaces<'_> {
    pub(crate) fn show_search_editor_tab(&mut self, ui: &mut Ui, tab: &Tab) {
        self.show_unavailable_tab(ui, tab);
    }

    pub(crate) fn show_breadcrumbs(
        &mut self,
        ui: &mut Ui,
        pane: &PaneId,
        tab: &Tab,
        view: ViewId,
        snapshot: &taide_native_editor::document::DocumentSnapshot,
        intents: &mut Vec<ShellIntent>,
    ) -> AppResult<()> {
        let owner = self
            .layouts
            .iter()
            .find(|(_, layout)| {
                taide_layout::service::all_roots(layout)
                    .any(|root| taide_layout::service::find_leaf(root, pane).is_some())
            })
            .map(|(project, _)| project);
        let project =
            owner.and_then(|owner| self.projects.iter().find(|project| &project.id == owner));
        let symbols = self.editor_symbols.palette(owner, Some(snapshot));
        let source = match (&tab.kind, project) {
            (TabKind::File { path }, Some(project)) => Some(crate::breadcrumbs::Source {
                project: project.id.clone(),
                pane: pane.clone(),
                tab: tab.id.clone(),
                path: path.clone(),
                document: snapshot.id,
                revision: snapshot.revision,
                language: snapshot.metadata.language_id.clone(),
                generation: symbols.generation,
            }),
            _ => None,
        };
        let caret = self
            .store
            .views()
            .get(view)
            .and_then(|state| state.selection.selections.get(state.selection.primary))
            .map(|selection| selection.head);
        let rows = owner
            .and_then(|project| self.trees.get(project))
            .map_or(&[][..], |page| page.rows.as_slice());
        let mut empty = crate::breadcrumbs::Bar::default();
        let bar = match &source {
            Some(source) => self.breadcrumbs.entry(ui.ctx().viewport_id(), source),
            None => &mut empty,
        };
        let output = bar.show(
            ui,
            egui::Id::new(("breadcrumbs", ui.ctx().viewport_id(), pane, &tab.id)),
            crate::breadcrumbs::Scope {
                source,
                root: project.map_or("", |project| project.root.as_str()),
                caret,
                symbols,
                rows,
            },
            self.locale,
            self.navigation_appearance,
            self.navigation_icons,
        )?;
        if output
            .segments
            .iter()
            .chain(&output.entries)
            .any(|response| response.has_focus())
        {
            *self.focused = Some((pane.clone(), tab.id.clone()));
            if Some(pane) != self.target {
                intents.push(ShellIntent::Mutate(ShellMutation::FocusPane(pane.clone())));
            }
        }
        for action in output.actions {
            match action {
                crate::breadcrumbs::Action::RevealTree(source) => {
                    self.commands
                        .push(HostCommand::ExplorerSearch(Command::RevealBreadcrumbTree(
                            source,
                        )))
                }
                crate::breadcrumbs::Action::OpenFile { source, path } => self.commands.push(
                    HostCommand::Editor(crate::domains::editor::Command::OpenBreadcrumbFile {
                        source,
                        path,
                        viewport: ui.ctx().viewport_id(),
                    }),
                ),
                crate::breadcrumbs::Action::RevealSymbol { source, index } => {
                    if let Some(position) = self.editor_symbols.position(
                        &source.project,
                        snapshot,
                        source.generation,
                        index,
                    ) {
                        self.reveals.queue_position(
                            crate::editor_reveal::Target {
                                project: &source.project,
                                tab: &source.tab,
                                path: &source.path,
                                viewport: ui.ctx().viewport_id(),
                            },
                            position,
                            self.layouts,
                            Instant::now(),
                        );
                        ui.ctx().request_repaint();
                    }
                }
            }
        }
        Ok(())
    }
}

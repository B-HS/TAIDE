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

/// Who asked for a tree refresh, which decides how a failure shows (`ipc-sync-provider.tsx`
/// watcher sync, the toolbar's unhandled refresh, the delete's parent refresh toast).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshOrigin {
    Watcher,
    Toolbar,
    Delete,
}

pub enum Command {
    RestoreWatchers,
    RefreshTree {
        project: ProjectId,
        dirs: crate::events::TreeInvalidation,
        origin: RefreshOrigin,
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
        quiet: bool,
    },
    ReopenProject {
        project: ProjectId,
        root: String,
    },
    RunSearch {
        run: crate::search_panel::Run,
        target: SearchTarget,
        project: ProjectId,
        streams: Arc<crate::search_stream::Streams>,
    },
    CancelSearch(crate::search_stream::Session),
    OpenSearchMatch {
        project: ProjectId,
        pane: Option<PaneId>,
        open: crate::search_panel::Open,
        viewport: egui::ViewportId,
    },
    RecordRecentSearches(Vec<String>),
    ReplaceSearch {
        mount: ShellSlotId,
        project: ProjectId,
        replace: crate::search_panel::Replace,
    },
    OpenSearchEditor {
        project: ProjectId,
        pane: Option<PaneId>,
        query: taide_model::search::SearchQuery,
        title: String,
    },
    OpenTerminalAt {
        project: ProjectId,
        pane: Option<PaneId>,
        cwd: String,
        title: String,
    },
}

/// The surface a search run streams into: a sidebar Search view or a search editor tab body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchTarget {
    Sidebar(ShellSlotId),
    Editor(TabId),
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
        origin: RefreshOrigin,
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
        quiet: bool,
        result: AppResult<TreeRowPage>,
    },
    ProjectReopened {
        closed: ProjectId,
        failure: Option<AppError>,
    },
    SearchFinished {
        target: SearchTarget,
        generation: u64,
        result: AppResult<u32>,
    },
    SearchMatchOpened {
        result: AppResult<crate::terminal_tabs::OpenedFileLink>,
    },
    RecentSearchesSaved,
    SearchReplaced {
        mount: ShellSlotId,
        project: ProjectId,
        query: taide_model::search::SearchQuery,
        result: AppResult<taide_model::search::SearchReplaceResult>,
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
    pub(crate) explorers: crate::explorer_mounts::Mounts,
    pub(crate) explorer_clipboards: crate::explorer_clipboard_owners::ClipboardOwners,
    pub(crate) pending_entry_delete: Option<crate::delete_dialog::Confirmation>,
    pub(crate) deleting_entry: bool,
    pub(crate) tree_loading: HashSet<ProjectId>,
    pub(crate) tree_changes: Arc<crate::events::TreeChanges>,
    pub(crate) tree_failures: HashSet<ProjectId>,
    pub(crate) git_decorations: crate::explorer_git_decorations::GitDecorations,
    pub(crate) search_streams: Arc<crate::search_stream::Streams>,
    pub(crate) search_history: crate::search_history::Queue,
    pub(crate) search_editors: crate::search_editor::Editors,
}

/// Explorer and search inputs a sidebar region render needs besides the tree pages; only the
/// region's finish step writes the recent-search queue. Search editor tab bodies draw from and
/// into `search_editors` directly.
pub(crate) struct ExplorerInputs<'a> {
    pub(crate) tree_failures: &'a HashSet<ProjectId>,
    pub(crate) git_decorations: &'a crate::explorer_git_decorations::GitDecorations,
    pub(crate) is_auto_reveal_enabled: bool,
    pub(crate) search_streams: &'a Arc<crate::search_stream::Streams>,
    pub(crate) search_history: &'a mut crate::search_history::Queue,
    pub(crate) search_editors: &'a mut crate::search_editor::Editors,
}

#[derive(Default)]
pub(crate) struct FrameOutput {
    pub(crate) rename_entries: Vec<(ProjectId, crate::explorer::RenameRequest)>,
    pub(crate) paste_entries: Vec<(ProjectId, crate::explorer_clipboard::Request)>,
    pub(crate) create_entries: Vec<(ProjectId, crate::explorer::CreateRequest)>,
    pub(crate) delete_entries: Vec<crate::explorer_delete::Request>,
    pub(crate) discarded_explorers: Vec<ProjectId>,
    pub(crate) load_trees: Vec<ProjectId>,
    pub(crate) is_editor_widget_focused: bool,
}

pub(crate) async fn dispatch(worker: &HostWorker<'_>, command: Command) -> Option<HostReply> {
    let services = worker.services;
    let sender = worker.sender;
    let repaint = worker.repaint;
    match command {
        Command::RefreshTree {
            project,
            dirs,
            origin,
        } => {
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
                origin,
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
            quiet,
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
                quiet,
                result,
            }))
        }
        Command::ReopenProject { project, root } => {
            if services.state.projects.read().contains_key(&project) {
                return None;
            }
            let failure = match crate::domains::shell::dispatch(
                worker,
                crate::domains::shell::Command::OpenProject(root),
            )
            .await
            {
                Some(HostReply::Failed(error)) => Some(error),
                Some(reply) => {
                    let _ = sender.send(reply).await;
                    None
                }
                None => None,
            };
            Some(HostReply::ExplorerSearch(Reply::ProjectReopened {
                closed: project,
                failure,
            }))
        }
        Command::RunSearch {
            run,
            target,
            project,
            streams,
        } => {
            let crate::search_panel::Run {
                session,
                generation,
                query,
                ..
            } = run;
            let pending_services = services.clone();
            let sender = sender.clone();
            let repaint = repaint.clone();
            let pending_target = target.clone();
            if services
                .tasks
                .spawn_transient("native-search-run", async move {
                    let batch_repaint = repaint.clone();
                    let batch_session = session.clone();
                    let result = taide_runtime::search_actions::search_run(
                        taide_runtime::search_actions::SearchRunContext {
                            state: &pending_services.state,
                            store: &pending_services.search,
                            tasks: &pending_services.tasks,
                        },
                        project,
                        session.owner,
                        session.id,
                        query,
                        move |batch| {
                            if streams.push(&batch_session, generation, batch) {
                                batch_repaint();
                            }
                        },
                    )
                    .await;
                    let reply = HostReply::ExplorerSearch(Reply::SearchFinished {
                        target: pending_target,
                        generation,
                        result,
                    });
                    if sender.send(reply).await.is_ok() {
                        repaint();
                    }
                })
            {
                None
            } else {
                Some(HostReply::ExplorerSearch(Reply::SearchFinished {
                    target,
                    generation,
                    result: Err(stopped()),
                }))
            }
        }
        Command::ReplaceSearch {
            mount,
            project,
            replace,
        } => {
            let pending_services = services.clone();
            let sender = sender.clone();
            let repaint = repaint.clone();
            let pending_mount = mount.clone();
            let pending_project = project.clone();
            let query = replace.query.clone();
            if services
                .tasks
                .spawn_transient("native-search-replace", async move {
                    let result = taide_runtime::search_actions::search_replace(
                        &pending_services.state,
                        &pending_services.tasks,
                        pending_project.clone(),
                        replace.query.clone(),
                        replace.replacement,
                        replace.paths,
                    )
                    .await;
                    let reply = HostReply::ExplorerSearch(Reply::SearchReplaced {
                        mount: pending_mount,
                        project: pending_project,
                        query: replace.query,
                        result,
                    });
                    if sender.send(reply).await.is_ok() {
                        repaint();
                    }
                })
            {
                None
            } else {
                Some(HostReply::ExplorerSearch(Reply::SearchReplaced {
                    mount,
                    project,
                    query,
                    result: Err(stopped()),
                }))
            }
        }
        Command::OpenSearchEditor {
            project,
            pane,
            query,
            title,
        } => {
            open_tab(
                services,
                project,
                TabKind::SearchEditor { query },
                title,
                pane,
            )
            .await
        }
        Command::OpenTerminalAt {
            project,
            pane,
            cwd,
            title,
        } => {
            open_tab(
                services,
                project,
                TabKind::Terminal {
                    session_id: String::new(),
                    cwd: Some(cwd),
                },
                title,
                pane,
            )
            .await
        }
        Command::CancelSearch(session) => {
            let _ = taide_runtime::search_actions::search_cancel(
                &services.state,
                &services.search,
                session.owner,
                session.id,
            )
            .await;
            None
        }
        Command::OpenSearchMatch {
            project,
            pane,
            open,
            viewport,
        } => Some(HostReply::ExplorerSearch(Reply::SearchMatchOpened {
            result: open_search_match(services, project, pane, open, viewport).await,
        })),
        Command::RecordRecentSearches(recent) => {
            let failure = crate::host::update_control_settings(
                services,
                &worker.integrations.reconcile,
                taide_model::settings::SettingsPatch {
                    recent_searches: Some(recent),
                    ..Default::default()
                },
            )
            .await;
            if let Some(reply) = failure
                && sender.send(reply).await.is_ok()
            {
                repaint();
            }
            Some(HostReply::ExplorerSearch(Reply::RecentSearchesSaved))
        }
    }
}

async fn open_tab(
    services: &taide_runtime::AppServices,
    project: ProjectId,
    kind: TabKind,
    title: String,
    pane: Option<PaneId>,
) -> Option<HostReply> {
    taide_runtime::layout_actions::layout_open_tab(
        services.events.as_ref(),
        &services.state,
        project,
        kind,
        title,
        pane,
        false,
    )
    .await
    .err()
    .map(HostReply::Failed)
}

async fn open_search_match(
    services: &taide_runtime::AppServices,
    project: ProjectId,
    pane: Option<PaneId>,
    open: crate::search_panel::Open,
    viewport: egui::ViewportId,
) -> AppResult<crate::terminal_tabs::OpenedFileLink> {
    let title = std::path::Path::new(&open.path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&open.path)
        .to_owned();
    let layout = taide_runtime::layout_actions::layout_open_tab(
        services.events.as_ref(),
        &services.state,
        project.clone(),
        TabKind::File {
            path: open.path.clone(),
        },
        title,
        pane.clone(),
        true,
    )
    .await?;
    let pane = pane.unwrap_or_else(|| layout.focused_pane.clone());
    let tab = taide_layout::service::all_roots(&layout)
        .find_map(|root| taide_native_ui::snapshot::active_tab(root, &pane))
        .filter(|tab| matches!(&tab.kind, TabKind::File { path } if path == &open.path))
        .ok_or_else(|| AppError::NotFound("native search match file is closed".into()))?
        .id
        .clone();
    Ok(crate::terminal_tabs::OpenedFileLink {
        project,
        pane,
        tab,
        path: open.path,
        line: f64::from(open.line),
        column: f64::from(open.column),
        viewport,
        layout,
    })
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
                            self.accept_explorer_tree(source.project, page);
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
                origin,
            } => match origin {
                RefreshOrigin::Watcher => match result {
                    Ok(page) => self.accept_explorer_tree(project, page),
                    Err(_) => {
                        self.explorer_search_state.tree_failures.insert(project);
                    }
                },
                RefreshOrigin::Toolbar => {
                    if let Ok(page) = result {
                        self.accept_explorer_tree(project, page);
                    }
                }
                RefreshOrigin::Delete => {
                    let failure = match result {
                        Ok(page) => {
                            self.accept_explorer_tree(project, page);
                            None
                        }
                        Err(error) => Some(error),
                    };
                    if let Some(error) = errors.into_iter().next().or(failure) {
                        self.report(&error);
                    }
                }
            },
            Reply::Tree {
                project,
                offset,
                result,
            } => {
                self.explorer_search_state.tree_loading.remove(&project);
                match result {
                    Ok(page) if offset == 0 => self.accept_explorer_tree(project, page),
                    Ok(page) => {
                        if let Some(current) = self.explorer_search_state.trees.get_mut(&project) {
                            current.rows.extend(page.rows);
                            current.total = page.total;
                        }
                    }
                    Err(_) if offset == 0 => {
                        self.explorer_search_state.tree_failures.insert(project);
                    }
                    Err(error) => self.status = Some(error.to_string()),
                }
            }
            Reply::TreeToggled {
                project,
                path,
                result,
            } => {
                for explorer in self
                    .explorer_search_state
                    .explorers
                    .of_project_mut(&project)
                {
                    explorer.toggle_finished(&path, result.as_ref().ok());
                }
                match result {
                    Ok(page) => self.accept_explorer_tree(project, page),
                    Err(error) => self.status = Some(error.to_string()),
                }
            }
            Reply::ExplorerPathRevealed {
                project,
                mount,
                path,
                rename,
                quiet,
                result,
            } => match result {
                Ok(page) => {
                    let explorer = self.explorer_search_state.explorers.entry(&project, &mount);
                    explorer.select_path(&page.rows, &path);
                    if rename && let Some(row) = page.rows.iter().find(|row| row.path == path) {
                        explorer.start_rename(row);
                    }
                    self.accept_explorer_tree(project, page);
                    context.request_repaint();
                }
                Err(_) if quiet => {}
                Err(error) => self.status = Some(error.to_string()),
            },
            Reply::ProjectReopened { closed, failure } => {
                self.forget_explorer_search_project(&closed);
                if let Some(error) = failure {
                    self.report(&error);
                }
                context.request_repaint();
            }
            Reply::SearchFinished {
                target,
                generation,
                result,
            } => {
                let state = &mut self.explorer_search_state;
                let failure = match &target {
                    SearchTarget::Sidebar(mount) => state
                        .symbol_sidebars
                        .get_mut(mount)
                        .and_then(|sidebar| sidebar.search.as_mut()),
                    SearchTarget::Editor(tab) => state.search_editors.get_mut(tab),
                }
                .and_then(|panel| panel.finish(&state.search_streams, generation, result));
                if let Some(error) = failure {
                    self.report(&error);
                }
                context.request_repaint();
            }
            Reply::SearchMatchOpened { result } => self.apply_file_link_reply(context, result),
            Reply::RecentSearchesSaved => self.explorer_search_state.search_history.settle(),
            Reply::SearchReplaced {
                mount,
                project,
                query,
                result,
            } => self.apply_search_replaced(context, mount, project, query, result),
        }
    }

    /// Reports a finished Replace All to its panel (`search-panel-container.tsx` 124-162): the
    /// summary toast, the skipped-file warning, and the re-run of the replaced query. A panel that
    /// is already gone takes no feedback, like the original's unmounted mutation callbacks.
    fn apply_search_replaced(
        &mut self,
        context: &egui::Context,
        mount: ShellSlotId,
        project: ProjectId,
        query: taide_model::search::SearchQuery,
        result: AppResult<taide_model::search::SearchReplaceResult>,
    ) {
        let state = &mut self.explorer_search_state;
        let Some(panel) = state
            .symbol_sidebars
            .get_mut(&mount)
            .and_then(|sidebar| sidebar.search.as_mut())
            .filter(|panel| panel.is_replacing())
        else {
            return;
        };
        let replaced = match result {
            Ok(replaced) => replaced,
            Err(error) => {
                panel.replace_finished(&state.search_streams, None);
                self.report(&error);
                return;
            }
        };
        let run = panel.replace_finished(&state.search_streams, Some(query));
        let now = Instant::now();
        let files = replaced.changed_files.to_string();
        let matches = replaced.replaced_matches.to_string();
        let summary = presentation::message(
            &self.locale,
            "search.replaceDone",
            &[("files", &files), ("matches", &matches)],
        );
        self.toasts.success(summary.clone(), now);
        self.notify_event(
            context,
            taide_model::notification::NotificationCategory::SearchReplace,
            Some(&project),
            "notification.searchReplaceDone",
            summary,
        );
        if let Some(report) = crate::search_panel::skip_report(&replaced, &self.locale) {
            self.toasts.notify(
                taide_native_ui::toast::Kind::Warning,
                report.title,
                taide_native_ui::toast::Options {
                    description: Some(report.description),
                    action: None,
                },
                now,
            );
        }
        if let Some(run) = run {
            self.submit(HostCommand::ExplorerSearch(Command::RunSearch {
                run,
                target: SearchTarget::Sidebar(mount),
                project,
                streams: self.explorer_search_state.search_streams.clone(),
            }));
        }
        context.request_repaint();
    }

    fn accept_explorer_tree(&mut self, project: ProjectId, page: TreeRowPage) {
        self.explorer_search_state.tree_failures.remove(&project);
        self.explorer_search_state.trees.insert(project, page);
    }

    /// Drops every explorer mount state, tree page and failure of a closed project.
    pub(crate) fn forget_explorer_search_project(&mut self, project: &ProjectId) {
        let state = &mut self.explorer_search_state;
        state.explorers.forget_project(project);
        state.trees.remove(project);
        state.tree_loading.remove(project);
        state.tree_failures.remove(project);
        state.git_decorations.forget(project);
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
                    quiet: false,
                }));
            }
            Intent::ReopenProject { project, root } => {
                self.close_project(project.clone());
                self.submit(HostCommand::ExplorerSearch(Command::ReopenProject {
                    project,
                    root,
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
                        quiet: false,
                    }));
                }
            }
            Intent::OpenSearch {
                project,
                slot,
                replace,
            } => {
                let request = self.search_request(context, snapshot, &project, replace);
                self.show_explorer_files(snapshot, &project, &slot);
                self.explorer_search_state
                    .symbol_sidebars
                    .entry(&project, &slot)
                    .request_search(request);
                context.request_repaint();
            }
            Intent::OpenAuxiliarySearch {
                project,
                slot,
                replace,
            } => {
                let request = self.search_request(context, snapshot, &project, replace);
                if let Some(explorer) = self.show_auxiliary_explorer_files(&project, slot) {
                    self.explorer_search_state
                        .symbol_sidebars
                        .entry(&project, &explorer)
                        .request_search(request);
                }
                context.request_repaint();
            }
        }
    }

    /// The request ⌘⇧F/⌘⇧H hand the search panel (`editor-area.tsx` 154-168): Find seeds the
    /// term with the focused editor's single-line primary selection, Replace only opens replace.
    fn search_request(
        &self,
        context: &egui::Context,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
        project: &ProjectId,
        replace: bool,
    ) -> crate::search_panel::Request {
        crate::search_panel::Request {
            seed_text: (!replace)
                .then(|| self.focused_selection_seed(context, snapshot, project))
                .flatten(),
            open_replace: replace,
            ..Default::default()
        }
    }

    fn focused_selection_seed(
        &self,
        context: &egui::Context,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
        project: &ProjectId,
    ) -> Option<String> {
        let layout = snapshot.layouts.get(project)?;
        let (root, pane) = crate::symbol_sidebar::window_tree(project, layout, &self.shell.scope)?;
        let tab = taide_native_ui::snapshot::active_tab(root, pane)?;
        let view = self.store.views().find(&ViewKey {
            window: WINDOW_LABEL.into(),
            pane: pane.clone(),
            tab: tab.id.clone(),
        })?;
        let body = crate::editor_context_menu::body(context, view)?;
        if !context.memory(|memory| memory.has_focus(body)) {
            return None;
        }
        let state = self.store.views().get(view)?;
        let selection = state.selection.selections.get(state.selection.primary)?;
        let document = self.store.documents().snapshot(state.document).ok()?;
        let text = document
            .rope
            .get_byte_slice(
                selection.anchor.min(selection.head)..selection.anchor.max(selection.head),
            )?
            .to_string();
        (!text.is_empty() && !text.contains('\n')).then_some(text)
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
                            self.accept_explorer_tree(event.project.clone(), pasted.page);
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
                        self.accept_explorer_tree(event.project.clone(), created.page);
                        if let Some(layout) = created.layout {
                            self.controller
                                .apply_layouts(HashMap::from([(event.project.clone(), layout)]));
                        }
                        if let Some(explorer) =
                            self.request_explorer(&event.project, event.request.mount.as_ref())
                        {
                            explorer.create_finished(&event.request, Ok(()));
                        }
                    }
                }
                Err(error) => self.report_create_failure(&event.project, &event.request, &error),
            },
            crate::lsp::Reply::ExplorerRenamed(event) => match event.result {
                Ok(page) => {
                    self.accept_explorer_tree(event.project.clone(), page);
                    if let Some(explorer) =
                        self.request_explorer(&event.project, event.request.mount.as_ref())
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
                                origin: RefreshOrigin::Delete,
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
        for explorer in self.explorer_search_state.explorers.all_mut() {
            explorer.moved(from, to);
        }
    }

    pub(crate) fn request_explorer(
        &mut self,
        project: &ProjectId,
        mount: Option<&ShellSlotId>,
    ) -> Option<&mut crate::explorer::Explorer> {
        self.explorer_search_state
            .explorers
            .get_mut(project, mount?)
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
        self.explorer_search_state
            .explorers
            .reconcile(snapshot.shell.tree.as_ref(), &auxiliary_explorers);
        let mounted = self.visible_explorer_files(snapshot);
        self.explorer_search_state.git_decorations.mount(mounted);
        let shown_tabs = snapshot
            .layouts
            .values()
            .flat_map(taide_layout::service::all_roots)
            .flat_map(taide_layout::service::collect_leaves)
            .filter_map(|leaf| match leaf {
                taide_model::layout::PaneNode::Leaf { active, .. } => active.clone(),
                taide_model::layout::PaneNode::Split { .. } => None,
            })
            .collect::<HashSet<_>>();
        let hidden = self
            .explorer_search_state
            .search_editors
            .unmount_hidden(&shown_tabs);
        for session in self
            .explorer_search_state
            .symbol_sidebars
            .take_cancelled()
            .into_iter()
            .chain(hidden)
        {
            self.explorer_search_state.search_streams.forget(&session);
            self.submit(HostCommand::ExplorerSearch(Command::CancelSearch(session)));
        }
    }

    fn visible_explorer_files(
        &self,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
    ) -> HashMap<ProjectId, String> {
        let is_files = |project: &ProjectId, mount: &ShellSlotId| {
            self.explorer_search_state
                .symbol_sidebars
                .view(project, mount)
                == crate::symbol_sidebar::View::Files
        };
        let main = snapshot
            .shell
            .tree
            .iter()
            .filter(|_| !snapshot.shell.window_chrome.zen)
            .flat_map(slot_projects)
            .filter(|(mount, project)| {
                snapshot
                    .layouts
                    .get(*project)
                    .is_some_and(|layout| !layout.shell_view.sidebar_collapsed)
                    && is_files(project, mount)
            })
            .map(|(_, project)| project.clone());
        let auxiliary = self
            .shell_state
            .auxiliary_windows
            .windows()
            .iter()
            .filter(|window| window.is_explorer_open && is_files(&window.project, &window.explorer))
            .map(|window| window.project.clone());
        main.chain(auxiliary)
            .filter_map(|project| {
                let root = snapshot
                    .projects
                    .iter()
                    .find(|entry| entry.id == project)?
                    .root
                    .clone();
                Some((project, root))
            })
            .collect()
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
                origin: RefreshOrigin::Watcher,
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
            if let Some(explorer) = self.request_explorer(&project, request.mount.as_ref()) {
                explorer.create_finished(
                    &request,
                    Err("native dirty tab update is unavailable".into()),
                );
            }
        }
        for (project, request) in rename_entries {
            if let Some(explorer) = self.request_explorer(&project, request.mount.as_ref()) {
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
        if let Some(explorer) = self.request_explorer(project, request.mount.as_ref()) {
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
        if let Some(explorer) = self.request_explorer(project, request.mount.as_ref()) {
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

fn slot_projects(tree: &taide_model::project::ShellSlotTree) -> Vec<(&ShellSlotId, &ProjectId)> {
    match tree {
        taide_model::project::ShellSlotTree::Leaf {
            slot_id,
            project_id,
        } => vec![(slot_id, project_id)],
        taide_model::project::ShellSlotTree::Split { children, .. } => {
            children.iter().flat_map(slot_projects).collect()
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
    auto_reveal: Option<String>,
    window_label: String,
    search_run: Option<crate::search_panel::Run>,
    search_open: Option<crate::search_panel::Open>,
    search_cancel: Option<crate::search_stream::Session>,
    search_replace: Option<crate::search_panel::Replace>,
    search_editor: Option<taide_model::search::SearchQuery>,
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
            taide_native_ui::shell::ExplorerMount::Slot(slot) => {
                Some((slot.clone(), WINDOW_LABEL.to_owned()))
            }
            taide_native_ui::shell::ExplorerMount::Auxiliary(slot) => self
                .auxiliary_windows
                .find(project, slot)
                .map(|window| (window.explorer.clone(), window.label.clone())),
        };
        let Some((slot, window_label)) = slot else {
            return ExplorerRegion::default();
        };
        let mut region = ExplorerRegion {
            sidebar: Some(self.symbol_sidebars.take(project, &slot)),
            explorer: self.explorers.take(project, &slot),
            window_label,
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
            auto_reveal,
            window_label,
            search_run,
            search_open,
            search_cancel,
            search_replace,
            search_editor,
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
        let this_window_pane = || match self.scope {
            WindowScope::Main => None,
            WindowScope::Auxiliary { .. } => self
                .layouts
                .get(project)
                .and_then(|layout| crate::symbol_sidebar::window_tree(project, layout, self.scope))
                .map(|(_, focused)| focused.clone()),
        };
        if sidebar.view != crate::symbol_sidebar::View::Search
            && let Some(panel) = sidebar.search.take()
        {
            *search_cancel = panel.running_session();
        }
        if sidebar.view == crate::symbol_sidebar::View::Search {
            let panel = sidebar.search.get_or_insert_with(|| {
                crate::search_panel::Panel::new(crate::search_stream::Session {
                    owner: window_label.clone(),
                    id: format!("search-panel-{slot}"),
                })
            });
            let (settings, history) = {
                let settings = self.services.state.settings.read();
                (
                    crate::search_panel::Settings {
                        is_live: settings.search_on_type,
                        debounce_ms: settings.search_on_type_debounce_ms,
                    },
                    self.explorer_inputs
                        .search_history
                        .list(&settings.recent_searches)
                        .to_vec(),
                )
            };
            match panel.show(
                ui,
                egui::Id::new(("search-panel", project, slot)),
                crate::search_panel::Inputs {
                    request: sidebar.search_request.as_ref(),
                    nonce: sidebar.open_nonce,
                    settings,
                    history: &history,
                    streams: self.explorer_inputs.search_streams,
                    appearance: self.explorer_appearance,
                    locale: self.locale,
                },
                &mut *icons,
            ) {
                Ok(output) => {
                    for (response, text) in &output.tooltips {
                        self.tooltips.show(
                            response,
                            text,
                            egui::RectAlign::BOTTOM,
                            self.tooltip_appearance,
                        );
                    }
                    if output.clear_scope {
                        sidebar.search_request = sidebar
                            .search_request
                            .as_ref()
                            .map(crate::search_panel::Request::without_scope);
                    }
                    *search_run = output.run;
                    *search_open = output.open;
                    *search_replace = output.replace;
                    *search_editor = output.open_editor;
                    *window_pane = this_window_pane();
                }
                Err(error) => errors.push(error.to_string()),
            }
            return;
        }
        if sidebar.view == crate::symbol_sidebar::View::Git {
            self.show_git_view(ui, project, slot, &mut *icons, errors, intents);
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
        let entry = self.projects.iter().find(|entry| &entry.id == project);
        let is_unavailable = entry.is_some_and(|entry| entry.root_missing)
            || self.explorer_inputs.tree_failures.contains(project);
        let page = self.trees.get(project).filter(|_| !is_unavailable);
        if page.is_none() && !is_unavailable {
            if !self.tree_loading.contains(project) {
                *is_tree_requested = true;
            }
            ui.spinner();
            return;
        }
        let explorer = explorer.get_or_insert_with(Default::default);
        if let Some(entry) = entry {
            explorer.root = Some(entry.root.clone());
            explorer.title = Some(entry.name.clone());
        }
        let row_icons = crate::explorer::RowIcons {
            glyphs: icons,
            appearance: self.explorer_appearance,
            decorations: self.explorer_inputs.git_decorations.get(project),
        };
        let Some(page) = page else {
            let output = explorer.show_missing(
                ui,
                project,
                entry.map(|entry| entry.root.as_str()),
                self.locale,
                row_icons,
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
            if let Some(entry) = entry
                && output.reopen.is_some_and(|reopen| reopen.clicked())
            {
                intents.push(ShellIntent::ExplorerSearch(Intent::ReopenProject {
                    project: project.clone(),
                    root: entry.root.clone(),
                }));
            }
            *actions = output.actions;
            return;
        };
        explorer.replace_clipboard(self.explorer_clipboards.clipboard(owner).cloned());
        if let Some(path) = self.explorer_clipboards.reveal(owner) {
            explorer.paste_finished(false, Some(path.clone()));
            *is_reveal_taken = true;
        }
        let active_path = self
            .layouts
            .get(project)
            .and_then(|layout| crate::symbol_sidebar::window_tree(project, layout, self.scope))
            .and_then(|(root, focused)| taide_native_ui::snapshot::active_tab(root, focused))
            .and_then(|tab| match &tab.kind {
                TabKind::File { path } => Some(path.as_str()),
                _ => None,
            })
            .filter(|path| {
                entry.is_some_and(|entry| {
                    path.strip_prefix(entry.root.as_str())
                        .is_some_and(|rest| rest.starts_with('/'))
                })
            });
        match crate::explorer_auto_reveal::decide(crate::explorer_auto_reveal::Input {
            is_enabled: self.explorer_inputs.is_auto_reveal_enabled,
            active_path,
            rows: &page.rows,
            is_sidebar_visible: true,
            is_files_view: true,
            last_revealed: explorer.auto_revealed.as_deref(),
        }) {
            crate::explorer_auto_reveal::Decision::Skip => {}
            crate::explorer_auto_reveal::Decision::SelectOnly => {
                if let Some(path) = active_path {
                    explorer.auto_revealed = Some(path.to_owned());
                    explorer.select_path(&page.rows, path);
                }
            }
            crate::explorer_auto_reveal::Decision::RevealThenSelect => {
                explorer.auto_revealed = active_path.map(str::to_owned);
                *auto_reveal = explorer.auto_revealed.clone();
            }
        }
        let output = explorer.show_with_icons(ui, project, slot, page, self.locale, row_icons);
        if let Some(error) = &output.icon_error {
            errors.push(error.to_string());
        }
        crate::explorer_toolbar::show_tooltips(
            &output,
            self.locale,
            self.tooltips,
            self.tooltip_appearance,
        );
        *window_pane = this_window_pane();
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
            auto_reveal,
            window_label: _,
            search_run,
            search_open,
            search_cancel,
            search_replace,
            search_editor,
        } = region;
        let Some(slot) = mount else {
            return;
        };
        if let Some(replace) = search_replace {
            self.commands
                .push(HostCommand::ExplorerSearch(Command::ReplaceSearch {
                    mount: slot.clone(),
                    project: project.clone(),
                    replace,
                }));
        }
        if let Some(query) = search_editor {
            let title = if query.text.is_empty() {
                presentation::message(self.locale, "searchEditor.title", &[])
            } else {
                query.text.clone()
            };
            self.commands
                .push(HostCommand::ExplorerSearch(Command::OpenSearchEditor {
                    project: project.clone(),
                    pane: window_pane.clone(),
                    query,
                    title,
                }));
        }
        if let Some(session) = search_cancel {
            self.explorer_inputs.search_streams.forget(&session);
            self.commands
                .push(HostCommand::ExplorerSearch(Command::CancelSearch(session)));
        }
        if let Some(run) = search_run {
            self.submit_search_run(run, SearchTarget::Sidebar(slot.clone()), project);
        }
        if let Some(open) = search_open {
            self.commands
                .push(HostCommand::ExplorerSearch(Command::OpenSearchMatch {
                    project: project.clone(),
                    pane: window_pane.clone(),
                    open,
                    viewport: context.viewport_id(),
                }));
        }
        if let Some(sidebar) = sidebar {
            self.symbol_sidebars.put(slot.clone(), sidebar);
        }
        if let Some(explorer) = explorer {
            self.explorers.put(project, slot.clone(), explorer);
        }
        if let Some(path) = auto_reveal {
            self.commands
                .push(HostCommand::ExplorerSearch(Command::RevealExplorerPath {
                    project: project.clone(),
                    mount: slot.clone(),
                    path,
                    rename: false,
                    quiet: true,
                }));
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
                crate::explorer::Action::OpenInTerminal(row) => {
                    let cwd = if row.kind == taide_model::tree::TreeEntryKind::Directory {
                        Some(row.path)
                    } else {
                        std::path::Path::new(&row.path)
                            .parent()
                            .and_then(|parent| parent.to_str())
                            .map(str::to_owned)
                    };
                    let root = self
                        .projects
                        .iter()
                        .find(|entry| &entry.id == project)
                        .map(|entry| entry.root.clone());
                    if let Some(cwd) = cwd.or(root) {
                        self.commands
                            .push(HostCommand::ExplorerSearch(Command::OpenTerminalAt {
                                project: project.clone(),
                                pane: window_pane.clone(),
                                cwd,
                                title: presentation::message(self.locale, "terminal.title", &[]),
                            }));
                    }
                }
                crate::explorer::Action::FindInFolder(scope_dir) => self
                    .symbol_sidebars
                    .entry(project, &slot)
                    .request_search(crate::search_panel::Request {
                        scope_dir: Some(scope_dir),
                        ..Default::default()
                    }),
                crate::explorer::Action::Refresh => {
                    self.commands
                        .push(HostCommand::ExplorerSearch(Command::RefreshTree {
                            project: project.clone(),
                            dirs: None,
                            origin: RefreshOrigin::Toolbar,
                        }))
                }
                crate::explorer::Action::Collapse => {
                    self.commands
                        .push(HostCommand::ExplorerSearch(Command::TreeCollapse(
                            project.clone(),
                        )))
                }
                crate::explorer::Action::SelectForCompare(path) => {
                    if let Some(explorer) = self.explorers.get_mut(project, &slot) {
                        explorer.compare_source = Some(path);
                    }
                }
                crate::explorer::Action::CompareWithSelected(path) => {
                    if let Some(command) = self
                        .explorers
                        .get_mut(project, &slot)
                        .and_then(|explorer| explorer.compare_source.take())
                        .map(|source| {
                            crate::domains::git::compare_tab(
                                project.clone(),
                                source,
                                path,
                                window_pane.clone(),
                            )
                        })
                    {
                        self.commands.push(command);
                    }
                }
                crate::explorer::Action::FileHistory(_) => {}
            }
        }
        if is_files_shown && let Some(explorer) = self.explorers.get(&slot) {
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
        for session in region.search_cancel.into_iter().chain(
            region
                .sidebar
                .as_ref()
                .and_then(|sidebar| sidebar.running_search()),
        ) {
            self.explorer_inputs.search_streams.forget(&session);
            self.commands
                .push(HostCommand::ExplorerSearch(Command::CancelSearch(session)));
        }
        self.explorer_output
            .discarded_explorers
            .push(project.clone());
        self.explorer_clipboards.unmount(&slot);
        if let Some(explorer) = region.explorer {
            self.explorers.put(project, slot, explorer.remounted());
        }
    }
}

impl AppSurfaces<'_> {
    /// Records an explicit run's term in the recent searches, then starts the run.
    fn submit_search_run(
        &mut self,
        run: crate::search_panel::Run,
        target: SearchTarget,
        project: &ProjectId,
    ) {
        if let Some(term) = &run.record {
            let settled = self.services.state.settings.read().recent_searches.clone();
            if let Some(recent) = self.explorer_inputs.search_history.enqueue(&settled, term) {
                self.commands
                    .push(HostCommand::ExplorerSearch(Command::RecordRecentSearches(
                        recent,
                    )));
            }
        }
        self.commands
            .push(HostCommand::ExplorerSearch(Command::RunSearch {
                run,
                target,
                project: project.clone(),
                streams: self.explorer_inputs.search_streams.clone(),
            }));
    }

    /// The search editor tab body (`search-editor-pane.tsx`): the remembered panel for this tab,
    /// or one started from the tab's query that runs it once; matches open as a preview in the
    /// focused pane of this window.
    pub(crate) fn show_search_editor_tab(&mut self, ui: &mut Ui, tab: &Tab) {
        let TabKind::SearchEditor { query } = &tab.kind else {
            self.show_unavailable_tab(ui, tab);
            return;
        };
        let Some(project) = self.layouts.iter().find_map(|(project, layout)| {
            taide_layout::service::all_roots(layout)
                .any(|root| taide_layout::service::find_tab(root, &tab.id).is_some())
                .then(|| project.clone())
        }) else {
            self.show_unavailable_tab(ui, tab);
            return;
        };
        let (owner, window_pane) = match self.scope {
            WindowScope::Main => (WINDOW_LABEL.to_owned(), None),
            WindowScope::Auxiliary { project, slot } => (
                self.auxiliary_windows
                    .find(project, *slot)
                    .map_or_else(|| WINDOW_LABEL.to_owned(), |window| window.label.clone()),
                self.layouts
                    .get(project)
                    .and_then(|layout| {
                        crate::symbol_sidebar::window_tree(project, layout, self.scope)
                    })
                    .map(|(_, focused)| focused.clone()),
            ),
        };
        let request = crate::search_panel::Request {
            include_glob: query.include_glob.clone(),
            scope_dir: query.scope_dir.clone(),
            ..Default::default()
        };
        let history = {
            let settings = self.services.state.settings.read();
            self.explorer_inputs
                .search_history
                .list(&settings.recent_searches)
                .to_vec()
        };
        let streams = self.explorer_inputs.search_streams.clone();
        let mounted = self.explorer_inputs.search_editors.mount(
            &tab.id,
            &project,
            || crate::search_stream::Session {
                owner,
                id: tab.id.to_string(),
            },
            query,
        );
        let auto_run = mounted
            .is_created
            .then(|| mounted.panel.auto_run(&streams, Some(&request)))
            .flatten();
        let mut icons = crate::navigation_icons::Deferred::default();
        let result = mounted.panel.show(
            ui,
            egui::Id::new(("search-editor", &tab.id)),
            crate::search_panel::Inputs {
                request: Some(&request),
                nonce: mounted.nonce,
                settings: crate::search_panel::Settings {
                    is_live: false,
                    debounce_ms: 0,
                },
                history: &history,
                streams: &streams,
                appearance: self.explorer_appearance,
                locale: self.locale,
            },
            &mut icons,
        );
        if let Err(error) = icons.fill(&mut *self.explorer_icons, &mut *self.navigation_icons) {
            *self.status = Some(error.to_string());
        }
        let output = match result {
            Ok(output) => output,
            Err(error) => {
                *self.status = Some(error.to_string());
                return;
            }
        };
        for (response, text) in &output.tooltips {
            self.tooltips.show(
                response,
                text,
                egui::RectAlign::BOTTOM,
                self.tooltip_appearance,
            );
        }
        let target = SearchTarget::Editor(tab.id.clone());
        for run in auto_run.into_iter().chain(output.run) {
            self.submit_search_run(run, target.clone(), &project);
        }
        if let Some(open) = output.open {
            self.commands
                .push(HostCommand::ExplorerSearch(Command::OpenSearchMatch {
                    project,
                    pane: window_pane,
                    open,
                    viewport: ui.ctx().viewport_id(),
                }));
        }
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

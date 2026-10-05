use std::future::Future;
use std::sync::Arc;

use taide_model::{
    error::{AppError, AppResult},
    ids::{PaneId, ProjectId, TabId},
    layout::{DropEdge, PaneNode, ProjectLayout, Tab, TabKind},
};
use taide_native_terminal::Size;
use taide_runtime::{AppServices, layout_actions, terminal_actions};

use crate::{terminal_dispatch::EffectPorts, terminal_host::Hub};

const REPLAY_BYTES_PER_LINE: u32 = 512;

#[derive(Clone)]
pub struct MenuTarget {
    pub project: ProjectId,
    pub pane: PaneId,
    pub tab: TabId,
    pub session: String,
}

pub enum MenuOperation {
    Split(DropEdge),
    New,
    Kill,
}

pub struct OpenedFileLink {
    pub project: ProjectId,
    pub pane: PaneId,
    pub tab: TabId,
    pub path: String,
    pub line: f64,
    pub column: f64,
    pub viewport: eframe::egui::ViewportId,
    pub layout: ProjectLayout,
}

pub(crate) fn menu_tab<'layout>(
    layout: &'layout ProjectLayout,
    target: &MenuTarget,
) -> Option<&'layout Tab> {
    std::iter::once(&layout.root)
        .chain(layout.auxiliary_windows.iter().map(|window| &window.root))
        .filter_map(|root| taide_layout::service::find_leaf(root, &target.pane))
        .find_map(|leaf| match leaf {
            PaneNode::Leaf { tabs, active, .. } if active.as_ref() == Some(&target.tab) => tabs.iter().find(|tab|
                tab.id == target.tab && matches!(&tab.kind, TabKind::Terminal { session_id, .. } if session_id == &target.session)),
            _ => None,
        })
}

pub struct Tabs {
    hub: Arc<Hub>,
    services: Arc<AppServices>,
    starting: tokio::sync::Mutex<()>,
}

struct PendingSession {
    hub: Arc<Hub>,
    id: Option<String>,
}

impl Drop for PendingSession {
    fn drop(&mut self) {
        if let Some(id) = &self.id {
            self.hub.discard(id);
        }
    }
}

impl Tabs {
    pub fn new(
        services: Arc<AppServices>,
        limits: crate::terminal_host::Limits,
    ) -> AppResult<Self> {
        Ok(Self {
            hub: Arc::new(Hub::new(services.clone(), limits)?),
            services,
            starting: tokio::sync::Mutex::new(()),
        })
    }

    pub fn hub(&self) -> &Arc<Hub> {
        &self.hub
    }

    pub async fn open_file_link(
        &self,
        owner: crate::terminal_surface::PasteTarget,
        source: MenuTarget,
        link: crate::terminal_file_links::ResolvedFileLink,
    ) -> AppResult<OpenedFileLink> {
        let guard = self.services.state.begin_owned_mutation().await;
        let services = self.services.clone();
        let hub = self.hub.clone();
        self.services
            .tasks
            .run_blocking_result("native-terminal-open-file-link", move || {
                let _guard = guard;
                crate::host::authorize_terminal_link(&services, Some(&hub), &owner, &source)?;
                if link.path.len() > crate::terminal_links::MAX_LINK_BYTES {
                    return Err(AppError::InvalidArgument(
                        "native terminal file link exceeds its limit".into(),
                    ));
                }
                let projects = services.state.projects.read().clone();
                let (_, resolved) = taide_infra::root_guard::resolve_owning_project_or_cli_opened(
                    &projects,
                    &services.state.cli_opened_paths.read(),
                    std::path::Path::new(&link.path),
                )?;
                taide_infra::root_guard::ensure_existing_file(&resolved, &link.path)?;
                let mut layouts = services.state.layouts.read().clone();
                let layout = layouts.get_mut(&source.project).ok_or_else(|| {
                    AppError::NotFound("native terminal file link project is closed".into())
                })?;
                let pane = if taide_layout::service::find_leaf(&layout.root, &source.pane).is_some()
                {
                    layout.focused_pane.clone()
                } else {
                    layout
                        .auxiliary_windows
                        .iter()
                        .find(|window| {
                            taide_layout::service::find_leaf(&window.root, &source.pane).is_some()
                        })
                        .map(|window| window.focused_pane.clone())
                        .ok_or_else(|| {
                            AppError::NotFound("native terminal file link window is closed".into())
                        })?
                };
                let preview = services.state.settings.read().enable_preview_tabs;
                let title = std::path::Path::new(&link.path)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let tab = taide_layout::service::open_tab(
                    layout,
                    &pane,
                    Tab {
                        id: TabId::new(),
                        kind: TabKind::File {
                            path: link.path.clone(),
                        },
                        title,
                        pinned: false,
                        preview: false,
                        dirty: false,
                        view_state: None,
                    },
                    preview,
                )?;
                let updated = layout_actions::finish_mutation(
                    services.events.as_ref(),
                    &services.state,
                    &source.project,
                    layout,
                );
                *services.state.layouts.write() = layouts;
                Ok(OpenedFileLink {
                    project: source.project,
                    pane,
                    tab,
                    path: link.path,
                    line: link.matched.line.unwrap_or(1.0),
                    column: link.matched.column.unwrap_or(1.0),
                    viewport: owner.viewport(),
                    layout: updated,
                })
            })
            .await
    }

    pub async fn menu(
        &self,
        target: MenuTarget,
        operation: MenuOperation,
        title: String,
    ) -> AppResult<Option<crate::tabs::ClosedNativeTab>> {
        let _operation = self
            .services
            .tasks
            .begin_operation("native-terminal-menu")
            .ok_or_else(|| AppError::Forbidden("native terminal menu is stopping".into()))?;
        let guard = self.services.state.begin_owned_mutation().await;
        let services = self.services.clone();
        let live_cwd = self
            .hub
            .get(&target.session)
            .map(|session| {
                if session.metadata().project_id() != &target.project {
                    return Err(AppError::Forbidden(
                        "native terminal menu project changed".into(),
                    ));
                }
                Ok(session.metadata().cwd())
            })
            .transpose()?;
        let closed = self
            .services
            .tasks
            .run_blocking_result("native-terminal-menu", move || {
                let _guard = guard;
                if services.state.is_shutting_down() {
                    return Err(AppError::Forbidden(
                        "native terminal menu is stopping".into(),
                    ));
                }
                let root = services
                    .state
                    .projects
                    .read()
                    .get(&target.project)
                    .map(|project| project.root.clone())
                    .ok_or_else(|| {
                        AppError::NotFound("native terminal menu project is closed".into())
                    })?;
                let mut layouts = services.state.layouts.read().clone();
                let layout = layouts.get_mut(&target.project).ok_or_else(|| {
                    AppError::NotFound("native terminal menu layout is closed".into())
                })?;
                let candidate = menu_tab(layout, &target).ok_or_else(|| {
                    AppError::Forbidden("native terminal menu target changed".into())
                })?;
                let TabKind::Terminal { cwd: tab_cwd, .. } = &candidate.kind else {
                    return Err(AppError::InvalidArgument(
                        "native terminal menu target is not terminal".into(),
                    ));
                };
                let closed = match operation {
                    MenuOperation::Kill => {
                        if candidate.pinned {
                            return Err(AppError::Forbidden("pinned tab cannot close".into()));
                        }
                        let closed = taide_layout::service::close_tab(layout, &target.tab)?;
                        Some(crate::tabs::ClosedNativeTab {
                            project: target.project.clone(),
                            tab: closed.tab,
                            has_remaining_file: false,
                            discarded: false,
                        })
                    }
                    MenuOperation::New => {
                        taide_layout::service::open_tab(
                            layout,
                            &target.pane,
                            Tab {
                                id: TabId::new(),
                                kind: TabKind::Terminal {
                                    session_id: String::new(),
                                    cwd: None,
                                },
                                title,
                                pinned: false,
                                preview: false,
                                dirty: false,
                                view_state: None,
                            },
                            false,
                        )?;
                        None
                    }
                    MenuOperation::Split(edge) => {
                        if edge == DropEdge::Center {
                            return Err(AppError::InvalidArgument(
                                "native terminal split edge is invalid".into(),
                            ));
                        }
                        let cwd = live_cwd
                            .or_else(|| services.terminal.cwd(&target.session))
                            .or_else(|| tab_cwd.clone())
                            .filter(|cwd| !cwd.is_empty())
                            .and_then(|cwd| {
                                taide_infra::root_guard::ensure_within_root(
                                    std::path::Path::new(&root),
                                    std::path::Path::new(&cwd),
                                )
                                .ok()
                            })
                            .map(|cwd| cwd.to_string_lossy().into_owned());
                        taide_layout::service::open_tab_in_split(
                            layout,
                            &target.pane,
                            edge,
                            Tab {
                                id: TabId::new(),
                                kind: TabKind::Terminal {
                                    session_id: String::new(),
                                    cwd,
                                },
                                title,
                                pinned: false,
                                preview: false,
                                dirty: false,
                                view_state: None,
                            },
                        )?;
                        None
                    }
                };
                layout_actions::finish_mutation(
                    services.events.as_ref(),
                    &services.state,
                    &target.project,
                    layout,
                );
                *services.state.layouts.write() = layouts;
                Ok(closed)
            })
            .await?;
        if let Some(closed) = &closed
            && let TabKind::Terminal { session_id, .. } = &closed.tab.kind
        {
            self.close(session_id).await?;
        }
        Ok(closed)
    }

    pub async fn attach<E>(
        &self,
        tab: TabId,
        size: Size,
        extra_env: E,
        ports: EffectPorts,
    ) -> AppResult<String>
    where
        E: Future<Output = Vec<(String, String)>>,
    {
        let _starting = self.starting.lock().await;
        self.attach_inner(tab, size, extra_env, ports).await
    }

    pub async fn restart<E>(
        &self,
        tab: TabId,
        size: Size,
        extra_env: E,
        ports: EffectPorts,
    ) -> AppResult<String>
    where
        E: Future<Output = Vec<(String, String)>>,
    {
        let _starting = self.starting.lock().await;
        let (project, _, prior) = self.target(&tab)?;
        if let Some(session) = self.hub.get(&prior) {
            if session.metadata().project_id() != &project {
                return Err(AppError::Forbidden(
                    "native terminal belongs to another project".into(),
                ));
            }
            if session
                .snapshot(|state| state.phase == taide_native_terminal::session::Phase::Running)?
                && session.failure().is_none()
            {
                return Err(AppError::Forbidden(
                    "native running terminal cannot be restarted".into(),
                ));
            }
            let result = self.hub.close(&prior).await;
            if let Err(error) = result
                && !(session.failure().is_some() && session.is_finished())
            {
                return Err(error);
            }
        }
        self.attach_inner(tab, size, extra_env, ports).await
    }

    fn target(&self, tab: &TabId) -> AppResult<(ProjectId, Option<String>, String)> {
        let layouts = self.services.state.layouts.read();
        let (project, candidate) = layouts
            .iter()
            .find_map(|(project, layout)| {
                crate::tabs::tabs_in(&layout.root)
                    .into_iter()
                    .chain(
                        layout
                            .auxiliary_windows
                            .iter()
                            .flat_map(|window| crate::tabs::tabs_in(&window.root)),
                    )
                    .find(|candidate| &candidate.id == tab)
                    .map(|candidate| (project, candidate))
            })
            .ok_or_else(|| AppError::NotFound("native terminal tab no longer exists".into()))?;
        let TabKind::Terminal { session_id, cwd } = &candidate.kind else {
            return Err(AppError::InvalidArgument(
                "native tab is not a terminal".into(),
            ));
        };
        Ok((project.clone(), cwd.clone(), session_id.clone()))
    }

    async fn attach_inner<E>(
        &self,
        tab: TabId,
        size: Size,
        extra_env: E,
        ports: EffectPorts,
    ) -> AppResult<String>
    where
        E: Future<Output = Vec<(String, String)>>,
    {
        let (project, cwd, prior) = self.target(&tab)?;
        if let Some(session) = self.hub.get(&prior) {
            if session.metadata().project_id() != &project {
                return Err(AppError::Forbidden(
                    "native terminal belongs to another project".into(),
                ));
            }
            if session.failure().is_some() {
                return Err(AppError::Forbidden("native terminal session failed".into()));
            }
            return Ok(prior);
        }
        let owner = project.clone();
        let mut options =
            terminal_actions::pty_default_options(&self.services.state, project, cwd).await?;
        options.cols = size.columns;
        options.rows = size.rows;
        let history = self.services.state.settings.read().terminal_scrollback;
        options.scrollback_bytes = Some(history.saturating_mul(REPLAY_BYTES_PER_LINE));
        let id = self
            .hub
            .spawn(options, history as usize, extra_env, ports)
            .await?;
        let mut pending = PendingSession {
            hub: self.hub.clone(),
            id: Some(id.clone()),
        };
        let _guard = self.services.state.begin_owned_mutation().await;
        if self.services.state.is_shutting_down() {
            return Err(AppError::Forbidden(
                "native terminal tab is stopping".into(),
            ));
        }
        let mut layouts = self.services.state.layouts.read().clone();
        let project = taide_layout::service::locate_project_with_tab(&layouts, &tab)?;
        if project != owner {
            return Err(AppError::Forbidden(
                "native terminal tab changed project during spawn".into(),
            ));
        }
        let layout = taide_layout::service::get_layout_mut(&mut layouts, &project)?;
        let candidate = crate::tabs::tabs_in(&layout.root)
            .into_iter()
            .chain(
                layout
                    .auxiliary_windows
                    .iter()
                    .flat_map(|window| crate::tabs::tabs_in(&window.root)),
            )
            .find(|candidate| candidate.id == tab)
            .ok_or_else(|| AppError::NotFound("native terminal tab no longer exists".into()))?;
        if !matches!(&candidate.kind, TabKind::Terminal { session_id, .. } if session_id == &prior)
        {
            return Err(AppError::Forbidden(
                "native terminal tab changed during spawn".into(),
            ));
        }
        if !self.services.state.projects.read().contains_key(&project) {
            return Err(AppError::NotFound(
                "native terminal project is closed".into(),
            ));
        }
        taide_layout::service::set_terminal_session(layout, &tab, id.clone())?;
        layout_actions::finish_mutation(
            self.services.events.as_ref(),
            &self.services.state,
            &project,
            layout,
        );
        *self.services.state.layouts.write() = layouts;
        pending.id = None;
        Ok(id)
    }

    pub async fn close(&self, id: &str) -> AppResult<()> {
        if self.hub.get(id).is_none() {
            return Ok(());
        }
        self.hub.close(id).await
    }
}

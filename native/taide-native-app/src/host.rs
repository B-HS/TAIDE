use std::sync::Arc;

use taide_model::error::{AppError, AppResult};
use taide_model::ids::{PaneId, ProjectId};
use taide_model::layout::TabKind;
use taide_runtime::{AppServices, layout_actions};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub const HOST_COMMAND_CAPACITY: usize = 64;

pub type ClipboardWriter = Arc<dyn Fn(&str) -> AppResult<()> + Send + Sync>;
pub type ClipboardReader = Arc<dyn Fn() -> AppResult<String> + Send + Sync>;

pub enum HostCommand {
    Shell(crate::application::domains::shell::Command),
    Commands(crate::application::domains::commands::Command),
    Editor(crate::application::domains::editor::Command),
    ExplorerSearch(crate::application::domains::explorer_search::Command),
    Git(crate::application::domains::git::Command),
    TerminalAgent(crate::application::domains::terminal_agent::Command),
    Settings(crate::application::domains::settings::Command),
    Preview(crate::application::domains::preview::Command),
}

pub enum HostReply {
    Shell(crate::application::domains::shell::Reply),
    Commands(crate::application::domains::commands::Reply),
    Editor(crate::application::domains::editor::Reply),
    ExplorerSearch(crate::application::domains::explorer_search::Reply),
    Git(crate::application::domains::git::Reply),
    TerminalAgent(crate::application::domains::terminal_agent::Reply),
    Settings(crate::application::domains::settings::Reply),
    Preview(crate::application::domains::preview::Reply),
    SettingsFailed(AppError),
    Failed(AppError),
}

pub struct HostBridge {
    state: taide_runtime::AppState,
    commands: mpsc::Sender<HostCommand>,
    replies: mpsc::Receiver<HostReply>,
    worker: JoinHandle<()>,
}

pub struct Terminals {
    pub tabs: Arc<crate::terminal_tabs::Tabs>,
    pub environment: crate::terminal_environment::Environment,
}

pub(crate) struct HostIntegrations {
    view_states: Option<Arc<crate::editor_view_state_sync::Shared>>,
    pub(crate) terminals: Option<Terminals>,
    pub(crate) reconcile: Option<crate::remote_preferences::Reconcile>,
    pub(crate) wait_markers: Arc<crate::external_open::WaitMarkers>,
    pub(crate) agent_cli: Arc<crate::agent_cli::Ports>,
}

pub(crate) struct HostWorker<'a> {
    pub(crate) services: &'a Arc<AppServices>,
    pub(crate) clipboard: &'a ClipboardWriter,
    pub(crate) read_clipboard: &'a ClipboardReader,
    pub(crate) sender: &'a mpsc::Sender<HostReply>,
    pub(crate) repaint: &'a Arc<dyn Fn() + Send + Sync>,
    pub(crate) integrations: &'a HostIntegrations,
}

impl HostIntegrations {
    pub(crate) async fn flush_view_states(&self, services: &AppServices) -> AppResult<()> {
        if let Some(view_states) = &self.view_states {
            view_states.flush(services).await?;
        }
        Ok(())
    }

    /// The project lifecycle the remote `project_close` uses, so a close also discards the
    /// project's terminal tabs.
    pub(crate) fn projects(&self, services: &Arc<AppServices>) -> crate::projects::NativeProjects {
        let projects = crate::projects::NativeProjects::new(services.clone());
        match &self.terminals {
            Some(terminals) => projects.with_terminals(terminals.tabs.hub().clone()),
            None => projects,
        }
    }
}

impl HostBridge {
    pub fn connect(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
    ) -> AppResult<Self> {
        Self::connect_with_clipboard(services, repaint, clipboard_writer())
    }

    pub fn connect_with_clipboard(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
        clipboard: ClipboardWriter,
    ) -> AppResult<Self> {
        Self::connect_with_ports(services, repaint, clipboard, None)
    }

    pub fn connect_with_terminals(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
        terminals: Terminals,
    ) -> AppResult<Self> {
        Self::connect_with_ports(services, repaint, clipboard_writer(), Some(terminals))
    }

    pub fn connect_with_application_ports(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
        terminals: Terminals,
        ports: &crate::application_ports::Ports,
    ) -> AppResult<Self> {
        Self::connect_with_integrations(
            services,
            repaint,
            clipboard_writer(),
            clipboard_reader(),
            HostIntegrations {
                terminals: Some(terminals),
                reconcile: Some(ports.reconcile.clone()),
                view_states: Some(ports.view_states.clone()),
                wait_markers: ports.wait_markers.clone(),
                agent_cli: Arc::new(crate::agent_cli::Ports::system()),
            },
        )
    }

    pub fn connect_with_ports(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
        clipboard: ClipboardWriter,
        terminals: Option<Terminals>,
    ) -> AppResult<Self> {
        Self::connect_with_clipboard_ports(
            services,
            repaint,
            clipboard,
            clipboard_reader(),
            terminals,
        )
    }

    pub fn connect_with_clipboard_ports(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
        clipboard: ClipboardWriter,
        read_clipboard: ClipboardReader,
        terminals: Option<Terminals>,
    ) -> AppResult<Self> {
        Self::connect_with_integrations(
            services,
            repaint,
            clipboard,
            read_clipboard,
            HostIntegrations {
                view_states: None,
                terminals,
                reconcile: None,
                wait_markers: Arc::default(),
                agent_cli: Arc::new(crate::agent_cli::Ports::system()),
            },
        )
    }

    pub fn connect_with_settings_ports(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
        clipboard: ClipboardWriter,
        read_clipboard: ClipboardReader,
        terminals: Option<Terminals>,
        reconcile: crate::remote_preferences::Reconcile,
    ) -> AppResult<Self> {
        Self::connect_with_integrations(
            services,
            repaint,
            clipboard,
            read_clipboard,
            HostIntegrations {
                view_states: None,
                terminals,
                reconcile: Some(reconcile),
                wait_markers: Arc::default(),
                agent_cli: Arc::new(crate::agent_cli::Ports::system()),
            },
        )
    }

    #[cfg(all(test, target_os = "macos"))]
    pub(crate) fn connect_with_agent_cli(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
        terminals: Terminals,
        agent_cli: crate::agent_cli::Ports,
    ) -> AppResult<Self> {
        Self::connect_with_integrations(
            services,
            repaint,
            clipboard_writer(),
            clipboard_reader(),
            HostIntegrations {
                view_states: None,
                terminals: Some(terminals),
                reconcile: None,
                wait_markers: Arc::default(),
                agent_cli: Arc::new(agent_cli),
            },
        )
    }

    fn connect_with_integrations(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
        clipboard: ClipboardWriter,
        read_clipboard: ClipboardReader,
        integrations: HostIntegrations,
    ) -> AppResult<Self> {
        let (commands, mut receiver) = mpsc::channel(HOST_COMMAND_CAPACITY);
        let (sender, replies) = mpsc::channel(HOST_COMMAND_CAPACITY);
        let tasks = services.tasks.clone();
        let state = services.state.clone();
        let worker = tasks
            .spawn_transient_handle("native-host-commands", async move {
                let worker = HostWorker {
                    services: &services,
                    clipboard: &clipboard,
                    read_clipboard: &read_clipboard,
                    sender: &sender,
                    repaint: &repaint,
                    integrations: &integrations,
                };
                while let Some(command) = receiver.recv().await {
                    if services.state.is_shutting_down() {
                        break;
                    }
                    let reply = dispatch(&worker, command).await;
                    if let Some(reply) = reply
                        && sender.send(reply).await.is_err()
                    {
                        break;
                    }
                    repaint();
                }
            })
            .ok_or_else(stopped)?;
        Ok(Self {
            state,
            commands,
            replies,
            worker,
        })
    }

    pub fn submit(&self, command: HostCommand) -> AppResult<()> {
        let command = match command {
            HostCommand::Settings(crate::application::domains::settings::Command::SnippetEdit(
                request,
            )) => {
                HostCommand::Settings(crate::application::domains::settings::Command::SnippetEdit(
                    request.admit(&self.state)?,
                ))
            }
            command => command,
        };
        self.commands
            .try_send(command)
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => {
                    AppError::Internal("native host command queue is full".into())
                }
                mpsc::error::TrySendError::Closed(_) => stopped(),
            })
    }

    pub fn poll(&mut self) -> Option<HostReply> {
        self.replies.try_recv().ok()
    }

    pub fn pending_replies(&self) -> usize {
        self.replies.len()
    }

    pub fn disconnect(self) -> JoinHandle<()> {
        drop(self.commands);
        drop(self.replies);
        self.worker
    }
}

fn clipboard_writer() -> ClipboardWriter {
    let clipboard = std::sync::Mutex::new(None);
    Arc::new(move |text| {
        let unavailable = || AppError::Internal("native clipboard write failed".into());
        let mut clipboard = clipboard.lock().map_err(|_| unavailable())?;
        if clipboard.is_none() {
            *clipboard = Some(arboard::Clipboard::new().map_err(|_| unavailable())?);
        }
        clipboard
            .as_mut()
            .ok_or_else(unavailable)?
            .set_text(text)
            .map_err(|_| unavailable())
    })
}

fn clipboard_reader() -> ClipboardReader {
    let clipboard = std::sync::Mutex::new(None);
    Arc::new(move || {
        let unavailable = || AppError::Internal("native clipboard read failed".into());
        let mut clipboard = clipboard.lock().map_err(|_| unavailable())?;
        if clipboard.is_none() {
            *clipboard = Some(arboard::Clipboard::new().map_err(|_| unavailable())?);
        }
        match clipboard.as_mut().ok_or_else(unavailable)?.get_text() {
            Ok(text) => Ok(text),
            Err(arboard::Error::ContentNotAvailable) => Ok(String::new()),
            Err(_) => Err(unavailable()),
        }
    })
}

async fn dispatch(worker: &HostWorker<'_>, command: HostCommand) -> Option<HostReply> {
    match command {
        HostCommand::Shell(command) => {
            crate::application::domains::shell::dispatch(worker, command).await
        }
        HostCommand::Commands(command) => {
            crate::application::domains::commands::dispatch(worker, command).await
        }
        HostCommand::Editor(command) => {
            crate::application::domains::editor::dispatch(worker, command).await
        }
        HostCommand::ExplorerSearch(command) => {
            crate::application::domains::explorer_search::dispatch(worker, command).await
        }
        HostCommand::Git(command) => {
            crate::application::domains::git::dispatch(worker, command).await
        }
        HostCommand::TerminalAgent(command) => {
            crate::application::domains::terminal_agent::dispatch(worker, command).await
        }
        HostCommand::Settings(command) => {
            crate::application::domains::settings::dispatch(worker, command).await
        }
        HostCommand::Preview(command) => {
            crate::application::domains::preview::dispatch(worker, command).await
        }
    }
}

pub(crate) async fn open_file_tab(
    services: &AppServices,
    project: ProjectId,
    pane: Option<PaneId>,
    path: String,
    preview: bool,
) -> AppResult<()> {
    let title = std::path::Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&path)
        .to_owned();
    layout_actions::layout_open_tab(
        services.events.as_ref(),
        &services.state,
        project,
        TabKind::File { path },
        title,
        pane,
        preview,
    )
    .await
    .map(drop)
}

pub(crate) async fn write_clipboard(
    services: &AppServices,
    clipboard: &ClipboardWriter,
    task: &'static str,
    text: String,
) -> AppResult<()> {
    let clipboard = clipboard.clone();
    let state = services.state.clone();
    services
        .tasks
        .run_blocking_result(task, move || {
            if state.is_shutting_down() {
                return Err(stopped());
            }
            clipboard(&text)
        })
        .await
}

pub(crate) fn authorize_terminal_link(
    services: &AppServices,
    hub: Option<&crate::terminal_host::Hub>,
    owner: &crate::terminal_surface::PasteTarget,
    source: &crate::terminal_tabs::MenuTarget,
) -> AppResult<()> {
    if services.state.is_shutting_down() || !owner.is_alive() || !owner.matches(source) {
        return Err(stopped());
    }
    let session = hub
        .and_then(|hub| hub.get(&source.session))
        .ok_or_else(stopped)?;
    if session.metadata().project_id() != &source.project
        || !session
            .snapshot(|snapshot| snapshot.phase == taide_native_terminal::session::Phase::Running)?
        || !services.state.projects.read().contains_key(&source.project)
        || !services
            .state
            .layouts
            .read()
            .get(&source.project)
            .is_some_and(|layout| crate::terminal_tabs::menu_tab(layout, source).is_some())
    {
        return Err(stopped());
    }
    Ok(())
}

pub(crate) async fn update_control_settings(
    services: &Arc<AppServices>,
    reconcile: &Option<crate::remote_preferences::Reconcile>,
    patch: taide_model::settings::SettingsPatch,
) -> Option<HostReply> {
    match taide_runtime::settings_actions::settings_update(
        &services.state,
        patch,
        |current, updated| match reconcile {
            Some(reconcile) => (reconcile)(services.clone(), current, updated),
            None => Box::pin(async {}),
        },
        services.events.as_ref(),
    )
    .await
    {
        Ok(_) => None,
        Err(error) => Some(HostReply::SettingsFailed(error)),
    }
}

pub(crate) fn stopped() -> AppError {
    AppError::Forbidden("native host command admission is closed".into())
}

pub async fn flush_layouts(services: &AppServices) -> AppResult<()> {
    let state = services.state.clone();
    let operation = services
        .tasks
        .begin_operation("native-close-layout-flush")
        .ok_or_else(stopped)?;
    let guard = state.begin_owned_mutation().await;
    services
        .tasks
        .run_blocking_result("native-close-layout-flush", move || {
            let _operation = operation;
            let _guard = guard;
            let dirty: Vec<_> = state.dirty_layouts.read().iter().cloned().collect();
            for project in dirty {
                let layout = state.layouts.read().get(&project).cloned().ok_or_else(|| {
                    AppError::Internal("native dirty layout snapshot is missing".into())
                })?;
                taide_layout::service::save_layout(&state.paths, &project, &layout)?;
                state.dirty_layouts.write().remove(&project);
            }
            Ok(())
        })
        .await
}

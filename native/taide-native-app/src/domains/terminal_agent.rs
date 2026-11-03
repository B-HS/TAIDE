use std::sync::Arc;
use std::time::Instant;

use eframe::egui::{self, Ui};
use taide_model::agent::CliInstallStatus;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::layout::{Tab, TabKind};
use taide_native_ui::commands::{ShellIntent, ShellMutation};
use taide_native_ui::intents::terminal_agent::Intent;
use taide_native_ui::surfaces::terminal_agent::TerminalAgentSurfaces;
use taide_runtime::{layout_actions, system_actions};

use crate::application::{AppSurfaces, NativeApplication, editor_error};
use crate::host::{HostCommand, HostReply, HostWorker};
use crate::presentation;

const TERMINAL_CLIPBOARD_BYTES: usize = 64 * 1024;
const AGENT_CLI_TASK: &str = "native-agent-cli";

pub enum Command {
    EditorIdeSelection(crate::editor_ide_selection::Request),
    ResolveTerminalFileLinks {
        owner: crate::terminal_surface::PasteTarget,
        source: crate::terminal_tabs::MenuTarget,
        request: crate::terminal_file_links::Request,
    },
    OpenTerminalFile {
        owner: crate::terminal_surface::PasteTarget,
        source: crate::terminal_tabs::MenuTarget,
        link: crate::terminal_file_links::ResolvedFileLink,
    },
    OpenTerminalUrl {
        owner: crate::terminal_surface::PasteTarget,
        source: crate::terminal_tabs::MenuTarget,
        uri: String,
    },
    TerminalMenu {
        target: crate::terminal_tabs::MenuTarget,
        operation: crate::terminal_tabs::MenuOperation,
        title: String,
    },
    ReadTerminalClipboard(crate::terminal_surface::PasteTarget),
    RestartTerminal {
        tab: TabId,
        size: taide_native_terminal::Size,
        ports: crate::terminal_dispatch::EffectPorts,
    },
    ResizeTerminal {
        session: String,
        size: taide_native_terminal::Size,
    },
    AttachTerminal {
        tab: TabId,
        size: taide_native_terminal::Size,
        ports: crate::terminal_dispatch::EffectPorts,
    },
    NewTerminal {
        project: ProjectId,
        pane: PaneId,
        title: String,
    },
    CopyTerminalSelection(String),
    Cli(crate::agent_cli::Action),
}

pub enum Reply {
    TerminalFileLinks {
        owner: crate::terminal_surface::PasteTarget,
        request: crate::terminal_file_links::Request,
        result: AppResult<Vec<Option<String>>>,
    },
    TerminalFileOpened {
        result: AppResult<crate::terminal_tabs::OpenedFileLink>,
    },
    TerminalUrlOpened {
        result: AppResult<()>,
    },
    TerminalClipboard {
        target: crate::terminal_surface::PasteTarget,
        result: AppResult<String>,
    },
    TerminalResized {
        session: String,
        result: AppResult<bool>,
    },
    TerminalAttached {
        tab: TabId,
        result: AppResult<String>,
    },
    TerminalSelectionCopied {
        result: AppResult<()>,
    },
    Cli {
        action: crate::agent_cli::Action,
        result: AppResult<CliInstallStatus>,
    },
}

pub(crate) struct State {
    pub(crate) terminals: Arc<crate::terminal_tabs::Tabs>,
    pub(crate) terminal_views: crate::terminal_surface::Views,
    pub(crate) terminal_appearance: crate::terminal_surface::Appearance,
    pub(crate) terminal_fonts: crate::terminal_fonts::Loader,
    pub(crate) editor_ide_selection: crate::editor_ide_selection::State,
    pub(crate) status_ide_appearance: crate::status_ide::Appearance,
    pub(crate) status_ide_icons: crate::status_ide::Icons,
    pub(crate) command_colors: Option<taide_native_terminal::CommandColors>,
    pub(crate) cli_status: Option<CliInstallStatus>,
}

pub(crate) async fn dispatch(worker: &HostWorker<'_>, command: Command) -> Option<HostReply> {
    let services = worker.services;
    let clipboard = worker.clipboard;
    let read_clipboard = worker.read_clipboard;
    let integrations = worker.integrations;
    let terminals = integrations.terminals.as_ref();
    match command {
        Command::EditorIdeSelection(request) => {
            if let Err(error) = request.apply(services).await {
                log::warn!("native IDE selection request failed: {:?}", error.kind());
            }
            None
        }
        Command::ResolveTerminalFileLinks {
            owner,
            source,
            request,
        } => {
            let hub = terminals.map(|terminals| terminals.tabs.hub().clone());
            let services_for_task = services.clone();
            let task_owner = owner.clone();
            let task_request = request.clone();
            let result = services
                .tasks
                .run_blocking_result("native-terminal-file-links", move || {
                    crate::host::authorize_terminal_link(
                        &services_for_task,
                        hub.as_deref(),
                        &task_owner,
                        &source,
                    )?;
                    if task_request.cwd().len() > crate::terminal_links::MAX_LINK_BYTES
                        || task_request.candidates().len()
                            > crate::terminal_file_links::MAX_CANDIDATES
                        || task_request.candidates().iter().any(|candidate| {
                            candidate.len() > crate::terminal_links::MAX_LINK_BYTES
                        })
                    {
                        return Err(AppError::InvalidArgument(
                            "native terminal file link request exceeds its limit".into(),
                        ));
                    }
                    let projects = services_for_task.state.projects.read().clone();
                    Ok(taide_terminal::service::resolve_link_candidates(
                        &projects,
                        task_request.cwd(),
                        task_request.candidates(),
                    ))
                })
                .await;
            Some(HostReply::TerminalAgent(Reply::TerminalFileLinks {
                owner,
                request,
                result,
            }))
        }
        Command::OpenTerminalFile {
            owner,
            source,
            link,
        } => {
            let result = match terminals {
                Some(terminals) => terminals.tabs.open_file_link(owner, source, link).await,
                None => Err(AppError::Internal(
                    "native terminal file link host is unavailable".into(),
                )),
            };
            Some(HostReply::TerminalAgent(Reply::TerminalFileOpened {
                result,
            }))
        }
        Command::OpenTerminalUrl { owner, source, uri } => {
            let hub = terminals.map(|terminals| terminals.tabs.hub().clone());
            let services_for_task = services.clone();
            let result = services
                .tasks
                .run_blocking_result("native-terminal-open-url", move || {
                    crate::host::authorize_terminal_link(
                        &services_for_task,
                        hub.as_deref(),
                        &owner,
                        &source,
                    )?;
                    if uri.len() > crate::terminal_links::MAX_LINK_BYTES {
                        return Err(AppError::InvalidArgument(
                            "native terminal URL exceeds its byte limit".into(),
                        ));
                    }
                    system_actions::system_open_external_url(
                        services_for_task.platform.0.as_ref(),
                        &uri,
                    )
                })
                .await;
            Some(HostReply::TerminalAgent(Reply::TerminalUrlOpened {
                result,
            }))
        }
        Command::TerminalMenu {
            target,
            operation,
            title,
        } => {
            let tab = target.tab.clone();
            let result = match terminals {
                Some(terminals) => terminals.tabs.menu(target, operation, title).await,
                None => Err(AppError::Internal(
                    "native terminal menu host is unavailable".into(),
                )),
            };
            match result {
                Ok(Some(closed)) => Some(HostReply::Editor(
                    crate::application::domains::editor::Reply::Closed {
                        tab,
                        result: Ok(closed),
                    },
                )),
                Ok(None) => None,
                Err(error) => Some(HostReply::Failed(error)),
            }
        }
        Command::ReadTerminalClipboard(target) => {
            let clipboard = read_clipboard.clone();
            let state = services.state.clone();
            let owner = target.clone();
            let result = services
                .tasks
                .run_blocking_result("native-terminal-read-clipboard", move || {
                    if state.is_shutting_down() || !owner.is_alive() {
                        return Err(crate::host::stopped());
                    }
                    let text = clipboard()?;
                    if text.capacity() > TERMINAL_CLIPBOARD_BYTES {
                        return Err(AppError::InvalidArgument(
                            "native terminal clipboard budget exceeded".into(),
                        ));
                    }
                    Ok(text)
                })
                .await;
            Some(HostReply::TerminalAgent(Reply::TerminalClipboard {
                target,
                result,
            }))
        }
        Command::RestartTerminal { tab, size, ports } => {
            let result = match terminals {
                Some(terminals) => {
                    terminals
                        .tabs
                        .restart(
                            tab.clone(),
                            size,
                            (terminals.environment)(services.clone()),
                            ports,
                        )
                        .await
                }
                None => Err(AppError::Forbidden(
                    "native terminal host is unavailable".into(),
                )),
            };
            Some(HostReply::TerminalAgent(Reply::TerminalAttached {
                tab,
                result,
            }))
        }
        Command::ResizeTerminal { session, size } => {
            let result = match terminals.and_then(|terminals| terminals.tabs.hub().get(&session)) {
                Some(terminal) => terminal.resize(services, size).await,
                None => Err(AppError::NotFound(
                    "native terminal session no longer exists".into(),
                )),
            };
            Some(HostReply::TerminalAgent(Reply::TerminalResized {
                session,
                result,
            }))
        }
        Command::AttachTerminal { tab, size, ports } => {
            let result = match terminals {
                Some(terminals) => {
                    terminals
                        .tabs
                        .attach(
                            tab.clone(),
                            size,
                            (terminals.environment)(services.clone()),
                            ports,
                        )
                        .await
                }
                None => Err(AppError::Forbidden(
                    "native terminal host is unavailable".into(),
                )),
            };
            Some(HostReply::TerminalAgent(Reply::TerminalAttached {
                tab,
                result,
            }))
        }
        Command::NewTerminal {
            project,
            pane,
            title,
        } => {
            match layout_actions::layout_open_tab(
                services.events.as_ref(),
                &services.state,
                project,
                TabKind::Terminal {
                    session_id: String::new(),
                    cwd: None,
                },
                title,
                Some(pane),
                false,
            )
            .await
            {
                Ok(_) => None,
                Err(error) => Some(HostReply::Failed(error)),
            }
        }
        Command::CopyTerminalSelection(text) => {
            let result = crate::host::write_clipboard(
                services,
                clipboard,
                "native-terminal-copy-selection",
                text,
            )
            .await;
            Some(HostReply::TerminalAgent(Reply::TerminalSelectionCopied {
                result,
            }))
        }
        Command::Cli(action) => {
            let ports = integrations.agent_cli.clone();
            let tasks = services.tasks.clone();
            let sender = worker.sender.clone();
            let repaint = worker.repaint.clone();
            if services.tasks.spawn_transient(AGENT_CLI_TASK, async move {
                let result = tasks
                    .run_blocking_result(AGENT_CLI_TASK, move || {
                        crate::agent_cli::run(&ports, action)
                    })
                    .await;
                let reply = HostReply::TerminalAgent(Reply::Cli { action, result });
                if sender.send(reply).await.is_ok() {
                    repaint();
                }
            }) {
                None
            } else {
                Some(HostReply::TerminalAgent(Reply::Cli {
                    action,
                    result: Err(crate::host::stopped()),
                }))
            }
        }
    }
}

impl NativeApplication {
    pub(crate) fn apply_terminal_agent_reply(&mut self, context: &egui::Context, reply: Reply) {
        match reply {
            Reply::TerminalFileLinks {
                owner,
                request,
                result,
            } => {
                if self.closing.is_none() && !self.services.state.is_shutting_down() {
                    self.terminal_agent_state
                        .terminal_views
                        .accept_file_links(&owner, &request, result);
                }
            }
            Reply::TerminalFileOpened { result } => self.apply_file_link_reply(context, result),
            Reply::TerminalClipboard { target, result } => {
                if self.closing.is_none() && !self.services.state.is_shutting_down() {
                    match result {
                        Ok(text) => {
                            if let Err(error) =
                                self.terminal_agent_state.terminal_views.accept_paste(
                                    &target,
                                    &text,
                                    self.terminal_agent_state.terminals.hub(),
                                    &self.services,
                                )
                            {
                                log::warn!("native terminal paste failed: {:?}", error.kind());
                            }
                        }
                        Err(error) if target.is_alive() => {
                            log::warn!("native terminal clipboard read failed: {:?}", error.kind())
                        }
                        Err(_) => {}
                    }
                }
            }
            Reply::TerminalAttached { tab, result } => {
                if result.is_err() {
                    self.editor_terminal_writes
                        .retain(|delivery| delivery.tab != tab);
                }
                self.terminal_agent_state.terminal_views.attached(
                    tab,
                    &result,
                    self.terminal_agent_state.terminals.hub(),
                    &self.services,
                );
                if let Err(error) = result {
                    self.report(&error);
                }
            }
            Reply::TerminalResized { session, result } => {
                self.terminal_agent_state.terminal_views.resized(&session);
                if let Err(error) = result {
                    log::warn!("native terminal resize failed: {error}");
                }
            }
            Reply::TerminalSelectionCopied { result } => crate::toast::copy_finished(
                &mut self.toasts,
                &self.locale,
                crate::toast::CopyOrigin::Terminal,
                &result,
                Instant::now(),
            ),
            Reply::TerminalUrlOpened { result } => {
                if result.is_err() {
                    crate::toast::open_link_failed(&mut self.toasts, &self.locale, Instant::now());
                }
            }
            Reply::Cli { action, result } => self.finish_agent_cli(action, result),
        }
    }

    pub(crate) fn apply_terminal_agent_intent(&mut self, intent: Intent) {
        match intent {
            Intent::NewTerminal { project, pane } => {
                self.submit(HostCommand::TerminalAgent(Command::NewTerminal {
                    project,
                    pane,
                    title: presentation::message(&self.locale, "terminal.title", &[]),
                }));
            }
            Intent::Cli(action) => {
                let action = crate::agent_cli::Action::from(action);
                if !self.submit(HostCommand::TerminalAgent(Command::Cli(action))) {
                    self.finish_agent_cli(action, Err(crate::host::stopped()));
                }
            }
        }
    }

    fn finish_agent_cli(
        &mut self,
        action: crate::agent_cli::Action,
        result: AppResult<CliInstallStatus>,
    ) {
        use crate::agent_cli::Action;
        let is_live = |status: &CliInstallStatus| status.installed && !status.dangling;
        let notice = match (action, &result) {
            (Action::Status, _) => None,
            (Action::Install, Ok(status)) => {
                is_live(status).then_some((true, "settings.cliInstallSuccess"))
            }
            (Action::Install, Err(_)) => Some((false, "settings.cliInstallFailed")),
            (Action::Uninstall, Ok(status)) => {
                (!status.installed).then_some((true, "settings.cliUninstallSuccess"))
            }
            (Action::Uninstall, Err(_)) => Some((false, "settings.cliUninstallFailed")),
            (Action::ConnectExternalEditor, Ok(status)) if is_live(status) => {
                Some((true, "settings.cliExternalEditorConnected"))
            }
            (Action::ConnectExternalEditor, _) => Some((false, "settings.cliInstallFailed")),
        };
        match result {
            Ok(status) => self.terminal_agent_state.cli_status = Some(status),
            Err(error) => {
                log::warn!("native agent CLI {action:?} failed: {:?}", error.kind())
            }
        }
        if let Some((is_success, key)) = notice {
            let title = presentation::message(&self.locale, key, &[]);
            if is_success {
                self.toasts.success(title, Instant::now());
            } else {
                self.toasts.error(title, Instant::now());
            }
        }
    }

    pub(crate) fn has_terminal_agent_session(&self, id: &str) -> bool {
        self.terminal_agent_state.terminals.hub().get(id).is_some()
    }

    pub(crate) fn tick_terminal_agent_fonts(&mut self, context: &egui::Context) {
        if self.closing.is_none() && !self.is_exit_ready && !self.services.state.is_shutting_down()
        {
            let families = crate::editor_fonts::Families::new(&self.services.state.settings.read());
            if let Some(result) = self.terminal_agent_state.terminal_fonts.update(
                families,
                context,
                &self.services.tasks,
            ) {
                match result {
                    Ok(warnings) => {
                        for warning in warnings {
                            log::warn!("{warning}");
                        }
                    }
                    Err(error) => self.status = Some(error.to_string()),
                }
            }
        } else {
            self.terminal_agent_state.terminal_fonts.cancel();
        }
        let font_size = self.services.state.settings.read().terminal_font_size;
        if self
            .terminal_agent_state
            .terminal_appearance
            .set_font_size(font_size)
        {
            context.request_repaint();
        }
    }

    pub(crate) fn tick_terminal_agent_cursors(&mut self, context: &egui::Context) {
        match self
            .terminal_agent_state
            .terminals
            .hub()
            .configure_cursors()
        {
            Ok(true) => context.request_repaint(),
            Ok(false) => {}
            Err(error) => self.status = Some(error.to_string()),
        }
    }

    pub(crate) fn tick_terminal_agent_live_sessions(&mut self, context: &egui::Context) {
        let hub = self.terminal_agent_state.terminals.hub();
        if hub.configure_history() {
            context.request_repaint();
        }
        let colors = self
            .terminal_agent_state
            .terminal_appearance
            .command_colors();
        if self.terminal_agent_state.command_colors != Some(colors) {
            hub.configure_command_colors(colors);
            self.terminal_agent_state.command_colors = Some(colors);
        }
    }

    pub(crate) fn tick_terminal_agent_views(
        &mut self,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
    ) {
        let terminal_tabs = snapshot
            .layouts
            .values()
            .flat_map(taide_layout::service::all_roots)
            .flat_map(crate::tabs::tabs_in)
            .filter(|tab| matches!(tab.kind, TabKind::Terminal { .. }))
            .collect::<Vec<_>>();
        let live_tabs = terminal_tabs.iter().map(|tab| tab.id.clone()).collect();
        let live_sessions = terminal_tabs
            .iter()
            .filter_map(|tab| match &tab.kind {
                TabKind::Terminal { session_id, .. } => Some(session_id.clone()),
                _ => None,
            })
            .collect();
        self.terminal_agent_state
            .terminal_views
            .retain(&live_tabs, &live_sessions);
    }

    pub(crate) fn tick_terminal_agent_ide_selection(&mut self, context: &egui::Context) {
        let now = Instant::now();
        for request in self
            .terminal_agent_state
            .editor_ide_selection
            .finish_frame(&self.store, now)
        {
            match request {
                Ok(request) => {
                    if !self.submit(HostCommand::TerminalAgent(Command::EditorIdeSelection(
                        request.clone(),
                    ))) {
                        self.terminal_agent_state
                            .editor_ide_selection
                            .retry(request, now);
                    }
                }
                Err(error) => self.report(&editor_error(error)),
            }
        }
        if let Some(deadline) = self
            .terminal_agent_state
            .editor_ide_selection
            .next_deadline()
        {
            context.request_repaint_after(deadline.saturating_duration_since(now));
        }
    }

    pub(crate) fn finish_terminal_agent_frame(&mut self, context: &egui::Context) {
        if let Err(error) = self.terminal_agent_state.terminal_views.finish_frame(
            self.terminal_agent_state.terminals.hub(),
            &self.services,
            context,
        ) {
            self.status = Some(error.to_string());
        }
    }
}

impl TerminalAgentSurfaces for AppSurfaces<'_> {
    fn project_agents(&self, project: &ProjectId) -> Vec<taide_model::agent::DetectedAgent> {
        self.services.agents.agents_for(project)
    }
    fn is_agent_status_badge_enabled(&self) -> bool {
        self.services
            .state
            .settings
            .read()
            .agent_status_badge_enabled
    }
}

impl AppSurfaces<'_> {
    pub(crate) fn show_terminal_tab(
        &mut self,
        ui: &mut Ui,
        pane: &PaneId,
        tab: &Tab,
        session_id: &str,
        intents: &mut Vec<ShellIntent>,
    ) {
        let request_focus = self.target == Some(pane)
            && self
                .focused
                .as_ref()
                .is_none_or(|(old_pane, old_tab)| old_pane != pane || old_tab != &tab.id);
        let keymap_actions = &mut *self.keymap_actions;
        let has_focused_shell = self.target.is_some();
        let command_context = self.command_context;
        match self.terminal_views.show_with_keymap(
            ui,
            crate::terminal_surface::Request {
                pane,
                tab: &tab.id,
                session_id,
                hub: self.terminals.hub(),
                services: self.services,
                appearance: self.terminal_appearance,
                locale: self.locale,
                request_focus,
                commands: self.commands,
            },
            |action| {
                if !crate::command_dispatch::accepts(action, has_focused_shell, command_context) {
                    return false;
                }
                keymap_actions.push(action.into());
                true
            },
        ) {
            Ok(response) => {
                if response.has_focus() {
                    *self.focused = Some((pane.clone(), tab.id.clone()));
                }
                if response.clicked() {
                    intents.push(ShellIntent::Mutate(ShellMutation::FocusPane(pane.clone())));
                }
            }
            Err(error) => *self.status = Some(error.to_string()),
        }
    }
}

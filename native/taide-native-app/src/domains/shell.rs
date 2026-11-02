use std::sync::Arc;
use std::time::Instant;

use eframe::egui;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::layout::{TabKind, TabWindowTarget};
use taide_native_editor::document::DocumentId;
use taide_native_ui::commands::{ShellIntent, ShellMutation};
use taide_native_ui::shell::WindowScope;
use taide_runtime::{AppServices, layout_actions, project_actions, system_actions};

use crate::application::{APPLICATION_NAME, NativeApplication, WindowPass, paint_rail_icon};
use crate::host::{
    HostCommand, HostIntegrations, HostReply, HostWorker, flush_layouts, open_file_tab, stopped,
    write_clipboard,
};
use crate::presentation;

/// Original `WELCOME_TAB_TITLE`.
const WELCOME_TAB_TITLE: &str = "Welcome";

pub enum Command {
    OpenWelcome {
        project: ProjectId,
        pane: PaneId,
    },
    CopyText(String),
    RevealPath(String),
    OpenInBrowser(String),
    OpenPath(String),
    OpenProject(String),
    OpenProjectByPath(String),
    OpenDroppedPaths {
        focused: Option<ProjectId>,
        paths: Vec<String>,
    },
    OpenRecentProject(ProjectId),
    ClearRecentProjects,
    CloseProject(ProjectId),
    OpenProjectInSlot {
        project: ProjectId,
        slot: taide_model::ids::ShellSlotId,
        edge: taide_model::project::ShellSlotEdge,
    },
    SetProjectGroupMembers {
        group: taide_model::ids::ProjectGroupId,
        members: Vec<ProjectId>,
    },
    ReorderProjects(Vec<ProjectId>),
    ReorderProjectGroups(Vec<taide_model::ids::ProjectGroupId>),
    ReturnAuxiliaryWindowTabs {
        project: ProjectId,
        slot: u32,
    },
    MoveTabToWindow {
        tab: TabId,
        target: TabWindowTarget,
        label: Option<String>,
    },
    CreateProjectGroup {
        name: String,
        color: Option<String>,
        members: Vec<ProjectId>,
    },
    EditProjectGroup {
        group: taide_model::ids::ProjectGroupId,
        name: Option<String>,
        color: Option<Option<String>>,
    },
    DeleteProjectGroup(taide_model::ids::ProjectGroupId),
    OpenProjectGroup(taide_model::ids::ProjectGroupId),
    SetProjectDisplay {
        project: ProjectId,
        patch: taide_model::project::ProjectDisplayPatch,
    },
}

pub enum Reply {
    TabMovedToWindow {
        label: Option<String>,
        result: AppResult<Option<(ProjectId, u32)>>,
    },
    CopiedText {
        result: AppResult<()>,
    },
    SystemFinished {
        result: AppResult<()>,
    },
    ProjectOpenedByPath(AppResult<()>),
    DroppedPathsOpened(Vec<crate::file_drop::Notice>),
    ProjectDisplaySet(AppResult<()>),
    ProjectClosed(ProjectId),
    ProjectGroupSaved(AppResult<()>),
    ProjectGroupOpened(AppResult<taide_model::project::ProjectGroupOpenResult>),
}

pub(crate) struct State {
    pub(crate) welcome: crate::welcome::State,
    pub(crate) app_menu: crate::app_menu::State,
    pub(crate) open_path_dialog: crate::open_path_dialog::OpenPathDialog,
    pub(crate) project_display_dialog:
        taide_native_ui::project_display_dialog::ProjectDisplayDialog,
    pub(crate) project_group_dialog: taide_native_ui::project_group_dialog::ProjectGroupDialog,
    pub(crate) project_group_delete:
        taide_native_ui::project_group_dialog::ProjectGroupDeleteDialog,
    pub(crate) system_usage: crate::system_usage::Sampler,
    pub(crate) system_usage_appearance: crate::system_usage_view::Appearance,
    pub(crate) system_usage_icon: crate::system_usage_view::Icon,
    pub(crate) zen_hint: crate::zen::Hint,
    pub(crate) zen_hint_colors: crate::zen::HintColors,
    pub(crate) file_drop_colors: crate::file_drop::Colors,
    pub(crate) auxiliary_windows: crate::auxiliary_windows::Windows,
    pub(crate) window_state: crate::window_state::Tracker,
    pub(crate) auxiliary_pass_root: Option<WindowScope>,
    pub(crate) overlay_viewport: egui::ViewportId,
    pub(crate) zen_fullscreen_state: crate::zen::Fullscreen,
}

pub(crate) async fn dispatch(worker: &HostWorker<'_>, command: Command) -> Option<HostReply> {
    let services = worker.services;
    let clipboard = worker.clipboard;
    let integrations = worker.integrations;
    match command {
        Command::OpenWelcome { project, pane } => {
            match layout_actions::layout_open_tab(
                services.events.as_ref(),
                &services.state,
                project,
                TabKind::Welcome,
                WELCOME_TAB_TITLE.into(),
                Some(pane),
                false,
            )
            .await
            {
                Ok(_) => None,
                Err(error) => Some(HostReply::Failed(error)),
            }
        }
        Command::OpenPath(path) => {
            let task_services = services.clone();
            let result = services
                .tasks
                .run_blocking_result("native-preview-open-path", move || {
                    if task_services.state.is_shutting_down() {
                        return Err(stopped());
                    }
                    system_actions::system_open_path(
                        &task_services.state,
                        task_services.platform.0.as_ref(),
                        &path,
                    )
                })
                .await;
            Some(HostReply::Shell(Reply::SystemFinished { result }))
        }
        Command::CopyText(text) => {
            let result =
                write_clipboard(services, clipboard, "native-explorer-copy-path", text).await;
            Some(HostReply::Shell(Reply::CopiedText { result }))
        }
        Command::RevealPath(path) => {
            let task_services = services.clone();
            let result = services
                .tasks
                .run_blocking_result("native-explorer-reveal-path", move || {
                    if task_services.state.is_shutting_down() {
                        return Err(stopped());
                    }
                    system_actions::system_reveal_path(
                        &task_services.state,
                        task_services.platform.0.as_ref(),
                        &path,
                    )
                })
                .await;
            Some(HostReply::Shell(Reply::SystemFinished { result }))
        }
        Command::OpenInBrowser(path) => {
            let task_services = services.clone();
            let result = services
                .tasks
                .run_blocking_result("native-explorer-open-browser", move || {
                    if task_services.state.is_shutting_down() {
                        return Err(stopped());
                    }
                    system_actions::system_open_in_browser(
                        &task_services.state,
                        task_services.platform.0.as_ref(),
                        &path,
                    )
                })
                .await;
            Some(HostReply::Shell(Reply::SystemFinished { result }))
        }
        Command::OpenProject(path) => {
            let result = async {
                taide_runtime::project_actions::validate_open_root(std::path::Path::new(&path))?;
                integrations.flush_view_states(services).await?;
                crate::projects::NativeProjects::new(services.clone())
                    .open(path)
                    .await
            }
            .await;
            match result {
                Ok(_) => None,
                Err(error) => Some(HostReply::Failed(error)),
            }
        }
        Command::OpenProjectByPath(path) => {
            let result = async {
                taide_runtime::project_actions::validate_open_root(std::path::Path::new(&path))?;
                integrations.flush_view_states(services).await?;
                crate::projects::NativeProjects::new(services.clone())
                    .open(path)
                    .await
                    .map(|_| ())
            }
            .await;
            Some(HostReply::Shell(Reply::ProjectOpenedByPath(result)))
        }
        Command::OpenDroppedPaths { focused, paths } => {
            let notices = open_dropped_paths(services, integrations, focused, paths).await;
            (!notices.is_empty()).then_some(HostReply::Shell(Reply::DroppedPathsOpened(notices)))
        }
        Command::OpenRecentProject(project) => {
            let result = async {
                let Some(root) =
                    taide_runtime::project_actions::project_list_recent(&services.state)
                        .await?
                        .into_iter()
                        .find(|candidate| candidate.id == project)
                        .map(|candidate| candidate.root)
                else {
                    log::warn!("native recent menu project record is missing");
                    return Ok(());
                };
                taide_runtime::project_actions::validate_open_root(std::path::Path::new(&root))?;
                integrations.flush_view_states(services).await?;
                crate::projects::NativeProjects::new(services.clone())
                    .open(root)
                    .await
                    .map(|_| ())
            }
            .await;
            if let Err(error) = result {
                log::warn!("native recent menu open failed: {:?}", error.kind());
            }
            None
        }
        Command::ClearRecentProjects => {
            if let Err(error) = taide_runtime::project_actions::project_forget_recent(
                services.events.as_ref(),
                &services.state,
            )
            .await
            {
                log::warn!("native clear recent failed: {:?}", error.kind());
            }
            None
        }
        Command::CloseProject(project) => {
            let closed = project.clone();
            let result = async {
                project_actions::get_project(&services.state.projects.read(), &project)?;
                integrations.flush_view_states(services).await?;
                flush_layouts(services).await?;
                project_actions::project_close(
                    services.events.as_ref(),
                    &services.state,
                    &integrations.projects(services),
                    project,
                )
                .await
            }
            .await;
            Some(match result {
                Ok(()) => HostReply::Shell(Reply::ProjectClosed(closed)),
                Err(error) => HostReply::Failed(error),
            })
        }
        Command::OpenProjectInSlot {
            project,
            slot,
            edge,
        } => {
            let result = async {
                project_actions::get_project(&services.state.projects.read(), &project)?;
                project_actions::ensure_slot_placement_allowed(
                    &services.state.session.read(),
                    Some(&project),
                    &slot,
                    edge,
                )?;
                integrations.flush_view_states(services).await?;
                project_actions::project_open_in_slot(
                    services.events.as_ref(),
                    &services.state,
                    &integrations.projects(services),
                    taide_model::project::OpenProjectInSlotRequest {
                        path: None,
                        project_id: Some(project),
                        target_slot: slot,
                        edge,
                    },
                )
                .await
            }
            .await;
            result.err().map(HostReply::Failed)
        }
        Command::SetProjectGroupMembers { group, members } => {
            project_actions::project_group_set_members(
                services.events.as_ref(),
                &services.state,
                group,
                members,
            )
            .await
            .err()
            .map(HostReply::Failed)
        }
        Command::ReorderProjects(ids) => {
            project_actions::project_reorder(services.events.as_ref(), &services.state, ids)
                .await
                .err()
                .map(HostReply::Failed)
        }
        Command::ReorderProjectGroups(ids) => {
            project_actions::project_group_reorder(services.events.as_ref(), &services.state, ids)
                .await
                .err()
                .map(HostReply::Failed)
        }
        Command::ReturnAuxiliaryWindowTabs { project, slot } => {
            layout_actions::return_auxiliary_window_tabs(
                services.events.as_ref(),
                &services.state,
                project,
                slot,
            )
            .await;
            None
        }
        Command::MoveTabToWindow { tab, target, label } => {
            let opened = std::sync::OnceLock::new();
            let result = layout_actions::layout_move_tab_to_window(
                services.events.as_ref(),
                &services.state,
                &services.windows,
                tab,
                target,
                |project, slot| {
                    let result = if !services.state.projects.read().contains_key(&project) {
                        Err(AppError::NotFound(format!("project not open: {project}")))
                    } else if let Some(label) = &label {
                        services
                            .windows
                            .register(label.clone(), project.clone(), slot);
                        opened.get_or_init(|| (project, slot));
                        Ok(label.clone())
                    } else {
                        Err(AppError::Internal(
                            "auxiliary window label was not reserved".into(),
                        ))
                    };
                    std::future::ready(result)
                },
                |closed| {
                    if opened.get().is_some() && label.as_deref() == Some(closed) {
                        services.windows.forget(closed);
                    }
                },
            )
            .await;
            Some(HostReply::Shell(Reply::TabMovedToWindow {
                label,
                result: result.map(|_| opened.into_inner()),
            }))
        }
        Command::CreateProjectGroup {
            name,
            color,
            members,
        } => Some(HostReply::Shell(Reply::ProjectGroupSaved(
            project_actions::project_group_create(
                services.events.as_ref(),
                &services.state,
                name,
                color,
                Some(members),
            )
            .await
            .map(|_| ()),
        ))),
        Command::EditProjectGroup { group, name, color } => {
            let result = async {
                if let Some(name) = name {
                    project_actions::project_group_rename(
                        services.events.as_ref(),
                        &services.state,
                        group.clone(),
                        name,
                    )
                    .await?;
                }
                if let Some(color) = color {
                    project_actions::project_group_set_color(
                        services.events.as_ref(),
                        &services.state,
                        group,
                        color,
                    )
                    .await?;
                }
                Ok(())
            }
            .await;
            Some(HostReply::Shell(Reply::ProjectGroupSaved(result)))
        }
        Command::DeleteProjectGroup(group) => {
            project_actions::project_group_delete(services.events.as_ref(), &services.state, group)
                .await
                .err()
                .map(HostReply::Failed)
        }
        Command::OpenProjectGroup(group) => {
            let result = async {
                project_actions::group_members(&services.state.session.read(), &group)?;
                integrations.flush_view_states(services).await?;
                let worker = services.clone();
                let projects = integrations.projects(services);
                services
                    .tasks
                    .run_nonabortable_result("native-project-group-open", async move {
                        project_actions::project_group_open(
                            worker.events.as_ref(),
                            &worker.state,
                            &projects,
                            group,
                        )
                        .await
                    })
                    .await
            }
            .await;
            Some(HostReply::Shell(Reply::ProjectGroupOpened(result)))
        }
        Command::SetProjectDisplay { project, patch } => {
            Some(HostReply::Shell(Reply::ProjectDisplaySet(
                project_actions::project_set_display(
                    services.events.as_ref(),
                    &services.state,
                    project,
                    patch,
                )
                .await,
            )))
        }
    }
}

async fn open_dropped_paths(
    services: &Arc<AppServices>,
    integrations: &HostIntegrations,
    focused: Option<ProjectId>,
    paths: Vec<String>,
) -> Vec<crate::file_drop::Notice> {
    let mut target = focused;
    let mut notices = Vec::new();
    for path in paths {
        let opened = async {
            taide_runtime::project_actions::validate_open_root(std::path::Path::new(&path))?;
            integrations.flush_view_states(services).await?;
            crate::projects::NativeProjects::new(services.clone())
                .open(path.clone())
                .await
        }
        .await;
        if let Ok(project) = opened {
            target = Some(project.id);
            continue;
        }
        let Some(project) = target.clone() else {
            notices.push(crate::file_drop::Notice::OpenProjectFirst);
            continue;
        };
        let result = async {
            integrations.flush_view_states(services).await?;
            open_file_tab(services, project, None, path, true).await
        }
        .await;
        if let Err(error) = result {
            notices.push(crate::file_drop::Notice::Failed(error));
        }
    }
    notices
}

impl NativeApplication {
    pub(crate) fn apply_shell_reply(&mut self, context: &egui::Context, reply: Reply) {
        match reply {
            Reply::TabMovedToWindow { label, result } => {
                if let Some(label) = label {
                    match &result {
                        Ok(Some((project, slot))) => {
                            self.shell_state
                                .auxiliary_windows
                                .adopt(project.clone(), *slot, label)
                        }
                        _ => self.shell_state.auxiliary_windows.release(&label),
                    }
                }
                if let Err(error) = result {
                    self.report(&error);
                }
                context.request_repaint();
            }
            Reply::CopiedText { result } => crate::toast::copy_finished(
                &mut self.toasts,
                &self.locale,
                crate::toast::CopyOrigin::Explorer,
                &result,
                Instant::now(),
            ),
            Reply::SystemFinished { result } => {
                if let Err(error) = result {
                    self.report(&error);
                }
            }
            Reply::ProjectOpenedByPath(result) => {
                self.shell_state.open_path_dialog.finish(result.is_ok());
                if let Err(error) = result {
                    self.report(&error);
                }
                context.request_repaint();
            }
            Reply::DroppedPathsOpened(notices) => {
                for notice in notices {
                    match notice {
                        crate::file_drop::Notice::OpenProjectFirst => {
                            self.notify_open_project_first()
                        }
                        crate::file_drop::Notice::Failed(error) => self.report(&error),
                    }
                }
                context.request_repaint();
            }
            Reply::ProjectDisplaySet(result) => {
                self.shell_state
                    .project_display_dialog
                    .finish(result.is_ok());
                if let Err(error) = result {
                    self.report(&error);
                }
                context.request_repaint();
            }
            Reply::ProjectClosed(project) => {
                self.release_project_documents(&project);
                context.request_repaint();
            }
            Reply::ProjectGroupSaved(result) => {
                self.shell_state.project_group_dialog.finish(result.is_ok());
                if let Err(error) = result {
                    self.report(&error);
                }
                context.request_repaint();
            }
            Reply::ProjectGroupOpened(result) => {
                match result {
                    Ok(result) => {
                        let (opened, skipped) = (
                            result.opened.len().to_string(),
                            result.skipped.len().to_string(),
                        );
                        self.toasts.success(
                            presentation::message(
                                &self.locale,
                                "projectGroup.openResult",
                                &[("opened", &opened), ("skipped", &skipped)],
                            ),
                            Instant::now(),
                        );
                    }
                    Err(error) => self.report(&error),
                }
                context.request_repaint();
            }
        }
    }

    pub(crate) fn apply_shell_intent(
        &mut self,
        intent: ShellIntent,
        frame: &eframe::Frame,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
    ) {
        match intent {
            ShellIntent::Shell(intent) => match intent {},
            ShellIntent::OpenWelcome { project, pane } => {
                self.submit(HostCommand::Shell(Command::OpenWelcome { project, pane }));
            }
            ShellIntent::CopyText(text) => {
                self.submit(HostCommand::Shell(Command::CopyText(text)));
            }
            ShellIntent::RevealPath(path) => {
                self.submit(HostCommand::Shell(Command::RevealPath(path)));
            }
            ShellIntent::MoveTabToWindow { tab, target } => {
                let label = matches!(target, TabWindowTarget::NewAuxiliary)
                    .then(|| self.shell_state.auxiliary_windows.reserve());
                if !self.submit(HostCommand::Shell(Command::MoveTabToWindow {
                    tab,
                    target,
                    label: label.clone(),
                })) && let Some(label) = label
                {
                    self.shell_state.auxiliary_windows.release(&label);
                }
            }
            ShellIntent::OpenPath(path) => {
                self.submit(HostCommand::Shell(Command::OpenPath(path)));
            }
            ShellIntent::CloseProject(project) => self.close_project(project),
            ShellIntent::OpenProjectInSlot {
                project,
                slot,
                edge,
            } => {
                self.submit(HostCommand::Shell(Command::OpenProjectInSlot {
                    project,
                    slot,
                    edge,
                }));
            }
            ShellIntent::SetProjectGroupMembers { group, members } => {
                self.submit(HostCommand::Shell(Command::SetProjectGroupMembers {
                    group,
                    members,
                }));
            }
            ShellIntent::ReorderProjects(ids) => {
                self.submit(HostCommand::Shell(Command::ReorderProjects(ids)));
            }
            ShellIntent::ReorderProjectGroups(ids) => {
                self.submit(HostCommand::Shell(Command::ReorderProjectGroups(ids)));
            }
            ShellIntent::NewProjectGroup(project) => {
                if snapshot.project(&project).is_some() {
                    self.shell_state.project_group_dialog.create(project);
                }
            }
            ShellIntent::OpenProjectGroup(group) => {
                self.submit(HostCommand::Shell(Command::OpenProjectGroup(group)));
            }
            ShellIntent::EditProjectGroup(group) => {
                if let Some(group) = snapshot.groups.iter().find(|item| item.id == group) {
                    self.shell_state.project_group_dialog.edit(group);
                }
            }
            ShellIntent::DeleteProjectGroup(group) => {
                if let Some(group) = snapshot.groups.iter().find(|item| item.id == group) {
                    self.shell_state.project_group_delete.open(group);
                }
            }
            ShellIntent::EditProjectDisplay(project) => {
                if let Some(project) = snapshot.project(&project) {
                    self.shell_state.project_display_dialog.open(project);
                }
            }
            ShellIntent::SetProjectDisplay { project, patch } => {
                self.submit(HostCommand::Shell(Command::SetProjectDisplay {
                    project,
                    patch,
                }));
            }
            ShellIntent::ShowOpenProjectNotice => self.notify_open_project_first(),
            ShellIntent::RequestCloseTab(tab) => self.request_tab_close(tab),
            ShellIntent::RequestCloseTabs(tabs) => self.request_close_tabs(tabs),
            ShellIntent::SelectRecentProject { project, root } => {
                if self.services.state.projects.read().contains_key(&project) {
                    if let Err(error) = self
                        .controller
                        .submit(ShellMutation::ActivateProject(project))
                    {
                        self.report(&error);
                    }
                } else {
                    self.submit(HostCommand::Shell(Command::OpenProject(root)));
                }
            }
            ShellIntent::OpenProjectByPath => self.shell_state.open_path_dialog.open(),
            ShellIntent::OpenFolder => {
                if let Some(path) = rfd::FileDialog::new().set_parent(frame).pick_folder() {
                    match path.into_os_string().into_string() {
                        Ok(path) => {
                            self.submit(HostCommand::Shell(Command::OpenProject(path)));
                        }
                        Err(_) => self.status = Some("native project path is not UTF-8".into()),
                    }
                }
            }
            ShellIntent::OpenFile { project, pane } => {
                let root = self
                    .services
                    .state
                    .projects
                    .read()
                    .get(&project)
                    .map(|project| project.root.clone());
                if let Some(root) = root
                    && let Some(path) = rfd::FileDialog::new()
                        .set_parent(frame)
                        .set_directory(root)
                        .pick_file()
                {
                    match path.into_os_string().into_string() {
                        Ok(path) => {
                            self.submit(HostCommand::Editor(
                                crate::domains::editor::Command::OpenFileTab {
                                    project,
                                    pane: Some(pane),
                                    path,
                                    preview: false,
                                },
                            ));
                        }
                        Err(_) => self.status = Some("native file path is not UTF-8".into()),
                    }
                }
            }
            ShellIntent::Mutate(command) => {
                if let Err(error) = self.controller.submit(command) {
                    self.report(&error);
                }
            }
            ShellIntent::Commands(_)
            | ShellIntent::Editor(_)
            | ShellIntent::ExplorerSearch(_)
            | ShellIntent::Git(_)
            | ShellIntent::TerminalAgent(_)
            | ShellIntent::Settings(_)
            | ShellIntent::Preview(_) => {}
        }
    }

    pub(crate) fn tick_shell_menus(&mut self, context: &egui::Context) {
        if self.closing.is_none() && !self.services.state.is_shutting_down() {
            let snapshot = self.controller.snapshot();
            self.shell_state.welcome.tick(
                &snapshot,
                &self.services.state,
                &self.services.tasks,
                context,
            );
        }
        let menu_actions = self.shell_state.app_menu.tick(
            &self.locale,
            &self.services.state,
            &self.services.tasks,
            context,
        );
        for count in self.shell_state.app_menu.take_kept_drafts() {
            self.toasts.info(
                presentation::message(
                    &self.locale,
                    "project.clearRecentKeptDrafts",
                    &[("count", &count.to_string())],
                ),
                Instant::now(),
            );
        }
        for action in menu_actions {
            self.run_app_menu_action(context, action);
        }
    }

    pub(crate) fn reconnect_shell(&mut self) {
        self.shell_state
            .auxiliary_windows
            .settle_reserved(&self.services.state.layouts.read(), &self.services.windows);
    }

    pub(crate) fn tick_shell_system_usage(&mut self, context: &egui::Context) {
        let (show_usage, hide_in_zen) = {
            let settings = self.services.state.settings.read();
            (settings.show_system_usage, settings.zen_hide_status_bar)
        };
        let status_mounted = !(self.services.state.session.read().window_chrome.zen && hide_in_zen);
        let usage_visible = status_mounted && show_usage;
        if !status_mounted {
            self.shell_state.system_usage.unmount(Instant::now());
        }
        if self.closing.is_some() || self.is_exit_ready {
            self.shell_state.system_usage.detail_open = false;
        }
        self.shell_state.system_usage.tick(
            &self.services,
            context,
            usage_visible && self.closing.is_none() && !self.is_exit_ready,
            Instant::now(),
        );
    }

    pub(crate) fn show_shell_usage_detail(&mut self, context: &egui::Context) {
        if self.shell_state.system_usage.detail_open
            && crate::system_usage_view::show_detail(
                context,
                &self.locale,
                &self.shell_state.system_usage_appearance,
                self.shell_state.system_usage.processes(),
            )
        {
            self.shell_state.system_usage.detail_open = false;
            context.request_repaint();
        }
    }

    pub(crate) fn show_shell_dialogs(
        &mut self,
        context: &egui::Context,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
    ) {
        if let Some(path) = self
            .shell_state
            .open_path_dialog
            .show(context, &self.locale)
            && !self.submit(HostCommand::Shell(Command::OpenProjectByPath(path)))
        {
            self.shell_state.open_path_dialog.finish(false);
        }
        if self
            .shell_state
            .project_display_dialog
            .project()
            .is_some_and(|project| snapshot.project(project).is_none())
        {
            self.shell_state.project_display_dialog.close();
        }
        let locale = &self.locale;
        let icons = &mut self.explorer_search_state.explorer_icons;
        let status = &mut self.status;
        let saved = self.shell_state.project_display_dialog.show(
            context,
            &self.shell.colors,
            &|key| presentation::message(locale, key, &[]),
            &mut |ui, rect, icon, color| paint_rail_icon(icons, status, ui, rect, icon, color),
        );
        if let Some((project, patch)) = saved
            && !self.submit(HostCommand::Shell(Command::SetProjectDisplay {
                project,
                patch,
            }))
        {
            self.shell_state.project_display_dialog.finish(false);
        }
        self.project_group_dialogs(context, snapshot);
    }

    /// The rail's group dialogs: creating a group for the project whose menu asked (`app-sidebar.tsx`
    /// 124-128), editing a group (`sortable-project-group-header.tsx` 79-87) and confirming its
    /// deletion (129-144). A group that disappears closes its dialogs, as the header unmounts.
    fn project_group_dialogs(
        &mut self,
        context: &egui::Context,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
    ) {
        use taide_native_ui::project_group_dialog::{GroupEdit, Target};
        let stored = |group: &taide_model::ids::ProjectGroupId| {
            snapshot.groups.iter().find(|item| &item.id == group)
        };
        if let Some(Target::Edit(group)) = self.shell_state.project_group_dialog.target()
            && stored(group).is_none()
        {
            self.shell_state.project_group_dialog.close();
        }
        if self
            .shell_state
            .project_group_delete
            .group()
            .is_some_and(|group| stored(group).is_none())
        {
            self.shell_state.project_group_delete.close();
        }
        let submission =
            self.shell_state
                .project_group_dialog
                .show(context, &self.shell.colors, &|key| {
                    presentation::message(&self.locale, key, &[])
                });
        if let Some(submission) = submission {
            let color = submission.color.map(str::to_owned);
            let command = match submission.target {
                Target::Create(project) => Some(HostCommand::Shell(Command::CreateProjectGroup {
                    name: submission.name,
                    color,
                    members: vec![project],
                })),
                Target::Edit(group) => stored(&group)
                    .map(|stored| GroupEdit::new(stored, &submission.name, submission.color))
                    .filter(|edit| !edit.is_empty())
                    .map(|edit| {
                        HostCommand::Shell(Command::EditProjectGroup {
                            group,
                            name: edit.name,
                            color: edit.color,
                        })
                    }),
            };
            match command {
                Some(command) => {
                    if !self.submit(command) {
                        self.shell_state.project_group_dialog.finish(false);
                    }
                }
                None => self.shell_state.project_group_dialog.finish(true),
            }
        }
        let deleted = self
            .shell_state
            .project_group_delete
            .show(context, &|key, arguments| {
                presentation::message(&self.locale, key, arguments)
            });
        if let Some(group) = deleted {
            self.submit(HostCommand::Shell(Command::DeleteProjectGroup(group)));
        }
    }

    /// Original `project_close` first has every window mirror the project's dirty buffers
    /// (`await_project_flush`); the host queue is serial, so the mirrors submitted here land before
    /// the close removes the project root they are written under.
    pub(crate) fn close_project(&mut self, project: ProjectId) {
        if !self.flush_dirty() {
            return;
        }
        let documents: Vec<_> = self
            .files
            .values()
            .filter(|file| file.project.as_ref() == Some(&project))
            .map(|file| file.id)
            .chain(
                self.untitled
                    .values()
                    .filter(|draft| draft.project == project)
                    .map(|draft| draft.id),
            )
            .collect();
        for document in documents {
            let Ok(snapshot) = self.store.documents().snapshot(document) else {
                continue;
            };
            if let Some(job) = self.persistence.mirror_job(snapshot, project.clone()) {
                if !self.submit(HostCommand::Editor(
                    crate::domains::editor::Command::MirrorDraft(job.clone()),
                )) {
                    return;
                }
                self.persistence.mirror_started(&job);
            }
        }
        self.submit(HostCommand::Shell(Command::CloseProject(project)));
    }

    pub(crate) fn window_scopes(&self) -> Vec<WindowScope> {
        let root = self
            .shell_state
            .auxiliary_pass_root
            .as_ref()
            .unwrap_or(&self.shell.scope);
        let mut scopes = vec![root.clone()];
        if matches!(root, WindowScope::Main) {
            scopes.extend(
                self.shell_state
                    .auxiliary_windows
                    .windows()
                    .iter()
                    .map(crate::auxiliary_windows::Window::scope),
            );
        }
        scopes
    }

    pub(crate) fn active_window_scope(
        &self,
        is_active: impl Fn(&WindowScope) -> bool,
    ) -> Option<WindowScope> {
        self.window_scopes()
            .into_iter()
            .find(|scope| is_active(scope))
    }

    pub(crate) fn is_any_window_focused(&self, context: &egui::Context) -> bool {
        context.input(|input| input.raw.focused)
            || self
                .shell_state
                .auxiliary_windows
                .windows()
                .iter()
                .any(|window| context.input_for(window.viewport, |input| input.raw.focused))
    }

    pub(crate) fn focused_viewport(&self, context: &egui::Context) -> egui::ViewportId {
        if context.input(|input| input.raw.focused) {
            return egui::ViewportId::ROOT;
        }
        self.shell_state
            .auxiliary_windows
            .windows()
            .iter()
            .find(|window| context.input_for(window.viewport, |input| input.raw.focused))
            .map_or(egui::ViewportId::ROOT, |window| window.viewport)
    }

    pub(crate) fn run_app_menu_action(
        &mut self,
        context: &egui::Context,
        action: crate::app_menu::Action,
    ) {
        use crate::app_menu::{Action, Input};
        let is_live = self.closing.is_none()
            && !self.is_exit_ready
            && !self.services.state.is_shutting_down();
        match action {
            Action::Quit => {
                context.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Close)
            }
            Action::OpenRecent(project) if is_live => {
                self.submit(HostCommand::Shell(Command::OpenRecentProject(project)));
            }
            Action::ClearRecent if is_live => {
                self.submit(HostCommand::Shell(Command::ClearRecentProjects));
            }
            Action::Edit(edit) => {
                let viewport = self.focused_viewport(context);
                match edit.input() {
                    Input::Request(command) => context.send_viewport_cmd_to(viewport, command),
                    Input::Chord(key, modifiers) => self
                        .shell_state
                        .app_menu
                        .queue_chord(viewport, key, modifiers),
                }
                context.request_repaint_of(viewport);
            }
            Action::OpenRecent(_) | Action::ClearRecent | Action::Ignored => {}
        }
    }

    pub(crate) fn overlay_viewport(&mut self, context: &egui::Context) -> egui::ViewportId {
        let windows = self.shell_state.auxiliary_windows.windows();
        if !windows
            .iter()
            .any(|window| window.viewport == self.shell_state.overlay_viewport)
        {
            self.shell_state.overlay_viewport = egui::ViewportId::ROOT;
        }
        let has_overlay = self.commands_state.keybindings.is_open()
            || self.commands_state.palette.is_open()
            || self.pending_tab_close.is_some()
            || self.explorer_search_state.pending_entry_delete.is_some()
            || self.shell_state.system_usage.detail_open
            || self.shell_state.open_path_dialog.is_open()
            || self.shell_state.project_display_dialog.is_open()
            || self.shell_state.project_group_dialog.is_open()
            || self.shell_state.project_group_delete.is_open();
        if !has_overlay {
            if context.input(|input| input.raw.focused) {
                self.shell_state.overlay_viewport = egui::ViewportId::ROOT;
            } else if let Some(window) = windows
                .iter()
                .find(|window| context.input_for(window.viewport, |input| input.raw.focused))
            {
                self.shell_state.overlay_viewport = window.viewport;
            }
        }
        self.shell_state.overlay_viewport
    }

    pub(crate) fn show_auxiliary_windows(
        &mut self,
        context: &egui::Context,
        frame: &mut eframe::Frame,
        pass: &mut WindowPass,
    ) -> bool {
        if !matches!(self.shell.scope, WindowScope::Main) {
            return true;
        }
        self.submit_auxiliary_window_returns();
        let is_live = self.closing.is_none()
            && !self.is_exit_ready
            && !self.services.state.is_shutting_down();
        let mut all_proceeded = true;
        for window in self.shell_state.auxiliary_windows.windows().to_vec() {
            if !window.is_alive(&self.services.state.layouts.read()) {
                if is_live {
                    self.close_auxiliary_window(&window.label);
                }
                continue;
            }
            let title = crate::auxiliary_windows::title(
                pass.snapshot
                    .project(&window.project)
                    .map_or(window.project.as_str(), |project| project.name.as_str()),
                APPLICATION_NAME,
            );
            let mut proceeded = true;
            let mut close_requested = false;
            context.show_viewport_immediate(
                window.viewport,
                crate::auxiliary_windows::builder(title),
                |ui, _| {
                    ui.ctx().input_mut(|input| {
                        self.shell_state
                            .app_menu
                            .inject(window.viewport, &mut input.events)
                    });
                    let root = std::mem::replace(&mut self.shell.scope, window.scope());
                    self.shell_state.auxiliary_pass_root = Some(root.clone());
                    proceeded = self.show_window(ui, frame, pass);
                    self.shell_state.auxiliary_pass_root = None;
                    self.shell.scope = root;
                    close_requested = ui.input(|input| input.viewport().close_requested());
                },
            );
            if close_requested && is_live {
                self.close_auxiliary_window(&window.label);
            }
            all_proceeded &= proceeded;
        }
        all_proceeded
    }

    pub(crate) fn submit_auxiliary_window_returns(&mut self) {
        for (project, slot) in self.shell_state.auxiliary_windows.pending_returns() {
            if !self.submit(HostCommand::Shell(Command::ReturnAuxiliaryWindowTabs {
                project: project.clone(),
                slot,
            })) {
                return;
            }
            self.shell_state.auxiliary_windows.returned(&project, slot);
        }
    }

    pub(crate) fn close_auxiliary_window(&mut self, label: &str) {
        let Some(window) = self
            .shell_state
            .auxiliary_windows
            .close(label, &self.services.windows)
        else {
            return;
        };
        if self.shell_state.overlay_viewport == window.viewport {
            self.shell_state.overlay_viewport = egui::ViewportId::ROOT;
        }
        let layouts = self.services.state.layouts.read();
        let documents: Vec<DocumentId> = window
            .layout(&layouts)
            .map(|layout| crate::tabs::tabs_in(&layout.root))
            .unwrap_or_default()
            .into_iter()
            .filter_map(|tab| match &tab.kind {
                TabKind::File { path } => self.files.get(path).map(|file| file.id),
                TabKind::Untitled { .. } => self.untitled.get(&tab.id).map(|document| document.id),
                _ => None,
            })
            .collect();
        drop(layouts);
        for document in documents {
            let Some(project) = self.draft_project(document) else {
                continue;
            };
            if let Ok(snapshot) = self.store.documents().snapshot(document)
                && let Some(job) = self.persistence.mirror_job(snapshot, project)
                && self.submit(HostCommand::Editor(
                    crate::domains::editor::Command::MirrorDraft(job.clone()),
                ))
            {
                self.persistence.mirror_started(&job);
            }
        }
        self.submit_auxiliary_window_returns();
    }
}

use std::collections::HashSet;
use std::sync::Arc;

use eframe::egui::{self, Ui};
use taide_model::ids::ProjectId;
use taide_model::layout::Tab;
use taide_native_ui::intents::git::Intent;
use taide_native_ui::surfaces::git::GitSurfaces;
use taide_runtime::git_actions;

use crate::application::{AppSurfaces, NativeApplication};
use crate::host::{HostCommand, HostReply, HostWorker};

pub enum Command {
    GitBranch(ProjectId),
}

pub enum Reply {
    GitBranch {
        project: ProjectId,
        branch: Option<String>,
    },
}

pub(crate) struct State {
    pub(crate) title_branch_changes: Arc<crate::title_branch::Changes>,
    pub(crate) title_branches: crate::title_branch::Branches,
}

pub(crate) async fn dispatch(worker: &HostWorker<'_>, command: Command) -> Option<HostReply> {
    let services = worker.services;
    let sender = worker.sender;
    let repaint = worker.repaint;
    match command {
        Command::GitBranch(project) => {
            let pending_services = services.clone();
            let sender = sender.clone();
            let repaint = repaint.clone();
            let pending_project = project.clone();
            if services
                .tasks
                .spawn_transient("native-title-branch", async move {
                    let status = git_actions::git_status(
                        || {},
                        git_actions::GitActionContext::new(
                            &pending_services.state,
                            &pending_services.git,
                            &pending_services.tasks,
                        ),
                        pending_project.clone(),
                    )
                    .await;
                    let reply = HostReply::Git(Reply::GitBranch {
                        project: pending_project,
                        branch: status.ok().and_then(|status| status.branch),
                    });
                    if sender.send(reply).await.is_ok() {
                        repaint();
                    }
                })
            {
                None
            } else {
                Some(HostReply::Git(Reply::GitBranch {
                    project,
                    branch: None,
                }))
            }
        }
    }
}

impl NativeApplication {
    pub(crate) fn apply_git_reply(&mut self, context: &egui::Context, reply: Reply) {
        match reply {
            Reply::GitBranch { project, branch } => {
                self.git_state.title_branches.finish(&project, branch);
                context.request_repaint();
            }
        }
    }

    pub(crate) fn apply_git_intent(&mut self, intent: Intent) {
        match intent {}
    }

    pub(crate) fn reconnect_git(&mut self) {
        self.git_state.title_branches.cancel_fetches();
    }

    pub(crate) fn tick_git_title_branches(
        &mut self,
        context: &egui::Context,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
        open_projects: &HashSet<ProjectId>,
    ) {
        self.git_state
            .title_branches
            .invalidate(self.git_state.title_branch_changes.take());
        self.git_state.title_branches.retain(open_projects);
        if self.shell.has_title_bar {
            let shown = snapshot.focused_project().cloned().into_iter().chain(
                self.shell_state
                    .auxiliary_windows
                    .windows()
                    .iter()
                    .map(|window| window.project.clone()),
            );
            for project in self.git_state.title_branches.requests(shown) {
                if !self.submit(HostCommand::Git(Command::GitBranch(project.clone()))) {
                    self.git_state.title_branches.retry(&project);
                    context.request_repaint();
                }
            }
        }
    }
}

impl GitSurfaces for AppSurfaces<'_> {
    fn branch(&self, project: &ProjectId) -> Option<&str> {
        self.title_branches.branch(project)
    }
    fn title_branch_icon(&mut self, ui: &Ui, rect: egui::Rect, color: egui::Color32) {
        if let Err(error) = self.explorer_icons.paint(
            ui,
            rect,
            crate::problems_icons::Glyph::GitBranch,
            color,
            0.0,
        ) {
            *self.status = Some(error.to_string());
        }
    }
}

impl AppSurfaces<'_> {
    pub(crate) fn show_diff_tab(&mut self, ui: &mut Ui, tab: &Tab) {
        self.show_unavailable_tab(ui, tab);
    }

    pub(crate) fn show_claude_diff_tab(&mut self, ui: &mut Ui, tab: &Tab) {
        self.show_unavailable_tab(ui, tab);
    }
}

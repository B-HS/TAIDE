use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures_util::future::BoxFuture;
use taide_infra::watcher::{self, WatchNotification, WatchScope, WatcherHandle};
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::project::{CapabilityKind, Project};
use taide_runtime::project_actions::{self, ProjectLifecyclePort};
use taide_runtime::{AppServices, project_build};

const GIT_DIR_NAME: &str = ".git";

pub type HookReconcile = Arc<dyn Fn(Arc<AppServices>) -> BoxFuture<'static, ()> + Send + Sync>;

pub struct NativeProjects {
    services: Arc<AppServices>,
    reconcile_hooks: HookReconcile,
    terminals: Option<Arc<crate::terminal_host::Hub>>,
}

struct Attachment {
    layout: Option<taide_model::layout::ProjectLayout>,
    files: Option<WatcherHandle>,
    git: Option<WatcherHandle>,
}

impl NativeProjects {
    pub fn new(services: Arc<AppServices>) -> Self {
        Self::with_hook_reconcile(
            services,
            Arc::new(|services| Box::pin(crate::agent_hooks::reconcile_installed(services))),
        )
    }

    pub fn with_hook_reconcile(services: Arc<AppServices>, reconcile_hooks: HookReconcile) -> Self {
        Self {
            services,
            reconcile_hooks,
            terminals: None,
        }
    }

    pub fn with_terminals(mut self, terminals: Arc<crate::terminal_host::Hub>) -> Self {
        self.terminals = Some(terminals);
        self
    }

    pub async fn open(&self, path: String) -> AppResult<Project> {
        let _operation = self
            .services
            .tasks
            .begin_operation("native-project-open")
            .ok_or_else(stopped)?;
        Ok(project_actions::project_open(
            self.services.events.as_ref(),
            &self.services.state,
            self,
            path,
        )
        .await?
        .project)
    }

    pub async fn restore_watchers(&self) -> AppResult<()> {
        let pending = project_actions::projects_pending_watcher_restore(
            &self.services.state.projects.read(),
            &self.services.state.session.read(),
        );
        let mut first_error = None;
        for (id, _) in pending {
            let project = self.services.state.projects.read().get(&id).cloned();
            if let Some(project) = project
                && let Err(error) = self.attach(&project).await
                && first_error.is_none()
            {
                first_error = Some(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    async fn attach(&self, project: &Project) -> AppResult<()> {
        let services = self.services.clone();
        let pending_project = project.clone();
        let reconcile_hooks = self.reconcile_hooks.clone();
        let pending = project_build::run_project_build(&self.services.tasks, move || {
            let state = &services.state;
            let id = &pending_project.id;
            let layout = (!state.layouts.read().contains_key(id))
                .then(|| taide_layout::service::load_layout(&state.paths, id));
            let files = if state.watchers.read().contains_key(id) {
                None
            } else {
                optional_watcher(build_files(&services, &pending_project), "file")
            };
            let git = if state.git_watchers.read().contains_key(id)
                || !Path::new(&pending_project.root).join(GIT_DIR_NAME).is_dir()
            {
                None
            } else {
                optional_watcher(build_git(&services, &pending_project), "Git")
            };
            if let Err(error) = refresh_lockfile(&services) {
                log::warn!(
                    "native project IDE lockfile refresh failed: {:?}",
                    error.kind()
                );
            }
            let hook_services = services.clone();
            services
                .tasks
                .spawn_transient("agent-hooks-attach", async move {
                    (reconcile_hooks)(hook_services).await;
                });
            Ok::<_, AppError>(Attachment { layout, files, git })
        })
        .await?;
        let guard = self.services.state.begin_mutation().await;
        let live = self
            .services
            .state
            .projects
            .read()
            .get(&project.id)
            .cloned();
        if self.services.state.is_shutting_down()
            || live
                .as_ref()
                .is_none_or(|live| live.root != project.root || live.root_missing)
        {
            return Err(stopped());
        }
        let emit_git = pending
            .value
            .as_ref()
            .is_ok_and(|attachment| attachment.git.is_some());
        let Attachment { layout, files, git } = pending.value?;
        if let Some(layout) = layout {
            self.services
                .state
                .layouts
                .write()
                .entry(project.id.clone())
                .or_insert(layout);
        }
        if let Some(files) = files {
            self.services
                .state
                .watchers
                .write()
                .entry(project.id.clone())
                .or_insert(files);
        }
        if let Some(git) = git {
            self.services
                .state
                .git_watchers
                .write()
                .entry(project.id.clone())
                .or_insert(git);
        }
        if emit_git {
            self.services.events.publish(AppEvent::GitStatusChanged {
                project_id: project.id.clone(),
            });
        }
        drop(guard);
        Ok(())
    }
}

impl ProjectLifecyclePort for NativeProjects {
    fn detected_kinds(&self, root: &Path) -> Vec<CapabilityKind> {
        let mut kinds = Vec::new();
        if root.join(GIT_DIR_NAME).is_dir() {
            kinds.push(CapabilityKind::Git);
        }
        kinds.push(CapabilityKind::Terminal);
        kinds
    }

    async fn attach_project_capabilities(&self, project: &Project) -> AppResult<()> {
        self.attach(project).await
    }

    async fn await_project_flush(&self, _: &ProjectId) {
        if let Err(error) = crate::host::flush_layouts(&self.services).await {
            log::warn!("native project rollback layout flush failed: {error}");
        }
    }

    fn detach_all(&self, project: &ProjectId) {
        self.services.state.layouts.write().remove(project);
        self.services.state.watchers.write().remove(project);
        self.services.state.git_watchers.write().remove(project);
        self.services.terminal.kill_project(project);
        if let Some(terminals) = &self.terminals {
            terminals.discard_project(project);
        }
        self.services.git.remove(project);
        self.services.tree.remove(project);
        if let Err(error) = refresh_lockfile(&self.services) {
            log::warn!("native project rollback IDE lockfile refresh failed: {error}");
        }
    }
}

fn optional_watcher(result: AppResult<WatcherHandle>, kind: &str) -> Option<WatcherHandle> {
    match result {
        Ok(handle) => Some(handle),
        Err(error) => {
            log::warn!(
                "native project {kind} watcher start failed: {:?}",
                error.kind()
            );
            None
        }
    }
}

fn build_files(services: &Arc<AppServices>, project: &Project) -> AppResult<WatcherHandle> {
    let callback = services.clone();
    let id = project.id.clone();
    let handle = watcher::start_watch(
        PathBuf::from(&project.root),
        WatchScope::Project,
        move |notification| match notification {
            WatchNotification::RescanRequired => {
                callback.git.invalidate_status(&id);
                callback.events.publish(AppEvent::FsRescanRequired {
                    project_id: id.clone(),
                })
            }
            WatchNotification::Changes(changes) => {
                callback.git.invalidate_status(&id);
                for change in
                    taide_infra::self_write::resolve_from_app(&callback.state.self_writes, changes)
                {
                    callback.events.publish(AppEvent::FsChanged {
                        project_id: id.clone(),
                        change,
                    });
                }
            }
        },
    )?;
    let stops = services.state.watcher_stops.clone();
    Ok(handle.with_stop_scheduler(move |job| stops.schedule(job)))
}

fn build_git(services: &Arc<AppServices>, project: &Project) -> AppResult<WatcherHandle> {
    let callback = services.clone();
    let id = project.id.clone();
    let handle = watcher::start_watch(
        Path::new(&project.root).join(GIT_DIR_NAME),
        WatchScope::GitDir,
        move |notification| {
            let (status, refs) = git_invalidation(&notification);
            if status || refs {
                callback.git.invalidate_status(&id);
            }
            if status {
                callback.events.publish(AppEvent::GitStatusChanged {
                    project_id: id.clone(),
                });
            }
            if refs {
                callback.events.publish(AppEvent::GitRefsChanged {
                    project_id: id.clone(),
                });
            }
        },
    )?;
    let stops = services.state.watcher_stops.clone();
    Ok(handle.with_stop_scheduler(move |job| stops.schedule(job)))
}

pub fn git_invalidation(notification: &WatchNotification) -> (bool, bool) {
    let WatchNotification::Changes(changes) = notification else {
        return (true, true);
    };
    let mut status = false;
    let mut refs = false;
    for change in changes {
        for path in &change.paths {
            let components: Vec<_> = Path::new(path)
                .components()
                .filter_map(|part| part.as_os_str().to_str())
                .collect();
            let Some(index) = components.iter().rposition(|part| *part == GIT_DIR_NAME) else {
                continue;
            };
            match &components[index + 1..] {
                ["index"] => status = true,
                ["HEAD"] | ["refs", ..] => refs = true,
                _ => {}
            }
        }
    }
    (status, refs)
}

fn refresh_lockfile(services: &AppServices) -> AppResult<()> {
    let Some((port, token, dir)) = services.ide.lockfile_context() else {
        return Ok(());
    };
    let folders = taide_ide::service::workspace_folders(&services.state.projects.read());
    let content = taide_ide::lockfile::build_lockfile_content(std::process::id(), folders, token);
    taide_ide::lockfile::write_lockfile_atomic(&dir, port, &content)
}

fn stopped() -> AppError {
    AppError::Forbidden("native project is no longer open or its runtime is stopping".into())
}

#[cfg(test)]
#[path = "projects-tests.rs"]
mod tests;

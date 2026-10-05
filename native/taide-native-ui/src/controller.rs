use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::layout::ProjectLayout;
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::{Notify, mpsc, watch};
use tokio::task::JoinHandle;

use crate::commands::{ShellMutation, dispatch};
use crate::snapshot::ShellSnapshot;

pub const SHELL_COMMAND_CAPACITY: usize = 64;

pub struct NativeShellEvents {
    forward: Arc<dyn EventSink>,
    refresh: Arc<Notify>,
}

impl EventSink for NativeShellEvents {
    fn publish(&self, event: AppEvent) {
        self.forward.publish(event);
        self.refresh.notify_one();
    }
}

pub struct ShellController {
    commands: mpsc::Sender<ShellMutation>,
    snapshots: watch::Receiver<Arc<ShellSnapshot>>,
    errors: watch::Receiver<Option<Arc<AppError>>>,
    committed_layouts: Mutex<HashMap<ProjectId, ProjectLayout>>,
}

pub struct ShellConnection {
    pub controller: ShellController,
    pub events: Arc<NativeShellEvents>,
    pub worker: JoinHandle<()>,
}

impl ShellController {
    pub async fn connect(
        state: AppState,
        tasks: &TaskSupervisor,
        forward: Arc<dyn EventSink>,
        repaint: Arc<dyn Fn() + Send + Sync>,
    ) -> AppResult<ShellConnection> {
        let initial = Arc::new(ShellSnapshot::read(&state).await);
        let (commands, mut receiver) = mpsc::channel(SHELL_COMMAND_CAPACITY);
        let (snapshot_sender, snapshots) = watch::channel(initial);
        let (error_sender, errors) = watch::channel(None);
        let events = Arc::new(NativeShellEvents {
            forward,
            refresh: Arc::new(Notify::new()),
        });
        let worker_events = events.clone();
        let worker = tasks
            .spawn_transient_handle("native-shell-controller", async move {
                loop {
                    tokio::select! {
                        biased;
                        command = receiver.recv() => {
                            let Some(command) = command else { break; };
                            let result = dispatch(worker_events.as_ref(), &state, command).await;
                            if let Err(error) = result {
                                error_sender.send_replace(Some(Arc::new(error)));
                            }
                        }
                        _ = worker_events.refresh.notified() => {}
                    }
                    snapshot_sender.send_replace(Arc::new(ShellSnapshot::read(&state).await));
                    repaint();
                }
            })
            .ok_or_else(|| AppError::Internal("native shell task admission is closed".into()))?;
        Ok(ShellConnection {
            controller: Self {
                commands,
                snapshots,
                errors,
                committed_layouts: Mutex::new(HashMap::new()),
            },
            events,
            worker,
        })
    }

    pub fn submit(&self, command: ShellMutation) -> AppResult<()> {
        self.commands
            .try_send(command)
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => {
                    AppError::Internal("native shell command queue is full".into())
                }
                mpsc::error::TrySendError::Closed(_) => {
                    AppError::Internal("native shell command queue is closed".into())
                }
            })
    }

    pub fn snapshot(&self) -> Arc<ShellSnapshot> {
        let current = self.snapshots.borrow().clone();
        let mut committed = self
            .committed_layouts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        committed.retain(|project, layout| {
            current.project(project).is_some()
                && current
                    .layouts
                    .get(project)
                    .is_some_and(|present| present.revision < layout.revision)
        });
        if committed.is_empty() {
            return current;
        }
        let mut snapshot = current.as_ref().clone();
        snapshot.layouts.extend(
            committed
                .iter()
                .map(|(project, layout)| (project.clone(), layout.clone())),
        );
        Arc::new(snapshot)
    }

    pub fn apply_layouts(&mut self, layouts: HashMap<ProjectId, ProjectLayout>) {
        let current = self.snapshot();
        let mut committed = self
            .committed_layouts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        for (project, layout) in layouts {
            if current
                .layouts
                .get(&project)
                .is_some_and(|present| present.revision >= layout.revision)
            {
                continue;
            }
            committed.insert(project, layout);
        }
    }

    pub fn take_error(&mut self) -> Option<Arc<AppError>> {
        let latest = self.errors.borrow_and_update();
        if !latest.has_changed() {
            return None;
        }
        latest.clone()
    }

    pub async fn changed(&mut self) -> AppResult<Arc<ShellSnapshot>> {
        self.snapshots
            .changed()
            .await
            .map_err(|_| AppError::Internal("native shell snapshot owner stopped".into()))?;
        self.snapshots.borrow_and_update();
        Ok(self.snapshot())
    }
}

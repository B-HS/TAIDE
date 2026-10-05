use std::{
    future::Future,
    sync::Arc,
    time::{Duration, Instant},
};

use eframe::egui;
use taide_model::{
    error::{AppError, AppResult},
    system::{SystemUsage, SystemUsageProcess},
};
use taide_runtime::{AppServices, TaskSupervisor};
use tokio::sync::oneshot;

use crate::remote_utilities::{self, Ports};

const POLL_INTERVAL: Duration = Duration::from_secs(3);

struct Pending<T> {
    generation: Arc<()>,
    receiver: oneshot::Receiver<AppResult<T>>,
}

struct Query<T> {
    enabled: bool,
    generation: Arc<()>,
    data: Option<T>,
    pending: Option<Pending<T>>,
    due: Instant,
    initial: bool,
    retain_disabled: bool,
}

impl<T: Send + 'static> Query<T> {
    fn new(retain_disabled: bool) -> Self {
        Self {
            enabled: false,
            generation: Arc::new(()),
            data: None,
            pending: None,
            due: Instant::now(),
            initial: false,
            retain_disabled,
        }
    }

    fn gate(&mut self, enabled: bool, now: Instant) {
        if self.enabled == enabled {
            return;
        }
        self.enabled = enabled;
        self.generation = Arc::new(());
        self.initial = enabled;
        self.due = now;
        if !enabled && !self.retain_disabled {
            self.data = None;
        }
    }

    fn poll(&mut self, now: Instant) {
        let Some(pending) = self.pending.as_mut() else {
            return;
        };
        let result = match pending.receiver.try_recv() {
            Ok(result) => result,
            Err(oneshot::error::TryRecvError::Empty) => return,
            Err(oneshot::error::TryRecvError::Closed) => {
                Err(AppError::Forbidden("system usage worker stopped".into()))
            }
        };
        let current = self.enabled && Arc::ptr_eq(&self.generation, &pending.generation);
        self.pending = None;
        if !current {
            return;
        }
        self.due = now + POLL_INTERVAL;
        match result {
            Ok(data) => self.data = Some(data),
            Err(error) => {
                log::warn!("native system usage refresh failed: {:?}", error.kind())
            }
        }
    }

    fn request(
        &mut self,
        tasks: &TaskSupervisor,
        context: &egui::Context,
        now: Instant,
        foreground: bool,
        work: impl Future<Output = AppResult<T>> + Send + 'static,
    ) {
        if !self.enabled || self.pending.is_some() || (!self.initial && !foreground) {
            return;
        }
        if now < self.due {
            context.request_repaint_after(self.due.saturating_duration_since(now));
            return;
        }
        self.initial = false;
        let (sender, receiver) = oneshot::channel();
        let repaint = context.clone();
        if tasks.spawn_transient("native-system-usage", async move {
            drop(sender.send(work.await));
            repaint.request_repaint();
        }) {
            self.pending = Some(Pending {
                generation: self.generation.clone(),
                receiver,
            });
        } else {
            self.due = now + POLL_INTERVAL;
        }
    }
}

pub(crate) struct Sampler {
    ports: Arc<Ports>,
    summary: Query<SystemUsage>,
    detail: Query<Vec<SystemUsageProcess>>,
    pub(crate) detail_open: bool,
}

impl Sampler {
    pub(crate) fn new(ports: Ports) -> Self {
        Self {
            ports: Arc::new(ports),
            summary: Query::new(true),
            detail: Query::new(false),
            detail_open: false,
        }
    }

    pub(crate) fn tick(
        &mut self,
        services: &Arc<AppServices>,
        context: &egui::Context,
        visible: bool,
        now: Instant,
    ) {
        let stopping = services.state.is_shutting_down();
        if stopping {
            self.detail_open = false;
        }
        self.summary.gate(visible && !stopping, now);
        self.detail.gate(self.detail_open && !stopping, now);
        self.summary.poll(now);
        self.detail.poll(now);
        if stopping {
            return;
        }
        let foreground = context.input(|input| input.raw.focused);
        let summary_services = services.clone();
        let summary_ports = self.ports.clone();
        self.summary
            .request(&services.tasks, context, now, foreground, async move {
                remote_utilities::collect_usage(&summary_services, summary_ports).await
            });
        let detail_services = services.clone();
        let detail_ports = self.ports.clone();
        self.detail
            .request(&services.tasks, context, now, foreground, async move {
                remote_utilities::collect_breakdown(&detail_services, detail_ports).await
            });
    }

    pub(crate) fn unmount(&mut self, now: Instant) {
        self.summary.gate(false, now);
        self.summary.data = None;
        self.detail.gate(false, now);
        self.detail_open = false;
    }

    pub(crate) fn summary(&self) -> Option<&SystemUsage> {
        self.summary
            .enabled
            .then_some(self.summary.data.as_ref())
            .flatten()
    }

    pub(crate) fn processes(&self) -> &[SystemUsageProcess] {
        self.detail.data.as_deref().unwrap_or_default()
    }
}

#[cfg(test)]
#[path = "system-usage-tests.rs"]
mod tests;

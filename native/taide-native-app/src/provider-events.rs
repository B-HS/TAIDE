use std::collections::VecDeque;
use std::sync::Mutex;

use taide_model::agent::DetectedAgent;
use taide_model::app_event::AppEvent;
use taide_model::ids::ProjectId;
use taide_model::lsp::LspInstallPhase;

/// Upper bound on the provider events kept between two frames; the oldest non-closing event is
/// dropped first so a burst cannot grow the inbox without limit.
pub(crate) const MAX_PROVIDER_EVENTS: usize = 256;

/// One app event the original app-level providers react to, copied out of the event sink so the UI
/// thread can consume it on its next tick.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Event {
    Agents {
        project: ProjectId,
        agents: Vec<DetectedAgent>,
    },
    CommandFinished {
        session: String,
        cwd: Option<String>,
        exit_code: Option<i32>,
        duration_ms: u32,
    },
    LspInstallFinished {
        server: String,
        succeeded: bool,
        message: Option<String>,
    },
    IdeSave {
        request: String,
        project: ProjectId,
        path: String,
    },
    ProjectClosed(ProjectId),
}

/// Everything recorded since the last [`Inbox::take`].
#[derive(Debug, Default)]
pub(crate) struct Pending {
    pub(crate) events: VecDeque<Event>,
    pub(crate) external_opens: bool,
    pub(crate) raise_main_window: bool,
}

/// Provider event inbox filled by the paint sink on whichever thread publishes an [`AppEvent`].
#[derive(Debug, Default)]
pub(crate) struct Inbox(Mutex<Pending>);

impl Inbox {
    pub(crate) fn record(&self, event: &AppEvent) {
        let recorded = match event {
            AppEvent::AgentStateChanged { project_id, agents } => Event::Agents {
                project: project_id.clone(),
                agents: agents.clone(),
            },
            AppEvent::TerminalCommandFinished {
                session_id,
                cwd,
                exit_code,
                duration_ms,
            } => Event::CommandFinished {
                session: session_id.clone(),
                cwd: cwd.clone(),
                exit_code: *exit_code,
                duration_ms: *duration_ms,
            },
            AppEvent::LspInstallProgress {
                server_id,
                phase: phase @ (LspInstallPhase::Done | LspInstallPhase::Failed),
                message,
                ..
            } => Event::LspInstallFinished {
                server: server_id.as_str().to_owned(),
                succeeded: *phase == LspInstallPhase::Done,
                message: message.clone(),
            },
            AppEvent::IdeSaveRequested {
                request_id,
                project_id,
                path,
            } => Event::IdeSave {
                request: request_id.clone(),
                project: project_id.clone(),
                path: path.clone(),
            },
            AppEvent::ProjectClosed { project_id } => Event::ProjectClosed(project_id.clone()),
            AppEvent::AgentExternalOpen { .. } => {
                self.request_external_opens();
                return;
            }
            _ => return,
        };
        let mut pending = self.lock();
        if let Event::ProjectClosed(project) = &recorded
            && pending
                .events
                .iter()
                .any(|event| matches!(event, Event::ProjectClosed(queued) if queued == project))
        {
            return;
        }
        if pending.events.len() >= MAX_PROVIDER_EVENTS
            && let Some(oldest) = pending
                .events
                .iter()
                .position(|event| !matches!(event, Event::ProjectClosed(_)))
        {
            pending.events.remove(oldest);
            log::warn!("native provider events overflowed; the oldest event was dropped");
        }
        pending.events.push_back(recorded);
    }

    /// Asks the next tick to drain the agent external-open queue, as the original provider does on
    /// mount and on every `AgentExternalOpen`.
    pub(crate) fn request_external_opens(&self) {
        self.lock().external_opens = true;
    }

    /// Asks the next tick to bring the main window forward, as the original single-instance
    /// handler does for every second launch.
    pub(crate) fn request_main_window(&self) {
        self.lock().raise_main_window = true;
    }

    pub(crate) fn take(&self) -> Pending {
        std::mem::take(&mut *self.lock())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Pending> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taide_model::lsp::LspServerId;

    fn command(session: &str) -> AppEvent {
        AppEvent::TerminalCommandFinished {
            session_id: session.into(),
            cwd: None,
            exit_code: Some(0),
            duration_ms: 1,
        }
    }

    #[test]
    fn provider_inbox는_알림_이벤트만_순서대로_기록하고_비운다() {
        let inbox = Inbox::default();
        let project = ProjectId::new();
        inbox.record(&command("first"));
        inbox.record(&AppEvent::LspInstallProgress {
            server_id: LspServerId("rust-analyzer".into()),
            phase: LspInstallPhase::Downloading,
            received_bytes: 0.0,
            total_bytes: None,
            message: None,
        });
        inbox.record(&AppEvent::LspInstallProgress {
            server_id: LspServerId("rust-analyzer".into()),
            phase: LspInstallPhase::Failed,
            received_bytes: 0.0,
            total_bytes: None,
            message: Some("offline".into()),
        });
        inbox.record(&AppEvent::ProjectClosed {
            project_id: project.clone(),
        });
        inbox.record(&AppEvent::ProjectClosed {
            project_id: project.clone(),
        });
        inbox.record(&AppEvent::AgentExternalOpen {
            request: taide_model::agent::ExternalOpenRequest {
                path: "/tmp/a".into(),
                wait_marker: None,
            },
        });
        let pending = inbox.take();
        assert!(pending.external_opens);
        assert!(!pending.raise_main_window);
        assert_eq!(
            pending.events.into_iter().collect::<Vec<_>>(),
            vec![
                Event::CommandFinished {
                    session: "first".into(),
                    cwd: None,
                    exit_code: Some(0),
                    duration_ms: 1,
                },
                Event::LspInstallFinished {
                    server: "rust-analyzer".into(),
                    succeeded: false,
                    message: Some("offline".into()),
                },
                Event::ProjectClosed(project),
            ]
        );
        let empty = inbox.take();
        assert!(empty.events.is_empty() && !empty.external_opens);
    }

    #[test]
    fn provider_inbox는_상한을_넘으면_닫힘을_남기고_가장_오래된_것을_버린다() {
        let inbox = Inbox::default();
        let project = ProjectId::new();
        inbox.record(&AppEvent::ProjectClosed {
            project_id: project.clone(),
        });
        for index in 0..MAX_PROVIDER_EVENTS {
            inbox.record(&command(&index.to_string()));
        }
        let events = inbox.take().events;
        assert_eq!(events.len(), MAX_PROVIDER_EVENTS);
        assert_eq!(events[0], Event::ProjectClosed(project));
        assert!(matches!(&events[1], Event::CommandFinished { session, .. } if session == "1"));
    }
}

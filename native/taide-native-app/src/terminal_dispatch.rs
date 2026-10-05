use std::sync::Arc;

use taide_infra::terminal_scan::ScanEvent;
use taide_model::{
    app_event::AppEvent,
    error::{AppError, AppResult},
};
use taide_native_terminal::{Effect, Rgb, TerminalEvent, WindowSize};
use taide_runtime::AppServices;
use taide_terminal::{command_clock::TerminalCommandClock, metadata::TerminalSessionMetadata};

use crate::{terminal_frames::Delivery, terminal_writer::Writer};

pub type EffectSink<T> = Arc<dyn Fn(&T) -> AppResult<()> + Send + Sync>;

pub struct EffectPorts {
    pub command_colors: taide_native_terminal::CommandColors,
    pub updated: Arc<dyn Fn() + Send + Sync>,
    pub color: Arc<dyn Fn(usize) -> AppResult<Rgb> + Send + Sync>,
    pub geometry: Arc<dyn Fn() -> AppResult<WindowSize> + Send + Sync>,
    pub event: EffectSink<TerminalEvent>,
    pub stream: EffectSink<ScanEvent>,
}

pub struct ObservePorts {
    pub command_colors: taide_native_terminal::CommandColors,
    pub updated: Arc<dyn Fn() + Send + Sync>,
    pub event: EffectSink<TerminalEvent>,
    pub stream: EffectSink<ScanEvent>,
}

pub(crate) enum SessionPorts {
    Native(EffectPorts),
    Renderer(ObservePorts),
}

impl SessionPorts {
    pub(crate) fn command_colors(&self) -> taide_native_terminal::CommandColors {
        match self {
            Self::Native(ports) => ports.command_colors,
            Self::Renderer(ports) => ports.command_colors,
        }
    }
}

struct ConsumptionPorts<'ports> {
    queries: Option<&'ports EffectPorts>,
    updated: &'ports Arc<dyn Fn() + Send + Sync>,
    event: &'ports EffectSink<TerminalEvent>,
    stream: &'ports EffectSink<ScanEvent>,
}

pub struct Dispatcher {
    session: String,
    metadata: Arc<TerminalSessionMetadata>,
    clock: TerminalCommandClock,
    revision: u64,
    is_failed: bool,
}

impl Dispatcher {
    pub fn new(session: String, metadata: Arc<TerminalSessionMetadata>) -> Self {
        Self {
            session,
            metadata,
            clock: TerminalCommandClock::new(),
            revision: 0,
            is_failed: false,
        }
    }

    pub async fn consume(
        &mut self,
        delivery: &Delivery,
        services: &AppServices,
        writer: &Writer,
        ports: &EffectPorts,
    ) -> AppResult<()> {
        self.consume_with_reply_policy(delivery, services, writer, ports, true)
            .await
    }

    pub async fn consume_with_reply_policy(
        &mut self,
        delivery: &Delivery,
        services: &AppServices,
        writer: &Writer,
        ports: &EffectPorts,
        can_reply: bool,
    ) -> AppResult<()> {
        self.consume_ports(
            delivery,
            services,
            writer,
            ConsumptionPorts {
                queries: can_reply.then_some(ports),
                updated: &ports.updated,
                event: &ports.event,
                stream: &ports.stream,
            },
        )
        .await
    }

    pub async fn consume_observed(
        &mut self,
        delivery: &Delivery,
        services: &AppServices,
        writer: &Writer,
        ports: &ObservePorts,
    ) -> AppResult<()> {
        self.consume_ports(
            delivery,
            services,
            writer,
            ConsumptionPorts {
                queries: None,
                updated: &ports.updated,
                event: &ports.event,
                stream: &ports.stream,
            },
        )
        .await
    }

    pub(crate) async fn consume_display(
        &mut self,
        delivery: &Delivery,
        services: &AppServices,
        writer: &Writer,
        ports: &SessionPorts,
        can_reply: bool,
    ) -> AppResult<()> {
        match ports {
            SessionPorts::Native(ports) => {
                self.consume_with_reply_policy(delivery, services, writer, ports, can_reply)
                    .await
            }
            SessionPorts::Renderer(ports) => {
                self.consume_observed(delivery, services, writer, ports)
                    .await
            }
        }
    }

    async fn consume_ports(
        &mut self,
        delivery: &Delivery,
        services: &AppServices,
        writer: &Writer,
        ports: ConsumptionPorts<'_>,
    ) -> AppResult<()> {
        if self.is_failed || self.revision.checked_add(1) != Some(delivery.frame().revision) {
            self.is_failed = true;
            return Err(AppError::InvalidArgument(
                "native terminal effects are out of order or already failed".into(),
            ));
        }
        self.is_failed = true;
        self.revision = delivery.frame().revision;
        let outcome = &delivery.frame().outcome;
        let now = delivery.observed_at();
        let latest_cwd = outcome
            .effects
            .iter()
            .rev()
            .find_map(|effect| match effect {
                Effect::Stream(ScanEvent::Cwd(cwd)) => Some(cwd),
                _ => None,
            });
        if let Some(cwd) = latest_cwd
            && self.metadata.update_cwd(cwd.clone())
        {
            services.events.publish(AppEvent::TerminalCwdChanged {
                session_id: self.session.clone(),
                cwd: cwd.clone(),
            });
        }
        for effect in &outcome.effects {
            if let Effect::Stream(ScanEvent::CommandMarker(marker)) = effect
                && let Some(timed) = self.clock.record(*marker, now)
            {
                services.events.publish(AppEvent::TerminalCommandFinished {
                    session_id: self.session.clone(),
                    cwd: Some(self.metadata.cwd()),
                    exit_code: timed.exit_code,
                    duration_ms: timed.duration_ms,
                });
            }
        }
        services.agents.record_scan_parts_at(
            &self.session,
            outcome.effects.iter().filter_map(|effect| match effect {
                Effect::Stream(event) => Some(event),
                _ => None,
            }),
            &outcome.text,
            &outcome.overlap,
            now,
        );
        for effect in &outcome.effects {
            if let Effect::Stream(event) = effect {
                (ports.stream)(event)?;
                continue;
            }
            let Effect::Terminal(event) = effect else {
                continue;
            };
            if ports.queries.is_none()
                && matches!(
                    event,
                    TerminalEvent::PtyWrite(..)
                        | TerminalEvent::NativeColorRequest(..)
                        | TerminalEvent::NativeTextAreaSizeRequest(..)
                )
            {
                continue;
            }
            let reply = match event {
                TerminalEvent::PtyWrite(text) => Some(text.as_bytes().to_vec()),
                TerminalEvent::NativeColorRequest(index, query) => {
                    let Some(queries) = ports.queries else {
                        continue;
                    };
                    let color = match query.override_color {
                        Some(color) => color,
                        None => (queries.color)(*index)?,
                    };
                    Some(query.reply(color).into_bytes())
                }
                TerminalEvent::NativeTextAreaSizeRequest(query) => {
                    let Some(queries) = ports.queries else {
                        continue;
                    };
                    Some(query.reply((queries.geometry)()?).into_bytes())
                }
                TerminalEvent::ColorRequest(..)
                | TerminalEvent::TextAreaSizeRequest(..)
                | TerminalEvent::ClipboardLoad(..)
                | TerminalEvent::ClipboardStore(..)
                | TerminalEvent::ChildExit(..) => {
                    return Err(AppError::Forbidden(
                        "native terminal produced an unsupported or forbidden effect".into(),
                    ));
                }
                _ => {
                    (ports.event)(event)?;
                    None
                }
            };
            if let Some(reply) = reply {
                writer.submit_wait(reply).await?.wait().await?;
            }
        }
        (ports.updated)();
        self.is_failed = false;
        Ok(())
    }
}

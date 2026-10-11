use std::collections::{BTreeMap, VecDeque};
use std::future::pending;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use lsp_types::NumberOrString;
use parking_lot::Mutex;
use serde_json::Value;
use taide_infra::lsp_frame::{FrameLimits, TransportFailure};
use taide_infra::lsp_proc::{self, LspProcConfig, LspProcHandle};
use taide_infra::lsp_writer::{WriteReceipt, WriterLimits};
use tokio::sync::{mpsc, oneshot, watch, Notify, OwnedSemaphorePermit, Semaphore};
use tokio::time::Instant;

use super::budget::PayloadSize;
use super::feature::{self, FeatureRequest, TypedReply};
use super::progress::ProgressRegistry;
use super::protocol::{
    self, IncomingMessage, Rejection, ServerNotification, ServerReply, ServerRequestKind,
};
use super::{decode_request_id, DocumentMirror, Failure, LspCoordinator, Outcome, Phase};
use crate::store::LspStore;

const PAYLOAD_CAPACITY_GROWTH_FACTOR: usize = 2;
const REINITIALIZE_MAX_ATTEMPTS: u32 = 3;
const REINITIALIZE_RETRY_DELAY: Duration = Duration::from_secs(2);

pub struct SessionOptions {
    pub frame_limits: FrameLimits,
    pub writer_limits: WriterLimits,
    pub command_capacity: usize,
    pub command_bytes: usize,
    pub incoming_capacity: usize,
    pub incoming_bytes: usize,
    pub outgoing_capacity: usize,
    pub outgoing_bytes: usize,
    pub request_timeout_ms: u64,
    pub write_timeout: Duration,
    pub exit_grace: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub phase: Phase,
    pub generation: u64,
    pub pending: usize,
    pub server_pending: usize,
    pub progress_tokens: usize,
    pub registrations: usize,
    pub capability_revision: u64,
    pub document_methods: Arc<BTreeMap<String, Vec<&'static str>>>,
    pub document_signature_options: Arc<BTreeMap<String, Vec<lsp_types::SignatureHelpOptions>>>,
    pub document_completion_options: Arc<BTreeMap<String, Vec<lsp_types::CompletionOptions>>>,
    pub document_on_type_formatting_options:
        Arc<BTreeMap<String, Vec<lsp_types::DocumentOnTypeFormattingOptions>>>,
    pub pid: Option<u32>,
    pub failure: Option<Failure>,
}

impl SessionSnapshot {
    pub fn supports_document(&self, uri: &str, method: &str) -> bool {
        self.phase == Phase::Running
            && self
                .document_methods
                .get(uri)
                .is_some_and(|methods| methods.contains(&method))
    }
}

pub struct SessionNotice {
    pub generation: u64,
    pub message: Value,
    pub kind: SessionNoticeKind,
    _bytes: Arc<OwnedSemaphorePermit>,
}

pub enum SessionNoticeKind {
    Notification(ServerNotification),
    Request(ServerRequest),
}

pub struct ServerRequest {
    pub ticket: ServerTicket,
    pub kind: Arc<ServerRequestKind>,
}

#[derive(Clone, Debug)]
pub struct ServerTicket {
    generation: u64,
    serial: u64,
    scope: Arc<()>,
    id: NumberOrString,
}

impl ServerTicket {
    pub fn id(&self) -> &NumberOrString {
        &self.id
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

enum Command {
    Open(DocumentMirror),
    WorkspaceRoot(String),
    Close(String),
    Saved(String),
    Change {
        uri: String,
        revision: u64,
        next_revision: u64,
        text: String,
    },
    Request {
        method: String,
        params: Value,
        document: Option<(String, u64)>,
    },
    Restart(LspProcConfig),
    Stop,
    Reply {
        ticket: ServerTicket,
        response: ServerReply,
    },
}

impl Command {
    fn payload_bytes(&self, limit: usize) -> Result<usize, Failure> {
        if matches!(self, Self::Stop) {
            return Ok(0);
        }
        let mut size = PayloadSize::new(limit);
        size.add(std::mem::size_of::<Self>())?;
        match self {
            Self::Open(document) => {
                size.add(document.uri.capacity())?;
                size.add(document.language_id.capacity())?;
                size.add(document.text.capacity())?;
            }
            Self::Close(uri) | Self::Saved(uri) | Self::WorkspaceRoot(uri) => {
                size.add(uri.capacity())?
            }
            Self::Change { uri, text, .. } => {
                size.add(uri.capacity())?;
                size.add(text.capacity())?;
            }
            Self::Request {
                method,
                params,
                document,
            } => {
                size.add(method.capacity())?;
                size.value(params, 0)?;
                if let Some((uri, _)) = document {
                    size.add(uri.capacity())?;
                }
            }
            Self::Restart(config) => {
                size.add(config.command.capacity())?;
                size.add(config.cwd.capacity())?;
                size.slots::<String>(config.args.capacity())?;
                for arg in &config.args {
                    size.add(arg.capacity())?;
                }
            }
            Self::Reply { ticket, response } => {
                if let NumberOrString::String(id) = &ticket.id {
                    size.add(id.capacity())?;
                }
                size.reply(response)?;
            }
            Self::Stop => {}
        }
        Ok(size.bytes)
    }
}

struct Envelope {
    command: Command,
    reply: oneshot::Sender<Result<Value, Failure>>,
    cancelled: Arc<AtomicBool>,
    _bytes: Option<OwnedSemaphorePermit>,
}

struct Cancellation {
    flag: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl Drop for Cancellation {
    fn drop(&mut self) {
        self.flag.store(true, Ordering::SeqCst);
        self.notify.notify_one();
    }
}

#[derive(Clone)]
pub struct SessionClient {
    commands: mpsc::Sender<Envelope>,
    state: watch::Receiver<SessionSnapshot>,
    cancellation: Arc<Notify>,
    notifications: Arc<Mutex<Option<mpsc::Receiver<SessionNotice>>>>,
    command_budget: Arc<Semaphore>,
    command_bytes: usize,
}

impl SessionClient {
    pub fn take_notifications(&self) -> Option<mpsc::Receiver<SessionNotice>> {
        self.notifications.lock().take()
    }

    pub fn subscribe(&self) -> watch::Receiver<SessionSnapshot> {
        self.state.clone()
    }
    pub fn snapshot(&self) -> SessionSnapshot {
        let mut snapshot = self.state.borrow().clone();
        if self.state.has_changed().is_err() && snapshot.phase != Phase::Stopped {
            snapshot.phase = Phase::Degraded;
            snapshot.failure = Some(Failure::TransportClosed);
            snapshot.pending = 0;
            snapshot.server_pending = 0;
            snapshot.progress_tokens = 0;
            snapshot.registrations = 0;
            snapshot.document_methods = Arc::default();
            snapshot.document_signature_options = Arc::default();
            snapshot.document_completion_options = Arc::default();
            snapshot.document_on_type_formatting_options = Arc::default();
        }
        snapshot
    }

    pub async fn wait_for_phase(&mut self, phase: Phase) -> Result<SessionSnapshot, Failure> {
        loop {
            let snapshot = self.snapshot();
            if snapshot.phase == phase {
                return Ok(snapshot);
            }
            if phase == Phase::Running
                && matches!(
                    snapshot.phase,
                    Phase::Degraded | Phase::Stopping | Phase::Stopped
                )
            {
                return Err(snapshot.failure.unwrap_or(Failure::Stopped));
            }
            self.state
                .changed()
                .await
                .map_err(|_| Failure::TransportClosed)?;
        }
    }

    async fn send(&self, command: Command) -> Result<Value, Failure> {
        if self.commands.is_closed() {
            return Err(Failure::TransportClosed);
        }
        let bytes = u32::try_from(command.payload_bytes(self.command_bytes)?)
            .map_err(|_| Failure::Capacity)?;
        let permit = self
            .command_budget
            .clone()
            .try_acquire_many_owned(bytes)
            .map_err(|_| Failure::Capacity)?;
        let (reply, receive) = oneshot::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let _cancellation = Cancellation {
            flag: cancelled.clone(),
            notify: self.cancellation.clone(),
        };
        self.commands
            .try_send(Envelope {
                command,
                reply,
                cancelled,
                _bytes: Some(permit),
            })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => Failure::Capacity,
                mpsc::error::TrySendError::Closed(_) => Failure::TransportClosed,
            })?;
        receive.await.unwrap_or(Err(Failure::TransportClosed))
    }

    pub async fn open(&self, document: DocumentMirror) -> Result<(), Failure> {
        self.send(Command::Open(document)).await.map(|_| ())
    }

    pub async fn close(&self, uri: String) -> Result<(), Failure> {
        self.send(Command::Close(uri)).await.map(|_| ())
    }

    pub async fn saved(&self, uri: String) -> Result<(), Failure> {
        self.send(Command::Saved(uri)).await.map(|_| ())
    }

    pub async fn change(
        &self,
        uri: String,
        revision: u64,
        next_revision: u64,
        text: String,
    ) -> Result<(), Failure> {
        self.send(Command::Change {
            uri,
            revision,
            next_revision,
            text,
        })
        .await
        .map(|_| ())
    }

    pub async fn request(
        &self,
        method: String,
        params: Value,
        document: Option<(String, u64)>,
    ) -> Result<Value, Failure> {
        self.send(Command::Request {
            method,
            params,
            document,
        })
        .await
    }

    pub async fn request_typed<R: FeatureRequest>(
        &self,
        params: R::Params,
        document: Option<(String, u64)>,
    ) -> Result<TypedReply<R>, Failure> {
        let params = serde_json::to_value(params).map_err(|_| Failure::MalformedRequest)?;
        let reply = self.request(R::METHOD.into(), params, document).await?;
        TypedReply::<R>::decode(reply)
    }

    pub async fn restart(&self, config: LspProcConfig) -> Result<(), Failure> {
        self.send(Command::Restart(config)).await.map(|_| ())
    }

    pub async fn add_workspace_root(&self, root: String) -> Result<(), Failure> {
        self.send(Command::WorkspaceRoot(root)).await.map(|_| ())
    }

    pub async fn stop(&self) -> Result<(), Failure> {
        if self.snapshot().phase == Phase::Stopped {
            return Ok(());
        }
        self.send(Command::Stop).await.map(|_| ())
    }

    pub async fn reply(&self, ticket: ServerTicket, response: ServerReply) -> Result<(), Failure> {
        self.send(Command::Reply { ticket, response })
            .await
            .map(|_| ())
    }
}

struct Inbound {
    payload: String,
    _bytes: OwnedSemaphorePermit,
}

struct ActiveProcess(Arc<LspProcHandle>);

impl Drop for ActiveProcess {
    fn drop(&mut self) {
        self.0.kill();
    }
}

struct PendingCall {
    method: String,
    reply: oneshot::Sender<Result<Value, Failure>>,
    cancelled: Arc<AtomicBool>,
}

struct PendingServerCall {
    id: NumberOrString,
    request: Arc<ServerRequestKind>,
    deadline: Instant,
    _bytes: Arc<OwnedSemaphorePermit>,
}

struct OutgoingFrame {
    payload: String,
    is_exit: bool,
}

struct BoundedJsonPayload {
    bytes: Vec<u8>,
    limit: usize,
}

impl BoundedJsonPayload {
    fn serialize(message: &Value, limit: usize) -> Result<String, Failure> {
        let mut payload = Self {
            bytes: Vec::new(),
            limit,
        };
        serde_json::to_writer(&mut payload, message).map_err(|error| {
            if error.is_io() {
                return Failure::Capacity;
            }
            Failure::MalformedResponse
        })?;
        String::from_utf8(payload.bytes).map_err(|_| Failure::MalformedResponse)
    }
}

impl Write for BoundedJsonPayload {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let length = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .filter(|length| *length <= self.limit)
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "JSON payload capacity exceeded")
            })?;
        if length > self.bytes.capacity() {
            let capacity = self
                .bytes
                .capacity()
                .checked_mul(PAYLOAD_CAPACITY_GROWTH_FACTOR)
                .unwrap_or(self.limit)
                .max(length)
                .min(self.limit);
            self.bytes
                .try_reserve_exact(capacity - self.bytes.len())
                .map_err(|_| {
                    io::Error::new(io::ErrorKind::OutOfMemory, "JSON payload allocation failed")
                })?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct PendingWrite {
    receipt: WriteReceipt,
    deadline: Instant,
    is_exit: bool,
}

enum Event {
    Command(Option<Envelope>),
    Message(Option<Inbound>),
    Exited,
    Written(Result<(), Failure>),
    Deadline,
    Cancelled,
}

pub struct SessionRunner {
    store: LspStore,
    config: Option<LspProcConfig>,
    initialize: Value,
    options: SessionOptions,
    coordinator: LspCoordinator,
    initialize_attempts: u32,
    initialize_retry: Option<Instant>,
    commands: mpsc::Receiver<Envelope>,
    has_commands: bool,
    state: watch::Sender<SessionSnapshot>,
    cancellation: Arc<Notify>,
    origin: Instant,
    process: Option<ActiveProcess>,
    messages: Option<mpsc::Receiver<Inbound>>,
    exited: Option<oneshot::Receiver<()>>,
    pending: BTreeMap<u64, PendingCall>,
    server_pending: BTreeMap<u64, PendingServerCall>,
    server_serial: u64,
    server_scope: Arc<()>,
    progress: ProgressRegistry,
    outgoing: VecDeque<OutgoingFrame>,
    outgoing_bytes: usize,
    writing: Option<PendingWrite>,
    exit_deadline: Option<Instant>,
    stop_deadline: Option<Instant>,
    stop_reply: Option<oneshot::Sender<Result<Value, Failure>>>,
    notifications: mpsc::Sender<SessionNotice>,
    incoming_budget: Arc<Semaphore>,
    is_terminal_error: bool,
}

impl SessionRunner {
    pub fn prepare(
        store: LspStore,
        config: LspProcConfig,
        initialize: Value,
        options: SessionOptions,
    ) -> Result<(SessionClient, Self), Failure> {
        if !initialize.is_object()
            || options.request_timeout_ms == 0
            || options.write_timeout.is_zero()
            || options.exit_grace.is_zero()
            || [
                options.command_capacity,
                options.command_bytes,
                options.incoming_capacity,
                options.outgoing_capacity,
                options.incoming_bytes,
                options.outgoing_bytes,
            ]
            .iter()
            .any(|size| *size == 0 || *size > Semaphore::MAX_PERMITS)
            || u32::try_from(options.incoming_bytes).is_err()
            || u32::try_from(options.command_bytes).is_err()
        {
            return Err(Failure::Capacity);
        }
        let origin = Instant::now();
        origin
            .checked_add(Duration::from_millis(options.request_timeout_ms))
            .ok_or(Failure::CounterOverflow)?;
        origin
            .checked_add(options.write_timeout)
            .ok_or(Failure::CounterOverflow)?;
        origin
            .checked_add(options.exit_grace)
            .ok_or(Failure::CounterOverflow)?;
        let progress = ProgressRegistry::new(options.incoming_capacity);
        progress.prepare_request(&initialize)?;
        let (commands, receiver) = mpsc::channel(options.command_capacity);
        let (state, watch) = watch::channel(SessionSnapshot {
            phase: Phase::Detected,
            generation: 0,
            pending: 0,
            server_pending: 0,
            progress_tokens: 0,
            registrations: 0,
            capability_revision: 0,
            document_methods: Arc::default(),
            document_signature_options: Arc::default(),
            document_completion_options: Arc::default(),
            document_on_type_formatting_options: Arc::default(),
            pid: None,
            failure: None,
        });
        let cancellation = Arc::new(Notify::new());
        let (notifications, notification_receiver) = mpsc::channel(options.incoming_capacity);
        let incoming_budget = Arc::new(Semaphore::new(options.incoming_bytes));
        let client = SessionClient {
            commands,
            state: watch,
            cancellation: cancellation.clone(),
            notifications: Arc::new(Mutex::new(Some(notification_receiver))),
            command_budget: Arc::new(Semaphore::new(options.command_bytes)),
            command_bytes: options.command_bytes,
        };
        let runner = Self {
            store,
            config: Some(config),
            initialize,
            coordinator: LspCoordinator::new(options.request_timeout_ms),
            initialize_attempts: 1,
            initialize_retry: None,
            options,
            commands: receiver,
            has_commands: true,
            state,
            cancellation,
            origin,
            process: None,
            messages: None,
            exited: None,
            pending: BTreeMap::new(),
            server_pending: BTreeMap::new(),
            server_serial: 0,
            server_scope: Arc::new(()),
            progress,
            outgoing: VecDeque::new(),
            outgoing_bytes: 0,
            writing: None,
            exit_deadline: None,
            stop_deadline: None,
            stop_reply: None,
            notifications,
            incoming_budget,
            is_terminal_error: false,
        };
        Ok((client, runner))
    }

    fn now(&self) -> u64 {
        u64::try_from(self.origin.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    fn publish(&self) {
        let (
            document_methods,
            document_signature_options,
            document_completion_options,
            document_on_type_formatting_options,
        ) = {
            let previous = self.state.borrow();
            if previous.phase == self.coordinator.phase()
                && previous.generation == self.coordinator.generation()
                && previous.capability_revision == self.coordinator.capability_revision()
                && previous
                    .document_methods
                    .keys()
                    .eq(self.coordinator.documents.keys())
            {
                (
                    previous.document_methods.clone(),
                    previous.document_signature_options.clone(),
                    previous.document_completion_options.clone(),
                    previous.document_on_type_formatting_options.clone(),
                )
            } else {
                let methods = Arc::new(
                    self.coordinator
                        .documents
                        .keys()
                        .map(|uri| {
                            let methods = feature::METHODS
                                .iter()
                                .copied()
                                .filter(|method| self.coordinator.supports_document(method, uri))
                                .collect();
                            (uri.clone(), methods)
                        })
                        .collect(),
                );
                let signatures = Arc::new(
                    self.coordinator
                        .documents
                        .keys()
                        .filter_map(|uri| {
                            let options = self.coordinator.signature_options(uri);
                            (!options.is_empty()).then_some((uri.clone(), options))
                        })
                        .collect(),
                );
                let completions = Arc::new(
                    self.coordinator
                        .documents
                        .keys()
                        .filter_map(|uri| {
                            let options = self.coordinator.completion_options(uri);
                            (!options.is_empty()).then_some((uri.clone(), options))
                        })
                        .collect(),
                );
                let on_type_formatting = Arc::new(
                    self.coordinator
                        .documents
                        .keys()
                        .filter_map(|uri| {
                            let options = self.coordinator.on_type_formatting_options(uri);
                            (!options.is_empty()).then_some((uri.clone(), options))
                        })
                        .collect(),
                );
                (methods, signatures, completions, on_type_formatting)
            }
        };
        self.state.send_replace(SessionSnapshot {
            phase: if self.initialize_retry.is_some() {
                Phase::Initializing
            } else {
                self.coordinator.phase()
            },
            generation: self.coordinator.generation(),
            pending: self.coordinator.pending_count(),
            server_pending: self.server_pending.len(),
            progress_tokens: self.progress.len(),
            registrations: self.coordinator.registration_count(),
            capability_revision: self.coordinator.capability_revision(),
            document_methods,
            document_signature_options,
            document_completion_options,
            document_on_type_formatting_options,
            pid: self.process.as_ref().and_then(|process| process.0.pid()),
            failure: if self.initialize_retry.is_some() {
                None
            } else {
                self.coordinator.last_failure().cloned()
            },
        });
    }

    fn spawn(&mut self, config: LspProcConfig) -> Result<(), Failure> {
        let (sender, messages) = mpsc::channel(self.options.incoming_capacity);
        let bytes = self.incoming_budget.clone();
        let (report, exited) = oneshot::channel();
        let process = self
            .store
            .spawn_process(|| {
                lsp_proc::spawn_owned(
                    config,
                    self.options.frame_limits,
                    self.options.writer_limits,
                    move |payload| {
                        let length = u32::try_from(payload.len())
                            .map_err(|_| TransportFailure::BodyTooLarge)?;
                        let permit = bytes
                            .clone()
                            .try_acquire_many_owned(length)
                            .map_err(|_| TransportFailure::ConsumerUnavailable)?;
                        sender
                            .try_send(Inbound {
                                payload,
                                _bytes: permit,
                            })
                            .map_err(|_| TransportFailure::ConsumerUnavailable)
                    },
                    move |_, _| {
                        report.send(()).ok();
                    },
                )
                .map(Arc::new)
            })
            .map_err(|_| Failure::TransportClosed)?;
        self.process = Some(ActiveProcess(process));
        self.messages = Some(messages);
        self.exited = Some(exited);
        Ok(())
    }

    fn enqueue(&mut self, messages: Vec<Value>) -> Result<(), Failure> {
        if self
            .outgoing
            .len()
            .checked_add(messages.len())
            .is_none_or(|size| size > self.options.outgoing_capacity)
        {
            return Err(Failure::Capacity);
        }
        let mut outgoing = Vec::new();
        outgoing
            .try_reserve_exact(messages.len())
            .map_err(|_| Failure::Capacity)?;
        let mut bytes = self.outgoing_bytes;
        for message in messages {
            let is_exit = message.get("method").and_then(Value::as_str) == Some("exit");
            let remaining = self
                .options
                .outgoing_bytes
                .checked_sub(bytes)
                .ok_or(Failure::Capacity)?;
            let payload = BoundedJsonPayload::serialize(
                &message,
                remaining.min(self.options.frame_limits.body_bytes()),
            )?;
            bytes = bytes.checked_add(payload.len()).ok_or(Failure::Capacity)?;
            outgoing.push(OutgoingFrame { payload, is_exit });
        }
        self.outgoing
            .try_reserve(outgoing.len())
            .map_err(|_| Failure::Capacity)?;
        self.outgoing.extend(outgoing);
        self.outgoing_bytes = bytes;
        Ok(())
    }

    fn apply(&mut self, outcome: Outcome) -> Result<(), Failure> {
        for completion in outcome.completed {
            self.progress.finish_request(completion.id);
            if let Some(call) = self.pending.remove(&completion.id) {
                let result = completion.result.and_then(|result| {
                    feature::validate_response(&call.method, &result)?;
                    Ok(result)
                });
                call.reply.send(result).ok();
            }
        }
        self.enqueue(outcome.outgoing)
    }

    async fn reap(&mut self) -> Result<(), Failure> {
        self.initialize_retry = None;
        self.server_pending.clear();
        self.progress.clear();
        self.outgoing.clear();
        self.outgoing_bytes = 0;
        self.writing = None;
        self.messages = None;
        self.exited = None;
        if let Some(process) = self.process.take() {
            process.0.kill();
            process.0.wait_for_completion().await;
            if !process.0.is_exited() {
                self.is_terminal_error = true;
                return Err(Failure::TransportClosed);
            }
        }
        Ok(())
    }

    async fn disconnected(&mut self) {
        let reaped = self.reap().await;
        self.is_terminal_error |= reaped.is_err();
        if let Ok(outcome) = self
            .coordinator
            .process_disconnected(self.coordinator.generation())
        {
            self.apply(outcome).ok();
        }
        if self.coordinator.phase() == Phase::Stopping {
            let stopped = if self.is_terminal_error {
                Err(Failure::TransportClosed)
            } else {
                reaped.and_then(|_| self.coordinator.finish_stop(self.coordinator.generation()))
            };
            self.is_terminal_error |= stopped.is_err();
            self.publish();
            if let Some(reply) = self.stop_reply.take() {
                reply.send(stopped.map(|_| Value::Null)).ok();
            }
        }
        self.exit_deadline = None;
        self.stop_deadline = None;
        self.publish();
    }

    fn next_deadline(&self) -> Option<Instant> {
        let request = self
            .coordinator
            .request_deadline()
            .and_then(|ms| self.origin.checked_add(Duration::from_millis(ms)));
        [
            request,
            self.writing.as_ref().map(|writing| writing.deadline),
            self.exit_deadline,
            self.stop_deadline,
            self.initialize_retry,
            self.server_pending.values().map(|call| call.deadline).min(),
        ]
        .into_iter()
        .flatten()
        .min()
    }

    fn handle_message(&mut self, inbound: Inbound) -> Result<(), Failure> {
        let message: Value =
            serde_json::from_str(&inbound.payload).map_err(|_| Failure::MalformedResponse)?;
        match protocol::decode(&message)? {
            IncomingMessage::Request { id, request } => {
                if self.server_pending.values().any(|call| call.id == id) {
                    return Err(Failure::MalformedResponse);
                }
                let request = match request {
                    Ok(request) => request,
                    Err(rejection) => return self.enqueue(vec![rejection.response(&id)]),
                };
                let registration = match &request {
                    ServerRequestKind::Register(params) => Some(
                        self.coordinator
                            .register_capabilities(self.coordinator.generation(), params.clone()),
                    ),
                    ServerRequestKind::Unregister(params) => Some(
                        self.coordinator
                            .unregister_capabilities(self.coordinator.generation(), params.clone()),
                    ),
                    _ => None,
                };
                if let Some(result) = registration {
                    let response = match result {
                        Ok(()) => serde_json::json!({"jsonrpc":"2.0","id":id,"result":null}),
                        Err(Failure::UnsupportedCapability) => Rejection::Unsupported.response(&id),
                        Err(Failure::Capacity | Failure::CounterOverflow) => {
                            Rejection::Unavailable.response(&id)
                        }
                        Err(_) => Rejection::InvalidParams.response(&id),
                    };
                    return self.enqueue(vec![response]);
                }
                if self.notifications.is_closed()
                    || self.server_pending.len() >= self.options.incoming_capacity
                    || matches!(
                        self.coordinator.phase(),
                        Phase::Stopping | Phase::Stopped | Phase::Degraded
                    )
                {
                    return self.enqueue(vec![Rejection::Unavailable.response(&id)]);
                }
                let serial = self
                    .server_serial
                    .checked_add(1)
                    .ok_or(Failure::CounterOverflow)?;
                let deadline = Instant::now()
                    .checked_add(Duration::from_millis(self.options.request_timeout_ms))
                    .ok_or(Failure::CounterOverflow)?;
                let request = Arc::new(request);
                let bytes = Arc::new(inbound._bytes);
                let ticket = ServerTicket {
                    generation: self.coordinator.generation(),
                    serial,
                    scope: self.server_scope.clone(),
                    id: id.clone(),
                };
                let notice = SessionNotice {
                    generation: self.coordinator.generation(),
                    message,
                    kind: SessionNoticeKind::Request(ServerRequest {
                        ticket,
                        kind: request.clone(),
                    }),
                    _bytes: bytes.clone(),
                };
                if self.notifications.try_send(notice).is_err() {
                    return self.enqueue(vec![Rejection::Unavailable.response(&id)]);
                }
                self.server_serial = serial;
                self.server_pending.insert(
                    serial,
                    PendingServerCall {
                        id,
                        request,
                        deadline,
                        _bytes: bytes,
                    },
                );
                Ok(())
            }
            IncomingMessage::Notification(mut notification) => {
                if let ServerNotification::Progress { token, value } = &mut notification {
                    self.progress.classify(token, value)?;
                }
                if let ServerNotification::Cancel(params) = &notification {
                    if let Some(serial) = self
                        .server_pending
                        .iter()
                        .find_map(|(serial, call)| (call.id == params.id).then_some(*serial))
                    {
                        let call = self
                            .server_pending
                            .remove(&serial)
                            .ok_or(Failure::StaleRequest)?;
                        self.enqueue(vec![Rejection::Cancelled.response(&call.id)])?;
                    }
                }
                self.notifications
                    .try_send(SessionNotice {
                        generation: self.coordinator.generation(),
                        message,
                        kind: SessionNoticeKind::Notification(notification),
                        _bytes: Arc::new(inbound._bytes),
                    })
                    .map_err(|_| Failure::Capacity)
            }
            IncomingMessage::Response => {
                let outcome =
                    self.coordinator
                        .receive(self.coordinator.generation(), self.now(), message)?;
                self.apply(outcome)
            }
        }
    }

    fn expire_server_requests(&mut self) -> Result<(), Failure> {
        let now = Instant::now();
        let expired = self
            .server_pending
            .iter()
            .filter_map(|(serial, call)| (call.deadline <= now).then_some(*serial))
            .collect::<Vec<_>>();
        for serial in expired {
            let call = self
                .server_pending
                .remove(&serial)
                .ok_or(Failure::StaleRequest)?;
            self.enqueue(vec![Rejection::Unavailable.response(&call.id)])?;
        }
        Ok(())
    }

    fn start_write(&mut self) -> Result<(), Failure> {
        if self.writing.is_some() {
            return Ok(());
        }
        let Some(frame) = self.outgoing.pop_front() else {
            return Ok(());
        };
        self.outgoing_bytes -= frame.payload.len();
        let process = self.process.as_ref().ok_or(Failure::TransportClosed)?;
        let receipt = process
            .0
            .submit_message(&frame.payload)
            .map_err(|_| Failure::TransportClosed)?;
        self.writing = Some(PendingWrite {
            receipt,
            deadline: Instant::now()
                .checked_add(self.options.write_timeout)
                .ok_or(Failure::CounterOverflow)?,
            is_exit: frame.is_exit,
        });
        Ok(())
    }

    async fn handle_command(&mut self, envelope: Envelope) -> Result<(), Failure> {
        let Envelope {
            command,
            reply,
            cancelled,
            _bytes,
        } = envelope;
        match command {
            Command::WorkspaceRoot(root) => {
                let result = (|| {
                    if self.coordinator.phase() != Phase::Running {
                        return Err(Failure::InvalidPhase);
                    }
                    if root.is_empty() {
                        return Err(Failure::MalformedRequest);
                    }
                    let message: Value =
                        serde_json::from_str(&crate::protocol::workspace_folders_notification(
                            std::slice::from_ref(&root),
                            &[],
                        ))
                        .map_err(|_| Failure::MalformedRequest)?;
                    let folder = &message["params"]["event"]["added"][0];
                    let mut initialize = self.initialize.clone();
                    let folders = initialize["workspaceFolders"]
                        .as_array_mut()
                        .ok_or(Failure::MalformedRequest)?;
                    if folders
                        .iter()
                        .any(|existing| existing["uri"] == folder["uri"])
                    {
                        return Ok(());
                    }
                    folders.push(folder.clone());
                    let mut size = PayloadSize::new(self.options.command_bytes);
                    size.value(&initialize, 0)?;
                    BoundedJsonPayload::serialize(
                        &initialize,
                        self.options.frame_limits.body_bytes(),
                    )?;
                    self.enqueue(vec![message])?;
                    self.initialize = initialize;
                    Ok(())
                })();
                let failed = result.clone();
                reply.send(result.map(|_| Value::Null)).ok();
                if matches!(failed, Err(Failure::Capacity)) {
                    return failed;
                }
            }
            Command::Reply { ticket, response } => {
                let result = if ticket.generation != self.coordinator.generation() {
                    Err(Failure::StaleGeneration)
                } else if !Arc::ptr_eq(&ticket.scope, &self.server_scope) {
                    Err(Failure::StaleRequest)
                } else if let Some(call) = self.server_pending.get(&ticket.serial) {
                    if call.deadline <= Instant::now() {
                        self.expire_server_requests()?;
                        Err(Failure::StaleRequest)
                    } else {
                        let token = match (&response, &*call.request) {
                            (
                                ServerReply::Acknowledged,
                                ServerRequestKind::CreateProgress(params),
                            ) => Some(params.token.clone()),
                            _ => None,
                        };
                        if token
                            .as_ref()
                            .is_some_and(|token| !self.progress.can_register_server(token))
                        {
                            Err(Failure::UnsupportedCapability)
                        } else {
                            response
                                .response(&call.id, &call.request)
                                .and_then(|message| self.enqueue(vec![message]))
                                .map(|()| {
                                    self.server_pending.remove(&ticket.serial);
                                    if let Some(token) = token {
                                        self.progress.register_server(token);
                                    }
                                })
                        }
                    }
                } else {
                    Err(Failure::StaleRequest)
                };
                let failed = result.clone();
                reply.send(result.map(|_| Value::Null)).ok();
                if matches!(failed, Err(Failure::Capacity)) {
                    return failed;
                }
            }
            Command::Request {
                method,
                params,
                document,
            } => {
                if cancelled.load(Ordering::SeqCst) {
                    reply.send(Err(Failure::Cancelled)).ok();
                    return Ok(());
                }
                if let Err(error) = feature::validate_request(&method, &params) {
                    reply.send(Err(error)).ok();
                    return Ok(());
                }
                let progress = match self.progress.prepare_request(&params) {
                    Ok(progress) => progress,
                    Err(error) => {
                        reply.send(Err(error)).ok();
                        return Ok(());
                    }
                };
                let document = document
                    .as_ref()
                    .map(|(uri, revision)| (uri.as_str(), *revision));
                match self
                    .coordinator
                    .request(self.now(), &method, params, document)
                {
                    Ok(message) => {
                        let id =
                            decode_request_id(&message["id"])?.ok_or(Failure::MalformedResponse)?;
                        self.progress.register_request(id, progress);
                        self.pending.insert(
                            id,
                            PendingCall {
                                method,
                                reply,
                                cancelled,
                            },
                        );
                        self.enqueue(vec![message])?;
                    }
                    Err(error) => {
                        reply.send(Err(error)).ok();
                    }
                }
            }
            Command::Open(document) => {
                let result = self
                    .coordinator
                    .open(document)
                    .and_then(|message| self.enqueue(message.into_iter().collect()));
                let failed = result.clone();
                reply.send(result.map(|_| Value::Null)).ok();
                if matches!(failed, Err(Failure::Capacity)) {
                    return failed;
                }
            }
            notification @ (Command::Close(_) | Command::Saved(_)) => {
                let result = match notification {
                    Command::Close(uri) => self.coordinator.close(&uri),
                    Command::Saved(uri) => self.coordinator.saved(&uri),
                    _ => unreachable!(),
                }
                .and_then(|message| self.enqueue(message.into_iter().collect()));
                let failed = result.clone();
                reply.send(result.map(|_| Value::Null)).ok();
                if matches!(failed, Err(Failure::Capacity)) {
                    return failed;
                }
            }
            Command::Change {
                uri,
                revision,
                next_revision,
                text,
            } => {
                let result = self
                    .coordinator
                    .change(&uri, revision, next_revision, text)
                    .and_then(|message| self.enqueue(message.into_iter().collect()));
                let failed = result.clone();
                reply.send(result.map(|_| Value::Null)).ok();
                if matches!(failed, Err(Failure::Capacity)) {
                    return failed;
                }
            }
            Command::Restart(config) => {
                if matches!(
                    self.coordinator.phase(),
                    Phase::Detected | Phase::Stopping | Phase::Stopped
                ) {
                    reply.send(Err(Failure::InvalidPhase)).ok();
                    return Ok(());
                }
                self.reap().await?;
                let outcome = self
                    .coordinator
                    .restart(self.now(), self.initialize.clone())?;
                self.initialize_attempts = 1;
                for message in &outcome.outgoing {
                    if message["method"] == "initialize" {
                        self.register_initialize_progress(message)?;
                    }
                }
                let result = self.spawn(config).and_then(|_| self.apply(outcome));
                reply.send(result.clone().map(|_| Value::Null)).ok();
                result?;
            }
            Command::Stop => {
                self.initialize_retry = None;
                let pending = std::mem::take(&mut self.server_pending);
                self.enqueue(
                    pending
                        .values()
                        .map(|call| Rejection::Cancelled.response(&call.id))
                        .collect(),
                )?;
                match self.coordinator.stop(self.now()) {
                    Ok(outcome) => {
                        self.stop_reply = Some(reply);
                        self.stop_deadline = Instant::now().checked_add(
                            Duration::from_millis(self.options.request_timeout_ms)
                                .saturating_add(self.options.exit_grace)
                                .saturating_add(self.options.write_timeout),
                        );
                        self.apply(outcome)?;
                        if self.process.is_none() {
                            self.disconnected().await;
                        }
                    }
                    Err(error) => {
                        reply.send(Err(error)).ok();
                    }
                }
            }
        }
        self.publish();
        Ok(())
    }

    pub async fn run(mut self) {
        let initialize = self.coordinator.begin(self.now(), self.initialize.clone());
        let starting = initialize.and_then(|message| {
            self.register_initialize_progress(&message)?;
            let config = self.config.take().ok_or(Failure::InvalidPhase)?;
            self.spawn(config)?;
            self.enqueue(vec![message])
        });
        if starting.is_err() {
            self.disconnected().await;
            if self.coordinator.phase() == Phase::Detected {
                self.is_terminal_error = true;
            }
        }
        self.publish();
        while !self.is_terminal_error && self.coordinator.phase() != Phase::Stopped {
            if self.writing.is_none()
                && self.outgoing.is_empty()
                && self.coordinator.phase() == Phase::Replaying
            {
                match self
                    .coordinator
                    .finish_replay(self.coordinator.generation())
                    .and_then(|messages| self.enqueue(messages))
                {
                    Ok(()) => self.publish(),
                    Err(_) => {
                        self.disconnected().await;
                        continue;
                    }
                }
            }
            if self.start_write().is_err() {
                self.disconnected().await;
                continue;
            }
            let deadline = self.next_deadline();
            let was_initializing = self.coordinator.phase() == Phase::Initializing;
            let event = tokio::select! {
                biased;
                _ = wait_deadline(deadline) => Event::Deadline,
                _ = self.cancellation.notified() => Event::Cancelled,
                message = receive_message(&mut self.messages) => Event::Message(message),
                result = receive_write(&mut self.writing) => Event::Written(result),
                _ = receive_exit(&mut self.exited) => Event::Exited,
                command = self.commands.recv(), if self.has_commands => Event::Command(command),
            };
            let result = match event {
                Event::Command(Some(command)) => self.handle_command(command).await,
                Event::Command(None) => {
                    self.has_commands = false;
                    let (reply, _) = oneshot::channel();
                    self.handle_command(Envelope {
                        command: Command::Stop,
                        reply,
                        cancelled: Arc::new(AtomicBool::new(false)),
                        _bytes: None,
                    })
                    .await
                }
                Event::Message(Some(message)) => self.handle_message(message),
                Event::Message(None) | Event::Exited => {
                    self.disconnected().await;
                    Ok(())
                }
                Event::Written(result) => {
                    let was_exit = self.writing.take().is_some_and(|writing| writing.is_exit);
                    if was_exit {
                        self.exit_deadline = Instant::now().checked_add(self.options.exit_grace);
                    }
                    result
                }
                Event::Deadline => {
                    let now = Instant::now();
                    if [
                        self.exit_deadline,
                        self.stop_deadline,
                        self.writing.as_ref().map(|writing| writing.deadline),
                    ]
                    .into_iter()
                    .flatten()
                    .any(|deadline| deadline <= now)
                    {
                        self.disconnected().await;
                        Ok(())
                    } else {
                        self.expire_server_requests().and_then(|()| {
                            if self
                                .initialize_retry
                                .is_some_and(|deadline| deadline <= now)
                            {
                                self.initialize_retry = None;
                                self.initialize_attempts += 1;
                                let message = self
                                    .coordinator
                                    .retry_initialize(self.now(), self.initialize.clone())?;
                                self.register_initialize_progress(&message)?;
                                return self.enqueue(vec![message]);
                            }
                            self.coordinator
                                .expire(self.now())
                                .and_then(|outcome| self.apply(outcome))
                        })
                    }
                }
                Event::Cancelled => {
                    let cancelled = self
                        .pending
                        .iter()
                        .filter_map(|(id, call)| {
                            call.cancelled.load(Ordering::SeqCst).then_some(*id)
                        })
                        .collect::<Vec<_>>();
                    let mut result = Ok(());
                    for id in cancelled {
                        result = self
                            .coordinator
                            .cancel(id)
                            .and_then(|outcome| self.apply(outcome));
                        if result.is_err() {
                            break;
                        }
                    }
                    result
                }
            };
            let should_retry = was_initializing
                && self.coordinator.phase() == Phase::Degraded
                && self.coordinator.generation() > 0
                && self.process.is_some();
            if should_retry {
                if self.initialize_attempts < REINITIALIZE_MAX_ATTEMPTS {
                    self.initialize_retry = Instant::now().checked_add(REINITIALIZE_RETRY_DELAY);
                    self.progress.clear();
                } else {
                    self.coordinator.degrade(Failure::ReinitializeExhausted);
                }
            }
            if result.is_err() && self.initialize_retry.is_none() {
                self.disconnected().await;
            }
            if self.coordinator.phase() == Phase::Degraded
                && self.process.is_some()
                && self.initialize_retry.is_none()
            {
                self.disconnected().await;
            }
            self.publish();
        }
        self.commands.close();
    }

    fn register_initialize_progress(&mut self, message: &Value) -> Result<(), Failure> {
        let progress = self.progress.prepare_request(&message["params"])?;
        let id = decode_request_id(&message["id"])?.ok_or(Failure::MalformedRequest)?;
        self.progress.register_request(id, progress);
        Ok(())
    }
}

async fn receive_message(messages: &mut Option<mpsc::Receiver<Inbound>>) -> Option<Inbound> {
    match messages {
        Some(messages) => messages.recv().await,
        None => pending().await,
    }
}

async fn receive_exit(exited: &mut Option<oneshot::Receiver<()>>) {
    match exited {
        Some(exited) => {
            exited.await.ok();
        }
        None => pending().await,
    }
}

async fn receive_write(writing: &mut Option<PendingWrite>) -> Result<(), Failure> {
    match writing {
        Some(writing) => writing
            .receipt
            .wait_ref()
            .await
            .map_err(|_| Failure::TransportClosed),
        None => pending().await,
    }
}

async fn wait_deadline(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => pending().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    const TEST_BYTES: usize = 1024;
    const TEST_FRAMES: usize = 16;
    const TEST_TIMEOUT_MS: u64 = 500;

    #[tokio::test]
    async fn 문서별_기능_공급은_정적과_dynamic_selector_재등록_닫힘_채널종료를_확인한다() {
        let initialize =
            json!({"capabilities":{"textDocument":{"references":{"dynamicRegistration":true}}}});
        let (client, mut runner) = prepare(initialize.clone());
        let rust_uri = "file:///synthetic/rust.rs";
        let text_uri = "file:///synthetic/text.txt";
        for (uri, language_id) in [(rust_uri, "rust"), (text_uri, "plaintext")] {
            runner
                .coordinator
                .open(DocumentMirror {
                    uri: uri.into(),
                    language_id: language_id.into(),
                    revision: 0,
                    version: 0,
                    text: "synthetic".into(),
                })
                .unwrap();
        }
        let init = runner.coordinator.begin(0, initialize).unwrap();
        runner.coordinator.receive(0, 1, json!({"jsonrpc":"2.0","id":init["id"],"result":{"capabilities":{"textDocumentSync":1,"definitionProvider":true}}})).unwrap();
        runner.coordinator.finish_replay(0).unwrap();
        runner.coordinator.register_capabilities(0, serde_json::from_value(json!({"registrations":[{"id":"references","method":"textDocument/references","registerOptions":{"documentSelector":[{"language":"rust","scheme":"file","pattern":"**/*.rs"}]}}]})).unwrap()).unwrap();
        runner.publish();
        let before = client.snapshot();
        assert!(before.supports_document(rust_uri, "textDocument/definition"));
        assert!(before.supports_document(text_uri, "textDocument/definition"));
        assert!(before.supports_document(rust_uri, "textDocument/references"));
        assert!(!before.supports_document(text_uri, "textDocument/references"));
        assert!(!before.supports_document("file:///synthetic/closed.rs", "textDocument/definition"));
        runner
            .coordinator
            .change(rust_uri, 0, 1, "new".into())
            .unwrap();
        runner.publish();
        assert!(Arc::ptr_eq(
            &before.document_methods,
            &client.snapshot().document_methods
        ));
        runner.coordinator.unregister_capabilities(0, serde_json::from_value(json!({"unregisterations":[{"id":"references","method":"textDocument/references"}]})).unwrap()).unwrap();
        runner.publish();
        assert!(!client
            .snapshot()
            .supports_document(rust_uri, "textDocument/references"));
        assert!(!Arc::ptr_eq(
            &before.document_methods,
            &client.snapshot().document_methods
        ));
        runner.coordinator.close(rust_uri).unwrap();
        runner.publish();
        assert!(!client
            .snapshot()
            .supports_document(rust_uri, "textDocument/definition"));
        drop(runner);
        assert!(!client
            .snapshot()
            .supports_document(text_uri, "textDocument/definition"));
        assert!(client.snapshot().document_methods.is_empty());
    }

    #[tokio::test]
    async fn signature_trigger는_정적과_문서별_동적_옵션_캐시와_회수를_보존한다() {
        let initialize =
            json!({"capabilities":{"textDocument":{"signatureHelp":{"dynamicRegistration":true}}}});
        let (client, mut runner) = prepare(initialize.clone());
        let rust_uri = "file:///synthetic/rust.rs";
        let text_uri = "file:///synthetic/text.txt";
        for (uri, language_id) in [(rust_uri, "rust"), (text_uri, "plaintext")] {
            runner
                .coordinator
                .open(DocumentMirror {
                    uri: uri.into(),
                    language_id: language_id.into(),
                    revision: 0,
                    version: 0,
                    text: "synthetic".into(),
                })
                .unwrap();
        }
        let init = runner.coordinator.begin(0, initialize).unwrap();
        runner
            .coordinator
            .receive(
                0,
                1,
                json!({"jsonrpc":"2.0","id":init["id"],"result":{"capabilities":{
                    "textDocumentSync":1,"signatureHelpProvider":{"triggerCharacters":["("]}
                }}}),
            )
            .unwrap();
        runner.coordinator.finish_replay(0).unwrap();
        runner.coordinator.register_capabilities(0, serde_json::from_value(json!({"registrations":[{
            "id":"signature", "method":"textDocument/signatureHelp", "registerOptions":{
                "documentSelector":[{"language":"rust","scheme":"file","pattern":"**/*.rs"}],
                "triggerCharacters":[","],"retriggerCharacters":[")"]
            }
        }]})).unwrap()).unwrap();
        runner.publish();
        let before = client.snapshot();
        let rust = before.document_signature_options.get(rust_uri).unwrap();
        assert_eq!(rust.len(), 2);
        assert_eq!(
            rust[0].trigger_characters.as_deref(),
            Some(["(".to_owned()].as_slice())
        );
        assert_eq!(
            rust[1].trigger_characters.as_deref(),
            Some([",".to_owned()].as_slice())
        );
        assert_eq!(
            rust[1].retrigger_characters.as_deref(),
            Some([")".to_owned()].as_slice())
        );
        assert_eq!(
            before
                .document_signature_options
                .get(text_uri)
                .unwrap()
                .len(),
            1
        );
        assert!(!before
            .document_signature_options
            .contains_key("file:///synthetic/closed.rs"));
        runner
            .coordinator
            .change(rust_uri, 0, 1, "changed".into())
            .unwrap();
        runner.publish();
        assert!(Arc::ptr_eq(
            &before.document_signature_options,
            &client.snapshot().document_signature_options
        ));
        runner
            .coordinator
            .unregister_capabilities(
                0,
                serde_json::from_value(json!({"unregisterations":[{
                    "id":"signature", "method":"textDocument/signatureHelp"
                }]}))
                .unwrap(),
            )
            .unwrap();
        runner.publish();
        assert_eq!(
            client
                .snapshot()
                .document_signature_options
                .get(rust_uri)
                .unwrap()
                .len(),
            1
        );
        assert!(!Arc::ptr_eq(
            &before.document_signature_options,
            &client.snapshot().document_signature_options
        ));
        runner.coordinator.close(rust_uri).unwrap();
        runner.publish();
        assert!(!client
            .snapshot()
            .document_signature_options
            .contains_key(rust_uri));
        drop(runner);
        assert!(client.snapshot().document_signature_options.is_empty());
    }

    #[tokio::test]
    async fn on_type_formatting은_정적과_문서별_동적_trigger_캐시와_회수를_보존한다() {
        let initialize = json!({"capabilities":{"textDocument":{"onTypeFormatting":{"dynamicRegistration":true}}}});
        let (client, mut runner) = prepare(initialize.clone());
        let rust_uri = "file:///synthetic/format.rs";
        let text_uri = "file:///synthetic/format.txt";
        for (uri, language_id) in [(rust_uri, "rust"), (text_uri, "plaintext")] {
            runner
                .coordinator
                .open(DocumentMirror {
                    uri: uri.into(),
                    language_id: language_id.into(),
                    revision: 0,
                    version: 0,
                    text: "synthetic".into(),
                })
                .unwrap();
        }
        let init = runner.coordinator.begin(0, initialize).unwrap();
        runner.coordinator.receive(0, 1, json!({"jsonrpc":"2.0","id":init["id"],"result":{"capabilities":{"textDocumentSync":1,"documentOnTypeFormattingProvider":{"firstTriggerCharacter":";","moreTriggerCharacter":["\n"]}}}})).unwrap();
        runner.coordinator.finish_replay(0).unwrap();
        runner.coordinator.register_capabilities(0, serde_json::from_value(json!({"registrations":[{"id":"format","method":"textDocument/onTypeFormatting","registerOptions":{"documentSelector":[{"language":"rust","scheme":"file","pattern":"**/*.rs"}],"firstTriggerCharacter":"}"}}]})).unwrap()).unwrap();
        runner.publish();
        let before = client.snapshot();
        let rust = before
            .document_on_type_formatting_options
            .get(rust_uri)
            .unwrap();
        assert_eq!(rust.len(), 2);
        assert_eq!(rust[0].first_trigger_character, ";");
        assert_eq!(
            rust[0].more_trigger_character.as_deref(),
            Some(["\n".to_owned()].as_slice())
        );
        assert_eq!(rust[1].first_trigger_character, "}");
        assert_eq!(
            before
                .document_on_type_formatting_options
                .get(text_uri)
                .unwrap()
                .len(),
            1
        );
        runner
            .coordinator
            .change(rust_uri, 0, 1, "changed".into())
            .unwrap();
        runner.publish();
        assert!(Arc::ptr_eq(
            &before.document_on_type_formatting_options,
            &client.snapshot().document_on_type_formatting_options
        ));
        runner.coordinator.unregister_capabilities(0, serde_json::from_value(json!({"unregisterations":[{"id":"format","method":"textDocument/onTypeFormatting"}]})).unwrap()).unwrap();
        runner.publish();
        assert_eq!(
            client
                .snapshot()
                .document_on_type_formatting_options
                .get(rust_uri)
                .unwrap()
                .len(),
            1
        );
        assert!(!Arc::ptr_eq(
            &before.document_on_type_formatting_options,
            &client.snapshot().document_on_type_formatting_options
        ));
        runner.coordinator.close(rust_uri).unwrap();
        runner.publish();
        assert!(!client
            .snapshot()
            .document_on_type_formatting_options
            .contains_key(rust_uri));
        drop(runner);
        assert!(client
            .snapshot()
            .document_on_type_formatting_options
            .is_empty());
    }

    #[tokio::test]
    async fn completion_trigger는_문서별_정적과_동적_옵션_캐시와_회수를_보존한다() {
        let initialize =
            json!({"capabilities":{"textDocument":{"completion":{"dynamicRegistration":true}}}});
        let (client, mut runner) = prepare(initialize.clone());
        let rust_uri = "file:///synthetic/rust.rs";
        let text_uri = "file:///synthetic/text.txt";
        for (uri, language_id) in [(rust_uri, "rust"), (text_uri, "plaintext")] {
            runner
                .coordinator
                .open(DocumentMirror {
                    uri: uri.into(),
                    language_id: language_id.into(),
                    revision: 0,
                    version: 0,
                    text: "synthetic".into(),
                })
                .unwrap();
        }
        let init = runner.coordinator.begin(0, initialize).unwrap();
        runner.coordinator.receive(0, 1, json!({"jsonrpc":"2.0","id":init["id"],"result":{"capabilities":{
            "textDocumentSync":1, "completionProvider":{"triggerCharacters":["."], "resolveProvider":true}
        }}})).unwrap();
        runner.coordinator.finish_replay(0).unwrap();
        let registrations = serde_json::from_value(json!({"registrations":[{
            "id":"completion", "method":"textDocument/completion", "registerOptions":{
                "documentSelector":[{"language":"rust","scheme":"file","pattern":"**/*.rs"}],
                "triggerCharacters":[":"], "allCommitCharacters":[";"],
                "completionItem":{"labelDetailsSupport":true}
            }
        }]}))
        .unwrap();
        runner
            .coordinator
            .register_capabilities(0, registrations)
            .unwrap();
        runner.publish();
        let before = client.snapshot();
        let rust = before.document_completion_options.get(rust_uri).unwrap();
        assert_eq!(rust.len(), 2);
        assert_eq!(
            rust[0].trigger_characters.as_deref(),
            Some([".".to_owned()].as_slice())
        );
        assert_eq!(rust[0].resolve_provider, Some(true));
        assert_eq!(
            rust[1].trigger_characters.as_deref(),
            Some([":".to_owned()].as_slice())
        );
        assert_eq!(
            rust[1].all_commit_characters.as_deref(),
            Some([";".to_owned()].as_slice())
        );
        assert_eq!(
            rust[1]
                .completion_item
                .as_ref()
                .unwrap()
                .label_details_support,
            Some(true)
        );
        assert!(before.supports_document(rust_uri, "textDocument/completion"));
        assert_eq!(
            before
                .document_completion_options
                .get(text_uri)
                .unwrap()
                .len(),
            1
        );
        assert!(!before
            .document_completion_options
            .contains_key("file:///synthetic/closed.rs"));
        runner
            .coordinator
            .change(rust_uri, 0, 1, "changed".into())
            .unwrap();
        runner.publish();
        assert!(Arc::ptr_eq(
            &before.document_completion_options,
            &client.snapshot().document_completion_options
        ));
        runner
            .coordinator
            .unregister_capabilities(
                0,
                serde_json::from_value(json!({"unregisterations":[{
                    "id":"completion", "method":"textDocument/completion"
                }]}))
                .unwrap(),
            )
            .unwrap();
        runner.publish();
        assert_eq!(
            client
                .snapshot()
                .document_completion_options
                .get(rust_uri)
                .unwrap()
                .len(),
            1
        );
        assert!(!Arc::ptr_eq(
            &before.document_completion_options,
            &client.snapshot().document_completion_options
        ));
        runner.coordinator.close(rust_uri).unwrap();
        runner.publish();
        assert!(!client
            .snapshot()
            .document_completion_options
            .contains_key(rust_uri));
        drop(runner);
        assert!(client.snapshot().document_completion_options.is_empty());
    }

    fn prepare(initialize: Value) -> (SessionClient, SessionRunner) {
        SessionRunner::prepare(
            LspStore::new(),
            LspProcConfig {
                command: "synthetic-no-spawn".into(),
                args: Vec::new(),
                cwd: PathBuf::from("."),
            },
            initialize,
            SessionOptions {
                frame_limits: FrameLimits::new(TEST_BYTES, TEST_BYTES).unwrap(),
                writer_limits: WriterLimits::new(TEST_FRAMES, TEST_BYTES).unwrap(),
                command_capacity: TEST_FRAMES,
                command_bytes: TEST_BYTES,
                incoming_capacity: TEST_FRAMES,
                incoming_bytes: TEST_BYTES,
                outgoing_capacity: TEST_FRAMES,
                outgoing_bytes: TEST_BYTES,
                request_timeout_ms: TEST_TIMEOUT_MS,
                write_timeout: Duration::from_millis(TEST_TIMEOUT_MS),
                exit_grace: Duration::from_millis(TEST_TIMEOUT_MS),
            },
        )
        .unwrap()
    }

    #[tokio::test]
    async fn reinitialize_backoff의_stop은_예약_timer와_pending_초기화를_취소한다() {
        let params = json!({"capabilities":{}});
        let (client, mut runner) = prepare(params.clone());
        runner.coordinator.begin(0, params.clone()).unwrap();
        runner.coordinator.restart(0, params).unwrap();
        runner.coordinator.expire(TEST_TIMEOUT_MS).unwrap();
        runner.initialize_attempts = REINITIALIZE_MAX_ATTEMPTS - 1;
        runner.initialize_retry = Some(Instant::now() + REINITIALIZE_RETRY_DELAY);
        runner.publish();
        let retrying = client.snapshot();
        assert_eq!(retrying.phase, Phase::Initializing);
        assert_eq!(retrying.failure, None);
        assert_eq!(retrying.generation, 1);
        assert_eq!(runner.next_deadline(), runner.initialize_retry);
        let (reply, receive) = oneshot::channel();
        runner
            .handle_command(Envelope {
                command: Command::Stop,
                reply,
                cancelled: Arc::new(AtomicBool::new(false)),
                _bytes: None,
            })
            .await
            .unwrap();
        assert_eq!(receive.await.unwrap(), Ok(Value::Null));
        assert!(runner.initialize_retry.is_none());
        assert!(runner.next_deadline().is_none());
        assert!(runner.outgoing.is_empty());
        assert_eq!(client.snapshot().phase, Phase::Stopped);
        assert_eq!(client.snapshot().pending, 0);
        assert_eq!(client.snapshot().generation, retrying.generation);
        assert_eq!(
            runner
                .coordinator
                .retry_initialize(runner.now(), runner.initialize.clone()),
            Err(Failure::InvalidPhase)
        );
    }

    #[tokio::test]
    async fn workspace_root_추가는_중복과_phase_aggregate_budget_거절에_초기화를_보존한다() {
        let first = json!({"uri":"file:///synthetic/first","name":"first"});
        let initialize = json!({"capabilities":{},"workspaceFolders":[first]});
        let (client, mut runner) = prepare(initialize.clone());
        let (finished, reply) = oneshot::channel();
        runner
            .handle_command(Envelope {
                command: Command::WorkspaceRoot("/synthetic/second".into()),
                reply: finished,
                cancelled: Arc::new(AtomicBool::new(false)),
                _bytes: None,
            })
            .await
            .unwrap();
        assert_eq!(reply.await.unwrap(), Err(Failure::InvalidPhase));
        assert_eq!(runner.initialize, initialize);
        runner.options.command_bytes = TEST_BYTES * TEST_FRAMES;
        let request = runner.coordinator.begin(0, initialize).unwrap();
        runner.coordinator.receive(0,0,json!({"jsonrpc":"2.0","id":request["id"],"result":{"capabilities":{"textDocumentSync":1}}})).unwrap();
        runner.coordinator.finish_replay(0).unwrap();
        for root in ["/synthetic/second", "/synthetic/second"] {
            let (finished, reply) = oneshot::channel();
            runner
                .handle_command(Envelope {
                    command: Command::WorkspaceRoot(root.into()),
                    reply: finished,
                    cancelled: Arc::new(AtomicBool::new(false)),
                    _bytes: None,
                })
                .await
                .unwrap();
            assert_eq!(reply.await.unwrap(), Ok(Value::Null));
        }
        assert_eq!(
            runner.initialize["workspaceFolders"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(runner.outgoing.len(), 1);
        let accepted = runner.initialize.clone();
        let outgoing_bytes = runner.outgoing_bytes;
        runner.options.command_bytes = 1;
        let (finished, reply) = oneshot::channel();
        assert_eq!(
            runner
                .handle_command(Envelope {
                    command: Command::WorkspaceRoot("/synthetic/third".into()),
                    reply: finished,
                    cancelled: Arc::new(AtomicBool::new(false)),
                    _bytes: None,
                })
                .await,
            Err(Failure::Capacity)
        );
        assert_eq!(reply.await.unwrap(), Err(Failure::Capacity));
        assert_eq!(runner.initialize, accepted);
        assert_eq!(runner.outgoing.len(), 1);
        assert_eq!(runner.outgoing_bytes, outgoing_bytes);
        let mut oversized = String::with_capacity(TEST_BYTES);
        oversized.push('/');
        assert_eq!(
            client.add_workspace_root(oversized).await,
            Err(Failure::Capacity)
        );
        assert!(runner.commands.try_recv().is_err());
    }

    #[tokio::test]
    async fn closed_owner_snapshot은_폐기된_pending_progress_registration을_활성으로_표시하지_않는다(
    ) {
        let (client, runner) = prepare(json!({"capabilities":{}}));
        let mut active = client.snapshot();
        active.phase = Phase::Running;
        active.pending = 1;
        active.server_pending = 1;
        active.progress_tokens = TEST_FRAMES;
        active.registrations = 1;
        let generation = active.generation;
        runner.state.send_replace(active);
        drop(runner);
        let ended = client.snapshot();
        assert_eq!(ended.phase, Phase::Degraded);
        assert_eq!(ended.failure, Some(Failure::TransportClosed));
        assert_eq!(ended.generation, generation);
        assert_eq!(ended.pending, 0);
        assert_eq!(ended.server_pending, 0);
        assert_eq!(ended.progress_tokens, 0);
        assert_eq!(ended.registrations, 0);
    }

    #[tokio::test]
    async fn running_대기는_초기화_실패와_종료를_즉시_반환하되_stop_join_대기는_유지한다() {
        use std::future::Future;
        use std::task::{Context, Poll, Waker};
        let (mut client, runner) = prepare(json!({"capabilities":{}}));
        for phase in [Phase::Degraded, Phase::Stopping, Phase::Stopped] {
            let mut snapshot = client.snapshot();
            snapshot.phase = phase;
            snapshot.failure = Some(Failure::TransportClosed);
            runner.state.send_replace(snapshot);
            assert_eq!(
                client.wait_for_phase(Phase::Running).await.unwrap_err(),
                Failure::TransportClosed
            );
        }
        let mut stopping = client.snapshot();
        stopping.phase = Phase::Stopping;
        stopping.failure = None;
        runner.state.send_replace(stopping.clone());
        let mut waiting = Box::pin(client.wait_for_phase(Phase::Stopped));
        let mut context = Context::from_waker(Waker::noop());
        assert!(waiting.as_mut().poll(&mut context).is_pending());
        stopping.phase = Phase::Stopped;
        runner.state.send_replace(stopping);
        assert!(
            matches!(waiting.as_mut().poll(&mut context), Poll::Ready(Ok(snapshot)) if snapshot.phase == Phase::Stopped)
        );
    }

    #[tokio::test]
    async fn outgoing_payload_budget은_batch_거절을_원자적으로_처리한다() {
        let (_client, mut runner) = prepare(json!({"capabilities":{}}));
        let accepted = json!({"jsonrpc":"2.0","method":"synthetic","params":"한글\n\""});
        let rejected =
            json!({"jsonrpc":"2.0","method":"synthetic","params":"x".repeat(TEST_BYTES)});
        assert_eq!(
            runner.enqueue(vec![accepted.clone(), rejected]),
            Err(Failure::Capacity)
        );
        assert_eq!(runner.outgoing_bytes, 0);
        assert!(runner.outgoing.is_empty());
        let encoded = serde_json::to_string(&accepted).unwrap();
        runner.options.frame_limits = FrameLimits::new(TEST_BYTES, encoded.len()).unwrap();
        runner.options.outgoing_bytes = encoded.len();
        runner.enqueue(vec![accepted.clone()]).unwrap();
        let payload = &runner.outgoing.front().unwrap().payload;
        assert_eq!(serde_json::from_str::<Value>(payload).unwrap(), accepted);
        assert_eq!(payload, &encoded);
        assert_eq!(runner.outgoing_bytes, payload.len());
        assert_eq!(
            runner.enqueue(vec![accepted.clone()]),
            Err(Failure::Capacity)
        );
        assert_eq!(runner.outgoing.len(), 1);
        assert_eq!(runner.outgoing_bytes, encoded.len());
        assert_eq!(runner.outgoing.pop_front().unwrap().payload, encoded);
        runner.outgoing_bytes = 0;
        runner.options.frame_limits = FrameLimits::new(TEST_BYTES, encoded.len() - 1).unwrap();
        assert_eq!(runner.enqueue(vec![accepted]), Err(Failure::Capacity));
        assert!(runner.outgoing.is_empty());
        assert_eq!(runner.outgoing_bytes, 0);
        let mut bounded = BoundedJsonPayload {
            bytes: Vec::new(),
            limit: TEST_BYTES,
        };
        bounded.write_all(&vec![b'x'; TEST_BYTES]).unwrap();
        assert!(bounded.write_all(b"x").is_err());
        assert_eq!(bounded.bytes.len(), TEST_BYTES);
        bounded.flush().unwrap();
    }

    #[tokio::test]
    async fn command_byte_budget은_포화_취소_envelope_폐기와_closed_경계를_구분한다() {
        use std::future::Future;
        use std::task::{Context, Waker};

        let (mut client, mut runner) = prepare(json!({"capabilities":{}}));
        let command = Command::Request {
            method: "textDocument/hover".into(),
            params: json!({"textDocument":{"uri":"file:///synthetic.rs"},"position":{"line":0,"character":0}}),
            document: None,
        };
        let bytes = command.payload_bytes(TEST_BYTES).unwrap();
        client.command_bytes = bytes;
        client.command_budget = Arc::new(Semaphore::new(bytes));
        let budget = client.command_budget.clone();
        let mut request = Box::pin(client.send(command));
        let mut context = Context::from_waker(Waker::noop());
        assert!(request.as_mut().poll(&mut context).is_pending());
        assert_eq!(budget.available_permits(), 0);
        assert_eq!(
            client
                .send(Command::Restart(LspProcConfig {
                    command: "synthetic".into(),
                    args: Vec::new(),
                    cwd: std::path::PathBuf::new()
                }))
                .await
                .unwrap_err(),
            Failure::Capacity
        );
        drop(request);
        assert_eq!(budget.available_permits(), 0);
        let envelope = runner.commands.try_recv().unwrap();
        assert!(envelope.cancelled.load(Ordering::SeqCst));
        assert!(envelope._bytes.is_some());
        assert_eq!(budget.available_permits(), 0);
        drop(envelope);
        assert_eq!(budget.available_permits(), bytes);
        let mut large = String::with_capacity(TEST_BYTES);
        large.push('x');
        assert_eq!(
            client
                .request("textDocument/hover".into(), Value::String(large), None)
                .await
                .unwrap_err(),
            Failure::Capacity
        );
        assert_eq!(budget.available_permits(), bytes);
        assert!(runner.commands.try_recv().is_err());
        let reservation = budget
            .clone()
            .try_acquire_many_owned(u32::try_from(bytes).unwrap())
            .unwrap();
        let mut stop = Box::pin(client.stop());
        assert!(stop.as_mut().poll(&mut context).is_pending());
        let envelope = runner.commands.try_recv().unwrap();
        assert!(matches!(envelope.command, Command::Stop));
        assert_eq!(budget.available_permits(), 0);
        let Envelope { reply, _bytes, .. } = envelope;
        reply.send(Ok(Value::Null)).unwrap();
        drop(_bytes);
        assert!(stop.as_mut().poll(&mut context).is_ready());
        drop(stop);
        drop(reservation);
        assert_eq!(budget.available_permits(), bytes);
        let mut queued = Vec::new();
        for _ in 0..TEST_FRAMES {
            let mut stop = Box::pin(client.stop());
            assert!(stop.as_mut().poll(&mut context).is_pending());
            queued.push(stop);
        }
        assert_eq!(client.stop().await.unwrap_err(), Failure::Capacity);
        assert_eq!(budget.available_permits(), bytes);
        drop(queued);
        for _ in 0..TEST_FRAMES {
            drop(runner.commands.try_recv().unwrap());
        }
        runner.commands.close();
        assert_eq!(
            client.send(Command::Stop).await.unwrap_err(),
            Failure::TransportClosed
        );
        assert_eq!(budget.available_permits(), bytes);
        let mut size = PayloadSize::new(TEST_BYTES);
        assert_eq!(size.add(usize::MAX), Err(Failure::Capacity));
        assert_eq!(size.slots::<Value>(usize::MAX), Err(Failure::Capacity));
        let mut values = Vec::with_capacity(TEST_BYTES);
        values.push(Value::Null);
        assert_eq!(
            Command::Request {
                method: "textDocument/hover".into(),
                params: Value::Array(values),
                document: None
            }
            .payload_bytes(TEST_BYTES),
            Err(Failure::Capacity)
        );
    }

    #[tokio::test]
    async fn duplicate_server_id는_자동_registration_응답보다_먼저_거절한다() {
        let initialize =
            json!({"capabilities":{"textDocument":{"hover":{"dynamicRegistration":true}}}});
        let (_client, mut runner) = prepare(initialize.clone());
        let init = runner.coordinator.begin(0, initialize).unwrap();
        runner
            .coordinator
            .receive(
                0,
                1,
                json!({"jsonrpc":"2.0","id":init["id"],"result":{"capabilities":{}}}),
            )
            .unwrap();
        runner.coordinator.finish_replay(0).unwrap();
        let inbound = |message: Value| {
            let payload = message.to_string();
            let permit = runner
                .incoming_budget
                .clone()
                .try_acquire_many_owned(u32::try_from(payload.len()).unwrap())
                .unwrap();
            Inbound {
                payload,
                _bytes: permit,
            }
        };
        let first = inbound(
            json!({"jsonrpc":"2.0","id":0,"method":"workspace/configuration","params":{"items":[]}}),
        );
        let duplicate = inbound(
            json!({"jsonrpc":"2.0","id":0,"method":"client/registerCapability","params":{"registrations":[{"id":"hover","method":"textDocument/hover","registerOptions":{"documentSelector":null}}]}}),
        );
        runner.handle_message(first).unwrap();
        assert_eq!(
            runner.handle_message(duplicate),
            Err(Failure::MalformedResponse)
        );
        assert_eq!(runner.coordinator.registration_count(), 0);
        assert_eq!(runner.server_pending.len(), 1);
        assert!(runner.outgoing.is_empty());
    }

    #[tokio::test]
    async fn wire_id_actor는_문자열_initialize_feature의_pending과_진행_token을_같은_identity로_정리한다(
    ) {
        let initialize = json!({"capabilities":{},"workDoneToken":"initialize"});
        let (_client, mut runner) = prepare(initialize.clone());
        runner.coordinator.next_id = u64::try_from(i32::MAX).unwrap();
        let init = runner.coordinator.begin(0, initialize).unwrap();
        assert!(init["id"].is_string());
        runner.register_initialize_progress(&init).unwrap();
        assert_eq!(runner.progress.len(), 1);
        let budget = runner.incoming_budget.clone();
        let inbound = |message: Value| {
            let payload = message.to_string();
            let permit = budget
                .clone()
                .try_acquire_many_owned(u32::try_from(payload.len()).unwrap())
                .unwrap();
            Inbound {
                payload,
                _bytes: permit,
            }
        };
        runner.handle_message(inbound(json!({"jsonrpc":"2.0","id":init["id"],"result":{"capabilities":{"hoverProvider":true}}}))).unwrap();
        assert_eq!(runner.progress.len(), 0);
        runner.coordinator.finish_replay(0).unwrap();
        let params = json!({"textDocument":{"uri":"file:///synthetic/request-id.rs"},"position":{"line":0,"character":0},"workDoneToken":"feature"});
        let (reply, receive) = oneshot::channel();
        runner
            .handle_command(Envelope {
                command: Command::Request {
                    method: "textDocument/hover".into(),
                    params: params.clone(),
                    document: None,
                },
                reply,
                cancelled: Arc::new(AtomicBool::new(false)),
                _bytes: None,
            })
            .await
            .unwrap();
        assert_eq!(runner.pending.len(), 1);
        assert_eq!(runner.progress.len(), 1);
        let request: Value =
            serde_json::from_str(&runner.outgoing.back().unwrap().payload).unwrap();
        assert!(request["id"].is_string());
        let result = json!({"contents":{"kind":"plaintext","value":"합성"},"experimental":{"retained":true}});
        runner
            .handle_message(inbound(
                json!({"jsonrpc":"2.0","id":request["id"],"result":result}),
            ))
            .unwrap();
        assert_eq!(receive.await.unwrap().unwrap(), result);
        assert!(runner.pending.is_empty());
        assert_eq!(runner.progress.len(), 0);
        let (reply, receive) = oneshot::channel();
        runner
            .handle_command(Envelope {
                command: Command::Request {
                    method: "textDocument/hover".into(),
                    params,
                    document: None,
                },
                reply,
                cancelled: Arc::new(AtomicBool::new(false)),
                _bytes: None,
            })
            .await
            .unwrap();
        let request: Value =
            serde_json::from_str(&runner.outgoing.back().unwrap().payload).unwrap();
        let id = *runner.pending.keys().next().unwrap();
        let cancelled = runner.coordinator.cancel(id).unwrap();
        runner.apply(cancelled).unwrap();
        assert_eq!(receive.await.unwrap().unwrap_err(), Failure::Cancelled);
        let notification: Value =
            serde_json::from_str(&runner.outgoing.back().unwrap().payload).unwrap();
        assert_eq!(notification["method"], "$/cancelRequest");
        assert_eq!(notification["params"]["id"], request["id"]);
        assert!(runner.pending.is_empty());
        assert_eq!(runner.progress.len(), 0);
    }
}

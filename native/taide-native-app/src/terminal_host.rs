use std::collections::HashMap;
use std::future::Future;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use taide_infra::pty::PtyCompletionHandle;
use taide_model::{
    app_event::AppEvent,
    error::{AppError, AppResult},
    ids::ProjectId,
    terminal::PtySpawnOptions,
};
use taide_native_terminal::{
    Size,
    input::{InputAction, NativeInput},
    session::{Failure as CoreFailure, Phase, SharedTerminal},
};
use taide_runtime::{
    AppServices,
    terminal_actions::{self, TerminalSpawnPorts},
};
use taide_terminal::metadata::TerminalSessionMetadata;
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinHandle;

use crate::{
    terminal_dispatch::{Dispatcher, EffectPorts, ObservePorts, SessionPorts},
    terminal_frames,
    terminal_writer::{self, Writer},
};

const CONTROL_CAPACITY_GROWTH: usize = 2;

#[derive(Clone, Copy)]
pub struct Limits {
    pub sessions: usize,
    pub core: taide_native_terminal::Limits,
    pub frames: terminal_frames::Limits,
    pub writer: terminal_writer::Limits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Transport,
    Effects,
    FinalFrame,
    Join,
    Supervisor,
    Resize,
}

pub enum InputResult {
    Write(terminal_writer::Receipt),
    Local(InputAction),
    Pending(PendingInput),
}

impl InputResult {
    fn with_input_recorded(mut self, recorded: bool) -> Self {
        if let Self::Pending(pending) = &mut self {
            pending.is_recorded = recorded;
        }
        self
    }
}

pub struct PendingInput {
    owner: Arc<()>,
    data: Vec<u8>,
    is_user_input: bool,
    is_recorded: bool,
    order: terminal_writer::Order,
}

pub(crate) struct PreparedInput {
    owner: Arc<()>,
    data: Vec<u8>,
    focus: Option<bool>,
    is_recorded: bool,
}

pub(crate) enum InputPreparation {
    Prepared(PreparedInput),
    Local(InputAction),
}

impl PreparedInput {
    pub(crate) fn is_focus(&self) -> bool {
        self.focus.is_some()
    }

    pub(crate) fn payload_capacity(&self) -> usize {
        self.data.capacity()
    }
}

impl PendingInput {
    pub(crate) fn can_append_control(&self, next: &Self) -> AppResult<bool> {
        if self.is_user_input || next.is_user_input || !Arc::ptr_eq(&self.owner, &next.owner) {
            return Ok(false);
        }
        self.order.is_next(&next.order)
    }
    pub fn retained_bytes(&self) -> Option<usize> {
        self.data.capacity().checked_add(size_of::<Self>())
    }

    pub(crate) fn append_control(&mut self, next: Self, limit: usize) -> AppResult<()> {
        if !self.can_append_control(&next)? {
            return Err(AppError::InvalidArgument(
                "native terminal control batch identity mismatch".into(),
            ));
        }
        let length = self
            .data
            .len()
            .checked_add(next.data.len())
            .filter(|length| *length <= limit)
            .ok_or_else(|| {
                AppError::InvalidArgument("native terminal focus report budget exceeded".into())
            })?;
        if length > self.data.capacity() {
            let capacity = self
                .data
                .capacity()
                .saturating_mul(CONTROL_CAPACITY_GROWTH)
                .max(length)
                .min(limit);
            let mut data = Vec::new();
            data.try_reserve_exact(capacity).map_err(|_| {
                AppError::Internal("native terminal control allocation failed".into())
            })?;
            if data.capacity() > limit {
                return Err(AppError::InvalidArgument(
                    "native terminal focus allocation budget exceeded".into(),
                ));
            }
            data.extend_from_slice(&self.data);
            data.extend_from_slice(&next.data);
            self.data = data;
            return Ok(());
        }
        self.data.extend_from_slice(&next.data);
        Ok(())
    }
}

enum InputRejection {
    Pending(Vec<u8>, terminal_writer::Order),
    Failed(AppError),
}

pub struct Session {
    id: String,
    terminal: SharedTerminal,
    metadata: Arc<TerminalSessionMetadata>,
    writer: Writer,
    input_identity: Arc<()>,
    frames: terminal_frames::Sender,
    writer_worker: tokio::sync::Mutex<Option<JoinHandle<()>>>,
    completion: PtyCompletionHandle,
    failure: Mutex<Option<Failure>>,
    is_dispatched: AtomicBool,
    changed: Notify,
    _admission: Arc<OwnedSemaphorePermit>,
}

impl Session {
    pub(crate) fn input_payload_limit(&self) -> usize {
        self.writer.payload_limit()
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn snapshot<R>(
        &self,
        read: impl FnOnce(taide_native_terminal::session::Snapshot<'_>) -> R,
    ) -> AppResult<R> {
        self.terminal.snapshot(read)
    }
    pub fn clear_current_row(&self) -> AppResult<bool> {
        self.terminal.clear_current_row()
    }
    pub fn configure_command_colors(
        &self,
        colors: taide_native_terminal::CommandColors,
    ) -> AppResult<()> {
        self.terminal.configure_command_colors(colors)
    }
    pub fn metadata(&self) -> &TerminalSessionMetadata {
        &self.metadata
    }
    pub fn failure(&self) -> Option<Failure> {
        *self
            .failure
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }
    pub fn is_finished(&self) -> bool {
        self.completion.is_finished() && self.is_dispatched.load(Ordering::Acquire)
    }

    pub async fn wait_dispatch(&self) -> AppResult<()> {
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.is_dispatched.load(Ordering::Acquire) {
                return match self.failure() {
                    Some(_) => Err(stopped()),
                    None => Ok(()),
                };
            }
            changed.await;
        }
    }

    pub fn input(
        &self,
        services: &AppServices,
        input: NativeInput<'_>,
        capacity: usize,
    ) -> AppResult<InputResult> {
        self.queue_input(services, input, capacity, false)
    }

    pub fn queue_input(
        &self,
        services: &AppServices,
        input: NativeInput<'_>,
        capacity: usize,
        defer: bool,
    ) -> AppResult<InputResult> {
        let is_user_input = !matches!(&input, NativeInput::Focus(_));
        let result = self
            .terminal
            .try_input(input, capacity, |action| {
                let InputAction::Write(data) = action else {
                    return Ok(InputResult::Local(action));
                };
                let order = self
                    .writer
                    .reserve_order()
                    .map_err(InputRejection::Failed)?;
                if defer {
                    return Err(InputRejection::Pending(data, order));
                }
                self.send_input(data, order)
            })
            .map_err(|error| {
                AppError::InvalidArgument(format!("native terminal input rejected: {error:?}"))
            })?;
        self.finish_input(services, is_user_input, result)
    }

    pub(crate) fn prepare_input(
        &self,
        input: NativeInput<'_>,
        capacity: usize,
    ) -> AppResult<InputPreparation> {
        if let NativeInput::Focus(focused) = input {
            if !self.snapshot(|snapshot| snapshot.phase == Phase::Running)? {
                return Err(AppError::InvalidArgument(
                    "native terminal input is retired".into(),
                ));
            }
            return Ok(InputPreparation::Prepared(PreparedInput {
                owner: self.input_identity.clone(),
                data: Vec::new(),
                focus: Some(focused),
                is_recorded: false,
            }));
        }
        let action = self.snapshot(|snapshot| {
            if snapshot.phase != Phase::Running {
                return Err(AppError::InvalidArgument(
                    "native terminal input is retired".into(),
                ));
            }
            snapshot
                .core
                .encode_input(input, capacity)
                .map_err(|error| {
                    AppError::InvalidArgument(format!("native terminal input rejected: {error:?}"))
                })
        })??;
        match action {
            InputAction::Write(data) => Ok(InputPreparation::Prepared(PreparedInput {
                owner: self.input_identity.clone(),
                data,
                focus: None,
                is_recorded: false,
            })),
            action => Ok(InputPreparation::Local(action)),
        }
    }

    pub(crate) fn admit_prepared_input(
        &self,
        mut prepared: PreparedInput,
    ) -> AppResult<PreparedInput> {
        if !Arc::ptr_eq(&prepared.owner, &self.input_identity) || prepared.is_recorded {
            return Err(AppError::InvalidArgument(
                "native terminal prepared input admission mismatch".into(),
            ));
        }
        if prepared.focus.is_none() {
            self.terminal
                .try_write(true, || Ok::<_, AppError>(()))
                .map_err(|error| {
                    AppError::InvalidArgument(format!(
                        "native terminal prepared input admission rejected: {error:?}"
                    ))
                })??;
            prepared.is_recorded = true;
        }
        Ok(prepared)
    }

    pub(crate) fn submit_prepared_input(
        &self,
        services: &AppServices,
        prepared: PreparedInput,
        capacity: usize,
        defer: bool,
    ) -> AppResult<InputResult> {
        if !Arc::ptr_eq(&prepared.owner, &self.input_identity) {
            return Err(AppError::InvalidArgument(
                "native terminal prepared input belongs to another session".into(),
            ));
        }
        if let Some(focused) = prepared.focus {
            return self.queue_input(services, NativeInput::Focus(focused), capacity, defer);
        }
        if !prepared.is_recorded {
            return Err(AppError::InvalidArgument(
                "native terminal prepared input is not admitted".into(),
            ));
        }
        let result = self
            .terminal
            .try_write(false, || {
                let order = self
                    .writer
                    .reserve_order()
                    .map_err(InputRejection::Failed)?;
                if defer {
                    return Err(InputRejection::Pending(prepared.data, order));
                }
                self.send_input(prepared.data, order)
            })
            .map_err(|error| {
                AppError::InvalidArgument(format!(
                    "native terminal prepared input rejected: {error:?}"
                ))
            })?;
        Ok(self
            .finish_input(services, true, result)?
            .with_input_recorded(true))
    }

    pub fn retry_input(
        &self,
        services: &AppServices,
        pending: PendingInput,
    ) -> AppResult<InputResult> {
        if !Arc::ptr_eq(&pending.owner, &self.input_identity) {
            return Err(AppError::InvalidArgument(
                "native terminal pending input belongs to another session".into(),
            ));
        }
        let is_recorded = pending.is_recorded;
        let result = self
            .terminal
            .try_write(pending.is_user_input && !is_recorded, || {
                self.send_input(pending.data, pending.order)
            })
            .map_err(|error| {
                AppError::InvalidArgument(format!("native terminal retry rejected: {error:?}"))
            })?;
        Ok(self
            .finish_input(services, pending.is_user_input, result)?
            .with_input_recorded(is_recorded))
    }

    pub async fn write_raw(
        self: &Arc<Self>,
        services: &AppServices,
        data: String,
    ) -> AppResult<()> {
        let order = self
            .terminal
            .try_write(true, || self.writer.reserve_order())
            .map_err(|error| {
                AppError::InvalidArgument(format!("native terminal raw input rejected: {error:?}"))
            })??;
        let session = self.clone();
        services
            .tasks
            .run_nonabortable_result("native-terminal-remote-write", async move {
                let limit = session.writer.payload_limit();
                if limit == 0 {
                    return Err(AppError::InvalidArgument(
                        "native terminal raw input budget is empty".into(),
                    ));
                }
                if data.is_empty() {
                    return session
                        .writer
                        .submit_wait_ordered(Vec::new(), &order)
                        .await?
                        .wait()
                        .await;
                }
                for bytes in data.as_bytes().chunks(limit) {
                    session
                        .writer
                        .submit_wait_ordered(bytes.to_vec(), &order)
                        .await?
                        .wait()
                        .await?;
                }
                Ok(())
            })
            .await
    }

    fn send_input(
        &self,
        data: Vec<u8>,
        order: terminal_writer::Order,
    ) -> Result<InputResult, InputRejection> {
        match self.writer.try_submit_ordered(data, &order) {
            Ok(terminal_writer::Submission::Accepted(receipt)) => Ok(InputResult::Write(receipt)),
            Ok(terminal_writer::Submission::Pending(data)) => {
                Err(InputRejection::Pending(data, order))
            }
            Err(error) => Err(InputRejection::Failed(error)),
        }
    }

    fn finish_input(
        &self,
        services: &AppServices,
        is_user_input: bool,
        result: Result<InputResult, InputRejection>,
    ) -> AppResult<InputResult> {
        match result {
            Ok(result) => {
                if matches!(&result, InputResult::Write(_)) {
                    services.agents.record_input(&self.id);
                }
                Ok(result)
            }
            Err(InputRejection::Pending(data, order)) => Ok(InputResult::Pending(PendingInput {
                owner: self.input_identity.clone(),
                data,
                is_user_input,
                is_recorded: false,
                order,
            })),
            Err(InputRejection::Failed(error)) => Err(error),
        }
    }

    pub async fn resize(
        self: &Arc<Self>,
        services: &Arc<AppServices>,
        size: Size,
    ) -> AppResult<bool> {
        let session = self.clone();
        let store = services.terminal.clone();
        services
            .tasks
            .run_blocking_result("native-terminal-resize", move || {
                let result = session.terminal.resize_with_delivery(
                    size,
                    || store.resize(&session.id, size.columns, size.rows),
                    |frame| session.frames.submit(frame, Instant::now()).is_ok(),
                );
                if result.is_err()
                    && session
                        .terminal
                        .snapshot(|state| {
                            matches!(
                                state.phase,
                                taide_native_terminal::session::Phase::Failed(_)
                            )
                        })
                        .unwrap_or(true)
                {
                    session.fail(Failure::Resize);
                }
                result
            })
            .await
    }

    fn fail(&self, failure: Failure) {
        self.failure
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get_or_insert(failure);
        let core_failure = match failure {
            Failure::Join => CoreFailure::Join,
            _ => CoreFailure::Delivery,
        };
        drop(self.terminal.fail_and_stop(core_failure));
        self.writer.close();
    }
}

struct Entry {
    session: Arc<Session>,
    worker: Option<JoinHandle<()>>,
    services: Arc<AppServices>,
}
impl Drop for Entry {
    fn drop(&mut self) {
        self.session.writer.close();
        drop(self.services.terminal.kill(self.session.id()));
    }
}

pub struct Hub {
    services: Arc<AppServices>,
    sessions: Mutex<HashMap<String, Entry>>,
    admission: Arc<Semaphore>,
    limits: Limits,
}

struct Prepared {
    session: Arc<Session>,
    sender: terminal_frames::Sender,
    receiver: terminal_frames::Receiver,
}
#[derive(Default)]
struct PendingState {
    is_cancelled: bool,
    prepared: Option<Prepared>,
}
struct Pending {
    state: Arc<Mutex<PendingState>>,
    services: Arc<AppServices>,
}
impl Drop for Pending {
    fn drop(&mut self) {
        let prepared = {
            let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            state.is_cancelled = true;
            state.prepared.take()
        };
        if let Some(prepared) = prepared {
            prepared.session.fail(Failure::Supervisor);
            drop(self.services.terminal.kill(prepared.session.id()));
        }
    }
}

struct ActorGuard {
    session: Arc<Session>,
    store: taide_terminal::store::TerminalStore,
    is_successful: bool,
}
impl Drop for ActorGuard {
    fn drop(&mut self) {
        if !self.is_successful {
            self.session.fail(Failure::Supervisor);
            drop(self.store.kill(self.session.id()));
        }
        self.session.is_dispatched.store(true, Ordering::Release);
        self.session.changed.notify_waiters();
    }
}

fn stopped() -> AppError {
    AppError::Forbidden("native terminal session is stopped or failed".into())
}
fn queue_error(_: terminal_frames::Error) -> AppError {
    AppError::InvalidArgument("native terminal output queue failed".into())
}

impl Hub {
    pub fn new(services: Arc<AppServices>, limits: Limits) -> AppResult<Self> {
        if limits.sessions == 0 || limits.sessions > Semaphore::MAX_PERMITS {
            return Err(AppError::InvalidArgument(
                "native terminal session limit is invalid".into(),
            ));
        }
        let _ = terminal_frames::channel(limits.frames).map_err(queue_error)?;
        let _ = limits.writer.validate()?;
        Ok(Self {
            services,
            sessions: Mutex::new(HashMap::new()),
            admission: Arc::new(Semaphore::new(limits.sessions)),
            limits,
        })
    }

    pub fn get(&self, id: &str) -> Option<Arc<Session>> {
        self.sessions
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(id)
            .map(|entry| entry.session.clone())
    }

    pub fn configure_cursors(&self) -> AppResult<bool> {
        let style = crate::terminal_settings::cursor(&self.services.state.settings.read());
        let sessions = self
            .sessions
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut changed = false;
        for entry in sessions.values() {
            changed |= entry.session.terminal.configure_cursor(style)?;
        }
        Ok(changed)
    }

    pub fn discard(&self, id: &str) {
        self.sessions
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(id);
    }

    pub fn discard_project(&self, project: &ProjectId) {
        let discarded = self
            .sessions
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .extract_if(|_, entry| entry.session.metadata.project_id() == project)
            .collect::<Vec<_>>();
        drop(discarded);
    }

    pub async fn close(&self, id: &str) -> AppResult<()> {
        let mut entry = self
            .sessions
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(id)
            .ok_or_else(stopped)?;
        let session = entry.session.clone();
        let worker = entry.worker.take();
        drop(entry);
        let result = session.wait_dispatch().await;
        if let Some(worker) = worker {
            worker.await.map_err(|_| stopped())?;
        }
        result
    }

    pub async fn spawn<E>(
        &self,
        opts: PtySpawnOptions,
        history: usize,
        extra_env: E,
        ports: EffectPorts,
    ) -> AppResult<String>
    where
        E: Future<Output = Vec<(String, String)>>,
    {
        self.spawn_with_initial_sink(opts, history, extra_env, ports, || {})
            .await
    }

    pub async fn spawn_with_initial_sink<E, D>(
        &self,
        opts: PtySpawnOptions,
        history: usize,
        extra_env: E,
        ports: EffectPorts,
        discard_initial_sink: D,
    ) -> AppResult<String>
    where
        E: Future<Output = Vec<(String, String)>>,
        D: FnOnce(),
    {
        self.spawn_display_with_initial_sink(
            opts,
            history,
            extra_env,
            SessionPorts::Native(ports),
            discard_initial_sink,
        )
        .await
    }

    pub async fn spawn_observed_with_initial_sink<E, D>(
        &self,
        opts: PtySpawnOptions,
        history: usize,
        extra_env: E,
        ports: ObservePorts,
        discard_initial_sink: D,
    ) -> AppResult<String>
    where
        E: Future<Output = Vec<(String, String)>>,
        D: FnOnce(),
    {
        self.spawn_display_with_initial_sink(
            opts,
            history,
            extra_env,
            SessionPorts::Renderer(ports),
            discard_initial_sink,
        )
        .await
    }

    async fn spawn_display_with_initial_sink<E, D>(
        &self,
        opts: PtySpawnOptions,
        history: usize,
        extra_env: E,
        ports: SessionPorts,
        discard_initial_sink: D,
    ) -> AppResult<String>
    where
        E: Future<Output = Vec<(String, String)>>,
        D: FnOnce(),
    {
        let admission = self.admission.clone().try_acquire_owned().map_err(|_| {
            AppError::InvalidArgument("native terminal session capacity is exhausted".into())
        })?;
        let pending = Pending {
            state: Arc::new(Mutex::new(PendingState::default())),
            services: self.services.clone(),
        };
        let slot = pending.state.clone();
        let limits = self.limits;
        let command_colors = ports.command_colors();
        let services = self.services.clone();
        let result = terminal_actions::pty_spawn(
            self.services.events.as_ref(),
            &self.services.state,
            &self.services.terminal,
            &self.services.tasks,
            opts,
            TerminalSpawnPorts {
                extra_env,
                discard_initial_sink,
                create_session_id: || format!("term-{}", ProjectId::new()),
                create_session: move |config: taide_infra::pty::PtySpawnConfig,
                                      id,
                                      metadata: Arc<TerminalSessionMetadata>,
                                      legacy_output: Arc<
                    taide_terminal::session::TerminalSessionOutput,
                >| {
                    let terminal = SharedTerminal::new(
                        Size {
                            columns: config.cols,
                            rows: config.rows,
                        },
                        history,
                        limits.core,
                    )?;
                    terminal.configure_cursor(crate::terminal_settings::cursor(
                        &services.state.settings.read(),
                    ))?;
                    terminal.configure_command_colors(command_colors)?;
                    let paused_terminal = terminal.clone();
                    let (sender, receiver) = terminal_frames::channel_with_flow(
                        limits.frames,
                        Arc::new(move |paused| paused_terminal.set_output_paused(paused)),
                    )
                    .map_err(queue_error)?;
                    let output = sender.clone();
                    let exit_metadata = metadata.clone();
                    let admission = Arc::new(admission);
                    let callback_admission = admission.clone();
                    let exit_admission = admission.clone();
                    let pty = terminal.spawn_with_output(
                        config,
                        move |bytes| legacy_output.append_and_broadcast(bytes),
                        move |frame| {
                            let _ = &callback_admission;
                            output.submit(frame, Instant::now()).is_ok()
                        },
                        move |_| {
                            let _ = &exit_admission;
                            exit_metadata.mark_exited();
                        },
                    )?;
                    let handle = pty.writer_handle();
                    let writer = Writer::start(&services.tasks, limits.writer, move |data| {
                        let mut writer = handle.lock();
                        writer.write_all(data)?;
                        writer.flush()?;
                        Ok(())
                    });
                    let (writer, writer_worker) = match writer {
                        Ok(writer) => writer,
                        Err(error) => {
                            services.terminal.retire_session(pty);
                            return Err(error);
                        }
                    };
                    let session = Arc::new(Session {
                        input_identity: Arc::new(()),
                        id,
                        terminal,
                        metadata,
                        writer,
                        frames: sender.clone(),
                        writer_worker: tokio::sync::Mutex::new(Some(writer_worker)),
                        completion: pty.completion_handle(),
                        failure: Mutex::new(None),
                        is_dispatched: AtomicBool::new(false),
                        changed: Notify::new(),
                        _admission: admission,
                    });
                    let mut slot = slot.lock().unwrap_or_else(|error| error.into_inner());
                    if slot.is_cancelled {
                        drop(slot);
                        session.fail(Failure::Supervisor);
                        services.terminal.retire_session(pty);
                        return Err(stopped());
                    }
                    slot.prepared = Some(Prepared {
                        session,
                        sender,
                        receiver,
                    });
                    Ok(pty)
                },
            },
        )
        .await;
        let id = result?;
        let prepared = pending
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .prepared
            .take()
            .ok_or_else(stopped)?;
        let session = prepared.session.clone();
        let guard = ActorGuard {
            session: session.clone(),
            store: self.services.terminal.clone(),
            is_successful: false,
        };
        let services = self.services.clone();
        let tasks = services.tasks.clone();
        let worker = tasks
            .spawn_transient_handle("native-terminal-session", async move {
                run_actor(prepared, services, ports, guard).await;
            })
            .ok_or_else(stopped)?;
        self.sessions
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(
                id.clone(),
                Entry {
                    session,
                    worker: Some(worker),
                    services: self.services.clone(),
                },
            );
        Ok(id)
    }
}

async fn consume(
    delivery: terminal_frames::Delivery,
    dispatcher: &mut Dispatcher,
    services: &AppServices,
    session: &Session,
    ports: &SessionPorts,
) -> AppResult<()> {
    let can_reply = session
        .terminal
        .snapshot(|state| state.exit_code.is_none())?;
    let result = dispatcher
        .consume_display(&delivery, services, &session.writer, ports, can_reply)
        .await;
    drop(delivery);
    result
}

async fn run_actor(
    mut prepared: Prepared,
    services: Arc<AppServices>,
    ports: SessionPorts,
    mut guard: ActorGuard,
) {
    let session = prepared.session.clone();
    let mut dispatcher = Dispatcher::new(session.id.clone(), session.metadata.clone());
    let terminal = session.terminal.clone();
    let completion = session.completion.clone();
    let finished = terminal.finish(&completion);
    tokio::pin!(finished);
    let final_frame = loop {
        let deadline = terminal
            .snapshot(|state| state.core.sync_deadline())
            .ok()
            .flatten();
        let sync_wait = async {
            match deadline {
                Some(deadline) => {
                    tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)).await
                }
                None => std::future::pending().await,
            }
        };
        tokio::select! {
            result = &mut finished => break result,
            () = sync_wait => {
                let now = Instant::now();
                if terminal.flush_sync_if_due(now, |frame| prepared.sender.submit(frame, now).is_ok()).is_err() {
                    session.fail(Failure::Transport);
                    break Err(stopped());
                }
            },
            delivery = prepared.receiver.recv() => match delivery {
                Ok(Some(delivery)) => if consume(delivery, &mut dispatcher, &services, &session, &ports).await.is_err() { session.fail(Failure::Effects); break Err(stopped()); },
                Ok(None) | Err(_) => { session.fail(Failure::Transport); break Err(stopped()); }
            },
        }
    };
    match final_frame {
        Ok(frame) => {
            let result = async {
                while let Some(delivery) = prepared.receiver.try_recv().map_err(queue_error)? {
                    consume(delivery, &mut dispatcher, &services, &session, &ports).await?;
                }
                prepared
                    .sender
                    .submit(frame, Instant::now())
                    .map_err(queue_error)?;
                let delivery = prepared
                    .receiver
                    .recv()
                    .await
                    .map_err(queue_error)?
                    .ok_or_else(stopped)?;
                consume(delivery, &mut dispatcher, &services, &session, &ports).await
            }
            .await;
            if result.is_err() {
                session.fail(Failure::FinalFrame);
            }
        }
        Err(_) => {
            session.fail(Failure::Join);
        }
    }
    if !session.completion.is_finished() && session.completion.wait_for_completion().await.is_err()
    {
        session.fail(Failure::Join);
    }
    if let Ok(Some(code)) = session.terminal.snapshot(|state| state.exit_code) {
        session.metadata.mark_exited();
        services.events.publish(AppEvent::TerminalExited {
            session_id: session.id.clone(),
            code,
        });
    }
    session.writer.close();
    if let Some(worker) = session.writer_worker.lock().await.take()
        && worker.await.is_err()
    {
        session.fail(Failure::Effects);
    }
    guard.is_successful = true;
}

#[cfg(test)]
mod tests {
    use std::future::{Future, poll_fn};
    use std::task::Poll;
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn control_batch는_소유권과_상한을_검사하고_거절시_기존_보고를_보존한다() {
        const LIMIT: usize = 9;
        const WRITER_BYTES: usize = 4096;
        const TIMEOUT: Duration = Duration::from_secs(3);
        let tasks = taide_runtime::TaskSupervisor::new(tokio::runtime::Handle::current());
        let (writer, worker) = terminal_writer::Writer::start(
            &tasks,
            terminal_writer::Limits {
                bytes: WRITER_BYTES,
                count: 1,
            },
            |_| Ok(()),
        )
        .unwrap();
        let owner = Arc::new(());
        let input = |data: &[u8], is_user_input| PendingInput {
            owner: owner.clone(),
            data: data.to_vec(),
            is_user_input,
            is_recorded: false,
            order: writer.reserve_order().unwrap(),
        };
        let mut batch = input(b"\x1b[O", false);
        batch
            .append_control(input(b"\x1b[I", false), LIMIT)
            .unwrap();
        batch
            .append_control(input(b"\x1b[O", false), LIMIT)
            .unwrap();
        let original = batch.data.clone();
        assert_eq!(original, b"\x1b[O\x1b[I\x1b[O");
        assert_eq!(
            batch.retained_bytes(),
            Some(LIMIT + size_of::<PendingInput>())
        );
        assert!(
            batch
                .append_control(input(b"\x1b[I", false), LIMIT)
                .is_err()
        );
        assert!(batch.append_control(input(b"x", true), LIMIT).is_err());
        assert!(
            batch
                .append_control(
                    PendingInput {
                        owner: Arc::new(()),
                        data: b"\x1b[I".to_vec(),
                        is_user_input: false,
                        is_recorded: false,
                        order: writer.reserve_order().unwrap(),
                    },
                    LIMIT
                )
                .is_err()
        );
        assert_eq!(batch.data, original);
        let mut user = input(b"x", true);
        assert!(user.append_control(input(b"\x1b[I", false), LIMIT).is_err());
        assert_eq!(user.data, b"x");
        let mut before = input(b"\x1b[O", false);
        let query = writer.reserve_order().unwrap();
        let after = input(b"\x1b[I", false);
        assert!(!before.can_append_control(&after).unwrap());
        drop(query);
        assert!(before.can_append_control(&after).unwrap());
        before.append_control(after, LIMIT).unwrap();
        assert_eq!(before.data, b"\x1b[O\x1b[I");
        let mut waiting = Box::pin(writer.submit_wait(b"waiting".to_vec()));
        assert!(poll_fn(|context| Poll::Ready(waiting.as_mut().poll(context).is_pending())).await);
        writer.close();
        assert!(
            tokio::time::timeout(TIMEOUT, waiting)
                .await
                .unwrap()
                .is_err()
        );
        worker.await.unwrap();
        tasks.shutdown().await;
        assert_eq!(tasks.tracked_count(), 0);
    }
}

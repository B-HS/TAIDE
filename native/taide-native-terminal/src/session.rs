use std::sync::{Arc, Condvar, Mutex, MutexGuard};

use taide_infra::pty::{self, PtyCompletionHandle, PtySession, PtySpawnConfig, PtyStopHandle};
use taide_model::error::{AppError, AppResult};

use crate::{
    Limits, Outcome, Size, TerminalCore,
    input::{InputAction, InputError, NativeInput},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Parser,
    Delivery,
    Sequence,
    Join,
    Spawn,
    Resize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Running,
    Draining(Option<i32>),
    Exited(Option<i32>),
    Failed(Failure),
}

#[derive(taide_native_retained::RetainedBytes)]
pub struct Frame {
    pub revision: u64,
    pub outcome: Outcome,
}

pub struct Snapshot<'core> {
    pub revision: u64,
    pub phase: Phase,
    pub exit_code: Option<Option<i32>>,
    pub core: &'core TerminalCore,
}

struct State {
    core: TerminalCore,
    cursor_defaults: Option<crate::CursorStyle>,
    phase: Phase,
    revision: u64,
    is_bound: bool,
    completion: Option<PtyStopHandle>,
    exit_code: Option<Option<i32>>,
    output: OutputGate,
}

#[derive(Clone)]
pub struct SharedTerminal(Arc<Mutex<State>>, Arc<Mutex<()>>, OutputGate);

#[derive(Default)]
struct OutputHold {
    is_paused: bool,
    is_retired: bool,
}

#[derive(Default)]
struct OutputFlow {
    hold: Mutex<OutputHold>,
    resumed: Condvar,
}

#[derive(Clone, Default)]
struct OutputGate(Arc<OutputFlow>);

impl OutputGate {
    fn set_paused(&self, paused: bool) {
        let mut hold = self
            .0
            .hold
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        hold.is_paused = paused && !hold.is_retired;
        if !hold.is_paused {
            self.0.resumed.notify_all();
        }
    }

    fn retire(&self) {
        let mut hold = self
            .0
            .hold
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        hold.is_retired = true;
        hold.is_paused = false;
        self.0.resumed.notify_all();
    }

    fn wait_while_paused(&self) {
        let mut hold = self
            .0
            .hold
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        while hold.is_paused {
            hold = self
                .0
                .resumed
                .wait(hold)
                .unwrap_or_else(|error| error.into_inner());
        }
    }
}

#[derive(Default)]
struct StopState {
    handle: Option<PtyStopHandle>,
    is_requested: bool,
}

#[derive(Clone, Default)]
struct StopGate(Arc<Mutex<StopState>>);

struct DeliveryGuard {
    terminal: SharedTerminal,
    stop: StopGate,
    is_armed: bool,
}

impl Drop for DeliveryGuard {
    fn drop(&mut self) {
        if !self.is_armed {
            return;
        }
        if let Ok(mut state) = lock(&self.terminal.0)
            && state.can_feed()
        {
            state.fail(Failure::Delivery);
        }
        self.stop.request();
    }
}

fn lock<T>(mutex: &Mutex<T>) -> AppResult<MutexGuard<'_, T>> {
    mutex
        .lock()
        .map_err(|_| AppError::Internal("native terminal owner is poisoned".into()))
}

fn invalid(message: &str) -> AppError {
    AppError::InvalidArgument(message.into())
}

impl StopGate {
    fn request(&self) {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        state.is_requested = true;
        let handle = state.handle.clone();
        drop(state);
        if let Some(handle) = handle {
            drop(handle.kill());
        }
    }

    fn bind(&self, handle: PtyStopHandle) {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        state.handle = Some(handle.clone());
        let should_stop = state.is_requested;
        drop(state);
        if should_stop {
            drop(handle.kill());
        }
    }
}

impl State {
    fn accept_input<R, E>(
        &mut self,
        should_record: bool,
        accept: impl FnOnce() -> Result<R, E>,
    ) -> Result<Result<R, E>, InputError> {
        if !matches!(self.phase, Phase::Running) {
            return Err(InputError::Retired);
        }
        if should_record
            && !self
                .core
                .selection_stamp()
                .is_ok_and(|stamp| stamp.input_epoch < u64::MAX)
        {
            self.fail(Failure::Sequence);
            return Err(InputError::Retired);
        }
        let result = accept();
        if result.is_ok() && should_record && self.core.record_user_input().is_err() {
            self.fail(Failure::Sequence);
            return Err(InputError::Retired);
        }
        Ok(result)
    }

    fn fail(&mut self, failure: Failure) {
        if matches!(self.phase, Phase::Failed(_)) {
            return;
        }
        self.core.retire();
        self.phase = Phase::Failed(failure);
        self.output.retire();
    }

    fn next_revision(&mut self) -> AppResult<u64> {
        match self.revision.checked_add(1) {
            Some(revision) => Ok(revision),
            None => {
                self.fail(Failure::Sequence);
                Err(invalid("native terminal revision is exhausted"))
            }
        }
    }

    fn can_feed(&self) -> bool {
        matches!(self.phase, Phase::Running | Phase::Draining(_))
    }
}

impl SharedTerminal {
    pub fn new(size: Size, history: usize, limits: Limits) -> AppResult<Self> {
        let output = OutputGate::default();
        Ok(Self(
            Arc::new(Mutex::new(State {
                core: TerminalCore::new(size, history, limits)?,
                cursor_defaults: None,
                phase: Phase::Running,
                revision: 0,
                is_bound: false,
                completion: None,
                exit_code: None,
                output: output.clone(),
            })),
            Arc::new(Mutex::new(())),
            output,
        ))
    }

    pub fn set_output_paused(&self, paused: bool) {
        self.2.set_paused(paused);
    }

    pub fn snapshot<R>(&self, read: impl FnOnce(Snapshot<'_>) -> R) -> AppResult<R> {
        let state = lock(&self.0)?;
        Ok(read(Snapshot {
            revision: state.revision,
            phase: state.phase,
            exit_code: state.exit_code,
            core: &state.core,
        }))
    }

    pub fn configure_cursor(&self, style: crate::CursorStyle) -> AppResult<bool> {
        let _publishing = lock(&self.1)?;
        let mut state = lock(&self.0)?;
        if matches!(state.phase, Phase::Failed(_)) || state.cursor_defaults == Some(style) {
            return Ok(false);
        }
        state.core.set_default_cursor_style(style)?;
        state.cursor_defaults = Some(style);
        Ok(true)
    }

    pub fn configure_command_colors(&self, colors: crate::CommandColors) -> AppResult<()> {
        let _publishing = lock(&self.1)?;
        let mut state = lock(&self.0)?;
        if matches!(state.phase, Phase::Failed(_)) {
            return Ok(());
        }
        state.core.configure_command_colors(colors)
    }

    pub fn clear_current_row(&self) -> AppResult<bool> {
        let mut guard = self.delivery_guard()?;
        let _publishing = lock(&self.1)?;
        let mut state = lock(&self.0)?;
        if !matches!(state.phase, Phase::Running) {
            return Err(invalid("native terminal is not running for clear"));
        }
        guard.is_armed = true;
        let changed = match state.core.clear_current_row() {
            Ok(changed) => changed,
            Err(error) => {
                state.fail(Failure::Parser);
                return Err(error);
            }
        };
        guard.is_armed = false;
        Ok(changed)
    }

    pub fn advance(&self, bytes: &[u8]) -> AppResult<Frame> {
        let mut state = lock(&self.0)?;
        if !state.can_feed() {
            return Err(invalid("native terminal no longer accepts output"));
        }
        let revision = state.next_revision()?;
        let outcome = match state.core.advance_outcome(bytes) {
            Ok(outcome) => outcome,
            Err(error) => {
                state.fail(Failure::Parser);
                return Err(error);
            }
        };
        state.revision = revision;
        Ok(Frame { revision, outcome })
    }

    fn delivery_guard(&self) -> AppResult<DeliveryGuard> {
        let stop = StopGate::default();
        if let Some(handle) = lock(&self.0)?.completion.clone() {
            stop.bind(handle);
        }
        Ok(DeliveryGuard {
            terminal: self.clone(),
            stop,
            is_armed: false,
        })
    }

    fn advance_with_delivery(
        &self,
        bytes: &[u8],
        deliver: &impl Fn(Frame) -> bool,
    ) -> AppResult<bool> {
        let _publishing = lock(&self.1)?;
        Ok(deliver(self.advance(bytes)?))
    }

    fn feed_with_delivery(
        &self,
        bytes: &[u8],
        deliver: &impl Fn(Frame) -> bool,
    ) -> AppResult<bool> {
        let feed_limit = lock(&self.0)?.core.feed_limit();
        for chunk in bytes.chunks(feed_limit) {
            self.2.wait_while_paused();
            if !self.advance_with_delivery(chunk, deliver)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn flush_sync_if_due(
        &self,
        now: std::time::Instant,
        deliver: impl FnOnce(Frame) -> bool,
    ) -> AppResult<bool> {
        let mut guard = self.delivery_guard()?;
        let _publishing = lock(&self.1)?;
        let frame = {
            let mut state = lock(&self.0)?;
            if !state.can_feed()
                || !state
                    .core
                    .sync_deadline()
                    .is_some_and(|deadline| deadline <= now)
            {
                return Ok(false);
            }
            guard.is_armed = true;
            let revision = state.next_revision()?;
            let outcome = match state.core.flush_sync_outcome() {
                Ok(outcome) => outcome,
                Err(error) => {
                    state.fail(Failure::Parser);
                    return Err(error);
                }
            };
            state.revision = revision;
            Frame { revision, outcome }
        };
        if !deliver(frame) {
            return Err(invalid("native terminal synchronized delivery failed"));
        }
        guard.is_armed = false;
        Ok(true)
    }

    pub fn resize_with_delivery(
        &self,
        size: Size,
        resize_pty: impl FnOnce() -> AppResult<()>,
        deliver: impl FnOnce(Frame) -> bool,
    ) -> AppResult<bool> {
        use alacritty_terminal::grid::Dimensions;
        let mut guard = self.delivery_guard()?;
        let _publishing = lock(&self.1)?;
        let frame = {
            let mut state = lock(&self.0)?;
            if !matches!(state.phase, Phase::Running) {
                return Err(invalid("native terminal is not running for resize"));
            }
            state.core.validate_resize(size)?;
            let grid = state.core.grid()?;
            if grid.columns() == usize::from(size.columns)
                && grid.screen_lines() == usize::from(size.rows)
            {
                return Ok(false);
            }
            guard.is_armed = true;
            let revision = state.next_revision()?;
            let outcome = match state.core.resize_outcome(size) {
                Ok(outcome) => outcome,
                Err(error) => {
                    state.fail(Failure::Parser);
                    return Err(error);
                }
            };
            if let Err(error) = resize_pty() {
                state.fail(Failure::Resize);
                return Err(error);
            }
            state.revision = revision;
            Frame { revision, outcome }
        };
        if !deliver(frame) {
            return Err(invalid("native terminal resize delivery failed"));
        }
        guard.is_armed = false;
        Ok(true)
    }

    pub fn encode_input(
        &self,
        input: NativeInput<'_>,
        capacity: usize,
    ) -> Result<InputAction, InputError> {
        self.try_input(input, capacity, Ok::<_, InputError>)?
    }

    pub fn try_input<R, E>(
        &self,
        input: NativeInput<'_>,
        capacity: usize,
        accept: impl FnOnce(InputAction) -> Result<R, E>,
    ) -> Result<Result<R, E>, InputError> {
        let mut state = self.0.lock().map_err(|_| InputError::Retired)?;
        if !matches!(state.phase, Phase::Running) {
            return Err(InputError::Retired);
        }
        if let NativeInput::Focus(focused) = &input {
            state.core.observe_focus(*focused)?;
        }
        let is_user_input = !matches!(&input, NativeInput::Focus(_));
        let action = state.core.encode_input(input, capacity)?;
        let should_record = is_user_input && matches!(&action, InputAction::Write(_));
        state.accept_input(should_record, || accept(action))
    }

    pub fn try_write<R, E>(
        &self,
        is_user_input: bool,
        write: impl FnOnce() -> Result<R, E>,
    ) -> Result<Result<R, E>, InputError> {
        let mut state = self.0.lock().map_err(|_| InputError::Retired)?;
        state.accept_input(is_user_input, write)
    }

    pub fn spawn(
        &self,
        config: PtySpawnConfig,
        deliver: impl Fn(Frame) -> bool + Send + 'static,
    ) -> AppResult<PtySession> {
        self.spawn_with_output(config, |_| {}, deliver, |_| {})
    }

    pub fn spawn_with_output(
        &self,
        config: PtySpawnConfig,
        on_chunk: impl Fn(&[u8]) + Send + 'static,
        deliver: impl Fn(Frame) -> bool + Send + 'static,
        on_exit: impl FnOnce(Option<i32>) + Send + 'static,
    ) -> AppResult<PtySession> {
        {
            use alacritty_terminal::grid::Dimensions;
            let mut state = lock(&self.0)?;
            if state.is_bound || !matches!(state.phase, Phase::Running) {
                return Err(invalid("native terminal already has a PTY owner"));
            }
            let grid = state.core.grid()?;
            if grid.columns() != usize::from(config.cols)
                || grid.screen_lines() != usize::from(config.rows)
            {
                return Err(invalid("native terminal and PTY dimensions differ"));
            }
            state.is_bound = true;
        }
        let output = self.clone();
        let exit = self.clone();
        let stop = StopGate::default();
        let output_stop = stop.clone();
        let pty = pty::spawn(
            config,
            move |bytes| {
                let mut guard = DeliveryGuard {
                    terminal: output.clone(),
                    stop: output_stop.clone(),
                    is_armed: true,
                };
                on_chunk(bytes);
                if output.feed_with_delivery(bytes, &deliver).unwrap_or(false) {
                    guard.is_armed = false;
                }
            },
            move |code| {
                if let Ok(mut state) = lock(&exit.0) {
                    state.exit_code = Some(code);
                    if matches!(state.phase, Phase::Running) {
                        state.phase = Phase::Draining(code);
                    }
                }
                on_exit(code);
            },
        );
        let pty = match pty {
            Ok(pty) => pty,
            Err(error) => {
                lock(&self.0)?.fail(Failure::Spawn);
                return Err(error);
            }
        };
        let handle = pty.completion_handle().stop_handle();
        let should_stop = {
            let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
            state.completion = Some(handle.clone());
            if self.0.is_poisoned() {
                state.fail(Failure::Join);
            }
            matches!(state.phase, Phase::Failed(_))
        };
        stop.bind(handle.clone());
        if should_stop {
            drop(handle.kill());
        }
        Ok(pty)
    }

    pub fn fail_and_stop(&self, failure: Failure) -> AppResult<()> {
        let handle = {
            let mut state = lock(&self.0)?;
            state.fail(failure);
            state.completion.clone()
        };
        match handle {
            Some(handle) => handle.kill(),
            None => Ok(()),
        }
    }

    pub async fn finish(&self, completion: &PtyCompletionHandle) -> AppResult<Frame> {
        if !lock(&self.0)?
            .completion
            .as_ref()
            .is_some_and(|owner| owner.is_same_worker(completion))
        {
            return Err(invalid(
                "native terminal completion belongs to a different PTY",
            ));
        }
        if let Err(error) = completion.wait_for_completion().await {
            lock(&self.0)?.fail(Failure::Join);
            return Err(error);
        }
        let _publishing = lock(&self.1)?;
        let mut state = lock(&self.0)?;
        let Phase::Draining(code) = state.phase else {
            return Err(invalid("native terminal cannot finalize this phase"));
        };
        let revision = state.next_revision()?;
        let outcome = match state.core.flush_sync_outcome() {
            Ok(outcome) => outcome,
            Err(error) => {
                state.fail(Failure::Parser);
                return Err(error);
            }
        };
        state.revision = revision;
        state.phase = Phase::Exited(code);
        Ok(Frame { revision, outcome })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const COLUMNS: u16 = 80;
    const ROWS: u16 = 24;
    const HISTORY: usize = 128;
    const TIMEOUT: Duration = Duration::from_secs(3);

    struct Release(Option<std::sync::mpsc::SyncSender<()>>);
    impl Drop for Release {
        fn drop(&mut self) {
            if let Some(sender) = self.0.take() {
                let _ = sender.try_send(());
            }
        }
    }

    #[test]
    fn 출력_publication중_resize는_같은_revision_순서로_제출한다() {
        let terminal = SharedTerminal::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            HISTORY,
            Limits::default(),
        )
        .unwrap();
        let revisions = Arc::new(Mutex::new(Vec::new()));
        let (entered, ready) = std::sync::mpsc::sync_channel(1);
        let (release, released) = std::sync::mpsc::sync_channel(1);
        let release = Release(Some(release));
        let source = terminal.clone();
        let captured = revisions.clone();
        let output = std::thread::spawn(move || {
            source
                .advance_with_delivery(b"first", &|frame| {
                    captured.lock().unwrap().push(frame.revision);
                    entered.send(()).unwrap();
                    released.recv_timeout(TIMEOUT).unwrap();
                    true
                })
                .unwrap()
        });
        ready.recv_timeout(TIMEOUT).unwrap();
        assert!(terminal.1.try_lock().is_err());
        assert!(terminal.0.try_lock().is_ok());
        assert_eq!(terminal.snapshot(|state| state.revision).unwrap(), 1);
        let resized = terminal.clone();
        let captured = revisions.clone();
        let (started, starting) = std::sync::mpsc::sync_channel(1);
        let resize = std::thread::spawn(move || {
            started.send(()).unwrap();
            resized
                .resize_with_delivery(
                    Size {
                        columns: COLUMNS + 1,
                        rows: ROWS + 1,
                    },
                    || Ok(()),
                    |frame| {
                        captured.lock().unwrap().push(frame.revision);
                        true
                    },
                )
                .unwrap()
        });
        starting.recv_timeout(TIMEOUT).unwrap();
        assert_eq!(terminal.snapshot(|state| state.revision).unwrap(), 1);
        drop(release);
        assert!(output.join().unwrap());
        assert!(resize.join().unwrap());
        assert_eq!(*revisions.lock().unwrap(), vec![1, 2]);
    }

    #[test]
    fn feed_한도를_넘는_pty_batch는_한도_단위로_나눠_순서대로_제출한다() {
        let limits = Limits::default();
        let terminal = SharedTerminal::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            HISTORY,
            limits,
        )
        .unwrap();
        let delivered = Mutex::new(Vec::new());
        let batch = vec![b'x'; limits.feed_bytes + 1];
        assert!(
            terminal
                .feed_with_delivery(&batch, &|frame| {
                    delivered
                        .lock()
                        .unwrap()
                        .push((frame.revision, frame.outcome.text.len()));
                    true
                })
                .unwrap()
        );
        assert_eq!(
            *delivered.lock().unwrap(),
            vec![(1, limits.feed_bytes), (2, 1)]
        );
        assert_eq!(
            terminal.snapshot(|state| state.phase).unwrap(),
            Phase::Running
        );
    }
}

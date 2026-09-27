use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;

use parking_lot::Mutex;
use taide_infra::pty::{PtyCompletionHandle, PtySession};
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::terminal::{PtyAttachResult, TerminalSession};
use tokio::sync::Notify;

use crate::metadata::TerminalSessionMetadata;
use crate::session::TerminalSessionOutput;

/// Couples one PTY resource with its metadata and output stream.
pub struct TerminalSessionEntry {
    pty: PtySession,
    metadata: Arc<TerminalSessionMetadata>,
    output: Arc<TerminalSessionOutput>,
}

impl TerminalSessionEntry {
    pub fn new(pty: PtySession, metadata: Arc<TerminalSessionMetadata>, output: Arc<TerminalSessionOutput>) -> Self {
        Self { pty, metadata, output }
    }
}

/// Serializes terminal session lookup, resource operations, and project-scoped cleanup.
#[derive(Clone, Default)]
pub struct TerminalStore(Arc<TerminalStoreInner>);

#[derive(Default)]
struct TerminalStoreInner {
    state: Mutex<TerminalState>,
    idle: Notify,
}

#[derive(Default)]
struct TerminalState {
    sessions: HashMap<String, TerminalSessionEntry>,
    is_stopped: bool,
    active_spawns: usize,
    owned: Vec<PtyCompletionHandle>,
    drains: Vec<Arc<PtyDrainTask>>,
}

struct PtyDrainTask(tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>);

impl PtyDrainTask {
    fn is_finished(&self) -> bool {
        self.0.try_lock().is_ok_and(|task| task.as_ref().is_none_or(|task| task.is_finished()))
    }

    async fn finish(&self) {
        let mut task = self.0.lock().await;
        if let Some(task) = task.as_mut() {
            task.await.ok();
        }
        task.take();
    }
}

impl Drop for PtyDrainTask {
    fn drop(&mut self) {
        if let Some(task) = self.0.get_mut().as_ref() {
            task.abort();
        }
    }
}

impl TerminalState {
    fn prune_finished(&mut self) {
        self.owned.retain(|worker| !worker.is_finished());
        self.drains.retain(|drain| !drain.is_finished());
    }

    fn register_worker(&mut self, completion: PtyCompletionHandle) {
        if !self.owned.iter().any(|worker| worker.is_same_worker(&completion)) {
            self.owned.push(completion);
        }
    }
}

/// Keeps an admitted blocking spawn counted until its result has been registered or failed.
pub struct TerminalSpawnLease(TerminalStore);

impl TerminalSpawnLease {
    /// Runs one admitted factory without holding the store mutex across blocking OS work.
    /// A shutdown racing an already-started factory retains and terminates its produced worker set.
    pub fn spawn(self, create: impl FnOnce() -> AppResult<PtySession>) -> AppResult<PtySession> {
        if self.0 .0.state.lock().is_stopped {
            return Err(terminal_shutdown_error());
        }
        let session = create()?;
        let is_stopped = {
            let mut state = self.0 .0.state.lock();
            state.register_worker(session.completion_handle());
            state.is_stopped
        };
        if is_stopped {
            self.0.retire_session(session);
            return Err(terminal_shutdown_error());
        }
        Ok(session)
    }
}

impl Drop for TerminalSpawnLease {
    fn drop(&mut self) {
        let mut state = self.0 .0.state.lock();
        state.active_spawns -= 1;
        if state.active_spawns == 0 {
            self.0 .0.idle.notify_waiters();
        }
    }
}

impl TerminalStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an admitted session, keeping superseded workers owned until actual completion.
    /// Producers must obtain a spawn lease before creating a child so shutdown can await admission.
    pub fn insert(&self, session_id: String, entry: TerminalSessionEntry) -> AppResult<()> {
        let previous = {
            let mut state = self.0.state.lock();
            state.register_worker(entry.pty.completion_handle());
            if state.is_stopped {
                drop(state);
                self.retire_session(entry.pty);
                return Err(terminal_shutdown_error());
            }
            state.sessions.insert(session_id, entry)
        };
        if let Some(previous) = previous {
            self.retire_session(previous.pty);
        }
        Ok(())
    }

    /// Admits one spawn independently of its request waiter. Closing admission does not abort started OS work.
    pub fn begin_spawn(&self) -> AppResult<TerminalSpawnLease> {
        let mut state = self.0.state.lock();
        state.prune_finished();
        if state.is_stopped {
            return Err(terminal_shutdown_error());
        }
        state.active_spawns = state.active_spawns.checked_add(1).expect("terminal spawn count exhausted");
        Ok(TerminalSpawnLease(self.clone()))
    }

    /// Retains worker ownership before dropping an unreturned, removed, or superseded PTY resource.
    /// When a runtime is available, one owned cleanup task joins it; shutdown awaits it regardless.
    pub fn retire_session(&self, session: PtySession) {
        let completion = session.completion_handle();
        {
            let mut state = self.0.state.lock();
            state.register_worker(completion.clone());
        }
        drop(session);
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let mut state = self.0.state.lock();
        if state.is_stopped {
            return;
        }
        state.prune_finished();
        let task = runtime.spawn(async move {
            completion.wait_for_completion().await.ok();
        });
        state.drains.push(Arc::new(PtyDrainTask(tokio::sync::Mutex::new(Some(task)))));
    }

    pub fn cwd(&self, session_id: &str) -> Option<String> {
        self.0.state.lock().sessions.get(session_id).map(|entry| entry.metadata.cwd())
    }

    pub fn update_cwd(&self, session_id: &str, cwd: String) -> bool {
        self.0.state.lock().sessions.get(session_id).is_some_and(|entry| entry.metadata.update_cwd(cwd))
    }

    pub fn writer_handle(&self, session_id: &str) -> AppResult<Arc<Mutex<Box<dyn Write + Send>>>> {
        let state = self.0.state.lock();
        Ok(find_entry(&state.sessions, session_id)?.pty.writer_handle())
    }

    pub fn resize(&self, session_id: &str, cols: u16, rows: u16) -> AppResult<()> {
        let state = self.0.state.lock();
        find_entry(&state.sessions, session_id)?.pty.resize(cols, rows)
    }

    pub fn kill(&self, session_id: &str) -> AppResult<()> {
        let removed = self.0.state.lock().sessions.remove(session_id);
        match removed {
            Some(entry) => {
                let result = entry.pty.kill();
                self.retire_session(entry.pty);
                result
            }
            None => Err(AppError::NotFound(format!("terminal session not found: {session_id}"))),
        }
    }

    pub fn set_paused(&self, session_id: &str, paused: bool) -> AppResult<()> {
        let state = self.0.state.lock();
        find_entry(&state.sessions, session_id)?.pty.set_paused(paused);
        Ok(())
    }

    pub fn attach(&self, session_id: &str, sink: impl Fn(&[u8]) -> bool + Send + Sync + 'static) -> AppResult<PtyAttachResult> {
        let state = self.0.state.lock();
        Ok(find_entry(&state.sessions, session_id)?.output.attach(sink))
    }

    pub fn detach(&self, session_id: &str, subscription_id: u32) -> AppResult<()> {
        let state = self.0.state.lock();
        if let Some(entry) = state.sessions.get(session_id) {
            entry.output.detach(subscription_id);
        }
        Ok(())
    }

    pub fn sessions_for_project(&self, project_id: &ProjectId) -> Vec<TerminalSession> {
        self.0
            .state
            .lock()
            .sessions
            .iter()
            .filter(|(_, entry)| entry.metadata.project_id() == project_id)
            .map(|(session_id, entry)| entry.metadata.snapshot(session_id))
            .collect()
    }

    pub fn foreground_pids(&self, project_id: &ProjectId) -> Vec<(String, u32)> {
        self.0
            .state
            .lock()
            .sessions
            .iter()
            .filter(|(_, entry)| entry.metadata.project_id() == project_id)
            .filter_map(|(session_id, entry)| entry.pty.foreground_pid().map(|pid| (session_id.clone(), pid)))
            .collect()
    }

    pub fn kill_all(&self) {
        let workers = self.0.state.lock().owned.clone();
        for worker in workers {
            worker.kill().ok();
        }
    }

    /// Closes spawn and session admission before requesting termination of all owned worker sets.
    pub fn shutdown(&self) {
        self.0.state.lock().is_stopped = true;
        self.kill_all();
    }

    /// Waits for admitted factories, every worker set, and owned cleanup tasks after shutdown.
    /// Dropping this future retains each completion handle and task for a later wait.
    pub async fn wait_for_idle(&self) -> AppResult<()> {
        loop {
            let changed = self.0.idle.notified();
            if self.0.state.lock().active_spawns == 0 {
                break;
            }
            changed.await;
        }
        let (workers, drains) = {
            let state = self.0.state.lock();
            (state.owned.clone(), state.drains.clone())
        };
        let mut failure = None;
        for worker in workers {
            if let Err(error) = worker.wait_for_completion().await {
                failure.get_or_insert(error);
            }
        }
        for drain in drains {
            drain.finish().await;
        }
        self.0.state.lock().prune_finished();
        failure.map_or(Ok(()), Err)
    }

    pub fn kill_project(&self, project_id: &ProjectId) {
        let removed = {
            let mut state = self.0.state.lock();
            let session_ids: Vec<_> = state
                .sessions
                .iter()
                .filter(|(_, entry)| entry.metadata.project_id() == project_id)
                .map(|(session_id, _)| session_id.clone())
                .collect();
            session_ids.into_iter().filter_map(|id| state.sessions.remove(&id)).collect::<Vec<_>>()
        };
        for entry in removed {
            self.retire_session(entry.pty);
        }
    }

    pub fn kill_session(&self, session_id: &str) {
        let removed = self.0.state.lock().sessions.remove(session_id);
        if let Some(entry) = removed {
            self.retire_session(entry.pty);
        }
    }
}

fn terminal_shutdown_error() -> AppError {
    AppError::Forbidden("terminal runtime is shutting down".to_string())
}

fn find_entry<'a>(sessions: &'a HashMap<String, TerminalSessionEntry>, session_id: &str) -> AppResult<&'a TerminalSessionEntry> {
    sessions.get(session_id).ok_or_else(|| AppError::NotFound(format!("terminal session not found: {session_id}")))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use super::*;

    const FIXTURE_TIMEOUT_MS: u64 = 2_000;
    const PENDING_PROBE_MS: u64 = 60;
    #[cfg(unix)]
    const TEST_COLS: u16 = 80;
    #[cfg(unix)]
    const TEST_ROWS: u16 = 24;
    #[cfg(unix)]
    const TEST_SCROLLBACK_BYTES: usize = 64 * 1024;

    #[test]
    fn shutdown_뒤에는_spawn_lease를_발급하지_않는다() {
        let store = TerminalStore::new();
        store.shutdown();
        assert!(store.begin_spawn().is_err());
    }

    #[test]
    fn shutdown_전에_발급한_lease도_미시작_factory를_실행하지_않는다() {
        let store = TerminalStore::new();
        let lease = store.begin_spawn().unwrap();
        let called = AtomicBool::new(false);
        store.shutdown();
        let result = lease.spawn(|| {
            called.store(true, Ordering::SeqCst);
            Err(AppError::Internal("synthetic factory must not run".to_string()))
        });
        assert!(result.is_err());
        assert!(!called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn idle은_마지막_spawn_lease와_대기_취소_뒤_복수_재대기자를_보존한다() {
        let store = TerminalStore::new();
        let lease = store.begin_spawn().unwrap();
        store.shutdown();
        let mut waiting = Box::pin(store.wait_for_idle());
        assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), &mut waiting).await.is_err());
        drop(waiting);
        let mut first = Box::pin(store.wait_for_idle());
        let mut second = Box::pin(store.wait_for_idle());
        assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), &mut first).await.is_err());
        assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), &mut second).await.is_err());
        drop(lease);
        let (first, second) = tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), async { tokio::join!(first, second) }).await.unwrap();
        assert!(first.is_ok() && second.is_ok());
    }

    #[cfg(unix)]
    fn controlled_shell_config() -> taide_infra::pty::PtySpawnConfig {
        taide_infra::pty::PtySpawnConfig {
            shell: Some("/bin/sh".to_string()),
            cwd: std::env::temp_dir().to_string_lossy().to_string(),
            cols: TEST_COLS,
            rows: TEST_ROWS,
            extra_env: vec![("ENV".to_string(), String::new()), ("BASH_ENV".to_string(), String::new())],
        }
    }

    #[cfg(unix)]
    fn fixture_entry(session: PtySession) -> TerminalSessionEntry {
        TerminalSessionEntry::new(
            session,
            Arc::new(TerminalSessionMetadata::new(ProjectId::from("synthetic".to_string()), "/synthetic".to_string(), "sh".to_string())),
            Arc::new(TerminalSessionOutput::new(TEST_SCROLLBACK_BYTES)),
        )
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn 제거와_교체된_세션도_실제_callback_완료_전에는_idle이_아니다() {
        const EXIT_CODE: i32 = 7;
        for action in ["kill", "project", "replace"] {
            let store = TerminalStore::new();
            let (started, started_rx) = tokio::sync::oneshot::channel();
            let (release, held) = std::sync::mpsc::channel::<()>();
            let session = store
                .begin_spawn()
                .unwrap()
                .spawn(|| {
                    taide_infra::pty::spawn(
                        controlled_shell_config(),
                        |_| {},
                        move |code| {
                            started.send(code).ok();
                            held.recv().ok();
                        },
                    )
                })
                .unwrap();
            let completion = session.completion_handle();
            session.write(format!("exit {EXIT_CODE}\n").as_bytes()).unwrap();
            store.insert("held".to_string(), fixture_entry(session)).unwrap();
            assert_eq!(store.0.state.lock().owned.len(), 1);
            let code = tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), started_rx).await.unwrap().unwrap();
            assert_eq!(code, Some(EXIT_CODE));
            match action {
                "kill" => store.kill("held").unwrap(),
                "project" => store.kill_project(&ProjectId::from("synthetic".to_string())),
                "replace" => {
                    let next = store.begin_spawn().unwrap().spawn(|| taide_infra::pty::spawn(controlled_shell_config(), |_| {}, |_| {})).unwrap();
                    store.insert("held".to_string(), fixture_entry(next)).unwrap();
                    assert_eq!(store.0.state.lock().owned.len(), 2);
                }
                _ => unreachable!(),
            }
            store.shutdown();
            let mut waiting = Box::pin(store.wait_for_idle());
            assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), &mut waiting).await.is_err());
            drop(waiting);
            assert!(!completion.is_finished());
            let mut waiting = Box::pin(store.wait_for_idle());
            assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), &mut waiting).await.is_err());
            drop(release);
            tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), waiting).await.unwrap().unwrap();
            assert!(completion.is_finished());
            let state = store.0.state.lock();
            assert!(state.owned.is_empty() && state.drains.is_empty());
        }
    }
}

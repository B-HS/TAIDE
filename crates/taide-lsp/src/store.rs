use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use taide_infra::lsp_proc::LspProcHandle;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::lsp::{LanguageServerSpec, LspServerId, LspSessionInfo};

use crate::session::{LspMessageSubscribers, LspSessionLifecycle, LspSessionRoots};

/// Stores the process slot and shared state of one language-server session.
pub struct LspSessionEntry {
    pub project_id: ProjectId,
    pub server_id: LspServerId,
    pub root: String,
    pub spec: LanguageServerSpec,
    pub proc: Mutex<Option<Arc<LspProcHandle>>>,
    pub subscribers: LspMessageSubscribers,
    pub lifecycle: LspSessionLifecycle,
    pub roots: LspSessionRoots,
}

impl LspSessionEntry {
    pub fn new(
        project_id: ProjectId,
        spec: LanguageServerSpec,
        root: String,
        subscribers: LspMessageSubscribers,
    ) -> Self {
        Self {
            project_id,
            server_id: spec.id.clone(),
            roots: LspSessionRoots::new(root.clone()),
            root,
            spec,
            proc: Mutex::new(None),
            subscribers,
            lifecycle: LspSessionLifecycle::new(),
        }
    }
}

/// Tracks language-server sessions without depending on a UI or IPC runtime.
#[derive(Clone, Default)]
pub struct LspStore(Arc<LspStoreInner>);

#[derive(Default)]
struct LspStoreInner {
    entries: Mutex<HashMap<String, Arc<LspSessionEntry>>>,
    processes: Mutex<LspProcesses>,
}

#[derive(Default)]
struct LspProcesses {
    is_stopped: bool,
    owned: Vec<Arc<LspProcHandle>>,
}

impl LspStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, session_id: &str) -> Option<Arc<LspSessionEntry>> {
        self.0.entries.lock().get(session_id).cloned()
    }

    pub fn insert(&self, session_id: String, entry: Arc<LspSessionEntry>) {
        self.0.entries.lock().insert(session_id, entry);
    }

    pub fn remove(&self, session_id: &str) -> Option<Arc<LspSessionEntry>> {
        self.0.entries.lock().remove(session_id)
    }

    pub fn contains(&self, session_id: &str) -> bool {
        self.0.entries.lock().contains_key(session_id)
    }

    pub fn find_reusable(
        &self,
        project_id: &ProjectId,
        server_id: &LspServerId,
        owner: &str,
    ) -> Option<(String, Arc<LspSessionEntry>)> {
        self.0
            .entries
            .lock()
            .iter()
            .find(|(_, entry)| {
                &entry.project_id == project_id
                    && &entry.server_id == server_id
                    && entry.spec.shares_sessions
                    && !entry.lifecycle.is_stopping()
                    && entry.subscribers.contains(owner)
            })
            .map(|(id, entry)| (id.clone(), entry.clone()))
    }

    pub fn sessions_for_project(&self, project_id: &ProjectId) -> Vec<LspSessionInfo> {
        self.0
            .entries
            .lock()
            .iter()
            .filter(|(_, entry)| &entry.project_id == project_id)
            .map(|(id, entry)| {
                let snapshot = entry.lifecycle.snapshot();
                LspSessionInfo {
                    session_id: id.clone(),
                    project_id: entry.project_id.clone(),
                    server_id: entry.server_id.clone(),
                    root: entry.root.clone(),
                    status: snapshot.status,
                    last_error: snapshot.last_error,
                    generation: snapshot.generation,
                }
            })
            .collect()
    }

    pub fn kill_all(&self) {
        for entry in self.0.entries.lock().values() {
            entry.lifecycle.mark_stopping();
            if let Some(proc) = entry.proc.lock().as_ref() {
                proc.kill();
            }
        }
        let processes = self.0.processes.lock().owned.clone();
        for process in processes {
            process.kill();
        }
    }

    /// Creates and owns a process under the shutdown admission gate, independently of session entries.
    pub fn spawn_process(
        &self,
        create: impl FnOnce() -> AppResult<Arc<LspProcHandle>>,
    ) -> AppResult<Arc<LspProcHandle>> {
        let mut processes = self.0.processes.lock();
        processes.owned.retain(|process| !process.is_finished());
        if processes.is_stopped {
            return Err(AppError::Forbidden(
                "language server runtime is shutting down".to_string(),
            ));
        }
        let process = create()?;
        processes.owned.push(process.clone());
        Ok(process)
    }

    /// Closes process admission before requesting termination, including removed or superseded sessions.
    pub fn shutdown(&self) {
        self.0.processes.lock().is_stopped = true;
        self.kill_all();
    }

    /// Waits for every owned process worker after shutdown; dropping this future does not detach workers.
    pub async fn wait_for_idle(&self) {
        let processes = self.0.processes.lock().owned.clone();
        for process in processes {
            process.wait_for_completion().await;
        }
        self.0
            .processes
            .lock()
            .owned
            .retain(|process| !process.is_finished());
    }

    pub fn server_pids(&self) -> Vec<(ProjectId, String, u32)> {
        self.0
            .entries
            .lock()
            .values()
            .filter_map(|entry| {
                let pid = entry.proc.lock().as_ref().and_then(|proc| proc.pid())?;
                Some((entry.project_id.clone(), entry.spec.name.clone(), pid))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    #[cfg(unix)]
    use std::time::Duration;

    #[cfg(unix)]
    use taide_infra::lsp_proc;
    use taide_model::error::AppError;

    use super::*;

    #[cfg(unix)]
    const FIXTURE_TIMEOUT_MS: u64 = 2_000;

    #[cfg(unix)]
    struct ExitRelease(Option<std::sync::mpsc::Sender<()>>);

    #[cfg(unix)]
    impl Drop for ExitRelease {
        fn drop(&mut self) {
            if let Some(sender) = self.0.take() {
                sender.send(()).ok();
            }
        }
    }

    #[test]
    fn shutdown_뒤에는_프로세스_factory를_호출하지_않는다() {
        let store = LspStore::new();
        let called = AtomicBool::new(false);
        store.shutdown();
        let result = store.spawn_process(|| {
            called.store(true, Ordering::SeqCst);
            Err(AppError::Internal(
                "synthetic factory must not run".to_string(),
            ))
        });
        assert!(result.is_err());
        assert!(!called.load(Ordering::SeqCst));
    }

    #[test]
    fn factory_실패는_프로세스_소유_목록을_늘리지_않는다() {
        let store = LspStore::new();
        assert!(store
            .spawn_process(|| Err(AppError::Internal("synthetic spawn failure".to_string())))
            .is_err());
        assert!(store.0.processes.lock().owned.is_empty());
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn 제거한_세션의_작업도_보유하고_대기_drop_뒤_실제_callback_완료를_다시_기다린다() {
        let store = LspStore::new();
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, held) = std::sync::mpsc::channel();
        let release = ExitRelease(Some(release));
        let process = store
            .spawn_process(|| {
                Ok(Arc::new(lsp_proc::spawn(
                    lsp_proc::LspProcConfig {
                        command: "sh".to_string(),
                        args: vec!["-c".to_string(), "exit 0".to_string()],
                        cwd: std::env::temp_dir(),
                    },
                    |_| {},
                    move |_, _| {
                        started.send(()).ok();
                        held.recv().ok();
                    },
                )?))
            })
            .unwrap();
        let entry = Arc::new(LspSessionEntry::new(
            ProjectId::new(),
            crate::manifest::servers().into_iter().next().unwrap(),
            "synthetic-root".to_string(),
            LspMessageSubscribers::new(),
        ));
        *entry.proc.lock() = Some(process.clone());
        store.insert("synthetic-session".to_string(), entry.clone());
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), ready)
            .await
            .unwrap()
            .unwrap();
        assert!(process.is_exited());
        assert!(!process.is_finished());
        store.remove("synthetic-session");
        drop(entry);
        drop(process);
        store.shutdown();
        let waiting_store = store.clone();
        let waiting = tokio::spawn(async move { waiting_store.wait_for_idle().await });
        tokio::task::yield_now().await;
        assert!(!waiting.is_finished());
        waiting.abort();
        assert!(waiting.await.unwrap_err().is_cancelled());
        drop(release);
        tokio::time::timeout(
            Duration::from_millis(FIXTURE_TIMEOUT_MS),
            store.wait_for_idle(),
        )
        .await
        .unwrap();
    }
}

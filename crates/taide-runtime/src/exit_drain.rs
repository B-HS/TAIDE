use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use taide_lsp::install::LspInstallStore;
use taide_lsp::store::LspStore;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

use crate::TaskSupervisor;

/// Owns exit drainage outside the supervisor it stops, keeping the native event loop available.
#[derive(Default)]
pub struct ExitDrain {
    task: Option<JoinHandle<()>>,
    ready: Arc<AtomicBool>,
}

impl ExitDrain {
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }

    /// Starts one owned drainage task and calls on_ready only after tracked tasks, installs, and LSP workers have ended.
    pub fn begin(
        &mut self,
        runtime: &Handle,
        tasks: TaskSupervisor,
        installs: LspInstallStore,
        processes: LspStore,
        on_ready: impl FnOnce() + Send + 'static,
    ) -> bool {
        if self.is_ready() || self.task.as_ref().is_some_and(|task| !task.is_finished()) {
            return false;
        }
        installs.shutdown();
        processes.shutdown();
        tasks.stop_all();
        let ready = self.ready.clone();
        self.task = Some(runtime.spawn(async move {
            tasks.shutdown().await;
            installs.wait_for_idle().await;
            processes.wait_for_idle().await;
            ready.store(true, Ordering::Release);
            on_ready();
        }));
        true
    }
}

impl Drop for ExitDrain {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use taide_lsp::install::LspInstallStore;
    use taide_lsp::store::LspStore;
    use taide_model::lsp::LspServerId;
    use tokio::sync::oneshot;

    use super::ExitDrain;
    use crate::TaskSupervisor;

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

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn 종료_드레인은_프로세스_종료와_reader_callback_완료를_구분해_기다린다() {
        let runtime = tokio::runtime::Handle::current();
        let tasks = TaskSupervisor::new(runtime.clone());
        let processes = LspStore::new();
        let (started, ready) = oneshot::channel();
        let (release, held) = std::sync::mpsc::channel();
        let release = ExitRelease(Some(release));
        let process = processes
            .spawn_process(|| {
                Ok(Arc::new(taide_infra::lsp_proc::spawn(
                    taide_infra::lsp_proc::LspProcConfig {
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
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), ready)
            .await
            .unwrap()
            .unwrap();
        assert!(process.is_exited());
        assert!(!process.is_finished());
        let mut drain = ExitDrain::default();
        let (exit, mut exit_rx) = oneshot::channel();
        assert!(drain.begin(&runtime, tasks, LspInstallStore::new(), processes, move || {
            exit.send(()).ok();
        }));
        tokio::task::yield_now().await;
        assert!(!drain.is_ready());
        assert!(matches!(exit_rx.try_recv(), Err(tokio::sync::oneshot::error::TryRecvError::Empty)));
        drop(release);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), exit_rx)
            .await
            .unwrap()
            .unwrap();
        assert!(drain.is_ready());
        assert!(process.is_finished());
    }

    #[tokio::test]
    async fn 종료_드레인은_메인_응답을_막지_않고_worker와_lease_종료_뒤_한번만_종료를_요청한다() {
        let runtime = tokio::runtime::Handle::current();
        let tasks = TaskSupervisor::new(runtime.clone());
        let installs = LspInstallStore::new();
        let guard = installs.begin(&LspServerId::from("synthetic-exit")).unwrap();
        let (started, started_rx) = oneshot::channel();
        let (reply, reply_rx) = std::sync::mpsc::channel();
        let worker = tasks
            .spawn_blocking_transient_handle("synthetic-main-reply", move || {
                started.send(()).unwrap();
                reply_rx.recv().unwrap();
            })
            .unwrap();
        started_rx.await.unwrap();
        let mut drain = ExitDrain::default();
        let (exit, exit_rx) = oneshot::channel();
        assert!(drain.begin(&runtime, tasks.clone(), installs.clone(), LspStore::new(), move || {
            exit.send(()).unwrap();
        }));
        let duplicate_exit = Arc::new(AtomicBool::new(false));
        let duplicate_signal = duplicate_exit.clone();
        assert!(!drain.begin(&runtime, tasks.clone(), installs.clone(), LspStore::new(), move || {
            duplicate_signal.store(true, Ordering::SeqCst);
        }));
        assert!(!drain.is_ready());
        assert!(!worker.is_finished());
        reply.send(()).unwrap();
        worker.await.unwrap();
        tokio::task::yield_now().await;
        assert!(!drain.is_ready());
        drop(guard);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), exit_rx)
            .await
            .unwrap()
            .unwrap();
        assert!(drain.is_ready());
        assert_eq!(tasks.tracked_count(), 0);
        assert!(!duplicate_exit.load(Ordering::SeqCst));
        assert!(installs.is_stopped());
        assert!(!drain.begin(&runtime, tasks, installs, LspStore::new(), || {}));
    }

    #[tokio::test]
    async fn 드레인_owner_drop은_완료_전_종료_callback을_실행하지_않는다() {
        let runtime = tokio::runtime::Handle::current();
        let tasks = TaskSupervisor::new(runtime.clone());
        let installs = LspInstallStore::new();
        let guard = installs.begin(&LspServerId::from("synthetic-exit")).unwrap();
        let mut drain = ExitDrain::default();
        let (exit, exit_rx) = oneshot::channel();
        assert!(drain.begin(&runtime, tasks, installs.clone(), LspStore::new(), move || {
            exit.send(()).unwrap();
        }));
        drop(drain);
        assert!(exit_rx.await.is_err());
        drop(guard);
        installs.wait_for_idle().await;
    }
}

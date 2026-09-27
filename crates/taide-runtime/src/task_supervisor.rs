use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::runtime::Handle;
use tokio::task::{AbortHandle, JoinHandle};

const SHUTDOWN_POLL_INTERVAL_MS: u64 = 10;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum TaskKey {
    Named(&'static str),
    Transient(u64, &'static str),
    Blocking(u64, &'static str),
}

#[derive(Default)]
struct TaskState {
    is_stopped: bool,
    next_transient_id: u64,
    handles: HashMap<TaskKey, AbortHandle>,
    operations: HashMap<u64, &'static str>,
}

#[derive(Clone)]
pub struct TaskSupervisor(Arc<TaskSupervisorInner>);

/// Keeps an admitted operation tracked until its final owner is dropped.
#[derive(Clone)]
pub struct TaskOperationLease {
    _owner: Arc<TaskOperationLeaseInner>,
}

struct TaskOperationLeaseInner {
    supervisor: TaskSupervisor,
    id: u64,
}

impl Drop for TaskOperationLeaseInner {
    fn drop(&mut self) {
        self.supervisor
            .0
            .state
            .lock()
            .expect("task supervisor lock poisoned")
            .operations
            .remove(&self.id);
    }
}

struct TaskSupervisorInner {
    runtime: Handle,
    state: Mutex<TaskState>,
}

impl TaskState {
    fn prune_finished(&mut self) {
        self.handles.retain(|_, handle| !handle.is_finished());
    }
}

impl TaskSupervisor {
    pub fn new(runtime: Handle) -> Self {
        Self(Arc::new(TaskSupervisorInner {
            runtime,
            state: Mutex::new(TaskState::default()),
        }))
    }

    /// Admits an operation whose owners may outlive its registered worker.
    pub fn begin_operation(&self, name: &'static str) -> Option<TaskOperationLease> {
        let mut state = self.0.state.lock().expect("task supervisor lock poisoned");
        if state.is_stopped {
            return None;
        }
        let id = state.next_transient_id.checked_add(1).expect("task supervisor ID exhausted");
        state.next_transient_id = id;
        state.operations.insert(id, name);
        Some(TaskOperationLease {
            _owner: Arc::new(TaskOperationLeaseInner {
                supervisor: self.clone(),
                id,
            }),
        })
    }

    pub fn spawn(&self, name: &'static str, task: impl Future<Output = ()> + Send + 'static) -> bool {
        let mut state = self.0.state.lock().expect("task supervisor lock poisoned");
        self.spawn_locked(&mut state, TaskKey::Named(name), task).is_some()
    }

    pub fn spawn_transient(&self, name: &'static str, task: impl Future<Output = ()> + Send + 'static) -> bool {
        self.spawn_transient_handle(name, task).is_some()
    }

    pub fn spawn_transient_handle(&self, name: &'static str, task: impl Future<Output = ()> + Send + 'static) -> Option<JoinHandle<()>> {
        let mut state = self.0.state.lock().expect("task supervisor lock poisoned");
        if state.is_stopped {
            return None;
        }
        let task_id = state.next_transient_id.checked_add(1).expect("task supervisor ID exhausted");
        state.next_transient_id = task_id;
        self.spawn_locked(&mut state, TaskKey::Transient(task_id, name), task)
    }

    pub fn spawn_blocking_transient_handle(&self, name: &'static str, task: impl FnOnce() + Send + 'static) -> Option<JoinHandle<()>> {
        let mut state = self.0.state.lock().expect("task supervisor lock poisoned");
        state.prune_finished();
        if state.is_stopped {
            return None;
        }
        let task_id = state.next_transient_id.checked_add(1).expect("task supervisor ID exhausted");
        state.next_transient_id = task_id;
        let key = TaskKey::Blocking(task_id, name);
        let supervisor = Arc::downgrade(&self.0);
        let handle = self.0.runtime.spawn_blocking(move || {
            let Some(supervisor) = supervisor.upgrade() else {
                return;
            };
            if supervisor.state.lock().expect("task supervisor lock poisoned").is_stopped {
                return;
            }
            task();
        });
        state.handles.insert(key, handle.abort_handle());
        Some(handle)
    }

    fn spawn_locked(&self, state: &mut TaskState, key: TaskKey, task: impl Future<Output = ()> + Send + 'static) -> Option<JoinHandle<()>> {
        state.prune_finished();
        if state.is_stopped || state.handles.contains_key(&key) {
            return None;
        }

        let handle = self.0.runtime.spawn(task);
        state.handles.insert(key, handle.abort_handle());
        Some(handle)
    }

    pub fn tracked_count(&self) -> usize {
        let mut state = self.0.state.lock().expect("task supervisor lock poisoned");
        state.prune_finished();
        state.handles.len() + state.operations.len()
    }

    fn cancel_all(&self) -> Vec<AbortHandle> {
        let handles = {
            let mut state = self.0.state.lock().expect("task supervisor lock poisoned");
            state.is_stopped = true;
            state.prune_finished();
            state.handles.values().cloned().collect::<Vec<_>>()
        };
        for handle in &handles {
            handle.abort();
        }
        handles
    }

    /// Requests cancellation and keeps unfinished async and blocking tasks tracked.
    pub fn stop_all(&self) {
        self.cancel_all();
    }

    /// Closes admission and waits for task completion and the final drop of admitted operation owners.
    pub async fn shutdown(&self) {
        let handles = self.cancel_all();
        while handles.iter().any(|handle| !handle.is_finished())
            || !self.0.state.lock().expect("task supervisor lock poisoned").operations.is_empty()
        {
            tokio::time::sleep(Duration::from_millis(SHUTDOWN_POLL_INTERVAL_MS)).await;
        }
        self.0.state.lock().expect("task supervisor lock poisoned").prune_finished();
    }
}

#[cfg(test)]
mod tests {
    use std::future::pending;
    use std::time::Duration;

    use tokio::sync::oneshot;

    use super::TaskSupervisor;

    const FIXTURE_TIMEOUT_MS: u64 = 2_000;
    const BLOCKED_WORK_OBSERVATION_MS: u64 = 20;

    #[tokio::test]
    async fn 공유_operation은_마지막_owner_drop까지_추적하며_종료_요청으로_완료되지_않는다() {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let owner = tasks.begin_operation("synthetic-operation").unwrap();
        let shared = owner.clone();
        assert_eq!(tasks.tracked_count(), 1);
        tasks.stop_all();
        assert!(tasks.begin_operation("synthetic-operation").is_none());
        drop(owner);
        assert_eq!(tasks.tracked_count(), 1);
        assert!(
            tokio::time::timeout(Duration::from_millis(BLOCKED_WORK_OBSERVATION_MS), tasks.shutdown())
                .await
                .is_err()
        );
        drop(shared);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    }

    #[tokio::test]
    async fn 같은_이름의_독립_operation은_각각_반납하며_감독자의_최종_소유를_유지한다() {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let first = tasks.begin_operation("synthetic-operation").unwrap();
        let second = tasks.begin_operation("synthetic-operation").unwrap();
        let weak = std::sync::Arc::downgrade(&tasks.0);
        assert_eq!(tasks.tracked_count(), 2);
        drop(first);
        assert_eq!(tasks.tracked_count(), 1);
        drop(tasks);
        assert!(weak.upgrade().is_some());
        drop(second);
        assert!(weak.upgrade().is_none());
    }

    #[tokio::test]
    async fn stop_all은_취소_요청_직후_async_작업을_완료로_집계하지_않는다() {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let (sender, receiver) = oneshot::channel::<()>();
        assert!(tasks.spawn_transient("synthetic-pending", async move {
            let _owner = sender;
            pending::<()>().await;
        }));
        tasks.stop_all();
        assert_eq!(tasks.tracked_count(), 1);
        assert!(receiver.await.is_err());
        assert_eq!(tasks.tracked_count(), 0);
    }

    #[tokio::test]
    async fn shutdown은_async_owner의_실제_drop을_기다리고_모든_신규_등록을_거절한다() {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let (sender, receiver) = oneshot::channel::<()>();
        assert!(tasks.spawn("synthetic-owned", async move {
            let _owner = sender;
            pending::<()>().await;
        }));
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), tasks.shutdown())
            .await
            .unwrap();
        assert!(receiver.await.is_err());
        assert_eq!(tasks.tracked_count(), 0);
        assert!(!tasks.spawn("synthetic-owned", async {}));
        assert!(tasks.spawn_transient_handle("synthetic-owned", async {}).is_none());
        assert!(tasks.spawn_blocking_transient_handle("synthetic-owned", || {}).is_none());
        tasks.shutdown().await;
    }

    #[tokio::test]
    async fn shutdown_대기가_drop되어도_시작한_blocking_작업의_완료를_다시_기다린다() {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let (started, started_rx) = oneshot::channel();
        let (release, release_rx) = std::sync::mpsc::channel();
        let worker = tasks
            .spawn_blocking_transient_handle("synthetic-blocked", move || {
                started.send(()).unwrap();
                release_rx.recv().unwrap();
            })
            .unwrap();
        started_rx.await.unwrap();
        tasks.stop_all();
        let mut shutdown = Box::pin(tasks.shutdown());
        assert!(
            tokio::time::timeout(Duration::from_millis(BLOCKED_WORK_OBSERVATION_MS), &mut shutdown)
                .await
                .is_err()
        );
        drop(shutdown);
        assert_eq!(tasks.tracked_count(), 1);
        assert!(!worker.is_finished());
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), tasks.shutdown())
            .await
            .unwrap();
        assert!(worker.is_finished());
        worker.await.unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    }
}

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, Weak};

use tokio::runtime::Handle;
use tokio::task::{AbortHandle, JoinHandle};

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
}

#[derive(Clone)]
pub struct TaskSupervisor(Arc<TaskSupervisorInner>);

struct TaskSupervisorInner {
    runtime: Handle,
    state: Mutex<TaskState>,
}

struct TaskCleanup {
    supervisor: Weak<TaskSupervisorInner>,
    key: TaskKey,
}

impl Drop for TaskCleanup {
    fn drop(&mut self) {
        let Some(supervisor) = self.supervisor.upgrade() else {
            return;
        };
        supervisor
            .state
            .lock()
            .expect("task supervisor lock poisoned")
            .handles
            .remove(&self.key);
    }
}

impl TaskSupervisor {
    pub fn new(runtime: Handle) -> Self {
        Self(Arc::new(TaskSupervisorInner {
            runtime,
            state: Mutex::new(TaskState::default()),
        }))
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
        if state.is_stopped {
            return None;
        }
        let task_id = state.next_transient_id.checked_add(1).expect("task supervisor ID exhausted");
        state.next_transient_id = task_id;
        let key = TaskKey::Blocking(task_id, name);
        let cleanup = TaskCleanup {
            supervisor: Arc::downgrade(&self.0),
            key,
        };
        let handle = self.0.runtime.spawn_blocking(move || {
            let cleanup = cleanup;
            let Some(supervisor) = cleanup.supervisor.upgrade() else {
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
        if state.is_stopped || state.handles.contains_key(&key) {
            return None;
        }

        let cleanup = TaskCleanup {
            supervisor: Arc::downgrade(&self.0),
            key,
        };
        let handle = self.0.runtime.spawn(async move {
            let _cleanup = cleanup;
            task.await;
        });
        state.handles.insert(key, handle.abort_handle());
        Some(handle)
    }

    pub fn tracked_count(&self) -> usize {
        self.0.state.lock().expect("task supervisor lock poisoned").handles.len()
    }

    pub fn stop_all(&self) {
        let handles = {
            let mut state = self.0.state.lock().expect("task supervisor lock poisoned");
            state.is_stopped = true;
            let handles: Vec<_> = state.handles.values().cloned().collect();
            state.handles.retain(|key, _| matches!(key, TaskKey::Blocking(..)));
            handles
        };

        for handle in handles {
            handle.abort();
        }
    }
}

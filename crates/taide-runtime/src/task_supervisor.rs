use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, Weak};

use tokio::runtime::Handle;
use tokio::task::AbortHandle;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum TaskKey {
    Named(&'static str),
    Transient(u64, &'static str),
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
        self.spawn_locked(&mut state, TaskKey::Named(name), task)
    }

    pub fn spawn_transient(&self, name: &'static str, task: impl Future<Output = ()> + Send + 'static) -> bool {
        let mut state = self.0.state.lock().expect("task supervisor lock poisoned");
        if state.is_stopped {
            return false;
        }
        let task_id = state.next_transient_id.checked_add(1).expect("task supervisor ID exhausted");
        state.next_transient_id = task_id;
        self.spawn_locked(&mut state, TaskKey::Transient(task_id, name), task)
    }

    fn spawn_locked(&self, state: &mut TaskState, key: TaskKey, task: impl Future<Output = ()> + Send + 'static) -> bool {
        if state.is_stopped || state.handles.contains_key(&key) {
            return false;
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
        true
    }

    pub fn tracked_count(&self) -> usize {
        self.0.state.lock().expect("task supervisor lock poisoned").handles.len()
    }

    pub fn stop_all(&self) {
        let handles = {
            let mut state = self.0.state.lock().expect("task supervisor lock poisoned");
            state.is_stopped = true;
            std::mem::take(&mut state.handles)
        };

        for handle in handles.into_values() {
            handle.abort();
        }
    }
}

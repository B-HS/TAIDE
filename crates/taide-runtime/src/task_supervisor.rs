use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

use tokio::runtime::Handle;
use tokio::task::AbortHandle;

#[derive(Default)]
struct TaskState {
    is_stopped: bool,
    handles: HashMap<&'static str, AbortHandle>,
}

#[derive(Clone)]
pub struct TaskSupervisor(Arc<TaskSupervisorInner>);

struct TaskSupervisorInner {
    runtime: Handle,
    state: Mutex<TaskState>,
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
        if state.is_stopped || state.handles.contains_key(name) {
            return false;
        }

        let handle = self.0.runtime.spawn(task);
        state.handles.insert(name, handle.abort_handle());
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

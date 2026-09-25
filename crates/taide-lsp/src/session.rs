use std::collections::HashMap;

use parking_lot::Mutex;
use taide_model::lsp::LspSessionStatus;

type MessageSink = Box<dyn Fn(&str) -> bool + Send + Sync>;

/// Stores LSP message sinks by owner and removes sinks that reject a message.
#[derive(Default)]
pub struct LspMessageSubscribers(Mutex<HashMap<String, MessageSink>>);

impl LspMessageSubscribers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, owner: String, sink: impl Fn(&str) -> bool + Send + Sync + 'static) {
        self.0.lock().insert(owner, Box::new(sink));
    }

    pub fn contains(&self, owner: &str) -> bool {
        self.0.lock().contains_key(owner)
    }

    pub fn remove(&self, owner: &str) {
        self.0.lock().remove(owner);
    }

    pub fn broadcast(&self, message: &str) {
        self.0.lock().retain(|_, sink| sink(message));
    }
}

/// Counts root references shared by one LSP session.
pub struct LspSessionRoots(Mutex<Vec<(String, u32)>>);

/// Describes whether releasing a root removed a workspace folder or ended the session.
#[derive(Debug, PartialEq, Eq)]
pub struct LspRootRelease {
    pub removed_root: Option<String>,
    pub has_remaining_roots: bool,
}

impl LspSessionRoots {
    pub fn new(root: String) -> Self {
        Self(Mutex::new(vec![(root, 1)]))
    }

    pub fn paths(&self) -> Vec<String> {
        self.0.lock().iter().map(|(root, _)| root.clone()).collect()
    }

    /// Returns true when the root is newly added to this session.
    pub fn acquire(&self, root: String) -> bool {
        let mut roots = self.0.lock();
        if let Some((_, count)) = roots
            .iter_mut()
            .find(|(existing_root, _)| existing_root == &root)
        {
            *count += 1;
            return false;
        }
        roots.push((root, 1));
        true
    }

    /// Releases one reference and reports the remaining session roots.
    pub fn release(&self, root: &str) -> LspRootRelease {
        let mut roots = self.0.lock();
        let mut removed_root = None;
        if let Some(position) = roots
            .iter()
            .position(|(existing_root, _)| existing_root == root)
        {
            roots[position].1 = roots[position].1.saturating_sub(1);
            if roots[position].1 == 0 {
                removed_root = Some(roots.remove(position).0);
            }
        }
        LspRootRelease {
            removed_root,
            has_remaining_roots: !roots.is_empty(),
        }
    }
}

/// One coherent view of an LSP session's process lifecycle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LspLifecycleSnapshot {
    pub status: LspSessionStatus,
    pub last_error: Option<String>,
    pub generation: u32,
}

struct LspLifecycleState {
    snapshot: LspLifecycleSnapshot,
    restart_count: u32,
    is_stopping: bool,
}

/// Owns session lifecycle transitions independently of the Tauri process adapter.
pub struct LspSessionLifecycle(Mutex<LspLifecycleState>);

impl LspSessionLifecycle {
    pub fn new() -> Self {
        Self(Mutex::new(LspLifecycleState {
            snapshot: LspLifecycleSnapshot {
                status: LspSessionStatus::Starting,
                last_error: None,
                generation: 0,
            },
            restart_count: 0,
            is_stopping: false,
        }))
    }

    pub fn snapshot(&self) -> LspLifecycleSnapshot {
        self.0.lock().snapshot.clone()
    }

    pub fn is_stopping(&self) -> bool {
        self.0.lock().is_stopping
    }

    pub fn mark_stopping(&self) {
        self.0.lock().is_stopping = true;
    }

    pub fn begin_exit_recovery(&self) -> Option<u32> {
        let mut state = self.0.lock();
        if state.is_stopping {
            return None;
        }
        state.restart_count = state.restart_count.wrapping_add(1);
        Some(state.restart_count)
    }

    pub fn reset_restart_count(&self) {
        self.0.lock().restart_count = 0;
    }

    pub fn set_status(
        &self,
        status: LspSessionStatus,
        last_error: Option<String>,
    ) -> LspLifecycleSnapshot {
        let mut state = self.0.lock();
        state.snapshot.status = status;
        state.snapshot.last_error = last_error;
        state.snapshot.clone()
    }

    pub fn auto_respawned(&self, last_error: String) -> LspLifecycleSnapshot {
        let mut state = self.0.lock();
        state.snapshot.generation = state.snapshot.generation.wrapping_add(1);
        state.snapshot.status = LspSessionStatus::Crashed;
        state.snapshot.last_error = Some(last_error);
        state.snapshot.clone()
    }

    pub fn begin_manual_restart(&self) -> LspLifecycleSnapshot {
        let mut state = self.0.lock();
        state.is_stopping = false;
        state.restart_count = 0;
        state.snapshot.status = LspSessionStatus::Starting;
        state.snapshot.last_error = None;
        state.snapshot.clone()
    }

    pub fn confirm_reinitialized(&self, generation: u32) -> Option<LspLifecycleSnapshot> {
        let mut state = self.0.lock();
        if state.snapshot.generation != generation
            || state.snapshot.status != LspSessionStatus::Crashed
        {
            return None;
        }
        state.snapshot.status = LspSessionStatus::Running;
        state.snapshot.last_error = None;
        Some(state.snapshot.clone())
    }

    pub fn report_reinitialize_failure(
        &self,
        generation: u32,
        last_error: String,
    ) -> Option<LspLifecycleSnapshot> {
        let mut state = self.0.lock();
        if state.snapshot.generation != generation
            || state.snapshot.status != LspSessionStatus::Crashed
        {
            return None;
        }
        state.snapshot.last_error = Some(last_error);
        Some(state.snapshot.clone())
    }
}

impl Default for LspSessionLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

use std::collections::HashMap;

use parking_lot::Mutex;

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

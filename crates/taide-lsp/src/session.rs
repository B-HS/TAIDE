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

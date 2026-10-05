use std::collections::BTreeMap;

use serde_json::{Value, json};
use taide_model::ids::TabId;

use crate::shell::{Call, Failure};
use crate::{InvokeError, ResponsePayload};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirtyUpdate {
    pub tab: TabId,
    pub dirty: bool,
}

impl DirtyUpdate {
    pub fn call(&self) -> Call {
        Call {
            command: "layout_set_dirty",
            args: json!({"tabId": self.tab, "dirty": self.dirty}),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DirtyFailure {
    pub update: DirtyUpdate,
    pub error: Failure,
}

pub enum FlushState<'a> {
    Ready,
    Pending,
    Failed(&'a Failure),
}

struct Entry {
    desired: bool,
    confirmed: Option<bool>,
    failure: Option<Failure>,
}

impl Default for Entry {
    fn default() -> Self {
        Self {
            desired: false,
            confirmed: Some(false),
            failure: None,
        }
    }
}

#[derive(Default)]
pub struct DirtyState {
    entries: BTreeMap<TabId, Entry>,
    inflight: BTreeMap<u32, DirtyUpdate>,
    failures: Vec<DirtyFailure>,
}

impl DirtyState {
    pub fn observe(&mut self, tab: TabId, dirty: bool) {
        let entry = self.entries.entry(tab).or_default();
        if entry.desired != dirty {
            entry.desired = dirty;
            entry.failure = None;
        }
    }

    pub fn retain(&mut self, mut active: impl FnMut(&TabId) -> bool) {
        self.entries.retain(|tab, _| active(tab));
    }

    pub fn next_updates(&self) -> Vec<DirtyUpdate> {
        self.entries
            .iter()
            .filter(|(tab, entry)| {
                Some(entry.desired) != entry.confirmed
                    && entry.failure.is_none()
                    && !self.inflight.values().any(|update| &update.tab == *tab)
            })
            .map(|(tab, entry)| DirtyUpdate {
                tab: tab.clone(),
                dirty: entry.desired,
            })
            .collect()
    }

    pub fn sent(&mut self, update: DirtyUpdate, seq: u32) {
        self.inflight.insert(seq, update);
    }

    pub fn invocation_failed(&mut self, update: DirtyUpdate, error: InvokeError) {
        self.failed(update, Failure::Invocation(error));
    }

    pub fn response(&mut self, seq: u32, result: &Result<ResponsePayload, Value>) -> bool {
        let Some(update) = self.inflight.remove(&seq) else {
            return false;
        };
        let result = match result {
            Ok(ResponsePayload::Json(value)) => {
                serde_json::from_value::<()>(value.clone()).map_err(|_| Failure::MalformedResponse)
            }
            Ok(ResponsePayload::Binary(_)) => Err(Failure::MalformedResponse),
            Err(error) => Err(Failure::Remote(error.clone())),
        };
        match result {
            Ok(()) => {
                if let Some(entry) = self.entries.get_mut(&update.tab) {
                    entry.confirmed = Some(update.dirty);
                    entry.failure = None;
                }
            }
            Err(error) => self.failed(update, error),
        }
        true
    }

    pub fn flush_state(&self) -> FlushState<'_> {
        if let Some(error) = self
            .entries
            .values()
            .find_map(|entry| entry.failure.as_ref())
        {
            return FlushState::Failed(error);
        }
        if !self.inflight.is_empty()
            || self
                .entries
                .values()
                .any(|entry| Some(entry.desired) != entry.confirmed)
        {
            return FlushState::Pending;
        }
        FlushState::Ready
    }

    pub fn retry(&mut self) {
        for entry in self.entries.values_mut() {
            entry.failure = None;
        }
    }

    pub fn disconnected(&mut self) {
        for (_, update) in std::mem::take(&mut self.inflight) {
            self.invocation_failed(update, InvokeError::Closed);
        }
        for update in self.next_updates() {
            self.invocation_failed(update, InvokeError::Closed);
        }
    }

    pub fn take_failures(&mut self) -> Vec<DirtyFailure> {
        std::mem::take(&mut self.failures)
    }

    fn failed(&mut self, update: DirtyUpdate, error: Failure) {
        if let Some(entry) = self.entries.get_mut(&update.tab) {
            entry.confirmed = None;
            if entry.desired == update.dirty || error == Failure::Invocation(InvokeError::Closed) {
                entry.failure = Some(error.clone());
            }
        }
        self.failures.push(DirtyFailure { update, error });
    }
}

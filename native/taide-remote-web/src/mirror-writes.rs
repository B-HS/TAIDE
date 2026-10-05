use std::collections::BTreeMap;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use taide_model::file::{MirrorEntry, MirrorWriteReceipt};
use taide_model::ids::ProjectId;
use taide_native_editor::document::{DocumentId, EditorError};

use crate::shell::{Call, Failure};
use crate::{InvokeError, ResponsePayload};

pub const MIRROR_DEBOUNCE: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MirrorKey {
    pub project: ProjectId,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MirrorError {
    Rpc(Failure),
    Editor(EditorError),
    TimerUnavailable,
    EpochExhausted,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MirrorFailure {
    pub key: MirrorKey,
    pub error: MirrorError,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MirrorFlushStatus {
    Ready,
    Pending,
    Failed(MirrorError),
}

#[derive(Clone)]
pub struct WriteRequest {
    key: MirrorKey,
    document: DocumentId,
    revision: u64,
    epoch: u64,
    content: String,
}

impl WriteRequest {
    pub fn call(&self) -> Call {
        Call {
            command: "file_mirror_dirty",
            args: json!({"projectId":self.key.project,"path":self.key.path,"content":self.content,"receipt":true}),
        }
    }
}

#[derive(Clone)]
pub enum ClearExpected {
    Receipt(MirrorWriteReceipt),
    Listed(Option<MirrorEntry>),
}

#[derive(Clone)]
pub struct ClearRequest {
    id: u64,
    key: MirrorKey,
    expected: ClearExpected,
}

impl ClearRequest {
    pub fn call(&self) -> Call {
        let mut args = json!({"projectId":self.key.project,"path":self.key.path});
        match &self.expected {
            ClearExpected::Receipt(receipt) => args["expectedReceipt"] = json!(receipt),
            ClearExpected::Listed(entry) => args["expected"] = json!(entry),
        }
        Call {
            command: "file_clear_mirror",
            args,
        }
    }
}

struct Entry {
    document: DocumentId,
    epoch: u64,
    revision: Option<u64>,
    due: Option<Duration>,
    active: bool,
    failure: Option<MirrorError>,
}

struct Cleanup {
    request: ClearRequest,
    failure: Option<MirrorError>,
}

enum Pending {
    Write(WriteRequest),
    Clear(u64),
}

pub enum MirrorOutcome {
    Committed {
        key: MirrorKey,
        receipt: MirrorWriteReceipt,
    },
    Invalidated(MirrorKey),
}

#[derive(Default)]
pub struct MirrorWrites {
    entries: BTreeMap<MirrorKey, Entry>,
    pending: BTreeMap<u32, Pending>,
    cleanups: BTreeMap<u64, Cleanup>,
    next_cleanup: u64,
    failures: Vec<MirrorFailure>,
    outcomes: Vec<MirrorOutcome>,
}

impl MirrorWrites {
    pub fn observe(&mut self, key: MirrorKey, document: DocumentId, revision: u64, now: Duration) {
        let entry = self.entries.entry(key.clone()).or_insert(Entry {
            document,
            epoch: 0,
            revision: None,
            due: None,
            active: true,
            failure: None,
        });
        if entry.document != document {
            let Some(epoch) = entry.epoch.checked_add(1) else {
                self.fail(key, MirrorError::EpochExhausted);
                return;
            };
            entry.epoch = epoch;
            entry.document = document;
        }
        entry.active = true;
        entry.revision = Some(revision);
        entry.due = Some(now.saturating_add(MIRROR_DEBOUNCE));
        entry.failure = None;
    }

    pub fn document(&self, key: &MirrorKey) -> Option<DocumentId> {
        self.entries.get(key).map(|entry| entry.document)
    }

    pub fn keys_for_document(&self, document: DocumentId) -> Vec<MirrorKey> {
        self.entries
            .iter()
            .filter(|(_, entry)| entry.document == document)
            .map(|(key, _)| key.clone())
            .collect()
    }

    pub fn has_project(&self, project: &ProjectId) -> bool {
        self.entries.keys().any(|key| &key.project == project)
            || self
                .cleanups
                .values()
                .any(|cleanup| &cleanup.request.key.project == project)
    }

    pub fn settle(&mut self, key: &MirrorKey, dirty: bool, now: Duration) {
        let Some(entry) = self.entries.get_mut(key) else {
            return;
        };
        let Some(epoch) = entry.epoch.checked_add(1) else {
            self.fail(key.clone(), MirrorError::EpochExhausted);
            return;
        };
        entry.epoch = epoch;
        if !dirty {
            entry.revision = None;
            entry.due = None;
            entry.failure = None;
            return;
        }
        if entry.revision.is_some() && entry.failure.is_none() {
            entry.due = Some(now);
        }
    }

    pub fn retain_active(&mut self, mut active: impl FnMut(&MirrorKey) -> bool, now: Duration) {
        for (key, entry) in &mut self.entries {
            let is_active = active(key);
            if entry.active && !is_active && entry.revision.is_some() && entry.failure.is_none() {
                entry.due = Some(now);
            }
            entry.active = is_active;
        }
        self.collect_unused();
    }

    fn has_write(&self, key: &MirrorKey) -> bool {
        self.pending
            .values()
            .any(|pending| matches!(pending, Pending::Write(request) if &request.key == key))
    }

    pub fn next_deadline(&self) -> Option<Duration> {
        self.entries
            .iter()
            .filter(|(key, entry)| entry.failure.is_none() && !self.has_write(key))
            .filter_map(|(_, entry)| entry.due)
            .min()
    }

    pub fn ready(&self, now: Duration) -> Vec<MirrorKey> {
        self.entries
            .iter()
            .filter(|(key, entry)| {
                entry.revision.is_some()
                    && entry.failure.is_none()
                    && entry.due.is_some_and(|due| due <= now)
                    && !self.has_write(key)
            })
            .map(|(key, _)| key.clone())
            .collect()
    }

    pub fn prepare(
        &mut self,
        key: &MirrorKey,
        revision: u64,
        content: String,
    ) -> Option<WriteRequest> {
        if self.has_write(key) {
            return None;
        }
        let entry = self.entries.get_mut(key)?;
        if entry.revision.is_none() || entry.failure.is_some() {
            return None;
        }
        entry.revision = Some(revision);
        Some(WriteRequest {
            key: key.clone(),
            document: entry.document,
            revision,
            epoch: entry.epoch,
            content,
        })
    }

    pub fn sent(&mut self, request: WriteRequest, seq: u32) {
        if let Some(entry) = self.entries.get_mut(&request.key) {
            entry.due = None;
        }
        self.pending.insert(seq, Pending::Write(request));
    }

    pub fn invocation_failed(&mut self, request: WriteRequest, error: InvokeError) {
        self.fail(request.key, MirrorError::Rpc(Failure::Invocation(error)));
    }

    pub fn fail(&mut self, key: MirrorKey, error: MirrorError) {
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.due = None;
            entry.failure = Some(error.clone());
        }
        self.failures.push(MirrorFailure { key, error });
    }

    pub fn queue_clear(&mut self, key: MirrorKey, expected: ClearExpected) {
        let Some(id) = self.next_cleanup.checked_add(1) else {
            self.fail(key, MirrorError::EpochExhausted);
            return;
        };
        self.next_cleanup = id;
        self.cleanups.insert(
            id,
            Cleanup {
                request: ClearRequest { id, key, expected },
                failure: None,
            },
        );
    }

    pub fn next_clears(&self) -> Vec<ClearRequest> {
        self.cleanups
            .iter()
            .filter(|(id, cleanup)| {
                cleanup.failure.is_none()
                    && !self
                        .pending
                        .values()
                        .any(|pending| matches!(pending, Pending::Clear(pending) if pending == *id))
            })
            .map(|(_, cleanup)| cleanup.request.clone())
            .collect()
    }

    pub fn clear_sent(&mut self, request: ClearRequest, seq: u32) {
        self.pending.insert(seq, Pending::Clear(request.id));
    }

    pub fn clear_failed(&mut self, request: ClearRequest, error: InvokeError) {
        let error = MirrorError::Rpc(Failure::Invocation(error));
        if let Some(cleanup) = self.cleanups.get_mut(&request.id) {
            cleanup.failure = Some(error.clone());
        }
        self.failures.push(MirrorFailure {
            key: request.key,
            error,
        });
    }

    pub fn response(
        &mut self,
        seq: u32,
        result: &Result<ResponsePayload, Value>,
        now: Duration,
        mut draft: impl FnMut(DocumentId) -> Option<String>,
    ) -> bool {
        let Some(pending) = self.pending.remove(&seq) else {
            return false;
        };
        match pending {
            Pending::Write(request) => {
                let receipt = parse::<MirrorWriteReceipt>(result).and_then(|receipt| {
                    if receipt.entry.path != request.key.path
                        || receipt.entry.content != request.content
                        || receipt.write_id.as_str().is_empty()
                        || receipt.entry.conflict
                        || receipt.entry.source_missing != receipt.entry.disk_modified_ms.is_none()
                    {
                        return Err(MirrorError::Rpc(Failure::MalformedResponse));
                    }
                    Ok(receipt)
                });
                match receipt {
                    Err(error) => self.fail(request.key, error),
                    Ok(receipt) => {
                        let entry = self.entries.get_mut(&request.key);
                        if let Some(entry) =
                            entry.filter(|entry| entry.document == request.document)
                        {
                            if entry.epoch == request.epoch {
                                if entry.revision == Some(request.revision) {
                                    entry.revision = None;
                                    entry.due = None;
                                }
                                self.outcomes.push(MirrorOutcome::Committed {
                                    key: request.key,
                                    receipt,
                                });
                                self.collect_unused();
                                return true;
                            }
                            if entry.revision.is_some()
                                && draft(entry.document).as_deref()
                                    == Some(request.content.as_str())
                            {
                                if entry.failure.is_none() {
                                    entry.due = Some(now);
                                }
                                return true;
                            }
                        }
                        self.queue_clear(request.key, ClearExpected::Receipt(receipt));
                    }
                }
            }
            Pending::Clear(id) => match parse::<bool>(result) {
                Ok(_) => {
                    if let Some(cleanup) = self.cleanups.remove(&id) {
                        self.outcomes
                            .push(MirrorOutcome::Invalidated(cleanup.request.key));
                    }
                }
                Err(error) => {
                    if let Some(cleanup) = self.cleanups.get_mut(&id) {
                        cleanup.failure = Some(error.clone());
                        self.failures.push(MirrorFailure {
                            key: cleanup.request.key.clone(),
                            error,
                        });
                    }
                }
            },
        }
        self.collect_unused();
        true
    }

    pub fn flush(&mut self, now: Duration) {
        for entry in self
            .entries
            .values_mut()
            .filter(|entry| entry.revision.is_some() && entry.failure.is_none())
        {
            entry.due = Some(now);
        }
    }

    pub fn flush_key(&mut self, key: &MirrorKey, now: Duration) {
        if let Some(entry) = self.entries.get_mut(key)
            && entry.revision.is_some()
            && entry.failure.is_none()
        {
            entry.due = Some(now);
        }
    }

    pub fn retry(&mut self, now: Duration) {
        for entry in self.entries.values_mut() {
            entry.failure = None;
            if entry.revision.is_some() {
                entry.due = Some(now);
            }
        }
        for cleanup in self.cleanups.values_mut() {
            cleanup.failure = None;
        }
        self.collect_unused();
    }

    pub fn disconnected(&mut self) {
        let inflight = std::mem::take(&mut self.pending);
        for entry in self.entries.values_mut() {
            entry.due = None;
        }
        let keys = self.entries.iter().filter(|(key, entry)| entry.failure.is_none() && (entry.revision.is_some()
            || inflight.values().any(|pending| matches!(pending, Pending::Write(request) if &request.key == *key))))
            .map(|(key, _)| key.clone()).collect::<Vec<_>>();
        for key in keys {
            self.fail(
                key,
                MirrorError::Rpc(Failure::Invocation(InvokeError::Closed)),
            );
        }
        for cleanup in self
            .cleanups
            .values_mut()
            .filter(|cleanup| cleanup.failure.is_none())
        {
            let error = MirrorError::Rpc(Failure::Invocation(InvokeError::Closed));
            cleanup.failure = Some(error.clone());
            self.failures.push(MirrorFailure {
                key: cleanup.request.key.clone(),
                error,
            });
        }
    }

    pub fn flush_status(&self) -> MirrorFlushStatus {
        if let Some(error) = self
            .entries
            .values()
            .find_map(|entry| entry.failure.as_ref())
            .or_else(|| {
                self.cleanups
                    .values()
                    .find_map(|cleanup| cleanup.failure.as_ref())
            })
        {
            return MirrorFlushStatus::Failed(error.clone());
        }
        if self.entries.values().any(|entry| entry.revision.is_some())
            || !self.pending.is_empty()
            || !self.cleanups.is_empty()
        {
            return MirrorFlushStatus::Pending;
        }
        MirrorFlushStatus::Ready
    }

    pub fn take_failures(&mut self) -> Vec<MirrorFailure> {
        std::mem::take(&mut self.failures)
    }
    pub fn take_outcomes(&mut self) -> Vec<MirrorOutcome> {
        std::mem::take(&mut self.outcomes)
    }

    fn collect_unused(&mut self) {
        self.entries.retain(|key, entry| {
            entry.active
                || entry.revision.is_some()
                || entry.failure.is_some()
                || self.pending.values().any(
                    |pending| matches!(pending, Pending::Write(request) if &request.key == key),
                )
                || self
                    .cleanups
                    .values()
                    .any(|cleanup| &cleanup.request.key == key)
        });
    }
}

fn parse<T: DeserializeOwned>(result: &Result<ResponsePayload, Value>) -> Result<T, MirrorError> {
    match result {
        Ok(ResponsePayload::Json(value)) => serde_json::from_value(value.clone())
            .map_err(|_| MirrorError::Rpc(Failure::MalformedResponse)),
        Ok(ResponsePayload::Binary(_)) => Err(MirrorError::Rpc(Failure::MalformedResponse)),
        Err(error) => Err(MirrorError::Rpc(Failure::Remote(error.clone()))),
    }
}

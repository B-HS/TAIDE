use std::collections::BTreeMap;

use lsp_types::NumberOrString;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

mod budget;
mod capabilities;
pub mod feature;
mod progress;
pub mod protocol;
pub mod registration;
mod selector;
pub mod session;
mod synchronization;

use synchronization::SyncPolicy;

const REQUEST_ID_PREFIX: &str = "taide-native-request:";

pub const MAX_PENDING_REQUESTS: usize = 256;
pub const MAX_OPEN_DOCUMENTS: usize = 256;
pub const MAX_MIRROR_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Detected,
    Initializing,
    Replaying,
    Running,
    Degraded,
    Stopping,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    InvalidPhase,
    StaleGeneration,
    StaleRevision,
    StaleRequest,
    Restarted,
    Cancelled,
    TimedOut,
    ServerError(i64),
    ReinitializeExhausted,
    UnsupportedEncoding,
    UnsupportedCapability,
    MalformedResponse,
    MalformedRequest,
    DocumentNotOpen,
    DuplicateDocument,
    InvalidRevision,
    CounterOverflow,
    Capacity,
    Stopped,
    TransportClosed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentMirror {
    pub uri: String,
    pub language_id: String,
    pub revision: u64,
    pub version: i32,
    pub text: String,
}

#[derive(Clone, PartialEq, Eq)]
struct OpenDocument {
    mirror: DocumentMirror,
    epoch: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Completion {
    pub id: u64,
    pub generation: u64,
    pub result: Result<Value, Failure>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub outgoing: Vec<Value>,
    pub completed: Vec<Completion>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RequestKind {
    Initialize,
    Feature,
    Shutdown,
}

struct Pending {
    deadline: u64,
    kind: RequestKind,
    document: Option<(String, u64, u64)>,
}

pub struct LspCoordinator {
    phase: Phase,
    generation: u64,
    next_id: u64,
    request_timeout_ms: u64,
    pending: BTreeMap<u64, Pending>,
    documents: BTreeMap<String, OpenDocument>,
    replayed_documents: BTreeMap<String, OpenDocument>,
    document_epoch: u64,
    capabilities: Option<Value>,
    client_capabilities: Value,
    registrations: registration::Registry,
    synchronization: SyncPolicy,
    last_failure: Option<Failure>,
}

impl LspCoordinator {
    pub fn new(request_timeout_ms: u64) -> Self {
        Self {
            phase: Phase::Detected,
            generation: 0,
            next_id: 0,
            request_timeout_ms,
            pending: BTreeMap::new(),
            documents: BTreeMap::new(),
            replayed_documents: BTreeMap::new(),
            document_epoch: 0,
            capabilities: None,
            client_capabilities: json!({}),
            registrations: registration::Registry::default(),
            synchronization: SyncPolicy::default(),
            last_failure: None,
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn request_deadline(&self) -> Option<u64> {
        self.pending.values().map(|pending| pending.deadline).min()
    }

    pub fn process_disconnected(&mut self, generation: u64) -> Result<Outcome, Failure> {
        if generation != self.generation {
            return Err(Failure::StaleGeneration);
        }
        if matches!(self.phase, Phase::Detected | Phase::Stopped) {
            return Err(Failure::InvalidPhase);
        }
        let is_stopping = self.phase == Phase::Stopping;
        let failure = if is_stopping {
            Failure::Stopped
        } else {
            Failure::TransportClosed
        };
        let completed = std::mem::take(&mut self.pending)
            .into_keys()
            .map(|id| Completion {
                id,
                generation,
                result: Err(failure.clone()),
            })
            .collect();
        self.capabilities = None;
        self.registrations.clear();
        self.synchronization = SyncPolicy::default();
        self.replayed_documents.clear();
        if !is_stopping {
            let failure = self.last_failure.clone().unwrap_or(failure);
            self.degrade(failure);
        }
        Ok(Outcome {
            outgoing: Vec::new(),
            completed,
        })
    }

    pub fn last_failure(&self) -> Option<&Failure> {
        self.last_failure.as_ref()
    }

    pub fn supports(&self, method: &str) -> bool {
        self.phase == Phase::Running
            && (self
                .capabilities
                .as_ref()
                .is_some_and(|capabilities| capabilities::supports(capabilities, method))
                || self.registrations.supports(method, None, None, true))
    }

    pub fn registration_count(&self) -> usize {
        self.registrations.count()
    }
    pub fn supports_document(&self, method: &str, uri: &str) -> bool {
        let Some(document) = self.documents.get(uri) else {
            return false;
        };
        self.phase == Phase::Running
            && (self
                .capabilities
                .as_ref()
                .is_some_and(|capabilities| capabilities::supports(capabilities, method))
                || self
                    .registrations
                    .supports(method, Some(&document.mirror), None, false))
    }
    pub fn capability_revision(&self) -> u64 {
        self.registrations.revision()
    }

    pub(crate) fn signature_options(&self, uri: &str) -> Vec<lsp_types::SignatureHelpOptions> {
        self.document_options(uri, "signatureHelpProvider", "textDocument/signatureHelp")
    }

    pub(crate) fn completion_options(&self, uri: &str) -> Vec<lsp_types::CompletionOptions> {
        self.document_options(uri, "completionProvider", "textDocument/completion")
    }

    pub(crate) fn on_type_formatting_options(
        &self,
        uri: &str,
    ) -> Vec<lsp_types::DocumentOnTypeFormattingOptions> {
        self.document_options(
            uri,
            "documentOnTypeFormattingProvider",
            "textDocument/onTypeFormatting",
        )
    }

    fn document_options<T: DeserializeOwned>(
        &self,
        uri: &str,
        provider: &str,
        method: &str,
    ) -> Vec<T> {
        if self.phase != Phase::Running {
            return Vec::new();
        }
        let Some(document) = self.documents.get(uri) else {
            return Vec::new();
        };
        let mut options = self
            .capabilities
            .as_ref()
            .and_then(|capabilities| capabilities.get(provider))
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .into_iter()
            .collect::<Vec<_>>();
        options.extend(
            self.registrations
                .document_options(&document.mirror, method, provider),
        );
        options
    }

    pub fn register_capabilities(
        &mut self,
        generation: u64,
        params: lsp_types::RegistrationParams,
    ) -> Result<(), Failure> {
        if generation != self.generation {
            return Err(Failure::StaleGeneration);
        }
        if !matches!(self.phase, Phase::Running | Phase::Replaying) {
            return Err(Failure::InvalidPhase);
        }
        self.registrations
            .register(params, &self.client_capabilities)
    }

    pub fn unregister_capabilities(
        &mut self,
        generation: u64,
        params: lsp_types::UnregistrationParams,
    ) -> Result<(), Failure> {
        if generation != self.generation {
            return Err(Failure::StaleGeneration);
        }
        if !matches!(self.phase, Phase::Running | Phase::Replaying) {
            return Err(Failure::InvalidPhase);
        }
        self.registrations.unregister(params)
    }

    pub fn begin(&mut self, now: u64, params: Value) -> Result<Value, Failure> {
        if self.phase != Phase::Detected {
            return Err(Failure::InvalidPhase);
        }
        let client_capabilities = params
            .get("capabilities")
            .filter(|capabilities| capabilities.is_object())
            .cloned()
            .unwrap_or_else(|| json!({}));
        let message =
            self.create_request(now, "initialize", params, None, RequestKind::Initialize)?;
        self.client_capabilities = client_capabilities;
        self.phase = Phase::Initializing;
        Ok(message)
    }

    pub fn request(
        &mut self,
        now: u64,
        method: &str,
        params: Value,
        document: Option<(&str, u64)>,
    ) -> Result<Value, Failure> {
        if self.phase != Phase::Running
            || matches!(method, "initialize" | "initialized" | "shutdown" | "exit")
        {
            return Err(Failure::InvalidPhase);
        }
        if let Some((uri, revision)) = document {
            let current = self.documents.get(uri).ok_or(Failure::DocumentNotOpen)?;
            if current.mirror.revision != revision {
                return Err(Failure::StaleRevision);
            }
            if params
                .get("textDocument")
                .and_then(|document| document.get("uri"))
                .is_some_and(|value| value.as_str() != Some(uri))
            {
                return Err(Failure::MalformedResponse);
            }
        }
        let is_static = self
            .capabilities
            .as_ref()
            .is_some_and(|capabilities| capabilities::supports(capabilities, method));
        let mirror = document.map(|(uri, _)| &self.documents[uri].mirror);
        if !is_static
            && !self.registrations.supports(
                method,
                mirror,
                params.get("command").and_then(Value::as_str),
                false,
            )
        {
            return Err(Failure::UnsupportedCapability);
        }
        let document =
            document.map(|(uri, revision)| (uri.into(), revision, self.documents[uri].epoch));
        self.create_request(now, method, params, document, RequestKind::Feature)
    }

    pub fn open(&mut self, document: DocumentMirror) -> Result<Option<Value>, Failure> {
        if matches!(self.phase, Phase::Stopping | Phase::Stopped) {
            return Err(Failure::InvalidPhase);
        }
        if self.documents.contains_key(&document.uri) {
            return Err(Failure::DuplicateDocument);
        }
        if document.version < 0 {
            return Err(Failure::InvalidRevision);
        }
        if self.documents.len() >= MAX_OPEN_DOCUMENTS
            || self.mirror_bytes() + document.text.len() > MAX_MIRROR_BYTES
        {
            return Err(Failure::Capacity);
        }
        let epoch = self
            .document_epoch
            .checked_add(1)
            .ok_or(Failure::CounterOverflow)?;
        let message = (self.phase == Phase::Running && self.synchronization.open_close)
            .then(|| did_open(&document));
        self.documents.insert(
            document.uri.clone(),
            OpenDocument {
                mirror: document,
                epoch,
            },
        );
        self.document_epoch = epoch;
        Ok(message)
    }

    pub fn change(
        &mut self,
        uri: &str,
        revision: u64,
        next_revision: u64,
        text: String,
    ) -> Result<Option<Value>, Failure> {
        if matches!(self.phase, Phase::Stopping | Phase::Stopped) {
            return Err(Failure::InvalidPhase);
        }
        let current = self.documents.get(uri).ok_or(Failure::DocumentNotOpen)?;
        if current.mirror.revision != revision {
            return Err(Failure::StaleRevision);
        }
        if revision.checked_add(1) != Some(next_revision) {
            return Err(Failure::InvalidRevision);
        }
        let version = current
            .mirror
            .version
            .checked_add(1)
            .ok_or(Failure::CounterOverflow)?;
        if self.mirror_bytes() - current.mirror.text.len() + text.len() > MAX_MIRROR_BYTES {
            return Err(Failure::Capacity);
        }
        let message = if self.phase == Phase::Running {
            self.synchronization
                .changed(uri, &current.mirror.text, &text, version)
        } else {
            None
        };
        let current = self
            .documents
            .get_mut(uri)
            .ok_or(Failure::DocumentNotOpen)?;
        current.mirror.revision = next_revision;
        current.mirror.version = version;
        current.mirror.text = text;
        Ok(message)
    }

    pub fn close(&mut self, uri: &str) -> Result<Option<Value>, Failure> {
        if matches!(self.phase, Phase::Stopping | Phase::Stopped) {
            return Err(Failure::InvalidPhase);
        }
        self.documents.remove(uri).ok_or(Failure::DocumentNotOpen)?;
        Ok(
            (self.phase == Phase::Running && self.synchronization.open_close).then(|| {
                notification("textDocument/didClose", json!({"textDocument":{"uri":uri}}))
            }),
        )
    }

    pub fn saved(&self, uri: &str) -> Result<Option<Value>, Failure> {
        if self.phase != Phase::Running {
            return Err(Failure::InvalidPhase);
        }
        let document = self.documents.get(uri).ok_or(Failure::DocumentNotOpen)?;
        Ok(self.synchronization.saved(uri, &document.mirror.text))
    }

    pub fn receive(
        &mut self,
        generation: u64,
        now: u64,
        message: Value,
    ) -> Result<Outcome, Failure> {
        if generation != self.generation {
            return Ok(Outcome::default());
        }
        if message.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
            || message.get("method").is_some()
        {
            return Err(Failure::MalformedResponse);
        }
        let id = message
            .get("id")
            .ok_or(Failure::MalformedResponse)
            .and_then(decode_request_id)?;
        let result = match (message.get("result"), message.get("error")) {
            (Some(result), None) => Ok(result.clone()),
            (None, Some(error)) => {
                let code = error
                    .get("code")
                    .and_then(Value::as_i64)
                    .ok_or(Failure::MalformedResponse)?;
                error
                    .get("message")
                    .and_then(Value::as_str)
                    .ok_or(Failure::MalformedResponse)?;
                Err(Failure::ServerError(code))
            }
            _ => return Err(Failure::MalformedResponse),
        };
        let mut outcome = self.expire(now)?;
        let Some(id) = id else {
            return Ok(outcome);
        };
        let Some(pending) = self.pending.remove(&id) else {
            return Ok(outcome);
        };
        if pending.kind == RequestKind::Shutdown {
            let result = match result {
                Ok(Value::Null) => Ok(Value::Null),
                Ok(_) => Err(Failure::MalformedResponse),
                Err(error) => Err(error),
            };
            if let Err(error) = &result {
                self.last_failure = Some(error.clone());
            }
            outcome
                .outgoing
                .push(json!({"jsonrpc":"2.0","method":"exit"}));
            outcome.completed.push(Completion {
                id,
                generation,
                result,
            });
            return Ok(outcome);
        }
        if pending.kind == RequestKind::Initialize {
            if let Err(error) = &result {
                self.degrade(error.clone());
            } else {
                let capabilities = result
                    .as_ref()
                    .ok()
                    .and_then(|value| value.get("capabilities"))
                    .filter(|value| value.is_object());
                let Some(capabilities) = capabilities else {
                    self.degrade(Failure::MalformedResponse);
                    return Err(Failure::MalformedResponse);
                };
                if capabilities
                    .get("positionEncoding")
                    .is_some_and(|value| value.as_str() != Some("utf-16"))
                {
                    self.degrade(Failure::UnsupportedEncoding);
                    return Err(Failure::UnsupportedEncoding);
                }
                let synchronization = match SyncPolicy::parse(capabilities.get("textDocumentSync"))
                {
                    Ok(synchronization) => synchronization,
                    Err(error) => {
                        self.degrade(error.clone());
                        return Err(error);
                    }
                };
                self.synchronization = synchronization;
                self.capabilities = Some(capabilities.clone());
                self.phase = Phase::Replaying;
                outcome
                    .outgoing
                    .push(notification("initialized", json!({})));
                if self.synchronization.open_close {
                    outcome.outgoing.extend(
                        self.documents
                            .values()
                            .map(|document| did_open(&document.mirror)),
                    );
                }
                self.replayed_documents = self.documents.clone();
            }
        }
        let is_stale = pending
            .document
            .as_ref()
            .is_some_and(|(uri, revision, epoch)| {
                self.documents.get(uri).is_none_or(|document| {
                    document.mirror.revision != *revision || document.epoch != *epoch
                })
            });
        outcome.completed.push(Completion {
            id,
            generation,
            result: if is_stale {
                Err(Failure::StaleRevision)
            } else {
                result
            },
        });
        Ok(outcome)
    }

    pub fn finish_replay(&mut self, generation: u64) -> Result<Vec<Value>, Failure> {
        if generation != self.generation {
            return Err(Failure::StaleGeneration);
        }
        if self.phase != Phase::Replaying {
            return Err(Failure::InvalidPhase);
        }
        let mut outgoing = Vec::new();
        for (uri, previous) in &self.replayed_documents {
            if self
                .documents
                .get(uri)
                .is_none_or(|current| current.epoch != previous.epoch)
                && self.synchronization.open_close
            {
                outgoing.push(notification(
                    "textDocument/didClose",
                    json!({"textDocument":{"uri":uri}}),
                ));
            }
        }
        for (uri, current) in &self.documents {
            match self.replayed_documents.get(uri) {
                Some(previous) if previous.epoch == current.epoch => {
                    if previous != current {
                        if let Some(message) = self.synchronization.changed(
                            uri,
                            &previous.mirror.text,
                            &current.mirror.text,
                            current.mirror.version,
                        ) {
                            outgoing.push(message);
                        }
                    }
                }
                _ if self.synchronization.open_close => outgoing.push(did_open(&current.mirror)),
                _ => {}
            }
        }
        if outgoing.is_empty() {
            self.replayed_documents.clear();
            self.phase = Phase::Running;
            return Ok(outgoing);
        }
        self.replayed_documents = self.documents.clone();
        Ok(outgoing)
    }

    pub fn restart(&mut self, now: u64, params: Value) -> Result<Outcome, Failure> {
        if matches!(
            self.phase,
            Phase::Detected | Phase::Stopping | Phase::Stopped
        ) {
            return Err(Failure::InvalidPhase);
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(Failure::CounterOverflow)?;
        self.next_id
            .checked_add(1)
            .ok_or(Failure::CounterOverflow)?;
        now.checked_add(self.request_timeout_ms)
            .ok_or(Failure::CounterOverflow)?;
        let completed = std::mem::take(&mut self.pending)
            .into_keys()
            .map(|id| Completion {
                id,
                generation: self.generation,
                result: Err(Failure::Restarted),
            })
            .collect();
        self.generation = generation;
        self.phase = Phase::Detected;
        self.capabilities = None;
        self.registrations.clear();
        self.synchronization = SyncPolicy::default();
        self.replayed_documents.clear();
        self.last_failure = None;
        let initialize = self.begin(now, params)?;
        Ok(Outcome {
            outgoing: vec![initialize],
            completed,
        })
    }

    fn retry_initialize(&mut self, now: u64, params: Value) -> Result<Value, Failure> {
        if self.generation == 0 || self.phase != Phase::Degraded || !self.pending.is_empty() {
            return Err(Failure::InvalidPhase);
        }
        self.phase = Phase::Detected;
        let message = match self.begin(now, params) {
            Ok(message) => message,
            Err(error) => {
                self.phase = Phase::Degraded;
                return Err(error);
            }
        };
        self.last_failure = None;
        Ok(message)
    }

    pub fn cancel(&mut self, id: u64) -> Result<Outcome, Failure> {
        let pending = self.pending.get(&id).ok_or(Failure::InvalidPhase)?;
        if pending.kind != RequestKind::Feature {
            return Err(Failure::InvalidPhase);
        }
        self.pending.remove(&id);
        Ok(Outcome {
            outgoing: vec![notification(
                "$/cancelRequest",
                json!({"id":wire_request_id(id)}),
            )],
            completed: vec![Completion {
                id,
                generation: self.generation,
                result: Err(Failure::Cancelled),
            }],
        })
    }

    pub fn stop(&mut self, now: u64) -> Result<Outcome, Failure> {
        if matches!(
            self.phase,
            Phase::Detected | Phase::Stopping | Phase::Stopped
        ) {
            return Err(Failure::InvalidPhase);
        }
        let should_shutdown = self.phase == Phase::Running;
        if should_shutdown {
            self.next_id
                .checked_add(1)
                .ok_or(Failure::CounterOverflow)?;
            now.checked_add(self.request_timeout_ms)
                .ok_or(Failure::CounterOverflow)?;
        }
        let mut outcome = Outcome::default();
        for (id, pending) in std::mem::take(&mut self.pending) {
            if should_shutdown && pending.kind == RequestKind::Feature {
                outcome.outgoing.push(notification(
                    "$/cancelRequest",
                    json!({"id":wire_request_id(id)}),
                ));
            }
            outcome.completed.push(Completion {
                id,
                generation: self.generation,
                result: Err(Failure::Stopped),
            });
        }
        if should_shutdown {
            if self.synchronization.open_close {
                outcome.outgoing.extend(self.documents.keys().map(|uri| {
                    notification("textDocument/didClose", json!({"textDocument":{"uri":uri}}))
                }));
            }
            let mut shutdown =
                self.create_request(now, "shutdown", Value::Null, None, RequestKind::Shutdown)?;
            shutdown
                .as_object_mut()
                .ok_or(Failure::MalformedResponse)?
                .remove("params");
            outcome.outgoing.push(shutdown);
        } else {
            outcome
                .outgoing
                .push(json!({"jsonrpc":"2.0","method":"exit"}));
        }
        self.phase = Phase::Stopping;
        self.capabilities = None;
        self.synchronization = SyncPolicy::default();
        Ok(outcome)
    }

    pub fn finish_stop(&mut self, generation: u64) -> Result<(), Failure> {
        if generation != self.generation {
            return Err(Failure::StaleGeneration);
        }
        if self.phase != Phase::Stopping || !self.pending.is_empty() {
            return Err(Failure::InvalidPhase);
        }
        self.documents.clear();
        self.replayed_documents.clear();
        self.phase = Phase::Stopped;
        Ok(())
    }

    pub fn expire(&mut self, now: u64) -> Result<Outcome, Failure> {
        let ids = self
            .pending
            .iter()
            .filter_map(|(id, pending)| (pending.deadline <= now).then_some(*id))
            .collect::<Vec<_>>();
        let mut outcome = Outcome::default();
        for id in ids {
            let pending = self.pending.remove(&id).ok_or(Failure::InvalidPhase)?;
            match pending.kind {
                RequestKind::Initialize => self.degrade(Failure::TimedOut),
                RequestKind::Feature => {
                    outcome.outgoing.push(notification(
                        "$/cancelRequest",
                        json!({"id":wire_request_id(id)}),
                    ));
                }
                RequestKind::Shutdown => {
                    self.last_failure = Some(Failure::TimedOut);
                    outcome
                        .outgoing
                        .push(json!({"jsonrpc":"2.0","method":"exit"}));
                }
            }
            outcome.completed.push(Completion {
                id,
                generation: self.generation,
                result: Err(Failure::TimedOut),
            });
        }
        Ok(outcome)
    }

    fn create_request(
        &mut self,
        now: u64,
        method: &str,
        params: Value,
        document: Option<(String, u64, u64)>,
        kind: RequestKind,
    ) -> Result<Value, Failure> {
        if self.pending.len() >= MAX_PENDING_REQUESTS {
            return Err(Failure::Capacity);
        }
        let id = self
            .next_id
            .checked_add(1)
            .ok_or(Failure::CounterOverflow)?;
        let deadline = now
            .checked_add(self.request_timeout_ms)
            .ok_or(Failure::CounterOverflow)?;
        self.next_id = id;
        self.pending.insert(
            id,
            Pending {
                deadline,
                kind,
                document,
            },
        );
        Ok(json!({"jsonrpc":"2.0", "id":wire_request_id(id), "method":method, "params":params}))
    }

    fn mirror_bytes(&self) -> usize {
        self.documents
            .values()
            .map(|document| document.mirror.text.len())
            .sum()
    }

    fn degrade(&mut self, failure: Failure) {
        self.phase = Phase::Degraded;
        self.capabilities = None;
        self.synchronization = SyncPolicy::default();
        self.last_failure = Some(failure);
    }
}

fn wire_request_id(id: u64) -> NumberOrString {
    match i32::try_from(id) {
        Ok(id) => NumberOrString::Number(id),
        Err(_) => NumberOrString::String(format!("{REQUEST_ID_PREFIX}{id}")),
    }
}

pub(super) fn decode_request_id(value: &Value) -> Result<Option<u64>, Failure> {
    let wire = serde_json::from_value::<NumberOrString>(value.clone())
        .map_err(|_| Failure::MalformedResponse)?;
    let id = match &wire {
        NumberOrString::Number(id) => u64::try_from(*id).ok(),
        NumberOrString::String(id) => id
            .strip_prefix(REQUEST_ID_PREFIX)
            .and_then(|id| id.parse::<u64>().ok()),
    };
    Ok(id.filter(|id| wire_request_id(*id) == wire))
}

fn notification(method: &str, params: Value) -> Value {
    json!({"jsonrpc":"2.0", "method":method, "params":params})
}

fn did_open(document: &DocumentMirror) -> Value {
    notification(
        "textDocument/didOpen",
        json!({"textDocument": {
            "uri":document.uri, "languageId":document.language_id, "version":document.version, "text":document.text
        }}),
    )
}

#[cfg(test)]
mod replay_ownership_tests {
    use super::*;

    const REQUEST_TIMEOUT_MS: u64 = 500;
    const DOCUMENT_URI: &str = "file:///synthetic/replay-ownership.rs";

    #[test]
    fn reinitialize_retry는_latest_replay_deadline_지연응답과_stop을_보존한다() {
        let params = json!({"capabilities":{}});
        let mut coordinator = LspCoordinator::new(REQUEST_TIMEOUT_MS);
        let initial = coordinator.begin(0, params.clone()).unwrap();
        coordinator.expire(REQUEST_TIMEOUT_MS).unwrap();
        assert_eq!(
            coordinator.retry_initialize(REQUEST_TIMEOUT_MS, params.clone()),
            Err(Failure::InvalidPhase)
        );
        let first = coordinator
            .restart(REQUEST_TIMEOUT_MS, params.clone())
            .unwrap()
            .outgoing
            .remove(0);
        let generation = coordinator.generation();
        coordinator
            .open(DocumentMirror {
                uri: DOCUMENT_URI.into(),
                language_id: "rust".into(),
                revision: 0,
                version: 0,
                text: "original".into(),
            })
            .unwrap();
        let expired_at = REQUEST_TIMEOUT_MS * 2;
        coordinator.expire(expired_at).unwrap();
        assert_eq!(coordinator.phase(), Phase::Degraded);
        assert_eq!(
            coordinator.retry_initialize(u64::MAX, params.clone()),
            Err(Failure::CounterOverflow)
        );
        assert_eq!(coordinator.phase(), Phase::Degraded);
        assert_eq!(coordinator.pending_count(), 0);
        coordinator
            .change(DOCUMENT_URI, 0, 1, "latest".into())
            .unwrap();
        let retry = coordinator
            .retry_initialize(expired_at, params.clone())
            .unwrap();
        assert_ne!(retry["id"], first["id"]);
        assert_eq!(coordinator.generation(), generation);
        assert_eq!(
            coordinator.request_deadline(),
            Some(expired_at + REQUEST_TIMEOUT_MS)
        );
        let response = |id: Value| json!({"jsonrpc":"2.0","id":id,"result":{"capabilities":{"textDocumentSync":1}}});
        assert_eq!(
            coordinator
                .receive(generation, expired_at, response(first["id"].clone()))
                .unwrap(),
            Outcome::default()
        );
        assert_eq!(
            coordinator
                .receive(0, expired_at, response(initial["id"].clone()))
                .unwrap(),
            Outcome::default()
        );
        assert_eq!(coordinator.phase(), Phase::Initializing);
        let replay = coordinator
            .receive(generation, expired_at, response(retry["id"].clone()))
            .unwrap();
        assert_eq!(replay.outgoing[0]["method"], "initialized");
        assert_eq!(
            replay.outgoing[1]["params"]["textDocument"]["text"],
            "latest"
        );
        assert_eq!(replay.outgoing[1]["params"]["textDocument"]["version"], 1);
        coordinator.finish_replay(generation).unwrap();
        assert_eq!(coordinator.phase(), Phase::Running);
        coordinator.restart(expired_at, params.clone()).unwrap();
        coordinator.expire(expired_at + REQUEST_TIMEOUT_MS).unwrap();
        coordinator.stop(expired_at + REQUEST_TIMEOUT_MS).unwrap();
        assert_eq!(
            coordinator.retry_initialize(expired_at, params),
            Err(Failure::InvalidPhase)
        );
        coordinator.finish_stop(coordinator.generation()).unwrap();
        assert!(coordinator.documents.is_empty());
    }

    #[test]
    fn replay_snapshot은_delta_write_동안_보유하고_running_뒤_폐기한다() {
        let mut coordinator = LspCoordinator::new(REQUEST_TIMEOUT_MS);
        coordinator
            .open(DocumentMirror {
                uri: DOCUMENT_URI.into(),
                language_id: "rust".into(),
                revision: 0,
                version: 0,
                text: "original".into(),
            })
            .unwrap();
        let initialize = coordinator.begin(0, json!({"capabilities":{}})).unwrap();
        coordinator.receive(0, 1, json!({"jsonrpc":"2.0","id":initialize["id"],"result":{"capabilities":{"textDocumentSync":1}}})).unwrap();
        assert_eq!(coordinator.phase(), Phase::Replaying);
        assert_eq!(coordinator.replayed_documents.len(), 1);
        coordinator
            .change(DOCUMENT_URI, 0, 1, "newest".into())
            .unwrap();
        let delta = coordinator.finish_replay(0).unwrap();
        assert_eq!(delta.len(), 1);
        assert_eq!(delta[0]["method"], "textDocument/didChange");
        assert_eq!(delta[0]["params"]["contentChanges"][0]["text"], "newest");
        assert_eq!(coordinator.phase(), Phase::Replaying);
        assert_eq!(coordinator.replayed_documents.len(), 1);
        assert_eq!(
            coordinator.replayed_documents[DOCUMENT_URI].mirror.revision,
            1
        );
        assert_eq!(coordinator.finish_replay(1), Err(Failure::StaleGeneration));
        assert_eq!(coordinator.replayed_documents.len(), 1);
        assert!(coordinator.finish_replay(0).unwrap().is_empty());
        assert_eq!(coordinator.phase(), Phase::Running);
        assert!(coordinator.replayed_documents.is_empty());
        assert_eq!(coordinator.documents[DOCUMENT_URI].mirror.text, "newest");
        let restarted = coordinator.restart(1, json!({"capabilities":{}})).unwrap();
        let replay = coordinator.receive(1, 2, json!({"jsonrpc":"2.0","id":restarted.outgoing[0]["id"],"result":{"capabilities":{"textDocumentSync":1}}})).unwrap();
        assert_eq!(
            replay.outgoing[1]["params"]["textDocument"]["text"],
            "newest"
        );
        assert_eq!(coordinator.replayed_documents.len(), 1);
        assert!(coordinator.finish_replay(1).unwrap().is_empty());
        assert!(coordinator.replayed_documents.is_empty());
        coordinator.close(DOCUMENT_URI).unwrap();
        assert!(coordinator.documents.is_empty());
        assert!(coordinator.replayed_documents.is_empty());
    }
}

#[cfg(test)]
mod request_id_tests {
    use super::*;

    const REQUEST_TIMEOUT_MS: u64 = 500;
    const DOCUMENT_URI: &str = "file:///synthetic/request-id.rs";

    #[test]
    fn wire_id는_integer_경계를_넘어도_response_cancel_timeout_restart_shutdown을_유지한다() {
        let mut coordinator = LspCoordinator::new(REQUEST_TIMEOUT_MS);
        let boundary = u64::try_from(i32::MAX).unwrap();
        coordinator.next_id = boundary - 1;
        let initialize_params = json!({"capabilities":{}});
        let initialize = coordinator.begin(0, initialize_params.clone()).unwrap();
        assert_eq!(initialize["id"], json!(i32::MAX));
        let initialize_reply = |id: Value| json!({"jsonrpc":"2.0","id":id,"result":{"capabilities":{"hoverProvider":true}}});
        coordinator
            .receive(0, 1, initialize_reply(initialize["id"].clone()))
            .unwrap();
        coordinator.finish_replay(0).unwrap();
        coordinator
            .open(DocumentMirror {
                uri: DOCUMENT_URI.into(),
                language_id: "rust".into(),
                revision: 0,
                version: 0,
                text: "합성".into(),
            })
            .unwrap();
        let params =
            json!({"textDocument":{"uri":DOCUMENT_URI},"position":{"line":0,"character":0}});
        let request = coordinator
            .request(
                1,
                "textDocument/hover",
                params.clone(),
                Some((DOCUMENT_URI, 0)),
            )
            .unwrap();
        let internal_id = boundary + 1;
        assert_eq!(
            request["id"],
            json!(format!("taide-native-request:{internal_id}"))
        );
        let response = |id: Value| json!({"jsonrpc":"2.0","id":id,"result":null});
        assert!(coordinator
            .receive(0, 1, response(json!(internal_id)))
            .is_err());
        assert!(coordinator
            .receive(0, 1, response(json!(i32::MIN)))
            .unwrap()
            .completed
            .is_empty());
        assert!(coordinator
            .receive(0, 1, response(json!("1")))
            .unwrap()
            .completed
            .is_empty());
        assert!(coordinator
            .receive(
                0,
                1,
                response(json!(format!("taide-native-request:0{internal_id}")))
            )
            .unwrap()
            .completed
            .is_empty());
        assert_eq!(coordinator.pending_count(), 1);
        let cancelled = coordinator.cancel(internal_id).unwrap();
        assert_eq!(cancelled.outgoing[0]["params"]["id"], request["id"]);
        assert_eq!(cancelled.completed[0].id, internal_id);
        assert!(coordinator
            .receive(0, 1, response(request["id"].clone()))
            .unwrap()
            .completed
            .is_empty());
        let timed = coordinator
            .request(
                1,
                "textDocument/hover",
                params.clone(),
                Some((DOCUMENT_URI, 0)),
            )
            .unwrap();
        let expired = coordinator.expire(1 + REQUEST_TIMEOUT_MS).unwrap();
        assert_eq!(expired.outgoing[0]["params"]["id"], timed["id"]);
        assert_eq!(expired.completed[0].result, Err(Failure::TimedOut));
        let restarted = coordinator
            .restart(1 + REQUEST_TIMEOUT_MS, initialize_params)
            .unwrap();
        let new_initialize_id = restarted.outgoing[0]["id"].clone();
        assert!(new_initialize_id.is_string());
        assert!(coordinator
            .receive(
                0,
                1 + REQUEST_TIMEOUT_MS,
                initialize_reply(new_initialize_id.clone())
            )
            .unwrap()
            .completed
            .is_empty());
        coordinator
            .receive(
                1,
                1 + REQUEST_TIMEOUT_MS,
                initialize_reply(new_initialize_id),
            )
            .unwrap();
        coordinator.finish_replay(1).unwrap();
        let active = coordinator
            .request(
                1 + REQUEST_TIMEOUT_MS,
                "textDocument/hover",
                params,
                Some((DOCUMENT_URI, 0)),
            )
            .unwrap();
        let stopping = coordinator.stop(1 + REQUEST_TIMEOUT_MS).unwrap();
        assert_eq!(stopping.outgoing[0]["params"]["id"], active["id"]);
        let shutdown = stopping.outgoing.last().unwrap();
        assert_eq!(shutdown["method"], "shutdown");
        assert!(shutdown["id"].is_string());
        let stopped = coordinator
            .receive(1, 1 + REQUEST_TIMEOUT_MS, response(shutdown["id"].clone()))
            .unwrap();
        assert_eq!(stopped.outgoing[0]["method"], "exit");
        coordinator.finish_stop(1).unwrap();
        assert_eq!(coordinator.phase(), Phase::Stopped);
        assert_eq!(coordinator.pending_count(), 0);
    }
}

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use eframe::egui::Context;
use taide_model::file::{FileSizeTier, LARGE_FILE_BYTES};
use taide_native_editor::diagnostics::Marker;
use taide_native_editor::document::{DocumentId, DocumentKey, DocumentSnapshot, EditorError};
use taide_native_editor::json_parser;
use taide_native_editor::json_scanner;
use taide_native_editor::json_schema_associations;
use taide_native_editor::json_schema_document::{self, Contribution};
use taide_native_editor::json_schema_evaluation;
use taide_native_editor::json_schema_graph;
use taide_native_editor::json_schema_load;
use taide_native_editor::json_schema_policy::{self, Policy};
use taide_native_editor::json_schema_refs;
use taide_native_editor::json_schema_scalar;
use taide_native_editor::json_schema_service;
use taide_native_editor::json_schema_uri;
use taide_native_editor::json_validation::{self, Level, Settings};
use taide_native_editor::json_value;
use taide_native_editor::store::EditorStore;
use taide_native_syntax::{JsonSchemaPatternCompiler, JsonSchemaValidationPatternCompiler};
use taide_runtime::TaskSupervisor;
use tokio::sync::oneshot;

const WORKER_NAME: &str = "native-json-validation";
const MAX_ACTIVE: usize = 1;
const DEBOUNCE: Duration = Duration::from_millis(500);
const MAX_NODES: usize = 256 * 1024;
const MAX_TOKENS: usize = 1024 * 1024;
const MAX_DEPTH: usize = 8192;
const MAX_DIAGNOSTICS: usize = 10 * 1024;
const MAX_MESSAGE_BYTES: usize = LARGE_FILE_BYTES as usize;
const MAX_VALUES: usize = 256 * 1024;
const MAX_TEXT_UNITS: usize = LARGE_FILE_BYTES as usize;
const MAX_WORK: usize = 64 * 1024 * 1024;
const MAX_MESSAGE_UNITS: usize = 4 * 1024 * 1024;
const MAX_URI_UNITS: usize = 64 * 1024;
const MAX_SCHEMA_HANDLES: usize = 256;
const MAX_SCHEMA_DOCUMENTS: usize = 64;
const MAX_SCHEMA_STEPS: usize = 256;
const MAX_FRAMES: usize = 16 * 1024;
const MAX_MATCHES: usize = 1024 * 1024;
const PARSER_LIMITS: json_parser::Limits = json_parser::Limits {
    scanner: json_scanner::Limits {
        max_bytes: LARGE_FILE_BYTES as usize,
        max_tokens: MAX_TOKENS,
        max_value_units: LARGE_FILE_BYTES as usize,
    },
    max_nodes: MAX_NODES,
    max_depth: MAX_DEPTH,
    max_diagnostics: MAX_DIAGNOSTICS,
    max_comments: MAX_DIAGNOSTICS,
};
const LIMITS: json_validation::Limits = json_validation::Limits {
    parser: PARSER_LIMITS,
    max_diagnostics: MAX_DIAGNOSTICS,
    max_message_bytes: MAX_MESSAGE_BYTES,
};
const VALUE_LIMITS: json_value::Limits = json_value::Limits {
    parser: PARSER_LIMITS,
    max_values: MAX_VALUES,
    max_text_units: MAX_TEXT_UNITS,
};
const URI_LIMITS: json_schema_uri::Limits = json_schema_uri::Limits {
    max_input_units: MAX_URI_UNITS,
    max_output_units: 4 * MAX_URI_UNITS,
    max_work: MAX_WORK,
};
const REF_LIMITS: json_schema_refs::Limits = json_schema_refs::Limits {
    values: VALUE_LIMITS,
    max_work: MAX_WORK,
    max_merges: MAX_VALUES,
    max_text_units: MAX_TEXT_UNITS,
    max_messages: MAX_DIAGNOSTICS,
    max_message_units: MAX_MESSAGE_UNITS,
};
const SCHEMA_LIMITS: json_schema_document::Limits = json_schema_document::Limits {
    service: json_schema_service::Limits {
        associations: json_schema_associations::Limits {
            uri: URI_LIMITS,
            max_patterns: MAX_SCHEMA_HANDLES,
            max_pattern_units: MAX_URI_UNITS,
            max_regex_units: 4 * MAX_URI_UNITS,
            max_uris: MAX_SCHEMA_HANDLES,
            max_uri_units: 4 * MAX_URI_UNITS,
            max_work: MAX_WORK,
        },
        max_handles: MAX_SCHEMA_HANDLES,
        max_registered: MAX_SCHEMA_HANDLES,
        max_associations: MAX_SCHEMA_HANDLES,
        max_values: MAX_VALUES,
        max_text_units: MAX_TEXT_UNITS,
        max_work: MAX_WORK,
    },
    graph: json_schema_graph::Limits {
        refs: REF_LIMITS,
        uri: URI_LIMITS,
        max_documents: MAX_SCHEMA_DOCUMENTS,
        max_requests: MAX_SCHEMA_HANDLES,
        max_uri_units: 4 * MAX_URI_UNITS,
    },
    load: json_schema_load::Limits {
        scanner: PARSER_LIMITS.scanner,
        uri: URI_LIMITS,
        max_values: MAX_VALUES,
        max_depth: MAX_DEPTH,
        max_text_units: MAX_TEXT_UNITS,
        max_work: MAX_WORK,
        max_messages: MAX_DIAGNOSTICS,
        max_message_units: MAX_MESSAGE_UNITS,
    },
    policy: json_schema_policy::Limits {
        syntax: LIMITS,
        evaluation: json_schema_evaluation::Limits {
            values: VALUE_LIMITS,
            scalar: json_schema_scalar::Limits {
                max_work: MAX_WORK,
                max_diagnostics: MAX_DIAGNOSTICS,
                max_message_units: MAX_MESSAGE_UNITS,
                max_enum_values: MAX_VALUES,
                max_depth: MAX_DEPTH,
            },
            max_work: MAX_WORK,
            max_frames: MAX_FRAMES,
            max_diagnostics: MAX_DIAGNOSTICS,
            max_message_units: MAX_MESSAGE_UNITS,
            max_matches: MAX_MATCHES,
            max_processed: MAX_MATCHES,
        },
        max_schema_work: MAX_WORK,
    },
    contributions: VALUE_LIMITS,
    max_steps: MAX_SCHEMA_STEPS,
};
const POLICY: Policy = Policy {
    schema_validation: Some(Level::Warning),
    schema_request: Some(Level::Warning),
    schema_draft: None,
};
const SETTINGS: Settings = Settings {
    validate: true,
    allow_comments: true,
    comments: Some(Level::Error),
    trailing_commas: Some(Level::Error),
    schema_allows_comments: false,
    schema_allows_trailing_commas: false,
};

struct Entry {
    snapshot: DocumentSnapshot,
    cancelled: Arc<AtomicBool>,
    receiver: Option<oneshot::Receiver<Result<Vec<Marker>, EditorError>>>,
    complete: bool,
    ready_at: Instant,
}

impl Drop for Entry {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

struct Slot(Arc<AtomicUsize>, Context);
impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
        self.1.request_repaint();
    }
}

#[derive(Default)]
pub(crate) struct State {
    entries: HashMap<DocumentId, Entry>,
    active: Arc<AtomicUsize>,
}

impl State {
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    pub(crate) fn documents(&self) -> HashSet<DocumentId> {
        self.entries.keys().copied().collect()
    }

    pub(crate) fn tick(
        &mut self,
        store: &EditorStore,
        tasks: &TaskSupervisor,
        context: &Context,
        now: Instant,
    ) -> Vec<(DocumentSnapshot, Result<Vec<Marker>, EditorError>)> {
        let snapshots = store
            .documents()
            .snapshots()
            .filter(|snapshot| {
                snapshot.metadata.language_id == "json"
                    && snapshot.metadata.tier == FileSizeTier::Normal
                    && !matches!(snapshot.key, DocumentKey::AppFile(_))
                    && store.views().for_document(snapshot.id).next().is_some()
            })
            .collect::<Vec<_>>();
        let documents = snapshots
            .iter()
            .map(|snapshot| snapshot.id)
            .collect::<HashSet<_>>();
        self.entries.retain(|id, _| documents.contains(id));
        for snapshot in snapshots {
            let unchanged = self.entries.get(&snapshot.id).is_some_and(|entry| {
                entry.snapshot.key == snapshot.key
                    && entry.snapshot.revision == snapshot.revision
                    && entry.snapshot.metadata.language_id == snapshot.metadata.language_id
                    && entry.snapshot.metadata.tier == snapshot.metadata.tier
            });
            if !unchanged {
                let ready_at = self.entries.get(&snapshot.id).map_or(now, |entry| {
                    if entry.snapshot.key == snapshot.key
                        && entry.snapshot.metadata.language_id == snapshot.metadata.language_id
                    {
                        return now + DEBOUNCE;
                    }
                    now
                });
                self.entries.insert(
                    snapshot.id,
                    Entry {
                        snapshot,
                        cancelled: Arc::new(AtomicBool::new(false)),
                        receiver: None,
                        complete: false,
                        ready_at,
                    },
                );
            }
        }
        let mut completed = Vec::new();
        for entry in self.entries.values_mut() {
            let Some(receiver) = &mut entry.receiver else {
                continue;
            };
            let result = match receiver.try_recv() {
                Ok(result) => result,
                Err(oneshot::error::TryRecvError::Empty) => continue,
                Err(oneshot::error::TryRecvError::Closed) => Err(EditorError::StaleRevision),
            };
            entry.receiver.take();
            entry.complete = true;
            completed.push((entry.snapshot.clone(), result));
        }
        for entry in self
            .entries
            .values_mut()
            .filter(|entry| !entry.complete && entry.receiver.is_none())
        {
            if now < entry.ready_at {
                context.request_repaint_after(entry.ready_at.saturating_duration_since(now));
                continue;
            }
            if self.active.load(Ordering::Acquire) >= MAX_ACTIVE {
                break;
            }
            self.active.fetch_add(1, Ordering::AcqRel);
            let slot = Slot(self.active.clone(), context.clone());
            let (sender, receiver) = oneshot::channel();
            let snapshot = entry.snapshot.clone();
            let cancelled = entry.cancelled.clone();
            let repaint = context.clone();
            let started = tasks
                .spawn_blocking_transient_handle(WORKER_NAME, move || {
                    let _slot = slot;
                    if cancelled.load(Ordering::Acquire) {
                        return;
                    }
                    let result = build(&snapshot, &cancelled);
                    if !cancelled.load(Ordering::Acquire) && !sender.is_closed() {
                        drop(sender.send(result));
                        repaint.request_repaint();
                    }
                })
                .is_some();
            if started {
                entry.receiver = Some(receiver);
            } else {
                entry.complete = true;
                completed.push((entry.snapshot.clone(), Err(EditorError::StaleRevision)));
            }
        }
        completed
    }
}

fn contributions() -> Result<&'static [Contribution], EditorError> {
    static CONTRIBUTIONS: OnceLock<Result<Vec<Contribution>, EditorError>> = OnceLock::new();
    CONTRIBUTIONS
        .get_or_init(|| json_schema_document::contributions(VALUE_LIMITS))
        .as_deref()
        .map_err(Clone::clone)
}

fn resource_uri(key: &DocumentKey) -> Result<String, EditorError> {
    match key {
        DocumentKey::File(path) => Ok(url::Url::from_file_path(path)
            .map_err(|_| EditorError::InvalidIdentity)?
            .to_string()),
        DocumentKey::Untitled(tab) => Ok(format!("untitled:{tab}")),
        DocumentKey::AppFile(_) => Err(EditorError::InvalidIdentity),
    }
}

fn build(snapshot: &DocumentSnapshot, cancelled: &AtomicBool) -> Result<Vec<Marker>, EditorError> {
    if snapshot.rope.len_bytes() > LIMITS.parser.scanner.max_bytes {
        return Err(EditorError::Capacity);
    }
    if cancelled.load(Ordering::Acquire) {
        return Err(EditorError::StaleRevision);
    }
    let markers = json_schema_document::diagnostics(
        &snapshot.rope.to_string(),
        &resource_uri(&snapshot.key)?,
        &snapshot.metadata.language_id,
        SETTINGS,
        POLICY,
        contributions()?,
        &JsonSchemaPatternCompiler,
        &JsonSchemaValidationPatternCompiler,
        SCHEMA_LIMITS,
        &|| cancelled.load(Ordering::Acquire),
    )?;
    if cancelled.load(Ordering::Acquire) {
        return Err(EditorError::StaleRevision);
    }
    Ok(markers)
}

#[cfg(test)]
#[path = "editor-json-validation-tests.rs"]
mod tests;

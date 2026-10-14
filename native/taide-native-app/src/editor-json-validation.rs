use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use eframe::egui::Context;
use taide_model::file::{FileSizeTier, LARGE_FILE_BYTES};
use taide_native_editor::diagnostics::Marker;
use taide_native_editor::document::{DocumentId, DocumentKey, DocumentSnapshot, EditorError};
use taide_native_editor::json_parser;
use taide_native_editor::json_scanner;
use taide_native_editor::json_validation::{self, Level, Settings};
use taide_native_editor::store::EditorStore;
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
const LIMITS: json_validation::Limits = json_validation::Limits {
    parser: json_parser::Limits {
        scanner: json_scanner::Limits {
            max_bytes: LARGE_FILE_BYTES as usize,
            max_tokens: MAX_TOKENS,
            max_value_units: LARGE_FILE_BYTES as usize,
        },
        max_nodes: MAX_NODES,
        max_depth: MAX_DEPTH,
        max_diagnostics: MAX_DIAGNOSTICS,
        max_comments: MAX_DIAGNOSTICS,
    },
    max_diagnostics: MAX_DIAGNOSTICS,
    max_message_bytes: MAX_MESSAGE_BYTES,
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

fn build(snapshot: &DocumentSnapshot, cancelled: &AtomicBool) -> Result<Vec<Marker>, EditorError> {
    if snapshot.rope.len_bytes() > LIMITS.parser.scanner.max_bytes {
        return Err(EditorError::Capacity);
    }
    if cancelled.load(Ordering::Acquire) {
        return Err(EditorError::StaleRevision);
    }
    let markers = json_validation::syntax_diagnostics(
        &snapshot.rope.to_string(),
        &snapshot.metadata.language_id,
        SETTINGS,
        LIMITS,
    )?;
    if cancelled.load(Ordering::Acquire) {
        return Err(EditorError::StaleRevision);
    }
    Ok(markers)
}

#[cfg(test)]
#[path = "editor-json-validation-tests.rs"]
mod tests;

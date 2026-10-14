use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use eframe::egui::Context;
use taide_model::file::{FileSizeTier, LARGE_FILE_BYTES};
use taide_native_editor::document::{DocumentId, DocumentKey, DocumentSnapshot, EditorError};
use taide_native_editor::document_symbols::{DocumentSymbols, Symbol};
use taide_native_editor::json_folding;
use taide_native_editor::json_parser;
use taide_native_editor::json_scanner;
use taide_native_editor::json_symbols;
use taide_native_editor::store::EditorStore;
use taide_native_editor::syntax_folding::SyntaxFolds;
use taide_runtime::TaskSupervisor;
use tokio::sync::oneshot;

const WORKER_NAME: &str = "native-json-structure";
const MAX_ACTIVE: usize = 2;
const MAX_NODES: usize = 256 * 1024;
const MAX_TOKENS: usize = 1024 * 1024;
const MAX_DEPTH: usize = 8192;
const TEXT_MULTIPLIER: usize = 4;
const WORK_MULTIPLIER: usize = 4;
const SCANNER_LIMITS: json_scanner::Limits = json_scanner::Limits {
    max_bytes: LARGE_FILE_BYTES as usize,
    max_tokens: MAX_TOKENS,
    max_value_units: LARGE_FILE_BYTES as usize,
};
pub(crate) const PARSER_LIMITS: json_parser::Limits = json_parser::Limits {
    scanner: SCANNER_LIMITS,
    max_nodes: MAX_NODES,
    max_depth: MAX_DEPTH,
    max_diagnostics: MAX_NODES,
    max_comments: MAX_NODES,
};
const SYMBOL_LIMITS: json_symbols::Limits = json_symbols::Limits {
    parser: PARSER_LIMITS,
    max_symbols: MAX_NODES,
    max_text_units: LARGE_FILE_BYTES as usize * TEXT_MULTIPLIER,
    result_limit: None,
};
const FOLD_LIMITS: json_folding::Limits = json_folding::Limits {
    scanner: SCANNER_LIMITS,
    max_work_bytes: LARGE_FILE_BYTES as usize * WORK_MULTIPLIER,
    max_depth: MAX_DEPTH,
    max_ranges: MAX_NODES,
    result_limit: Some(taide_native_editor::folding::MAX_FOLDING_REGIONS),
};

pub(crate) struct Models {
    pub symbols: Arc<DocumentSymbols>,
    pub folds: Arc<SyntaxFolds>,
}

struct Entry {
    snapshot: DocumentSnapshot,
    cancelled: Arc<AtomicBool>,
    receiver: Option<oneshot::Receiver<Result<Models, EditorError>>>,
    complete: bool,
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
    ) -> Vec<(DocumentSnapshot, Result<Models, EditorError>)> {
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
                self.entries.insert(
                    snapshot.id,
                    Entry {
                        snapshot,
                        cancelled: Arc::new(AtomicBool::new(false)),
                        receiver: None,
                        complete: false,
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

fn build(snapshot: &DocumentSnapshot, cancelled: &AtomicBool) -> Result<Models, EditorError> {
    if snapshot.rope.len_bytes() > SCANNER_LIMITS.max_bytes {
        return Err(EditorError::Capacity);
    }
    let text = snapshot.rope.to_string();
    let uri = match &snapshot.key {
        DocumentKey::File(path) => url::Url::from_file_path(path)
            .map_err(|_| EditorError::InvalidIdentity)?
            .to_string(),
        DocumentKey::Untitled(tab) => format!("untitled:{tab}"),
        DocumentKey::AppFile(_) => return Err(EditorError::InvalidIdentity),
    };
    let native = json_symbols::document_symbols(&text, &uri, true, SYMBOL_LIMITS)?;
    if cancelled.load(Ordering::Acquire) {
        return Err(EditorError::StaleRevision);
    }
    let mut symbols = Vec::<Symbol>::with_capacity(native.items.len());
    let mut text_bytes = 0usize;
    for symbol in native.items {
        let container_label = symbol
            .parent
            .map(|index| {
                let parent = &symbols[index];
                let length = parent
                    .container_label
                    .len()
                    .checked_add(parent.name.len())
                    .and_then(|length| length.checked_add(" > ".len()))
                    .ok_or(EditorError::Capacity)?;
                if length > SYMBOL_LIMITS.max_text_units.saturating_sub(text_bytes) {
                    return Err(EditorError::Capacity);
                }
                if parent.container_label.is_empty() {
                    return Ok(parent.name.clone());
                }
                Ok(format!("{} > {}", parent.container_label, parent.name))
            })
            .transpose()?
            .unwrap_or_default();
        let name = String::from_utf16_lossy(&symbol.name);
        let detail = symbol
            .detail
            .as_deref()
            .map(String::from_utf16_lossy)
            .unwrap_or_default();
        text_bytes = text_bytes
            .checked_add(name.len())
            .and_then(|count| count.checked_add(detail.len()))
            .and_then(|count| count.checked_add(container_label.len()))
            .ok_or(EditorError::Capacity)?;
        if text_bytes > SYMBOL_LIMITS.max_text_units {
            return Err(EditorError::Capacity);
        }
        symbols.push(Symbol {
            name,
            detail,
            kind: symbol.kind,
            tags: Vec::new(),
            parent: symbol.parent,
            container_label,
            bytes: symbol.bytes,
            selection: symbol.selection_bytes,
        });
    }
    let symbols = Arc::new(DocumentSymbols::from_symbols(snapshot, symbols)?);
    let folds = json_folding::folding_ranges(&text, FOLD_LIMITS)?;
    if cancelled.load(Ordering::Acquire) {
        return Err(EditorError::StaleRevision);
    }
    Ok(Models {
        symbols,
        folds: Arc::new(SyntaxFolds::new(snapshot, vec![folds.ranges])),
    })
}

#[cfg(test)]
#[path = "editor-json-tests.rs"]
mod tests;

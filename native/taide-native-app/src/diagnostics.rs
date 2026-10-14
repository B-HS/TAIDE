use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use taide_lsp::native::protocol::lsp_types::{Diagnostic, DiagnosticSeverity};
use taide_native_editor::diagnostics::{
    Marker, MarkerSet, Message, Severity, display_range, marker_range,
};
use taide_native_editor::document::{DocumentId, DocumentKey, DocumentSnapshot, EditorError};
use taide_native_editor::store::EditorStore;
use uuid::Uuid;

const SEVERITY_COUNT: usize = 4;
const MAX_DISPLAY_MARKERS: usize = 500;
pub(crate) const ERROR: usize = 0;
pub(crate) const WARNING: usize = 1;
pub(crate) const INFO: usize = 2;
pub(crate) const HINT: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Owner(Uuid);

impl Owner {
    pub(crate) fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

pub type Bindings = HashMap<Owner, HashSet<DocumentId>>;

pub(crate) fn severity(diagnostic: &Diagnostic) -> usize {
    match diagnostic.severity {
        Some(DiagnosticSeverity::WARNING) => WARNING,
        Some(DiagnosticSeverity::INFORMATION) => INFO,
        Some(DiagnosticSeverity::HINT) => HINT,
        _ => ERROR,
    }
}

pub(crate) struct Batch {
    pub(crate) document: DocumentId,
    pub(crate) key: DocumentKey,
    pub(crate) title: String,
    pub(crate) diagnostics: Vec<Arc<Diagnostic>>,
    counts: [usize; SEVERITY_COUNT],
    markers: MarkerSet,
    navigation: MarkerSet,
    language: String,
}

impl Batch {
    pub(crate) fn revision(&self) -> u64 {
        self.navigation.revision()
    }
}

#[derive(Default)]
pub(crate) struct Store {
    bindings: Bindings,
    documents: HashSet<DocumentId>,
    batches: HashMap<(Owner, DocumentId), Batch>,
    builtin: HashMap<DocumentId, Batch>,
    builtin_documents: HashSet<DocumentId>,
    counts: [usize; SEVERITY_COUNT],
    revision: Arc<()>,
    display_cache: RefCell<HashMap<DocumentId, Arc<MarkerSet>>>,
}

impl Store {
    pub(crate) fn counts(&self) -> &[usize; SEVERITY_COUNT] {
        &self.counts
    }

    pub(crate) fn revision(&self) -> &Arc<()> {
        &self.revision
    }

    pub(crate) fn batches(&self) -> impl Iterator<Item = &Batch> {
        self.batches.values().chain(self.builtin.values())
    }

    pub(crate) fn title(&self, document: DocumentId) -> Option<&str> {
        self.builtin
            .get(&document)
            .map(|batch| batch.title.as_str())
    }

    pub(crate) fn problems(
        &self,
        editor: &EditorStore,
    ) -> Vec<taide_native_editor::problem_navigation::Problem> {
        let mut batches = self
            .batches
            .iter()
            .map(|((owner, id), batch)| (*id, batch, Some(owner.0)))
            .chain(self.builtin.iter().map(|(id, batch)| (*id, batch, None)))
            .collect::<Vec<_>>();
        batches.sort_by(|(_, left, left_owner), (_, right, right_owner)| {
            resource(&left.key)
                .cmp(&resource(&right.key))
                .then(left_owner.cmp(right_owner))
        });
        let mut problems = Vec::new();
        for (id, batch, _) in batches {
            let Ok(document) = editor.documents().snapshot(id) else {
                continue;
            };
            let Ok(changes) = editor.changes_since(id, batch.navigation.revision()) else {
                continue;
            };
            let Some(markers) = batch.navigation.tracked(&document, changes) else {
                continue;
            };
            let Some(resource) = resource(&batch.key) else {
                continue;
            };
            let rules = taide_native_syntax::monaco_language(&document.metadata.language_id)
                .ok()
                .flatten()
                .map(|rules| {
                    rules as &dyn taide_native_editor::language_configuration::LanguageRules
                });
            problems.extend(
                markers
                    .markers()
                    .iter()
                    .filter(|marker| marker.message.severity != Severity::Hint)
                    .map(|marker| taide_native_editor::problem_navigation::Problem {
                        resource: resource.clone(),
                        document: id,
                        marker: marker.clone(),
                        initial_range: display_range(&document, marker.bytes.clone(), rules),
                    }),
            );
        }
        problems
    }

    pub(crate) fn display(
        &self,
        editor: &EditorStore,
        document: &DocumentSnapshot,
    ) -> Option<Arc<MarkerSet>> {
        if let Some(cached) = self.display_cache.borrow().get(&document.id)
            && cached.revision() == document.revision
            && cached
                .tracked(
                    document,
                    editor.changes_since(document.id, cached.revision()).ok()?,
                )
                .is_some()
        {
            return Some(cached.clone());
        }
        let mut batches = self
            .batches
            .iter()
            .filter(|((_, id), _)| *id == document.id)
            .collect::<Vec<_>>();
        batches.sort_by_key(|((owner, _), _)| owner.0);
        let mut markers = Vec::new();
        let builtin = self.builtin.get(&document.id);
        for batch in batches.into_iter().map(|(_, batch)| batch).chain(builtin) {
            let changes = editor
                .changes_since(document.id, batch.markers.revision())
                .ok()?;
            let Some(tracked) = batch.markers.tracked(document, changes) else {
                continue;
            };
            markers.extend(
                tracked
                    .markers()
                    .iter()
                    .take(MAX_DISPLAY_MARKERS - markers.len())
                    .cloned(),
            );
            if markers.len() == MAX_DISPLAY_MARKERS {
                break;
            }
        }
        let display = Arc::new(MarkerSet::new(document, markers).ok()?);
        self.display_cache
            .borrow_mut()
            .insert(document.id, display.clone());
        Some(display)
    }

    pub(crate) fn retain_documents(&mut self, documents: HashSet<DocumentId>) {
        self.documents = documents;
        self.prune();
    }

    pub(crate) fn reconcile(&mut self, bindings: Bindings) {
        self.bindings = bindings;
        self.prune();
    }

    pub(crate) fn remove_document(&mut self, document: DocumentId) {
        self.documents.remove(&document);
        self.builtin_documents.remove(&document);
        self.remove_builtin(document);
        self.prune();
    }

    pub(crate) fn retain_builtin(&mut self, editor: &EditorStore, documents: HashSet<DocumentId>) {
        self.builtin_documents = documents;
        let removed = self
            .builtin
            .iter()
            .filter(|(id, batch)| {
                !self.builtin_documents.contains(id)
                    || !editor.documents().snapshot(**id).is_ok_and(|snapshot| {
                        snapshot.key == batch.key && snapshot.metadata.language_id == batch.language
                    })
            })
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for id in removed {
            self.remove_builtin(id);
        }
    }

    pub(crate) fn remove_builtin(&mut self, document: DocumentId) {
        if let Some(batch) = self.builtin.remove(&document) {
            self.subtract(&batch.counts);
            self.revision = Arc::new(());
            self.display_cache.get_mut().clear();
        }
    }

    pub(crate) fn publish_builtin(
        &mut self,
        snapshot: &DocumentSnapshot,
        markers: Vec<Marker>,
        title: String,
    ) -> Result<(), EditorError> {
        if !self.builtin_documents.contains(&snapshot.id) {
            return Ok(());
        }
        let navigation = MarkerSet::new(snapshot, markers)?;
        let rules = taide_native_syntax::monaco_language(&snapshot.metadata.language_id)
            .ok()
            .flatten()
            .map(|rules| rules as &dyn taide_native_editor::language_configuration::LanguageRules);
        let display = MarkerSet::new(
            snapshot,
            navigation
                .markers()
                .iter()
                .map(|marker| Marker {
                    bytes: display_range(snapshot, marker.bytes.clone(), rules),
                    message: marker.message.clone(),
                })
                .collect(),
        )?;
        let diagnostics = navigation
            .markers()
            .iter()
            .map(|marker| {
                Ok(Arc::new(Diagnostic {
                    range: taide_lsp::native::protocol::lsp_types::Range {
                        start: taide_native_editor::lsp::byte_to_position(
                            snapshot,
                            marker.bytes.start,
                        )?,
                        end: taide_native_editor::lsp::byte_to_position(
                            snapshot,
                            marker.bytes.end,
                        )?,
                    },
                    severity: Some(match marker.message.severity {
                        Severity::Error => DiagnosticSeverity::ERROR,
                        Severity::Warning => DiagnosticSeverity::WARNING,
                        Severity::Information => DiagnosticSeverity::INFORMATION,
                        Severity::Hint => DiagnosticSeverity::HINT,
                    }),
                    message: marker.message.text.clone(),
                    source: marker.message.source.clone(),
                    code: marker
                        .message
                        .code
                        .clone()
                        .map(taide_lsp::native::protocol::lsp_types::NumberOrString::String),
                    ..Default::default()
                }))
            })
            .collect::<Result<Vec<_>, EditorError>>()?;
        let mut counts = [0; SEVERITY_COUNT];
        for diagnostic in &diagnostics {
            counts[severity(diagnostic)] += 1;
        }
        self.remove_builtin(snapshot.id);
        for (total, count) in self.counts.iter_mut().zip(counts) {
            *total += count;
        }
        if !diagnostics.is_empty() {
            self.builtin.insert(
                snapshot.id,
                Batch {
                    document: snapshot.id,
                    key: snapshot.key.clone(),
                    title,
                    diagnostics,
                    counts,
                    markers: display,
                    navigation,
                    language: snapshot.metadata.language_id.clone(),
                },
            );
        }
        self.revision = Arc::new(());
        self.display_cache.get_mut().clear();
        Ok(())
    }

    pub(crate) fn publish(
        &mut self,
        owner: Owner,
        snapshot: &DocumentSnapshot,
        diagnostics: Vec<Diagnostic>,
    ) {
        if !self.documents.contains(&snapshot.id)
            || !self
                .bindings
                .get(&owner)
                .is_some_and(|documents| documents.contains(&snapshot.id))
        {
            return;
        }
        let DocumentKey::File(path) = &snapshot.key else {
            return;
        };
        let key = (owner, snapshot.id);
        if self.batches.get(&key).is_some_and(|batch| {
            batch.key == snapshot.key
                && batch.markers.revision() == snapshot.revision
                && batch.language == snapshot.metadata.language_id
                && batch.diagnostics.len() == diagnostics.len()
                && batch
                    .diagnostics
                    .iter()
                    .zip(&diagnostics)
                    .all(|(stored, incoming)| stored.as_ref() == incoming)
        }) {
            return;
        }
        if let Some(previous) = self.batches.remove(&key) {
            self.subtract(&previous.counts);
        }
        let mut counts = [0; SEVERITY_COUNT];
        for diagnostic in &diagnostics {
            counts[severity(diagnostic)] += 1;
        }
        for (total, count) in self.counts.iter_mut().zip(counts) {
            *total += count;
        }
        if !diagnostics.is_empty() {
            let rules = taide_native_syntax::monaco_language(&snapshot.metadata.language_id)
                .ok()
                .flatten()
                .map(|rules| {
                    rules as &dyn taide_native_editor::language_configuration::LanguageRules
                });
            let navigation_markers = diagnostics
                .iter()
                .filter_map(|diagnostic| {
                    let severity = match severity(diagnostic) {
                        WARNING => Severity::Warning,
                        INFO => Severity::Information,
                        HINT => Severity::Hint,
                        _ => Severity::Error,
                    };
                    Some(Marker {
                        bytes: marker_range(snapshot, diagnostic.range, severity)?,
                        message: Arc::new(Message {
                            severity,
                            text: diagnostic.message.clone(),
                            source: diagnostic.source.clone(),
                            code: diagnostic.code.as_ref().map(|code| match code {
                                taide_lsp::native::protocol::lsp_types::NumberOrString::Number(
                                    number,
                                ) => number.to_string(),
                                taide_lsp::native::protocol::lsp_types::NumberOrString::String(
                                    text,
                                ) => text.clone(),
                            }),
                        }),
                    })
                })
                .collect::<Vec<_>>();
            let markers = navigation_markers
                .iter()
                .map(|marker| Marker {
                    bytes: display_range(snapshot, marker.bytes.clone(), rules),
                    message: marker.message.clone(),
                })
                .collect();
            self.batches.insert(
                key,
                Batch {
                    document: snapshot.id,
                    key: snapshot.key.clone(),
                    title: path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    diagnostics: diagnostics.into_iter().map(Arc::new).collect(),
                    counts,
                    markers: MarkerSet::new(snapshot, markers)
                        .expect("validated diagnostic boundaries"),
                    navigation: MarkerSet::new(snapshot, navigation_markers)
                        .expect("validated diagnostic boundaries"),
                    language: snapshot.metadata.language_id.clone(),
                },
            );
        }
        self.revision = Arc::new(());
        self.display_cache.borrow_mut().clear();
    }

    fn prune(&mut self) {
        self.display_cache
            .get_mut()
            .retain(|id, _| self.documents.contains(id));
        let removed = self
            .batches
            .keys()
            .filter(|(owner, document)| {
                !self.documents.contains(document)
                    || !self
                        .bindings
                        .get(owner)
                        .is_some_and(|documents| documents.contains(document))
            })
            .copied()
            .collect::<Vec<_>>();
        if removed.is_empty() {
            return;
        }
        for key in removed {
            if let Some(batch) = self.batches.remove(&key) {
                self.subtract(&batch.counts);
            }
        }
        self.revision = Arc::new(());
        self.display_cache.get_mut().clear();
    }

    fn subtract(&mut self, counts: &[usize; SEVERITY_COUNT]) {
        for (total, count) in self.counts.iter_mut().zip(counts) {
            *total -= count;
        }
    }
}

pub(crate) fn resource(key: &DocumentKey) -> Option<String> {
    match key {
        DocumentKey::File(path) => url::Url::from_file_path(path)
            .ok()
            .map(|uri| uri.to_string()),
        DocumentKey::Untitled(tab) => Some(format!("untitled:{tab}")),
        DocumentKey::AppFile(_) => None,
    }
}

#[cfg(test)]
#[path = "diagnostics-tests.rs"]
mod tests;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use taide_lsp::native::protocol::lsp_types::{Diagnostic, DiagnosticSeverity};
use taide_native_editor::document::{DocumentId, DocumentKey, DocumentSnapshot};
use uuid::Uuid;

const SEVERITY_COUNT: usize = 4;
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
    pub(crate) path: PathBuf,
    pub(crate) diagnostics: Vec<Arc<Diagnostic>>,
    counts: [usize; SEVERITY_COUNT],
}

#[derive(Default)]
pub(crate) struct Store {
    bindings: Bindings,
    documents: HashSet<DocumentId>,
    batches: HashMap<(Owner, DocumentId), Batch>,
    counts: [usize; SEVERITY_COUNT],
    revision: Arc<()>,
}

impl Store {
    pub(crate) fn counts(&self) -> &[usize; SEVERITY_COUNT] {
        &self.counts
    }

    pub(crate) fn revision(&self) -> &Arc<()> {
        &self.revision
    }

    pub(crate) fn batches(&self) -> impl Iterator<Item = &Batch> {
        self.batches.values()
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
        self.prune();
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
            batch.path == *path
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
            self.batches.insert(
                key,
                Batch {
                    path: path.clone(),
                    diagnostics: diagnostics.into_iter().map(Arc::new).collect(),
                    counts,
                },
            );
        }
        self.revision = Arc::new(());
    }

    fn prune(&mut self) {
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
    }

    fn subtract(&mut self, counts: &[usize; SEVERITY_COUNT]) {
        for (total, count) in self.counts.iter_mut().zip(counts) {
            *total -= count;
        }
    }
}

#[cfg(test)]
#[path = "diagnostics-tests.rs"]
mod tests;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use taide_model::file::{FileSizeTier, OpenedFile};
use taide_native_editor::document::{DocumentId, DocumentKey, EditorError};
use taide_native_editor::store::EditorStore;
use taide_native_editor::symbol_locations::Target;
use taide_native_editor::view::ViewId;

pub(crate) const PRELOAD_LIMIT: usize = 8;
pub(crate) const MODEL_TTL: Duration = Duration::from_secs(60);

struct Cached {
    key: DocumentKey,
    deadline: Instant,
    project: Option<taide_model::ids::ProjectId>,
}

#[derive(Default)]
pub(crate) struct Models {
    owned: HashMap<DocumentId, Cached>,
}

impl Models {
    pub(crate) fn preload_paths(&self, store: &EditorStore, targets: &[Target]) -> Vec<PathBuf> {
        let mut seen = HashSet::new();
        targets
            .iter()
            .filter_map(|target| {
                let path = crate::editor_locations::file_path(&target.uri)?;
                (store
                    .documents()
                    .find(&DocumentKey::File(path.clone()))
                    .is_none()
                    && seen.insert(path.clone()))
                .then_some(path)
            })
            .take(PRELOAD_LIMIT)
            .collect()
    }

    pub(crate) fn admit(
        &mut self,
        store: &mut EditorStore,
        file: OpenedFile,
        now: Instant,
    ) -> Result<Option<DocumentId>, EditorError> {
        let key = DocumentKey::File(PathBuf::from(&file.path));
        if let Some(document) = store.documents().find(&key) {
            return Ok(Some(document));
        }
        if file.tier != FileSizeTier::Normal || file.read_only || file.encoding_lossy {
            return Ok(None);
        }
        let DocumentKey::File(path) = &key else {
            return Err(EditorError::InvalidIdentity);
        };
        let document = store.open_file(path.clone(), file)?;
        self.owned.insert(
            document,
            Cached {
                key,
                deadline: now + MODEL_TTL,
                project: None,
            },
        );
        Ok(Some(document))
    }

    pub(crate) fn adopt(&mut self, document: DocumentId) {
        self.owned.remove(&document);
    }

    pub(crate) fn associate(
        &mut self,
        document: DocumentId,
        project: Option<taide_model::ids::ProjectId>,
    ) {
        if let Some(cached) = self.owned.get_mut(&document) {
            cached.project = project;
        }
    }

    pub(crate) fn project(&self, document: DocumentId) -> Option<taide_model::ids::ProjectId> {
        self.owned
            .get(&document)
            .and_then(|cached| cached.project.clone())
    }

    pub(crate) fn owns(&self, document: DocumentId) -> bool {
        self.owned.contains_key(&document)
    }

    pub(crate) fn sweep(
        &mut self,
        store: &mut EditorStore,
        preview_views: &HashSet<ViewId>,
        now: Instant,
    ) -> Vec<DocumentId> {
        let mut disposed = Vec::new();
        self.owned.retain(|document, cached| {
            let Ok(current) = store.documents().snapshot(*document) else {
                return false;
            };
            if current.key != cached.key {
                return false;
            }
            let views = store
                .views()
                .for_document(*document)
                .map(|view| view.id)
                .collect::<Vec<_>>();
            if views.iter().any(|view| !preview_views.contains(view)) {
                return false;
            }
            if now < cached.deadline || !views.is_empty() || current.dirty {
                return true;
            }
            if store.release_document(*document).is_ok() {
                disposed.push(*document);
                return false;
            }
            true
        });
        disposed
    }

    pub(crate) fn next_expiry(&self, now: Instant) -> Option<Duration> {
        self.owned
            .values()
            .filter(|entry| entry.deadline > now)
            .map(|entry| entry.deadline - now)
            .min()
    }
}

#[cfg(test)]
#[path = "peek-models-tests.rs"]
mod tests;

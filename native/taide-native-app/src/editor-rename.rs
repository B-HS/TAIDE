use std::collections::HashSet;
use std::ops::Range;

use taide_lsp::native::{Failure, protocol::lsp_types};
use taide_model::ids::ProjectId;
use taide_native_editor::document::{DocumentSnapshot, EditorError};
use taide_native_editor::lsp::byte_to_position;
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{SelectionSet, ViewId, ViewKey};
use tokio::sync::watch;
use uuid::Uuid;

use crate::editor_symbols::ProviderIdentity;

#[cfg(test)]
#[path = "editor-rename-tests.rs"]
mod tests;

#[derive(Clone)]
pub(crate) enum Stage {
    Prepare,
    Rename {
        name: String,
        provider: ProviderIdentity,
    },
}

#[derive(Clone)]
pub(crate) struct Request {
    pub(crate) project: ProjectId,
    pub(crate) snapshot: DocumentSnapshot,
    pub(crate) position: lsp_types::Position,
    pub(crate) fallback: Range<usize>,
    pub(crate) stage: Stage,
    pub(crate) cancelled: watch::Receiver<bool>,
    pub(crate) source: ViewId,
    source_key: ViewKey,
    pub(crate) owner: ViewId,
    owner_key: ViewKey,
    owner_document: taide_native_editor::document::DocumentId,
    pub(crate) selection: SelectionSet,
    pub(crate) token: Uuid,
}

impl Request {
    pub(crate) fn is_cancelled(&self) -> bool {
        *self.cancelled.borrow() || self.cancelled.has_changed().is_err()
    }

    pub(crate) fn describes(&self, store: &EditorStore) -> bool {
        !self.is_cancelled()
            && store.views().get(self.owner).is_some_and(|owner| {
                owner.key == self.owner_key && owner.document == self.owner_document
            })
            && store.views().get(self.source).is_some_and(|source| {
                source.key == self.source_key
                    && source.document == self.snapshot.id
                    && source.composition.is_none()
                    && source.selection == self.selection
                    && store
                        .documents()
                        .snapshot(source.document)
                        .is_ok_and(|current| {
                            current.key == self.snapshot.key
                                && current.revision == self.snapshot.revision
                                && current.metadata.language_id
                                    == self.snapshot.metadata.language_id
                                && current.metadata.tier == self.snapshot.metadata.tier
                                && !current.metadata.read_only
                        })
            })
    }

    pub(crate) fn fallback_name(&self) -> String {
        self.snapshot
            .rope
            .byte_slice(self.fallback.clone())
            .to_string()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Prepared {
    pub(crate) provider: ProviderIdentity,
    pub(crate) range: Range<usize>,
    pub(crate) name: String,
}

pub(crate) struct Edits {
    pub(crate) provider: ProviderIdentity,
    pub(crate) edit: lsp_types::WorkspaceEdit,
    pub(crate) documents: Vec<crate::lsp::ProtocolDocument>,
    pub(crate) roots: Vec<String>,
}

pub(crate) enum Response {
    Prepared(Prepared),
    Edits(Edits),
    Unavailable,
}

struct Entry {
    request: Request,
    providers: HashSet<ProviderIdentity>,
    prepared: Option<Prepared>,
    applying: bool,
    cancel: watch::Sender<bool>,
}

impl Drop for Entry {
    fn drop(&mut self) {
        self.cancel.send_replace(true);
    }
}

#[derive(Default)]
pub(crate) struct State {
    entry: Option<Entry>,
}

impl State {
    pub(crate) fn clear(&mut self) {
        self.entry = None;
    }

    pub(crate) fn begin(
        &mut self,
        store: &EditorStore,
        project: ProjectId,
        source: ViewId,
        owner: ViewId,
        providers: HashSet<ProviderIdentity>,
    ) -> Result<Option<Request>, EditorError> {
        self.clear();
        let source = store.views().get(source).ok_or(EditorError::NotFound)?;
        let owner = store.views().get(owner).ok_or(EditorError::NotFound)?;
        let snapshot = store.documents().snapshot(source.document)?;
        if snapshot.metadata.read_only {
            return Err(EditorError::ReadOnly);
        }
        if providers.is_empty() || source.composition.is_some() {
            return Ok(None);
        }
        let head = source.selection.selections[source.selection.primary].head;
        let rules = crate::editor_syntax::language_rules(&snapshot.metadata.language_id)
            .or_else(|| crate::editor_syntax::language_rules("plaintext"));
        let fallback = taide_native_editor::cursor_commands::word_range(&snapshot, head, rules)
            .unwrap_or(head..head);
        let (cancel, cancelled) = watch::channel(false);
        let request = Request {
            position: byte_to_position(&snapshot, head)?,
            project,
            snapshot,
            fallback,
            stage: Stage::Prepare,
            cancelled,
            source: source.id,
            source_key: source.key.clone(),
            owner: owner.id,
            owner_key: owner.key.clone(),
            owner_document: owner.document,
            selection: source.selection.clone(),
            token: Uuid::new_v4(),
        };
        self.entry = Some(Entry {
            request: request.clone(),
            providers,
            prepared: None,
            applying: false,
            cancel,
        });
        Ok(Some(request))
    }

    pub(crate) fn accept(
        &mut self,
        store: &EditorStore,
        request: &Request,
        providers: HashSet<ProviderIdentity>,
        response: Result<Response, Failure>,
    ) -> Result<Option<Edits>, Failure> {
        let Some(entry) = self.entry.as_mut().filter(|entry| {
            entry.request.token == request.token
                && !entry.applying
                && request.describes(store)
                && entry.providers == providers
        }) else {
            return Ok(None);
        };
        match response {
            Ok(Response::Prepared(prepared)) if matches!(request.stage, Stage::Prepare) => {
                if !providers.contains(&prepared.provider)
                    || prepared.range.start > prepared.range.end
                    || byte_to_position(&request.snapshot, prepared.range.start).is_err()
                    || byte_to_position(&request.snapshot, prepared.range.end).is_err()
                {
                    self.clear();
                    return Err(Failure::MalformedResponse);
                }
                entry.prepared = Some(prepared);
                Ok(None)
            }
            Ok(Response::Edits(edits)) if matches!(request.stage, Stage::Rename { .. }) => {
                if !providers.contains(&edits.provider) {
                    self.clear();
                    return Err(Failure::StaleGeneration);
                }
                entry.applying = true;
                Ok(Some(edits))
            }
            Ok(Response::Unavailable) => {
                self.clear();
                Err(Failure::UnsupportedCapability)
            }
            Ok(_) => {
                self.clear();
                Err(Failure::MalformedResponse)
            }
            Err(error) => {
                self.clear();
                Err(error)
            }
        }
    }

    pub(crate) fn prepared(&self) -> Option<(&Request, &Prepared)> {
        let entry = self
            .entry
            .as_ref()
            .filter(|entry| matches!(entry.request.stage, Stage::Prepare) && !entry.applying)?;
        Some((&entry.request, entry.prepared.as_ref()?))
    }

    pub(crate) fn rename(&mut self, store: &EditorStore, name: String) -> Option<Request> {
        let (request, prepared) = self.prepared()?;
        if !request.describes(store)
            || name == prepared.name
            || name
                .chars()
                .all(taide_native_editor::language_configuration::is_js_whitespace)
        {
            self.clear();
            return None;
        }
        let mut request = request.clone();
        request.stage = Stage::Rename {
            name,
            provider: prepared.provider,
        };
        request.token = Uuid::new_v4();
        let entry = self.entry.as_mut()?;
        entry.request = request.clone();
        entry.prepared = None;
        Some(request)
    }

    pub(crate) fn finish(&mut self, request: &Request) {
        if self
            .entry
            .as_ref()
            .is_some_and(|entry| entry.request.token == request.token)
        {
            self.clear();
        }
    }

    pub(crate) fn reconcile(
        &mut self,
        store: &EditorStore,
        providers: impl FnOnce(&Request) -> HashSet<ProviderIdentity>,
    ) {
        if self.entry.as_ref().is_some_and(|entry| {
            !entry.applying
                && (!entry.request.describes(store) || entry.providers != providers(&entry.request))
        }) {
            self.clear();
        }
    }
}

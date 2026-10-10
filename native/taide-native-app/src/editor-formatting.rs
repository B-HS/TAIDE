use std::collections::{HashMap, HashSet};

use taide_lsp::native::Failure;
use taide_lsp::native::protocol::lsp_types;
use taide_model::ids::ProjectId;
use taide_native_editor::document::{DocumentSnapshot, EditorError};
use taide_native_editor::formatting::{Command, minimal_edits, selection_ranges};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{ViewId, ViewKey};
use tokio::sync::watch;
use uuid::Uuid;

use crate::editor_symbols::ProviderIdentity;

#[cfg(test)]
#[path = "editor-formatting-tests.rs"]
mod tests;

#[derive(Clone)]
pub(crate) enum Kind {
    Document,
    Ranges(Vec<lsp_types::Range>),
}

#[derive(Clone)]
pub struct Request {
    pub(crate) project: ProjectId,
    pub(crate) snapshot: DocumentSnapshot,
    pub(crate) kind: Kind,
    pub(crate) options: lsp_types::FormattingOptions,
    pub(crate) cancelled: watch::Receiver<bool>,
    pub(crate) command: Command,
    source: ViewId,
    source_key: ViewKey,
    owner: ViewId,
    owner_key: ViewKey,
    owner_document: taide_native_editor::document::DocumentId,
    head: usize,
    token: Uuid,
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
                    && source.selection.selections[source.selection.primary].head == self.head
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
                                && current
                                    .model_indentation(IndentOptions {
                                        tab_size: self.options.tab_size,
                                        insert_spaces: self.options.insert_spaces,
                                    })
                                    .formatting_options()
                                    == (IndentOptions {
                                        tab_size: self.options.tab_size,
                                        insert_spaces: self.options.insert_spaces,
                                    })
                        })
            })
    }
}

pub struct Response {
    pub(crate) provider: ProviderIdentity,
    pub(crate) edits: Vec<lsp_types::TextEdit>,
}

struct Entry {
    request: Request,
    providers: HashSet<ProviderIdentity>,
    cancel: watch::Sender<bool>,
}

impl Drop for Entry {
    fn drop(&mut self) {
        self.cancel.send_replace(true);
    }
}

#[derive(Default)]
pub(crate) struct State {
    entries: HashMap<ViewId, Entry>,
}

impl State {
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    pub(crate) fn begin(
        &mut self,
        store: &EditorStore,
        project: ProjectId,
        source: ViewId,
        owner: ViewId,
        command: Command,
        fallback: IndentOptions,
        providers: HashSet<ProviderIdentity>,
    ) -> Result<Option<Request>, EditorError> {
        let source = store.views().get(source).ok_or(EditorError::NotFound)?;
        let owner = store.views().get(owner).ok_or(EditorError::NotFound)?;
        let snapshot = store.documents().snapshot(source.document)?;
        if snapshot.metadata.read_only {
            return Err(EditorError::ReadOnly);
        }
        if source.composition.is_some() || providers.is_empty() {
            return Ok(None);
        }
        let options = snapshot.model_indentation(fallback).formatting_options();
        let kind = match command {
            Command::Document => Kind::Document,
            Command::Selection => Kind::Ranges(selection_ranges(&snapshot, &source.selection)?),
        };
        let (cancel, cancelled) = watch::channel(false);
        let request = Request {
            project,
            snapshot,
            kind,
            options: lsp_types::FormattingOptions {
                tab_size: options.tab_size,
                insert_spaces: options.insert_spaces,
                ..Default::default()
            },
            cancelled,
            command,
            source: source.id,
            source_key: source.key.clone(),
            owner: owner.id,
            owner_key: owner.key.clone(),
            owner_document: owner.document,
            head: source.selection.selections[source.selection.primary].head,
            token: Uuid::new_v4(),
        };
        self.entries.insert(
            source.id,
            Entry {
                request: request.clone(),
                providers,
                cancel,
            },
        );
        Ok(Some(request))
    }

    pub(crate) fn reconcile(
        &mut self,
        store: &EditorStore,
        providers: impl Fn(&Request) -> HashSet<ProviderIdentity>,
    ) {
        self.entries.retain(|_, entry| {
            entry.request.describes(store) && entry.providers == providers(&entry.request)
        });
    }

    pub(crate) fn accept(
        &mut self,
        store: &mut EditorStore,
        request: &Request,
        providers: HashSet<ProviderIdentity>,
        response: Result<Response, Failure>,
    ) -> Result<bool, EditorError> {
        let valid = self.entries.get(&request.source).is_some_and(|entry| {
            entry.request.token == request.token
                && request.describes(store)
                && entry.providers == providers
        });
        if !valid {
            return Ok(false);
        }
        let response = match response {
            Ok(response) if providers.contains(&response.provider) => response,
            _ => {
                self.entries.remove(&request.source);
                return Ok(false);
            }
        };
        let edits = minimal_edits(&request.snapshot, response.edits);
        self.entries.remove(&request.source);
        let changed = taide_native_editor::lsp::apply_text_edits(
            store,
            &request.snapshot,
            Some(request.source),
            edits?,
        )?;
        if changed {
            let source = store
                .views()
                .get(request.source)
                .ok_or(EditorError::NotFound)?;
            let head = source.selection.selections[source.selection.primary].head;
            store.request_selection_reveal(request.source, head..head, false)?;
        }
        Ok(changed)
    }

    pub(crate) fn reject(&mut self, request: &Request) {
        if self
            .entries
            .get(&request.source)
            .is_some_and(|entry| entry.request.token == request.token)
        {
            self.entries.remove(&request.source);
        }
    }
}

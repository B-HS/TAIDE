use std::collections::HashSet;
use std::sync::Arc;

use taide_lsp::native::{Failure, protocol::lsp_types};
use taide_model::file::FileSizeTier;
use taide_model::ids::ProjectId;
use taide_native_editor::document::{DocumentSnapshot, EditorError};
use taide_native_editor::lsp::byte_to_position;
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{SelectionSet, ViewId, ViewKey};
use taide_native_ui::editor_code_actions::{Action, Command};
use tokio::sync::watch;
use uuid::Uuid;

use crate::editor_symbols::ProviderIdentity;

#[cfg(test)]
#[path = "editor-code-actions-tests.rs"]
mod tests;

#[derive(Clone, Copy)]
pub(crate) struct Invocation {
    pub(crate) command: Command,
    pub(crate) automatic: bool,
}

#[derive(Clone)]
pub(crate) struct Request {
    pub(crate) project: ProjectId,
    pub(crate) snapshot: DocumentSnapshot,
    pub(crate) source: ViewId,
    source_key: ViewKey,
    pub(crate) owner: ViewId,
    owner_key: ViewKey,
    owner_document: taide_native_editor::document::DocumentId,
    pub(crate) selection: SelectionSet,
    pub(crate) range: lsp_types::Range,
    pub(crate) invocation: Invocation,
    pub(crate) token: Uuid,
    pub(crate) cancelled: watch::Receiver<bool>,
}

impl Request {
    pub(crate) fn is_cancelled(&self) -> bool {
        *self.cancelled.borrow() || self.cancelled.has_changed().is_err()
    }

    pub(crate) fn describes(&self, store: &EditorStore) -> bool {
        !self.is_cancelled() && self.matches(store)
    }

    pub(crate) fn matches(&self, store: &EditorStore) -> bool {
        self.has_views(store)
            && store.views().get(self.source).is_some_and(|source| {
                source.composition.is_none()
                    && source.selection == self.selection
                    && store
                        .documents()
                        .snapshot(source.document)
                        .is_ok_and(|current| {
                            current.key == self.snapshot.key
                                && current.revision == self.snapshot.revision
                                && current.metadata.language_id
                                    == self.snapshot.metadata.language_id
                                && current.metadata.tier == FileSizeTier::Normal
                                && !current.metadata.read_only
                        })
            })
    }

    fn has_views(&self, store: &EditorStore) -> bool {
        store.views().get(self.owner).is_some_and(|owner| {
            owner.key == self.owner_key && owner.document == self.owner_document
        }) && store.views().get(self.source).is_some_and(|source| {
            source.key == self.source_key && source.document == self.snapshot.id
        })
    }
}

#[derive(Clone)]
pub(crate) struct Candidate {
    pub(crate) provider: ProviderIdentity,
    pub(crate) raw: lsp_types::CodeActionOrCommand,
}

impl Candidate {
    pub(crate) fn presentation(&self) -> Action {
        match &self.raw {
            lsp_types::CodeActionOrCommand::Command(command) => Action {
                title: command.title.clone(),
                kind: None,
                preferred: false,
                disabled: None,
            },
            lsp_types::CodeActionOrCommand::CodeAction(action) => Action {
                title: action.title.clone(),
                kind: action.kind.as_ref().map(|kind| kind.as_str().to_owned()),
                preferred: action.is_preferred.unwrap_or(false),
                disabled: action
                    .disabled
                    .as_ref()
                    .map(|disabled| disabled.reason.clone()),
            },
        }
    }
}

#[derive(Clone)]
pub(crate) struct Chosen {
    pub(crate) request: Request,
    pub(crate) candidate: Candidate,
    pub(crate) preview: bool,
}

pub(crate) struct Resolved {
    pub(crate) provider: ProviderIdentity,
    pub(crate) edit: Option<lsp_types::WorkspaceEdit>,
    pub(crate) command: Option<lsp_types::Command>,
    pub(crate) documents: Vec<crate::lsp::ProtocolDocument>,
    pub(crate) roots: Vec<String>,
}

pub(crate) struct Navigation {
    pub(crate) uri: lsp_types::Uri,
    pub(crate) position: lsp_types::Position,
    pub(crate) targets: Vec<taide_native_editor::symbol_locations::Target>,
    pub(crate) mode: taide_native_editor::symbol_locations::Mode,
}

pub(crate) struct Finished {
    pub(crate) request: Request,
    pub(crate) provider: ProviderIdentity,
    pub(crate) navigation: Navigation,
}

struct Entry {
    request: Request,
    providers: HashSet<ProviderIdentity>,
    candidates: Vec<Candidate>,
    presentation: Arc<[Action]>,
    received: bool,
    menu_open: bool,
    choosing: bool,
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
        invocation: Invocation,
    ) -> Result<Option<Request>, EditorError> {
        self.clear();
        let source = store.views().get(source).ok_or(EditorError::NotFound)?;
        let owner = store.views().get(owner).ok_or(EditorError::NotFound)?;
        let snapshot = store.documents().snapshot(source.document)?;
        if snapshot.metadata.read_only {
            return Err(EditorError::ReadOnly);
        }
        if snapshot.metadata.tier != FileSizeTier::Normal
            || source.composition.is_some()
            || providers.is_empty()
        {
            return Ok(None);
        }
        let primary = source
            .selection
            .selections
            .get(source.selection.primary)
            .ok_or(EditorError::NotFound)?;
        let range = lsp_types::Range {
            start: byte_to_position(&snapshot, primary.anchor.min(primary.head))?,
            end: byte_to_position(&snapshot, primary.anchor.max(primary.head))?,
        };
        let (cancel, cancelled) = watch::channel(false);
        let request = Request {
            project,
            snapshot,
            source: source.id,
            source_key: source.key.clone(),
            owner: owner.id,
            owner_key: owner.key.clone(),
            owner_document: owner.document,
            selection: source.selection.clone(),
            range,
            invocation,
            token: Uuid::new_v4(),
            cancelled,
        };
        self.entry = Some(Entry {
            request: request.clone(),
            providers,
            candidates: Vec::new(),
            presentation: Arc::default(),
            received: false,
            menu_open: !invocation.automatic,
            choosing: false,
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
        response: Result<Vec<Candidate>, Failure>,
    ) -> Result<Option<Chosen>, Failure> {
        let Some(entry) = self.entry.as_mut().filter(|entry| {
            entry.request.token == request.token && !entry.received && !entry.applying
        }) else {
            return Ok(None);
        };
        if !request.describes(store) || entry.providers != providers {
            self.clear();
            return Ok(None);
        }
        let candidates = match response {
            Ok(candidates) => candidates,
            Err(error) => {
                self.clear();
                return Err(error);
            }
        };
        if candidates
            .iter()
            .any(|candidate| !providers.contains(&candidate.provider))
        {
            self.clear();
            return Err(Failure::StaleGeneration);
        }
        let mut filtered = candidates
            .into_iter()
            .map(|candidate| {
                let presentation = candidate.presentation();
                (candidate, presentation)
            })
            .filter(|(_, presentation)| request.invocation.command.accepts(presentation))
            .collect::<Vec<_>>();
        filtered.sort_by_key(|(_, presentation)| std::cmp::Reverse(presentation.preferred));
        let (candidates, presentation): (Vec<_>, Vec<_>) = filtered.into_iter().unzip();
        entry.candidates = candidates;
        entry.presentation = presentation.into();
        entry.received = true;
        let enabled = entry
            .presentation
            .iter()
            .enumerate()
            .filter(|(_, action)| action.disabled.is_none())
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if !request.invocation.automatic
            && request.invocation.command.auto_apply_single()
            && enabled.len() == 1
        {
            return Ok(self.select(
                store,
                request.source,
                request.token.as_u128(),
                enabled[0],
                false,
            ));
        }
        Ok(None)
    }

    pub(crate) fn listed(
        &self,
        store: &EditorStore,
        view: ViewId,
    ) -> Option<(&Request, Arc<[Action]>, bool)> {
        let entry = self.entry.as_ref().filter(|entry| {
            entry.received
                && !entry.choosing
                && !entry.applying
                && entry.request.source == view
                && entry.request.describes(store)
        })?;
        Some((&entry.request, entry.presentation.clone(), entry.menu_open))
    }

    pub(crate) fn open(&mut self, store: &EditorStore, view: ViewId, token: u128) -> bool {
        if !self
            .listed(store, view)
            .is_some_and(|(request, _, _)| request.token.as_u128() == token)
        {
            return false;
        }
        self.entry.as_mut().unwrap().menu_open = true;
        true
    }

    pub(crate) fn select(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        token: u128,
        index: usize,
        preview: bool,
    ) -> Option<Chosen> {
        let entry = self.entry.as_mut().filter(|entry| {
            entry.received
                && !entry.choosing
                && !entry.applying
                && entry.request.source == view
                && entry.request.token.as_u128() == token
                && entry.request.describes(store)
        })?;
        if entry.presentation.get(index)?.disabled.is_some() {
            return None;
        }
        let candidate = entry.candidates.get(index)?.clone();
        entry.choosing = true;
        Some(Chosen {
            request: entry.request.clone(),
            candidate,
            preview,
        })
    }

    pub(crate) fn begin_apply(
        &mut self,
        store: &EditorStore,
        chosen: &Chosen,
        providers: HashSet<ProviderIdentity>,
    ) -> bool {
        let Some(entry) = self.entry.as_mut().filter(|entry| {
            entry.request.token == chosen.request.token
                && entry.choosing
                && !entry.applying
                && chosen.request.describes(store)
                && entry.providers == providers
                && providers.contains(&chosen.candidate.provider)
        }) else {
            return false;
        };
        entry.applying = true;
        true
    }

    pub(crate) fn close(&mut self, view: ViewId, token: u128) {
        let Some(entry) = self.entry.as_mut().filter(|entry| {
            entry.request.source == view
                && entry.request.token.as_u128() == token
                && !entry.applying
        }) else {
            return;
        };
        if entry.received && !entry.choosing {
            entry.menu_open = false;
            return;
        }
        self.clear();
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
        active: impl FnOnce(&Request) -> bool,
        providers: impl FnOnce(&Request) -> HashSet<ProviderIdentity>,
    ) {
        if self.entry.as_ref().is_some_and(|entry| {
            !active(&entry.request)
                || !entry.request.has_views(store)
                || (!entry.applying
                    && (entry.providers != providers(&entry.request)
                        || !entry.request.describes(store)))
        }) {
            self.clear();
        }
    }

    pub(crate) fn is_applying(&self) -> bool {
        self.entry.as_ref().is_some_and(|entry| entry.applying)
    }

    pub(crate) fn menu_open(&self, view: ViewId) -> bool {
        self.entry.as_ref().is_some_and(|entry| {
            entry.received
                && !entry.choosing
                && !entry.applying
                && entry.menu_open
                && entry.request.source == view
                && !entry.request.is_cancelled()
        })
    }

    pub(crate) fn request(&self) -> Option<&Request> {
        self.entry.as_ref().map(|entry| &entry.request)
    }
}

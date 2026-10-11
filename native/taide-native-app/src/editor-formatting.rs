use std::collections::{HashMap, HashSet};

use taide_lsp::native::Failure;
use taide_lsp::native::protocol::lsp_types;
use taide_model::ids::ProjectId;
use taide_native_editor::document::{DocumentSnapshot, EditorError};
use taide_native_editor::formatting::{Command, selection_ranges};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{SelectionSet, ViewId, ViewKey};
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
    OnType {
        position: lsp_types::Position,
        character: char,
    },
}

#[derive(Clone)]
pub struct Request {
    pub(crate) project: ProjectId,
    pub(crate) snapshot: DocumentSnapshot,
    pub(crate) kind: Kind,
    pub(crate) options: lsp_types::FormattingOptions,
    pub(crate) cancelled: watch::Receiver<bool>,
    pub(crate) command: Command,
    pub(crate) automatic: bool,
    source: ViewId,
    source_key: ViewKey,
    owner: ViewId,
    owner_key: ViewKey,
    owner_document: taide_native_editor::document::DocumentId,
    selection: SelectionSet,
    token: Uuid,
}

impl Request {
    pub(crate) fn enabled(&self, settings: &taide_model::settings::Settings) -> bool {
        if !self.automatic {
            return true;
        }
        match self.kind {
            Kind::OnType { .. } => settings.editor_format_on_type,
            Kind::Ranges(_) => settings.editor_format_on_paste,
            Kind::Document => false,
        }
    }

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
                    && (!self.automatic || source.selection.selections.len() == 1)
                    && (!matches!(self.kind, Kind::OnType { .. })
                        || source.selection.selections[0].anchor
                            == source.selection.selections[0].head)
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
        self.begin_with_input(
            store, project, source, owner, command, None, fallback, providers,
        )
    }

    pub(crate) fn begin_automatic(
        &mut self,
        store: &EditorStore,
        project: ProjectId,
        source: ViewId,
        owner: ViewId,
        input: &taide_native_ui::editor_formatting_input::Input,
        fallback: IndentOptions,
        providers: HashSet<ProviderIdentity>,
    ) -> Result<Option<Request>, EditorError> {
        self.begin_with_input(
            store,
            project,
            source,
            owner,
            Command::Selection,
            Some(input),
            fallback,
            providers,
        )
    }

    fn begin_with_input(
        &mut self,
        store: &EditorStore,
        project: ProjectId,
        source: ViewId,
        owner: ViewId,
        command: Command,
        input: Option<&taide_native_ui::editor_formatting_input::Input>,
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
        let kind = match input {
            Some(input) => {
                if input.document != snapshot.id
                    || input.revision != snapshot.revision
                    || source.selection.selections.len() != 1
                {
                    return Ok(None);
                }
                match &input.kind {
                    taide_native_ui::editor_formatting_input::Kind::Type {
                        character,
                        position,
                    } => {
                        let selection = source.selection.selections[0];
                        if selection.anchor != selection.head
                            || taide_native_editor::lsp::byte_to_position(
                                &snapshot,
                                selection.head,
                            )? != *position
                        {
                            return Ok(None);
                        }
                        Kind::OnType {
                            position: *position,
                            character: *character,
                        }
                    }
                    taide_native_ui::editor_formatting_input::Kind::Paste { range } => {
                        taide_native_editor::lsp::range_to_bytes(&snapshot, *range)?;
                        Kind::Ranges(vec![*range])
                    }
                }
            }
            None => match command {
                Command::Document => Kind::Document,
                Command::Selection => Kind::Ranges(selection_ranges(&snapshot, &source.selection)?),
            },
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
            automatic: input.is_some(),
            source: source.id,
            source_key: source.key.clone(),
            owner: owner.id,
            owner_key: owner.key.clone(),
            owner_document: owner.document,
            selection: source.selection.clone(),
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
        self.entries.remove(&request.source);
        taide_native_editor::formatting::apply_edits(
            store,
            &request.snapshot,
            Some(request.source),
            response.edits,
        )
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

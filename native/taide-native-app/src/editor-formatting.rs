use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use taide_lsp::native::Failure;
use taide_lsp::native::protocol::lsp_types;
use taide_model::ids::ProjectId;
use taide_native_editor::document::{DocumentSnapshot, EditorError};
use taide_native_editor::formatting::{Command, selection_ranges};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{SelectionSet, ViewId, ViewKey};
use tokio::sync::{oneshot, watch};
use uuid::Uuid;

use crate::editor_symbols::ProviderIdentity;

const JSON_WORKER_NAME: &str = "native-json-formatting";
const MAX_JSON_ACTIVE: usize = 1;

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
    pub(crate) native_json: bool,
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
    native: Option<oneshot::Receiver<Result<Vec<lsp_types::TextEdit>, EditorError>>>,
    complete: bool,
}

impl Drop for Entry {
    fn drop(&mut self) {
        self.cancel.send_replace(true);
    }
}

#[derive(Default)]
pub(crate) struct State {
    entries: HashMap<ViewId, Entry>,
    active_json: Arc<AtomicUsize>,
    context: eframe::egui::Context,
}

struct JsonSlot(Arc<AtomicUsize>, eframe::egui::Context);

impl Drop for JsonSlot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
        self.1.request_repaint();
    }
}

impl State {
    pub(crate) fn new(context: eframe::egui::Context) -> Self {
        Self {
            context,
            ..Default::default()
        }
    }

    pub(crate) fn tick_native(
        &mut self,
        tasks: &taide_runtime::TaskSupervisor,
    ) -> Vec<(Request, Result<Vec<lsp_types::TextEdit>, EditorError>)> {
        let mut results = Vec::new();
        for entry in self
            .entries
            .values_mut()
            .filter(|entry| entry.request.native_json && !entry.complete)
        {
            if let Some(receiver) = &mut entry.native {
                let result = match receiver.try_recv() {
                    Ok(result) => result,
                    Err(oneshot::error::TryRecvError::Empty) => continue,
                    Err(oneshot::error::TryRecvError::Closed) => Err(EditorError::StaleRevision),
                };
                entry.complete = true;
                results.push((entry.request.clone(), result));
                continue;
            }
            if self.active_json.load(Ordering::Acquire) >= MAX_JSON_ACTIVE {
                continue;
            }
            self.active_json.fetch_add(1, Ordering::AcqRel);
            let slot = JsonSlot(self.active_json.clone(), self.context.clone());
            let request = entry.request.clone();
            let (sender, receiver) = oneshot::channel();
            if tasks
                .spawn_blocking_transient_handle(JSON_WORKER_NAME, move || {
                    let _slot = slot;
                    if request.is_cancelled() {
                        return;
                    }
                    let result = crate::editor_json_formatting::build(
                        &request.snapshot,
                        &request.kind,
                        &request.options,
                    );
                    if !request.is_cancelled() {
                        drop(sender.send(result));
                    }
                })
                .is_some()
            {
                entry.native = Some(receiver);
            } else {
                entry.complete = true;
                results.push((entry.request.clone(), Err(EditorError::StaleRevision)));
            }
        }
        results
    }

    pub(crate) fn accept_native(
        &mut self,
        store: &mut EditorStore,
        request: &Request,
        providers: HashSet<ProviderIdentity>,
        result: Result<Vec<lsp_types::TextEdit>, EditorError>,
    ) -> Result<bool, EditorError> {
        let valid = self.entries.get(&request.source).is_some_and(|entry| {
            entry.request.token == request.token
                && entry.request.native_json
                && entry.providers.is_empty()
                && request.native_json
                && providers.is_empty()
                && request.describes(store)
        });
        if !valid {
            self.reject(request);
            return Ok(false);
        }
        self.entries.remove(&request.source);
        taide_native_editor::formatting::apply_edits(
            store,
            &request.snapshot,
            Some(request.source),
            result?,
        )
    }

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
        let native_json =
            providers.is_empty() && crate::editor_json_formatting::is_eligible(&snapshot);
        if source.composition.is_some() || providers.is_empty() && !native_json {
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
        if native_json && matches!(kind, Kind::OnType { .. }) {
            return Ok(None);
        }
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
            native_json,
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
                native: None,
                complete: false,
            },
        );
        if native_json {
            self.context.request_repaint();
        }
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

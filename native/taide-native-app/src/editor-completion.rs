use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Range;
use std::rc::Rc;

use taide_lsp::native::Failure;
use taide_model::ids::ProjectId;
use taide_native_editor::completion::Candidates;
use taide_native_editor::completion_model::Model;
use taide_native_editor::document::{DocumentId, DocumentSnapshot, EditorError};
use taide_native_editor::lsp::{LspRange, Position, byte_to_position};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{SelectionSet, ViewId, ViewKey};
use tokio::sync::watch;
use uuid::Uuid;

use crate::editor_symbols::ProviderIdentity;

#[path = "editor-completion-cache.rs"]
mod cache;
#[path = "editor-completion-colors.rs"]
mod colors;
#[path = "editor-completion-insertion.rs"]
mod insertion;
#[path = "editor-completion-provider.rs"]
mod provider;
#[path = "editor-completion-supply.rs"]
mod supply;
#[path = "editor-completion-variables.rs"]
mod variables;

pub(crate) use insertion::PreparationContext;
pub(crate) use provider::{Consumer, Provider};
pub(crate) use variables::Clock as SnippetClock;

pub(crate) struct Context {
    pub project: ProjectId,
    pub source: ViewId,
    pub owner: ViewId,
    pub word: Range<usize>,
    pub viewport: eframe::egui::ViewportId,
    pub automatic: bool,
}

#[derive(Clone)]
pub struct Request {
    pub(crate) project: ProjectId,
    pub(crate) snapshot: DocumentSnapshot,
    pub(crate) source: ViewId,
    pub(crate) source_key: ViewKey,
    pub(crate) owner: ViewId,
    pub(crate) owner_key: ViewKey,
    pub(crate) owner_document: DocumentId,
    pub(crate) selection: SelectionSet,
    pub(crate) position: Position,
    pub(crate) word: LspRange,
    pub(crate) replace_word: LspRange,
    pub(crate) token: Uuid,
    pub(crate) providers: HashSet<ProviderIdentity>,
    pub(crate) query_providers: HashSet<ProviderIdentity>,
    pub(crate) cancelled: watch::Receiver<bool>,
    pub(crate) viewport: eframe::egui::ViewportId,
    pub(crate) automatic: bool,
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
                    && source.selection == self.selection
                    && source.composition.is_none()
                    && store
                        .documents()
                        .snapshot(source.document)
                        .is_ok_and(|snapshot| {
                            snapshot.key == self.snapshot.key
                                && snapshot.revision == self.snapshot.revision
                                && snapshot.metadata.language_id
                                    == self.snapshot.metadata.language_id
                                && !snapshot.metadata.read_only
                        })
            })
    }

    pub(crate) fn is_active(
        &self,
        layout: &taide_model::layout::ProjectLayout,
        scope: &taide_native_ui::shell::WindowScope,
    ) -> bool {
        crate::symbol_sidebar::window_tree(&self.project, layout, scope).is_some_and(|(root, _)| {
            taide_native_ui::snapshot::active_tab(root, &self.owner_key.pane)
                .is_some_and(|tab| tab.id == self.owner_key.tab)
        })
    }
}

pub struct Group {
    pub provider: ProviderIdentity,
    pub ordinal: usize,
    pub candidates: Candidates,
}

pub struct Response {
    pub groups: Vec<Group>,
}

struct Entry {
    request: Request,
    cancel: watch::Sender<bool>,
    complete: bool,
    groups: BTreeMap<usize, Group>,
    snippets: Vec<taide_native_editor::completion::Candidate>,
    words: Vec<taide_native_editor::completion::Candidate>,
    word_result:
        Option<tokio::sync::oneshot::Receiver<Vec<taide_native_editor::completion::Candidate>>>,
    supplied: bool,
    needs_clipboard: bool,
    clipboard: Option<std::sync::Arc<String>>,
    clipboard_pending: bool,
    model: Option<Rc<RefCell<Model>>>,
    widget_token: Uuid,
    origin_position: Position,
    leading: String,
    delta: isize,
    queued: bool,
    documents: HashMap<usize, std::sync::Arc<taide_native_editor::documentation::RichDocument>>,
    colors: HashMap<usize, Option<eframe::egui::Color32>>,
    choice: Option<taide_native_editor::snippet_syntax::Index>,
    preview: Option<provider::Preview>,
}

impl Entry {
    fn update_clipboard_requirement(&mut self) {
        self.needs_clipboard = self
            .snippets
            .iter()
            .chain(
                self.groups
                    .values()
                    .flat_map(|group| &group.candidates.items),
            )
            .any(supply::needs_clipboard);
    }
}

impl Drop for Entry {
    fn drop(&mut self) {
        self.cancel.send_replace(true);
    }
}

#[derive(Default)]
pub(crate) struct State {
    entries: HashMap<ViewId, Entry>,
    commands: HashMap<ViewId, Vec<taide_native_editor::completion::Command>>,
    snippet_sessions: HashMap<ViewId, provider::Snippet>,
    code: crate::editor_documentation_code::Cache,
    images: crate::editor_documentation_images::Cache,
}

impl State {
    pub(crate) fn begin(
        &mut self,
        store: &EditorStore,
        context: Context,
        providers: HashSet<ProviderIdentity>,
    ) -> Result<Request, EditorError> {
        let source = store
            .views()
            .get(context.source)
            .ok_or(EditorError::NotFound)?;
        let owner = store
            .views()
            .get(context.owner)
            .ok_or(EditorError::NotFound)?;
        let snapshot = store.documents().snapshot(source.document)?;
        if snapshot.metadata.read_only {
            return Err(EditorError::ReadOnly);
        }
        if source.composition.is_some() {
            return Err(EditorError::Refused);
        }
        let byte = source
            .selection
            .selections
            .get(source.selection.primary)
            .ok_or(EditorError::InvalidBoundary)?
            .head;
        if context.word.start > byte || context.word.end < byte {
            return Err(EditorError::InvalidBoundary);
        }
        let position = byte_to_position(&snapshot, byte)?;
        let start = byte_to_position(&snapshot, context.word.start)?;
        let end = byte_to_position(&snapshot, context.word.end)?;
        if start.line != position.line || end.line != position.line {
            return Err(EditorError::InvalidBoundary);
        }
        let (cancel, cancelled) = watch::channel(false);
        let request = Request {
            project: context.project,
            snapshot,
            source: context.source,
            source_key: source.key.clone(),
            owner: context.owner,
            owner_key: owner.key.clone(),
            owner_document: owner.document,
            selection: source.selection.clone(),
            position,
            word: LspRange::new(start, position),
            replace_word: LspRange::new(start, end),
            token: Uuid::new_v4(),
            query_providers: providers.clone(),
            providers,
            cancelled,
            viewport: context.viewport,
            automatic: context.automatic,
        };
        self.entries.insert(
            request.source,
            Entry {
                request: request.clone(),
                cancel,
                complete: request.providers.is_empty(),
                groups: BTreeMap::new(),
                snippets: Vec::new(),
                words: Vec::new(),
                word_result: None,
                supplied: false,
                needs_clipboard: false,
                clipboard: None,
                clipboard_pending: false,
                model: None,
                widget_token: request.token,
                origin_position: request.position,
                leading: cache::leading(&request.snapshot, byte),
                delta: 0,
                queued: false,
                documents: HashMap::new(),
                colors: HashMap::new(),
                choice: None,
                preview: None,
            },
        );
        Ok(request)
    }

    pub(crate) fn is_current(&self, request: &Request) -> bool {
        !request.is_cancelled()
            && self
                .entries
                .get(&request.source)
                .is_some_and(|entry| entry.request.token == request.token)
    }

    pub(crate) fn accept(
        &mut self,
        store: &EditorStore,
        request: &Request,
        providers: HashSet<ProviderIdentity>,
        result: Result<Response, Failure>,
    ) -> Result<bool, Failure> {
        if !self.is_current(request) || !request.describes(store) {
            return Ok(false);
        }
        if request.providers != providers {
            self.close(request.source);
            return Ok(false);
        }
        let entry = self.entries.get_mut(&request.source).unwrap();
        entry.complete = true;
        entry
            .groups
            .retain(|_, group| !request.query_providers.contains(&group.provider));
        entry.model = None;
        entry.documents.clear();
        entry.colors.clear();
        let byte = taide_native_editor::lsp::position_to_byte(&request.snapshot, request.position)
            .map_err(|_| Failure::Cancelled)?;
        for group in entry.groups.values_mut() {
            cache::rebase_candidates(store, &request.snapshot, byte, &mut group.candidates.items)
                .map_err(|_| Failure::Cancelled)?;
        }
        cache::rebase_candidates(store, &request.snapshot, byte, &mut entry.snippets)
            .map_err(|_| Failure::Cancelled)?;
        cache::rebase_candidates(store, &request.snapshot, byte, &mut entry.words)
            .map_err(|_| Failure::Cancelled)?;
        entry.origin_position = request.position;
        entry.leading = cache::leading(&request.snapshot, byte);
        entry.delta = 0;
        entry.update_clipboard_requirement();
        let response = result?;
        let mut accepted = HashSet::new();
        for group in response.groups {
            if request.query_providers.contains(&group.provider) && accepted.insert(group.provider)
            {
                entry.groups.insert(group.ordinal, group);
            }
        }
        entry.update_clipboard_requirement();
        Ok(true)
    }

    pub(crate) fn request(&self, store: &EditorStore, view: ViewId) -> Option<&Request> {
        let request = &self.entries.get(&view)?.request;
        request.describes(store).then_some(request)
    }

    #[cfg(test)]
    pub(crate) fn groups(
        &self,
        store: &EditorStore,
        view: ViewId,
    ) -> Option<&BTreeMap<usize, Group>> {
        self.request(store, view)?;
        Some(&self.entries.get(&view)?.groups)
    }

    pub(crate) fn pending(&self, store: &EditorStore, view: ViewId) -> bool {
        self.request(store, view).is_some()
            && self.entries.get(&view).is_some_and(|entry| {
                !entry.complete
                    || (entry.needs_clipboard && entry.clipboard.is_none())
                    || (entry.word_result.is_some()
                        && entry.snippets.is_empty()
                        && entry
                            .groups
                            .values()
                            .all(|group| group.candidates.items.is_empty()))
            })
    }

    pub(crate) fn take_clipboard_requests(&mut self, store: &EditorStore) -> Vec<Request> {
        self.entries
            .values_mut()
            .filter_map(|entry| {
                if !entry.complete
                    || !entry.needs_clipboard
                    || entry.clipboard.is_some()
                    || entry.clipboard_pending
                    || !entry.request.describes(store)
                {
                    return None;
                }
                entry.clipboard_pending = true;
                Some(entry.request.clone())
            })
            .collect()
    }

    pub(crate) fn accept_clipboard(
        &mut self,
        store: &EditorStore,
        request: &Request,
        text: String,
    ) -> bool {
        if !self.is_current(request) || !request.describes(store) {
            return false;
        }
        let entry = self.entries.get_mut(&request.source).unwrap();
        if !entry.clipboard_pending {
            return false;
        }
        entry.clipboard_pending = false;
        entry.clipboard = Some(std::sync::Arc::new(text));
        true
    }

    pub(crate) fn clipboard(
        &self,
        store: &EditorStore,
        view: ViewId,
    ) -> Option<std::sync::Arc<String>> {
        self.request(store, view)?;
        self.entries.get(&view)?.clipboard.clone()
    }

    pub(crate) fn close(&mut self, view: ViewId) {
        self.entries.remove(&view);
        self.commands.remove(&view);
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.commands.clear();
        self.snippet_sessions.clear();
        self.images = crate::editor_documentation_images::Cache::default();
    }

    pub(crate) fn reconcile(
        &mut self,
        store: &EditorStore,
        projects: &HashSet<ProjectId>,
        mut active: impl FnMut(&Request) -> bool,
        mut providers: impl FnMut(&ProjectId, &DocumentSnapshot) -> HashSet<ProviderIdentity>,
    ) {
        self.commands
            .retain(|view, _| store.views().get(*view).is_some());
        self.entries.retain(|_, entry| {
            projects.contains(&entry.request.project)
                && active(&entry.request)
                && (entry.choice.is_some()
                    || providers(&entry.request.project, &entry.request.snapshot)
                        == entry.request.providers)
                && (entry.request.describes(store) || cache::refresh(entry, store).unwrap_or(false))
        });
        self.snippet_sessions.retain(|view, snippet| {
            projects.contains(&snippet.origin.project)
                && active(&snippet.origin)
                && store
                    .views()
                    .get(snippet.origin.owner)
                    .is_some_and(|owner| {
                        owner.key == snippet.origin.owner_key
                            && owner.document == snippet.origin.owner_document
                    })
                && store.views().get(*view).is_some_and(|source| {
                    source.key == snippet.origin.source_key
                        && source.document == snippet.origin.snapshot.id
                        && store
                            .documents()
                            .snapshot(source.document)
                            .is_ok_and(|document| {
                                document.key == snippet.origin.snapshot.key
                                    && document.metadata.language_id
                                        == snippet.origin.snapshot.metadata.language_id
                            })
                })
                && snippet.session.synchronize(store)
        });
        self.entries.retain(|view, entry| {
            entry.choice.is_none_or(|choice| {
                self.snippet_sessions.get(view).is_some_and(|snippet| {
                    snippet
                        .session
                        .active_choice(store)
                        .is_ok_and(|active| active.is_some_and(|active| active.index == choice))
                })
            })
        });
    }
}

#[cfg(test)]
#[path = "editor-completion-tests.rs"]
mod tests;

#[cfg(test)]
#[path = "editor-completion-consumer-tests.rs"]
mod consumer_tests;

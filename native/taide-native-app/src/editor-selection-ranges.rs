use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use eframe::egui;
use taide_lsp::native::Failure;
use taide_lsp::native::feature::FeatureRequest;
use taide_lsp::native::protocol::lsp_types;
use taide_model::file::FileSizeTier;
use taide_model::ids::ProjectId;
use taide_native_editor::cursor_commands::{
    CursorCommand, clear_smart_selection, has_smart_selection, run_cursor_command,
    run_smart_selection_with_ranges,
};
use taide_native_editor::document::{DocumentId, DocumentKey, DocumentSnapshot, EditorError};
use taide_native_editor::line_commands::LineCommandContext;
use taide_native_editor::lsp::{Position, byte_to_position, position_to_byte};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{SelectionSet, ViewId, ViewKey};
use tokio::sync::oneshot;
use tokio::sync::watch;
use uuid::Uuid;

use crate::editor_symbols::ProviderIdentity;

const JSON_WORKER_NAME: &str = "native-json-selection";
const MAX_JSON_ACTIVE: usize = 1;
const JSON_LIMITS: taide_native_editor::json_selection::Limits =
    taide_native_editor::json_selection::Limits {
        parser: crate::editor_json::PARSER_LIMITS,
        max_positions: 1024,
        max_ranges: 256 * 1024,
    };

pub(crate) type ProviderRanges =
    <lsp_types::request::SelectionRangeRequest as FeatureRequest>::Reply;

#[derive(Clone)]
pub struct Request {
    pub(crate) project: Option<ProjectId>,
    pub(crate) snapshot: DocumentSnapshot,
    pub(crate) selection: SelectionSet,
    pub(crate) source: ViewId,
    pub(crate) source_key: ViewKey,
    pub(crate) owner: ViewId,
    pub(crate) owner_key: ViewKey,
    pub(crate) owner_document: DocumentId,
    pub(crate) viewport: egui::ViewportId,
    pub(crate) providers: HashSet<ProviderIdentity>,
    pub(crate) token: Uuid,
    pub(crate) cancelled: watch::Receiver<bool>,
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
                    && store
                        .documents()
                        .snapshot(source.document)
                        .is_ok_and(|current| {
                            current.key == self.snapshot.key
                                && current.revision == self.snapshot.revision
                                && current.metadata.language_id
                                    == self.snapshot.metadata.language_id
                                && current.metadata.tier == self.snapshot.metadata.tier
                        })
            })
    }

    pub(crate) fn is_active(
        &self,
        project: &ProjectId,
        layout: &taide_model::layout::ProjectLayout,
        scope: &taide_native_ui::shell::WindowScope,
    ) -> bool {
        self.project.as_ref().is_none_or(|owner| owner == project)
            && crate::symbol_sidebar::window_tree(project, layout, scope).is_some_and(
                |(root, _)| {
                    taide_native_ui::snapshot::active_tab(root, &self.owner_key.pane)
                        .is_some_and(|tab| tab.id == self.owner_key.tab)
                },
            )
    }
}

pub struct Response {
    pub(crate) groups: Vec<(ProviderIdentity, ProviderRanges)>,
}

pub(crate) struct Context {
    pub project: Option<ProjectId>,
    pub owner: ViewId,
    pub source: ViewId,
    pub viewport: egui::ViewportId,
}

struct Queued {
    request: Request,
    cancel: watch::Sender<bool>,
    forward: bool,
}

enum Phase {
    Pending,
    Ready(Vec<Vec<Range<usize>>>),
    Complete,
}

enum Native {
    Pending(Option<oneshot::Receiver<Vec<Vec<Range<usize>>>>>),
    Ready(Vec<Vec<Range<usize>>>),
}

struct Entry {
    request: Request,
    cancel: watch::Sender<bool>,
    expected: SelectionSet,
    forward: bool,
    phase: Phase,
    native: Option<Native>,
}

impl Drop for Entry {
    fn drop(&mut self) {
        self.cancel.send_replace(true);
    }
}

#[derive(Default)]
pub(crate) struct State {
    entries: HashMap<(egui::ViewportId, ViewId), Entry>,
    queued: Vec<Queued>,
    active_json: Arc<AtomicUsize>,
}

struct JsonSlot(Arc<AtomicUsize>, egui::Context);

impl Drop for JsonSlot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
        self.1.request_repaint();
    }
}

fn is_json(snapshot: &DocumentSnapshot) -> bool {
    snapshot.metadata.language_id == "json"
        && snapshot.metadata.tier == FileSizeTier::Normal
        && !matches!(snapshot.key, DocumentKey::AppFile(_))
}

fn json_ranges(request: &Request) -> Result<Vec<Vec<Range<usize>>>, EditorError> {
    if request.snapshot.rope.len_bytes() > JSON_LIMITS.parser.scanner.max_bytes
        || request.selection.selections.len() > JSON_LIMITS.max_positions
    {
        return Err(EditorError::Capacity);
    }
    let text = request.snapshot.rope.to_string();
    let positions = request
        .selection
        .selections
        .iter()
        .map(|selection| selection.head)
        .collect::<Vec<_>>();
    Ok(
        taide_native_editor::json_selection::selection_ranges(&text, &positions, JSON_LIMITS)?
            .into_iter()
            .map(|ranges| ranges.into_iter().map(|range| range.bytes).collect())
            .collect(),
    )
}

fn position(snapshot: &DocumentSnapshot, value: lsp_types::Position) -> Option<usize> {
    if value.line as usize >= snapshot.rope.len_lines() {
        return None;
    }
    let original = Position {
        line: value.line,
        character: value.character,
    };
    let byte = position_to_byte(snapshot, original).ok()?;
    (byte_to_position(snapshot, byte).ok()? == original).then_some(byte)
}

fn ranges(request: &Request, response: Response) -> Vec<Vec<Range<usize>>> {
    let mut ranges = vec![Vec::new(); request.selection.selections.len()];
    for (provider, provided) in response.groups {
        if !request.providers.contains(&provider) {
            continue;
        }
        let Some(provided) = provided.filter(|provided| provided.len() == ranges.len()) else {
            continue;
        };
        for ((ranges, selection), mut node) in ranges
            .iter_mut()
            .zip(&request.selection.selections)
            .zip(provided)
        {
            while let Some(current) = node {
                let start = position(&request.snapshot, current.range.start);
                let end = position(&request.snapshot, current.range.end);
                if let Some((start, end)) = start.zip(end)
                    && start <= selection.head
                    && selection.head <= end
                {
                    ranges.push(start..end);
                }
                node = current.parent.map(|parent| *parent);
            }
        }
    }
    ranges
}

impl State {
    pub(crate) fn tick_native(
        &mut self,
        store: &EditorStore,
        tasks: &taide_runtime::TaskSupervisor,
        context: &egui::Context,
    ) {
        for entry in self.entries.values_mut() {
            let Some(Native::Pending(receiver)) = &mut entry.native else {
                continue;
            };
            if !entry.request.describes(store)
                || store
                    .views()
                    .get(entry.request.source)
                    .is_none_or(|view| view.selection != entry.expected)
            {
                continue;
            }
            if let Some(receiver) = receiver {
                let result = match receiver.try_recv() {
                    Ok(ranges) => ranges,
                    Err(oneshot::error::TryRecvError::Empty) => continue,
                    Err(oneshot::error::TryRecvError::Closed) => {
                        vec![Vec::new(); entry.expected.selections.len()]
                    }
                };
                entry.native = Some(Native::Ready(result));
                context.request_repaint();
                continue;
            }
            if self.active_json.load(Ordering::Acquire) >= MAX_JSON_ACTIVE {
                continue;
            }
            self.active_json.fetch_add(1, Ordering::AcqRel);
            let slot = JsonSlot(self.active_json.clone(), context.clone());
            let request = entry.request.clone();
            let (sender, pending) = oneshot::channel();
            if tasks
                .spawn_blocking_transient_handle(JSON_WORKER_NAME, move || {
                    let _slot = slot;
                    if request.is_cancelled() {
                        return;
                    }
                    let ranges = json_ranges(&request)
                        .unwrap_or_else(|_| vec![Vec::new(); request.selection.selections.len()]);
                    if !request.is_cancelled() {
                        drop(sender.send(ranges));
                    }
                })
                .is_some()
            {
                *receiver = Some(pending);
            } else {
                entry.native = Some(Native::Ready(vec![
                    Vec::new();
                    entry.expected.selections.len()
                ]));
            }
        }
    }

    pub(crate) fn queue(
        &mut self,
        store: &EditorStore,
        context: Context,
        forward: bool,
    ) -> Result<(), EditorError> {
        let source = store
            .views()
            .get(context.source)
            .ok_or(EditorError::NotFound)?;
        let owner = store
            .views()
            .get(context.owner)
            .ok_or(EditorError::NotFound)?;
        if source.composition.is_some() {
            return Ok(());
        }
        let snapshot = store.documents().snapshot(source.document)?;
        let (cancel, cancelled) = watch::channel(false);
        self.queued.push(Queued {
            request: Request {
                project: context.project,
                snapshot,
                selection: source.selection.clone(),
                source: context.source,
                source_key: source.key.clone(),
                owner: context.owner,
                owner_key: owner.key.clone(),
                owner_document: owner.document,
                viewport: context.viewport,
                providers: HashSet::new(),
                token: Uuid::new_v4(),
                cancelled,
            },
            cancel,
            forward,
        });
        Ok(())
    }

    fn invalidate(&mut self, store: &mut EditorStore, key: (egui::ViewportId, ViewId)) {
        if self.entries.remove(&key).is_some() {
            let _ = clear_smart_selection(store, key.1);
        }
    }

    pub(crate) fn before(
        &mut self,
        store: &mut EditorStore,
        context: Context,
        providers: HashSet<ProviderIdentity>,
        rules: LineCommandContext<'_>,
    ) -> Result<Vec<Request>, EditorError> {
        let key = (context.viewport, context.source);
        self.after(store, context.viewport, context.source);
        if self.entries.get(&key).is_some_and(|entry| {
            entry.request.providers != providers
                || entry.request.owner != context.owner
                || entry.request.project != context.project
        }) {
            self.invalidate(store, key);
        }
        if let Some(entry) = self.entries.get_mut(&key)
            && let Phase::Ready(ranges) = &entry.phase
            && !matches!(entry.native, Some(Native::Pending(_)))
        {
            let mut combined = ranges.clone();
            if let Some(Native::Ready(native)) = &entry.native {
                for (ranges, native) in combined.iter_mut().zip(native) {
                    ranges.extend(native.iter().cloned());
                }
            }
            run_smart_selection_with_ranges(
                store,
                context.source,
                entry.forward,
                &combined,
                rules,
            )?;
            entry.expected = store
                .views()
                .get(context.source)
                .ok_or(EditorError::NotFound)?
                .selection
                .clone();
            entry.phase = Phase::Complete;
        }
        let initial = store
            .views()
            .get(context.source)
            .ok_or(EditorError::NotFound)?
            .selection
            .clone();
        let queued = self
            .queued
            .extract_if(.., |queued| {
                queued.request.viewport == context.viewport
                    && queued.request.source == context.source
            })
            .collect::<Vec<_>>();
        let mut submitted = Vec::new();
        for mut queued in queued {
            if !queued.request.describes(store) || queued.request.selection != initial {
                continue;
            }
            if has_smart_selection(store, context.source) {
                run_cursor_command(
                    store,
                    context.source,
                    if queued.forward {
                        CursorCommand::Expand
                    } else {
                        CursorCommand::Shrink
                    },
                    rules,
                )?;
                if let Some(entry) = self.entries.get_mut(&key) {
                    entry.expected = store
                        .views()
                        .get(context.source)
                        .ok_or(EditorError::NotFound)?
                        .selection
                        .clone();
                }
                continue;
            }
            if self.entries.get(&key).is_some_and(|entry| {
                matches!(entry.phase, Phase::Pending)
                    || matches!(entry.native, Some(Native::Pending(_)))
            }) {
                continue;
            }
            let native_json = is_json(&queued.request.snapshot);
            let external = !providers.is_empty()
                && context.project.is_some()
                && queued.request.snapshot.metadata.tier == FileSizeTier::Normal
                && matches!(queued.request.snapshot.key, DocumentKey::File(_));
            if !native_json && !external {
                run_cursor_command(
                    store,
                    context.source,
                    if queued.forward {
                        CursorCommand::Expand
                    } else {
                        CursorCommand::Shrink
                    },
                    rules,
                )?;
                continue;
            }
            queued.request.providers = providers.clone();
            let entry = Entry {
                expected: initial.clone(),
                request: queued.request.clone(),
                cancel: queued.cancel,
                forward: queued.forward,
                phase: if external {
                    Phase::Pending
                } else {
                    Phase::Ready(vec![Vec::new(); initial.selections.len()])
                },
                native: native_json.then_some(Native::Pending(None)),
            };
            if external {
                submitted.push(queued.request);
            }
            self.entries.insert(key, entry);
        }
        Ok(submitted)
    }

    pub(crate) fn accept(
        &mut self,
        store: &EditorStore,
        request: &Request,
        providers: HashSet<ProviderIdentity>,
        result: Result<Response, Failure>,
    ) -> bool {
        let Some(entry) = self.entries.get_mut(&(request.viewport, request.source)) else {
            return false;
        };
        if entry.request.token != request.token
            || !request.describes(store)
            || request.providers != providers
            || !matches!(entry.phase, Phase::Pending)
            || store
                .views()
                .get(request.source)
                .is_none_or(|source| source.selection != entry.expected)
        {
            return false;
        }
        entry.phase = Phase::Ready(ranges(
            request,
            result.unwrap_or(Response { groups: Vec::new() }),
        ));
        true
    }

    pub(crate) fn after(
        &mut self,
        store: &mut EditorStore,
        viewport: egui::ViewportId,
        source: ViewId,
    ) {
        let key = (viewport, source);
        if self.entries.get(&key).is_some_and(|entry| {
            !entry.request.describes(store)
                || store
                    .views()
                    .get(source)
                    .is_none_or(|view| view.selection != entry.expected)
        }) {
            self.invalidate(store, key);
        }
    }

    pub(crate) fn reconcile(
        &mut self,
        store: &mut EditorStore,
        valid: impl Fn(&Request) -> bool,
        providers: impl Fn(&ProjectId, &DocumentSnapshot) -> HashSet<ProviderIdentity>,
    ) {
        let retired = self
            .entries
            .iter()
            .filter_map(|(key, entry)| {
                let current = entry
                    .request
                    .project
                    .as_ref()
                    .map_or_else(HashSet::new, |project| {
                        providers(project, &entry.request.snapshot)
                    });
                (!valid(&entry.request)
                    || !entry.request.describes(store)
                    || current != entry.request.providers
                    || store
                        .views()
                        .get(key.1)
                        .is_none_or(|view| view.selection != entry.expected))
                .then_some(*key)
            })
            .collect::<Vec<_>>();
        for key in retired {
            self.invalidate(store, key);
        }
        self.queued
            .retain(|queued| valid(&queued.request) && queued.request.describes(store));
    }

    pub(crate) fn clear(&mut self, store: &mut EditorStore) {
        let keys = self.entries.keys().copied().collect::<Vec<_>>();
        for key in keys {
            self.invalidate(store, key);
        }
        self.queued.clear();
    }
}

#[derive(Clone)]
pub(crate) struct Consumer<'a, 'state> {
    pub state: Rc<RefCell<&'state mut State>>,
    pub lsp: Option<&'a crate::lsp::LspBridge>,
    pub context: egui::Context,
}

impl Consumer<'_, '_> {
    pub(crate) fn queue(
        &self,
        store: &EditorStore,
        project: Option<&ProjectId>,
        owner: ViewId,
        source: ViewId,
        forward: bool,
    ) -> Result<(), EditorError> {
        self.state.borrow_mut().queue(
            store,
            Context {
                project: project.cloned(),
                owner,
                source,
                viewport: self.context.viewport_id(),
            },
            forward,
        )?;
        self.context.request_repaint();
        Ok(())
    }

    pub(crate) fn before(
        &self,
        store: &mut EditorStore,
        project: Option<&ProjectId>,
        owner: ViewId,
        source: ViewId,
        rules: LineCommandContext<'_>,
    ) -> Result<(), EditorError> {
        let providers = project
            .zip(self.lsp)
            .and_then(|(project, lsp)| {
                store
                    .views()
                    .get(source)
                    .and_then(|view| store.documents().snapshot(view.document).ok())
                    .map(|snapshot| lsp.selection_range_providers(project, &snapshot))
            })
            .unwrap_or_default();
        let requests = self.state.borrow_mut().before(
            store,
            Context {
                project: project.cloned(),
                owner,
                source,
                viewport: self.context.viewport_id(),
            },
            providers.clone(),
            rules,
        )?;
        for request in requests {
            if self
                .lsp
                .is_none_or(|lsp| lsp.selection_ranges(request.clone()).is_err())
            {
                self.state.borrow_mut().accept(
                    store,
                    &request,
                    providers.clone(),
                    Err(Failure::TransportClosed),
                );
                self.context.request_repaint();
            }
        }
        Ok(())
    }

    pub(crate) fn after(&self, store: &mut EditorStore, source: ViewId) {
        self.state
            .borrow_mut()
            .after(store, self.context.viewport_id(), source);
    }
}

#[cfg(test)]
#[path = "editor-selection-ranges-tests.rs"]
mod tests;

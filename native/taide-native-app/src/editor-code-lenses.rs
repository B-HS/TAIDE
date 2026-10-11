use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui;
use taide_lsp::native::Failure;
use taide_lsp::native::protocol::lsp_types;
use taide_model::error::AppResult;
use taide_model::file::FileSizeTier;
use taide_model::ids::ProjectId;
use taide_model::theme::{ResolvedTheme, ThemeType};
use taide_native_editor::document::{DocumentKey, DocumentSnapshot, EditorError};
use taide_native_editor::lsp::{Position, byte_to_position, position_to_byte};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{ViewId, ViewKey};
use taide_native_ui::editor_code_lenses::{Colors, Item, Output, Presentation};
use tokio::sync::watch;
use uuid::Uuid;

use crate::editor_symbols::ProviderIdentity;

const REQUEST_DELAY: Duration = Duration::from_millis(250);
const REFRESH_DELAY: Duration = Duration::from_millis(300);
const DARK_FOREGROUND: egui::Color32 = egui::Color32::from_rgb(0x99, 0x99, 0x99);
const LIGHT_FOREGROUND: egui::Color32 = egui::Color32::from_rgb(0x91, 0x91, 0x91);
const DARK_HOVER: egui::Color32 = egui::Color32::from_rgb(0x4e, 0x94, 0xce);

pub(crate) fn colors(theme: &ResolvedTheme) -> AppResult<Colors> {
    let color = |key: &str, fallback| {
        theme.colors.get(key).map_or(Ok(fallback), |value| {
            taide_native_ui::presentation::parse_color(value, key)
        })
    };
    let dark = theme.theme_type == ThemeType::Dark;
    Ok(Colors {
        foreground: color(
            "editorCodeLens.foreground",
            if dark {
                DARK_FOREGROUND
            } else {
                LIGHT_FOREGROUND
            },
        )?,
        hover: color(
            "editorLink.activeForeground",
            if dark {
                DARK_HOVER
            } else {
                egui::Color32::BLUE
            },
        )?,
    })
}

#[derive(Clone)]
pub struct Request {
    pub(crate) project: ProjectId,
    pub(crate) snapshot: DocumentSnapshot,
    pub(crate) source: ViewId,
    pub(crate) source_key: ViewKey,
    pub(crate) owner: ViewId,
    pub(crate) owner_key: ViewKey,
    pub(crate) owner_document: taide_native_editor::document::DocumentId,
    pub(crate) viewport: egui::ViewportId,
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
                                && current.metadata.tier == FileSizeTier::Normal
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

pub struct Response {
    pub(crate) groups: Vec<(ProviderIdentity, bool, Vec<lsp_types::CodeLens>)>,
}

#[derive(Clone)]
pub struct ResolveRequest {
    pub(crate) request: Request,
    pub(crate) provider: ProviderIdentity,
    pub(crate) index: usize,
    pub(crate) lens: lsp_types::CodeLens,
}

#[derive(Clone)]
pub struct ExecuteRequest {
    pub(crate) request: Request,
    pub(crate) provider: ProviderIdentity,
    pub(crate) index: usize,
    pub(crate) command: lsp_types::Command,
}

struct Lens {
    provider: ProviderIdentity,
    raw: lsp_types::CodeLens,
    anchor: usize,
    resolve: ResolveState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ResolveState {
    Unresolved,
    Pending,
    Finished,
}

struct Entry {
    request: Request,
    providers: HashSet<ProviderIdentity>,
    cancel: watch::Sender<bool>,
    due: Instant,
    resolve_due: Instant,
    submitted: bool,
    received: bool,
    executing: bool,
    lenses: Vec<Lens>,
    items: Arc<[Item]>,
}

impl Drop for Entry {
    fn drop(&mut self) {
        self.cancel.send_replace(true);
    }
}

struct Picker {
    key: (egui::ViewportId, ViewId),
    token: Uuid,
    line: usize,
    generation: u64,
    indices: Vec<usize>,
    shown: bool,
}

pub(crate) struct PickerItems {
    pub generation: u64,
    pub titles: Vec<String>,
}

#[derive(Default)]
pub(crate) struct State {
    entries: HashMap<(egui::ViewportId, ViewId), Entry>,
    picker: Option<Picker>,
    generation: u64,
}

pub(crate) struct Context {
    pub project: ProjectId,
    pub owner: ViewId,
    pub source: ViewId,
    pub viewport: egui::ViewportId,
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

fn command(lens: &lsp_types::CodeLens) -> Option<&lsp_types::Command> {
    lens.command
        .as_ref()
        .filter(|command| !command.command.is_empty())
}

fn rebuild(entry: &mut Entry) {
    entry.items = entry
        .lenses
        .iter()
        .map(|lens| Item {
            anchor: lens.anchor,
            title: command(&lens.raw).map(|command| command.title.clone()),
        })
        .collect::<Vec<_>>()
        .into();
}

impl State {
    #[cfg(test)]
    pub(crate) fn request(&self, viewport: egui::ViewportId, view: ViewId) -> Option<&Request> {
        self.entries
            .get(&(viewport, view))
            .map(|entry| &entry.request)
    }

    pub(crate) fn observe(
        &mut self,
        store: &EditorStore,
        context: Context,
        providers: HashSet<ProviderIdentity>,
        enabled: bool,
        now: Instant,
    ) -> Result<Option<Request>, EditorError> {
        let key = (context.viewport, context.source);
        let source = store
            .views()
            .get(context.source)
            .ok_or(EditorError::NotFound)?;
        let owner = store
            .views()
            .get(context.owner)
            .ok_or(EditorError::NotFound)?;
        let snapshot = store.documents().snapshot(source.document)?;
        if !enabled
            || source.composition.is_some()
            || providers.is_empty()
            || snapshot.metadata.tier != FileSizeTier::Normal
            || !matches!(snapshot.key, DocumentKey::File(_))
        {
            self.entries.remove(&key);
            return Ok(None);
        }
        let unchanged = self.entries.get(&key).is_some_and(|entry| {
            entry.request.describes(store)
                && entry.request.project == context.project
                && entry.request.owner == context.owner
                && entry.providers == providers
        });
        if !unchanged {
            let (cancel, cancelled) = watch::channel(false);
            self.entries.insert(
                key,
                Entry {
                    request: Request {
                        project: context.project,
                        snapshot,
                        source: context.source,
                        source_key: source.key.clone(),
                        owner: context.owner,
                        owner_key: owner.key.clone(),
                        owner_document: owner.document,
                        viewport: context.viewport,
                        token: Uuid::new_v4(),
                        cancelled,
                    },
                    providers,
                    cancel,
                    due: now + REQUEST_DELAY,
                    resolve_due: now + REQUEST_DELAY,
                    submitted: false,
                    received: false,
                    executing: false,
                    lenses: Vec::new(),
                    items: Arc::from([]),
                },
            );
        }
        let entry = self.entries.get_mut(&key).ok_or(EditorError::NotFound)?;
        if entry.submitted || now < entry.due {
            return Ok(None);
        }
        entry.submitted = true;
        Ok(Some(entry.request.clone()))
    }

    pub(crate) fn accept(
        &mut self,
        store: &EditorStore,
        request: &Request,
        providers: HashSet<ProviderIdentity>,
        response: Result<Response, Failure>,
        now: Instant,
    ) -> bool {
        let Some(entry) = self.entries.get_mut(&(request.viewport, request.source)) else {
            return false;
        };
        if entry.request.token != request.token
            || !request.describes(store)
            || entry.providers != providers
        {
            return false;
        }
        let mut lenses = response
            .ok()
            .into_iter()
            .flat_map(|response| response.groups)
            .enumerate()
            .filter(|(_, (provider, _, _))| providers.contains(provider))
            .flat_map(|(order, (provider, can_resolve, lenses))| {
                lenses.into_iter().filter_map(move |lens| {
                    let start = position(&request.snapshot, lens.range.start)?;
                    let end = position(&request.snapshot, lens.range.end)?;
                    if start > end {
                        return None;
                    }
                    let resolve = if command(&lens).is_some() || !can_resolve {
                        ResolveState::Finished
                    } else {
                        ResolveState::Unresolved
                    };
                    Some((
                        order,
                        Lens {
                            provider,
                            raw: lens,
                            anchor: start,
                            resolve,
                        },
                    ))
                })
            })
            .collect::<Vec<_>>();
        lenses.sort_by_key(|(order, lens)| {
            (
                lens.raw.range.start.line,
                *order,
                lens.raw.range.start.character,
            )
        });
        entry.lenses = lenses.into_iter().map(|(_, lens)| lens).collect();
        entry.received = true;
        entry.resolve_due = now + REQUEST_DELAY;
        rebuild(entry);
        true
    }

    pub(crate) fn display(
        &self,
        store: &EditorStore,
        viewport: egui::ViewportId,
        source: ViewId,
        colors: Colors,
    ) -> Option<Presentation> {
        let entry = self.entries.get(&(viewport, source))?;
        entry.request.describes(store).then(|| Presentation {
            document: entry.request.snapshot.id,
            revision: entry.request.snapshot.revision,
            items: entry.items.clone(),
            colors,
        })
    }

    pub(crate) fn resolve_visible(
        &mut self,
        store: &EditorStore,
        viewport: egui::ViewportId,
        source: ViewId,
        visible: &[usize],
        now: Instant,
    ) -> Vec<ResolveRequest> {
        let key = (viewport, source);
        let picker_line = self
            .picker
            .as_ref()
            .filter(|picker| picker.key == key)
            .map(|picker| picker.line);
        let Some(entry) = self.entries.get_mut(&key) else {
            return Vec::new();
        };
        if !entry.received || !entry.request.describes(store) || now < entry.resolve_due {
            return Vec::new();
        }
        entry
            .lenses
            .iter_mut()
            .enumerate()
            .filter_map(|(index, lens)| {
                if lens.resolve != ResolveState::Unresolved
                    || (!visible.contains(&index)
                        && picker_line != Some(lens.raw.range.start.line as usize))
                {
                    return None;
                }
                lens.resolve = ResolveState::Pending;
                Some(ResolveRequest {
                    request: entry.request.clone(),
                    provider: lens.provider,
                    index,
                    lens: lens.raw.clone(),
                })
            })
            .collect()
    }

    pub(crate) fn resolved(
        &mut self,
        store: &EditorStore,
        request: &ResolveRequest,
        result: Result<lsp_types::CodeLens, Failure>,
    ) -> bool {
        let Some(entry) = self
            .entries
            .get_mut(&(request.request.viewport, request.request.source))
        else {
            return false;
        };
        if entry.request.token != request.request.token || !request.request.describes(store) {
            return false;
        }
        let Some(lens) = entry.lenses.get_mut(request.index).filter(|lens| {
            lens.provider == request.provider && lens.resolve == ResolveState::Pending
        }) else {
            return false;
        };
        lens.resolve = ResolveState::Finished;
        if let Ok(resolved) = result {
            lens.raw.command = resolved
                .command
                .filter(|command| !command.command.is_empty());
        }
        rebuild(entry);
        true
    }

    pub(crate) fn execute(
        &mut self,
        store: &EditorStore,
        viewport: egui::ViewportId,
        source: ViewId,
        index: usize,
    ) -> Option<ExecuteRequest> {
        let entry = self.entries.get_mut(&(viewport, source))?;
        if entry.executing || !entry.request.describes(store) {
            return None;
        }
        let lens = entry.lenses.get(index)?;
        let command = command(&lens.raw)?.clone();
        entry.executing = true;
        Some(ExecuteRequest {
            request: entry.request.clone(),
            provider: lens.provider,
            index,
            command,
        })
    }

    pub(crate) fn executed(&mut self, store: &EditorStore, request: &ExecuteRequest) -> bool {
        let Some(entry) = self
            .entries
            .get_mut(&(request.request.viewport, request.request.source))
        else {
            return false;
        };
        if entry.request.token != request.request.token || !request.request.describes(store) {
            return false;
        }
        if entry.lenses.get(request.index).is_none_or(|lens| {
            lens.provider != request.provider || command(&lens.raw) != Some(&request.command)
        }) {
            return false;
        }
        entry.executing = false;
        true
    }

    pub(crate) fn refresh(
        &mut self,
        project: &ProjectId,
        provider: ProviderIdentity,
        now: Instant,
    ) {
        for entry in self.entries.values_mut().filter(|entry| {
            entry.request.project == *project && entry.providers.contains(&provider)
        }) {
            entry.cancel.send_replace(true);
            let (cancel, cancelled) = watch::channel(false);
            entry.cancel = cancel;
            entry.request.cancelled = cancelled;
            entry.request.token = Uuid::new_v4();
            entry.submitted = false;
            entry.received = false;
            entry.executing = false;
            entry.due = now + REFRESH_DELAY + REQUEST_DELAY;
        }
    }

    pub(crate) fn begin_picker(
        &mut self,
        store: &EditorStore,
        viewport: egui::ViewportId,
        source: ViewId,
    ) -> bool {
        let Some(entry) = self
            .entries
            .get(&(viewport, source))
            .filter(|entry| entry.request.describes(store))
        else {
            return false;
        };
        let Some(view) = store.views().get(source) else {
            return false;
        };
        let head = view.selection.selections[view.selection.primary].head;
        self.generation += 1;
        self.picker = Some(Picker {
            key: (viewport, source),
            token: entry.request.token,
            line: entry.request.snapshot.rope.byte_to_line(head),
            generation: self.generation,
            indices: Vec::new(),
            shown: false,
        });
        true
    }

    pub(crate) fn picker(&mut self, store: &EditorStore) -> Option<PickerItems> {
        let picker = self.picker.as_mut()?;
        let Some(entry) = self
            .entries
            .get(&picker.key)
            .filter(|entry| entry.request.token == picker.token && entry.request.describes(store))
        else {
            self.picker = None;
            return None;
        };
        if picker.shown || !entry.received {
            return None;
        }
        if entry.lenses.iter().any(|lens| {
            lens.raw.range.start.line as usize == picker.line
                && lens.resolve != ResolveState::Finished
        }) {
            return None;
        }
        picker.shown = true;
        picker.indices = entry
            .lenses
            .iter()
            .enumerate()
            .filter(|(_, lens)| {
                lens.raw.range.start.line as usize == picker.line && command(&lens.raw).is_some()
            })
            .map(|(index, _)| index)
            .collect();
        if picker.indices.is_empty() {
            return None;
        }
        Some(PickerItems {
            generation: picker.generation,
            titles: picker
                .indices
                .iter()
                .filter_map(|index| {
                    command(&entry.lenses[*index].raw).map(|command| command.title.clone())
                })
                .collect(),
        })
    }

    pub(crate) fn choose(
        &mut self,
        store: &EditorStore,
        generation: u64,
        index: usize,
    ) -> Option<ExecuteRequest> {
        let picker = self
            .picker
            .take()
            .filter(|picker| picker.generation == generation)?;
        let item = *picker.indices.get(index)?;
        let entry = self.entries.get(&picker.key)?;
        if entry.request.token != picker.token {
            return None;
        }
        self.execute(store, picker.key.0, picker.key.1, item)
    }

    pub(crate) fn picker_is_current(&self, store: &EditorStore) -> bool {
        self.picker.as_ref().is_some_and(|picker| {
            self.entries.get(&picker.key).is_some_and(|entry| {
                entry.request.token == picker.token && entry.request.describes(store)
            })
        })
    }

    pub(crate) fn cancel_picker(&mut self) {
        self.picker = None;
    }

    pub(crate) fn reconcile(
        &mut self,
        store: &EditorStore,
        active: impl Fn(&Request) -> bool,
        providers: impl Fn(&ProjectId, &DocumentSnapshot) -> HashSet<ProviderIdentity>,
    ) {
        self.entries.retain(|_, entry| {
            entry.request.describes(store)
                && active(&entry.request)
                && entry.providers == providers(&entry.request.project, &entry.request.snapshot)
        });
        if self.picker.as_ref().is_some_and(|picker| {
            self.entries
                .get(&picker.key)
                .is_none_or(|entry| entry.request.token != picker.token)
        }) {
            self.picker = None;
        }
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.picker = None;
    }
}

#[derive(Clone)]
pub(crate) struct Consumer<'a, 'state> {
    pub state: Rc<RefCell<&'state mut State>>,
    pub lsp: Option<&'a crate::lsp::LspBridge>,
    pub context: egui::Context,
    pub colors: Colors,
    pub enabled: bool,
}

impl Consumer<'_, '_> {
    pub(crate) fn before(
        &self,
        store: &EditorStore,
        project: Option<&ProjectId>,
        owner: ViewId,
        source: ViewId,
    ) -> Option<Presentation> {
        let viewport = self.context.viewport_id();
        let Some(project) = project else {
            self.state.borrow_mut().entries.remove(&(viewport, source));
            return None;
        };
        let providers = store
            .views()
            .get(source)
            .and_then(|view| store.documents().snapshot(view.document).ok())
            .zip(self.lsp)
            .map_or_else(HashSet::new, |(snapshot, lsp)| {
                lsp.code_lens_providers(project, &snapshot)
            });
        let now = Instant::now();
        let result = self.state.borrow_mut().observe(
            store,
            Context {
                project: project.clone(),
                owner,
                source,
                viewport,
            },
            providers.clone(),
            self.enabled,
            now,
        );
        match result {
            Ok(Some(request)) => {
                if self
                    .lsp
                    .is_none_or(|lsp| lsp.code_lenses(request.clone()).is_err())
                {
                    self.state.borrow_mut().accept(
                        store,
                        &request,
                        providers,
                        Err(Failure::TransportClosed),
                        now,
                    );
                }
            }
            Err(_) => {
                self.state.borrow_mut().entries.remove(&(viewport, source));
            }
            _ => {}
        }
        if let Some(entry) = self.state.borrow().entries.get(&(viewport, source)) {
            if !entry.submitted {
                self.context
                    .request_repaint_after(entry.due.saturating_duration_since(now));
            }
        }
        self.state
            .borrow()
            .display(store, viewport, source, self.colors)
    }

    pub(crate) fn after(&self, store: &EditorStore, source: ViewId, output: &Output) {
        let viewport = self.context.viewport_id();
        let stale = self
            .state
            .borrow()
            .entries
            .get(&(viewport, source))
            .is_some_and(|entry| !entry.request.describes(store));
        if stale {
            self.state.borrow_mut().entries.remove(&(viewport, source));
            return;
        }
        let now = Instant::now();
        let requests =
            self.state
                .borrow_mut()
                .resolve_visible(store, viewport, source, &output.visible, now);
        for request in requests {
            if self
                .lsp
                .is_none_or(|lsp| lsp.resolve_code_lens(request.clone()).is_err())
            {
                self.state
                    .borrow_mut()
                    .resolved(store, &request, Err(Failure::TransportClosed));
            }
        }
        if let Some(entry) = self.state.borrow().entries.get(&(viewport, source)) {
            let picker_line = self
                .state
                .borrow()
                .picker
                .as_ref()
                .filter(|picker| picker.key == (viewport, source))
                .map(|picker| picker.line);
            if entry.received
                && entry.lenses.iter().enumerate().any(|(index, lens)| {
                    lens.resolve == ResolveState::Unresolved
                        && (output.visible.contains(&index)
                            || picker_line == Some(lens.raw.range.start.line as usize))
                })
            {
                self.context
                    .request_repaint_after(entry.resolve_due.saturating_duration_since(now));
            }
        }
        if let Some(index) = output.clicked {
            let request = self
                .state
                .borrow_mut()
                .execute(store, viewport, source, index);
            if let Some(request) = request {
                if self
                    .lsp
                    .is_none_or(|lsp| lsp.execute_code_lens(request.clone()).is_err())
                {
                    self.state.borrow_mut().executed(store, &request);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "editor-code-lenses-tests.rs"]
mod tests;

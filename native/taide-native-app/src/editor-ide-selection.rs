use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use taide_ide::protocol::IdeSelectionSnapshot;
use taide_model::error::{AppError, AppResult};
use taide_model::ide::IdeSelectionInput;
use taide_model::ids::ProjectId;
use taide_model::layout::TabKind;
use taide_native_editor::document::{DocumentId, DocumentKey, EditorError};
use taide_native_editor::lsp::byte_to_position;
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{SelectionSet, ViewId, ViewKey};
use taide_native_ui::shell::WindowScope;
use taide_runtime::{AppServices, AppState, ide_actions};
use tokio::sync::watch;

const SELECTION_DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Clone)]
struct Source {
    project: ProjectId,
    key: ViewKey,
    document: DocumentId,
    path: String,
    scope: WindowScope,
}

impl Source {
    fn matches(&self, other: &Self) -> bool {
        let same_scope = match (&self.scope, &other.scope) {
            (WindowScope::Main, WindowScope::Main) => true,
            (
                WindowScope::Auxiliary { project, slot },
                WindowScope::Auxiliary {
                    project: other_project,
                    slot: other_slot,
                },
            ) => project == other_project && slot == other_slot,
            _ => false,
        };
        same_scope
            && self.project == other.project
            && self.key == other.key
            && self.document == other.document
            && self.path == other.path
    }

    fn is_active(&self, state: &AppState) -> bool {
        if state.is_shutting_down() || !state.projects.read().contains_key(&self.project) {
            return false;
        }
        let layouts = state.layouts.read();
        layouts
            .get(&self.project)
            .and_then(|layout| {
                crate::symbol_sidebar::window_tree(&self.project, layout, &self.scope)
            })
            .and_then(|(root, _)| taide_native_ui::snapshot::active_tab(root, &self.key.pane))
            .is_some_and(|tab| {
                tab.id == self.key.tab
                    && matches!(&tab.kind, TabKind::File { path } if path == &self.path)
            })
    }
}

#[derive(Clone)]
struct Published {
    source: Source,
    input: Arc<IdeSelectionInput>,
}

impl Published {
    fn describes(&self, current: &IdeSelectionSnapshot) -> bool {
        current.project_id == self.input.project_id
            && current.path == self.input.path
            && current.text == self.input.text
            && current.start_line == self.input.start_line
            && current.start_character == self.input.start_character
            && current.end_line == self.input.end_line
            && current.end_character == self.input.end_character
            && current.is_empty == self.input.is_empty
    }
}

#[derive(Clone)]
pub struct Request {
    source: Source,
    input: Option<Arc<IdeSelectionInput>>,
    cancelled: Option<watch::Receiver<bool>>,
    published: Arc<Mutex<Option<Published>>>,
}

impl Request {
    fn is_cancelled(&self) -> bool {
        self.cancelled
            .as_ref()
            .is_some_and(|cancelled| *cancelled.borrow() || cancelled.has_changed().is_err())
    }

    pub(crate) async fn apply(self, services: &AppServices) -> AppResult<()> {
        let _guard = services.state.begin_mutation().await;
        let unavailable = || AppError::Internal("native IDE selection owner is unavailable".into());
        let Some(input) = self.input else {
            let published = self.published.lock().map_err(|_| unavailable())?.clone();
            let Some(published) =
                published.filter(|published| published.source.matches(&self.source))
            else {
                return Ok(());
            };
            if services
                .ide
                .current_selection()
                .is_some_and(|current| published.describes(&current))
            {
                ide_actions::ide_clear_selection(&services.ide, self.source.key.window).await?;
            }
            *self.published.lock().map_err(|_| unavailable())? = None;
            return Ok(());
        };
        if self
            .cancelled
            .as_ref()
            .is_none_or(|cancelled| *cancelled.borrow() || cancelled.has_changed().is_err())
            || !services.ide.is_running()
            || !self.source.is_active(&services.state)
        {
            return Ok(());
        }
        ide_actions::ide_set_selection(&services.ide, (*input).clone()).await?;
        *self.published.lock().map_err(|_| unavailable())? = Some(Published {
            source: self.source,
            input,
        });
        Ok(())
    }
}

struct Entry {
    source: Source,
    selection: SelectionSet,
    pending: Option<(Instant, u64)>,
    cancel: Option<watch::Sender<bool>>,
}

#[derive(Default)]
pub(crate) struct State {
    entries: HashMap<ViewId, Entry>,
    seen: HashSet<ViewId>,
    clears: Vec<Request>,
    retries: Vec<(Instant, Request)>,
    published: Arc<Mutex<Option<Published>>>,
    is_running: bool,
    order: u64,
}

impl State {
    pub(crate) fn retry(&mut self, request: Request, now: Instant) {
        if !request.is_cancelled() {
            self.retries.push((now + SELECTION_DEBOUNCE, request));
        }
    }

    pub(crate) fn begin_frame(&mut self, is_running: bool) {
        self.is_running = is_running;
        self.seen.clear();
        if !is_running {
            let closed = self.entries.keys().copied().collect::<Vec<_>>();
            for view in closed {
                self.forget(view);
            }
        }
    }

    fn forget(&mut self, view: ViewId) {
        let Some(entry) = self.entries.remove(&view) else {
            return;
        };
        if let Some(cancel) = entry.cancel {
            cancel.send_replace(true);
        }
        self.clears.push(Request {
            source: entry.source,
            input: None,
            cancelled: None,
            published: self.published.clone(),
        });
    }

    pub(crate) fn observe(
        &mut self,
        store: &EditorStore,
        project: &ProjectId,
        scope: &WindowScope,
        view: ViewId,
        path: &str,
        now: Instant,
    ) {
        if !self.is_running {
            return;
        }
        let Some(current) = store.views().get(view) else {
            return;
        };
        let source = Source {
            project: project.clone(),
            key: current.key.clone(),
            document: current.document,
            path: path.into(),
            scope: scope.clone(),
        };
        if self
            .entries
            .get(&view)
            .is_some_and(|entry| !entry.source.matches(&source))
        {
            self.forget(view);
        }
        self.seen.insert(view);
        let entry = self.entries.entry(view).or_insert_with(|| Entry {
            source,
            selection: current.selection.clone(),
            pending: None,
            cancel: None,
        });
        if entry.selection == current.selection {
            return;
        }
        if let Some(cancel) = entry.cancel.take() {
            cancel.send_replace(true);
        }
        entry.selection = current.selection.clone();
        self.order = self.order.wrapping_add(1);
        entry.pending = Some((now + SELECTION_DEBOUNCE, self.order));
    }

    pub(crate) fn next_deadline(&self) -> Option<Instant> {
        self.entries
            .values()
            .filter_map(|entry| entry.pending.map(|(deadline, _)| deadline))
            .chain(self.retries.iter().map(|(deadline, _)| *deadline))
            .min()
    }

    pub(crate) fn finish_frame(
        &mut self,
        store: &EditorStore,
        now: Instant,
    ) -> Vec<Result<Request, EditorError>> {
        let closed = self
            .entries
            .keys()
            .copied()
            .filter(|view| !self.seen.contains(view))
            .collect::<Vec<_>>();
        for view in closed {
            self.forget(view);
        }
        self.retries.retain(|(_, request)| !request.is_cancelled());
        let mut requests = self
            .retries
            .extract_if(.., |(deadline, _)| *deadline <= now)
            .map(|(_, request)| Ok(request))
            .chain(std::mem::take(&mut self.clears).into_iter().map(Ok))
            .collect::<Vec<_>>();
        let mut due = self
            .entries
            .iter()
            .filter_map(|(view, entry)| {
                entry
                    .pending
                    .filter(|(deadline, _)| *deadline <= now)
                    .map(|(deadline, order)| (deadline, order, *view))
            })
            .collect::<Vec<_>>();
        due.sort_by_key(|(deadline, order, _)| (*deadline, *order));
        for (_, _, view) in due {
            let Some(entry) = self.entries.get_mut(&view) else {
                continue;
            };
            entry.pending = None;
            let input = (|| {
                let current = store.views().get(view).ok_or(EditorError::NotFound)?;
                if current.document != entry.source.document || current.key != entry.source.key {
                    return Err(EditorError::InvalidIdentity);
                }
                let document = store.documents().snapshot(current.document)?;
                if !matches!(document.key, DocumentKey::File(_)) {
                    return Err(EditorError::InvalidIdentity);
                }
                let selection = current
                    .selection
                    .selections
                    .get(current.selection.primary)
                    .ok_or(EditorError::InvalidBoundary)?;
                let start_byte = selection.anchor.min(selection.head);
                let end_byte = selection.anchor.max(selection.head);
                let start = byte_to_position(&document, start_byte)?;
                let end = byte_to_position(&document, end_byte)?;
                Ok(IdeSelectionInput {
                    owner: current.key.window.clone(),
                    project_id: entry.source.project.clone(),
                    path: entry.source.path.clone(),
                    text: document.rope.byte_slice(start_byte..end_byte).to_string(),
                    start_line: start.line,
                    start_character: start.character,
                    end_line: end.line,
                    end_character: end.character,
                    is_empty: start_byte == end_byte,
                })
            })();
            let input = match input {
                Ok(input) => input,
                Err(error) => {
                    requests.push(Err(error));
                    continue;
                }
            };
            if let Some(cancel) = entry.cancel.take() {
                cancel.send_replace(true);
            }
            let (cancel, cancelled) = watch::channel(false);
            entry.cancel = Some(cancel);
            requests.push(Ok(Request {
                source: entry.source.clone(),
                input: Some(Arc::new(input)),
                cancelled: Some(cancelled),
                published: self.published.clone(),
            }));
        }
        requests
    }
}

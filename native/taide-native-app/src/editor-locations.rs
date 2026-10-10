use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use taide_lsp::native::Failure;
use taide_lsp::native::protocol::lsp_types;
use taide_model::ids::ProjectId;
use taide_native_editor::decoration::{
    Decoration, DecorationKind, DecorationLayer, InlineStyle, Stickiness,
};
use taide_native_editor::document::{DocumentId, DocumentSnapshot, EditorError};
use taide_native_editor::lsp::{Position, byte_to_position};
use taide_native_editor::store::EditorStore;
use taide_native_editor::symbol_locations::{Kind, Locations, Mode, Target};
use taide_native_editor::view::{SelectionSet, ViewId, ViewKey};
use tokio::sync::watch;
use uuid::Uuid;

use crate::editor_symbols::ProviderIdentity;

const NAVIGATION_HIGHLIGHT_TTL: Duration = Duration::from_millis(350);
const NAVIGATION_TTL: Duration = Duration::from_secs(5);
const HOVER_LINES: usize = 8;
const HOVER_SPAN_FIELDS: usize = 2;
const TOKEN_DECORATION_WIDTH: f32 = 1.0;

struct TrackedReference {
    index: usize,
    range: DecorationLayer,
    selection: DecorationLayer,
}

struct Highlight {
    document: DocumentId,
    layer: DecorationLayer,
    deadline: Instant,
}

struct PendingPeek {
    session: Session,
    project: ProjectId,
    pane: taide_model::ids::PaneId,
    tab: taide_model::ids::TabId,
    path: String,
    viewport: eframe::egui::ViewportId,
    deadline: Instant,
}

struct PendingNavigation {
    target: Target,
    deadline: Instant,
}

struct OpenOperation {
    source: Request,
    token: Uuid,
    cancel: watch::Sender<bool>,
    deadline: Instant,
}

impl Drop for OpenOperation {
    fn drop(&mut self) {
        self.cancel.send_replace(true);
    }
}

pub struct Group {
    pub provider: ProviderIdentity,
    pub targets: Vec<Target>,
}

pub struct Response {
    pub groups: Vec<Group>,
}

#[derive(Clone)]
pub struct Request {
    pub project: ProjectId,
    pub snapshot: DocumentSnapshot,
    pub source: ViewId,
    pub source_key: ViewKey,
    pub selection: SelectionSet,
    pub position: Position,
    pub kind: Kind,
    pub mode: Mode,
    pub token: Uuid,
    pub cancelled: watch::Receiver<bool>,
    pub viewport: eframe::egui::ViewportId,
    pub keyboard: bool,
}

impl Request {
    pub(crate) fn is_active(
        &self,
        layout: &taide_model::layout::ProjectLayout,
        scope: &taide_native_ui::shell::WindowScope,
    ) -> bool {
        crate::symbol_sidebar::window_tree(&self.project, layout, scope).is_some_and(
            |(root, focused)| {
                *focused == self.source_key.pane
                    && taide_native_ui::snapshot::active_tab(root, focused)
                        .is_some_and(|tab| tab.id == self.source_key.tab)
            },
        )
    }
    pub fn is_cancelled(&self) -> bool {
        *self.cancelled.borrow() || self.cancelled.has_changed().is_err()
    }

    pub(crate) fn describes(&self, store: &EditorStore, check_selection: bool) -> bool {
        store.views().get(self.source).is_some_and(|view| {
            view.key == self.source_key
                && view.document == self.snapshot.id
                && (!check_selection || view.selection == self.selection)
                && store
                    .documents()
                    .snapshot(view.document)
                    .is_ok_and(|current| {
                        current.key == self.snapshot.key
                            && current.revision == self.snapshot.revision
                            && current.metadata.language_id == self.snapshot.metadata.language_id
                    })
        })
    }
}

pub(crate) struct Session {
    pub(crate) request: Request,
    pub(crate) model: Option<Arc<Locations>>,
    pub(crate) selected: Option<usize>,
    providers: HashSet<ProviderIdentity>,
    cancel: watch::Sender<bool>,
    pub(crate) shown: bool,
    pub(crate) preview: Option<ViewId>,
    pub(crate) preview_selection: Option<usize>,
    pub(crate) preview_focus: bool,
    pub(crate) attempted: HashSet<std::path::PathBuf>,
    anchor: DecorationLayer,
    keymap: crate::keymap::Windows,
    tracked: HashMap<DocumentId, Vec<TrackedReference>>,
    open: Option<OpenOperation>,
    focus: Option<(u64, taide_native_ui::editor_locations::Focus)>,
    find: taide_native_ui::editor_find::EditorFind,
    folds: Vec<taide_native_editor::folding::FoldCommand>,
    documentation: Vec<taide_native_editor::documentation::Command>,
}

impl Drop for Session {
    fn drop(&mut self) {
        self.cancel.send_replace(true);
    }
}

#[derive(Default)]
pub(crate) struct State {
    sessions: HashMap<ViewId, Session>,
    hovers: HashMap<ViewId, Session>,
    retired_previews: Vec<ViewId>,
    highlights: HashMap<ViewId, Highlight>,
    navigations: HashMap<(eframe::egui::ViewportId, taide_model::ids::TabId), PendingNavigation>,
    pending_peeks: Vec<PendingPeek>,
}

impl State {
    pub(crate) fn cancel_open(&mut self, view: ViewId) {
        if let Some(session) = self.sessions.get_mut(&view) {
            session.open = None;
        }
    }

    pub(crate) fn begin_open(
        &mut self,
        store: &EditorStore,
        view: ViewId,
    ) -> Result<(Request, Uuid, watch::Receiver<bool>), EditorError> {
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        let session = self.sessions.get_mut(&view).ok_or(EditorError::NotFound)?;
        let mut source = session.request.clone();
        source.snapshot = store.documents().snapshot(current.document)?;
        source.selection = current.selection.clone();
        let token = Uuid::new_v4();
        let (cancel, cancelled) = watch::channel(false);
        session.open = Some(OpenOperation {
            source: source.clone(),
            token,
            cancel,
            deadline: Instant::now() + NAVIGATION_TTL,
        });
        Ok((source, token, cancelled))
    }

    pub(crate) fn is_current_open(&self, request: &crate::symbol_location_host::Request) -> bool {
        !request.is_cancelled()
            && self.is_current(&request.source)
            && self
                .sessions
                .get(&request.source.source)
                .and_then(|session| session.open.as_ref())
                .is_some_and(|open| open.token == request.token)
    }

    pub(crate) fn rehome(
        &mut self,
        source: ViewId,
        opened: &crate::terminal_tabs::OpenedFileLink,
        now: Instant,
    ) {
        if let Some(session) = self.sessions.remove(&source) {
            self.clear_hover(source);
            self.pending_peeks.push(PendingPeek {
                session,
                project: opened.project.clone(),
                pane: opened.pane.clone(),
                tab: opened.tab.clone(),
                path: opened.path.clone(),
                viewport: opened.viewport,
                deadline: now + NAVIGATION_TTL,
            });
        }
    }

    pub(crate) fn adopt_pending(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        project: &ProjectId,
        path: &str,
        viewport: eframe::egui::ViewportId,
        now: Instant,
    ) -> Result<bool, EditorError> {
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        let Some(index) = self.pending_peeks.iter().position(|pending| {
            pending.deadline > now
                && pending.project == *project
                && pending.path == path
                && pending.viewport == viewport
                && pending.pane == current.key.pane
                && pending.tab == current.key.tab
        }) else {
            return Ok(false);
        };
        let mut pending = self.pending_peeks.remove(index);
        let snapshot = store.documents().snapshot(current.document)?;
        if snapshot.key != taide_native_editor::document::DocumentKey::File(path.into()) {
            self.retired_previews.extend(pending.session.preview);
            return Ok(false);
        }
        let position = byte_to_position(
            &snapshot,
            current.selection.selections[current.selection.primary].head,
        )?;
        pending.session.request.source = view;
        pending.session.request.source_key = current.key.clone();
        pending.session.request.snapshot = snapshot.clone();
        pending.session.request.selection = current.selection.clone();
        pending.session.request.position = position;
        pending.session.request.viewport = viewport;
        pending.session.anchor = range_layer(&snapshot, lsp_types::Range::new(position, position))?;
        pending.session.shown = true;
        if let Some(before) = self.sessions.insert(view, pending.session) {
            self.retired_previews.extend(before.preview);
        }
        Ok(true)
    }

    pub(crate) fn navigate(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
        target: &Target,
        now: Instant,
    ) -> Result<(), EditorError> {
        reveal(store, view, target)?;
        let document = store.documents().snapshot(
            store
                .views()
                .get(view)
                .ok_or(EditorError::NotFound)?
                .document,
        )?;
        let bytes = taide_native_editor::lsp::range_to_bytes(&document, target.selection)?;
        store.request_selection_reveal_near_top(view, bytes.start..bytes.start)?;
        self.highlights.insert(
            view,
            Highlight {
                document: document.id,
                layer: range_layer(&document, target.selection)?,
                deadline: now + NAVIGATION_HIGHLIGHT_TTL,
            },
        );
        Ok(())
    }

    pub(crate) fn queue_navigation(
        &mut self,
        opened: &crate::terminal_tabs::OpenedFileLink,
        target: Target,
        now: Instant,
    ) {
        self.navigations.retain(|_, pending| pending.deadline > now);
        self.navigations.insert(
            (opened.viewport, opened.tab.clone()),
            PendingNavigation {
                target,
                deadline: now + NAVIGATION_TTL,
            },
        );
    }

    pub(crate) fn take_navigation(
        &mut self,
        tab: &taide_model::ids::TabId,
        path: &str,
        viewport: eframe::egui::ViewportId,
        now: Instant,
    ) -> Option<Target> {
        self.navigations.retain(|_, pending| pending.deadline > now);
        let key = (viewport, tab.clone());
        let pending = self.navigations.get(&key)?;
        if file_path(&pending.target.uri)?.to_str()? != path {
            return None;
        }
        self.navigations.remove(&key).map(|pending| pending.target)
    }

    pub(crate) fn highlight(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        now: Instant,
        color: [u8; 4],
    ) -> Option<(DecorationLayer, Duration)> {
        self.highlights.retain(|view, highlight| {
            highlight.deadline > now
                && store
                    .views()
                    .get(*view)
                    .is_some_and(|view| view.document == highlight.document)
        });
        let highlight = self.highlights.get_mut(&view)?;
        highlight.layer = highlight
            .layer
            .tracking(
                store
                    .changes_since(highlight.document, highlight.layer.revision())
                    .ok()?,
            )?
            .into_owned();
        let items = highlight
            .layer
            .items()
            .iter()
            .map(|item| Decoration {
                kind: DecorationKind::Inline(InlineStyle {
                    background: Some(color),
                    ..Default::default()
                }),
                ..item.clone()
            })
            .collect();
        Some((
            DecorationLayer::new(highlight.layer.revision(), 0, items),
            highlight.deadline.saturating_duration_since(now),
        ))
    }
    pub(crate) fn retain_active(
        &mut self,
        layouts: &HashMap<ProjectId, taide_model::layout::ProjectLayout>,
        scope: &taide_native_ui::shell::WindowScope,
    ) {
        let retired = &mut self.retired_previews;
        let now = Instant::now();
        self.pending_peeks.retain(|pending| {
            let active = pending.deadline > now
                && layouts.get(&pending.project).is_some_and(|layout| {
                    crate::symbol_sidebar::window_tree(&pending.project, layout, scope).is_some_and(
                        |(root, _)| {
                            taide_native_ui::snapshot::active_tab(root, &pending.pane)
                                .is_some_and(|tab| tab.id == pending.tab)
                        },
                    )
                });
            if !active {
                retired.extend(pending.session.preview);
            }
            active
        });
        for sessions in [&mut self.sessions, &mut self.hovers] {
            sessions.retain(|_, session| {
                if session
                    .open
                    .as_ref()
                    .is_some_and(|open| open.deadline <= now)
                {
                    session.open = None;
                }
                let active = layouts.get(&session.request.project).is_some_and(|layout| {
                    if session.open.is_some() {
                        return crate::symbol_sidebar::window_tree(
                            &session.request.project,
                            layout,
                            scope,
                        )
                        .is_some();
                    }
                    if session.model.is_none() || session.request.mode == Mode::Hover {
                        return session.request.is_active(layout, scope);
                    }
                    crate::symbol_sidebar::window_tree(&session.request.project, layout, scope)
                        .is_some_and(|(root, _)| {
                            taide_native_ui::snapshot::active_tab(
                                root,
                                &session.request.source_key.pane,
                            )
                            .is_some_and(|tab| tab.id == session.request.source_key.tab)
                        })
                });
                if !active {
                    retired.extend(session.preview);
                }
                active
            });
        }
    }
    pub(crate) fn begin(
        &mut self,
        project: ProjectId,
        store: &EditorStore,
        view: ViewId,
        kind: Kind,
        mode: Mode,
        providers: HashSet<ProviderIdentity>,
        position: Option<Position>,
    ) -> Result<Request, EditorError> {
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        let snapshot = store.documents().snapshot(current.document)?;
        let head = current.selection.selections[current.selection.primary].head;
        let keyboard = mode == Mode::Hover && position.is_none();
        let position = position.map_or_else(|| byte_to_position(&snapshot, head), Ok)?;
        let (cancel, cancelled) = watch::channel(false);
        let request = Request {
            project,
            snapshot,
            source: view,
            source_key: current.key.clone(),
            selection: current.selection.clone(),
            position,
            kind,
            mode,
            token: Uuid::new_v4(),
            cancelled,
            viewport: eframe::egui::ViewportId::ROOT,
            keyboard,
        };
        let sessions = if mode == Mode::Hover {
            &mut self.hovers
        } else {
            &mut self.sessions
        };
        if let Some(before) = sessions.remove(&view) {
            self.retired_previews.extend(before.preview);
        }
        let byte = taide_native_editor::lsp::position_to_byte(&request.snapshot, position)?;
        sessions.insert(
            view,
            Session {
                request: request.clone(),
                model: None,
                selected: None,
                providers,
                cancel,
                shown: false,
                preview: None,
                preview_selection: None,
                preview_focus: false,
                attempted: HashSet::new(),
                keymap: crate::keymap::Windows::default(),
                tracked: HashMap::new(),
                open: None,
                focus: None,
                find: Default::default(),
                folds: Vec::new(),
                documentation: Vec::new(),
                anchor: DecorationLayer::new(
                    request.snapshot.revision,
                    0,
                    vec![Decoration {
                        bytes: byte..byte,
                        kind: DecorationKind::Inline(InlineStyle::default()),
                        stickiness: Stickiness::NeverGrowsWhenTypingAtEdges,
                    }],
                ),
            },
        );
        Ok(request)
    }

    pub(crate) fn current(&self, view: ViewId) -> Option<&Session> {
        self.sessions.get(&view)
    }

    pub(crate) fn preview_find_visible(&self, view: ViewId) -> bool {
        self.current(view)
            .is_some_and(|session| session.find.visible)
    }

    pub(crate) fn clear_hover(&mut self, view: ViewId) {
        self.hovers.remove(&view);
    }

    pub(crate) fn close_request(&mut self, request: &Request) {
        if !self.is_current(request) {
            return;
        }
        if request.mode == Mode::Hover {
            self.clear_hover(request.source);
        } else {
            self.close(request.source);
        }
    }

    pub(crate) fn preview_owner(&self, view: ViewId) -> Option<ViewId> {
        self.sessions
            .iter()
            .find_map(|(source, session)| (session.preview == Some(view)).then_some(*source))
    }

    pub(crate) fn local_key(&self, view: ViewId, event: &eframe::egui::Event) -> bool {
        if self.preview_owner(view).is_some() {
            return true;
        }
        self.current(view).is_some_and(|session| session.shown)
            && matches!(
                event,
                eframe::egui::Event::Key {
                    key: eframe::egui::Key::F4
                        | eframe::egui::Key::F12
                        | eframe::egui::Key::Escape
                        | eframe::egui::Key::F2,
                    ..
                } | eframe::egui::Event::Key {
                    key: eframe::egui::Key::K,
                    modifiers: eframe::egui::Modifiers { command: true, .. },
                    ..
                }
            )
    }

    pub(crate) fn hover(&self, view: ViewId) -> Option<&Session> {
        self.hovers.get(&view)
    }

    pub(crate) fn hover_document(&self, store: &EditorStore, view: ViewId) -> Option<DocumentId> {
        let model = self.hover(view)?.model.as_ref()?;
        let path = file_path(&model.targets().get(model.first()?)?.uri)?;
        store
            .documents()
            .find(&taide_native_editor::document::DocumentKey::File(path))
    }

    pub(crate) fn close(&mut self, view: ViewId) {
        self.hovers.remove(&view);
        if let Some(session) = self.sessions.remove(&view) {
            self.retired_previews.extend(session.preview);
        }
    }

    pub(crate) fn reconcile(
        &mut self,
        store: &EditorStore,
        projects: &HashSet<ProjectId>,
        providers: impl Fn(&ProjectId, &DocumentSnapshot, Kind) -> HashSet<ProviderIdentity>,
    ) {
        let retired = &mut self.retired_previews;
        self.pending_peeks.retain(|pending| {
            let keep = projects.contains(&pending.project);
            if !keep {
                retired.extend(pending.session.preview);
            }
            keep
        });
        for sessions in [&mut self.sessions, &mut self.hovers] {
            sessions.retain(|_, session| {
                if session
                    .open
                    .as_ref()
                    .is_some_and(|open| !open.source.describes(store, true))
                {
                    session.open = None;
                }
                let current = store.documents().snapshot(session.request.snapshot.id).ok();
                let owns = store
                    .views()
                    .get(session.request.source)
                    .is_some_and(|view| {
                        view.key == session.request.source_key
                            && view.document == session.request.snapshot.id
                    })
                    && current.as_ref().is_some_and(|current| {
                        current.key == session.request.snapshot.key
                            && current.metadata.language_id
                                == session.request.snapshot.metadata.language_id
                    });
                let keep = projects.contains(&session.request.project)
                    && (session.model.is_some() && session.request.mode != Mode::Hover && owns
                        || (session.model.is_none() || session.request.mode == Mode::Hover)
                            && session.request.describes(store, true))
                    && (session.model.is_some() && session.request.mode != Mode::Hover
                        || session.providers
                            == providers(
                                &session.request.project,
                                current.as_ref().unwrap_or(&session.request.snapshot),
                                session.request.kind,
                            ));
                if !keep {
                    retired.extend(session.preview);
                }
                keep
            });
        }
    }

    pub(crate) fn is_current(&self, request: &Request) -> bool {
        !request.is_cancelled()
            && (if request.mode == Mode::Hover {
                &self.hovers
            } else {
                &self.sessions
            })
            .get(&request.source)
            .is_some_and(|session| session.request.token == request.token)
    }

    pub(crate) fn preview_views(&self) -> HashSet<ViewId> {
        self.sessions
            .values()
            .filter_map(|session| session.preview)
            .chain(
                self.pending_peeks
                    .iter()
                    .filter_map(|pending| pending.session.preview),
            )
            .collect()
    }

    pub(crate) fn preview_bindings(&self, store: &EditorStore) -> Vec<(ProjectId, DocumentId)> {
        self.sessions
            .values()
            .filter(|session| {
                session.shown
                    && store
                        .views()
                        .get(session.request.source)
                        .is_some_and(|source| {
                            source.key == session.request.source_key
                                && source.document == session.request.snapshot.id
                        })
            })
            .filter_map(|session| {
                Some((
                    session.request.project.clone(),
                    store.views().get(session.preview?)?.document,
                ))
            })
            .collect()
    }

    pub(crate) fn detach_retired(&mut self, store: &mut EditorStore) {
        for view in self.retired_previews.drain(..) {
            if store.views().get(view).is_some() {
                let _ = store.detach_view(view);
            }
        }
    }

    pub(crate) fn widget(
        &mut self,
        store: &EditorStore,
        view: ViewId,
    ) -> Option<taide_native_ui::editor_locations::Widget> {
        if !self.synchronize(store, view) {
            return None;
        }
        let session = self
            .sessions
            .get_mut(&view)
            .filter(|session| session.shown)?;
        let position = session.anchor.items().first()?.bytes.start;
        Some(taide_native_ui::editor_locations::Widget {
            token: session.request.token.to_string(),
            position,
            title: session.request.kind.title().into(),
            model: session.model.clone()?,
            selected: session.selected,
            focus: session.focus,
        })
    }

    fn synchronize(&mut self, store: &EditorStore, view: ViewId) -> bool {
        let Some(session) = self.sessions.get_mut(&view) else {
            return false;
        };
        let current = store
            .views()
            .get(view)
            .and_then(|view| store.documents().snapshot(view.document).ok());
        let anchor = current
            .and_then(|document| {
                store
                    .changes_since(document.id, session.anchor.revision())
                    .ok()
            })
            .and_then(|changes| session.anchor.tracking(changes))
            .map(|layer| layer.into_owned());
        if let Some(anchor) = anchor
            && refresh_references(session, store)
        {
            session.anchor = anchor;
            return true;
        }
        self.close(view);
        false
    }

    pub(crate) fn preload(
        &mut self,
        store: &EditorStore,
        models: &crate::peek_models::Models,
        view: ViewId,
        mode: Mode,
    ) -> Option<(Request, Vec<std::path::PathBuf>)> {
        let session = (if mode == Mode::Hover {
            &mut self.hovers
        } else {
            &mut self.sessions
        })
        .get_mut(&view)?;
        let model = session.model.as_ref()?;
        let mut paths = models.preload_paths(store, model.targets());
        if let Some(target) = session
            .selected
            .and_then(|index| model.targets().get(index))
            && let Some(path) = file_path(&target.uri)
            && store
                .documents()
                .find(&taide_native_editor::document::DocumentKey::File(
                    path.clone(),
                ))
                .is_none()
            && !paths.contains(&path)
        {
            paths.push(path);
        }
        paths.retain(|path| session.attempted.insert(path.clone()));
        (!paths.is_empty()).then(|| (session.request.clone(), paths))
    }

    pub(crate) fn show(&mut self, view: ViewId) {
        if let Some(session) = self.sessions.get_mut(&view) {
            session.shown = true;
        }
    }

    pub(crate) fn prepare_preview(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
    ) -> Result<Option<DocumentId>, EditorError> {
        if !self.synchronize(store, view) {
            return Ok(None);
        }
        let Some(session) = self.sessions.get_mut(&view).filter(|session| session.shown) else {
            return Ok(None);
        };
        let target = session
            .selected
            .and_then(|index| session.model.as_ref()?.targets().get(index))
            .cloned();
        let document = target
            .as_ref()
            .and_then(|target| file_path(&target.uri))
            .and_then(|path| {
                store
                    .documents()
                    .find(&taide_native_editor::document::DocumentKey::File(path))
            });
        if session
            .preview
            .is_some_and(|preview| store.views().get(preview).map(|view| view.document) != document)
        {
            if let Some(preview) = session.preview.take()
                && store.views().get(preview).is_some()
            {
                store.detach_view(preview)?;
            }
            session.preview_selection = None;
            session.folds.clear();
        }
        let Some(document) = document else {
            return Ok(None);
        };
        if !session.tracked.contains_key(&document) {
            let snapshot = store.documents().snapshot(document)?;
            let refs = session
                .model
                .as_ref()
                .into_iter()
                .flat_map(|model| model.targets().iter().enumerate())
                .filter(|(_, target)| {
                    file_path(&target.uri).is_some_and(|path| {
                        snapshot.key == taide_native_editor::document::DocumentKey::File(path)
                    })
                })
                .filter_map(|(index, target)| {
                    Some(TrackedReference {
                        index,
                        range: range_layer(&snapshot, target.range).ok()?,
                        selection: range_layer(&snapshot, target.selection).ok()?,
                    })
                })
                .collect();
            session.tracked.insert(document, refs);
        }
        let target = session
            .selected
            .and_then(|index| session.model.as_ref()?.targets().get(index))
            .cloned();
        let preview = if let Some(preview) = session.preview {
            preview
        } else {
            let preview = store.attach_view(
                ViewKey {
                    window: session.request.source_key.window.clone(),
                    pane: taide_model::ids::PaneId::new(),
                    tab: taide_model::ids::TabId::new(),
                },
                document,
            )?;
            session.preview = Some(preview);
            preview
        };
        if session.preview_selection != session.selected {
            reveal(store, preview, &target.ok_or(EditorError::NotFound)?)?;
            session.preview_selection = session.selected;
        }
        Ok(Some(document))
    }

    pub(crate) fn accept(
        &mut self,
        request: &Request,
        store: &EditorStore,
        providers: HashSet<ProviderIdentity>,
        result: Result<Response, Failure>,
    ) -> Result<Option<Arc<Locations>>, Failure> {
        let Some(session) = (if request.mode == Mode::Hover {
            &mut self.hovers
        } else {
            &mut self.sessions
        })
        .get_mut(&request.source)
        .filter(|session| {
            session.request.token == request.token
                && !request.is_cancelled()
                && request.describes(store, true)
                && session.providers == providers
        }) else {
            return Ok(None);
        };
        let response = result?;
        if response
            .groups
            .iter()
            .any(|group| !providers.contains(&group.provider))
        {
            return Ok(None);
        }
        let model = Arc::new(Locations::new(
            response
                .groups
                .into_iter()
                .flat_map(|group| group.targets)
                .collect(),
        ));
        let uri = match &request.snapshot.key {
            taide_native_editor::document::DocumentKey::File(path) => {
                taide_lsp::service::workspace_folder_uri(
                    path.to_str().ok_or(Failure::MalformedRequest)?,
                )
                .parse()
                .map_err(|_| Failure::MalformedRequest)?
            }
            _ => return Err(Failure::DocumentNotOpen),
        };
        session.selected = model.nearest(&uri, request.position);
        session.model = Some(model.clone());
        Ok(Some(model))
    }
}

fn range_layer(
    document: &DocumentSnapshot,
    range: lsp_types::Range,
) -> Result<DecorationLayer, EditorError> {
    let bytes = taide_native_editor::lsp::range_to_bytes(document, range)?;
    Ok(DecorationLayer::new(
        document.revision,
        0,
        vec![Decoration {
            bytes,
            kind: DecorationKind::Inline(InlineStyle::default()),
            stickiness: Stickiness::NeverGrowsWhenTypingAtEdges,
        }],
    ))
}

fn refresh_references(session: &mut Session, store: &EditorStore) -> bool {
    let Some(model) = session.model.as_mut() else {
        return true;
    };
    for (document, references) in &mut session.tracked {
        let Ok(snapshot) = store.documents().snapshot(*document) else {
            continue;
        };
        for reference in references {
            if reference.range.revision() == snapshot.revision {
                continue;
            }
            let range = store
                .changes_since(*document, reference.range.revision())
                .ok()
                .and_then(|changes| reference.range.tracking(changes))
                .map(|layer| layer.into_owned());
            let selection = store
                .changes_since(*document, reference.selection.revision())
                .ok()
                .and_then(|changes| reference.selection.tracking(changes))
                .map(|layer| layer.into_owned());
            let Some((range, selection)) = range.zip(selection) else {
                return false;
            };
            let to_range = |layer: &DecorationLayer| {
                let bytes = &layer.items().first()?.bytes;
                Some(lsp_types::Range::new(
                    byte_to_position(&snapshot, bytes.start).ok()?,
                    byte_to_position(&snapshot, bytes.end).ok()?,
                ))
            };
            if let Some((full, selected)) = to_range(&range).zip(to_range(&selection))
                && Arc::make_mut(model).update_ranges(reference.index, full, selected)
            {
                reference.range = range;
                reference.selection = selection;
            } else {
                return false;
            }
        }
    }
    true
}

pub(crate) fn file_path(uri: &lsp_types::Uri) -> Option<std::path::PathBuf> {
    let uri = url::Url::parse(uri.as_str()).ok()?;
    if uri.scheme() != "file" || uri.query().is_some() || uri.fragment().is_some() {
        return None;
    }
    let path = uri.to_file_path().ok()?;
    if !path.is_absolute() || path.to_str()?.contains('\0') {
        return None;
    }
    Some(path)
}

pub(crate) fn reveal(
    store: &mut EditorStore,
    view: ViewId,
    target: &Target,
) -> Result<(), EditorError> {
    let current = store
        .views()
        .get(view)
        .ok_or(EditorError::NotFound)?
        .clone();
    let document = store.documents().snapshot(current.document)?;
    let bytes = taide_native_editor::lsp::range_to_bytes(&document, target.selection)?;
    let folds = current
        .folds
        .into_iter()
        .filter(|fold| !fold.contains(&bytes.start) && !fold.contains(&bytes.end))
        .collect();
    store.set_view_state(
        view,
        SelectionSet {
            primary: 0,
            selections: vec![taide_native_editor::view::Selection {
                anchor: bytes.start,
                head: bytes.start,
            }],
        },
        current.scroll,
        folds,
    )?;
    store.request_selection_reveal(view, bytes, true)
}

pub(crate) struct Provider<'a, 'state> {
    pub(crate) state: &'a mut State,
    pub(crate) models: &'a crate::peek_models::Models,
    pub(crate) project: Option<ProjectId>,
    pub(crate) lsp: Option<&'a crate::lsp::LspBridge>,
    pub(crate) layout: Option<&'a taide_model::layout::ProjectLayout>,
    pub(crate) scope: &'a taide_native_ui::shell::WindowScope,
    pub(crate) commands: &'a mut Vec<crate::host::HostCommand>,
    pub(crate) viewport: eframe::egui::ViewportId,
    pub(crate) editor: &'a taide_native_ui::editor_surface::NativeEditor,
    pub(crate) indentation: taide_native_editor::indent::IndentConfiguration,
    pub(crate) presentation: &'a taide_native_ui::editor_surface::EditorPresentation,
    pub(crate) tokens: Option<Arc<crate::editor_syntax::PeekTokens>>,
    pub(crate) hover_tokens: Option<Arc<crate::editor_syntax::PeekTokens>>,
    pub(crate) shown_lines: &'a mut Vec<(DocumentId, std::ops::Range<usize>)>,
    pub(crate) changed: &'a mut HashMap<taide_native_editor::document::DocumentId, ViewId>,
    pub(crate) overrides: Option<&'a str>,
    pub(crate) focus_targets: &'a mut Vec<(eframe::egui::Id, ViewId)>,
    pub(crate) find_history: Option<&'a mut taide_native_ui::editor_find_widget::FindHistory>,
    pub(crate) find_appearance: Option<&'a taide_native_ui::editor_find_widget::FindAppearance>,
    pub(crate) documentation:
        Option<std::rc::Rc<std::cell::RefCell<&'state mut crate::editor_documentation::State>>>,
    pub(crate) completion: Option<crate::editor_completion::Consumer<'a, 'state>>,
    pub(crate) highlights: Option<crate::editor_highlights::Consumer<'a, 'state>>,
}

impl Provider<'_, '_> {
    pub(crate) fn request(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        kind: Kind,
        mode: Mode,
        position: Option<Position>,
    ) -> Result<bool, EditorError> {
        let Some(project) = self.project.clone() else {
            return Ok(false);
        };
        let Some(lsp) = self.lsp else {
            return Ok(false);
        };
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        let snapshot = store.documents().snapshot(current.document)?;
        let providers = lsp.location_providers(&project, &snapshot, kind);
        if providers.is_empty() {
            return Ok(false);
        }
        if mode == Mode::Peek
            && self.state.current(view).is_some_and(|session| {
                session.shown
                    && session.request.kind == kind
                    && session.request.selection == current.selection
            })
        {
            self.state.close(view);
            return Ok(true);
        }
        let mut request = self
            .state
            .begin(project, store, view, kind, mode, providers, position)?;
        request.viewport = self.viewport;
        if let Some(session) = (if mode == Mode::Hover {
            &mut self.state.hovers
        } else {
            &mut self.state.sessions
        })
        .get_mut(&view)
        {
            session.request.viewport = self.viewport;
        }
        if lsp.symbol_locations(request.clone()).is_err() {
            self.state.close_request(&request);
            return Ok(false);
        }
        Ok(true)
    }

    pub(crate) fn open(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
        target: Target,
        side: bool,
        keep_peek: bool,
    ) -> Result<bool, EditorError> {
        if !self.state.synchronize(store, view) {
            return Ok(false);
        }
        self.state.cancel_open(view);
        if self.state.current(view).is_none() {
            return Ok(false);
        }
        let Some(path) = file_path(&target.uri) else {
            return Ok(false);
        };
        let current = store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .clone();
        let snapshot = store.documents().snapshot(current.document)?;
        if snapshot.key == taide_native_editor::document::DocumentKey::File(path) && !side {
            self.state.navigate(store, view, &target, Instant::now())?;
            if keep_peek {
                if let Some(session) = self.state.sessions.get_mut(&view) {
                    session.anchor = range_layer(
                        &snapshot,
                        lsp_types::Range::new(target.selection.start, target.selection.start),
                    )?;
                }
            } else {
                self.state.close(view);
            }
            return Ok(true);
        }
        let Some(layout) = self.layout else {
            return Ok(false);
        };
        let (source, token, cancelled) = self.state.begin_open(store, view)?;
        self.commands
            .push(crate::host::HostCommand::OpenSymbolLocation(
                crate::symbol_location_host::Request {
                    source,
                    token,
                    cancelled,
                    target,
                    side,
                    keep_peek,
                    revision: layout.revision,
                    scope: self.scope.clone(),
                    viewport: self.viewport,
                },
            ));
        Ok(true)
    }
}

impl taide_native_ui::editor_locations::Provider for Provider<'_, '_> {
    fn preview_find_visible(&self, view: ViewId) -> bool {
        self.state.preview_find_visible(view)
    }
    fn keyboard_link(&self, store: &EditorStore, view: ViewId) -> Option<(String, usize)> {
        let request = &self.state.hover(view)?.request;
        if !request.keyboard || !request.describes(store, true) {
            return None;
        }
        Some((
            request.token.to_string(),
            taide_native_editor::lsp::position_to_byte(&request.snapshot, request.position).ok()?,
        ))
    }
    fn preserve_focus(&mut self, view: ViewId, focus: taide_native_ui::editor_locations::Focus) {
        if let Some(session) = self.state.sessions.get_mut(&view) {
            session.focus = Some((
                session.focus.map_or(0, |(generation, _)| generation + 1),
                focus,
            ));
        }
    }
    fn clear_preview_chord(&mut self, view: ViewId) {
        if let Some(session) = self.state.sessions.get_mut(&view) {
            session.keymap.clear_chord(self.viewport);
        }
    }
    fn definition_available(&self, store: &EditorStore, view: ViewId) -> bool {
        self.project
            .as_ref()
            .zip(self.lsp)
            .is_some_and(|(project, lsp)| {
                store
                    .views()
                    .get(view)
                    .and_then(|view| store.documents().snapshot(view.document).ok())
                    .is_some_and(|document| {
                        !lsp.location_providers(project, &document, Kind::Definition)
                            .is_empty()
                    })
            })
    }

    fn source_word(
        &self,
        store: &EditorStore,
        view: ViewId,
        byte: usize,
    ) -> Option<std::ops::Range<usize>> {
        let current = store.views().get(view)?;
        let snapshot = store.documents().snapshot(current.document).ok()?;
        let position = byte_to_position(&snapshot, byte).ok()?;
        let line =
            taide_native_editor::editing::line_content_range(&snapshot, position.line as usize);
        let text = snapshot.rope.byte_slice(line.clone()).to_string();
        let word = crate::editor_syntax::language_rules(&snapshot.metadata.language_id)?
            .word_range(&text, byte - line.start)?;
        Some(line.start + word.start..line.start + word.end)
    }

    fn request_at(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
        byte: usize,
        mode: Mode,
    ) -> Result<bool, EditorError> {
        let current = store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .clone();
        let document = store.documents().snapshot(current.document)?;
        let position = byte_to_position(&document, byte)?;
        if mode != Mode::Hover {
            store.set_view_state(
                view,
                SelectionSet {
                    primary: 0,
                    selections: vec![taide_native_editor::view::Selection {
                        anchor: byte,
                        head: byte,
                    }],
                },
                current.scroll,
                current.folds,
            )?;
        }
        self.request(store, view, Kind::Definition, mode, Some(position))
    }

    fn clear_link(&mut self, view: ViewId) {
        self.state.clear_hover(view);
    }

    fn link_preview(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        byte: usize,
    ) -> Option<taide_native_ui::editor_locations::LinkPreview> {
        if let Some((request, paths)) = self.state.preload(store, self.models, view, Mode::Hover) {
            self.commands
                .push(crate::host::HostCommand::ReadPeekModels { request, paths });
        }
        let session = self.state.hover(view)?;
        let current = store.views().get(view)?;
        let snapshot = store.documents().snapshot(current.document).ok()?;
        if session.request.position != byte_to_position(&snapshot, byte).ok()? {
            return None;
        }
        let model = session
            .model
            .as_ref()
            .filter(|model| !model.targets().is_empty())?;
        let word = self.source_word(store, view, byte)?;
        let fallback = lsp_types::Range::new(
            byte_to_position(&snapshot, word.start).ok()?,
            byte_to_position(&snapshot, word.end).ok()?,
        );
        let bytes =
            taide_native_editor::lsp::range_to_bytes(&snapshot, model.origin_range(fallback))
                .ok()?;
        let mut code = None;
        let text = if model.targets().len() > 1 {
            format!("Click to show {} definitions.", model.targets().len())
        } else {
            let target = model.first().and_then(|index| model.targets().get(index))?;
            let document = self
                .preview_document(store, target)
                .and_then(|document| store.documents().snapshot(document).ok());
            document.map_or_else(
                || "Click to go to definition.".into(),
                |document| {
                    let start =
                        (target.range.start.line as usize).min(document.rope.len_lines() - 1);
                    let end = (target.range.end.line as usize + 1)
                        .min(start + HOVER_LINES)
                        .min(document.rope.len_lines());
                    self.shown_lines.push((document.id, start..end));
                    let lines = (start..end)
                        .map(|line| document.rope.line(line).to_string())
                        .collect::<Vec<_>>();
                    let indent = lines
                        .iter()
                        .filter(|line| !line.trim().is_empty())
                        .map(|line| line.len() - line.trim_start_matches([' ', '\t']).len())
                        .min()
                        .unwrap_or(0);
                    let mut job = eframe::egui::text::LayoutJob::default();
                    let tokens = self
                        .hover_tokens
                        .as_ref()
                        .filter(|tokens| {
                            tokens.document == document.id
                                && tokens.revision == document.revision
                                && tokens.language_id == document.metadata.language_id
                        })
                        .and_then(|tokens| tokens.frame());
                    for (offset, line) in lines.iter().enumerate() {
                        let indent = if indent <= line.len() && line.is_char_boundary(indent) {
                            indent
                        } else {
                            0
                        };
                        let spans = tokens
                            .filter(|tokens| tokens.lines.has_accurate_tokens(start + offset))
                            .map(|tokens| {
                                tokens
                                    .lines
                                    .spans(start + offset)
                                    .as_chunks::<HOVER_SPAN_FIELDS>()
                                    .0
                            })
                            .unwrap_or_default();
                        let mut boundaries = spans
                            .iter()
                            .filter_map(|[byte, _]| {
                                let byte = *byte as usize;
                                (byte > indent && byte < line.len() && line.is_char_boundary(byte))
                                    .then_some(byte)
                            })
                            .collect::<Vec<_>>();
                        boundaries.insert(0, indent);
                        boundaries.push(line.len());
                        for pair in boundaries.windows(HOVER_SPAN_FIELDS) {
                            let style = tokens.map(|tokens| {
                                spans
                                    .iter()
                                    .rev()
                                    .find(|[byte, _]| *byte as usize <= pair[0])
                                    .map_or(tokens.styles.default_style(), |[_, style]| {
                                        tokens.styles.style(*style)
                                    })
                            });
                            let color = style.map_or(self.editor.appearance.foreground, |style| {
                                eframe::egui::Color32::from_rgba_unmultiplied(
                                    style.foreground[0],
                                    style.foreground[1],
                                    style.foreground[2],
                                    style.foreground[3],
                                )
                            });
                            let font = style
                                .filter(|style| style.is_bold)
                                .and_then(|_| self.presentation.options.bold_family.as_ref())
                                .map_or_else(
                                    || self.editor.appearance.font.clone(),
                                    |family| {
                                        eframe::egui::FontId::new(
                                            self.editor.appearance.font.size,
                                            family.clone(),
                                        )
                                    },
                                );
                            let stroke = eframe::egui::Stroke::new(TOKEN_DECORATION_WIDTH, color);
                            job.append(
                                &line[pair[0]..pair[1]],
                                0.0,
                                eframe::egui::TextFormat {
                                    font_id: font,
                                    color,
                                    italics: style.is_some_and(|style| style.is_italic),
                                    underline: if style.is_some_and(|style| style.is_underlined) {
                                        stroke
                                    } else {
                                        eframe::egui::Stroke::NONE
                                    },
                                    strikethrough: if style
                                        .is_some_and(|style| style.is_struck_through)
                                    {
                                        stroke
                                    } else {
                                        eframe::egui::Stroke::NONE
                                    },
                                    ..Default::default()
                                },
                            );
                        }
                    }
                    let text = job.text.clone();
                    code = Some(job);
                    text
                },
            )
        };
        Some(taide_native_ui::editor_locations::LinkPreview { bytes, text, code })
    }
    fn current(
        &mut self,
        store: &EditorStore,
        view: ViewId,
    ) -> Option<taide_native_ui::editor_locations::Widget> {
        if let Some((request, paths)) = self.state.preload(store, self.models, view, Mode::Peek) {
            self.commands
                .push(crate::host::HostCommand::ReadPeekModels { request, paths });
        }
        self.state.widget(store, view)
    }

    fn execute(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
        command: taide_native_editor::symbol_locations::Command,
    ) -> Result<bool, EditorError> {
        use taide_native_editor::symbol_locations::Command;
        if !matches!(command, Command::Request { .. }) && !self.state.synchronize(store, view) {
            return Ok(false);
        }
        match command {
            Command::Request { kind, mode } => self.request(store, view, kind, mode, None),
            Command::Close => {
                let visible = self.state.current(view).is_some();
                self.state.close(view);
                Ok(visible)
            }
            Command::Select {
                index,
                focus_preview,
            } => {
                let Some(session) = self.state.sessions.get_mut(&view) else {
                    return Ok(false);
                };
                if session
                    .model
                    .as_ref()
                    .is_none_or(|model| index >= model.targets().len())
                {
                    return Ok(false);
                }
                session.selected = Some(index);
                session.preview_focus |= focus_preview;
                Ok(true)
            }
            Command::Next | Command::Previous => {
                let Some(session) = self.state.sessions.get_mut(&view) else {
                    return Ok(false);
                };
                let Some(model) = session.model.as_ref() else {
                    return Ok(false);
                };
                session.selected =
                    model.next(session.selected.unwrap_or(0), command == Command::Next);
                let target = session
                    .selected
                    .and_then(|index| model.targets().get(index))
                    .cloned();
                if let Some(target) = target {
                    return self.open(store, view, target, false, true);
                }
                Ok(true)
            }
            Command::ToggleFocus => {
                let Some(session) = self.state.sessions.get_mut(&view) else {
                    return Ok(false);
                };
                session.preview_focus = true;
                Ok(true)
            }
            Command::OpenSelected { side } => {
                let Some(target) = self
                    .state
                    .current(view)
                    .and_then(|session| {
                        session
                            .selected
                            .and_then(|index| session.model.as_ref()?.targets().get(index))
                    })
                    .cloned()
                else {
                    return Ok(false);
                };
                self.open(store, view, target, side, false)
            }
            Command::GotoSelected => {
                let Some(session) = self.state.current(view) else {
                    return Ok(false);
                };
                let keep = session.request.mode == Mode::Peek;
                let Some(target) = session
                    .selected
                    .and_then(|index| session.model.as_ref()?.targets().get(index))
                    .cloned()
                else {
                    return Ok(false);
                };
                self.open(store, view, target, false, keep)
            }
        }
    }

    fn render_preview(
        &mut self,
        ui: &mut eframe::egui::Ui,
        store: &mut EditorStore,
        view: ViewId,
        rect: eframe::egui::Rect,
        focus: bool,
    ) -> Result<Vec<eframe::egui::Id>, EditorError> {
        let document = self.state.prepare_preview(store, view)?;
        let Some(document) = document else {
            ui.painter().with_clip_rect(rect).text(
                rect.center(),
                eframe::egui::Align2::CENTER_CENTER,
                "no preview available",
                self.editor.appearance.font.clone(),
                self.editor.appearance.foreground,
            );
            return Ok(Vec::new());
        };
        let session = self
            .state
            .sessions
            .get_mut(&view)
            .ok_or(EditorError::NotFound)?;
        let preview = session.preview.ok_or(EditorError::NotFound)?;
        let mut focus = focus || std::mem::take(&mut session.preview_focus);
        let mut ui = ui.new_child(
            eframe::egui::UiBuilder::new()
                .id_salt(("location-preview", view))
                .max_rect(rect),
        );
        ui.set_clip_rect(rect.intersect(ui.clip_rect()));
        let mut presentation = self.presentation.clone();
        presentation.options.minimap = false;
        presentation.options.sticky_scroll = false;
        presentation.options.scroll_beyond_last_line = false;
        presentation.options.location_colors = None;
        presentation.options.problem_colors = None;
        store.configure_indentation(document, self.indentation)?;
        let snapshot = store.documents().snapshot(document)?;
        let tokens = self.tokens.as_ref().filter(|tokens| {
            tokens.document == document
                && tokens.revision == snapshot.revision
                && tokens.language_id == snapshot.metadata.language_id
        });
        let syntax = tokens
            .map(|tokens| {
                tokens.as_ref() as &dyn taide_native_editor::language_configuration::LineSyntax
            })
            .unwrap_or(&taide_native_editor::language_configuration::UntokenizedLines);
        let language = crate::editor_syntax::language_rules(&snapshot.metadata.language_id)
            .map(|rules| taide_native_editor::language_configuration::Language { rules, syntax });
        let rules = language
            .map(|language| language.rules)
            .or_else(|| crate::editor_syntax::language_rules("plaintext"));
        let compiler = taide_native_syntax::MonacoFindPatternCompiler;
        let before_find = snapshot.revision;
        let mut find_focus = None;
        if let Some(history) = self.find_history.as_deref_mut()
            && let Some(appearance) = self.find_appearance
        {
            let mut queued = Vec::new();
            let mut next = 0;
            let keymap = &mut session.keymap;
            let output = taide_native_ui::editor_find_widget::show(
                &mut ui,
                &mut session.find,
                history,
                store,
                preview,
                &compiler,
                rules,
                appearance,
                |ui, event, composing| {
                    let index = crate::keymap::event_index(ui.ctx(), event, &mut next);
                    keymap
                        .route(
                            crate::keymap::Route {
                                context: ui.ctx(),
                                event,
                                index,
                                scope: crate::keymap::Context {
                                    editor: true,
                                    terminal: false,
                                },
                                composing,
                                overrides: self.overrides,
                            },
                            |decision| {
                                let id = match decision {
                                    crate::keymap::Decision::Dispatch(id)
                                    | crate::keymap::Decision::ResolveChord(id) => id,
                                    crate::keymap::Decision::EnterChord => return true,
                                    _ => return false,
                                };
                                let run = crate::command_registry::registry()
                                    .ok()
                                    .and_then(|registry| registry.command(id))
                                    .and_then(|command| {
                                        if let crate::command_registry::Execution::Native(run) =
                                            command.execution
                                        {
                                            Some(run)
                                        } else {
                                            None
                                        }
                                    })
                                    .or_else(|| crate::command_registry::keymap_run(id));
                                if let Some(
                                    run @ (crate::command_registry::Run::EditDocument(
                                        crate::command_registry::DocumentEdit::Find(_),
                                    )
                                    | crate::command_registry::Run::SaveActiveTab),
                                ) = run
                                {
                                    queued.push(run);
                                    return true;
                                }
                                false
                            },
                        )
                        .unwrap_or(false)
                },
            )
            .map_err(|_| EditorError::InvalidBoundary)?;
            for run in queued {
                match run {
                    crate::command_registry::Run::EditDocument(
                        crate::command_registry::DocumentEdit::Find(command),
                    ) => {
                        session
                            .find
                            .execute(command, store, preview, &compiler, rules)
                            .map_err(|_| EditorError::InvalidBoundary)?;
                    }
                    crate::command_registry::Run::SaveActiveTab => {
                        let snapshot = store.save_snapshot(document)?;
                        if let taide_native_editor::document::DocumentKey::File(path) =
                            snapshot.key()
                        {
                            self.commands.push(crate::host::HostCommand::Save {
                                path: path.to_string_lossy().into_owned(),
                                snapshot,
                            });
                        }
                    }
                    _ => {}
                }
            }
            focus |= output.request_editor_focus;
            find_focus = ui
                .memory(|memory| memory.focused())
                .filter(|_| output.focused);
            presentation.options.bracket_widget_focus = output.focused;
            if output.reserved_height > 0.0 {
                ui.allocate_space(eframe::egui::vec2(
                    ui.available_width(),
                    output.reserved_height,
                ));
            }
        }
        let colors = self.presentation.options.location_colors;
        let matches = session
            .model
            .as_ref()
            .into_iter()
            .flat_map(|model| model.targets())
            .filter(|target| {
                file_path(&target.uri).is_some_and(|path| {
                    snapshot.key == taide_native_editor::document::DocumentKey::File(path)
                })
            })
            .filter_map(|target| {
                Some(Decoration {
                    bytes: taide_native_editor::lsp::range_to_bytes(&snapshot, target.selection)
                        .ok()?,
                    kind: DecorationKind::Inline(InlineStyle {
                        background: colors.map(|colors| colors.highlight.to_array()),
                        ..Default::default()
                    }),
                    stickiness: Stickiness::NeverGrowsWhenTypingAtEdges,
                })
            })
            .collect();
        let matches = DecorationLayer::new(snapshot.revision, 0, matches);
        let find_decorations = self
            .find_appearance
            .map(|appearance| {
                session.find.decorations(
                    &store.views().get(preview).unwrap().selection,
                    appearance.highlight,
                    appearance.current_match,
                    appearance.scope,
                )
            })
            .unwrap_or_default();
        let document_highlights = self
            .highlights
            .as_ref()
            .and_then(|consumer| consumer.display(store, preview));
        let decorations = std::iter::once(&matches)
            .chain(find_decorations.iter())
            .chain(document_highlights.as_ref().map(|display| &display.layer))
            .collect::<Vec<_>>();
        let tab = store
            .views()
            .get(preview)
            .ok_or(EditorError::NotFound)?
            .key
            .tab
            .clone();
        let text_resources =
            crate::editor_command_text::resources().map_err(|_| EditorError::InvalidBoundary)?;
        let compare = |left: &str, right: &str| text_resources.compare(left, right);
        let keymap = &mut session.keymap;
        let commands = &mut self.commands;
        let mut errors = Vec::new();
        let mut next = 0;
        let fold_commands = std::mem::take(&mut session.folds);
        let pending_folds = &mut session.folds;
        let documentation_commands = std::mem::take(&mut session.documentation);
        let pending_documentation = &mut session.documentation;
        let mut documentation_host_commands = Vec::new();
        let mut documentation_provider =
            self.documentation
                .as_ref()
                .map(|state| crate::editor_documentation::Provider {
                    state: state.clone(),
                    project: self.project.clone(),
                    owner: view,
                    owner_key: session.request.source_key.clone(),
                    lsp: self.lsp,
                    viewport: self.viewport,
                    commands: &mut documentation_host_commands,
                    appearance: &self.editor.appearance,
                    language: snapshot.metadata.language_id.clone(),
                });
        let find = &mut session.find;
        let completion_consumer = self.completion.clone();
        let highlight_consumer = self.highlights.clone();
        let mut completion_host_commands = Vec::new();
        let mut completion_provider =
            self.completion
                .as_ref()
                .map(|consumer| crate::editor_completion::Provider {
                    consumer: consumer.clone(),
                    project: self.project.clone(),
                    owner: view,
                    owner_key: session.request.source_key.clone(),
                    lsp: self.lsp,
                    viewport: self.viewport,
                    commands: &mut completion_host_commands,
                    editor: self.editor,
                    syntax,
                    model_path: match &snapshot.key {
                        taide_native_editor::document::DocumentKey::File(path) => {
                            path.to_string_lossy().into_owned()
                        }
                        taide_native_editor::document::DocumentKey::Untitled(tab) => {
                            tab.to_string()
                        }
                        taide_native_editor::document::DocumentKey::AppFile(_) => String::new(),
                    },
                    language: snapshot.metadata.language_id.clone(),
                });
        let has_find = self.find_appearance.is_some() && self.find_history.is_some();
        let output = self.editor.show_request_with_editor_keymap(
            &mut ui,
            store,
            preview,
            taide_native_ui::editor_surface::EditorRequest {
                request_focus: focus,
                keymap: |_: &eframe::egui::Ui, _: &eframe::egui::Event, _: bool| false,
                route: |response: &eframe::egui::Response| {
                    response.ctx.keyboard_input_route(response.id)
                },
                presentation: &presentation,
                tokens: |_: &EditorStore| tokens.and_then(|tokens| tokens.frame()),
                language,
                decorations: &decorations,
                fold_commands: &fold_commands,
                fold_controls: None,
                problems: None,
                locations: None,
                syntax_folds: None,
                documentation: documentation_provider.as_mut().map(|provider| {
                    provider as &mut dyn taide_native_ui::editor_documentation::Provider
                }),
                documentation_commands: &documentation_commands,
                completion: completion_provider.as_mut().map(|provider| {
                    provider as &mut dyn taide_native_ui::editor_completion::Provider
                }),
                completion_commands: &[],
            },
            |ui, store, preview, event, composing| {
                let index = crate::keymap::event_index(ui.ctx(), event, &mut next);
                keymap
                    .route_editor(
                        crate::keymap::Route {
                            context: ui.ctx(),
                            event,
                            index,
                            scope: crate::keymap::Context {
                                editor: true,
                                terminal: false,
                            },
                            composing,
                            overrides: self.overrides,
                        },
                        completion_consumer
                            .as_ref()
                            .is_some_and(|consumer| consumer.defers(preview, event)),
                        |decision| {
                            let id = match decision {
                                crate::keymap::Decision::Dispatch(id)
                                | crate::keymap::Decision::ResolveChord(id) => id,
                                crate::keymap::Decision::EnterChord
                                | crate::keymap::Decision::NoMatch => return true,
                                _ => return false,
                            };
                            if let Some(consumer) = &completion_consumer
                                && let Some(command) = id
                                    .strip_prefix("monaco.")
                                    .and_then(taide_native_editor::completion::Command::from_action)
                            {
                                consumer.queue_command(preview, command);
                                return true;
                            }
                            if id == "save" {
                                if let Ok(snapshot) = store.save_snapshot(document)
                                    && let taide_native_editor::document::DocumentKey::File(path) =
                                        snapshot.key()
                                {
                                    commands.push(crate::host::HostCommand::Save {
                                        path: path.to_string_lossy().into_owned(),
                                        snapshot,
                                    });
                                }
                                return true;
                            }
                            let Some(run) = crate::command_registry::registry()
                                .ok()
                                .and_then(|registry| registry.command(id))
                                .and_then(|command| {
                                    if let crate::command_registry::Execution::Native(run) =
                                        command.execution
                                    {
                                        Some(run)
                                    } else {
                                        None
                                    }
                                })
                                .or_else(|| crate::command_registry::keymap_run(id))
                            else {
                                return false;
                            };
                            if let crate::command_registry::Run::FoldDocument(command) = run {
                                pending_folds.push(command);
                                ui.ctx().request_repaint();
                                return true;
                            }
                            let crate::command_registry::Run::EditDocument(edit) = run else {
                                return false;
                            };
                            if let crate::command_registry::DocumentEdit::Highlight(command) = edit
                            {
                                let Some(consumer) = &highlight_consumer else {
                                    return false;
                                };
                                return match consumer.execute(
                                    store,
                                    self.project.as_ref(),
                                    view,
                                    preview,
                                    command,
                                ) {
                                    Ok(handled) => handled,
                                    Err(error) => {
                                        errors.push(error);
                                        true
                                    }
                                };
                            }
                            if let crate::command_registry::DocumentEdit::Documentation(command) =
                                edit
                            {
                                pending_documentation.push(command);
                                ui.ctx().request_repaint();
                                return true;
                            }
                            if let crate::command_registry::DocumentEdit::Find(command) = edit {
                                if !has_find {
                                    return false;
                                }
                                if find
                                    .execute(command, store, preview, &compiler, rules)
                                    .is_err()
                                {
                                    errors.push(EditorError::InvalidBoundary);
                                }
                                ui.ctx().request_repaint();
                                return true;
                            }
                            if matches!(
                                edit,
                                crate::command_registry::DocumentEdit::Problem(_)
                                    | crate::command_registry::DocumentEdit::Location(_)
                            ) {
                                return false;
                            }
                            let mut pending = vec![(tab.clone(), edit)];
                            crate::command_dispatch::apply_document_edits(
                                store,
                                preview,
                                &tab,
                                taide_native_editor::line_commands::LineCommandContext {
                                    indent: self.editor.indent_options(&snapshot),
                                    language,
                                    syntax,
                                    compare: Some(&compare),
                                    transforms: Some(&text_resources.transforms),
                                    word_rules: language.map(|language| language.rules).or_else(
                                        || crate::editor_syntax::language_rules("plaintext"),
                                    ),
                                },
                                self.indentation,
                                &mut pending,
                                &mut errors,
                            );
                            true
                        },
                    )
                    .unwrap_or(false)
            },
        )?;
        self.commands.append(&mut documentation_host_commands);
        self.commands.append(&mut completion_host_commands);
        if let Some(error) = errors.into_iter().next() {
            return Err(error);
        }
        self.shown_lines
            .push((document, output.rendered_lines.clone()));
        self.focus_targets.extend(
            std::iter::once(output.response.id)
                .chain(output.focus_ids.iter().copied())
                .map(|id| (id, preview)),
        );
        if let Some(display) = &document_highlights {
            display.paint(&ui, &output.geometry);
        }
        if output.response.has_focus()
            || ui.memory(|memory| output.focus_ids.iter().any(|id| memory.has_focus(*id)))
        {
            if let Some(consumer) = &self.highlights {
                consumer.observe(store, self.project.as_ref(), view, preview);
            }
        }
        if output.changed || store.documents().snapshot(document)?.revision != before_find {
            self.changed.insert(document, preview);
        }
        if let Some(colors) = colors
            && let Some(matches) =
                matches.tracking(store.changes_since(document, matches.revision())?)
        {
            let painter = ui.painter().with_clip_rect(output.geometry.content_rect);
            const MATCH_BORDER_WIDTH: f32 = 2.0;
            for decoration in matches.items() {
                for rect in output.geometry.range_rects(decoration.bytes.clone()) {
                    painter.rect_stroke(
                        rect,
                        0.0,
                        eframe::egui::Stroke::new(MATCH_BORDER_WIDTH, colors.highlight_border),
                        eframe::egui::StrokeKind::Inside,
                    );
                }
            }
        }
        if output.response.double_clicked() {
            let head = store
                .views()
                .get(preview)
                .ok_or(EditorError::NotFound)?
                .selection
                .selections[0]
                .head;
            let position = byte_to_position(&store.documents().snapshot(document)?, head)?;
            if let Some(mut target) = session
                .selected
                .and_then(|index| session.model.as_ref()?.targets().get(index))
                .cloned()
            {
                target.selection = lsp_types::Range::new(position, position);
                target.range = target.selection;
                self.open(
                    store,
                    view,
                    target,
                    ui.input(|input| {
                        input.modifiers.command || input.modifiers.ctrl || input.modifiers.alt
                    }),
                    false,
                )?;
            }
        }
        if let Some(id) = find_focus {
            self.focus_targets.push((id, preview));
        }
        Ok(std::iter::once(output.response.id)
            .chain(output.focus_ids)
            .chain(find_focus)
            .collect())
    }

    fn preview_document(
        &self,
        store: &EditorStore,
        target: &Target,
    ) -> Option<taide_native_editor::document::DocumentId> {
        store
            .documents()
            .find(&taide_native_editor::document::DocumentKey::File(
                file_path(&target.uri)?,
            ))
    }

    fn file_label(&self, target: &Target) -> String {
        file_path(&target.uri)
            .and_then(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| target.uri.as_str().into())
    }

    fn file_description(&self, target: &Target) -> String {
        file_path(&target.uri)
            .and_then(|path| {
                path.parent()
                    .map(|parent| parent.to_string_lossy().into_owned())
            })
            .unwrap_or_default()
    }

    fn word_start(
        &self,
        store: &EditorStore,
        document: taide_native_editor::document::DocumentId,
        byte: usize,
    ) -> usize {
        let Ok(snapshot) = store.documents().snapshot(document) else {
            return byte;
        };
        let Ok(position) = byte_to_position(&snapshot, byte) else {
            return byte;
        };
        let line = position.line as usize;
        let range = taide_native_editor::editing::line_content_range(&snapshot, line);
        let text = snapshot.rope.byte_slice(range.clone()).to_string();
        crate::editor_syntax::language_rules(&snapshot.metadata.language_id)
            .and_then(|rules| rules.word_range(&text, byte - range.start))
            .map_or(byte, |word| range.start + word.start)
    }
}

#[cfg(test)]
#[path = "editor-locations-tests.rs"]
mod tests;

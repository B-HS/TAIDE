use std::collections::HashMap;
use std::sync::Arc;

use taide_model::ids::ProjectId;
use taide_model::layout::{ProjectLayout, TabKind};
use taide_native_editor::diagnostics::MarkerSet;
use taide_native_editor::document::{DocumentId, DocumentKey, EditorError};
use taide_native_editor::problem_navigation::{Command, Coordinate, Navigation};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{Selection, SelectionSet, ViewId, ViewKey};
use taide_native_ui::editor_problems::Widget;
use uuid::Uuid;

#[derive(Clone)]
pub struct Request {
    pub(crate) token: Uuid,
    pub(crate) source: ViewId,
    pub(crate) source_key: ViewKey,
    pub(crate) source_document: DocumentId,
    pub(crate) project: ProjectId,
    pub(crate) path: String,
    pub(crate) line: u64,
    pub(crate) column: u64,
    pub(crate) viewport: eframe::egui::ViewportId,
    coordinate: Coordinate,
    markers: MarkerSet,
    all_files: bool,
}

impl Request {
    pub(crate) fn source_is_active(&self, layout: &ProjectLayout) -> bool {
        pane_is_focused(layout, &self.source_key.pane)
            && taide_layout::service::all_roots(layout).any(|root| {
                taide_native_ui::snapshot::active_tab(root, &self.source_key.pane)
                    .is_some_and(|tab| tab.id == self.source_key.tab)
            })
    }
}

fn pane_is_focused(layout: &ProjectLayout, pane: &taide_model::ids::PaneId) -> bool {
    if taide_layout::service::find_leaf(&layout.root, pane).is_some() {
        return layout.focused_pane == *pane;
    }
    layout.auxiliary_windows.iter().any(|window| {
        window.focused_pane == *pane
            && taide_layout::service::find_leaf(&window.root, pane).is_some()
    })
}

fn destination_is_active(opened: &crate::terminal_tabs::OpenedFileLink) -> bool {
    pane_is_focused(&opened.layout, &opened.pane)
        && taide_layout::service::all_roots(&opened.layout).any(|root| {
            taide_native_ui::snapshot::active_tab(root, &opened.pane).is_some_and(|tab| {
                tab.id == opened.tab
                    && matches!(&tab.kind, TabKind::File { path } if path == &opened.path)
            })
        })
}

pub(crate) fn reply_is_current(
    opened: &crate::terminal_tabs::OpenedFileLink,
    layouts: &HashMap<ProjectId, ProjectLayout>,
) -> bool {
    destination_is_active(opened)
        && layouts
            .get(&opened.project)
            .is_some_and(|layout| layout == &opened.layout)
}

struct Shown {
    coordinate: Coordinate,
    markers: MarkerSet,
    revision: Arc<()>,
}

#[derive(Default)]
struct Session {
    navigation: Navigation,
    all_files: bool,
    shown: Option<Shown>,
}

#[derive(Default)]
pub(crate) struct State {
    sessions: HashMap<ViewId, Session>,
    pending: HashMap<ViewId, Request>,
}

impl State {
    pub(crate) fn retain(&mut self, store: &EditorStore) {
        self.sessions
            .retain(|view, _| store.views().get(*view).is_some());
        self.pending.retain(|view, request| {
            store.views().get(*view).is_some_and(|state| {
                state.key == request.source_key && state.document == request.source_document
            })
        });
    }

    pub(crate) fn accepts(&self, request: &Request, store: &EditorStore) -> bool {
        self.pending
            .get(&request.source)
            .is_some_and(|pending| pending.token == request.token)
            && store.views().get(request.source).is_some_and(|view| {
                view.key == request.source_key && view.document == request.source_document
            })
    }

    pub(crate) fn failed(&mut self, request: &Request) {
        if self
            .pending
            .get(&request.source)
            .is_some_and(|pending| pending.token == request.token)
        {
            self.pending.remove(&request.source);
        }
    }

    pub(crate) fn opened(
        &mut self,
        request: &Request,
        opened: &crate::terminal_tabs::OpenedFileLink,
        store: &mut EditorStore,
        diagnostics: &crate::diagnostics::Store,
    ) -> Result<bool, EditorError> {
        if !self.accepts(request, store)
            || opened.project != request.project
            || opened.path != request.path
            || opened.viewport != request.viewport
            || !destination_is_active(opened)
        {
            return Ok(false);
        }
        self.pending.remove(&request.source);
        let Ok(document) = store
            .documents()
            .snapshot(request.coordinate.problem.document)
        else {
            return Ok(false);
        };
        let Some(markers) = request.markers.tracked(
            &document,
            store.changes_since(document.id, request.markers.revision())?,
        ) else {
            return Ok(false);
        };
        let marker = &markers.markers()[0];
        let mut entries = diagnostics.problems(store);
        if !request.all_files {
            entries.retain(|entry| entry.document == document.id);
        }
        let mut navigation = Navigation::default();
        navigation.update(entries);
        let mut problem = request.coordinate.problem.clone();
        problem.marker = marker.clone();
        let Some(coordinate) = navigation.select(&problem) else {
            return Ok(false);
        };
        let view = store.attach_view(
            ViewKey {
                window: request.source_key.window.clone(),
                pane: opened.pane.clone(),
                tab: opened.tab.clone(),
            },
            document.id,
        )?;
        let session = self.sessions.entry(view).or_default();
        session.navigation = navigation;
        session.all_files = request.all_files;
        show(store, view, session, coordinate, diagnostics)?;
        Ok(true)
    }
}

pub(crate) struct Provider<'a> {
    pub(crate) state: &'a mut State,
    pub(crate) diagnostics: &'a crate::diagnostics::Store,
    pub(crate) project: Option<ProjectId>,
    pub(crate) commands: &'a mut Vec<crate::host::HostCommand>,
    pub(crate) viewport: eframe::egui::ViewportId,
}

impl taide_native_ui::editor_problems::Provider for Provider<'_> {
    fn current(&mut self, store: &EditorStore, view: ViewId) -> Option<Widget> {
        let session = self.state.sessions.get_mut(&view)?;
        let current = store.views().get(view)?;
        let document = store.documents().snapshot(current.document).ok()?;
        let shown = session.shown.as_mut()?;
        let markers = shown.markers.tracked(
            &document,
            store
                .changes_since(document.id, shown.markers.revision())
                .ok()?,
        )?;
        let marker = markers.markers().first()?.clone();
        if !Arc::ptr_eq(&shown.revision, self.diagnostics.revision()) {
            let mut navigation = Navigation::default();
            let entries = self
                .diagnostics
                .problems(store)
                .into_iter()
                .filter(|entry| session.all_files || entry.document == document.id)
                .collect();
            navigation.update(entries);
            let mut problem = shown.coordinate.problem.clone();
            problem.marker = marker.clone();
            let Some(selected) = navigation.select(&problem) else {
                session.shown = None;
                return None;
            };
            shown.coordinate = selected;
            shown.revision = self.diagnostics.revision().clone();
            session.navigation.reset();
        }
        shown.coordinate.problem.marker = marker;
        let selection = current.selection.selections[current.selection.primary];
        let bytes = &shown.coordinate.problem.marker.bytes;
        let position = if bytes.start <= selection.head && selection.head <= bytes.end {
            selection.head
        } else {
            bytes.start
        };
        let DocumentKey::File(path) = &document.key else {
            return None;
        };
        let title = path.file_name()?.to_string_lossy().into_owned();
        Some(Widget {
            coordinate: shown.coordinate.clone(),
            position,
            title,
        })
    }

    fn execute(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
        command: Command,
    ) -> Result<bool, EditorError> {
        let current = store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .clone();
        let document = store.documents().snapshot(current.document)?;
        let DocumentKey::File(path) = &document.key else {
            return Ok(false);
        };
        if command == Command::Close {
            let pending = self.state.pending.remove(&view).is_some();
            let shown = self
                .state
                .sessions
                .remove(&view)
                .is_some_and(|session| session.shown.is_some());
            return Ok(pending || shown);
        }
        self.state.pending.remove(&view);
        let resource = url::Url::from_file_path(path).map_err(|_| EditorError::InvalidBoundary)?;
        let session = self.state.sessions.entry(view).or_default();
        if session.all_files != command.all_files() {
            *session = Session {
                all_files: command.all_files(),
                ..Default::default()
            };
        }
        session.navigation.update(
            self.diagnostics
                .problems(store)
                .into_iter()
                .filter(|entry| session.all_files || entry.document == document.id)
                .collect(),
        );
        let head = current.selection.selections[current.selection.primary].head;
        session.navigation.follow_cursor(document.id, head);
        let Some(coordinate) =
            session
                .navigation
                .navigate(resource.as_str(), head, command.forward())
        else {
            session.shown = None;
            return Ok(true);
        };
        if coordinate.problem.document == document.id {
            show(store, view, session, coordinate, self.diagnostics)?;
            return Ok(true);
        }
        let Some(project) = self.project.clone() else {
            return Ok(false);
        };
        let target = store.documents().snapshot(coordinate.problem.document)?;
        let DocumentKey::File(path) = &target.key else {
            return Ok(false);
        };
        let position = taide_native_editor::lsp::byte_to_position(
            &target,
            coordinate.problem.marker.bytes.start,
        )?;
        let request = Request {
            token: Uuid::new_v4(),
            source: view,
            source_key: current.key,
            source_document: document.id,
            project,
            path: path.to_string_lossy().into_owned(),
            line: u64::from(position.line) + 1,
            column: u64::from(position.character) + 1,
            viewport: self.viewport,
            markers: MarkerSet::new(&target, vec![coordinate.problem.marker.clone()])?,
            coordinate,
            all_files: session.all_files,
        };
        session.shown = None;
        self.state.pending.insert(view, request.clone());
        self.commands
            .push(crate::host::HostCommand::OpenMarker(request));
        Ok(true)
    }
}

fn show(
    store: &mut EditorStore,
    view: ViewId,
    session: &mut Session,
    coordinate: Coordinate,
    diagnostics: &crate::diagnostics::Store,
) -> Result<(), EditorError> {
    let current = store
        .views()
        .get(view)
        .ok_or(EditorError::NotFound)?
        .clone();
    let document = store.documents().snapshot(current.document)?;
    let bytes = &coordinate.problem.marker.bytes;
    let head = current.selection.selections[current.selection.primary].head;
    let position = if bytes.start <= head && head <= bytes.end {
        head
    } else {
        bytes.start
    };
    let folds = current
        .folds
        .into_iter()
        .filter(|fold| !fold.contains(&position))
        .collect();
    store.set_view_state(
        view,
        SelectionSet {
            primary: 0,
            selections: vec![Selection {
                anchor: position,
                head: position,
            }],
        },
        current.scroll,
        folds,
    )?;
    store.request_selection_reveal(view, position..position, true)?;
    session.shown = Some(Shown {
        markers: MarkerSet::new(&document, vec![coordinate.problem.marker.clone()])?,
        coordinate,
        revision: diagnostics.revision().clone(),
    });
    Ok(())
}

#[cfg(test)]
#[path = "editor-problems-tests.rs"]
mod tests;

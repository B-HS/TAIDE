use std::cell::RefCell;
use std::rc::Rc;

use taide_lsp::native::Failure;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_native_editor::document::EditorError;
use taide_native_editor::lsp::byte_to_position;
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::ViewId;
use tokio::sync::oneshot;

use crate::editor_rename::{Request, Response, State};
use crate::lsp::LspBridge;

struct Pending {
    request: Request,
    receive: oneshot::Receiver<Result<Response, Failure>>,
}

struct Applying {
    request: Request,
    receive: oneshot::Receiver<AppResult<()>>,
}

#[derive(Default)]
pub(crate) struct Controller {
    pub(crate) state: State,
    pending: Option<Pending>,
    applying: Option<Applying>,
    error: Option<AppError>,
}

impl Controller {
    pub(crate) fn is_applying(&self) -> bool {
        self.applying.is_some()
    }
    pub(crate) fn clear(&mut self) {
        self.state.clear();
        self.pending = None;
        self.applying = None;
    }

    pub(crate) fn reconcile(
        &mut self,
        store: &EditorStore,
        lsp: &LspBridge,
        active: impl Fn(&Request) -> bool,
    ) {
        self.state.reconcile(store, |request| {
            if active(request) {
                lsp.rename_providers(&request.project, &request.snapshot)
            } else {
                Default::default()
            }
        });
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.request.is_cancelled())
        {
            self.pending = None;
        }
    }

    pub(crate) fn begin(
        &mut self,
        store: &EditorStore,
        project: ProjectId,
        source: ViewId,
        owner: ViewId,
        lsp: &LspBridge,
    ) -> Result<bool, EditorError> {
        if self.is_applying() {
            return Ok(false);
        }
        self.clear();
        let document = store
            .views()
            .get(source)
            .ok_or(EditorError::NotFound)?
            .document;
        let providers = lsp.rename_providers(&project, &store.documents().snapshot(document)?);
        let Some(request) = self.state.begin(store, project, source, owner, providers)? else {
            return Ok(false);
        };
        self.submit(lsp, request);
        Ok(true)
    }

    fn submit(&mut self, lsp: &LspBridge, request: Request) {
        match lsp.rename_editor(request.clone()) {
            Ok(receive) => self.pending = Some(Pending { request, receive }),
            Err(error) => {
                self.state.finish(&request);
                self.error = Some(error);
            }
        }
    }

    pub(crate) fn poll(&mut self, store: &mut EditorStore, lsp: &LspBridge) -> Option<AppError> {
        if let Some(mut pending) = self.pending.take() {
            if !pending.request.is_cancelled() {
                let response = match pending.receive.try_recv() {
                    Ok(response) => Some(response),
                    Err(oneshot::error::TryRecvError::Empty) => None,
                    Err(oneshot::error::TryRecvError::Closed) => {
                        Some(Err(Failure::TransportClosed))
                    }
                };
                if let Some(response) = response {
                    let providers =
                        lsp.rename_providers(&pending.request.project, &pending.request.snapshot);
                    match self
                        .state
                        .accept(store, &pending.request, providers, response)
                    {
                        Ok(Some(edits)) => {
                            match lsp.apply_editor_rename(pending.request.clone(), edits) {
                                Ok(receive) => {
                                    self.applying = Some(Applying {
                                        request: pending.request,
                                        receive,
                                    })
                                }
                                Err(error) => {
                                    self.state.finish(&pending.request);
                                    self.error = Some(error);
                                }
                            }
                        }
                        Ok(None) => {
                            if let Some((request, prepared)) = self.state.prepared() {
                                let _ = store.request_selection_reveal(
                                    request.source,
                                    prepared.range.clone(),
                                    true,
                                );
                            }
                        }
                        Err(error) => {
                            self.error =
                                Some(AppError::Internal(format!("native rename: {error:?}")))
                        }
                    }
                } else {
                    self.pending = Some(pending);
                }
            }
        }
        if let Some(mut applying) = self.applying.take() {
            match applying.receive.try_recv() {
                Ok(result) => {
                    self.state.finish(&applying.request);
                    if let Err(error) = result {
                        self.error = Some(error);
                    }
                }
                Err(oneshot::error::TryRecvError::Empty) => self.applying = Some(applying),
                Err(oneshot::error::TryRecvError::Closed) => {
                    self.state.finish(&applying.request);
                    self.error = Some(AppError::Internal("native rename: transport closed".into()));
                }
            }
        }
        self.error.take()
    }

    fn session(
        &self,
        store: &EditorStore,
        view: ViewId,
    ) -> Option<taide_native_ui::editor_rename::Session> {
        let (request, prepared) = self.state.prepared()?;
        if request.source != view || !request.describes(store) {
            return None;
        }
        let start = byte_to_position(&request.snapshot, prepared.range.start).ok()?;
        let end = byte_to_position(&request.snapshot, prepared.range.end).ok()?;
        let primary = request
            .selection
            .selections
            .get(request.selection.primary)?;
        let selected = primary.anchor.min(primary.head)..primary.anchor.max(primary.head);
        let selection_start = byte_to_position(&request.snapshot, selected.start).ok()?;
        let selection_end = byte_to_position(&request.snapshot, selected.end).ok()?;
        let selection = if !selected.is_empty()
            && selection_start.line == selection_end.line
            && prepared.range.start <= selected.start
            && selected.end <= prepared.range.end
        {
            char_offset(
                &prepared.name,
                selection_start.character.saturating_sub(start.character),
            )
                ..char_offset(
                    &prepared.name,
                    selection_end.character.saturating_sub(start.character),
                )
        } else {
            0..prepared.name.chars().count()
        };
        Some(taide_native_ui::editor_rename::Session {
            token: request.token.as_u128(),
            range: prepared.range.clone(),
            name: prepared.name.clone(),
            selection,
            columns: end.character.saturating_sub(start.character) as usize,
        })
    }

    pub(crate) fn local_key(&self, source: ViewId, event: &eframe::egui::Event) -> bool {
        self.state
            .prepared()
            .is_some_and(|(request, _)| request.source == source)
            && matches!(
                event,
                eframe::egui::Event::Key {
                    key: eframe::egui::Key::Enter | eframe::egui::Key::Escape,
                    ..
                }
            )
    }
}

fn char_offset(text: &str, position: u32) -> usize {
    let mut offset = 0;
    text.chars()
        .take_while(|character| {
            offset += character.len_utf16() as u32;
            offset <= position
        })
        .count()
}

#[derive(Clone)]
pub(crate) struct Consumer<'a>(pub(crate) Rc<RefCell<&'a mut Controller>>);

pub(crate) struct Provider<'a, 'state> {
    pub(crate) consumer: Consumer<'state>,
    pub(crate) lsp: Option<&'a LspBridge>,
}

impl taide_native_ui::editor_rename::Provider for Provider<'_, '_> {
    fn current(
        &self,
        store: &EditorStore,
        view: ViewId,
    ) -> Option<taide_native_ui::editor_rename::Session> {
        self.consumer.0.borrow().session(store, view)
    }

    fn accept(&mut self, store: &EditorStore, view: ViewId, token: u128, name: String) {
        let mut controller = self.consumer.0.borrow_mut();
        if !controller
            .session(store, view)
            .is_some_and(|session| session.token == token)
        {
            return;
        }
        if let Some(request) = controller.state.rename(store, name) {
            if let Some(lsp) = self.lsp {
                controller.submit(lsp, request);
            } else {
                controller.state.finish(&request);
            }
        }
    }

    fn cancel(&mut self, view: ViewId, token: u128) {
        let mut controller = self.consumer.0.borrow_mut();
        if controller
            .state
            .prepared()
            .is_some_and(|(request, _)| request.source == view && request.token.as_u128() == token)
        {
            controller.clear();
        }
    }
}

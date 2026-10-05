use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use serde_json::{Value, json};
use taide_model::{
    app::AppFileTarget,
    error::{AppError, AppResult},
    ids::ProjectId,
};
use taide_native_editor::{
    document::{DocumentId, DocumentKey, EditorError},
    store::{EditorStore, SaveSnapshot},
    view::{ViewId, ViewKey},
};

use crate::{
    InvokeError, ResponsePayload,
    settings_catalog::{decode, invocation_error, json_payload},
    shell::Call,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Owner {
    pub project: ProjectId,
    pub key: ViewKey,
    pub target: AppFileTarget,
}

impl Owner {
    pub fn exists_in(&self, snapshot: &taide_native_ui::snapshot::ShellSnapshot) -> bool {
        snapshot.project(&self.project).is_some()
            && snapshot.layouts.get(&self.project).is_some_and(|layout| {
                std::iter::once(&layout.root).chain(layout.auxiliary_windows.iter().map(|window|&window.root)).any(|root| {
                    matches!(taide_model::layout::find_leaf(root,&self.key.pane),Some(taide_model::layout::PaneNode::Leaf { tabs,.. }) if tabs.iter().any(|tab|tab.id == self.key.tab && matches!(tab.kind,taide_model::layout::TabKind::AppFile { target } if target == self.target)))
                })
            })
    }
}

struct Entry {
    mount: Arc<()>,
    attempted: bool,
    pending_read: Option<Arc<()>>,
    pending_write: Option<Arc<()>>,
    document: Option<DocumentId>,
    error: Option<AppError>,
}

#[derive(Clone)]
enum Operation {
    Read,
    Write(SaveSnapshot),
    Canonical(SaveSnapshot),
}

#[derive(Clone)]
pub struct Request {
    owner: Owner,
    mount: Arc<()>,
    ticket: Arc<()>,
    operation: Operation,
}

impl Request {
    pub fn call(&self) -> Call {
        match &self.operation {
            Operation::Write(snapshot) => Call {
                command: "app_file_write",
                args: json!({"target":self.owner.target,"content":snapshot.rope().to_string()}),
            },
            Operation::Read | Operation::Canonical(_) => Call {
                command: "app_file_read",
                args: json!({"target":self.owner.target}),
            },
        }
    }

    pub fn owner(&self) -> &Owner {
        &self.owner
    }

    pub fn is_write(&self) -> bool {
        !matches!(self.operation, Operation::Read)
    }
}

pub struct Error {
    pub target: AppFileTarget,
    pub is_write: bool,
    pub error: AppError,
}

#[derive(Default)]
pub struct AppFiles {
    entries: HashMap<Owner, Entry>,
    loaded: HashSet<AppFileTarget>,
    queued: Vec<Request>,
    pending: BTreeMap<u32, Request>,
    errors: Vec<Error>,
    settings_generation: u64,
}

impl AppFiles {
    pub fn owners(&self) -> impl Iterator<Item = &Owner> {
        self.entries.keys()
    }

    pub fn refresh(&mut self) {
        self.loaded.clear();
        for entry in self.entries.values_mut() {
            entry.attempted = false;
            entry.pending_read = None;
            entry.error = None;
        }
    }

    pub fn bind(&mut self, owner: Owner, store: &mut EditorStore) -> AppResult<Option<ViewId>> {
        let document = self
            .loaded
            .contains(&owner.target)
            .then(|| store.documents().find(&DocumentKey::AppFile(owner.target)))
            .flatten();
        let entry = self.entries.entry(owner.clone()).or_insert_with(|| Entry {
            mount: Arc::new(()),
            attempted: document.is_some(),
            pending_read: None,
            pending_write: None,
            document,
            error: None,
        });
        if !entry.attempted {
            let ticket = Arc::new(());
            entry.pending_read = Some(Arc::clone(&ticket));
            entry.attempted = true;
            self.queued.push(Request {
                owner: owner.clone(),
                mount: Arc::clone(&entry.mount),
                ticket,
                operation: Operation::Read,
            });
        }
        if entry.error.is_some() {
            return Ok(None);
        }
        entry
            .document
            .map(|document| store.attach_view(owner.key, document).map_err(editor_error))
            .transpose()
    }

    pub fn view(&self, owner: &Owner, store: &EditorStore) -> Option<ViewId> {
        self.entries
            .get(owner)
            .filter(|entry| entry.error.is_none())?;
        store.views().find(&owner.key)
    }

    pub fn error(&self, owner: &Owner) -> Option<&AppError> {
        self.entries.get(owner)?.error.as_ref()
    }

    pub fn next_requests(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.queued)
    }

    pub fn is_active(&self, request: &Request) -> bool {
        self.entries.get(&request.owner).is_some_and(|entry| {
            let ticket = if request.is_write() {
                &entry.pending_write
            } else {
                &entry.pending_read
            };
            Arc::ptr_eq(&entry.mount, &request.mount)
                && ticket
                    .as_ref()
                    .is_some_and(|ticket| Arc::ptr_eq(ticket, &request.ticket))
        })
    }

    pub fn sent(&mut self, request: Request, seq: u32) {
        self.pending.insert(seq, request);
    }

    pub fn failed(&mut self, request: Request, error: InvokeError) {
        self.finish_failure(&request, invocation_error(error));
    }

    pub fn prepare_save(&mut self, owner: &Owner, store: &mut EditorStore) -> AppResult<bool> {
        let Some(entry) = self.entries.get(owner) else {
            return Ok(false);
        };
        let Some(document) = entry.document else {
            return Ok(false);
        };
        if entry.error.is_some()
            || self
                .entries
                .values()
                .any(|entry| entry.document == Some(document) && entry.pending_write.is_some())
        {
            return Ok(false);
        }
        if !store
            .documents()
            .snapshot(document)
            .map_err(editor_error)?
            .dirty
        {
            return Ok(false);
        }
        let snapshot = store.save_snapshot(document).map_err(editor_error)?;
        let ticket = Arc::new(());
        let entry = self.entries.get_mut(owner).expect("live app file owner");
        entry.pending_write = Some(Arc::clone(&ticket));
        self.queued.push(Request {
            owner: owner.clone(),
            mount: Arc::clone(&entry.mount),
            ticket,
            operation: Operation::Write(snapshot),
        });
        Ok(true)
    }

    pub fn response(
        &mut self,
        seq: u32,
        result: &Result<ResponsePayload, Value>,
        store: &mut EditorStore,
    ) -> bool {
        let Some(mut request) = self.pending.remove(&seq) else {
            return false;
        };
        if !self.is_active(&request) {
            return true;
        }
        let result = match request.operation.clone() {
            Operation::Write(snapshot) => json_payload(result).and_then(decode::<()>).map(|()| {
                request.operation = Operation::Canonical(snapshot);
                self.queued.push(request.clone());
            }),
            Operation::Read => {
                json_payload(result)
                    .and_then(decode::<String>)
                    .and_then(|content| {
                        let target = request.owner.target;
                        let document = match store.documents().find(&DocumentKey::AppFile(target)) {
                            Some(document) => {
                                store
                                    .refresh_app_file(document, target, &content)
                                    .map_err(editor_error)?;
                                document
                            }
                            None => store
                                .open_app_file(target, &content)
                                .map_err(editor_error)?,
                        };
                        let entry = self
                            .entries
                            .get_mut(&request.owner)
                            .expect("live app file read owner");
                        entry.document = Some(document);
                        entry.pending_read = None;
                        entry.error = None;
                        self.loaded.insert(target);
                        store
                            .attach_view(request.owner.key.clone(), document)
                            .map_err(editor_error)?;
                        Ok(())
                    })
            }
            Operation::Canonical(snapshot) => json_payload(result)
                .and_then(decode::<String>)
                .and_then(|content| {
                    store
                        .mark_app_file_saved(snapshot, request.owner.target, &content)
                        .map_err(editor_error)?;
                    self.entries
                        .get_mut(&request.owner)
                        .expect("live app file save owner")
                        .pending_write = None;
                    Ok(())
                }),
        };
        if let Err(error) = result {
            self.finish_failure(&request, error);
        }
        true
    }

    fn finish_failure(&mut self, request: &Request, error: AppError) {
        if !self.is_active(request) {
            return;
        }
        let entry = self
            .entries
            .get_mut(&request.owner)
            .expect("live app file failure owner");
        if request.is_write() {
            entry.pending_write = None;
        } else {
            entry.pending_read = None;
            entry.error = Some(error.clone());
        }
        self.errors.push(Error {
            target: request.owner.target,
            is_write: request.is_write(),
            error,
        });
    }

    pub fn has_pending_writes(&self) -> bool {
        self.pending.values().any(Request::is_write)
            || self.queued.iter().any(Request::is_write)
            || self
                .entries
                .values()
                .any(|entry| entry.pending_write.is_some())
    }

    pub fn settings_updated(&mut self, generation: u64) {
        if self.settings_generation == generation {
            return;
        }
        self.settings_generation = generation;
        self.loaded.remove(&AppFileTarget::Settings);
        for (owner, entry) in &mut self.entries {
            if owner.target == AppFileTarget::Settings {
                entry.attempted = false;
                entry.pending_read = None;
                entry.error = None;
            }
        }
    }

    pub fn unbind(&mut self, owner: &Owner, store: &mut EditorStore) -> AppResult<()> {
        self.entries.remove(owner);
        if let Some(view) = store.views().find(&owner.key) {
            store.detach_view(view).map_err(editor_error)?;
        }
        self.queued.retain(|request| &request.owner != owner);
        Ok(())
    }

    pub fn disconnected(&mut self) {
        let requests = std::mem::take(&mut self.queued)
            .into_iter()
            .chain(std::mem::take(&mut self.pending).into_values());
        for request in requests {
            self.failed(request, InvokeError::Closed);
        }
    }

    pub fn take_errors(&mut self) -> Vec<Error> {
        std::mem::take(&mut self.errors)
    }
}

fn editor_error(error: EditorError) -> AppError {
    AppError::Internal(format!("remote app file editor: {error:?}"))
}

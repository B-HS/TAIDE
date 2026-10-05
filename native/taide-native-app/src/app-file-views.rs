use std::collections::{HashMap, HashSet};

use taide_model::{
    app::AppFileTarget,
    error::{AppError, AppResult},
    ids::TabId,
    layout::{PaneNode, Tab},
};
use taide_native_editor::{
    document::{DocumentId, DocumentKey, EditorError},
    store::EditorStore,
};
use taide_runtime::AppState;

use crate::app_file::{Owner, PreparedRead, ReadRequest, Session};

#[derive(Default)]
pub(crate) struct Views {
    entries: HashMap<Owner, Entry>,
    loaded: HashSet<AppFileTarget>,
    settings_revision: u64,
}

struct Entry {
    session: Session,
    attempted: bool,
    pending: Option<ReadRequest>,
    pending_write: Option<crate::app_file_write::WriteRequest>,
    has_draft: bool,
    document: Option<DocumentId>,
    error: Option<AppError>,
}

impl Views {
    pub(crate) fn begin(
        &mut self,
        owner: &Owner,
        store: &EditorStore,
        enabled: bool,
    ) -> Option<ReadRequest> {
        let document = self
            .loaded
            .contains(&owner.target)
            .then(|| store.documents().find(&DocumentKey::AppFile(owner.target)))
            .flatten();
        let entry = self.entries.entry(owner.clone()).or_insert_with(|| Entry {
            session: Session::new(owner.clone()),
            attempted: document.is_some(),
            pending: None,
            pending_write: None,
            has_draft: false,
            document,
            error: None,
        });
        if entry.attempted || !enabled {
            return None;
        }
        let request = entry.session.read_request();
        entry.attempted = true;
        entry.pending = Some(request.clone());
        Some(request)
    }

    pub(crate) fn document(&self, owner: &Owner) -> Option<DocumentId> {
        self.entries
            .get(owner)
            .filter(|entry| entry.error.is_none())
            .and_then(|entry| entry.document)
    }

    pub(crate) fn failed(&self, owner: &Owner) -> bool {
        self.entries
            .get(owner)
            .is_some_and(|entry| entry.error.is_some())
    }

    pub(crate) fn save_owner(&self, tab: &TabId, document: Option<DocumentId>) -> Option<Owner> {
        let document = document?;
        self.entries.iter().find_map(|(owner, entry)| {
            (owner.tab == *tab && entry.document == Some(document)).then(|| owner.clone())
        })
    }

    pub(crate) fn changed(&mut self, document: DocumentId) {
        for entry in self.entries.values_mut() {
            if entry.document == Some(document) {
                entry.has_draft = true;
            }
        }
    }

    pub(crate) fn begin_write(
        &mut self,
        owner: &Owner,
        store: &mut EditorStore,
    ) -> AppResult<Option<crate::app_file_write::WriteRequest>> {
        let Some(document) = self.document(owner) else {
            return Ok(None);
        };
        let dirty = store
            .documents()
            .snapshot(document)
            .map_err(|error| AppError::Internal(format!("native app file draft state: {error:?}")))?
            .dirty;
        if !dirty && !self.entries.get(owner).is_some_and(|entry| entry.has_draft) {
            return Ok(None);
        }
        if self.entries.values().any(|entry| {
            entry
                .pending_write
                .as_ref()
                .is_some_and(|request| request.snapshot().document() == document)
        }) {
            return Ok(None);
        }
        let snapshot = store.save_snapshot(document).map_err(|error| {
            AppError::Internal(format!("native app file save snapshot: {error:?}"))
        })?;
        let entry = self
            .entries
            .get_mut(owner)
            .expect("save document has an owner");
        let request = entry.session.write_request(snapshot)?;
        entry.has_draft = true;
        entry.pending_write = Some(request.clone());
        Ok(Some(request))
    }

    pub(crate) fn cancel_write(&mut self, request: &crate::app_file_write::WriteRequest) -> bool {
        let Some(entry) = self.entries.get_mut(request.owner()) else {
            return false;
        };
        if !entry.session.owns_write(request)
            || !entry
                .pending_write
                .as_ref()
                .is_some_and(|pending| pending.same_request(request))
        {
            return false;
        }
        entry.pending_write = None;
        true
    }

    pub(crate) fn accept_write(
        &mut self,
        request: &crate::app_file_write::WriteRequest,
        result: AppResult<crate::app_file_write::PreparedWrite>,
        store: &mut EditorStore,
    ) -> Option<AppResult<DocumentId>> {
        if !self.cancel_write(request) {
            return None;
        }
        Some(result.and_then(|prepared| {
            if !prepared.request().same_request(request) {
                return Err(AppError::Forbidden(
                    "native app file write reply belongs to another request".into(),
                ));
            }
            prepared.commit(store)
        }))
    }

    pub(crate) fn accept(
        &mut self,
        request: &ReadRequest,
        result: AppResult<PreparedRead>,
        store: &mut EditorStore,
    ) -> bool {
        let Some(entry) = self.entries.get_mut(request.owner()) else {
            return false;
        };
        if !entry.session.owns(request)
            || !entry
                .pending
                .as_ref()
                .is_some_and(|pending| pending.same_request(request))
        {
            return false;
        }
        entry.pending = None;
        let result = result.and_then(|prepared| {
            if !prepared.request().same_request(request) {
                return Err(AppError::Forbidden(
                    "native app file reply belongs to another request".into(),
                ));
            }
            prepared.commit(store)
        });
        match result {
            Ok(document) => {
                entry.document = Some(document);
                entry.error = None;
                self.loaded.insert(request.owner().target);
            }
            Err(error) => entry.error = Some(error),
        }
        true
    }

    pub(crate) fn observe_settings_revision(&mut self, revision: u64) {
        if self.settings_revision == revision {
            return;
        }
        self.settings_revision = revision;
        self.loaded.remove(&AppFileTarget::Settings);
        for (owner, entry) in &mut self.entries {
            if owner.target == AppFileTarget::Settings {
                entry.attempted = false;
                entry.pending = None;
                entry.error = None;
            }
        }
    }

    pub(crate) fn reconcile(
        &mut self,
        state: &AppState,
        store: &mut EditorStore,
    ) -> Result<(), EditorError> {
        let live = owners(state);
        self.entries.retain(|owner, _| live.contains(owner));
        let documents = store
            .documents()
            .snapshots()
            .filter_map(|document| {
                let DocumentKey::AppFile(target) = document.key else {
                    return None;
                };
                Some((document.id, target))
            })
            .collect::<Vec<_>>();
        for (document, target) in documents {
            let dead_views = store
                .views()
                .for_document(document)
                .filter(|view| {
                    !live.iter().any(|owner| {
                        owner.target == target
                            && owner.tab == view.key.tab
                            && owner.pane == view.key.pane
                    })
                })
                .map(|view| view.id)
                .collect::<Vec<_>>();
            for view in dead_views {
                store.detach_view(view)?;
            }
        }
        Ok(())
    }

    pub(crate) fn release_closed(
        &mut self,
        tab: &Tab,
        store: &mut EditorStore,
    ) -> Result<(), EditorError> {
        let taide_model::layout::TabKind::AppFile { target } = tab.kind else {
            return Err(EditorError::InvalidIdentity);
        };
        self.entries.retain(|owner, _| owner.tab != tab.id);
        let Some(document) = store.documents().find(&DocumentKey::AppFile(target)) else {
            return Ok(());
        };
        let views = store
            .views()
            .for_document(document)
            .filter(|view| view.key.tab == tab.id)
            .map(|view| view.id)
            .collect::<Vec<_>>();
        for view in views {
            store.detach_view(view)?;
        }
        Ok(())
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.loaded.clear();
    }
}

pub(crate) fn owners(state: &AppState) -> HashSet<Owner> {
    let layouts = state.layouts.read();
    let mut owners = HashSet::new();
    for (project, layout) in layouts.iter() {
        let mut pending = taide_layout::service::all_roots(layout).collect::<Vec<_>>();
        while let Some(node) = pending.pop() {
            match node {
                PaneNode::Split { children, .. } => pending.extend(children),
                PaneNode::Leaf { id, tabs, .. } => {
                    for tab in tabs {
                        if let taide_model::layout::TabKind::AppFile { target } = tab.kind {
                            owners.insert(Owner {
                                project: project.clone(),
                                pane: id.clone(),
                                tab: tab.id.clone(),
                                target,
                            });
                        }
                    }
                }
            }
        }
    }
    owners
}

pub(crate) fn target_tabs(state: &AppState, target: AppFileTarget) -> Vec<TabId> {
    owners(state)
        .into_iter()
        .filter(|owner| owner.target == target)
        .map(|owner| owner.tab)
        .collect()
}

pub(crate) fn settings_command(
    project: &taide_model::ids::ProjectId,
    layout: &taide_model::layout::ProjectLayout,
    scope: &taide_native_ui::shell::WindowScope,
) -> Option<crate::host::HostCommand> {
    let pane = match scope {
        taide_native_ui::shell::WindowScope::Main => layout.focused_pane.clone(),
        taide_native_ui::shell::WindowScope::Auxiliary {
            project: owner,
            slot,
        } => {
            if owner != project {
                return None;
            }
            layout
                .auxiliary_windows
                .iter()
                .find(|window| window.slot == *slot)?
                .focused_pane
                .clone()
        }
    };
    Some(crate::host::HostCommand::OpenAppFile {
        project: project.clone(),
        pane,
        target: AppFileTarget::Settings,
        title: "settings.json".into(),
    })
}

#[cfg(test)]
#[path = "app-file-views-tests.rs"]
mod tests;

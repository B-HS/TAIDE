use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use taide_model::{
    app::AppFileTarget,
    error::{AppError, AppResult},
    ids::{PaneId, ProjectId, TabId},
    layout::{PaneNode, TabKind},
};
use taide_native_editor::{
    document::{DocumentId, DocumentKey},
    store::EditorStore,
};
use taide_runtime::{AppServices, AppState, TaskOperationLease, app_actions};
use tokio::sync::OwnedMutexGuard;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Owner {
    pub project: ProjectId,
    pub pane: PaneId,
    pub tab: TabId,
    pub target: AppFileTarget,
}

impl Owner {
    pub fn exists(&self, state: &AppState) -> bool {
        state.layouts.read().get(&self.project).is_some_and(|layout| {
            taide_layout::service::all_roots(layout).any(|root| {
                matches!(taide_layout::service::find_leaf(root, &self.pane),
                    Some(PaneNode::Leaf { tabs, .. })
                    if tabs.iter().any(|tab| tab.id == self.tab
                        && matches!(tab.kind, TabKind::AppFile { target } if target == self.target)))
            })
        })
    }
}

#[derive(Clone)]
pub(crate) struct Authority {
    owner: Owner,
    active: Arc<AtomicBool>,
}

impl Authority {
    pub(crate) fn check(&self, state: &AppState) -> AppResult<()> {
        if state.is_shutting_down()
            || !self.active.load(Ordering::Acquire)
            || !self.owner.exists(state)
        {
            return Err(AppError::Forbidden(
                "native app file owner is no longer available".into(),
            ));
        }
        Ok(())
    }
}

pub struct Session {
    authority: Authority,
}

impl Session {
    pub fn new(owner: Owner) -> Self {
        Self {
            authority: Authority {
                owner,
                active: Arc::new(AtomicBool::new(true)),
            },
        }
    }

    pub fn read_request(&self) -> ReadRequest {
        ReadRequest {
            authority: self.authority.clone(),
            operation: Arc::new(()),
        }
    }

    pub fn owns(&self, request: &ReadRequest) -> bool {
        Arc::ptr_eq(&self.authority.active, &request.authority.active)
    }

    pub fn write_request(
        &self,
        snapshot: taide_native_editor::store::SaveSnapshot,
    ) -> AppResult<crate::app_file_write::WriteRequest> {
        crate::app_file_write::WriteRequest::new(self.authority.clone(), snapshot)
    }

    pub fn owns_write(&self, request: &crate::app_file_write::WriteRequest) -> bool {
        Arc::ptr_eq(&self.authority.active, request.ticket())
    }
}

impl Authority {
    pub(crate) fn owner(&self) -> &Owner {
        &self.owner
    }

    pub(crate) fn ticket(&self) -> &Arc<AtomicBool> {
        &self.active
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.authority.active.store(false, Ordering::Release);
    }
}

#[derive(Clone)]
pub struct ReadRequest {
    authority: Authority,
    operation: Arc<()>,
}

impl ReadRequest {
    pub fn owner(&self) -> &Owner {
        &self.authority.owner
    }

    pub fn same_request(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.operation, &other.operation)
    }

    pub async fn execute(&self, services: &AppServices) -> AppResult<PreparedRead> {
        self.authority.check(&services.state)?;
        let operation = services
            .tasks
            .begin_operation("native-app-file-admission")
            .ok_or_else(|| AppError::Forbidden("native app file admission is stopping".into()))?;
        let guard = services.state.begin_owned_mutation().await;
        let request = self.clone();
        let state = services.state.clone();
        let runtime = tokio::runtime::Handle::current();
        services
            .tasks
            .run_blocking_result("native-app-file-read", move || {
                request.authority.check(&state)?;
                let content = runtime.block_on(app_actions::app_file_read(
                    &state,
                    request.authority.owner.target,
                ))?;
                Ok(PreparedRead {
                    request,
                    state,
                    operation,
                    guard,
                    content,
                })
            })
            .await
    }
}

pub struct PreparedRead {
    request: ReadRequest,
    state: AppState,
    operation: TaskOperationLease,
    guard: OwnedMutexGuard<()>,
    content: String,
}

impl PreparedRead {
    pub fn request(&self) -> &ReadRequest {
        &self.request
    }

    pub fn commit(self, store: &mut EditorStore) -> AppResult<DocumentId> {
        let _operation = self.operation;
        let _guard = self.guard;
        self.request.authority.check(&self.state)?;
        let target = self.request.authority.owner.target;
        if let Some(document) = store.documents().find(&DocumentKey::AppFile(target)) {
            store
                .refresh_app_file(document, target, &self.content)
                .map_err(|error| {
                    AppError::Internal(format!("native app file refresh: {error:?}"))
                })?;
            return Ok(document);
        }
        store
            .open_app_file(target, &self.content)
            .map_err(|error| AppError::Internal(format!("native app file admission: {error:?}")))
    }
}

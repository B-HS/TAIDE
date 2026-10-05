use std::{
    future::Future,
    sync::{Arc, atomic::AtomicBool},
};

use taide_model::{
    error::{AppError, AppResult},
    settings::Settings,
};
use taide_native_editor::{
    document::{DocumentId, DocumentKey},
    store::{EditorStore, SaveSnapshot},
};
use taide_runtime::{AppServices, AppState, TaskOperationLease, app_actions};
use tokio::sync::OwnedMutexGuard;

use crate::app_file::{Authority, Owner};

#[derive(Clone)]
pub struct WriteRequest {
    authority: Authority,
    operation: Arc<()>,
    snapshot: SaveSnapshot,
}

impl WriteRequest {
    pub(crate) fn new(authority: Authority, snapshot: SaveSnapshot) -> AppResult<Self> {
        if snapshot.key() != &DocumentKey::AppFile(authority.owner().target) {
            return Err(AppError::Forbidden(
                "native app file save target does not match its document".into(),
            ));
        }
        Ok(Self {
            authority,
            operation: Arc::new(()),
            snapshot,
        })
    }

    pub fn owner(&self) -> &Owner {
        self.authority.owner()
    }

    pub fn snapshot(&self) -> &SaveSnapshot {
        &self.snapshot
    }

    pub fn same_request(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.operation, &other.operation)
    }

    pub(crate) fn ticket(&self) -> &Arc<AtomicBool> {
        self.authority.ticket()
    }

    pub async fn execute<F, Fut>(
        &self,
        services: &AppServices,
        apply_settings: F,
    ) -> AppResult<PreparedWrite>
    where
        F: FnOnce(Settings) -> Fut + Send + 'static,
        Fut: Future<Output = AppResult<Settings>> + Send + 'static,
    {
        self.authority.check(&services.state)?;
        let operation = services
            .tasks
            .begin_operation("native-app-file-save-admission")
            .ok_or_else(|| {
                AppError::Forbidden("native app file save admission is stopping".into())
            })?;
        let guard = services.state.begin_owned_mutation().await;
        let request = self.clone();
        let state = services.state.clone();
        let runtime = tokio::runtime::Handle::current();
        services
            .tasks
            .run_blocking_result("native-app-file-write", move || {
                request.authority.check(&state)?;
                let target = request.owner().target;
                runtime.block_on(app_actions::app_file_write_admitted(
                    &state,
                    target,
                    request.snapshot.rope().to_string(),
                    apply_settings,
                ))?;
                let canonical = runtime.block_on(app_actions::app_file_read(&state, target))?;
                Ok(PreparedWrite {
                    request,
                    state,
                    operation,
                    guard,
                    canonical,
                })
            })
            .await
    }
}

pub struct PreparedWrite {
    request: WriteRequest,
    state: AppState,
    operation: TaskOperationLease,
    guard: OwnedMutexGuard<()>,
    canonical: String,
}

impl PreparedWrite {
    pub fn request(&self) -> &WriteRequest {
        &self.request
    }

    pub fn commit(self, store: &mut EditorStore) -> AppResult<DocumentId> {
        let _operation = self.operation;
        let _guard = self.guard;
        self.request.authority.check(&self.state)?;
        let target = self.request.owner().target;
        let document = self.request.snapshot.document();
        store
            .mark_app_file_saved(self.request.snapshot, target, &self.canonical)
            .map_err(|error| {
                AppError::Internal(format!("native app file save completion: {error:?}"))
            })?;
        Ok(document)
    }
}

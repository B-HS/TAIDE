use taide_model::error::{AppError, AppResult};
use tokio::sync::oneshot;

use crate::{TaskOperationLease, TaskSupervisor};

/// Keeps a built resource owned through registration, publication and resource cleanup.
pub struct ProjectBuild<T> {
    pub value: T,
    _operation: TaskOperationLease,
}

fn project_shutdown_error() -> AppError {
    AppError::Forbidden("project runtime is shutting down".to_string())
}

/// Tracks the blocking builder and its resulting resource with the host's registered supervisor.
pub async fn run_project_build<T: Send + 'static>(
    tasks: &TaskSupervisor,
    work: impl FnOnce() -> T + Send + 'static,
) -> AppResult<ProjectBuild<T>> {
    let operation = tasks.begin_operation("project-build").ok_or_else(project_shutdown_error)?;
    let (sender, receiver) = oneshot::channel();
    let owner = operation.clone();
    let worker = tasks
        .spawn_blocking_transient_handle("project-build-worker", move || {
            let _owner = owner;
            drop(sender.send(work()));
        })
        .ok_or_else(project_shutdown_error)?;
    worker.await.map_err(|error| {
        if error.is_cancelled() {
            return project_shutdown_error();
        }
        AppError::Internal(error.to_string())
    })?;
    let value = receiver.await.map_err(|_| project_shutdown_error())?;
    Ok(ProjectBuild {
        value,
        _operation: operation,
    })
}

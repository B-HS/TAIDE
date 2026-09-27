use std::path::Path;
use std::sync::Arc;

use tokio::sync::OwnedMutexGuard;

use taide_infra::lsp_proc::LspProcHandle;
use taide_lsp::install::LspInstallStore;
use taide_lsp::process::shutdown_process;
use taide_lsp::protocol::workspace_folders_notification;
use taide_lsp::session::{LspLifecycleSnapshot, LspMessageSubscribers};
use taide_lsp::store::{LspSessionEntry, LspStore};
use taide_lsp::{manifest, service};
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::ids::ProjectId;
use taide_model::lsp::{LanguageServerSpec, LspInstallStrategy, LspServerId, LspSessionInfo, LspSessionStatus, LspSpawnRequest};

use crate::{AppState, EventSink, TaskOperationLease, TaskSupervisor};

const REINITIALIZE_FAILURE_MESSAGE: &str =
    "초기화 핸드셰이크 재시도를 모두 소진해 서버를 재연결하지 못했습니다. 수동으로 다시 시작해주세요.";

fn find_entry(store: &LspStore, session_id: &str) -> AppResult<Arc<LspSessionEntry>> {
    store
        .get(session_id)
        .ok_or_else(|| AppError::NotFound(format!("lsp session not found: {session_id}")))
}

fn emit_status(events: &dyn EventSink, session_id: &str, snapshot: LspLifecycleSnapshot) {
    events.publish(AppEvent::LspSessionStatusChanged {
        session_id: session_id.to_string(),
        status: snapshot.status,
        last_error: snapshot.last_error,
        generation: snapshot.generation,
    });
}

fn set_status(events: &dyn EventSink, session_id: &str, entry: &LspSessionEntry, status: LspSessionStatus) {
    emit_status(events, session_id, entry.lifecycle.set_status(status, None));
}

/// Supplies the host's shared state and supervisor for session lifecycle actions.
#[derive(Clone, Copy)]
pub struct LspActionContext<'a> {
    state: &'a AppState,
    store: &'a LspStore,
    tasks: &'a TaskSupervisor,
}

impl<'a> LspActionContext<'a> {
    /// Uses the existing application supervisor without creating another owner.
    pub fn new(state: &'a AppState, store: &'a LspStore, tasks: &'a TaskSupervisor) -> Self {
        Self { state, store, tasks }
    }
}

/// Supplies lazy channel, identifier and bare native process factories.
pub struct LspSpawnPorts<D, I, F> {
    create_message_sink: D,
    create_session_id: I,
    create_process: F,
}

impl<D, I, F> LspSpawnPorts<D, I, F> {
    /// Leaves process registration and application policy to the runtime action.
    pub fn new(create_message_sink: D, create_session_id: I, create_process: F) -> Self {
        Self {
            create_message_sink,
            create_session_id,
            create_process,
        }
    }
}

struct LspOperation {
    guard: Option<OwnedMutexGuard<()>>,
    _lease: TaskOperationLease,
}

impl LspOperation {
    fn begin(tasks: &TaskSupervisor, name: &'static str, guard: OwnedMutexGuard<()>) -> AppResult<Self> {
        let lease = tasks
            .begin_operation(name)
            .ok_or_else(|| AppError::Forbidden("language server runtime is shutting down".to_string()))?;
        Ok(Self {
            guard: Some(guard),
            _lease: lease,
        })
    }

    fn release_guard(&mut self) {
        self.guard.take();
    }
}

/// Selects a shared session only for the same project, server and subscribed owner, excluding teardown.
pub fn find_reusable_entry(
    store: &LspStore,
    project_id: &ProjectId,
    server_id: &LspServerId,
    owner: &str,
) -> Option<(String, Arc<LspSessionEntry>)> {
    store.find_reusable(project_id, server_id, owner)
}

/// Releases one root reference and removes the owner's sink only for full teardown.
pub fn release_owner_root(entry: &LspSessionEntry, owner: &str, root: Option<&str>) -> (Option<String>, bool) {
    let Some(root) = root else {
        entry.subscribers.remove(owner);
        return (None, false);
    };
    let release = entry.roots.release(root);
    let has_remaining_roots = release.has_remaining_roots;
    if !has_remaining_roots {
        entry.subscribers.remove(owner);
    }
    (release.removed_root, has_remaining_roots)
}

/// Marks teardown before the existing protocol shutdown and publishes the stopped snapshot afterward.
pub async fn shutdown_entry(events: &dyn EventSink, entry: &LspSessionEntry, session_id: &str) {
    entry.lifecycle.mark_stopping();
    let proc = entry.proc.lock().clone();
    if let Some(proc) = proc {
        shutdown_process(&proc).await;
    }
    set_status(events, session_id, entry, LspSessionStatus::Stopped);
}

/// Applies session reuse and spawn policy while owning admission, bookkeeping and publication.
pub async fn lsp_spawn<D, S, I, F>(
    events: &dyn EventSink,
    context: LspActionContext<'_>,
    request: LspSpawnRequest,
    ports: LspSpawnPorts<D, I, F>,
) -> AppResult<String>
where
    D: FnOnce() -> S,
    S: Fn(&str) -> bool + Send + Sync + 'static,
    I: FnOnce() -> String,
    F: FnOnce(String, u64, LanguageServerSpec, String) -> AppResult<Arc<LspProcHandle>>,
{
    let LspActionContext { state, store, tasks } = context;
    let LspSpawnRequest {
        project_id,
        server_id,
        root,
        owner,
    } = request;
    let guard = state.begin_owned_mutation().await;
    if !state.projects.read().contains_key(&project_id) {
        return Err(AppError::NotFound(format!("project not open: {project_id}")));
    }
    let spec = manifest::find_spec(server_id.as_str())
        .ok_or_else(|| AppError::InvalidArgument(format!("unknown language server: {server_id}")))?;
    let _operation = LspOperation::begin(tasks, "lsp-spawn-action", guard)?;

    if let Some((existing_id, existing_entry)) = find_reusable_entry(store, &project_id, &server_id, &owner) {
        let existing_roots = existing_entry.roots.paths();
        if service::should_reuse_session(&spec, &existing_roots, &root) {
            let is_new_root = existing_entry.roots.acquire(root.clone());
            existing_entry.subscribers.insert(owner, (ports.create_message_sink)());
            if is_new_root {
                let proc = existing_entry.proc.lock().clone();
                if let Some(proc) = proc {
                    let notification = workspace_folders_notification(std::slice::from_ref(&root), &[]);
                    let _ = proc.write_message(&notification).await;
                }
            }
            return Ok(existing_id);
        }
    }

    let session_id = (ports.create_session_id)();
    let subscribers = LspMessageSubscribers::new();
    subscribers.insert(owner, (ports.create_message_sink)());
    let entry = Arc::new(LspSessionEntry::new(project_id, spec.clone(), root.clone(), subscribers));
    store.insert(session_id.clone(), entry.clone());
    let process_epoch = entry.lifecycle.advance_process_epoch();
    let proc = match store.spawn_process(|| (ports.create_process)(session_id.clone(), process_epoch, spec, root)) {
        Ok(proc) => proc,
        Err(error) => {
            store.remove(&session_id);
            return Err(error);
        }
    };
    *entry.proc.lock() = Some(proc);
    set_status(events, &session_id, &entry, LspSessionStatus::Running);
    Ok(session_id)
}

/// Unlinks full teardown under the mutation guard and shuts down outside that guard.
pub async fn lsp_stop(
    events: &dyn EventSink,
    context: LspActionContext<'_>,
    session_id: String,
    root: Option<String>,
    owner: String,
) -> AppResult<()> {
    let LspActionContext { state, store, tasks } = context;
    let guard = state.begin_owned_mutation().await;
    let entry = find_entry(store, &session_id)?;
    let mut operation = LspOperation::begin(tasks, "lsp-stop-action", guard)?;
    let (removed_root, has_remaining_roots) = release_owner_root(&entry, &owner, root.as_deref());
    if has_remaining_roots {
        if let Some(removed_root) = removed_root {
            let proc = entry.proc.lock().clone();
            if let Some(proc) = proc {
                let notification = workspace_folders_notification(&[], std::slice::from_ref(&removed_root));
                let _ = proc.write_message(&notification).await;
            }
        }
        return Ok(());
    }
    store.remove(&session_id);
    operation.release_guard();
    shutdown_entry(events, &entry, &session_id).await;
    Ok(())
}

/// Rechecks session membership after unguarded teardown before registering the replacement process.
pub async fn lsp_restart(
    events: &dyn EventSink,
    context: LspActionContext<'_>,
    session_id: String,
    create_process: impl FnOnce(String, u64, LanguageServerSpec, String) -> AppResult<Arc<LspProcHandle>>,
) -> AppResult<()> {
    let LspActionContext { state, store, tasks } = context;
    let guard = state.begin_owned_mutation().await;
    let entry = find_entry(store, &session_id)?;
    let mut operation = LspOperation::begin(tasks, "lsp-restart-action", guard)?;
    operation.release_guard();
    shutdown_entry(events, &entry, &session_id).await;
    operation.guard = Some(state.begin_owned_mutation().await);
    if !store.contains(&session_id) {
        return Err(AppError::NotFound(format!("lsp session not found: {session_id}")));
    }
    let process_epoch = entry.lifecycle.advance_process_epoch();
    emit_status(events, &session_id, entry.lifecycle.begin_manual_restart());
    let proc = store.spawn_process(|| create_process(session_id.clone(), process_epoch, entry.spec.clone(), entry.root.clone()))?;
    *entry.proc.lock() = Some(proc);
    set_status(events, &session_id, &entry, LspSessionStatus::Running);
    Ok(())
}

/// Writes to the current process using its existing serialized protocol writer.
pub async fn lsp_send(store: &LspStore, session_id: String, message: String) -> AppResult<()> {
    let entry = find_entry(store, &session_id)?;
    let proc = entry
        .proc
        .lock()
        .clone()
        .ok_or_else(|| AppError::Internal("language server not ready".to_string()))?;
    proc.write_message(&message).await
}

/// Confirms reinitialization only for the current generation of a crashed session.
pub fn lsp_confirm_reinitialize(events: &dyn EventSink, store: &LspStore, session_id: String, generation: u32) -> AppResult<()> {
    let entry = find_entry(store, &session_id)?;
    if let Some(snapshot) = entry.lifecycle.confirm_reinitialized(generation) {
        emit_status(events, &session_id, snapshot);
    }
    Ok(())
}

/// Records reinitialization failure only for the current generation of a crashed session.
pub fn lsp_report_reinitialize_failure(events: &dyn EventSink, store: &LspStore, session_id: String, generation: u32) -> AppResult<()> {
    let entry = find_entry(store, &session_id)?;
    if let Some(snapshot) = entry
        .lifecycle
        .report_reinitialize_failure(generation, REINITIALIZE_FAILURE_MESSAGE.to_string())
    {
        emit_status(events, &session_id, snapshot);
    }
    Ok(())
}

/// Returns the existing session snapshots for one project without starting a server.
pub fn lsp_sessions(store: &LspStore, project_id: ProjectId) -> AppResult<Vec<LspSessionInfo>> {
    Ok(store.sessions_for_project(&project_id))
}

/// Resolves the bundled server before applying its existing root-discovery strategy.
pub fn lsp_resolve_root(server_id: LspServerId, file_path: String) -> AppResult<Option<String>> {
    let spec = manifest::find_spec(server_id.as_str())
        .ok_or_else(|| AppError::InvalidArgument(format!("unknown language server: {server_id}")))?;
    Ok(service::find_root(&spec, Path::new(&file_path)).map(|root| root.to_string_lossy().to_string()))
}

/// Requests installation cancellation without releasing slots held by running worker leases.
pub fn lsp_install_cancel(install_store: &LspInstallStore, server_id: LspServerId) -> AppResult<()> {
    install_store.cancel(&server_id);
    Ok(())
}

/// Applies the existing installation admission and strategy policy using the shared worker owners.
pub async fn lsp_install(
    events: &dyn EventSink,
    state: &AppState,
    install_store: &LspInstallStore,
    tasks: &TaskSupervisor,
    server_id: LspServerId,
) -> AppResult<()> {
    if state.is_shutting_down() {
        install_store.shutdown();
        return Err(taide_lsp::install::install_cancelled_error());
    }
    let spec = manifest::find_spec(server_id.as_str())
        .ok_or_else(|| AppError::InvalidArgument(format!("unknown language server: {server_id}")))?;
    let Some(install_guard) = install_store.begin(&server_id) else {
        if install_store.is_stopped() {
            return Err(taide_lsp::install::install_cancelled_error());
        }
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.lsp.installAlreadyRunning",
            format!("{server_id}: an install is already in progress"),
        )
        .with_arg("serverId", &server_id));
    };
    match spec.install.strategy {
        LspInstallStrategy::Download => {
            crate::lsp_install_actions::run_download_install(events, &state.paths, &spec, &install_guard.lease(), tasks).await
        }
        LspInstallStrategy::Toolchain => {
            crate::lsp_install_toolchain::run_toolchain_install(events, &spec, &install_guard.lease(), tasks).await
        }
        LspInstallStrategy::SdkDetect => Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.lsp.sdkDetectOnly",
            format!("{}: SDK-detect-only servers cannot be installed automatically", spec.id),
        )
        .with_arg("serverId", &spec.id)),
    }
}

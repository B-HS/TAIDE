use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use super::service;
use super::types::{LanguageServerSpec, LspServerDetection, LspServerId, LspSessionInfo, LspSessionStatus, RESTART_BACKOFF_LIMIT};
use crate::error::{AppError, AppResult};
use crate::events::LspSessionStatusChanged;
use crate::ids::ProjectId;
use crate::infra::lsp_proc;
use crate::state::AppState;

const LSP_SHUTDOWN_TIMEOUT_MS: u64 = 2_000;
const LSP_RESTART_BACKOFF_BASE_MS: u64 = 500;

struct SessionEntry {
    project_id: ProjectId,
    server_id: LspServerId,
    root: String,
    spec: LanguageServerSpec,
    proc: Mutex<Option<Arc<lsp_proc::LspProcHandle>>>,
    channel: Mutex<Option<Channel<String>>>,
    status: Mutex<LspSessionStatus>,
    last_error: Mutex<Option<String>>,
    restart_count: AtomicU32,
    stopping: Arc<AtomicBool>,
    roots: Mutex<Vec<(String, u32)>>,
}

fn workspace_folder_json(root: &str) -> serde_json::Value {
    serde_json::json!({
        "uri": format!("file://{root}"),
        "name": std::path::Path::new(root).file_name().and_then(|name| name.to_str()).unwrap_or("workspace"),
    })
}

fn workspace_folders_notification(added: &[String], removed: &[String]) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "workspace/didChangeWorkspaceFolders",
        "params": {
            "event": {
                "added": added.iter().map(|root| workspace_folder_json(root)).collect::<Vec<_>>(),
                "removed": removed.iter().map(|root| workspace_folder_json(root)).collect::<Vec<_>>(),
            }
        }
    })
    .to_string()
}

#[derive(Default)]
pub struct LspStore(Mutex<HashMap<String, Arc<SessionEntry>>>);

impl LspStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn kill_all(&self) {
        for entry in self.0.lock().values() {
            entry.stopping.store(true, Ordering::SeqCst);
            if let Some(proc) = entry.proc.lock().as_ref() {
                proc.kill();
            }
        }
    }
}

fn new_session_id() -> String {
    format!("lsp-{}", uuid::Uuid::new_v4())
}

fn find_entry(store: &LspStore, session_id: &str) -> AppResult<Arc<SessionEntry>> {
    store
        .0
        .lock()
        .get(session_id)
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("lsp session not found: {session_id}")))
}

fn find_reusable_entry(store: &LspStore, project_id: &ProjectId, server_id: LspServerId) -> Option<(String, Arc<SessionEntry>)> {
    store
        .0
        .lock()
        .iter()
        .find(|(_, entry)| &entry.project_id == project_id && entry.server_id == server_id && entry.spec.shares_sessions)
        .map(|(id, entry)| (id.clone(), entry.clone()))
}

fn ensure_project_open(state: &AppState, project_id: &ProjectId) -> AppResult<()> {
    if state.projects.read().contains_key(project_id) {
        return Ok(());
    }
    Err(AppError::NotFound(format!("project not open: {project_id}")))
}

fn set_status(app: &AppHandle, session_id: &str, entry: &SessionEntry, status: LspSessionStatus, last_error: Option<String>) {
    *entry.status.lock() = status;
    *entry.last_error.lock() = last_error.clone();
    let _ = LspSessionStatusChanged {
        session_id: session_id.to_string(),
        status,
        last_error,
    }
    .emit(app);
}

fn spawn_process(app: &AppHandle, session_id: String, spec: LanguageServerSpec, root: String) -> AppResult<Arc<lsp_proc::LspProcHandle>> {
    let resolved = service::resolve_command(&spec.command, Some(std::path::Path::new(&root)))
        .ok_or_else(|| AppError::NotFound(format!("language server executable not found: {}", spec.command)))?;

    let config = lsp_proc::LspProcConfig {
        command: resolved.to_string_lossy().to_string(),
        args: spec.args.clone(),
        cwd: PathBuf::from(&root),
    };

    let message_app = app.clone();
    let message_session_id = session_id.clone();

    let exit_app = app.clone();
    let exit_session_id = session_id.clone();

    let handle = lsp_proc::spawn(
        config,
        move |message| {
            let Some(store) = message_app.try_state::<LspStore>() else {
                return;
            };
            let Ok(entry) = find_entry(&store, &message_session_id) else {
                return;
            };
            if let Some(channel) = entry.channel.lock().as_ref() {
                let _ = channel.send(message);
            };
        },
        move |code| handle_process_exit(&exit_app, exit_session_id, code),
    )?;

    Ok(Arc::new(handle))
}

fn handle_process_exit(app: &AppHandle, session_id: String, code: Option<i32>) {
    let Some(store) = app.try_state::<LspStore>() else {
        return;
    };
    let Ok(entry) = find_entry(&store, &session_id) else {
        return;
    };

    if entry.stopping.load(Ordering::SeqCst) {
        set_status(app, &session_id, &entry, LspSessionStatus::Stopped, None);
        return;
    }

    let restarts = entry.restart_count.fetch_add(1, Ordering::SeqCst) + 1;
    if restarts > RESTART_BACKOFF_LIMIT {
        set_status(
            app,
            &session_id,
            &entry,
            LspSessionStatus::Crashed,
            Some(format!(
                "서버가 반복적으로 종료되어 재시작을 중지했습니다 (마지막 종료 코드: {code:?})"
            )),
        );
        return;
    }

    set_status(
        app,
        &session_id,
        &entry,
        LspSessionStatus::Starting,
        Some(format!("서버가 종료되어 재시작합니다 (마지막 종료 코드: {code:?})")),
    );

    let backoff = tokio::time::Duration::from_millis(LSP_RESTART_BACKOFF_BASE_MS * restarts as u64);
    let restart_app = app.clone();
    let restart_session_id = session_id.clone();
    let spec = entry.spec.clone();
    let root = entry.root.clone();

    tokio::spawn(async move {
        tokio::time::sleep(backoff).await;

        let Some(store) = restart_app.try_state::<LspStore>() else {
            return;
        };
        let Ok(entry) = find_entry(&store, &restart_session_id) else {
            return;
        };
        if entry.stopping.load(Ordering::SeqCst) {
            return;
        }

        match spawn_process(&restart_app, restart_session_id.clone(), spec, root) {
            Ok(proc) => {
                *entry.proc.lock() = Some(proc);
                set_status(&restart_app, &restart_session_id, &entry, LspSessionStatus::Running, None);
            }
            Err(error) => {
                set_status(
                    &restart_app,
                    &restart_session_id,
                    &entry,
                    LspSessionStatus::Crashed,
                    Some(error.to_string()),
                );
            }
        }
    });
}

async fn shutdown_entry(app: &AppHandle, entry: &SessionEntry, session_id: &str) {
    entry.stopping.store(true, Ordering::SeqCst);

    let proc = entry.proc.lock().clone();
    if let Some(proc) = proc {
        let shutdown_request = serde_json::json!({ "jsonrpc": "2.0", "id": "taide-shutdown", "method": "shutdown" }).to_string();
        let _ = proc.write_message(&shutdown_request).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(LSP_SHUTDOWN_TIMEOUT_MS)).await;

        let exit_notification = serde_json::json!({ "jsonrpc": "2.0", "method": "exit" }).to_string();
        let _ = proc.write_message(&exit_notification).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(LSP_SHUTDOWN_TIMEOUT_MS)).await;

        proc.kill();
    }

    set_status(app, session_id, entry, LspSessionStatus::Stopped, None);
}

#[tauri::command]
#[specta::specta]
pub async fn lsp_spawn(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, LspStore>,
    project_id: ProjectId,
    server_id: LspServerId,
    root: String,
    on_message: Channel<String>,
) -> AppResult<String> {
    let _guard = state.begin_mutation().await;
    ensure_project_open(&state, &project_id)?;

    let spec = service::builtin_specs()
        .into_iter()
        .find(|spec| spec.id == server_id)
        .ok_or_else(|| AppError::InvalidArgument(format!("unknown language server: {server_id:?}")))?;

    if let Some((existing_id, existing_entry)) = find_reusable_entry(&store, &project_id, server_id) {
        let existing_roots: Vec<String> = existing_entry.roots.lock().iter().map(|(root, _)| root.clone()).collect();

        if service::should_reuse_session(&spec, &existing_roots, &root) {
            let is_new_root = {
                let mut roots = existing_entry.roots.lock();
                match roots.iter_mut().find(|(existing_root, _)| existing_root == &root) {
                    Some((_, count)) => {
                        *count += 1;
                        false
                    }
                    None => {
                        roots.push((root.clone(), 1));
                        true
                    }
                }
            };

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

    let session_id = new_session_id();

    let entry = Arc::new(SessionEntry {
        project_id,
        server_id,
        root: root.clone(),
        spec: spec.clone(),
        proc: Mutex::new(None),
        channel: Mutex::new(Some(on_message)),
        status: Mutex::new(LspSessionStatus::Starting),
        last_error: Mutex::new(None),
        restart_count: AtomicU32::new(0),
        stopping: Arc::new(AtomicBool::new(false)),
        roots: Mutex::new(vec![(root.clone(), 1)]),
    });

    store.0.lock().insert(session_id.clone(), entry.clone());

    let proc = match spawn_process(&app, session_id.clone(), spec, root) {
        Ok(proc) => proc,
        Err(error) => {
            store.0.lock().remove(&session_id);
            return Err(error);
        }
    };

    *entry.proc.lock() = Some(proc);
    set_status(&app, &session_id, &entry, LspSessionStatus::Running, None);

    Ok(session_id)
}

#[tauri::command]
#[specta::specta]
pub async fn lsp_send(state: State<'_, AppState>, store: State<'_, LspStore>, session_id: String, message: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let entry = find_entry(&store, &session_id)?;
    let proc = entry
        .proc
        .lock()
        .clone()
        .ok_or_else(|| AppError::Internal("language server not ready".to_string()))?;
    proc.write_message(&message).await
}

#[tauri::command]
#[specta::specta]
pub async fn lsp_stop(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, LspStore>,
    session_id: String,
    root: Option<String>,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let entry = find_entry(&store, &session_id)?;

    if let Some(root) = root {
        let (removed_root, remaining_roots) = {
            let mut roots = entry.roots.lock();
            let mut removed_root: Option<String> = None;
            if let Some(position) = roots.iter().position(|(existing_root, _)| existing_root == &root) {
                roots[position].1 = roots[position].1.saturating_sub(1);
                if roots[position].1 == 0 {
                    removed_root = Some(roots.remove(position).0);
                }
            }
            (removed_root, !roots.is_empty())
        };

        if remaining_roots {
            if let Some(removed_root) = removed_root {
                let proc = entry.proc.lock().clone();
                if let Some(proc) = proc {
                    let notification = workspace_folders_notification(&[], std::slice::from_ref(&removed_root));
                    let _ = proc.write_message(&notification).await;
                }
            }
            return Ok(());
        }
    }

    shutdown_entry(&app, &entry, &session_id).await;
    store.0.lock().remove(&session_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn lsp_restart(app: AppHandle, state: State<'_, AppState>, store: State<'_, LspStore>, session_id: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let entry = find_entry(&store, &session_id)?;

    shutdown_entry(&app, &entry, &session_id).await;
    entry.stopping.store(false, Ordering::SeqCst);
    entry.restart_count.store(0, Ordering::SeqCst);
    set_status(&app, &session_id, &entry, LspSessionStatus::Starting, None);

    let proc = spawn_process(&app, session_id.clone(), entry.spec.clone(), entry.root.clone())?;
    *entry.proc.lock() = Some(proc);
    set_status(&app, &session_id, &entry, LspSessionStatus::Running, None);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn lsp_sessions(store: State<'_, LspStore>, project_id: ProjectId) -> AppResult<Vec<LspSessionInfo>> {
    let sessions = store.0.lock();
    Ok(sessions
        .iter()
        .filter(|(_, entry)| entry.project_id == project_id)
        .map(|(id, entry)| LspSessionInfo {
            session_id: id.clone(),
            project_id: entry.project_id.clone(),
            server_id: entry.server_id,
            root: entry.root.clone(),
            status: *entry.status.lock(),
            last_error: entry.last_error.lock().clone(),
        })
        .collect())
}

#[tauri::command]
#[specta::specta]
pub async fn lsp_detect_servers() -> AppResult<Vec<LspServerDetection>> {
    Ok(service::detect_servers())
}

#[tauri::command]
#[specta::specta]
pub async fn lsp_resolve_root(server_id: LspServerId, file_path: String) -> AppResult<Option<String>> {
    Ok(service::find_root(server_id, std::path::Path::new(&file_path)).map(|root| root.to_string_lossy().to_string()))
}

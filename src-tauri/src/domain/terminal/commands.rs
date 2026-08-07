use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, State};
use tauri_specta::Event;

use super::service;
use super::types::{PtySpawnOptions, ShellProfile, TerminalSession, DEFAULT_SCROLLBACK_BYTES};
use crate::domain::ide::commands::IdeStore;
use crate::domain::ide::types::CLAUDE_CODE_SSE_PORT_ENV;
use crate::error::{AppError, AppResult};
use crate::events::TerminalExited;
use crate::ids::ProjectId;
use crate::infra::pty;
use crate::infra::root_guard::ensure_within_root;
use crate::state::AppState;

struct SessionEntry {
    pty: pty::PtySession,
    project_id: ProjectId,
    cwd: String,
    shell: String,
    ring_buffer: Arc<Mutex<Vec<u8>>>,
    subscriber: Arc<Mutex<Option<Channel<InvokeResponseBody>>>>,
    running: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct TerminalStore(Mutex<HashMap<String, SessionEntry>>);

impl TerminalStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn kill_all(&self) {
        for entry in self.0.lock().values() {
            let _ = entry.pty.kill();
        }
    }

    pub fn foreground_pids(&self, project_id: &ProjectId) -> Vec<(String, u32)> {
        self.0
            .lock()
            .iter()
            .filter(|(_, entry)| &entry.project_id == project_id)
            .filter_map(|(session_id, entry)| entry.pty.foreground_pid().map(|pid| (session_id.clone(), pid)))
            .collect()
    }
}

fn new_session_id() -> String {
    format!("term-{}", uuid::Uuid::new_v4())
}

fn ensure_project_open(state: &AppState, project_id: &ProjectId) -> AppResult<()> {
    if state.projects.read().contains_key(project_id) {
        return Ok(());
    }
    Err(AppError::NotFound(format!("project not open: {project_id}")))
}

fn find_entry<'a>(store: &'a HashMap<String, SessionEntry>, session_id: &str) -> AppResult<&'a SessionEntry> {
    store
        .get(session_id)
        .ok_or_else(|| AppError::NotFound(format!("terminal session not found: {session_id}")))
}

#[tauri::command]
#[specta::specta]
pub async fn pty_spawn(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, TerminalStore>,
    ide: State<'_, IdeStore>,
    opts: PtySpawnOptions,
    on_data: Channel<InvokeResponseBody>,
) -> AppResult<String> {
    let _guard = state.begin_mutation().await;
    ensure_project_open(&state, &opts.project_id)?;

    let session_id = new_session_id();
    let ring_buffer: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let subscriber: Arc<Mutex<Option<Channel<InvokeResponseBody>>>> = Arc::new(Mutex::new(Some(on_data)));
    let running = Arc::new(AtomicBool::new(true));

    let ring_for_data = ring_buffer.clone();
    let subscriber_for_data = subscriber.clone();

    let exit_app = app.clone();
    let exit_session_id = session_id.clone();
    let exit_running = running.clone();

    let ide_status = ide.status();
    let extra_env = if ide_status.running {
        vec![(CLAUDE_CODE_SSE_PORT_ENV.to_string(), ide_status.port.to_string())]
    } else {
        Vec::new()
    };

    let config = pty::PtySpawnConfig {
        shell: opts.shell.clone(),
        cwd: opts.cwd.clone(),
        cols: opts.cols,
        rows: opts.rows,
        extra_env,
    };

    let handle = pty::spawn(
        config,
        move |bytes| {
            service::ring_buffer_append(&mut ring_for_data.lock(), bytes, DEFAULT_SCROLLBACK_BYTES);
            if let Some(channel) = subscriber_for_data.lock().as_ref() {
                let _ = channel.send(InvokeResponseBody::Raw(bytes.to_vec()));
            }
        },
        move |code| {
            exit_running.store(false, Ordering::SeqCst);
            let _ = TerminalExited {
                session_id: exit_session_id,
                code,
            }
            .emit(&exit_app);
        },
    )?;

    let entry = SessionEntry {
        pty: handle,
        project_id: opts.project_id,
        cwd: opts.cwd,
        shell: opts.shell.unwrap_or_else(|| "default".to_string()),
        ring_buffer,
        subscriber,
        running,
    };

    store.0.lock().insert(session_id.clone(), entry);

    Ok(session_id)
}

#[tauri::command]
#[specta::specta]
pub async fn pty_write(store: State<'_, TerminalStore>, session_id: String, data: String) -> AppResult<()> {
    let sessions = store.0.lock();
    let entry = find_entry(&sessions, &session_id)?;
    entry.pty.write(data.as_bytes())
}

#[tauri::command]
#[specta::specta]
pub async fn pty_resize(store: State<'_, TerminalStore>, session_id: String, cols: u16, rows: u16) -> AppResult<()> {
    let sessions = store.0.lock();
    let entry = find_entry(&sessions, &session_id)?;
    entry.pty.resize(cols, rows)
}

#[tauri::command]
#[specta::specta]
pub async fn pty_kill(state: State<'_, AppState>, store: State<'_, TerminalStore>, session_id: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let removed = store.0.lock().remove(&session_id);
    match removed {
        Some(entry) => entry.pty.kill(),
        None => Err(AppError::NotFound(format!("terminal session not found: {session_id}"))),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn pty_set_paused(store: State<'_, TerminalStore>, session_id: String, paused: bool) -> AppResult<()> {
    let sessions = store.0.lock();
    let entry = find_entry(&sessions, &session_id)?;
    entry.pty.set_paused(paused);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn pty_attach(
    state: State<'_, AppState>,
    store: State<'_, TerminalStore>,
    session_id: String,
    on_data: Channel<InvokeResponseBody>,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let sessions = store.0.lock();
    let entry = find_entry(&sessions, &session_id)?;

    let snapshot = entry.ring_buffer.lock().clone();
    if !snapshot.is_empty() {
        let _ = on_data.send(InvokeResponseBody::Raw(snapshot));
    }
    *entry.subscriber.lock() = Some(on_data);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn terminal_sessions(store: State<'_, TerminalStore>, project_id: ProjectId) -> AppResult<Vec<TerminalSession>> {
    let sessions = store.0.lock();
    Ok(sessions
        .iter()
        .filter(|(_, entry)| entry.project_id == project_id)
        .map(|(id, entry)| TerminalSession {
            id: id.clone(),
            project_id: entry.project_id.clone(),
            cwd: entry.cwd.clone(),
            shell: entry.shell.clone(),
            running: entry.running.load(Ordering::SeqCst),
        })
        .collect())
}

#[tauri::command]
#[specta::specta]
pub async fn shell_profiles() -> AppResult<Vec<ShellProfile>> {
    Ok(service::list_shell_profiles())
}

#[tauri::command]
#[specta::specta]
pub async fn resolve_terminal_path(path: String, cwd: String) -> AppResult<String> {
    service::resolve_terminal_path(&path, &cwd)
}

const DEFAULT_TERMINAL_COLS: u16 = 80;
const DEFAULT_TERMINAL_ROWS: u16 = 24;

#[tauri::command]
#[specta::specta]
pub async fn pty_default_options(state: State<'_, AppState>, project_id: ProjectId, cwd: Option<String>) -> AppResult<PtySpawnOptions> {
    let root = state
        .projects
        .read()
        .get(&project_id)
        .map(|project| project.root.clone())
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))?;

    let resolved_cwd = match cwd {
        Some(requested) => ensure_within_root(std::path::Path::new(&root), std::path::Path::new(&requested))?
            .to_string_lossy()
            .to_string(),
        None => root,
    };

    Ok(PtySpawnOptions {
        project_id,
        cwd: resolved_cwd,
        shell: state.settings.read().shell_override.clone(),
        cols: DEFAULT_TERMINAL_COLS,
        rows: DEFAULT_TERMINAL_ROWS,
    })
}

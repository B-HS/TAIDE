use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, State};
use tauri_specta::Event;

use super::service;
use super::types::{PtySpawnOptions, ShellProfile, TerminalSession, DEFAULT_SCROLLBACK_BYTES, IDE_READY_POLL_INTERVAL_MS};
use crate::domain::ide::commands::IdeStore;
use crate::domain::ide::types::{IdeStatus, CLAUDE_CODE_SSE_PORT_ENV, IDE_READY_WAIT_MS};
use crate::error::{AppError, AppResult};
use crate::events::TerminalExited;
use crate::ids::ProjectId;
use crate::infra::pty;
use crate::infra::root_guard::ensure_within_root;
use crate::state::AppState;

/// Subscriber list entries — a subscription id (`pty_attach`'s return value, consumed by
/// `pty_detach`) paired with the channel it identifies.
type PtySubscribers = Vec<(u32, Channel<InvokeResponseBody>)>;

struct SessionEntry {
    pty: pty::PtySession,
    project_id: ProjectId,
    cwd: String,
    shell: String,
    ring_buffer: Arc<Mutex<Vec<u8>>>,
    /// Every window/client currently attached to this pty's output, keyed by a subscription id
    /// `pty_attach` hands back and `pty_detach` consumes to remove exactly that entry. `pty_attach`
    /// used to overwrite this with a single slot, so a second `attach` (a second window, or remote
    /// and desktop viewing the same session) silently stole the first subscriber's stream — see
    /// `docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md` §2.3. A channel whose
    /// underlying window has closed simply fails to `send` and `broadcast_output` prunes it on the
    /// next chunk of output, but that alone only covers the window-closed case — a re-attach from a
    /// *still-open* window (e.g. a terminal pane unmounting/remounting as the user switches tabs)
    /// would otherwise accumulate one live subscriber per re-attach forever, since its `send` never
    /// fails. `pty_detach` closes that gap: callers that stop displaying a session (effect cleanup)
    /// remove their own subscription explicitly instead of relying on the window closing.
    /// `pty_set_paused` pauses the *single* underlying `PauseGate` shared by this whole `PtySession`
    /// — it stops the one reader thread from reading the child process at all, so pausing is still
    /// session-wide and affects every subscriber identically; it was never a per-subscriber
    /// backpressure mechanism and multiplexing here doesn't change that. `Channel::send` itself
    /// doesn't block on a slow frontend either (it queues onto the webview's IPC rather than
    /// waiting for JS to drain it), so one laggy subscriber can't stall delivery to the others.
    subscribers: Arc<Mutex<PtySubscribers>>,
    next_subscription_id: Arc<AtomicU32>,
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

/// Sends `bytes` to every subscriber, keeping only the ones that accept it — a subscriber whose
/// `send` fails (e.g. its window has closed) is dropped rather than aborting the whole broadcast
/// or being retried. Extracted from the pty read loop's `on_data` callback above so the
/// broadcast/prune behavior itself can be unit tested without spawning a real pty.
fn broadcast_output(subscribers: &Mutex<PtySubscribers>, bytes: &[u8]) {
    subscribers
        .lock()
        .retain(|(_, channel)| channel.send(InvokeResponseBody::Raw(bytes.to_vec())).is_ok());
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

async fn wait_for_ide_ready(state: &AppState, ide: &IdeStore) -> IdeStatus {
    let ide_integration_enabled = state.settings.read().ide_integration_enabled;
    let mut status = ide.status();

    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(IDE_READY_WAIT_MS);
    while service::should_wait_for_ide_ready(ide_integration_enabled, status.running) && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(tokio::time::Duration::from_millis(IDE_READY_POLL_INTERVAL_MS)).await;
        status = ide.status();
    }

    status
}

/// `on_data` is accepted for IPC-contract stability (a Tauri `Channel` argument the frontend must
/// supply to spawn), but is intentionally **not** registered as a live subscriber — the sole caller
/// (`terminal-session.tsx`) always passes a no-op sink and relies on a follow-up `pty_attach` call
/// (whose `on_data` *is* what actually drives the terminal display) to get real output, once the
/// pty's ring buffer has something to replay. Seeding `subscribers` with this channel too used to
/// mean every session broadcast every output chunk twice from the moment it was created — once to
/// this inert sink, once to the real attach — for the session's entire lifetime.
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
    let ide_status = wait_for_ide_ready(state.inner(), ide.inner()).await;
    let extra_env = if ide_status.running {
        vec![(CLAUDE_CODE_SSE_PORT_ENV.to_string(), ide_status.port.to_string())]
    } else {
        Vec::new()
    };

    let _guard = state.begin_mutation().await;
    ensure_project_open(&state, &opts.project_id)?;
    drop(on_data);

    let session_id = new_session_id();
    let ring_buffer: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let subscribers: Arc<Mutex<PtySubscribers>> = Arc::new(Mutex::new(Vec::new()));
    let next_subscription_id = Arc::new(AtomicU32::new(0));
    let running = Arc::new(AtomicBool::new(true));

    let ring_for_data = ring_buffer.clone();
    let subscribers_for_data = subscribers.clone();

    let exit_app = app.clone();
    let exit_session_id = session_id.clone();
    let exit_running = running.clone();

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
            broadcast_output(&subscribers_for_data, bytes);
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
        subscribers,
        next_subscription_id,
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

/// Attaches a new subscriber to an already-running pty session — every previously-attached
/// subscriber (another window, or remote and desktop viewing the same session concurrently) keeps
/// receiving output too, instead of this call stealing the stream from them (Wave I §2.3). Each
/// attach gets its own ring-buffer replay so a subscriber that joins late still sees the session's
/// recent scrollback, without re-sending it to subscribers that were already caught up. Returns the
/// subscription id the caller must pass to [`pty_detach`] once it stops displaying the session
/// (effect cleanup on tab switch/unmount) — otherwise-live channels (window still open) are never
/// pruned by `broadcast_output`'s send-failure check alone, so without an explicit detach every
/// re-attach to the same still-open window accumulates one more permanent subscriber.
#[tauri::command]
#[specta::specta]
pub async fn pty_attach(
    state: State<'_, AppState>,
    store: State<'_, TerminalStore>,
    session_id: String,
    on_data: Channel<InvokeResponseBody>,
) -> AppResult<u32> {
    let _guard = state.begin_mutation().await;
    let sessions = store.0.lock();
    let entry = find_entry(&sessions, &session_id)?;

    let snapshot = entry.ring_buffer.lock().clone();
    if !snapshot.is_empty() {
        let _ = on_data.send(InvokeResponseBody::Raw(snapshot));
    }
    let subscription_id = entry.next_subscription_id.fetch_add(1, Ordering::SeqCst);
    entry.subscribers.lock().push((subscription_id, on_data));

    Ok(subscription_id)
}

/// Removes exactly the subscriber `pty_attach` registered under `subscription_id` — the counterpart
/// that lets a still-open window stop receiving a session's output without waiting for
/// `broadcast_output`'s send-failure pruning (which only fires once the window itself closes). A
/// session or subscription that no longer exists is treated as already-detached rather than an
/// error, since cleanup can legitimately race a `pty_kill` for the same session.
#[tauri::command]
#[specta::specta]
pub async fn pty_detach(
    state: State<'_, AppState>,
    store: State<'_, TerminalStore>,
    session_id: String,
    subscription_id: u32,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let sessions = store.0.lock();
    let Ok(entry) = find_entry(&sessions, &session_id) else {
        return Ok(());
    };
    entry.subscribers.lock().retain(|(id, _)| *id != subscription_id);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn recording_channel(received: Arc<Mutex<Vec<Vec<u8>>>>) -> Channel<InvokeResponseBody> {
        Channel::new(move |body| {
            if let InvokeResponseBody::Raw(bytes) = body {
                received.lock().push(bytes);
            }
            Ok(())
        })
    }

    fn failing_channel() -> Channel<InvokeResponseBody> {
        Channel::new(|_| Err(tauri::Error::AssetNotFound("closed".to_string())))
    }

    #[test]
    fn broadcast_은_모든_구독자에게_전달된다() {
        let received_a = Arc::new(Mutex::new(Vec::new()));
        let received_b = Arc::new(Mutex::new(Vec::new()));
        let subscribers = Mutex::new(vec![
            (0, recording_channel(received_a.clone())),
            (1, recording_channel(received_b.clone())),
        ]);

        broadcast_output(&subscribers, b"hello");

        assert_eq!(*received_a.lock(), vec![b"hello".to_vec()]);
        assert_eq!(*received_b.lock(), vec![b"hello".to_vec()]);
    }

    #[test]
    fn 단일_구독자_시나리오는_기존과_동일하게_전달된다() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let subscribers = Mutex::new(vec![(0, recording_channel(received.clone()))]);

        broadcast_output(&subscribers, b"first");
        broadcast_output(&subscribers, b"second");

        assert_eq!(*received.lock(), vec![b"first".to_vec(), b"second".to_vec()]);
    }

    #[test]
    fn 전송에_실패한_구독자는_다음_브로드캐스트에서_제거되고_남은_구독자는_계속_받는다() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let subscribers = Mutex::new(vec![(0, failing_channel()), (1, recording_channel(received.clone()))]);

        broadcast_output(&subscribers, b"first");
        assert_eq!(subscribers.lock().len(), 1, "실패한 구독자는 제거되어야 한다");

        broadcast_output(&subscribers, b"second");
        assert_eq!(*received.lock(), vec![b"first".to_vec(), b"second".to_vec()]);
    }

    #[test]
    fn detach_은_해당_구독_id_만_제거하고_나머지는_유지한다() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let subscribers = Mutex::new(vec![
            (0, failing_channel()),
            (1, recording_channel(received.clone())),
            (2, failing_channel()),
        ]);

        subscribers.lock().retain(|(id, _)| *id != 2);
        assert_eq!(subscribers.lock().iter().map(|(id, _)| *id).collect::<Vec<_>>(), vec![0, 1]);

        broadcast_output(&subscribers, b"data");
        assert_eq!(*received.lock(), vec![b"data".to_vec()]);
    }
}

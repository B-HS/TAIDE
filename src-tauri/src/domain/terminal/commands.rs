use std::collections::HashMap;
use std::future::Future;
use std::io::Write as _;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use super::service;
use super::types::{PtySpawnOptions, ShellProfile, TerminalSession, DEFAULT_SCROLLBACK_BYTES};
use crate::domain::project::types::Project;
use crate::error::{AppError, AppResult};
use crate::events::{TerminalCommandFinished, TerminalCwdChanged, TerminalExited};
use crate::ids::ProjectId;
use crate::infra::pty;
use crate::infra::root_guard::{self, ensure_within_root};
use crate::infra::shell_integration;
use crate::state::AppState;

/// Subscriber list entries — a subscription id (`pty_attach`'s return value, consumed by
/// `pty_detach`) paired with the channel it identifies.
type PtySubscribers = Vec<(u32, Channel<InvokeResponseBody>)>;

/// A session's scrollback and its subscriber list behind **one** lock, so no output chunk can slip
/// between [`pty_attach`]'s replay and its subscriber registration.
///
/// They used to be two independent `Mutex`es, and the reader thread took them one after the other
/// (append, then broadcast) while `pty_attach` took them in the same order (snapshot, then push).
/// A chunk that landed after the snapshot but before the push was **never delivered** to the
/// attaching subscriber — the replay predated it and the broadcast missed it — and, in the other
/// interleaving, a chunk appended before the snapshot but broadcast after the push arrived
/// **twice** (audit §4-A-5). Re-attaching happens on every terminal tab switch, so this showed up
/// as missing or repeated output right at the moment a pane remounted. With a single lock the two
/// halves are one critical section on both sides: the replay covers exactly what was appended
/// before the attach and the broadcast covers exactly what comes after.
///
/// Holding the lock across the replay/broadcast sends costs nothing extra — `Channel::send` queues
/// onto the webview's IPC rather than waiting for JS to drain it, so a slow frontend can't stall
/// the reader thread here any more than it could when the subscriber list had its own lock.
struct SessionOutput {
    scrollback: service::ScrollbackRing,
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
    /// backpressure mechanism and multiplexing here doesn't change that.
    subscribers: PtySubscribers,
    next_subscription_id: u32,
}

impl SessionOutput {
    fn new(scrollback_capacity: usize) -> Self {
        Self {
            scrollback: service::ScrollbackRing::new(scrollback_capacity),
            subscribers: Vec::new(),
            next_subscription_id: 0,
        }
    }

    /// Reader-thread path: record the chunk for future replays and hand it to every current
    /// subscriber, as one indivisible step.
    fn append_and_broadcast(&mut self, bytes: &[u8]) {
        self.scrollback.append(bytes);
        broadcast_output(&mut self.subscribers, bytes);
    }

    /// Attach path: replay the scrollback into `channel` and register it as a subscriber, as one
    /// indivisible step. The replay goes out as the ring's two halves (the second is empty until
    /// the buffer has wrapped) rather than being made contiguous first, which would `memmove` the
    /// whole 2MB scrollback on every attach — see [`service::ScrollbackRing::as_slices`].
    fn attach(&mut self, channel: Channel<InvokeResponseBody>) -> u32 {
        let (front, back) = self.scrollback.as_slices();
        for half in [front, back] {
            if half.is_empty() {
                continue;
            }
            let _ = channel.send(InvokeResponseBody::Raw(half.to_vec()));
        }

        let subscription_id = self.next_subscription_id;
        self.next_subscription_id = subscription_id.wrapping_add(1);
        self.subscribers.push((subscription_id, channel));
        subscription_id
    }

    fn detach(&mut self, subscription_id: u32) {
        self.subscribers.retain(|(id, _)| *id != subscription_id);
    }
}

struct SessionEntry {
    pty: pty::PtySession,
    project_id: ProjectId,
    cwd: String,
    shell: String,
    output: Arc<Mutex<SessionOutput>>,
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

    /// Kills and removes every pty session belonging to `project_id` — `kill_all`'s counterpart
    /// scoped to a single project, called by `project_close` so closing a project reliably reaps its
    /// terminals instead of leaving them running with no owning project open. Before this, nothing
    /// called `pty_kill` for a closed project's sessions at all; they lingered until the whole app
    /// quit (`TerminalStore::kill_all`).
    pub fn kill_project(&self, project_id: &ProjectId) {
        let mut sessions = self.0.lock();
        let session_ids: Vec<String> = sessions
            .iter()
            .filter(|(_, entry)| &entry.project_id == project_id)
            .map(|(session_id, _)| session_id.clone())
            .collect();

        for session_id in session_ids {
            if let Some(entry) = sessions.remove(&session_id) {
                let _ = entry.pty.kill();
            }
        }
    }

    /// Kills and removes a single pty session by id — `kill_project`'s counterpart for one session,
    /// called when a terminal tab closes (`layout::service::close_tab_and_finish`) so tab-close
    /// reliably reaps the pty it owned instead of leaving it running until the owning project or the
    /// whole app closes. A missing `session_id` is silently ignored, the same as `kill_project`
    /// tolerates a project with no sessions: the tab may already be pointing at a session that was
    /// reaped some other way (`pty_kill`, `project_close`) first.
    pub fn kill_session(&self, session_id: &str) {
        if let Some(entry) = self.0.lock().remove(session_id) {
            let _ = entry.pty.kill();
        }
    }
}

fn new_session_id() -> String {
    format!("term-{}", uuid::Uuid::new_v4())
}

/// Sends `bytes` to every subscriber, keeping only the ones that accept it — a subscriber whose
/// `send` fails (e.g. its window has closed) is dropped rather than aborting the whole broadcast
/// or being retried. Takes the already-locked subscriber list rather than the lock itself, because
/// its caller ([`SessionOutput::append_and_broadcast`]) holds that lock across the scrollback append
/// too; keeping it a free function also lets the broadcast/prune behavior be unit tested without
/// spawning a real pty.
fn broadcast_output(subscribers: &mut PtySubscribers, bytes: &[u8]) {
    subscribers.retain(|(_, channel)| channel.send(InvokeResponseBody::Raw(bytes.to_vec())).is_ok());
}

/// Applies one pty output chunk's detected cwd-report (`infra::shell_integration::
/// extract_latest_cwd`) to `session_id`'s [`SessionEntry::cwd`], emitting [`TerminalCwdChanged`] only
/// when it actually differs from the last known value — `precmd`/`PROMPT_COMMAND` fire on every
/// prompt render, not just after `cd`, so without this check the renderer would get one event per
/// command instead of one per genuine directory change. A `session_id` not yet present in
/// `TerminalStore` (the pty reader thread can start delivering output before `pty_spawn`'s own
/// `store.0.lock().insert` below runs) is a silent no-op — the entry starts with its correct
/// spawn-time cwd anyway, so nothing is lost, only a redundant early report skipped.
fn report_cwd_change(app: &AppHandle, session_id: &str, cwd: String) {
    let store = app.state::<TerminalStore>();
    let mut sessions = store.0.lock();
    let Some(entry) = sessions.get_mut(session_id) else {
        return;
    };
    if entry.cwd == cwd {
        return;
    }
    entry.cwd = cwd.clone();
    drop(sessions);

    let _ = TerminalCwdChanged {
        session_id: session_id.to_string(),
        cwd,
    }
    .emit(app);
}

/// Folds one OSC 133 command marker (`infra::shell_integration::extract_command_markers`) into
/// `session_id`'s command clock, emitting [`TerminalCommandFinished`] for every command that was
/// actually timed.
///
/// The clock lives on the pty reader thread because that is the only place a session's output is
/// seen whether or not a window is displaying it — and "nobody is watching" is exactly the case the
/// task-completion notification exists for. The frontend's own OSC 133 tracker
/// (`features/terminal/terminal-osc133.ts`) cannot serve that case: `pane-node-view.tsx` renders
/// only the active tab, so switching away unmounts `TerminalSession`, detaches the pty and disposes
/// the xterm instance the tracker lives in; when the user returns, `pty_attach`'s scrollback replay
/// re-parses the same `C`/`D` bytes within milliseconds of each other, timing an hour-long build as
/// an instant one (batch 4 review F-1). Here the two markers are separated by real elapsed time.
///
/// A `D` with no remembered start (the `C` fell on a chunk boundary, or the session was already
/// running one command when the marker scan first saw it) reports nothing rather than a duration it
/// would have to invent — the same "not measurable ≠ instant" rule the frontend gate applies.
fn report_command_marker(app: &AppHandle, session_id: &str, started_at: &Mutex<Option<Instant>>, marker: shell_integration::CommandMarker) {
    let exit_code = match marker {
        shell_integration::CommandMarker::OutputStart => {
            *started_at.lock() = Some(Instant::now());
            return;
        }
        shell_integration::CommandMarker::Finished { exit_code } => exit_code,
    };

    let Some(started) = started_at.lock().take() else {
        return;
    };

    let cwd = app.state::<TerminalStore>().0.lock().get(session_id).map(|entry| entry.cwd.clone());

    let _ = TerminalCommandFinished {
        session_id: session_id.to_string(),
        cwd,
        exit_code,
        duration_ms: u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX),
    }
    .emit(app);
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

pub type PtySpawnEnvFuture<'a> = Pin<Box<dyn Future<Output = Vec<(String, String)>> + Send + 'a>>;

/// The extra `(name, value)` environment entries [`pty_spawn`] injects into every new shell,
/// contributed by whatever integration the assembly wires in. `lib.rs` registers the concrete
/// provider (currently `ide::store::claude_terminal_env` — the Claude Code SSE port — concatenated
/// with `agent::commands::editor_terminal_env` — the `EDITOR` pointing at the `taide` CLI) so this
/// domain never reads the IDE server's state directly (audit R8#10, T1-I §1.4). Awaited before
/// the mutation guard is taken, exactly where the old inline IDE-ready wait ran — the provider
/// may block the spawn briefly (bounded by its own deadline), never the whole app.
pub struct PtySpawnEnvProvider(Box<dyn for<'a> Fn(&'a AppHandle) -> PtySpawnEnvFuture<'a> + Send + Sync>);

impl PtySpawnEnvProvider {
    pub fn new(provider: Box<dyn for<'a> Fn(&'a AppHandle) -> PtySpawnEnvFuture<'a> + Send + Sync>) -> Self {
        Self(provider)
    }

    pub async fn extra_env(&self, app: &AppHandle) -> Vec<(String, String)> {
        (self.0)(app).await
    }
}

/// `on_data` is accepted for IPC-contract stability (a Tauri `Channel` argument the frontend must
/// supply to spawn), but is intentionally **not** registered as a live subscriber — the sole caller
/// (`terminal-session.tsx`) always passes a no-op sink and relies on a follow-up `pty_attach` call
/// (whose `on_data` *is* what actually drives the terminal display) to get real output, once the
/// pty's ring buffer has something to replay. Seeding `subscribers` with this channel too used to
/// mean every session broadcast every output chunk twice from the moment it was created — once to
/// this inert sink, once to the real attach — for the session's entire lifetime.
///
/// The pty spawn itself runs guard-held on a blocking thread (contract 2026-08-25 §1-d, same shape
/// as the git2 in-process migrations in `domain::git::commands`): `pty::spawn` writes the
/// shell-integration script(s) to a temp dir, opens the pty, and forks/execs the child, all
/// synchronous blocking work that previously ran directly on this async worker thread while
/// `_guard` was held. T1-H (2026-08-19 §4) deferred this exact move on three grounds, all since
/// resolved: (1) severity-tier framing only, not a correctness blocker; (2) "no reap path for an
/// orphaned pty" (R8#1) — closed by T1-J, and independently by `PtySession`'s `Drop` impl, which
/// the doc on that impl states "guarantees a `PtySession` never leaks its ... child process ... no
/// matter how it stops being reachable"; the one path this migration adds — the outer future
/// getting dropped while awaiting the join handle below, before `store.0.lock().insert` runs — is
/// exactly the "unreachable any other way" case that guarantee already covers, so a spawn that
/// never reaches `TerminalStore` still self-kills on drop; (3) T1-J landed (PROCESS.md d-8), so no
/// in-flight conflict remains. The closures were already required to be `Send + 'static` by
/// `pty::spawn`'s own bounds, so wrapping the whole call needs no new bounds.
#[tauri::command]
#[specta::specta]
pub async fn pty_spawn(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, TerminalStore>,
    env_provider: State<'_, PtySpawnEnvProvider>,
    opts: PtySpawnOptions,
    on_data: Channel<InvokeResponseBody>,
) -> AppResult<String> {
    let extra_env = env_provider.extra_env(&app).await;

    let _guard = state.begin_mutation().await;
    ensure_project_open(&state, &opts.project_id)?;
    drop(on_data);

    let session_id = new_session_id();
    let output = Arc::new(Mutex::new(SessionOutput::new(DEFAULT_SCROLLBACK_BYTES)));
    let running = Arc::new(AtomicBool::new(true));

    let output_for_data = output.clone();

    let exit_app = app.clone();
    let exit_session_id = session_id.clone();
    let exit_running = running.clone();

    let cwd_app = app.clone();
    let cwd_session_id = session_id.clone();

    let marker_app = app.clone();
    let marker_session_id = session_id.clone();
    let command_started_at = Mutex::new(None);

    let config = pty::PtySpawnConfig {
        shell: opts.shell.clone(),
        cwd: opts.cwd.clone(),
        cols: opts.cols,
        rows: opts.rows,
        extra_env,
    };

    let handle = tauri::async_runtime::spawn_blocking(move || {
        pty::spawn(
            config,
            move |bytes| {
                output_for_data.lock().append_and_broadcast(bytes);
                if let Some(cwd) = shell_integration::extract_latest_cwd(bytes) {
                    report_cwd_change(&cwd_app, &cwd_session_id, cwd);
                }
                for marker in shell_integration::extract_command_markers(bytes) {
                    report_command_marker(&marker_app, &marker_session_id, &command_started_at, marker);
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
        )
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))??;

    let entry = SessionEntry {
        pty: handle,
        project_id: opts.project_id,
        cwd: opts.cwd,
        shell: opts.shell.unwrap_or_else(|| "default".to_string()),
        output,
        running,
    };

    store.0.lock().insert(session_id.clone(), entry);

    Ok(session_id)
}

/// Holds `TerminalStore`'s lock only long enough to clone out the session's writer handle
/// (`PtySession::writer_handle` — an `Arc` around the same inner `Mutex<Box<dyn Write>>`
/// `PtySession::write` locks, so this is not a second, independent write path), not for the write
/// itself. The write blocks on the child's stdin pipe, which backs up (and this call blocks with
/// it) whenever the child isn't reading its input — previously that blocked write held the *whole
/// store's* lock, so every other terminal command (`pty_resize`, `pty_kill` for this very session,
/// `pty_attach`/`pty_detach` for any other session, `terminal_sessions`) queued behind it until the
/// child drained its input or was killed by some other means, which nothing could do while
/// `pty_kill` itself was one of the commands stuck waiting on the same lock.
///
/// That same blocking write also has to leave the async runtime's worker pool, for the same reason
/// the git and file domains moved theirs off it (§2 M-6): a child that has stopped reading its
/// stdin (a full pipe, a stopped process) pins the worker thread for as long as it takes to drain,
/// and enough of those starve every other command the runtime has to poll. The writer handle is
/// cloned out first so the blocking closure owns an `Arc` and needs no `State` (which isn't
/// `'static`), leaving the command's signature — and therefore the IPC surface — unchanged.
#[tauri::command]
#[specta::specta]
pub async fn pty_write(store: State<'_, TerminalStore>, session_id: String, data: String) -> AppResult<()> {
    let writer = {
        let sessions = store.0.lock();
        find_entry(&sessions, &session_id)?.pty.writer_handle()
    };

    tauri::async_runtime::spawn_blocking(move || {
        let mut writer = writer.lock();
        writer.write_all(data.as_bytes())?;
        writer.flush()?;
        Ok(())
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?
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
/// attach gets its own scrollback replay so a subscriber that joins late still sees the session's
/// recent output, without re-sending it to subscribers that were already caught up — and the replay
/// and the registration happen inside one [`SessionOutput`] critical section, so the reader thread
/// cannot slip a chunk between them (see that type's doc for the duplication/loss this closes).
/// Returns the
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

    let subscription_id = entry.output.lock().attach(on_data);
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
    entry.output.lock().detach(subscription_id);
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

/// Resolves a terminal-link `path`/`cwd` pair (both untrusted — `path` comes from regex-matched pty
/// output text, `cwd` from an OSC 7 report the pty's child process controls) to an absolute path,
/// but only ever returns one that falls inside an open project root. Every other command that
/// resolves a caller-supplied path this way (`file_open`, `pty_default_options`) gates it behind
/// `root_guard`; this one previously didn't, and `AppState` wasn't even in its signature to make
/// that possible.
///
/// A path outside every open root and a path that plain doesn't exist both map to the same
/// `AppError::NotFound` — deliberately not `AppError::Forbidden` for the escape case — so a caller
/// (including an authenticated remote mirror, which this command stays allowed for) can't use the
/// error variant to probe whether an arbitrary filesystem path exists outside the project. `cwd`
/// itself is never separately validated: only the *joined-then-canonicalized result* actually
/// matters (an absolute `path` ignores `cwd` entirely — `service::resolve_terminal_path` — so
/// gating on `cwd` up front would wrongly reject valid absolute-path links whenever the session's
/// live cwd has simply wandered outside the project, an everyday, non-malicious terminal action).
fn guard_terminal_path(projects: &HashMap<ProjectId, Project>, path: &str, cwd: &str) -> AppResult<String> {
    let resolved = service::resolve_terminal_path(path, cwd)?;

    root_guard::resolve_owning_project(projects, std::path::Path::new(&resolved))
        .map_err(|_| AppError::NotFound(format!("path not found: {path}")))?;

    Ok(resolved)
}

#[tauri::command]
#[specta::specta]
pub async fn resolve_terminal_path(state: State<'_, AppState>, path: String, cwd: String) -> AppResult<String> {
    let projects = state.projects.read().clone();
    guard_terminal_path(&projects, &path, &cwd)
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
    use std::path::Path;

    /// `resolve_terminal_path`'s `#[tauri::command]` wrapper needs a real `State<'_, AppState>`,
    /// which (unlike the plain `HashMap` [`guard_terminal_path`] takes) has no public constructor
    /// outside a running Tauri app — this codebase has no `tauri::test` mock-app harness anywhere
    /// (same constraint `root_guard.rs`'s own tests work around). These tests call
    /// `guard_terminal_path` directly instead — it's the actual guard logic the command runs, just
    /// factored out from the `State` extraction so it's plainly testable.
    fn single_project(id: &str, root: &Path) -> HashMap<ProjectId, Project> {
        let mut projects = HashMap::new();
        projects.insert(
            ProjectId::from(id.to_string()),
            Project {
                id: ProjectId::from(id.to_string()),
                root: root.to_string_lossy().to_string(),
                name: "project".to_string(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        projects
    }

    #[test]
    fn guard_terminal_path는_프로젝트_루트_안의_경로를_통과시킨다() {
        let dir = std::env::temp_dir().join(format!("taide-terminal-path-guard-inside-{}", uuid::Uuid::new_v4()));
        let root = dir.join("project");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src").join("main.rs"), b"fn main() {}").unwrap();

        let projects = single_project("project-1", &root);
        let resolved = guard_terminal_path(&projects, "src/main.rs", &root.to_string_lossy());

        assert!(resolved.is_ok(), "루트 안 경로는 통과해야 한다");
        assert!(resolved
            .unwrap()
            .starts_with(&std::fs::canonicalize(&root).unwrap().to_string_lossy().to_string()));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn guard_terminal_path는_루트_밖_경로를_notfound로_거부한다() {
        let dir = std::env::temp_dir().join(format!("taide-terminal-path-guard-outside-{}", uuid::Uuid::new_v4()));
        let root = dir.join("project");
        let outside_dir = dir.join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside_dir).unwrap();
        std::fs::write(outside_dir.join("secret.txt"), b"secret").unwrap();

        let projects = single_project("project-1", &root);
        let error =
            guard_terminal_path(&projects, "secret.txt", &outside_dir.to_string_lossy()).expect_err("루트 밖 경로는 거부되어야 한다");

        assert!(
            matches!(error, AppError::NotFound(_)),
            "존재 여부 오라클을 막기 위해 Forbidden 이 아니라 NotFound 로 나와야 한다"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn guard_terminal_path는_존재하지_않는_경로도_notfound로_거부한다() {
        let dir = std::env::temp_dir().join(format!("taide-terminal-path-guard-missing-{}", uuid::Uuid::new_v4()));
        let root = dir.join("project");
        std::fs::create_dir_all(&root).unwrap();

        let projects = single_project("project-1", &root);
        let error =
            guard_terminal_path(&projects, "does/not/exist.rs", &root.to_string_lossy()).expect_err("존재하지 않는 경로는 거부되어야 한다");

        assert!(matches!(error, AppError::NotFound(_)));

        std::fs::remove_dir_all(&dir).ok();
    }

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
        let mut subscribers = vec![
            (0, recording_channel(received_a.clone())),
            (1, recording_channel(received_b.clone())),
        ];

        broadcast_output(&mut subscribers, b"hello");

        assert_eq!(*received_a.lock(), vec![b"hello".to_vec()]);
        assert_eq!(*received_b.lock(), vec![b"hello".to_vec()]);
    }

    #[test]
    fn 단일_구독자_시나리오는_기존과_동일하게_전달된다() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let mut subscribers = vec![(0, recording_channel(received.clone()))];

        broadcast_output(&mut subscribers, b"first");
        broadcast_output(&mut subscribers, b"second");

        assert_eq!(*received.lock(), vec![b"first".to_vec(), b"second".to_vec()]);
    }

    #[test]
    fn 전송에_실패한_구독자는_다음_브로드캐스트에서_제거되고_남은_구독자는_계속_받는다() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let mut subscribers = vec![(0, failing_channel()), (1, recording_channel(received.clone()))];

        broadcast_output(&mut subscribers, b"first");
        assert_eq!(subscribers.len(), 1, "실패한 구독자는 제거되어야 한다");

        broadcast_output(&mut subscribers, b"second");
        assert_eq!(*received.lock(), vec![b"first".to_vec(), b"second".to_vec()]);
    }

    #[test]
    fn detach_은_해당_구독_id_만_제거하고_나머지는_유지한다() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let mut output = SessionOutput::new(TEST_SCROLLBACK_BYTES);
        output.attach(failing_channel());
        output.attach(recording_channel(received.clone()));
        let doomed = output.attach(failing_channel());

        output.detach(doomed);
        assert_eq!(output.subscribers.iter().map(|(id, _)| *id).collect::<Vec<_>>(), vec![0, 1]);

        output.append_and_broadcast(b"data");
        assert_eq!(*received.lock(), vec![b"data".to_vec()]);
    }

    const TEST_SCROLLBACK_BYTES: usize = 64 * 1024;

    #[test]
    fn attach_는_직전까지의_스크롤백만_재생하고_이후_출력은_브로드캐스트로_잇는다() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let mut output = SessionOutput::new(TEST_SCROLLBACK_BYTES);

        output.append_and_broadcast(b"before");
        output.attach(recording_channel(received.clone()));
        output.append_and_broadcast(b"after");

        assert_eq!(
            received.lock().concat(),
            b"beforeafter".to_vec(),
            "리플레이와 이후 브로드캐스트가 각 바이트를 정확히 한 번씩 전달해야 한다"
        );
    }

    #[test]
    fn 스크롤백이_비어_있으면_attach_는_아무것도_재생하지_않는다() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let mut output = SessionOutput::new(TEST_SCROLLBACK_BYTES);

        output.attach(recording_channel(received.clone()));

        assert!(received.lock().is_empty());
    }

    /// §4-A-5 회귀 가드. 스크롤백 append 와 브로드캐스트, 그리고 attach 의 리플레이와 구독 등록이
    /// **하나의 락** 아래 있다는 것이 곧 "재부착 순간 청크가 중복되거나 유실되지 않는다"의 근거이므로,
    /// 그 직렬화 자체를 검증한다: attach 가 진행 중인 동안 reader 경로의 append 는 진입할 수 없고,
    /// attach 가 끝난 뒤에야 그 청크가 새 구독자에게 정확히 한 번 도착한다. 락이 둘로 나뉘어 있던
    /// 이전 구현에서는 그 청크가 스냅샷 이후·등록 이전에 끼어들어 아무에게도 전달되지 않았다.
    #[test]
    fn attach_중에는_reader_의_출력이_끼어들_수_없다() {
        const APPEND_PROBE_MS: u64 = 50;

        let output = Arc::new(Mutex::new(SessionOutput::new(TEST_SCROLLBACK_BYTES)));
        output.lock().append_and_broadcast(b"before");

        let mut attaching = output.lock();

        let appended = Arc::new(AtomicBool::new(false));
        let writer_output = output.clone();
        let writer_appended = appended.clone();
        let writer = std::thread::spawn(move || {
            writer_output.lock().append_and_broadcast(b"during");
            writer_appended.store(true, Ordering::SeqCst);
        });

        std::thread::sleep(std::time::Duration::from_millis(APPEND_PROBE_MS));
        assert!(
            !appended.load(Ordering::SeqCst),
            "attach 가 락을 쥔 동안에는 reader 경로의 append 가 진행되면 안 된다"
        );

        let received = Arc::new(Mutex::new(Vec::new()));
        attaching.attach(recording_channel(received.clone()));
        drop(attaching);

        writer.join().expect("append 스레드 종료");

        assert_eq!(
            received.lock().concat(),
            b"beforeduring".to_vec(),
            "리플레이 이후 도착한 청크는 중복도 유실도 없이 이어져야 한다"
        );
    }

    /// 같은 §4-A-5 를 스레드 경합으로 한 번 더 조인다: 쓰기 스레드가 스트림을 흘리는 도중 아무
    /// 시점에 attach 가 끼어들어도, 그 구독자가 받은 바이트를 이어 붙이면 언제나 전체 스트림과
    /// **정확히** 같아야 한다(리플레이가 attach 이전 전부, 브로드캐스트가 이후 전부).
    #[test]
    fn 스트리밍_중_attach_해도_전체_스트림이_정확히_한_번씩_재구성된다() {
        const STREAM_CHUNKS: usize = 4096;
        const ATTEMPTS: usize = 16;

        for _ in 0..ATTEMPTS {
            let output = Arc::new(Mutex::new(SessionOutput::new(STREAM_CHUNKS)));
            let writer_output = output.clone();
            let writer = std::thread::spawn(move || {
                for index in 0..STREAM_CHUNKS {
                    writer_output.lock().append_and_broadcast(&[(index % 251) as u8]);
                }
            });

            let received = Arc::new(Mutex::new(Vec::new()));
            output.lock().attach(recording_channel(received.clone()));
            writer.join().expect("쓰기 스레드 종료");

            let full: Vec<u8> = (0..STREAM_CHUNKS).map(|index| (index % 251) as u8).collect();
            assert_eq!(received.lock().concat(), full);
        }
    }
}

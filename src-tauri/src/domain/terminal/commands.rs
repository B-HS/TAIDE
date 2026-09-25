use std::collections::HashMap;
use std::future::Future;
use std::io::Write as _;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use taide_terminal::command_clock::TerminalCommandClock;
use taide_terminal::metadata::TerminalSessionMetadata;
use taide_terminal::session::TerminalSessionOutput;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use super::service;
use super::types::{self, PtyAttachResult, PtySpawnOptions, ShellProfile, TerminalSession};
use crate::domain::project::types::Project;
use crate::error::{AppError, AppResult};
use crate::events::{TerminalCommandFinished, TerminalCwdChanged, TerminalExited, TerminalSpawned};
use crate::ids::ProjectId;
use crate::infra::perf::{self, CounterSlot};
use crate::infra::pty;
use crate::infra::root_guard::{self, ensure_within_root};
use crate::infra::shell_integration;
use crate::infra::terminal_scan::{OutputScanner, ScanEvent, ScanOutcome};
use crate::state::AppState;

struct SessionEntry {
    pty: pty::PtySession,
    metadata: Arc<TerminalSessionMetadata>,
    output: Arc<TerminalSessionOutput>,
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
            .filter(|(_, entry)| entry.metadata.project_id() == project_id)
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
            .filter(|(_, entry)| entry.metadata.project_id() == project_id)
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

fn output_channel_sink(channel: Channel<InvokeResponseBody>) -> impl Fn(&[u8]) -> bool + Send + Sync {
    move |bytes| channel.send(InvokeResponseBody::Raw(bytes.to_vec())).is_ok()
}

/// Applies one pty output chunk's detected cwd-report (`infra::terminal_scan::ScanEvent::Cwd`)
/// to `session_id`'s metadata, emitting [`TerminalCwdChanged`] only
/// when it actually differs from the last known value — `precmd`/`PROMPT_COMMAND` fire on every
/// prompt render, not just after `cd`, so without this check the renderer would get one event per
/// command instead of one per genuine directory change. A `session_id` not yet present in
/// `TerminalStore` (the pty reader thread can start delivering output before `pty_spawn`'s own
/// `store.0.lock().insert` below runs) is a silent no-op — the entry starts with its correct
/// spawn-time cwd anyway, so nothing is lost, only a redundant early report skipped.
fn report_cwd_change(app: &AppHandle, session_id: &str, cwd: String) {
    let store = app.state::<TerminalStore>();
    let sessions = store.0.lock();
    let Some(entry) = sessions.get(session_id) else {
        return;
    };
    if !entry.metadata.update_cwd(cwd.clone()) {
        return;
    }
    drop(sessions);

    let _ = TerminalCwdChanged {
        session_id: session_id.to_string(),
        cwd,
    }
    .emit(app);
}

/// Emits [`TerminalCommandFinished`] for every command the session clock actually timed.
///
/// The clock lives on the pty reader thread because that is the only place a session's output is
/// seen whether or not a window is displaying it — and "nobody is watching" is exactly the case the
/// task-completion notification exists for. The frontend's own OSC 133 tracker
/// (`features/terminal/terminal-osc133.ts`) cannot serve that case: `pane-node-view.tsx` renders
/// only the active tab, so switching away unmounts `TerminalSession`, detaches the pty and disposes
/// the xterm instance the tracker lives in; when the user returns, `pty_attach`'s scrollback replay
/// re-parses the same `C`/`D` bytes within milliseconds of each other, timing an hour-long build as
/// an instant one (batch 4 review F-1). Here the two markers are separated by real elapsed time.
fn report_command_marker(
    app: &AppHandle,
    session_id: &str,
    command_clock: &TerminalCommandClock,
    marker: shell_integration::CommandMarker,
) {
    let Some(timed) = command_clock.record(marker, Instant::now()) else {
        return;
    };

    let cwd = app
        .state::<TerminalStore>()
        .0
        .lock()
        .get(session_id)
        .map(|entry| entry.metadata.cwd());

    let _ = TerminalCommandFinished {
        session_id: session_id.to_string(),
        cwd,
        exit_code: timed.exit_code,
        duration_ms: timed.duration_ms,
    }
    .emit(app);
}

/// What a pty session just did, for the domains that read a terminal's traffic without owning one.
pub enum PtySessionSignal<'a> {
    /// One output chunk, already scanned (`infra::terminal_scan`).
    Output(&'a ScanOutcome),
    /// The user wrote to this session's pty.
    Input,
}

pub type PtySessionObserver = Box<dyn Fn(&AppHandle, &str, &PtySessionSignal<'_>) + Send + Sync>;

/// The reactions a pty session's traffic triggers outside this domain — currently the agent
/// domain's activity signals, which are defined over a session's own output and input (contract
/// 2026-09-06 §1.1). `lib.rs`'s assembly registers the concrete observers so the terminal domain
/// never calls into agent and the existing `agent → terminal` edge does not become a cycle
/// (architecture.md §2, same assembly-owned wiring as `PtySpawnEnvProvider`).
///
/// Observers run **synchronously on the pty reader thread**, once per output chunk: they must do
/// no IO and take no lock another pty command holds.
pub struct PtySessionObservers(Vec<PtySessionObserver>);

impl PtySessionObservers {
    pub fn new(observers: Vec<PtySessionObserver>) -> Self {
        Self(observers)
    }

    fn notify(&self, app: &AppHandle, session_id: &str, signal: &PtySessionSignal<'_>) {
        for observer in &self.0 {
            observer(app, session_id, signal);
        }
    }
}

fn notify_session_observers(app: &AppHandle, session_id: &str, signal: &PtySessionSignal<'_>) {
    let Some(observers) = app.try_state::<PtySessionObservers>() else {
        return;
    };
    observers.notify(app, session_id, signal);
}

/// The single place one scanned chunk becomes app-visible effects, so every signal the scanner
/// learns to recognize is wired in exactly once instead of at a growing list of call sites inside
/// the pty reader closure.
///
/// Cwd is collapsed to the chunk's last report before anything else runs: a batched chunk can carry
/// several prompt renders and only the newest describes where the shell now is, and applying it
/// first is what lets a command marker in the same chunk report the directory the command actually
/// ran in. The variants this domain has no use for are handed to the registered observers as a
/// whole outcome, which is also how the normalized text reaches the agent signals.
pub(crate) fn dispatch_scan_outcome(app: &AppHandle, session_id: &str, command_clock: &TerminalCommandClock, outcome: &ScanOutcome) {
    if let Some(cwd) = outcome.latest_cwd() {
        report_cwd_change(app, session_id, cwd.to_string());
    }

    for event in &outcome.events {
        match event {
            ScanEvent::CommandMarker(marker) => report_command_marker(app, session_id, command_clock, *marker),
            ScanEvent::Cwd(_) | ScanEvent::Title(_) | ScanEvent::AgentEvent(_) | ScanEvent::Notification9(_) => {}
        }
    }

    notify_session_observers(app, session_id, &PtySessionSignal::Output(outcome));
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
    let output = Arc::new(TerminalSessionOutput::new(types::resolve_scrollback_bytes(opts.scrollback_bytes)));
    let metadata = Arc::new(TerminalSessionMetadata::new(
        opts.project_id.clone(),
        opts.cwd.clone(),
        opts.shell.clone().unwrap_or_else(|| "default".to_string()),
    ));

    let output_for_data = output.clone();

    let exit_app = app.clone();
    let exit_session_id = session_id.clone();
    let exit_metadata = metadata.clone();

    let scan_app = app.clone();
    let scan_session_id = session_id.clone();
    let command_clock = TerminalCommandClock::new();
    let scanner = Mutex::new(OutputScanner::new());

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
                // Counters, never a `perf::span`: this closure runs once per pty output chunk on
                // the reader thread, where two `Instant::now()` calls per chunk would measure the
                // instrumentation as much as the terminal (`infra::perf::CounterSlot`). Throughput
                // is `pty.output_bytes` over the wall time between two `perf_snapshot` calls.
                perf::add(CounterSlot::PtyOutputBytes, bytes.len() as u64);
                perf::add(CounterSlot::PtyOutputChunks, 1);
                output_for_data.append_and_broadcast(bytes);

                let outcome = scanner.lock().scan(bytes);
                perf::add(CounterSlot::PtyScanEvents, outcome.events.len() as u64);
                dispatch_scan_outcome(&scan_app, &scan_session_id, &command_clock, &outcome);
            },
            move |code| {
                exit_metadata.mark_exited();
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
        metadata,
        output,
    };

    let spawned = TerminalSpawned {
        session_id: session_id.clone(),
        project_id: entry.metadata.project_id().clone(),
        cwd: entry.metadata.cwd(),
        shell: entry.metadata.shell().to_string(),
    };
    store.0.lock().insert(session_id.clone(), entry);
    let _ = spawned.emit(&app);

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
pub async fn pty_write(app: AppHandle, store: State<'_, TerminalStore>, session_id: String, data: String) -> AppResult<()> {
    notify_session_observers(&app, &session_id, &PtySessionSignal::Input);

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
/// and the registration happen inside one [`TerminalSessionOutput`] critical section, so the reader thread
/// cannot slip a chunk between them (see that type's doc for the duplication/loss this closes).
/// Returns the
/// subscription id the caller must pass to [`pty_detach`] once it stops displaying the session
/// (effect cleanup on tab switch/unmount) — otherwise-live channels (window still open) are never
/// pruned by the output sink's send-failure check alone, so without an explicit detach every
/// re-attach to the same still-open window accumulates one more permanent subscriber — alongside
/// the replayed byte count the caller needs to keep that replay out of its flow-control accounting
/// (see [`PtyAttachResult`]).
#[tauri::command]
#[specta::specta]
pub async fn pty_attach(
    state: State<'_, AppState>,
    store: State<'_, TerminalStore>,
    session_id: String,
    on_data: Channel<InvokeResponseBody>,
) -> AppResult<PtyAttachResult> {
    let _guard = state.begin_mutation().await;
    let sessions = store.0.lock();
    let entry = find_entry(&sessions, &session_id)?;

    let attached = entry.output.attach(output_channel_sink(on_data));
    Ok(attached)
}

/// Removes exactly the subscriber `pty_attach` registered under `subscription_id` — the counterpart
/// that lets a still-open window stop receiving a session's output without waiting for
/// the output sink's send-failure pruning (which only fires once the window itself closes). A
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
    entry.output.detach(subscription_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn terminal_sessions(store: State<'_, TerminalStore>, project_id: ProjectId) -> AppResult<Vec<TerminalSession>> {
    let sessions = store.0.lock();
    Ok(sessions
        .iter()
        .filter(|(_, entry)| entry.metadata.project_id() == &project_id)
        .map(|(id, entry)| entry.metadata.snapshot(id))
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

/// How many candidates one [`terminal_resolve_link_candidates`] call actually resolves. The unit of
/// work is one terminal row, and a row of dense punctuation (a stack trace, a `PATH` dump) can
/// regex-match far more candidates than a person could ever click; everything past this bound is
/// answered `None` instead of turning a single row's render into that many `canonicalize` syscalls.
const MAX_LINK_CANDIDATES_PER_ROW: usize = 16;

/// Resolves a row's worth of link candidates against `cwd` in one pass, answering positionally so
/// the caller can zip the results back onto the matches it sent.
///
/// Every candidate goes through the same [`guard_terminal_path`] a click would, and its failures —
/// outside every open project root, or simply not there — are folded into `None`. That keeps the
/// non-existence oracle closed exactly as `resolve_terminal_path` does (see its doc): a caller
/// learns "this is not a link", never which of the two reasons applies, and one bad candidate never
/// fails the whole row.
fn resolve_link_candidates(projects: &HashMap<ProjectId, Project>, cwd: &str, candidates: &[String]) -> Vec<Option<String>> {
    candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            if index >= MAX_LINK_CANDIDATES_PER_ROW {
                return None;
            }
            guard_terminal_path(projects, candidate, cwd).ok()
        })
        .collect()
}

/// Answers "which of these regex matches are real files?" for one terminal row, so the renderer can
/// underline only the candidates it can actually open instead of underlining every match and
/// failing at click time (`v18.20.4`, `127.0.0.1:8080`, `0.123s` all match the path pattern).
#[tauri::command]
#[specta::specta]
pub async fn terminal_resolve_link_candidates(
    state: State<'_, AppState>,
    cwd: String,
    candidates: Vec<String>,
) -> AppResult<Vec<Option<String>>> {
    let projects = state.projects.read().clone();
    Ok(resolve_link_candidates(&projects, &cwd, &candidates))
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
        scrollback_bytes: None,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use super::*;
    use std::path::{Path, PathBuf};

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

    fn link_candidate_fixture(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("taide-terminal-link-candidates-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("project").join("src")).unwrap();
        std::fs::write(dir.join("project").join("src").join("main.rs"), b"fn main() {}").unwrap();
        std::fs::write(dir.join("project").join("README.md"), b"# readme").unwrap();
        dir
    }

    #[test]
    fn 링크_후보는_입력_순서대로_존재하는_것만_경로를_돌려준다() {
        let dir = link_candidate_fixture("order");
        let root = dir.join("project");
        let projects = single_project("project-1", &root);

        let candidates = ["src/main.rs", "v18.20.4", "README.md"].map(str::to_string);
        let resolved = resolve_link_candidates(&projects, &root.to_string_lossy(), &candidates);

        assert_eq!(resolved.len(), candidates.len());
        assert!(
            resolved[0].as_deref().unwrap().ends_with("main.rs"),
            "첫 후보의 자리에 첫 결과가 와야 한다"
        );
        assert_eq!(resolved[1], None, "존재하지 않는 후보는 None 이다");
        assert!(resolved[2].as_deref().unwrap().ends_with("README.md"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 링크_후보_상한을_넘은_자리는_해석하지_않고_none_이_된다() {
        let dir = link_candidate_fixture("limit");
        let root = dir.join("project");
        let names: Vec<String> = (0..=MAX_LINK_CANDIDATES_PER_ROW).map(|index| format!("file-{index}.txt")).collect();
        for name in &names {
            std::fs::write(root.join(name), b"x").unwrap();
        }

        let projects = single_project("project-1", &root);
        let resolved = resolve_link_candidates(&projects, &root.to_string_lossy(), &names);

        assert_eq!(resolved.len(), names.len());
        assert!(
            resolved[..MAX_LINK_CANDIDATES_PER_ROW].iter().all(Option::is_some),
            "상한 안의 후보는 전부 해석되어야 한다"
        );
        assert_eq!(
            resolved[MAX_LINK_CANDIDATES_PER_ROW], None,
            "존재하는 파일이라도 상한을 넘으면 해석하지 않는다"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 루트_밖_링크_후보는_에러가_아니라_none_으로_접힌다() {
        let dir = link_candidate_fixture("outside");
        let root = dir.join("project");
        let outside_dir = dir.join("outside");
        std::fs::create_dir_all(&outside_dir).unwrap();
        std::fs::write(outside_dir.join("secret.txt"), b"secret").unwrap();

        let projects = single_project("project-1", &root);
        let candidates = ["secret.txt".to_string()];
        let resolved = resolve_link_candidates(&projects, &outside_dir.to_string_lossy(), &candidates);

        assert_eq!(resolved, vec![None], "루트 밖 존재 여부가 결과로 새어나가면 안 된다");

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

    const TEST_SCROLLBACK_BYTES: usize = 64 * 1024;

    #[test]
    fn 출력_채널_어댑터는_바이너리_재생과_실시간_출력을_전달한다() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let output = TerminalSessionOutput::new(TEST_SCROLLBACK_BYTES);

        output.append_and_broadcast(b"before");
        let attached = output.attach(output_channel_sink(recording_channel(received.clone())));
        output.append_and_broadcast(b"after");

        assert_eq!(
            attached.replay_bytes as usize,
            taide_terminal::session::TERMINAL_REPLAY_PREAMBLE.len() + b"before".len()
        );
        assert_eq!(
            received.lock().concat(),
            [taide_terminal::session::TERMINAL_REPLAY_PREAMBLE, b"beforeafter"].concat()
        );
    }

    #[test]
    fn 출력_채널_전송_실패는_구독_정리_신호가_된다() {
        let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let channel_attempts = attempts.clone();
        let failed_channel = Channel::new(move |_| {
            channel_attempts.fetch_add(1, Ordering::SeqCst);
            Err(tauri::Error::AssetNotFound("closed".to_string()))
        });
        let output = TerminalSessionOutput::new(TEST_SCROLLBACK_BYTES);
        output.attach(output_channel_sink(failed_channel));

        output.append_and_broadcast(b"first");
        output.append_and_broadcast(b"second");

        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }
}

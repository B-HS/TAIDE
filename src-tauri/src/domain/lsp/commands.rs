use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use super::manifest;
use super::service;
use super::types::{
    LanguageServerSpec, LspInstallPhase, LspInstallStrategy, LspServerDetection, LspServerId, LspSessionInfo, LspSessionStatus,
    LspSpawnRequest, RESTART_BACKOFF_LIMIT,
};
use crate::error::{AppError, AppResult};
use crate::events::{LspInstallProgress, LspSessionStatusChanged};
use crate::ids::ProjectId;
use crate::infra::http::{outbound_http_client, HttpClientProfile};
use crate::infra::lsp_install;
use crate::infra::lsp_proc;
use crate::paths::AppPaths;
use crate::state::AppState;

const LSP_SHUTDOWN_TIMEOUT_MS: u64 = 2_000;
/// Poll interval [`wait_for_process_exit`] sleeps between `LspProcHandle::is_exited` checks while
/// waiting (bounded by `LSP_SHUTDOWN_TIMEOUT_MS`) for a language server to exit during shutdown.
const LSP_SHUTDOWN_POLL_INTERVAL_MS: u64 = 50;
const LSP_RESTART_BACKOFF_BASE_MS: u64 = 500;
/// How long a crash-triggered respawn must keep running, with no further crash, before
/// [`handle_process_exit`] resets `restart_count` back to zero. Without this, `restart_count` only
/// ever went up (the sole reset was `lsp_restart`'s explicit manual restart) — a language server
/// that crashed 3 times over the course of a multi-day session, running healthily for hours between
/// each crash, would hit `RESTART_BACKOFF_LIMIT` on its next unrelated crash and stop auto-restarting
/// permanently, even though none of those crashes were part of an actual crash loop. 30 seconds of
/// uninterrupted uptime is long enough to distinguish "recovered" from "still crash-looping" (the
/// backoff between attempts is already well under this at every `restarts` value up to the limit).
const LSP_RESTART_HEALTHY_RESET_MS: u64 = 30_000;

struct SessionEntry {
    project_id: ProjectId,
    server_id: LspServerId,
    root: String,
    spec: LanguageServerSpec,
    proc: Mutex<Option<Arc<lsp_proc::LspProcHandle>>>,
    /// Every window/client currently subscribed to this session's server messages, keyed by the
    /// `owner` label the caller passed `lsp_spawn` (`getCurrentWindow().label` on the frontend —
    /// `main`/`editor-<n>`, or the remote client's fixed `"remote"` label). `find_reusable_entry`
    /// only offers a session for reuse to an owner already present in this map (or, for the
    /// creating owner, the one it seeds on the fresh-entry path below) — so two *different* windows
    /// editing the same project can never end up sharing one JSON-RPC connection: each window's LSP
    /// client is an independent JS realm with its own request-id counter and its own `initialize`
    /// handshake it unconditionally performs on acquiring a session, so two windows sharing one
    /// connection would send `initialize` twice (the language server rejects the second with an
    /// error, leaving that window's client capless forever) and could mint colliding request ids
    /// that resolve the wrong window's pending request. Reuse *within* the same window (re-spawning
    /// while already a member, or two tabs of the same project/server) is unaffected — that's the
    /// scenario `shares_sessions` was designed for and it predates Wave I. `lsp_stop` removes
    /// exactly its caller's `owner` entry before deciding whether the whole session is torn down, so
    /// a still-live owner is never left broadcasting into a channel its window no longer reads from
    /// — the gap Wave I's original `Vec`-based design left (`broadcast_message`'s send-failure
    /// pruning alone only catches a *closed* window, not one that simply released this session while
    /// staying open). See `docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md` §2.2.
    ///
    /// **Invariant (R7#6, decided 2026-08-19): this map is structurally always exactly one entry.**
    /// `find_reusable_entry`'s `channels.contains_key(owner)` gate means a session is only ever
    /// offered for reuse to the *one* owner already present in it — no code path ever inserts a
    /// *second, different* owner into an existing entry's `channels`. A brand-new owner for the same
    /// project/server instead falls through to `lsp_spawn`'s fresh-entry path and gets its own new
    /// `SessionEntry` (with its own single-owner `channels`), never joining this one. Multi-window
    /// subscription to one shared JSON-RPC connection was considered and rejected for this reason
    /// (see the paragraph above) — this map's shape is intentional, not a placeholder for a
    /// multi-owner map that was never finished. `docs/ipc-contract.md`'s prose describing multi-window
    /// subscription for LSP sessions is stale and is corrected separately (Phase D of the T1
    /// registry-cleanup contract); this doc comment is the authority in the meantime. Do not change
    /// `channels`' shape (e.g. to `Vec<Channel<String>>` for fan-out) without first re-deciding this
    /// invariant — it would require redesigning the LSP client to multiplex one connection across
    /// independent JS realms, which is out of scope for anything this map alone can fix.
    channels: Mutex<HashMap<String, Channel<String>>>,
    status: Mutex<LspSessionStatus>,
    last_error: Mutex<Option<String>>,
    restart_count: AtomicU32,
    /// Bumped once per successful *automatic* crash-restart respawn (never by `lsp_spawn`'s or
    /// `lsp_restart`'s own respawn — see [`handle_process_exit`]'s doc comment for why only the
    /// silent, unattended path needs this signal). The renderer watches
    /// `LspSessionStatusChanged.generation`: an increase paired with `status: Crashed` means the
    /// process behind this `session_id` was silently replaced — the renderer must discard its old LSP
    /// client state and re-run `initialize` over `lsp_send`, then call
    /// [`lsp_confirm_reinitialize`] with the same generation to let `status` honestly flip back to
    /// `Running` (R7#1 — the root fix superseding T0 #24's "stay `Crashed`, tell the user to restart
    /// manually" mitigation).
    generation: AtomicU32,
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

    /// Live language-server PIDs for system-usage attribution, keyed by owning
    /// project and labeled with the server's display name (`LanguageServerSpec.name`).
    pub fn server_pids(&self) -> Vec<(ProjectId, String, u32)> {
        self.0
            .lock()
            .values()
            .filter_map(|entry| {
                let pid = entry.proc.lock().as_ref().and_then(|proc| proc.pid())?;
                Some((entry.project_id.clone(), entry.spec.name.clone(), pid))
            })
            .collect()
    }
}

#[derive(Default)]
pub struct LspInstallStore(Mutex<HashMap<LspServerId, Arc<AtomicBool>>>);

impl LspInstallStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new install for `server_id`, returning `None` when one is already in
    /// progress. Reentrant `begin` calls must be rejected — otherwise a second install would
    /// silently overwrite the first's cancel token (making it uncancellable) and its `finish`
    /// could race-remove the second install's still-active entry.
    fn begin(&self, server_id: &LspServerId) -> Option<Arc<AtomicBool>> {
        let mut store = self.0.lock();
        if store.contains_key(server_id) {
            return None;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        store.insert(server_id.clone(), cancel.clone());
        Some(cancel)
    }

    fn finish(&self, server_id: &LspServerId, cancel: &Arc<AtomicBool>) {
        let mut store = self.0.lock();
        if store.get(server_id).is_some_and(|existing| Arc::ptr_eq(existing, cancel)) {
            store.remove(server_id);
        }
    }
}

/// Guarantees [`LspInstallStore::finish`] runs even when [`lsp_install`]'s awaited
/// `run_download_install`/`run_toolchain_install` future never returns normally — a panic
/// unwinding through it, or the command's own task being dropped mid-poll (app exit, or whatever
/// cancels an in-flight Tauri IPC call) — rather than only on the two ordinary `Ok`/`Err` exits a
/// plain post-`.await` call to `finish` would cover. Without this, `server_id` stays permanently
/// wedged as "install in progress" in [`LspInstallStore`]: [`LspInstallStore::begin`] rejects every
/// later `lsp_install` for it, and the cancel token that would have let [`lsp_install_cancel`] stop
/// it is unreachable once the future that held it is gone. Mirrors `architecture.md` §6.3's
/// "Drop 구현 + 명시적 shutdown 경로 이중화" — the explicit call this guard replaces was the single
/// non-Drop layer; this restores the second one.
struct LspInstallGuard<'a> {
    store: &'a LspInstallStore,
    server_id: &'a LspServerId,
    cancel: Arc<AtomicBool>,
}

impl Drop for LspInstallGuard<'_> {
    fn drop(&mut self) {
        self.store.finish(self.server_id, &self.cancel);
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

/// Excludes any entry currently mid-shutdown (`stopping == true`) from reuse. `lsp_stop`'s
/// full-teardown path unlinks the entry from [`LspStore`] before its unguarded
/// [`shutdown_entry`] call, so it never reaches this scan in the first place — but
/// `lsp_restart` deliberately leaves the entry linked (it reuses `session_id` for the
/// respawned process, see its own doc comment) while its own unguarded `shutdown_entry` call
/// is in flight. Without this check, a concurrent `lsp_spawn` for the same
/// project/server/owner could hand that mid-restart entry out as "reusable", wire a fresh
/// channel into it, and send `initialize` against whatever process happens to be installed on
/// `entry.proc` at that instant — the dying pre-restart process, or nothing — instead of the
/// freshly spawned one `lsp_restart` installs once it re-acquires the guard.
fn find_reusable_entry(
    store: &LspStore,
    project_id: &ProjectId,
    server_id: &LspServerId,
    owner: &str,
) -> Option<(String, Arc<SessionEntry>)> {
    store
        .0
        .lock()
        .iter()
        .find(|(_, entry)| {
            &entry.project_id == project_id
                && &entry.server_id == server_id
                && entry.spec.shares_sessions
                && !entry.stopping.load(Ordering::SeqCst)
                && entry.channels.lock().contains_key(owner)
        })
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
        generation: entry.generation.load(Ordering::SeqCst),
    }
    .emit(app);
}

fn managed_dir_for(paths: &AppPaths, server_id: &LspServerId) -> Option<PathBuf> {
    lsp_install::latest_installed_version(&paths.lsp_dir(), server_id.as_str())
        .map(|version| paths.lsp_server_version_dir(server_id.as_str(), &version))
}

fn resolved_args(spec: &LanguageServerSpec, root: &str, managed_dir: Option<&std::path::Path>) -> Vec<String> {
    let mut vars = vec![("workspaceDir", root.to_string())];
    if let Some(managed_dir) = managed_dir {
        vars.push(("serverDir", managed_dir.to_string_lossy().to_string()));
    }
    lsp_install::substitute_template_args(spec.command.args(), &vars)
}

fn spawn_process(app: &AppHandle, session_id: String, spec: LanguageServerSpec, root: String) -> AppResult<Arc<lsp_proc::LspProcHandle>> {
    let paths = &app.state::<AppState>().paths;
    let managed_dir = managed_dir_for(paths, &spec.id);
    let managed_relative_path = spec
        .install
        .download
        .as_ref()
        .and_then(|download| download.bin_path_in_archive.as_deref());
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let resolved = service::resolve_spec_command(
        &spec,
        Some(std::path::Path::new(&root)),
        managed_dir.as_deref(),
        managed_relative_path,
        &path_var,
    )
    .ok_or_else(|| AppError::NotFound(format!("language server executable not found: {}", spec.command.bin())))?;

    let args = resolved_args(&spec, &root, managed_dir.as_deref());
    if args.iter().any(|arg| arg.contains('{') && arg.contains('}')) {
        return Err(AppError::Internal(format!(
            "{} 실행 인자에 치환되지 않은 템플릿이 남아 있습니다 (관리 디렉토리 미설치 가능성)",
            spec.id
        )));
    }

    let config = lsp_proc::LspProcConfig {
        command: resolved.to_string_lossy().to_string(),
        args,
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
            broadcast_message(&entry.channels, &message);
        },
        move |code| handle_process_exit(&exit_app, exit_session_id, code),
    )?;

    Ok(Arc::new(handle))
}

/// Sends `message` to every subscriber, keeping only the ones that accept it — a subscriber whose
/// `send` fails (e.g. its window has closed) is dropped rather than aborting the whole broadcast
/// or being retried. Extracted from the process message handler above so the broadcast/prune
/// behavior itself can be unit tested without spawning a real language server process.
fn broadcast_message(channels: &Mutex<HashMap<String, Channel<String>>>, message: &str) {
    channels.lock().retain(|_, channel| channel.send(message.to_string()).is_ok());
}

/// Reacts to an unexpected language server process exit (`Some`/`None` exit code from a crash, not
/// a `lsp_stop`-initiated shutdown, which sets `stopping` first and short-circuits above). Retries
/// with backoff up to [`RESTART_BACKOFF_LIMIT`] times, then gives up and reports [`LspSessionStatus::Crashed`].
///
/// A successful respawn (`spawn_process` returning `Ok`) deliberately still reports
/// [`LspSessionStatus::Crashed`], not `Running` — unlike [`lsp_spawn`]/[`lsp_restart`], which *do*
/// report `Running` right after their own `spawn_process` call. The difference is who drives the LSP
/// `initialize` handshake: `lsp_spawn`/`lsp_restart` are directly awaited by the frontend's own
/// action, which sends `initialize` as the next step of that same client-side flow once the command
/// resolves. This restart runs entirely in the background with no frontend caller waiting on it —
/// the renderer's existing LSP client believes its old, already-initialized connection is still
/// good and has no trigger to redo `initialize` against the fresh process, so declaring `Running`
/// here would be a false report: requests sent to an unhandshaked server go nowhere.
///
/// T0 #24's mitigation stopped here (report `Crashed` forever, tell the user to restart manually).
/// R7#1's root fix (T1-D) closes the loop instead of just naming it: on a successful respawn,
/// `entry.generation` is bumped *before* `set_status` emits `LspSessionStatusChanged`, so that event
/// carries both the new `generation` and `status: Crashed` together. The renderer treats a `generation`
/// increase as "this session's process was silently replaced" — it discards its old client state,
/// re-runs `initialize` over `lsp_send` against the same `session_id`, and on success calls
/// [`lsp_confirm_reinitialize`] with that same generation. Only that confirmation (matched against
/// the *current* generation, so a stale confirmation racing a second crash is ignored — see its own
/// doc comment) flips `status` to `Running`; nothing in this function ever reports `Running` directly,
/// preserving the T0 #24 invariant that a silent respawn is never reported healthy before the
/// renderer has actually re-handshaked.
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
                *entry.proc.lock() = Some(proc.clone());
                entry.generation.fetch_add(1, Ordering::SeqCst);
                set_status(
                    &restart_app,
                    &restart_session_id,
                    &entry,
                    LspSessionStatus::Crashed,
                    Some("서버 프로세스가 자동으로 재시작됐습니다. 초기화 핸드셰이크가 다시 완료될 때까지 기다려주세요.".to_string()),
                );

                let healthy_reset_entry = entry.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(LSP_RESTART_HEALTHY_RESET_MS)).await;
                    if confirms_healthy_restart(&healthy_reset_entry.proc, &proc) {
                        healthy_reset_entry.restart_count.store(0, Ordering::SeqCst);
                    }
                });
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

/// True only when `respawned` is both still alive and still the process installed on `entry_proc` —
/// the condition [`handle_process_exit`]'s delayed health-check task requires before crediting a
/// crash-triggered respawn with [`LSP_RESTART_HEALTHY_RESET_MS`] of uninterrupted uptime and zeroing
/// `restart_count`. Either half failing means this exact respawn's health window doesn't count: a
/// later crash may have already swapped in a *different* process (`entry_proc` no longer points at
/// `respawned`), or `respawned` itself may have crashed again before the window elapsed (its own
/// `is_exited` flip) — crediting either case would silently extend the crash-loop budget instead of
/// only resetting it for a respawn that actually proved itself healthy.
fn confirms_healthy_restart(entry_proc: &Mutex<Option<Arc<lsp_proc::LspProcHandle>>>, respawned: &Arc<lsp_proc::LspProcHandle>) -> bool {
    !respawned.is_exited() && entry_proc.lock().as_ref().is_some_and(|current| Arc::ptr_eq(current, respawned))
}

/// Polls `proc.is_exited()` at [`LSP_SHUTDOWN_POLL_INTERVAL_MS`] intervals until the process has
/// exited or `timeout_ms` has elapsed, whichever comes first. Replaces a blind
/// `tokio::time::sleep(timeout_ms)` in [`shutdown_entry`] with an early return the moment the
/// language server actually exits — which is normally well under `timeout_ms` — while still
/// bounding the wait for a server that never responds to `shutdown`/`exit`.
async fn wait_for_process_exit(proc: &lsp_proc::LspProcHandle, timeout_ms: u64) {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
    while !proc.is_exited() && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(tokio::time::Duration::from_millis(LSP_SHUTDOWN_POLL_INTERVAL_MS)).await;
    }
}

/// Deliberately **not** guarded by `AppState::begin_mutation` — both callers ([`lsp_stop`],
/// [`lsp_restart`]) drop their guard before awaiting this, the same rationale `lsp_send` already
/// documents (holding the single global mutation lock across a multi-second, network/process-bound
/// wait would queue every unrelated layout/file/`lsp_spawn` command behind it, which is the exact
/// "file open blocks on LSP teardown" bug this restructuring fixes). By the time this runs, the
/// caller has already unlinked the entry from `LspStore` (for `lsp_stop`'s full-teardown path) or
/// otherwise made sure a concurrent `find_reusable_entry`/`find_entry` can't hand this
/// mid-shutdown session out to a new caller, so nothing outside this `Arc<SessionEntry>` needs the
/// guard for the duration of the wait. Sends the LSP `shutdown` request, waits (bounded) for the
/// process to exit, sends `exit`, waits (bounded) again, then unconditionally kills — the same
/// shutdown sequence as before, just with [`wait_for_process_exit`]'s early-return polling in
/// place of a blind sleep.
async fn shutdown_entry(app: &AppHandle, entry: &SessionEntry, session_id: &str) {
    entry.stopping.store(true, Ordering::SeqCst);

    let proc = entry.proc.lock().clone();
    if let Some(proc) = proc {
        let shutdown_request = serde_json::json!({ "jsonrpc": "2.0", "id": "taide-shutdown", "method": "shutdown" }).to_string();
        let _ = proc.write_message(&shutdown_request).await;
        wait_for_process_exit(&proc, LSP_SHUTDOWN_TIMEOUT_MS).await;

        let exit_notification = serde_json::json!({ "jsonrpc": "2.0", "method": "exit" }).to_string();
        let _ = proc.write_message(&exit_notification).await;
        wait_for_process_exit(&proc, LSP_SHUTDOWN_TIMEOUT_MS).await;

        proc.kill();
    }

    set_status(app, session_id, entry, LspSessionStatus::Stopped, None);
}

/// `request.owner` identifies the calling window (`getCurrentWindow().label` on the frontend —
/// `main`, `editor-<n>`, or the remote client's fixed `"remote"` label) so [`find_reusable_entry`]
/// only reuses a session within the same window. See the `channels` field doc on [`SessionEntry`]
/// for why. `request` bundles `project_id`/`server_id`/`root`/`owner` into one struct (mirroring
/// `pty_spawn`'s `opts`) purely to stay under `clippy::too_many_arguments`.
#[tauri::command]
#[specta::specta]
pub async fn lsp_spawn(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, LspStore>,
    request: LspSpawnRequest,
    on_message: Channel<String>,
) -> AppResult<String> {
    let LspSpawnRequest {
        project_id,
        server_id,
        root,
        owner,
    } = request;

    let _guard = state.begin_mutation().await;
    ensure_project_open(&state, &project_id)?;

    let spec = manifest::find_spec(server_id.as_str())
        .ok_or_else(|| AppError::InvalidArgument(format!("unknown language server: {server_id}")))?;

    if let Some((existing_id, existing_entry)) = find_reusable_entry(&store, &project_id, &server_id, &owner) {
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

            // Subscribe this caller's channel to the shared session instead of discarding it — see
            // the `channels` field doc on `SessionEntry` for why every owner needs its own slot
            // rather than only the first one ever getting messages, and why keying by `owner`
            // (replacing any previous entry of its own) rather than appending is correct.
            existing_entry.channels.lock().insert(owner, on_message);

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
        server_id: server_id.clone(),
        root: root.clone(),
        spec: spec.clone(),
        proc: Mutex::new(None),
        channels: Mutex::new(HashMap::from([(owner, on_message)])),
        status: Mutex::new(LspSessionStatus::Starting),
        last_error: Mutex::new(None),
        restart_count: AtomicU32::new(0),
        generation: AtomicU32::new(0),
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

/// Not guarded by `AppState::begin_mutation` — this command never touches `AppState`, and
/// per-session stdin writes are already serialized by `LspProcHandle`'s own
/// `tokio::sync::Mutex` (`infra::lsp_proc::LspProcHandle::write_message`). Gating this behind
/// the global mutation lock would only make LSP requests (which fire far more often than
/// saves/git operations) queue behind unrelated mutating commands for no correctness benefit.
#[tauri::command]
#[specta::specta]
pub async fn lsp_send(store: State<'_, LspStore>, session_id: String, message: String) -> AppResult<()> {
    let entry = find_entry(&store, &session_id)?;
    let proc = entry
        .proc
        .lock()
        .clone()
        .ok_or_else(|| AppError::Internal("language server not ready".to_string()))?;
    proc.write_message(&message).await
}

/// `owner` (`getCurrentWindow().label`, same value the caller originally passed to `lsp_spawn`)
/// removes exactly that caller's subscriber entry from `entry.channels` — see the `channels` field
/// doc on [`SessionEntry`] for why this can't be left to `broadcast_message`'s send-failure pruning
/// alone. Root refcounting (`root`) then decides, independently, whether the whole session (process
/// included) gets torn down — a still-live owner keeps receiving messages from the shared session
/// even after some other owner's root is removed from it. **This owner-scoping is preserved
/// unchanged** by the guard restructuring below: the "still has remaining roots" branch still
/// returns early, under the guard, without ever reaching teardown.
///
/// The guard (`AppState::begin_mutation`) is held only for the synchronous bookkeeping above and,
/// on the full-teardown path, for unlinking the entry from [`LspStore`] — `store.0.lock().remove`
/// runs *before* the guard is dropped, specifically so a concurrent `lsp_spawn`'s
/// `find_reusable_entry` or another `lsp_stop`/`lsp_send`'s `find_entry` can never observe this
/// session while [`shutdown_entry`] is mid-flight. [`shutdown_entry`] itself then runs unguarded
/// (see its own doc comment for why that's safe) — this is what stops LSP teardown from queuing
/// every other mutating command (`layout_*`, `file_save`, `lsp_spawn`) behind it for the whole
/// shutdown sequence, the file-open-blocks-on-LSP-teardown bug this restructuring fixes.
///
/// Two overlapping full-teardown calls for the same `session_id` (double-invoked `lsp_stop`, e.g.
/// from a React effect cleanup racing a manual close) are safe by construction: whichever call's
/// guarded section runs first removes the entry from the store; the second call's own
/// `find_entry` then fails fast with `NotFound` instead of running a second redundant
/// `shutdown_entry` — strictly better than the previous behavior, where both calls could hold the
/// entry live long enough to both reach `shutdown_entry` and both harmlessly re-send
/// shutdown/exit/kill to the same (possibly already-dead) process. A concurrent `lsp_spawn` for the
/// same project/server/owner racing this teardown *can* still lose to it (spawning a fresh session
/// while the old one's process is still being killed in the background) — but that window is now
/// bounded by however long the language server actually takes to exit (typically well under
/// `LSP_SHUTDOWN_TIMEOUT_MS`, per [`wait_for_process_exit`]'s polling) rather than a blind 4-second
/// hold, and the old process is guaranteed to be killed regardless once its `shutdown_entry` call
/// completes.
#[tauri::command]
#[specta::specta]
pub async fn lsp_stop(
    app: AppHandle,
    state: State<'_, AppState>,
    store: State<'_, LspStore>,
    session_id: String,
    root: Option<String>,
    owner: String,
) -> AppResult<()> {
    let entry = {
        let _guard = state.begin_mutation().await;
        let entry = find_entry(&store, &session_id)?;
        entry.channels.lock().remove(&owner);

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

        store.0.lock().remove(&session_id);
        entry
    };

    shutdown_entry(&app, &entry, &session_id).await;
    Ok(())
}

/// Guard-restructuring mirrors [`lsp_stop`]'s: the guard covers only the synchronous `find_entry`
/// lookup, is dropped before the unguarded [`shutdown_entry`] await, then re-acquired for the
/// respawn bookkeeping. Unlike `lsp_stop`'s full-teardown path, the entry is **not** unlinked from
/// [`LspStore`] here — `session_id` is reused for the restarted process, so `find_entry` continues
/// to resolve it throughout *this* call. That does leave a narrow unguarded window where a
/// concurrent `lsp_stop` for this same `session_id` can also observe the entry (still in the
/// store) and run its own full teardown — including unlinking it from `LspStore` — while this
/// call's `shutdown_entry` is also in flight against the identical `Arc<SessionEntry>` (harmless:
/// both converge on the same stopping/kill/`Stopped` end state). The respawn below re-checks the
/// store for `session_id` before installing the freshly spawned process specifically to catch that
/// case: if `lsp_stop` won the race and already removed the entry, spawning anyway would leak an
/// unreachable process no `session_id` could ever `lsp_stop`/`lsp_send` again (the old code
/// couldn't hit this — `begin_mutation`'s single guard spanned each call's *entire* body, so
/// `lsp_stop` and `lsp_restart` could never interleave at all).
#[tauri::command]
#[specta::specta]
pub async fn lsp_restart(app: AppHandle, state: State<'_, AppState>, store: State<'_, LspStore>, session_id: String) -> AppResult<()> {
    let entry = {
        let _guard = state.begin_mutation().await;
        find_entry(&store, &session_id)?
    };

    shutdown_entry(&app, &entry, &session_id).await;

    let _guard = state.begin_mutation().await;
    if !store.0.lock().contains_key(&session_id) {
        return Err(AppError::NotFound(format!("lsp session not found: {session_id}")));
    }

    entry.stopping.store(false, Ordering::SeqCst);
    entry.restart_count.store(0, Ordering::SeqCst);
    set_status(&app, &session_id, &entry, LspSessionStatus::Starting, None);

    let proc = spawn_process(&app, session_id.clone(), entry.spec.clone(), entry.root.clone())?;
    *entry.proc.lock() = Some(proc);
    set_status(&app, &session_id, &entry, LspSessionStatus::Running, None);

    Ok(())
}

/// True only when `generation` matches `entry`'s *current* generation — the guard
/// [`lsp_confirm_reinitialize`] applies before honoring a renderer's "I finished re-handshaking"
/// report. Without it, a confirmation for a generation that a second crash+auto-restart has since
/// superseded would incorrectly report the newer, still-unhandshaked process as `Running` — the
/// exact race [`handle_process_exit`]'s doc comment on the `generation` field warns about.
fn confirm_reinitialize(entry: &SessionEntry, generation: u32) -> bool {
    entry.generation.load(Ordering::SeqCst) == generation
}

/// Called by the renderer once it has finished re-running `initialize` against a session whose
/// [`LspSessionStatusChanged`] event reported a bumped `generation` (see the `generation` field doc
/// on [`SessionEntry`]) — the counterpart to [`handle_process_exit`]'s auto-restart path that closes
/// the loop T0 #24 left open (`Crashed` reported forever after a silent respawn, requiring a manual
/// restart). A confirmation for a generation the session has since moved past (see
/// [`confirm_reinitialize`]) is silently ignored rather than flipping a still-unhandshaked process to
/// `Running`.
#[tauri::command]
#[specta::specta]
pub async fn lsp_confirm_reinitialize(app: AppHandle, store: State<'_, LspStore>, session_id: String, generation: u32) -> AppResult<()> {
    let entry = find_entry(&store, &session_id)?;
    if confirm_reinitialize(&entry, generation) {
        set_status(&app, &session_id, &entry, LspSessionStatus::Running, None);
    }
    Ok(())
}

/// The `last_error` text [`lsp_report_reinitialize_failure`] applies — an honest terminal outcome
/// ("the auto-restart happened, but re-handshaking it never worked") in place of `handle_process_exit`'s
/// optimistic in-progress wording ("초기화 핸드셰이크가 다시 완료될 때까지 기다려주세요"), which would
/// otherwise sit unchanged forever once the renderer gives up retrying.
const REINITIALIZE_FAILURE_MESSAGE: &str =
    "초기화 핸드셰이크 재시도를 모두 소진해 서버를 재연결하지 못했습니다. 수동으로 다시 시작해주세요.";

/// [`lsp_confirm_reinitialize`]'s failure counterpart (§1.3(4),
/// `docs/acknowledge/2026-08-19-xa-wiring-cleanup-contract.md`) — called by the renderer once it has
/// exhausted its own retry budget re-running `initialize` against a session whose
/// [`LspSessionStatusChanged`] event reported a bumped `generation`, instead of ever succeeding. Without
/// this, a session whose re-handshake never lands sits forever on `handle_process_exit`'s own
/// optimistic "재시작됐습니다, 기다려주세요" `last_error` text — a status-bar poll of `lsp_sessions`/
/// `LspSessionInfo` would keep reporting "waiting" indefinitely instead of the honest "failed, restart
/// manually" this command lets it settle on. Applies the exact same generation guard as
/// [`lsp_confirm_reinitialize`] — a failure report for a generation the session has since moved past
/// (a second crash+auto-restart already superseded it) is silently ignored, the same race both
/// commands exist to resolve honestly rather than clobber a newer in-flight attempt's status. Allowed
/// remotely (T1-K): a remote mirror must be able to settle its own session's failed reinitialize just
/// as the desktop can, and the generation-mismatch-is-ignored guard already defends against a stale or
/// spoofed report reviving/clobbering a session it no longer describes.
#[tauri::command]
#[specta::specta]
pub async fn lsp_report_reinitialize_failure(
    app: AppHandle,
    store: State<'_, LspStore>,
    session_id: String,
    generation: u32,
) -> AppResult<()> {
    let entry = find_entry(&store, &session_id)?;
    if confirm_reinitialize(&entry, generation) {
        set_status(
            &app,
            &session_id,
            &entry,
            LspSessionStatus::Crashed,
            Some(REINITIALIZE_FAILURE_MESSAGE.to_string()),
        );
    }
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
            server_id: entry.server_id.clone(),
            root: entry.root.clone(),
            status: *entry.status.lock(),
            last_error: entry.last_error.lock().clone(),
            generation: entry.generation.load(Ordering::SeqCst),
        })
        .collect())
}

#[tauri::command]
#[specta::specta]
pub async fn lsp_detect_servers(state: State<'_, AppState>) -> AppResult<Vec<LspServerDetection>> {
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    Ok(service::detect_servers(&state.paths.lsp_dir(), &path_var))
}

#[tauri::command]
#[specta::specta]
pub async fn lsp_resolve_root(server_id: LspServerId, file_path: String) -> AppResult<Option<String>> {
    let spec = manifest::find_spec(server_id.as_str())
        .ok_or_else(|| AppError::InvalidArgument(format!("unknown language server: {server_id}")))?;
    Ok(service::find_root(&spec, std::path::Path::new(&file_path)).map(|root| root.to_string_lossy().to_string()))
}

fn emit_install_progress(
    app: &AppHandle,
    server_id: &LspServerId,
    phase: LspInstallPhase,
    received_bytes: u64,
    total_bytes: Option<u64>,
    message: Option<String>,
) {
    let _ = LspInstallProgress {
        server_id: server_id.clone(),
        phase,
        received_bytes: received_bytes as f64,
        total_bytes: total_bytes.map(|value| value as f64),
        message,
    }
    .emit(app);
}

async fn run_download_install(app: &AppHandle, paths: &AppPaths, spec: &LanguageServerSpec, cancel: Arc<AtomicBool>) -> AppResult<()> {
    let download = spec
        .install
        .download
        .as_ref()
        .ok_or_else(|| AppError::InvalidArgument(format!("{} 의 다운로드 정보가 아직 설정되지 않았습니다", spec.id)))?;

    let platform = lsp_install::platform_key();
    let url = download
        .urls
        .get(&platform)
        .ok_or_else(|| AppError::InvalidArgument(format!("{} 는 현재 플랫폼({platform})을 지원하지 않습니다", spec.id)))?;
    let expected_sha256 = download
        .sha256
        .get(&platform)
        .cloned()
        .flatten()
        .ok_or_else(|| AppError::InvalidArgument(format!("{} 의 체크섬이 아직 게시되지 않아 설치할 수 없습니다", spec.id)))?;

    emit_install_progress(app, &spec.id, LspInstallPhase::Downloading, 0, None, None);

    let client = outbound_http_client(HttpClientProfile::Download);
    let server_id = spec.id.clone();
    let progress_app = app.clone();
    let download_dest = lsp_install::temp_download_path(&paths.lsp_dir(), spec.id.as_str());
    let downloaded = match lsp_install::download_to_file(&client, url, &download_dest, &cancel, move |update| {
        emit_install_progress(
            &progress_app,
            &server_id,
            LspInstallPhase::Downloading,
            update.received_bytes,
            update.total_bytes,
            None,
        );
    })
    .await
    {
        Ok(downloaded) => downloaded,
        Err(error) => {
            std::fs::remove_file(&download_dest).ok();
            emit_install_progress(app, &spec.id, LspInstallPhase::Failed, 0, None, Some(error.to_string()));
            return Err(error);
        }
    };

    emit_install_progress(
        app,
        &spec.id,
        LspInstallPhase::Verifying,
        downloaded.total_bytes,
        Some(downloaded.total_bytes),
        None,
    );
    if !lsp_install::hashes_match(&downloaded.sha256, &expected_sha256) {
        std::fs::remove_file(&downloaded.path).ok();
        emit_install_progress(
            app,
            &spec.id,
            LspInstallPhase::Failed,
            0,
            None,
            Some("체크섬이 일치하지 않습니다".to_string()),
        );
        return Err(AppError::Internal(format!("{} 다운로드의 체크섬이 일치하지 않습니다", spec.id)));
    }

    emit_install_progress(
        app,
        &spec.id,
        LspInstallPhase::Extracting,
        downloaded.total_bytes,
        Some(downloaded.total_bytes),
        None,
    );
    let temp_dir = lsp_install::temp_install_dir(&paths.lsp_dir(), spec.id.as_str());
    let archive_kind = download.archive;
    let bin_path_in_archive = download.bin_path_in_archive.clone();
    let source_path = downloaded.path.clone();
    let extract_dir = temp_dir.clone();

    // Archive extraction is blocking filesystem/CPU work (unzip, gzip decode); running it
    // directly in this async fn would monopolize a tokio worker thread while decompressing
    // large archives (e.g. the ~387MB kotlin-lsp payload) and stall other IPC handlers.
    let extract_result = tokio::task::spawn_blocking(move || match archive_kind {
        super::types::LspArchiveKind::TarGz => lsp_install::extract_tar_gz(&source_path, &extract_dir),
        super::types::LspArchiveKind::TarXz => lsp_install::extract_tar_xz(&source_path, &extract_dir),
        super::types::LspArchiveKind::Zip => lsp_install::extract_zip(&source_path, &extract_dir),
        super::types::LspArchiveKind::Binary => {
            lsp_install::write_binary_from_file(&source_path, &extract_dir, bin_path_in_archive.as_deref())
        }
        super::types::LspArchiveKind::Gz => {
            lsp_install::write_gz_binary_from_file(&source_path, &extract_dir, bin_path_in_archive.as_deref())
        }
    })
    .await
    .map_err(|join_error| AppError::Internal(format!("압축 해제 작업 실패: {join_error}")))
    .and_then(|result| result);

    std::fs::remove_file(&downloaded.path).ok();

    if let Err(error) = extract_result {
        std::fs::remove_dir_all(&temp_dir).ok();
        emit_install_progress(app, &spec.id, LspInstallPhase::Failed, 0, None, Some(error.to_string()));
        return Err(error);
    }

    let final_dir = paths.lsp_server_version_dir(spec.id.as_str(), &download.version);
    if let Err(error) = lsp_install::atomic_install(&temp_dir, &final_dir) {
        std::fs::remove_dir_all(&temp_dir).ok();
        emit_install_progress(app, &spec.id, LspInstallPhase::Failed, 0, None, Some(error.to_string()));
        return Err(error);
    }

    emit_install_progress(
        app,
        &spec.id,
        LspInstallPhase::Done,
        downloaded.total_bytes,
        Some(downloaded.total_bytes),
        None,
    );
    Ok(())
}

const TOOLCHAIN_POLL_INTERVAL_MS: u64 = 100;
const TOOLCHAIN_OUTPUT_TAIL_LINES: usize = 20;

#[cfg(unix)]
fn kill_toolchain_process_group(pid: u32) {
    // start_kill() only signals the direct child; toolchain installers (go/gem/coursier/ghcup)
    // often spawn a compiler or sub-installer, so on cancel we must signal the whole process
    // group (negative pid) to avoid leaving orphaned grandchildren running.
    let _ = std::process::Command::new("kill").arg("-TERM").arg(format!("-{pid}")).status();
}

fn capture_output_tail(reader: impl tokio::io::AsyncRead + Unpin + Send + 'static) -> tokio::sync::oneshot::Receiver<Vec<String>> {
    use tokio::io::{AsyncBufReadExt, BufReader};

    let (sender, receiver) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        let mut tail = Vec::new();
        while let Ok(Some(line)) = lines.next_line().await {
            tail.push(line);
            if tail.len() > TOOLCHAIN_OUTPUT_TAIL_LINES {
                tail.remove(0);
            }
        }
        let _ = sender.send(tail);
    });
    receiver
}

async fn run_toolchain_install(app: &AppHandle, spec: &LanguageServerSpec, cancel: Arc<AtomicBool>) -> AppResult<()> {
    let toolchain = spec
        .install
        .toolchain
        .as_ref()
        .ok_or_else(|| AppError::InvalidArgument(format!("{} 의 툴체인 설치 정보가 아직 설정되지 않았습니다", spec.id)))?;

    let binary = service::toolchain_binary(toolchain.tool);
    if service::find_in_path(binary).is_none() {
        return Err(AppError::NotFound(format!("{binary} 툴체인을 찾을 수 없습니다")));
    }

    emit_install_progress(
        app,
        &spec.id,
        LspInstallPhase::Downloading,
        0,
        None,
        Some(format!("{binary} 로 설치 중")),
    );

    let mut command = tokio::process::Command::new(binary);
    command.args(&toolchain.install_args);
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    #[cfg(unix)]
    command.process_group(0);

    let mut child = command
        .spawn()
        .map_err(|error| AppError::Internal(format!("{binary} 실행 실패: {error}")))?;
    let child_pid = child.id();

    let stdout_tail = child.stdout.take().map(capture_output_tail);
    let stderr_tail = child.stderr.take().map(capture_output_tail);

    loop {
        if cancel.load(Ordering::SeqCst) {
            #[cfg(unix)]
            if let Some(pid) = child_pid {
                kill_toolchain_process_group(pid);
            }
            let _ = child.start_kill();
            let _ = child.wait().await;
            emit_install_progress(
                app,
                &spec.id,
                LspInstallPhase::Failed,
                0,
                None,
                Some("설치가 취소되었습니다".to_string()),
            );
            return Err(AppError::Internal("설치가 취소되었습니다".to_string()));
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    emit_install_progress(app, &spec.id, LspInstallPhase::Done, 0, None, None);
                    return Ok(());
                }

                let mut tail_lines = Vec::new();
                if let Some(receiver) = stderr_tail {
                    tail_lines.extend(receiver.await.unwrap_or_default());
                }
                if let Some(receiver) = stdout_tail {
                    tail_lines.extend(receiver.await.unwrap_or_default());
                }
                let tail = tail_lines.join("\n");

                let message = if tail.is_empty() {
                    format!("{binary} 설치 명령이 실패했습니다 (종료 코드: {:?})", status.code())
                } else {
                    format!("{binary} 설치 명령이 실패했습니다 (종료 코드: {:?}): {tail}", status.code())
                };
                emit_install_progress(app, &spec.id, LspInstallPhase::Failed, 0, None, Some(message.clone()));
                return Err(AppError::Internal(message));
            }
            Ok(None) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(TOOLCHAIN_POLL_INTERVAL_MS)).await;
            }
            Err(error) => {
                emit_install_progress(app, &spec.id, LspInstallPhase::Failed, 0, None, Some(error.to_string()));
                return Err(AppError::from(error));
            }
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn lsp_install(
    app: AppHandle,
    state: State<'_, AppState>,
    install_store: State<'_, LspInstallStore>,
    server_id: LspServerId,
) -> AppResult<()> {
    let spec = manifest::find_spec(server_id.as_str())
        .ok_or_else(|| AppError::InvalidArgument(format!("unknown language server: {server_id}")))?;

    let Some(cancel) = install_store.begin(&server_id) else {
        return Err(AppError::InvalidArgument(format!("{server_id} 설치가 이미 진행 중입니다")));
    };
    let _guard = LspInstallGuard {
        store: install_store.inner(),
        server_id: &server_id,
        cancel: cancel.clone(),
    };

    match spec.install.strategy {
        LspInstallStrategy::Download => run_download_install(&app, &state.paths, &spec, cancel).await,
        LspInstallStrategy::Toolchain => run_toolchain_install(&app, &spec, cancel).await,
        LspInstallStrategy::SdkDetect => Err(AppError::InvalidArgument(format!(
            "{} 는 SDK 감지 전용이라 자동 설치할 수 없습니다",
            spec.id
        ))),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn lsp_install_cancel(install_store: State<'_, LspInstallStore>, server_id: LspServerId) -> AppResult<()> {
    if let Some(cancel) = install_store.0.lock().get(&server_id) {
        cancel.store(true, Ordering::SeqCst);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::lsp::types::{LspCommandSpec, LspInstallSpec, LspRootStrategy};

    #[test]
    fn lspinstallguard는_설치_클로저가_패닉해도_슬롯을_해제한다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("test-server");

        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let cancel = store.begin(&server_id).expect("첫 install은 슬롯을 얻어야 한다");
            let _guard = LspInstallGuard {
                store: &store,
                server_id: &server_id,
                cancel,
            };
            panic!("설치 도중 패닉");
        }));

        assert!(panicked.is_err(), "패닉이 실제로 발생해야 이 테스트가 의미가 있다");
        assert!(
            store.begin(&server_id).is_some(),
            "가드가 패닉 언와인딩 중에도 finish를 호출해 슬롯을 해제해야 한다 — 그러지 않으면 이 server_id는 영원히 \"설치 중\"으로 잠긴다"
        );
    }

    #[test]
    fn lspinstallguard는_정상_종료_시에도_슬롯을_해제한다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("test-server");

        {
            let cancel = store.begin(&server_id).expect("첫 install은 슬롯을 얻어야 한다");
            let _guard = LspInstallGuard {
                store: &store,
                server_id: &server_id,
                cancel,
            };
        }

        assert!(
            store.begin(&server_id).is_some(),
            "정상적으로 스코프를 빠져나가도 슬롯은 해제되어야 한다"
        );
    }

    fn recording_channel(received: Arc<Mutex<Vec<String>>>) -> Channel<String> {
        Channel::new(move |body| {
            let text = match body {
                tauri::ipc::InvokeResponseBody::Json(text) => text,
                tauri::ipc::InvokeResponseBody::Raw(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            };
            received.lock().push(serde_json::from_str(&text).unwrap_or(text));
            Ok(())
        })
    }

    fn failing_channel() -> Channel<String> {
        Channel::new(|_| Err(tauri::Error::AssetNotFound("closed".to_string())))
    }

    fn test_session_entry(project_id: ProjectId, server_id: LspServerId, owner: &str, stopping: bool) -> Arc<SessionEntry> {
        let mut channels = HashMap::new();
        channels.insert(owner.to_string(), failing_channel());
        Arc::new(SessionEntry {
            project_id,
            server_id: server_id.clone(),
            root: "/tmp/project".to_string(),
            spec: LanguageServerSpec {
                id: server_id,
                name: "Test Server".to_string(),
                language_ids: vec!["rust".to_string()],
                shares_sessions: true,
                command: LspCommandSpec::Path {
                    bin: "test-lsp".to_string(),
                    args: vec![],
                },
                root_markers: vec![],
                root_strategy: LspRootStrategy::NearestMarker,
                initialization_options: None,
                install: LspInstallSpec {
                    strategy: LspInstallStrategy::Toolchain,
                    hint: None,
                    download: None,
                    toolchain: None,
                    sdk_detect: None,
                },
            },
            proc: Mutex::new(None),
            channels: Mutex::new(channels),
            status: Mutex::new(LspSessionStatus::Running),
            last_error: Mutex::new(None),
            restart_count: AtomicU32::new(0),
            generation: AtomicU32::new(0),
            stopping: Arc::new(AtomicBool::new(stopping)),
            roots: Mutex::new(vec![("/tmp/project".to_string(), 1)]),
        })
    }

    /// R7#1 회귀: 확인(confirm)이 실제로 반영해야 하는 세대와 일치할 때만 통과해야 한다 —
    /// 그러지 않으면 느리게 도착한 재핸드셰이크 확인이, 그 사이 두 번째 크래시+자동재시작이
    /// 이미 새 세대로 올린 (아직 재핸드셰이크되지 않은) 프로세스를 잘못 `Running`으로
    /// 보고하게 된다.
    #[test]
    fn confirm_reinitialize는_현재_세대와_일치할_때만_true를_반환한다() {
        let entry = test_session_entry(ProjectId::new(), LspServerId::from("test-server"), "owner-a", false);
        entry.generation.store(2, Ordering::SeqCst);

        assert!(confirm_reinitialize(&entry, 2), "현재 세대와 일치하는 확인은 통과해야 한다");
        assert!(
            !confirm_reinitialize(&entry, 1),
            "구세대 확인은 그 사이 새 크래시+재시작이 세대를 올렸을 수 있으므로 무시되어야 한다"
        );
        assert!(
            !confirm_reinitialize(&entry, 3),
            "아직 오지 않은 미래 세대의 확인도 무시되어야 한다"
        );
    }

    /// §1.3(4) 회귀: 실패 확인(`lsp_report_reinitialize_failure`)도 성공 확인(`lsp_confirm_reinitialize`)과
    /// 정확히 같은 세대 가드를 공유한다 — 가드 자체의 통과/거부 조건은 위 테스트가 이미 검증하므로,
    /// 여기서는 두 커맨드가 그 가드에 동일하게 의존한다는 계약을 고정한다. 미래에 이 가드가
    /// 커맨드별로 분리되면 이 테스트가 실패해 그 분리가 실패 확인 경로에도 반영됐는지 드러낸다.
    #[test]
    fn confirm_reinitialize_가드는_성공과_실패_확인_모두에_재사용된다() {
        let entry = test_session_entry(ProjectId::new(), LspServerId::from("test-server"), "owner-a", false);
        entry.generation.store(1, Ordering::SeqCst);

        assert!(
            confirm_reinitialize(&entry, 1),
            "lsp_confirm_reinitialize 경로가 의존하는 통과 조건"
        );
        assert!(
            !confirm_reinitialize(&entry, 0),
            "lsp_report_reinitialize_failure 경로도 구세대 실패 신고를 무시해야 한다"
        );
    }

    #[test]
    fn 새로_생성된_세션엔트리의_세대는_0에서_시작한다() {
        let entry = test_session_entry(ProjectId::new(), LspServerId::from("test-server"), "owner-a", false);
        assert_eq!(entry.generation.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn find_reusable_entry는_종료_중인_세션을_재사용_후보에서_제외한다() {
        let project_id = ProjectId::new();
        let server_id = LspServerId::from("test-server");
        let store = LspStore::new();
        store.0.lock().insert(
            "stopping-session".to_string(),
            test_session_entry(project_id.clone(), server_id.clone(), "owner-a", true),
        );

        assert!(
            find_reusable_entry(&store, &project_id, &server_id, "owner-a").is_none(),
            "lsp_restart 의 비가드 shutdown 구간에서는 stopping 엔트리를 재사용 후보로 내주면 안 된다"
        );
    }

    #[test]
    fn find_reusable_entry는_종료_중이_아닌_세션은_그대로_재사용_후보로_반환한다() {
        let project_id = ProjectId::new();
        let server_id = LspServerId::from("test-server");
        let store = LspStore::new();
        store.0.lock().insert(
            "active-session".to_string(),
            test_session_entry(project_id.clone(), server_id.clone(), "owner-a", false),
        );

        let reused = find_reusable_entry(&store, &project_id, &server_id, "owner-a");
        assert_eq!(reused.map(|(id, _)| id), Some("active-session".to_string()));
    }

    #[test]
    fn broadcast_은_모든_구독자에게_전달된다() {
        let received_a = Arc::new(Mutex::new(Vec::new()));
        let received_b = Arc::new(Mutex::new(Vec::new()));
        let channels = Mutex::new(HashMap::from([
            ("a".to_string(), recording_channel(received_a.clone())),
            ("b".to_string(), recording_channel(received_b.clone())),
        ]));

        broadcast_message(&channels, "hello");

        assert_eq!(*received_a.lock(), vec!["hello".to_string()]);
        assert_eq!(*received_b.lock(), vec!["hello".to_string()]);
    }

    #[test]
    fn 단일_구독자_시나리오는_기존과_동일하게_전달된다() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let channels = Mutex::new(HashMap::from([("a".to_string(), recording_channel(received.clone()))]));

        broadcast_message(&channels, "first");
        broadcast_message(&channels, "second");

        assert_eq!(*received.lock(), vec!["first".to_string(), "second".to_string()]);
    }

    #[test]
    fn 전송에_실패한_구독자는_다음_브로드캐스트에서_제거되고_남은_구독자는_계속_받는다() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let channels = Mutex::new(HashMap::from([
            ("a".to_string(), failing_channel()),
            ("b".to_string(), recording_channel(received.clone())),
        ]));

        broadcast_message(&channels, "first");
        assert_eq!(channels.lock().len(), 1, "실패한 구독자는 제거되어야 한다");

        broadcast_message(&channels, "second");
        assert_eq!(*received.lock(), vec!["first".to_string(), "second".to_string()]);
    }

    #[test]
    fn owner_로_제거하면_그_구독만_사라지고_나머지_owner_는_계속_받는다() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let channels = Mutex::new(HashMap::from([
            ("window-a".to_string(), failing_channel()),
            ("window-b".to_string(), recording_channel(received.clone())),
        ]));

        channels.lock().remove("window-a");
        assert_eq!(channels.lock().keys().collect::<Vec<_>>(), vec!["window-b"]);

        broadcast_message(&channels, "data");
        assert_eq!(*received.lock(), vec!["data".to_string()]);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wait_for_process_exit는_프로세스가_먼저_종료하면_타임아웃보다_일찍_반환한다() {
        let config = lsp_proc::LspProcConfig {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "exit 0".to_string()],
            cwd: std::env::temp_dir(),
        };
        let proc = lsp_proc::spawn(config, |_message| {}, |_code| {}).expect("프로세스 spawn 성공");

        let started = tokio::time::Instant::now();
        wait_for_process_exit(&proc, 2_000).await;

        assert!(proc.is_exited(), "폴링이 반환했다면 프로세스는 이미 종료된 상태여야 한다");
        assert!(
            started.elapsed() < tokio::time::Duration::from_millis(1_000),
            "빨리 종료하는 프로세스는 2초 타임아웃을 다 기다리지 않고 일찍 반환해야 한다"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wait_for_process_exit는_계속_살아있는_프로세스에서_타임아웃까지_대기한다() {
        let config = lsp_proc::LspProcConfig {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "sleep 5".to_string()],
            cwd: std::env::temp_dir(),
        };
        let proc = lsp_proc::spawn(config, |_message| {}, |_code| {}).expect("프로세스 spawn 성공");

        let started = tokio::time::Instant::now();
        wait_for_process_exit(&proc, 200).await;

        assert!(
            !proc.is_exited(),
            "타임아웃 안에 스스로 종료하지 않는 프로세스는 여전히 살아있어야 한다"
        );
        assert!(started.elapsed() >= tokio::time::Duration::from_millis(200));

        proc.kill();
    }

    fn sleeping_proc() -> Arc<lsp_proc::LspProcHandle> {
        let config = lsp_proc::LspProcConfig {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "sleep 5".to_string()],
            cwd: std::env::temp_dir(),
        };
        Arc::new(lsp_proc::spawn(config, |_message| {}, |_code| {}).expect("프로세스 spawn 성공"))
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn confirms_healthy_restart는_같은_프로세스가_아직_살아있고_현재_슬롯에_설치되어_있으면_true를_반환한다() {
        let proc = sleeping_proc();
        let slot = Mutex::new(Some(proc.clone()));

        assert!(confirms_healthy_restart(&slot, &proc));

        proc.kill();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn confirms_healthy_restart는_슬롯이_다른_프로세스로_교체됐으면_false를_반환한다() {
        let respawned = sleeping_proc();
        let replaced_by_a_later_crash = sleeping_proc();
        let slot = Mutex::new(Some(replaced_by_a_later_crash.clone()));

        assert!(
            !confirms_healthy_restart(&slot, &respawned),
            "슬롯이 이미 다른(더 최근) respawn으로 교체됐다면 이 respawn의 건강 판정을 내리면 안 된다"
        );

        respawned.kill();
        replaced_by_a_later_crash.kill();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn confirms_healthy_restart는_프로세스가_이미_종료됐으면_false를_반환한다() {
        let proc = sleeping_proc();
        let slot = Mutex::new(Some(proc.clone()));
        proc.kill();

        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(2);
        while !proc.is_exited() && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        assert!(
            !confirms_healthy_restart(&slot, &proc),
            "건강 판정 창이 끝나기 전에 다시 죽었다면 restart_count를 리셋하면 안 된다"
        );
    }
}

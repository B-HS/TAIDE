use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub use taide_lsp::install::LspInstallStore;
use taide_lsp::process::{
    confirms_healthy_restart, restart_backoff_delay, shutdown_process, spawn_language_server, HEALTHY_RESTART_WINDOW,
};
use taide_lsp::protocol::workspace_folders_notification;
use taide_lsp::session::{LspLifecycleSnapshot, LspMessageSubscribers};
use taide_lsp::store::LspSessionEntry as SessionEntry;
pub use taide_lsp::store::LspStore;
use taide_model::app_event::AppEvent;
use taide_runtime::{EventSink, TaskSupervisor};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use super::manifest;
use super::service;
use super::types::{
    LanguageServerSpec, LspInstallPhase, LspInstallStrategy, LspServerDetection, LspServerId, LspSessionInfo, LspSessionStatus,
    LspSpawnRequest,
};
use crate::error::{AppError, AppErrorKind, AppResult};
use crate::ids::ProjectId;
use crate::infra::lsp_proc;
use crate::infra::perf::{self, CounterSlot};
use crate::infra::redact::mask_known_secrets;
use crate::platform::event_sink::TauriEventSink;
use crate::state::AppState;

/// Joins the lines of a language server's stderr tail into the single log line
/// [`handle_process_exit`] writes. A multi-line value would interleave with every other line in the
/// rotating app log and break `grep`-ing one exit report out of it.
const STDERR_TAIL_LOG_SEPARATOR: &str = " / ";

fn new_session_id() -> String {
    format!("lsp-{}", uuid::Uuid::new_v4())
}

fn find_entry(store: &LspStore, session_id: &str) -> AppResult<Arc<SessionEntry>> {
    store
        .get(session_id)
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
    store.find_reusable(project_id, server_id, owner)
}

fn ensure_project_open(state: &AppState, project_id: &ProjectId) -> AppResult<()> {
    if state.projects.read().contains_key(project_id) {
        return Ok(());
    }
    Err(AppError::NotFound(format!("project not open: {project_id}")))
}

fn emit_status(app: &AppHandle, session_id: &str, snapshot: LspLifecycleSnapshot) {
    TauriEventSink(app).publish(AppEvent::LspSessionStatusChanged {
        session_id: session_id.to_string(),
        status: snapshot.status,
        last_error: snapshot.last_error,
        generation: snapshot.generation,
    });
}

fn set_status(app: &AppHandle, session_id: &str, entry: &SessionEntry, status: LspSessionStatus, last_error: Option<String>) {
    emit_status(app, session_id, entry.lifecycle.set_status(status, last_error));
}

fn spawn_process(
    app: &AppHandle,
    session_id: String,
    process_epoch: u64,
    spec: LanguageServerSpec,
    root: String,
) -> AppResult<Arc<lsp_proc::LspProcHandle>> {
    let paths = &app.state::<AppState>().paths;
    let message_app = app.clone();
    let message_session_id = session_id.clone();

    let exit_app = app.clone();
    let exit_session_id = session_id.clone();

    spawn_language_server(
        paths,
        &spec,
        &root,
        move |message| {
            let Some(store) = message_app.try_state::<LspStore>() else {
                return;
            };
            let Ok(entry) = find_entry(&store, &message_session_id) else {
                return;
            };
            if !entry.lifecycle.is_active_process_epoch(process_epoch) {
                return;
            }
            entry.subscribers.broadcast(&message);
        },
        move |code, stderr_tail| {
            let Some(tasks) = exit_app.try_state::<TaskSupervisor>() else {
                return;
            };
            let task_app = exit_app.clone();
            tasks.spawn_transient("lsp-process-exit", async move {
                let Some(state) = task_app.try_state::<AppState>() else {
                    return;
                };
                let _guard = state.begin_mutation().await;
                handle_process_exit(&task_app, exit_session_id, process_epoch, code, stderr_tail);
            });
        },
    )
}

fn channel_sink(channel: Channel<String>) -> impl Fn(&str) -> bool + Send + Sync {
    move |message| channel.send(message.to_string()).is_ok()
}

/// The stderr tail as one masked log line. A language server's stderr is process output exactly the
/// way a toolchain installer's is, so it gets the same treatment before reaching the rotating disk
/// log (d-57 §1.D): a server launched through a wrapper script — `npx`, a `.bin` shim, a venv
/// activation — can echo registry credentials or tokens into it while failing.
fn masked_stderr_tail(tail: &str) -> String {
    mask_known_secrets(tail)
        .lines()
        .map(|line| line.trim_end())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(STDERR_TAIL_LOG_SEPARATOR)
}

fn handle_process_exit(app: &AppHandle, session_id: String, process_epoch: u64, code: Option<i32>, stderr_tail: String) {
    let Some(store) = app.try_state::<LspStore>() else {
        return;
    };
    let Ok(entry) = find_entry(&store, &session_id) else {
        return;
    };

    let Some(restarts) = entry.lifecycle.begin_exit_recovery(process_epoch) else {
        log::info!(
            "lsp {}: ignored stopped or superseded process exit (code={code:?})",
            entry.server_id
        );
        return;
    };

    log::warn!(
        "lsp {}: exited code={code:?} restarts={restarts} stderr_tail={}",
        entry.server_id,
        masked_stderr_tail(&stderr_tail)
    );
    let Some(backoff) = restart_backoff_delay(restarts) else {
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
    };

    set_status(
        app,
        &session_id,
        &entry,
        LspSessionStatus::Starting,
        Some(format!("서버가 종료되어 재시작합니다 (마지막 종료 코드: {code:?})")),
    );

    let restart_app = app.clone();
    let restart_session_id = session_id.clone();
    let exited_process_epoch = process_epoch;
    let spec = entry.spec.clone();
    let root = entry.root.clone();

    app.state::<TaskSupervisor>().spawn_transient("lsp-auto-restart", async move {
        tokio::time::sleep(backoff).await;

        let Some(state) = restart_app.try_state::<AppState>() else {
            return;
        };
        let _guard = state.begin_mutation().await;
        let Some(store) = restart_app.try_state::<LspStore>() else {
            return;
        };
        let Ok(entry) = find_entry(&store, &restart_session_id) else {
            return;
        };
        if !entry.lifecycle.is_active_process_epoch(exited_process_epoch) {
            return;
        }

        let respawn_process_epoch = entry.lifecycle.advance_process_epoch();
        match spawn_process(&restart_app, restart_session_id.clone(), respawn_process_epoch, spec, root) {
            Ok(proc) => {
                *entry.proc.lock() = Some(proc.clone());
                emit_status(
                    &restart_app,
                    &restart_session_id,
                    entry.lifecycle.auto_respawned(
                        "서버 프로세스가 자동으로 재시작됐습니다. 초기화 핸드셰이크가 다시 완료될 때까지 기다려주세요.".to_string(),
                    ),
                );

                let healthy_reset_entry = entry.clone();
                restart_app
                    .state::<TaskSupervisor>()
                    .spawn_transient("lsp-healthy-reset", async move {
                        tokio::time::sleep(HEALTHY_RESTART_WINDOW).await;
                        if confirms_healthy_restart(&healthy_reset_entry.proc, &proc) {
                            healthy_reset_entry.lifecycle.reset_restart_count();
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

async fn shutdown_entry(app: &AppHandle, entry: &SessionEntry, session_id: &str) {
    entry.lifecycle.mark_stopping();

    let proc = entry.proc.lock().clone();
    if let Some(proc) = proc {
        shutdown_process(&proc).await;
    }

    set_status(app, session_id, entry, LspSessionStatus::Stopped, None);
}

/// `request.owner` identifies the calling window (`getCurrentWindow().label` on the frontend —
/// `main`, `editor-<n>`, or the remote client's fixed `"remote"` label) so [`find_reusable_entry`]
/// only reuses a session within the same window. `request` bundles the session inputs into one
/// struct, mirroring `pty_spawn`'s `opts`.
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
        let existing_roots = existing_entry.roots.paths();

        if service::should_reuse_session(&spec, &existing_roots, &root) {
            let is_new_root = existing_entry.roots.acquire(root.clone());

            existing_entry.subscribers.insert(owner, channel_sink(on_message));

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

    let subscribers = LspMessageSubscribers::new();
    subscribers.insert(owner, channel_sink(on_message));

    let entry = Arc::new(SessionEntry::new(project_id, spec.clone(), root.clone(), subscribers));

    store.insert(session_id.clone(), entry.clone());

    let process_epoch = entry.lifecycle.advance_process_epoch();
    let proc = match spawn_process(&app, session_id.clone(), process_epoch, spec, root) {
        Ok(proc) => proc,
        Err(error) => {
            store.remove(&session_id);
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
///
/// Instrumented with a [`CounterSlot`], never a `perf::span`, for the same frequency reason: the
/// `didChange`/completion/semantic-token traffic behind this command makes it one of the busiest
/// in the app, and what matters is the number of IPC round trips, not any single write's duration
/// (`infra::perf::CounterSlot`).
#[tauri::command]
#[specta::specta]
pub async fn lsp_send(store: State<'_, LspStore>, session_id: String, message: String) -> AppResult<()> {
    perf::add(CounterSlot::LspSend, 1);
    let entry = find_entry(&store, &session_id)?;
    let proc = entry
        .proc
        .lock()
        .clone()
        .ok_or_else(|| AppError::Internal("language server not ready".to_string()))?;
    proc.write_message(&message).await
}

fn release_owner_root(entry: &SessionEntry, owner: &str, root: Option<&str>) -> (Option<String>, bool) {
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

/// `owner` (`getCurrentWindow().label`, same value the caller passed to `lsp_spawn`) keeps its
/// subscriber while any root remains. The last root or a rootless stop removes the subscriber
/// explicitly; send-failure pruning cannot detect a live window that released the session.
///
/// The guard (`AppState::begin_mutation`) is held only for the synchronous bookkeeping above and,
/// on the full-teardown path, for unlinking the entry from [`LspStore`] — `store.remove`
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
/// while the old one's process is still being killed in the background), and the old process is
/// guaranteed to be killed once `shutdown_entry` completes.
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
    if !store.contains(&session_id) {
        return Err(AppError::NotFound(format!("lsp session not found: {session_id}")));
    }

    let process_epoch = entry.lifecycle.advance_process_epoch();
    emit_status(&app, &session_id, entry.lifecycle.begin_manual_restart());

    let proc = spawn_process(&app, session_id.clone(), process_epoch, entry.spec.clone(), entry.root.clone())?;
    *entry.proc.lock() = Some(proc);
    set_status(&app, &session_id, &entry, LspSessionStatus::Running, None);

    Ok(())
}

/// Confirms reinitialization only for the current generation of a crashed session.
#[tauri::command]
#[specta::specta]
pub async fn lsp_confirm_reinitialize(app: AppHandle, store: State<'_, LspStore>, session_id: String, generation: u32) -> AppResult<()> {
    let entry = find_entry(&store, &session_id)?;
    if let Some(snapshot) = entry.lifecycle.confirm_reinitialized(generation) {
        emit_status(&app, &session_id, snapshot);
    }
    Ok(())
}

const REINITIALIZE_FAILURE_MESSAGE: &str =
    "초기화 핸드셰이크 재시도를 모두 소진해 서버를 재연결하지 못했습니다. 수동으로 다시 시작해주세요.";

/// Records reinitialization failure only for the current generation of a crashed session.
#[tauri::command]
#[specta::specta]
pub async fn lsp_report_reinitialize_failure(
    app: AppHandle,
    store: State<'_, LspStore>,
    session_id: String,
    generation: u32,
) -> AppResult<()> {
    let entry = find_entry(&store, &session_id)?;
    if let Some(snapshot) = entry
        .lifecycle
        .report_reinitialize_failure(generation, REINITIALIZE_FAILURE_MESSAGE.to_string())
    {
        emit_status(&app, &session_id, snapshot);
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn lsp_sessions(store: State<'_, LspStore>, project_id: ProjectId) -> AppResult<Vec<LspSessionInfo>> {
    Ok(store.sessions_for_project(&project_id))
}

#[tauri::command]
#[specta::specta]
pub async fn lsp_detect_servers(state: State<'_, AppState>) -> AppResult<Vec<LspServerDetection>> {
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    log::debug!("lsp detect: PATH={}", path_var.to_string_lossy());

    let detections = service::detect_servers(&state.paths.lsp_dir(), &path_var);
    for detection in &detections {
        log::info!(
            "lsp detect: {} available={} path={:?}",
            detection.id,
            detection.available,
            detection.resolved_path
        );
    }

    Ok(detections)
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
    TauriEventSink(app).publish(AppEvent::LspInstallProgress {
        server_id: server_id.clone(),
        phase,
        received_bytes: received_bytes as f64,
        total_bytes: total_bytes.map(|value| value as f64),
        message,
    });
}

const TOOLCHAIN_POLL_INTERVAL_MS: u64 = 100;
const TOOLCHAIN_OUTPUT_TAIL_LINES: usize = 20;

/// Lowest pid that may be used as a `kill(-pid)` process-group target. `0` means "my own process
/// group" (every TAIDE thread and every terminal/language-server child it owns) and `1` means
/// "every process this user may signal" — see [`should_signal_process_group`].
#[cfg(unix)]
const MIN_SIGNALABLE_PGID: u32 = 2;

/// Whether `pid` is safe to pass to [`kill_toolchain_process_group`] as a group target. Refusing
/// `0`/`1` is what keeps a cancelled install from turning into an app-wide (or session-wide) kill
/// if `Child::id()` ever comes back with a pid the app does not own. Contract §1.E.
#[cfg(unix)]
fn should_signal_process_group(pid: u32) -> bool {
    pid >= MIN_SIGNALABLE_PGID
}

/// Signals the whole process group (negative pid) of a cancelled toolchain install: `start_kill()`
/// reaches only the direct child, but installers (go/gem/coursier/ghcup) usually spawn a compiler
/// or sub-installer that would otherwise be orphaned and keep running.
///
/// The caller must confirm the child is still alive (`Child::try_wait` returning anything but
/// `Ok(Some(_))`) immediately before calling: once a child has exited, its pid — and with it the
/// group id derived from it — can be recycled by the OS onto an unrelated process.
#[cfg(unix)]
fn kill_toolchain_process_group(pid: u32) {
    if !should_signal_process_group(pid) {
        log::warn!("툴체인 설치 취소: 프로세스 그룹으로 시그널할 수 없는 pid 라 건너뜁니다 ({pid})");
        return;
    }
    let _ = std::process::Command::new("kill").arg("-TERM").arg(format!("-{pid}")).status();
}

/// The failure text for a toolchain installer that exited non-zero, with the captured output tail
/// masked ([`mask_known_secrets`]) before it reaches either sink: this one string is both the
/// `emit_install_progress` message — which `native-notification-provider.tsx` shows as an OS
/// notification body — and the returned `AppError`, and package-manager installers routinely echo
/// registry credentials (`_authToken=…`) in exactly this tail. Contract §1.D.
fn toolchain_install_failure_message(binary: &str, exit_code: Option<i32>, tail: &str) -> String {
    let masked_tail = mask_known_secrets(tail);
    if masked_tail.is_empty() {
        return format!("{binary} 설치 명령이 실패했습니다 (종료 코드: {exit_code:?})");
    }
    format!("{binary} 설치 명령이 실패했습니다 (종료 코드: {exit_code:?}): {masked_tail}")
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
    let toolchain = spec.install.toolchain.as_ref().ok_or_else(|| {
        AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.lsp.toolchainInfoMissing",
            format!("{}: toolchain install info is not configured yet", spec.id),
        )
        .with_arg("serverId", &spec.id)
    })?;

    let binary = service::toolchain_binary(toolchain.tool);
    if service::find_in_path(binary).is_none() {
        return Err(AppError::localized(
            AppErrorKind::NotFound,
            "error.lsp.toolchainNotFound",
            format!("could not find the {binary} toolchain"),
        )
        .with_arg("binary", binary));
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

    let mut child = command.spawn().map_err(|error| {
        AppError::localized(
            AppErrorKind::Internal,
            "error.lsp.toolchainRunFailed",
            format!("{binary} failed to run: {error}"),
        )
        .with_arg("binary", binary)
        .with_arg("detail", &error)
    })?;
    let child_pid = child.id();

    let stdout_tail = child.stdout.take().map(capture_output_tail);
    let stderr_tail = child.stderr.take().map(capture_output_tail);

    loop {
        if cancel.load(Ordering::SeqCst) {
            #[cfg(unix)]
            if let Some(pid) = child_pid {
                if !matches!(child.try_wait(), Ok(Some(_))) {
                    kill_toolchain_process_group(pid);
                }
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
            return Err(AppError::localized(
                AppErrorKind::Internal,
                "error.lsp.installCancelled",
                "Installation was cancelled",
            ));
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
                let message = toolchain_install_failure_message(binary, status.code(), &tail_lines.join("\n"));
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
    tasks: State<'_, TaskSupervisor>,
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
    let cancel = install_guard.cancellation_token();

    match spec.install.strategy {
        LspInstallStrategy::Download => {
            taide_runtime::lsp_install_actions::run_download_install(
                &TauriEventSink(&app),
                &state.paths,
                &spec,
                &install_guard.lease(),
                &tasks,
            )
            .await
        }
        LspInstallStrategy::Toolchain => run_toolchain_install(&app, &spec, cancel).await,
        LspInstallStrategy::SdkDetect => Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.lsp.sdkDetectOnly",
            format!("{}: SDK-detect-only servers cannot be installed automatically", spec.id),
        )
        .with_arg("serverId", &spec.id)),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn lsp_install_cancel(install_store: State<'_, LspInstallStore>, server_id: LspServerId) -> AppResult<()> {
    install_store.cancel(&server_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use parking_lot::Mutex;

    use super::*;
    use crate::domain::lsp::types::{LspCommandSpec, LspInstallSpec, LspRootStrategy};

    #[test]
    fn 툴체인_설치_실패_메시지의_레지스트리_자격증명은_마스킹된다() {
        let tail = "npm ERR! code E401\nnpm ERR! //registry.npmjs.org/:_authToken=abcd-1234-efgh-5678\nnpm ERR! 401 Unauthorized";
        let message = toolchain_install_failure_message("npm", Some(1), tail);

        assert!(
            !message.contains("abcd-1234-efgh-5678"),
            "설치 실패 알림 본문에 토큰이 남아 있습니다: {message}"
        );
        assert!(message.contains("_authToken=[redacted:key_value]"));
        assert!(
            message.contains("npm ERR! 401 Unauthorized"),
            "실패 원인은 그대로 남아야 한다: {message}"
        );
        assert!(message.starts_with("npm 설치 명령이 실패했습니다 (종료 코드: Some(1))"));
    }

    /// The exit log's stderr tail follows the same masking policy as the install tail (d-57 §1.D) —
    /// it lands in the same rotating disk log — and is flattened so one exit report is one line.
    #[test]
    fn 종료_로그의_stderr_tail은_자격증명을_마스킹하고_한_줄로_합친다() {
        let masked = masked_stderr_tail("npm ERR! //registry.npmjs.org/:_authToken=abcd-1234-efgh-5678\n\nvtsls: exiting\n");

        assert!(
            !masked.contains("abcd-1234-efgh-5678"),
            "토큰이 로그에 그대로 남아 있습니다: {masked}"
        );
        assert!(masked.contains("_authToken=[redacted:key_value]"));
        assert!(masked.contains("vtsls: exiting"), "종료 원인 문구는 그대로 남아야 한다: {masked}");
        assert!(!masked.contains('\n'), "로그 한 줄로 합쳐져야 한다: {masked}");
    }

    #[test]
    fn 툴체인_설치_실패_메시지는_출력이_없으면_종료_코드만_남긴다() {
        assert_eq!(
            toolchain_install_failure_message("go", None, ""),
            "go 설치 명령이 실패했습니다 (종료 코드: None)"
        );
    }

    #[cfg(unix)]
    #[test]
    fn 프로세스_그룹_시그널은_자기_그룹과_전체_시그널_pid를_거부한다() {
        assert!(!should_signal_process_group(0), "0 은 TAIDE 자신의 프로세스 그룹이다");
        assert!(!should_signal_process_group(1), "1 은 시그널 가능한 모든 프로세스를 뜻한다");
        assert!(should_signal_process_group(MIN_SIGNALABLE_PGID));
        assert!(should_signal_process_group(48_231));
    }

    /// §4-A-7 regression at the notification level: the roots this sends must carry the same URI
    /// spelling `initialize` used, or the server treats the added root as a folder it has never seen
    /// and the removed one as a folder it does not have.
    #[test]
    fn 워크스페이스_폴더_알림의_uri는_퍼센트_인코딩된다() {
        let notification = workspace_folders_notification(&["/workspace/my project".to_string()], &["/tmp/한글 루트".to_string()]);
        let parsed: serde_json::Value = serde_json::from_str(&notification).expect("알림은 JSON 이어야 한다");
        let event = &parsed["params"]["event"];

        assert_eq!(event["added"][0]["uri"], "file:///workspace/my%20project");
        assert_eq!(event["added"][0]["name"], "my project");
        assert_eq!(event["removed"][0]["uri"], "file:///tmp/%ED%95%9C%EA%B8%80%20%EB%A3%A8%ED%8A%B8");
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
        let subscribers = LspMessageSubscribers::new();
        subscribers.insert(owner.to_string(), channel_sink(failing_channel()));
        let entry = Arc::new(SessionEntry::new(
            project_id,
            LanguageServerSpec {
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
            "/tmp/project".to_string(),
            subscribers,
        ));
        entry.lifecycle.set_status(LspSessionStatus::Running, None);
        if stopping {
            entry.lifecycle.mark_stopping();
        }
        entry
    }

    #[test]
    fn 새로_생성된_세션엔트리의_세대는_0에서_시작한다() {
        let entry = test_session_entry(ProjectId::new(), LspServerId::from("test-server"), "owner-a", false);
        assert_eq!(entry.lifecycle.snapshot().generation, 0);
    }

    #[test]
    fn find_reusable_entry는_종료_중인_세션을_재사용_후보에서_제외한다() {
        let project_id = ProjectId::new();
        let server_id = LspServerId::from("test-server");
        let store = LspStore::new();
        store.insert(
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
        store.insert(
            "active-session".to_string(),
            test_session_entry(project_id.clone(), server_id.clone(), "owner-a", false),
        );

        let reused = find_reusable_entry(&store, &project_id, &server_id, "owner-a");
        assert_eq!(reused.map(|(id, _)| id), Some("active-session".to_string()));
        assert!(find_reusable_entry(&store, &project_id, &server_id, "owner-b").is_none());
    }

    #[test]
    fn 일부_root_해제는_남은_root의_owner_구독을_유지한다() {
        let entry = test_session_entry(ProjectId::new(), LspServerId::from("test-server"), "owner-a", false);
        let received = Arc::new(Mutex::new(Vec::new()));
        entry
            .subscribers
            .insert("owner-a".to_string(), channel_sink(recording_channel(received.clone())));
        entry.roots.acquire("/tmp/second-root".to_string());

        let (removed_root, has_remaining_roots) = release_owner_root(&entry, "owner-a", Some("/tmp/project"));
        entry.subscribers.broadcast("remaining-root-message");

        assert_eq!(removed_root.as_deref(), Some("/tmp/project"));
        assert!(has_remaining_roots);
        assert!(entry.subscribers.contains("owner-a"));
        assert_eq!(*received.lock(), vec!["remaining-root-message".to_string()]);
    }

    #[test]
    fn 같은_root의_마지막_참조를_해제할_때만_owner_구독을_제거한다() {
        let entry = test_session_entry(ProjectId::new(), LspServerId::from("test-server"), "owner-a", false);
        entry.roots.acquire("/tmp/project".to_string());

        let (removed_root, has_remaining_roots) = release_owner_root(&entry, "owner-a", Some("/tmp/project"));
        assert!(removed_root.is_none());
        assert!(has_remaining_roots);
        assert!(entry.subscribers.contains("owner-a"));

        let (removed_root, has_remaining_roots) = release_owner_root(&entry, "owner-a", Some("/tmp/project"));
        assert_eq!(removed_root.as_deref(), Some("/tmp/project"));
        assert!(!has_remaining_roots);
        assert!(!entry.subscribers.contains("owner-a"));
    }

    #[test]
    fn root_없는_전체_종료는_owner_구독을_제거한다() {
        let entry = test_session_entry(ProjectId::new(), LspServerId::from("test-server"), "owner-a", false);

        let (removed_root, has_remaining_roots) = release_owner_root(&entry, "owner-a", None);

        assert!(removed_root.is_none());
        assert!(!has_remaining_roots);
        assert!(!entry.subscribers.contains("owner-a"));
    }

    #[test]
    fn broadcast_은_모든_구독자에게_전달된다() {
        let received_a = Arc::new(Mutex::new(Vec::new()));
        let received_b = Arc::new(Mutex::new(Vec::new()));
        let subscribers = LspMessageSubscribers::new();
        subscribers.insert("a".to_string(), channel_sink(recording_channel(received_a.clone())));
        subscribers.insert("b".to_string(), channel_sink(recording_channel(received_b.clone())));

        subscribers.broadcast("hello");

        assert_eq!(*received_a.lock(), vec!["hello".to_string()]);
        assert_eq!(*received_b.lock(), vec!["hello".to_string()]);
    }

    #[test]
    fn 단일_구독자_시나리오는_기존과_동일하게_전달된다() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let subscribers = LspMessageSubscribers::new();
        subscribers.insert("a".to_string(), channel_sink(recording_channel(received.clone())));

        subscribers.broadcast("first");
        subscribers.broadcast("second");

        assert_eq!(*received.lock(), vec!["first".to_string(), "second".to_string()]);
    }

    #[test]
    fn 전송에_실패한_구독자는_다음_브로드캐스트에서_제거되고_남은_구독자는_계속_받는다() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let subscribers = LspMessageSubscribers::new();
        subscribers.insert("a".to_string(), channel_sink(failing_channel()));
        subscribers.insert("b".to_string(), channel_sink(recording_channel(received.clone())));

        subscribers.broadcast("first");
        assert!(!subscribers.contains("a"), "실패한 구독자는 제거되어야 한다");

        subscribers.broadcast("second");
        assert_eq!(*received.lock(), vec!["first".to_string(), "second".to_string()]);
    }

    #[test]
    fn owner_로_제거하면_그_구독만_사라지고_나머지_owner_는_계속_받는다() {
        let received = Arc::new(Mutex::new(Vec::new()));
        let subscribers = LspMessageSubscribers::new();
        subscribers.insert("window-a".to_string(), channel_sink(failing_channel()));
        subscribers.insert("window-b".to_string(), channel_sink(recording_channel(received.clone())));

        subscribers.remove("window-a");
        assert!(!subscribers.contains("window-a"));
        assert!(subscribers.contains("window-b"));

        subscribers.broadcast("data");
        assert_eq!(*received.lock(), vec!["data".to_string()]);
    }
}

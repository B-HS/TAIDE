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
    RESTART_BACKOFF_LIMIT,
};
use crate::error::{AppError, AppResult};
use crate::events::{LspInstallProgress, LspSessionStatusChanged};
use crate::ids::ProjectId;
use crate::infra::lsp_install;
use crate::infra::lsp_proc;
use crate::paths::AppPaths;
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

fn find_reusable_entry(store: &LspStore, project_id: &ProjectId, server_id: &LspServerId) -> Option<(String, Arc<SessionEntry>)> {
    store
        .0
        .lock()
        .iter()
        .find(|(_, entry)| &entry.project_id == project_id && &entry.server_id == server_id && entry.spec.shares_sessions)
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
    let resolved = service::resolve_spec_command(
        &spec,
        Some(std::path::Path::new(&root)),
        managed_dir.as_deref(),
        managed_relative_path,
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

    let spec = manifest::find_spec(server_id.as_str())
        .ok_or_else(|| AppError::InvalidArgument(format!("unknown language server: {server_id}")))?;

    if let Some((existing_id, existing_entry)) = find_reusable_entry(&store, &project_id, &server_id) {
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
        server_id: server_id.clone(),
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
            server_id: entry.server_id.clone(),
            root: entry.root.clone(),
            status: *entry.status.lock(),
            last_error: entry.last_error.lock().clone(),
        })
        .collect())
}

#[tauri::command]
#[specta::specta]
pub async fn lsp_detect_servers(state: State<'_, AppState>) -> AppResult<Vec<LspServerDetection>> {
    Ok(service::detect_servers(&state.paths.lsp_dir()))
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

    let client = reqwest::Client::new();
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

    let result = match spec.install.strategy {
        LspInstallStrategy::Download => run_download_install(&app, &state.paths, &spec, cancel.clone()).await,
        LspInstallStrategy::Toolchain => run_toolchain_install(&app, &spec, cancel.clone()).await,
        LspInstallStrategy::SdkDetect => Err(AppError::InvalidArgument(format!(
            "{} 는 SDK 감지 전용이라 자동 설치할 수 없습니다",
            spec.id
        ))),
    };

    install_store.finish(&server_id, &cancel);
    result
}

#[tauri::command]
#[specta::specta]
pub async fn lsp_install_cancel(install_store: State<'_, LspInstallStore>, server_id: LspServerId) -> AppResult<()> {
    if let Some(cancel) = install_store.0.lock().get(&server_id) {
        cancel.store(true, Ordering::SeqCst);
    }
    Ok(())
}

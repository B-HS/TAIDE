use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use parking_lot::Mutex;
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;
use tokio::sync::{broadcast, oneshot};

use super::lockfile;
use super::server;
use super::service;
use super::types::{IdeDiagnostic, IdeDiffOutcome, IdeSelectionInput, IdeStatus, IDE_PORT_BIND_MAX_ATTEMPTS};
use crate::domain::layout::types::{Tab, TabKind};
use crate::domain::remote::types::REMOTE_OWNER_LABEL;
use crate::error::{AppError, AppResult};
use crate::events::IdeStatusChanged;
use crate::ids::ProjectId;
use crate::state::AppState;

const NOTIFY_CHANNEL_CAPACITY: usize = 64;

pub struct PendingDiff {
    pub project_id: ProjectId,
    pub new_path: PathBuf,
    pub responder: oneshot::Sender<(IdeDiffOutcome, Option<String>)>,
}

pub struct PendingSave {
    pub responder: oneshot::Sender<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdeSelectionSnapshot {
    pub project_id: ProjectId,
    pub path: String,
    pub text: String,
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
    pub is_empty: bool,
}

#[derive(Default)]
struct IdeStoreInner {
    running: bool,
    port: u32,
    token: String,
    lockfile_dir: Option<PathBuf>,
    client_count: u32,
    server_handle: Option<tauri::async_runtime::JoinHandle<()>>,
    connection_handles: Vec<tauri::async_runtime::JoinHandle<()>>,
    pending_diffs: HashMap<String, PendingDiff>,
    pending_saves: HashMap<String, PendingSave>,
    current_selection: Option<IdeSelectionSnapshot>,
    latest_selection: Option<IdeSelectionSnapshot>,
    diagnostics: HashMap<ProjectId, Vec<IdeDiagnostic>>,
    diagnostics_ready: bool,
}

pub struct ShutdownState {
    pub port: u32,
    pub dir: Option<PathBuf>,
    pub server_handle: Option<tauri::async_runtime::JoinHandle<()>>,
    pub connection_handles: Vec<tauri::async_runtime::JoinHandle<()>>,
    pub pending_diffs: Vec<PendingDiff>,
    pub pending_saves: Vec<PendingSave>,
}

pub struct IdeStore {
    inner: Mutex<IdeStoreInner>,
    notify_tx: broadcast::Sender<String>,
}

impl Default for IdeStore {
    fn default() -> Self {
        let (notify_tx, _rx) = broadcast::channel(NOTIFY_CHANNEL_CAPACITY);
        Self {
            inner: Mutex::new(IdeStoreInner::default()),
            notify_tx,
        }
    }
}

impl IdeStore {
    pub fn status(&self) -> IdeStatus {
        let inner = self.inner.lock();
        IdeStatus {
            running: inner.running,
            port: inner.port,
            connected: inner.client_count > 0,
            client_count: inner.client_count,
        }
    }

    pub fn is_running(&self) -> bool {
        self.inner.lock().running
    }

    pub fn mark_started(&self, port: u32, token: String, dir: PathBuf, server_handle: tauri::async_runtime::JoinHandle<()>) -> IdeStatus {
        let mut inner = self.inner.lock();
        inner.running = true;
        inner.port = port;
        inner.token = token;
        inner.lockfile_dir = Some(dir);
        inner.server_handle = Some(server_handle);
        inner.client_count = 0;
        IdeStatus {
            running: true,
            port,
            connected: false,
            client_count: 0,
        }
    }

    pub fn take_shutdown_state(&self) -> Option<ShutdownState> {
        let mut inner = self.inner.lock();
        if !inner.running {
            return None;
        }
        inner.running = false;
        let port = inner.port;
        inner.port = 0;
        inner.token.clear();
        inner.client_count = 0;
        inner.diagnostics.clear();
        inner.diagnostics_ready = false;
        inner.current_selection = None;
        inner.latest_selection = None;

        Some(ShutdownState {
            port,
            dir: inner.lockfile_dir.take(),
            server_handle: inner.server_handle.take(),
            connection_handles: std::mem::take(&mut inner.connection_handles),
            pending_diffs: inner.pending_diffs.drain().map(|(_, pending)| pending).collect(),
            pending_saves: inner.pending_saves.drain().map(|(_, pending)| pending).collect(),
        })
    }

    pub fn lockfile_context(&self) -> Option<(u32, String, PathBuf)> {
        let inner = self.inner.lock();
        if !inner.running {
            return None;
        }
        let dir = inner.lockfile_dir.clone()?;
        Some((inner.port, inner.token.clone(), dir))
    }

    pub fn register_connection(&self, handle: tauri::async_runtime::JoinHandle<()>) {
        let mut inner = self.inner.lock();
        inner.connection_handles.retain(|existing| !existing.inner().is_finished());
        inner.connection_handles.push(handle);
    }

    pub fn client_connected(&self) -> u32 {
        let mut inner = self.inner.lock();
        inner.client_count += 1;
        inner.client_count
    }

    pub fn client_disconnected(&self) -> u32 {
        let mut inner = self.inner.lock();
        inner.client_count = inner.client_count.saturating_sub(1);
        inner.client_count
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.notify_tx.subscribe()
    }

    pub fn broadcast(&self, message: String) {
        let _ = self.notify_tx.send(message);
    }

    pub fn insert_pending_diff(&self, request_id: String, pending: PendingDiff) {
        self.inner.lock().pending_diffs.insert(request_id, pending);
    }

    pub fn take_pending_diff(&self, request_id: &str) -> Option<PendingDiff> {
        self.inner.lock().pending_diffs.remove(request_id)
    }

    pub fn insert_pending_save(&self, request_id: String, pending: PendingSave) {
        self.inner.lock().pending_saves.insert(request_id, pending);
    }

    pub fn take_pending_save(&self, request_id: &str) -> Option<PendingSave> {
        self.inner.lock().pending_saves.remove(request_id)
    }

    pub fn resolve_pending_for_missing_projects(&self, open_projects: &HashSet<ProjectId>) {
        let stale: Vec<PendingDiff> = {
            let mut inner = self.inner.lock();
            let stale_ids: Vec<String> = inner
                .pending_diffs
                .iter()
                .filter(|(_, pending)| !open_projects.contains(&pending.project_id))
                .map(|(id, _)| id.clone())
                .collect();
            stale_ids.into_iter().filter_map(|id| inner.pending_diffs.remove(&id)).collect()
        };
        for pending in stale {
            let _ = pending.responder.send((IdeDiffOutcome::TabClosed, None));
        }
    }

    pub fn set_selection(&self, selection: IdeSelectionSnapshot) {
        let mut inner = self.inner.lock();
        inner.current_selection = Some(selection.clone());
        if !selection.is_empty {
            inner.latest_selection = Some(selection);
        }
    }

    pub fn clear_selection(&self) {
        self.inner.lock().current_selection = None;
    }

    /// True when `owner` is not the remote session's fixed label — the gate
    /// [`ide_set_selection`]/[`ide_clear_selection`] use before touching [`IdeStore`]'s selection
    /// slots or broadcasting a `selection_changed` notification, so a remote browser session's
    /// editor selection can never be mistaken for the local desktop editor's by
    /// [`ide::server`](super::server)'s MCP `getCurrentSelection`/`getLatestSelection` tools (R6#12).
    fn is_desktop_owner(owner: &str) -> bool {
        owner != REMOTE_OWNER_LABEL
    }

    pub fn current_selection(&self) -> Option<IdeSelectionSnapshot> {
        self.inner.lock().current_selection.clone()
    }

    pub fn latest_selection(&self) -> Option<IdeSelectionSnapshot> {
        self.inner.lock().latest_selection.clone()
    }

    pub fn publish_diagnostics(&self, project_id: ProjectId, items: Vec<IdeDiagnostic>) {
        let mut inner = self.inner.lock();
        inner.diagnostics.insert(project_id, items);
        inner.diagnostics_ready = true;
    }

    pub fn diagnostics(&self, uri_path: Option<&str>) -> Option<Vec<IdeDiagnostic>> {
        let inner = self.inner.lock();
        if !inner.diagnostics_ready {
            return None;
        }
        let all = inner.diagnostics.values().flatten().cloned();
        match uri_path {
            Some(path) => Some(all.filter(|diagnostic| diagnostic.path == path).collect()),
            None => Some(all.collect()),
        }
    }
}

pub fn emit_status_changed(app: &AppHandle, client_count: u32) {
    let ide = app.state::<IdeStore>();
    let mut status = ide.status();
    status.client_count = client_count;
    status.connected = client_count > 0;
    let _ = IdeStatusChanged { status }.emit(app);
}

pub fn reconcile_closed_tab(app: &AppHandle, tab: &Tab) {
    let TabKind::ClaudeDiff { request_id, .. } = &tab.kind else {
        return;
    };
    let ide = app.state::<IdeStore>();
    if let Some(pending) = ide.take_pending_diff(request_id) {
        let _ = pending.responder.send((IdeDiffOutcome::TabClosed, None));
    }
}

pub fn stop_server(app: &AppHandle, ide: &IdeStore) {
    let Some(shutdown) = ide.take_shutdown_state() else {
        return;
    };

    if let Some(handle) = &shutdown.server_handle {
        handle.abort();
    }
    for handle in &shutdown.connection_handles {
        handle.abort();
    }
    if let Some(dir) = &shutdown.dir {
        if let Err(error) = lockfile::remove_lockfile(dir, shutdown.port) {
            log::warn!("IDE lockfile 삭제 실패: {error}");
        }
    }
    for pending in shutdown.pending_diffs {
        let _ = pending.responder.send((IdeDiffOutcome::Rejected, None));
    }
    for pending in shutdown.pending_saves {
        let _ = pending.responder.send(false);
    }

    let _ = IdeStatusChanged {
        status: IdeStatus::default(),
    }
    .emit(app);
}

/// lockfile 의 `workspaceFolders` 는 Claude Code 가 후보 IDE 를 판정하는 근거다.
/// 프로젝트가 열리거나 닫히면 즉시 다시 써서 기동 시점 스냅샷으로 굳지 않게 한다.
pub fn refresh_lockfile(app: &AppHandle) {
    let ide = app.state::<IdeStore>();
    let Some((port, token, dir)) = ide.lockfile_context() else {
        return;
    };
    let workspace_folders = service::workspace_folders(&app.state::<AppState>().projects.read());
    let content = lockfile::build_lockfile_content(std::process::id(), workspace_folders, token);
    if let Err(error) = lockfile::write_lockfile_atomic(&dir, port, &content) {
        log::warn!("IDE lockfile 갱신 실패: {error}");
    }
}

pub fn reconcile_stale_pending(app: &AppHandle) {
    let ide = app.state::<IdeStore>();
    if !ide.is_running() {
        return;
    }
    let state = app.state::<AppState>();
    let open_projects: HashSet<ProjectId> = state.projects.read().keys().cloned().collect();
    ide.resolve_pending_for_missing_projects(&open_projects);
}

async fn bind_and_start(app: &AppHandle) -> AppResult<IdeStatus> {
    let state = app.state::<AppState>();
    let workspace_folders = service::workspace_folders(&state.projects.read());
    let token = service::generate_auth_token();
    let dir = lockfile::lockfile_dir()?;
    let current_pid = std::process::id();

    match lockfile::cleanup_stale_lockfiles(&dir, current_pid) {
        Ok(0) => {}
        Ok(removed) => log::info!("정지된 IDE lockfile {removed}개 정리"),
        Err(error) => log::warn!("정지된 IDE lockfile 정리 실패: {error}"),
    }

    let mut last_error: Option<std::io::Error> = None;
    for _ in 0..IDE_PORT_BIND_MAX_ATTEMPTS {
        let candidate_port = service::random_port();
        match tokio::net::TcpListener::bind(("127.0.0.1", candidate_port as u16)).await {
            Ok(listener) => {
                let content = lockfile::build_lockfile_content(current_pid, workspace_folders, token.clone());
                lockfile::write_lockfile_atomic(&dir, candidate_port, &content)?;

                let app_for_loop = app.clone();
                let token_for_loop = token.clone();
                let server_handle = tauri::async_runtime::spawn(async move {
                    server::accept_loop(app_for_loop, listener, token_for_loop).await;
                });

                let ide = app.state::<IdeStore>();
                let status = ide.mark_started(candidate_port, token, dir.clone(), server_handle);
                log::info!("IDE 서버 기동: port={candidate_port}, lockfile={}", dir.display());
                let _ = IdeStatusChanged { status }.emit(app);
                return Ok(status);
            }
            Err(error) => last_error = Some(error),
        }
    }

    Err(AppError::Internal(format!(
        "IDE 서버 포트를 찾지 못했습니다: {}",
        last_error.map(|error| error.to_string()).unwrap_or_default()
    )))
}

#[tauri::command]
#[specta::specta]
pub async fn ide_get_status(ide: State<'_, IdeStore>) -> AppResult<IdeStatus> {
    Ok(ide.status())
}

/// Starts the embedded IDE MCP server if it isn't already running — no longer a `#[tauri::command]`
/// (X1#13, `docs/acknowledge/2026-08-19-xa-wiring-cleanup-contract.md` §1.2): the only reachable path
/// to it was already `settings_update`'s `ideIntegrationEnabled` toggle
/// (`settings::commands::apply_integration_toggles`) and this module's own boot-time auto-start
/// (`lib.rs`), so the separate `ide_start` IPC surface duplicated that without adding a distinct
/// capability. Kept as a plain internal function since both of those call sites still need it.
pub async fn ide_start(app: AppHandle, state: State<'_, AppState>, ide: State<'_, IdeStore>) -> AppResult<IdeStatus> {
    if ide.is_running() {
        return Ok(ide.status());
    }
    if !state.settings.read().ide_integration_enabled {
        return Err(AppError::InvalidArgument("IDE integration is disabled in settings".to_string()));
    }
    bind_and_start(&app).await
}

/// A remote session's `owner` (`IdeSelectionInput.owner == REMOTE_OWNER_LABEL`) is a deliberate
/// no-op: neither [`IdeStore::set_selection`] nor the `selection_changed` broadcast run, so a
/// browser-side editor selection can never overwrite what the local desktop's IDE MCP protocol
/// (`getCurrentSelection`/`getLatestSelection` in [`server`]) reports for the actual desktop editor
/// (R6#12).
#[tauri::command]
#[specta::specta]
pub async fn ide_set_selection(ide: State<'_, IdeStore>, input: IdeSelectionInput) -> AppResult<()> {
    if !IdeStore::is_desktop_owner(&input.owner) {
        return Ok(());
    }

    let selection = IdeSelectionSnapshot {
        project_id: input.project_id,
        path: input.path,
        text: input.text,
        start_line: input.start_line,
        start_character: input.start_character,
        end_line: input.end_line,
        end_character: input.end_character,
        is_empty: input.is_empty,
    };
    let notification = server::selection_changed_notification(&selection);
    ide.set_selection(selection);
    ide.broadcast(notification);
    Ok(())
}

/// See [`ide_set_selection`]'s doc comment for why a remote `owner` is a no-op here too.
#[tauri::command]
#[specta::specta]
pub async fn ide_clear_selection(ide: State<'_, IdeStore>, owner: String) -> AppResult<()> {
    if !IdeStore::is_desktop_owner(&owner) {
        return Ok(());
    }
    ide.clear_selection();
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn ide_publish_diagnostics(ide: State<'_, IdeStore>, project_id: ProjectId, items: Vec<IdeDiagnostic>) -> AppResult<()> {
    ide.publish_diagnostics(project_id, items);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn ide_resolve_diff(
    state: State<'_, AppState>,
    ide: State<'_, IdeStore>,
    request_id: String,
    outcome: IdeDiffOutcome,
    content: Option<String>,
) -> AppResult<()> {
    let pending = ide
        .take_pending_diff(&request_id)
        .ok_or_else(|| AppError::NotFound(format!("pending diff not found: {request_id}")))?;

    let resolved_content = match outcome {
        IdeDiffOutcome::Saved => {
            let content = content.ok_or_else(|| AppError::InvalidArgument("saved outcome requires content".to_string()))?;
            let projects = state.projects.read().clone();
            match service::ensure_path_within_any_project(&projects, &pending.new_path) {
                Ok((project_id, resolved_path)) => {
                    crate::domain::file::service::save_file(&resolved_path, &content)?;
                    crate::domain::file::service::clear_mirror(&state.paths, &project_id, &resolved_path)?;
                }
                Err(error) => {
                    log::warn!("IDE diff 저장 대상이 더 이상 프로젝트 루트 안에 있지 않습니다: {error}");
                }
            }
            Some(content)
        }
        IdeDiffOutcome::Rejected | IdeDiffOutcome::TabClosed => None,
    };

    let _ = pending.responder.send((outcome, resolved_content));
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn ide_resolve_save(ide: State<'_, IdeStore>, request_id: String, saved: bool) -> AppResult<()> {
    let pending = ide
        .take_pending_save(&request_id)
        .ok_or_else(|| AppError::NotFound(format!("pending save not found: {request_id}")))?;
    let _ = pending.responder.send(saved);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn ide_notify_at_mention(ide: State<'_, IdeStore>, path: String, line_start: u32, line_end: u32) -> AppResult<()> {
    ide.broadcast(server::at_mentioned_notification(&path, line_start, line_end));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_handle() -> tauri::async_runtime::JoinHandle<()> {
        tauri::async_runtime::spawn(async {})
    }

    #[test]
    fn 시작_전에는_상태가_비어있다() {
        let store = IdeStore::default();
        let status = store.status();
        assert!(!status.running);
        assert_eq!(status.port, 0);
    }

    #[test]
    fn mark_started는_실행_상태로_전환한다() {
        let store = IdeStore::default();
        let status = store.mark_started(51234, "token".to_string(), PathBuf::from("/tmp/ide"), dummy_handle());
        assert!(status.running);
        assert_eq!(status.port, 51234);
        assert!(store.is_running());
    }

    #[test]
    fn 실행중이_아니면_shutdown_상태가_없다() {
        let store = IdeStore::default();
        assert!(store.take_shutdown_state().is_none());
    }

    #[test]
    fn take_shutdown_state는_pending_diff와_save를_모두_드레인하고_해소된다() {
        let store = IdeStore::default();
        store.mark_started(51235, "token".to_string(), PathBuf::from("/tmp/ide"), dummy_handle());

        let (diff_tx, diff_rx) = oneshot::channel();
        store.insert_pending_diff(
            "req-1".to_string(),
            PendingDiff {
                project_id: ProjectId::from("prj-1".to_string()),
                new_path: PathBuf::from("/tmp/a.rs"),
                responder: diff_tx,
            },
        );
        let (save_tx, save_rx) = oneshot::channel();
        store.insert_pending_save("save-1".to_string(), PendingSave { responder: save_tx });

        let shutdown = store.take_shutdown_state().expect("실행 중이었으므로 존재해야 한다");
        assert_eq!(shutdown.pending_diffs.len(), 1);
        assert_eq!(shutdown.pending_saves.len(), 1);
        assert!(!store.is_running());

        for pending in shutdown.pending_diffs {
            let _ = pending.responder.send((IdeDiffOutcome::Rejected, None));
        }
        for pending in shutdown.pending_saves {
            let _ = pending.responder.send(false);
        }

        let (outcome, content) = tauri::async_runtime::block_on(diff_rx).unwrap();
        assert_eq!(outcome, IdeDiffOutcome::Rejected);
        assert!(content.is_none());
        assert!(!tauri::async_runtime::block_on(save_rx).unwrap());
    }

    #[test]
    fn 프로젝트가_닫히면_해당_pending_diff만_tabclosed로_해소된다() {
        let store = IdeStore::default();
        store.mark_started(51236, "token".to_string(), PathBuf::from("/tmp/ide"), dummy_handle());

        let open_project = ProjectId::from("open".to_string());
        let closed_project = ProjectId::from("closed".to_string());

        let (open_tx, mut open_rx) = oneshot::channel();
        store.insert_pending_diff(
            "open-req".to_string(),
            PendingDiff {
                project_id: open_project.clone(),
                new_path: PathBuf::from("/a"),
                responder: open_tx,
            },
        );
        let (closed_tx, closed_rx) = oneshot::channel();
        store.insert_pending_diff(
            "closed-req".to_string(),
            PendingDiff {
                project_id: closed_project,
                new_path: PathBuf::from("/b"),
                responder: closed_tx,
            },
        );

        let still_open: HashSet<ProjectId> = HashSet::from([open_project]);
        store.resolve_pending_for_missing_projects(&still_open);

        let (outcome, _) = tauri::async_runtime::block_on(closed_rx).unwrap();
        assert_eq!(outcome, IdeDiffOutcome::TabClosed);
        assert!(open_rx.try_recv().is_err());

        let shutdown = store.take_shutdown_state().unwrap();
        assert_eq!(shutdown.pending_diffs.len(), 1);
    }

    #[test]
    fn 선택영역은_비어있지_않을_때만_latest로_남는다() {
        let store = IdeStore::default();
        let selection = IdeSelectionSnapshot {
            project_id: ProjectId::from("prj-1".to_string()),
            path: "/a.rs".to_string(),
            text: "hello".to_string(),
            start_line: 0,
            start_character: 0,
            end_line: 0,
            end_character: 5,
            is_empty: false,
        };
        store.set_selection(selection.clone());
        assert_eq!(store.current_selection(), Some(selection.clone()));
        assert_eq!(store.latest_selection(), Some(selection.clone()));

        let empty_selection = IdeSelectionSnapshot {
            is_empty: true,
            text: String::new(),
            ..selection
        };
        store.set_selection(empty_selection.clone());
        assert_eq!(store.current_selection(), Some(empty_selection));
        assert_eq!(store.latest_selection().unwrap().text, "hello");

        store.clear_selection();
        assert!(store.current_selection().is_none());
        assert!(store.latest_selection().is_some());
    }

    /// R6#12 회귀: `ide_set_selection`/`ide_clear_selection` 은 `IdeStore::is_desktop_owner`
    /// 로 원격 세션의 고정 owner(`REMOTE_OWNER_LABEL`)를 걸러낸 뒤에만 스토어에 쓴다 — 이
    /// 판정이 뒤집히면 원격 세션의 선택영역이 데스크톱 IDE MCP 프로토콜(`getCurrentSelection`)
    /// 이 보고하는 상태를 오염시킨다.
    #[test]
    fn is_desktop_owner는_원격_고정_라벨만_거부한다() {
        assert!(!IdeStore::is_desktop_owner(REMOTE_OWNER_LABEL));
        assert!(IdeStore::is_desktop_owner("main"));
        assert!(IdeStore::is_desktop_owner("editor-2"));
    }

    #[test]
    fn 진단은_한_번도_push되지_않으면_none이다() {
        let store = IdeStore::default();
        assert!(store.diagnostics(None).is_none());
    }

    #[test]
    fn 진단은_push된_이후_빈_결과와_준비안됨을_구분한다() {
        let store = IdeStore::default();
        store.publish_diagnostics(ProjectId::from("prj-1".to_string()), Vec::new());
        assert_eq!(store.diagnostics(None), Some(Vec::new()));
    }

    #[test]
    fn 클라이언트_연결_해제_카운트가_오간다() {
        let store = IdeStore::default();
        assert_eq!(store.client_connected(), 1);
        assert_eq!(store.client_connected(), 2);
        assert_eq!(store.client_disconnected(), 1);
        assert_eq!(store.client_disconnected(), 0);
        assert_eq!(store.client_disconnected(), 0);
    }
}

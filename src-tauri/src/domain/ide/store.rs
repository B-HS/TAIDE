use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use parking_lot::Mutex;
use tauri::{AppHandle, Manager};
use tokio::sync::{broadcast, oneshot};

use super::types::{IdeDiagnostic, IdeDiffOutcome, IdeStatus};
use crate::domain::layout::types::{Tab, TabKind};
use crate::domain::remote::types::REMOTE_OWNER_LABEL;
use crate::ids::ProjectId;

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
    /// [`ide_set_selection`](super::commands::ide_set_selection)/[`ide_clear_selection`](super::commands) use before touching
    /// [`IdeStore`]'s selection slots or broadcasting a `selection_changed` notification, so a
    /// remote browser session's editor selection can never be mistaken for the local desktop
    /// editor's by [`ide::server`](super::server)'s MCP
    /// `getCurrentSelection`/`getLatestSelection` tools (R6#12).
    pub(super) fn is_desktop_owner(owner: &str) -> bool {
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

pub fn reconcile_closed_tab(app: &AppHandle, tab: &Tab) {
    let TabKind::ClaudeDiff { request_id, .. } = &tab.kind else {
        return;
    };
    let ide = app.state::<IdeStore>();
    if let Some(pending) = ide.take_pending_diff(request_id) {
        let _ = pending.responder.send((IdeDiffOutcome::TabClosed, None));
    }
}

/// `true` while a spawner should keep polling for the IDE server before injecting its env —
/// only when the integration is enabled but the server hasn't come up yet.
fn should_wait_for_ide_ready(ide_integration_enabled: bool, ide_running: bool) -> bool {
    ide_integration_enabled && !ide_running
}

/// The Claude Code environment entries a newly spawned terminal should inherit — the SSE port of
/// the running IDE server, or nothing when it isn't running. Waits up to
/// [`super::types::IDE_READY_WAIT_MS`] (polling every [`super::types::IDE_READY_POLL_INTERVAL_MS`])
/// only while the integration is enabled but the server hasn't bound yet, so a terminal opened
/// right after app boot still gets the port. Registered by `lib.rs`'s assembly as the
/// `terminal::commands::PtySpawnEnvProvider` — the IDE domain owns its readiness semantics, the
/// terminal domain just injects whatever env the provider hands back (audit R8#10, T1-I §1.4).
pub async fn claude_terminal_env(app: &AppHandle) -> Vec<(String, String)> {
    use super::types::{CLAUDE_CODE_SSE_PORT_ENV, IDE_READY_POLL_INTERVAL_MS, IDE_READY_WAIT_MS};

    let ide_integration_enabled = app.state::<crate::state::AppState>().settings.read().ide_integration_enabled;
    let ide = app.state::<IdeStore>();
    let mut status = ide.status();

    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(IDE_READY_WAIT_MS);
    while should_wait_for_ide_ready(ide_integration_enabled, status.running) && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(tokio::time::Duration::from_millis(IDE_READY_POLL_INTERVAL_MS)).await;
        status = ide.status();
    }

    if status.running {
        vec![(CLAUDE_CODE_SSE_PORT_ENV.to_string(), status.port.to_string())]
    } else {
        Vec::new()
    }
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

    #[test]
    fn ide_연동_꺼져있으면_기동_여부와_무관하게_대기하지_않는다() {
        assert!(!should_wait_for_ide_ready(false, false));
        assert!(!should_wait_for_ide_ready(false, true));
    }

    #[test]
    fn ide_연동_켜져있고_이미_기동됐으면_대기하지_않는다() {
        assert!(!should_wait_for_ide_ready(true, true));
    }

    #[test]
    fn ide_연동_켜져있고_아직_기동_전이면_대기한다() {
        assert!(should_wait_for_ide_ready(true, false));
    }
}

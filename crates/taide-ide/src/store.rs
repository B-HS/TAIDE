use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use taide_model::ide::{IdeDiagnostic, IdeDiffOutcome, IdeStatus};
use taide_model::ids::ProjectId;
use taide_model::layout::{Tab, TabKind};
use taide_model::remote::REMOTE_OWNER_LABEL;
use tokio::sync::{broadcast, oneshot};

pub use crate::protocol::IdeSelectionSnapshot;

const NOTIFY_CHANNEL_CAPACITY: usize = 64;

pub struct PendingDiff {
    pub project_id: ProjectId,
    pub new_path: PathBuf,
    pub responder: oneshot::Sender<(IdeDiffOutcome, Option<String>)>,
}

pub struct PendingSave {
    pub responder: oneshot::Sender<bool>,
}

enum PendingRequestKind {
    Diff,
    Save,
}

#[must_use]
pub struct PendingRequestOwner {
    store: IdeStore,
    request_id: String,
    kind: PendingRequestKind,
}

impl Drop for PendingRequestOwner {
    fn drop(&mut self) {
        match self.kind {
            PendingRequestKind::Diff => {
                self.store.take_pending_diff(&self.request_id);
            }
            PendingRequestKind::Save => {
                self.store.take_pending_save(&self.request_id);
            }
        }
    }
}

#[derive(Default)]
struct IdeStoreInner {
    running: bool,
    port: u32,
    token: String,
    lockfile_dir: Option<PathBuf>,
    client_count: u32,
    server_handle: Option<tokio::task::JoinHandle<()>>,
    connection_handles: Vec<tokio::task::JoinHandle<()>>,
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
    pub server_handle: Option<tokio::task::JoinHandle<()>>,
    pub connection_handles: Vec<tokio::task::JoinHandle<()>>,
    pub pending_diffs: Vec<PendingDiff>,
    pub pending_saves: Vec<PendingSave>,
}

#[derive(Clone)]
pub struct IdeStore {
    inner: Arc<Mutex<IdeStoreInner>>,
    notify_tx: broadcast::Sender<String>,
}

impl Default for IdeStore {
    fn default() -> Self {
        let (notify_tx, _rx) = broadcast::channel(NOTIFY_CHANNEL_CAPACITY);
        Self { inner: Arc::new(Mutex::new(IdeStoreInner::default())), notify_tx }
    }
}

impl IdeStore {
    pub fn status(&self) -> IdeStatus {
        let inner = self.inner.lock();
        IdeStatus { running: inner.running, port: inner.port, connected: inner.client_count > 0, client_count: inner.client_count }
    }

    pub fn is_running(&self) -> bool {
        self.inner.lock().running
    }

    pub fn mark_started(&self, port: u32, token: String, dir: PathBuf, server_handle: tokio::task::JoinHandle<()>) -> Option<IdeStatus> {
        let mut inner = self.inner.lock();
        if inner.running {
            server_handle.abort();
            return None;
        }
        inner.running = true;
        inner.port = port;
        inner.token = token;
        inner.lockfile_dir = Some(dir);
        inner.server_handle = Some(server_handle);
        inner.client_count = 0;
        Some(IdeStatus { running: true, port, connected: false, client_count: 0 })
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

    pub fn register_connection(&self, handle: tokio::task::JoinHandle<()>) -> bool {
        let mut inner = self.inner.lock();
        if !inner.running {
            handle.abort();
            return false;
        }
        inner.connection_handles.retain(|existing| !existing.is_finished());
        inner.connection_handles.push(handle);
        true
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

    pub fn client_connected_for_token(&self, token: &str) -> Option<u32> {
        let mut inner = self.inner.lock();
        if !inner.running || inner.token != token {
            return None;
        }
        inner.client_count += 1;
        Some(inner.client_count)
    }

    pub fn client_disconnected_for_token(&self, token: &str) -> Option<u32> {
        let mut inner = self.inner.lock();
        if !inner.running || inner.token != token {
            return None;
        }
        inner.client_count = inner.client_count.saturating_sub(1);
        Some(inner.client_count)
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

    pub fn insert_pending_diff_owned(&self, request_id: String, pending: PendingDiff) -> PendingRequestOwner {
        self.insert_pending_diff(request_id.clone(), pending);
        PendingRequestOwner { store: self.clone(), request_id, kind: PendingRequestKind::Diff }
    }

    pub fn take_pending_diff(&self, request_id: &str) -> Option<PendingDiff> {
        self.inner.lock().pending_diffs.remove(request_id)
    }

    pub fn reconcile_closed_tab(&self, tab: &Tab) {
        let TabKind::ClaudeDiff { request_id, .. } = &tab.kind else {
            return;
        };
        if let Some(pending) = self.take_pending_diff(request_id) {
            let _ = pending.responder.send((IdeDiffOutcome::TabClosed, None));
        }
    }

    pub fn insert_pending_save(&self, request_id: String, pending: PendingSave) {
        self.inner.lock().pending_saves.insert(request_id, pending);
    }

    pub fn insert_pending_save_owned(&self, request_id: String, pending: PendingSave) -> PendingRequestOwner {
        self.insert_pending_save(request_id.clone(), pending);
        PendingRequestOwner { store: self.clone(), request_id, kind: PendingRequestKind::Save }
    }

    pub fn take_pending_save(&self, request_id: &str) -> Option<PendingSave> {
        self.inner.lock().pending_saves.remove(request_id)
    }

    pub fn resolve_pending_for_missing_projects(&self, open_projects: &HashSet<ProjectId>) {
        let stale: Vec<PendingDiff> = {
            let mut inner = self.inner.lock();
            let stale_ids: Vec<String> =
                inner.pending_diffs.iter().filter(|(_, pending)| !open_projects.contains(&pending.project_id)).map(|(id, _)| id.clone()).collect();
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
    /// `ide_set_selection`/`ide_clear_selection` use before touching
    /// [`IdeStore`]'s selection slots or broadcasting a `selection_changed` notification, so a
    /// remote browser session's editor selection can never be mistaken for the local desktop
    /// editor's by the IDE server's MCP
    /// `getCurrentSelection`/`getLatestSelection` tools (R6#12).
    pub fn is_desktop_owner(owner: &str) -> bool {
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

/// `true` while a spawner should keep polling for the IDE server before injecting its env —
/// only when the integration is enabled but the server hasn't come up yet.
pub fn should_wait_for_ide_ready(ide_integration_enabled: bool, ide_running: bool) -> bool {
    ide_integration_enabled && !ide_running
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SERVER_PORT: u32 = 51_234;

    #[test]
    fn 시작_전에는_상태가_비어있다() {
        let store = IdeStore::default();
        let status = store.status();
        assert!(!status.running);
        assert_eq!(status.port, 0);
    }

    #[tokio::test]
    async fn mark_started는_실행_상태로_전환한다() {
        let store = IdeStore::default();
        let status = store.mark_started(TEST_SERVER_PORT, "token".to_string(), PathBuf::from("/tmp/ide"), tokio::spawn(async {})).unwrap();
        assert!(status.running);
        assert_eq!(status.port, TEST_SERVER_PORT);
        assert!(store.is_running());
    }

    #[test]
    fn 실행중이_아니면_shutdown_상태가_없다() {
        let store = IdeStore::default();
        assert!(store.take_shutdown_state().is_none());
    }

    #[tokio::test]
    async fn take_shutdown_state는_pending_diff와_save를_모두_드레인하고_해소된다() {
        let store = IdeStore::default();
        assert!(store.mark_started(TEST_SERVER_PORT, "token".to_string(), PathBuf::from("/tmp/ide"), tokio::spawn(async {})).is_some());

        let (diff_tx, diff_rx) = oneshot::channel();
        store.insert_pending_diff(
            "req-1".to_string(),
            PendingDiff { project_id: ProjectId::from("prj-1".to_string()), new_path: PathBuf::from("/tmp/a.rs"), responder: diff_tx },
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

        let (outcome, content) = diff_rx.await.unwrap();
        assert_eq!(outcome, IdeDiffOutcome::Rejected);
        assert!(content.is_none());
        assert!(!save_rx.await.unwrap());
    }

    #[tokio::test]
    async fn 취소된_요청의_pending_diff만_제거한다() {
        let store = IdeStore::default();
        let (cancelled_tx, cancelled_rx) = oneshot::channel();
        let owner = store.insert_pending_diff_owned(
            "cancelled-diff".to_string(),
            PendingDiff { project_id: ProjectId::from("project".to_string()), new_path: PathBuf::from("/tmp/cancelled.rs"), responder: cancelled_tx },
        );
        let (remaining_tx, remaining_rx) = oneshot::channel();
        store.insert_pending_diff(
            "remaining-diff".to_string(),
            PendingDiff { project_id: ProjectId::from("project".to_string()), new_path: PathBuf::from("/tmp/remaining.rs"), responder: remaining_tx },
        );

        let task = tokio::spawn(async move {
            let _owner = owner;
            std::future::pending::<()>().await;
        });
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(store.take_pending_diff("cancelled-diff").is_none());
        assert!(cancelled_rx.await.is_err());
        assert!(store.take_pending_diff("remaining-diff").is_some());
        assert!(remaining_rx.await.is_err());
    }

    #[tokio::test]
    async fn 취소된_요청의_pending_save만_제거한다() {
        let store = IdeStore::default();
        let (cancelled_tx, cancelled_rx) = oneshot::channel();
        let owner = store.insert_pending_save_owned("cancelled-save".to_string(), PendingSave { responder: cancelled_tx });
        let (remaining_tx, remaining_rx) = oneshot::channel();
        store.insert_pending_save("remaining-save".to_string(), PendingSave { responder: remaining_tx });

        let task = tokio::spawn(async move {
            let _owner = owner;
            std::future::pending::<()>().await;
        });
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(store.take_pending_save("cancelled-save").is_none());
        assert!(cancelled_rx.await.is_err());
        assert!(store.take_pending_save("remaining-save").is_some());
        assert!(remaining_rx.await.is_err());
    }

    #[tokio::test]
    async fn 정상_응답을_받은_요청은_owner_종료에도_결과를_유지한다() {
        let store = IdeStore::default();
        let (diff_tx, diff_rx) = oneshot::channel();
        let diff_owner = store.insert_pending_diff_owned(
            "resolved-diff".to_string(),
            PendingDiff { project_id: ProjectId::from("project".to_string()), new_path: PathBuf::from("/tmp/resolved.rs"), responder: diff_tx },
        );
        let (save_tx, save_rx) = oneshot::channel();
        let save_owner = store.insert_pending_save_owned("resolved-save".to_string(), PendingSave { responder: save_tx });

        store.take_pending_diff("resolved-diff").unwrap().responder.send((IdeDiffOutcome::Saved, None)).unwrap();
        store.take_pending_save("resolved-save").unwrap().responder.send(true).unwrap();
        drop(diff_owner);
        drop(save_owner);

        assert_eq!(diff_rx.await.unwrap(), (IdeDiffOutcome::Saved, None));
        assert!(save_rx.await.unwrap());
    }

    #[tokio::test]
    async fn 전체_종료가_회수한_응답은_요청_owner_종료에도_유지된다() {
        let store = IdeStore::default();
        assert!(store.mark_started(TEST_SERVER_PORT, "token".to_string(), PathBuf::from("/tmp/ide"), tokio::spawn(async {})).is_some());

        let (diff_tx, diff_rx) = oneshot::channel();
        let diff_owner = store.insert_pending_diff_owned(
            "shutdown-diff".to_string(),
            PendingDiff { project_id: ProjectId::from("project".to_string()), new_path: PathBuf::from("/tmp/shutdown.rs"), responder: diff_tx },
        );
        let (save_tx, save_rx) = oneshot::channel();
        let save_owner = store.insert_pending_save_owned("shutdown-save".to_string(), PendingSave { responder: save_tx });

        let shutdown = store.take_shutdown_state().unwrap();
        drop(diff_owner);
        drop(save_owner);
        assert_eq!(shutdown.pending_diffs.len(), 1);
        assert_eq!(shutdown.pending_saves.len(), 1);

        for pending in shutdown.pending_diffs {
            pending.responder.send((IdeDiffOutcome::Rejected, None)).unwrap();
        }
        for pending in shutdown.pending_saves {
            pending.responder.send(false).unwrap();
        }
        assert_eq!(diff_rx.await.unwrap(), (IdeDiffOutcome::Rejected, None));
        assert!(!save_rx.await.unwrap());
    }

    #[tokio::test]
    async fn 닫힌_claude_diff만_pending_응답을_tabclosed로_해소한다() {
        let store = IdeStore::default();
        let project_id = ProjectId::from("project".to_string());
        let (diff_sender, diff_receiver) = oneshot::channel();
        let (file_sender, _file_receiver) = oneshot::channel();
        store.insert_pending_diff(
            "diff-request".to_string(),
            PendingDiff { project_id: project_id.clone(), new_path: PathBuf::from("/tmp/diff.rs"), responder: diff_sender },
        );
        store.insert_pending_diff(
            "file-request".to_string(),
            PendingDiff { project_id, new_path: PathBuf::from("/tmp/file.rs"), responder: file_sender },
        );

        let file_tab = Tab {
            id: taide_model::ids::TabId::new(),
            kind: TabKind::File { path: "/tmp/file.rs".to_string() },
            title: "file".to_string(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        };
        store.reconcile_closed_tab(&file_tab);
        assert!(store.take_pending_diff("file-request").is_some());

        let diff_tab = Tab { kind: TabKind::ClaudeDiff { request_id: "diff-request".to_string(), path: "/tmp/diff.rs".to_string() }, ..file_tab };
        store.reconcile_closed_tab(&diff_tab);
        store.reconcile_closed_tab(&diff_tab);

        assert_eq!(diff_receiver.await.expect("응답 수신"), (IdeDiffOutcome::TabClosed, None));
        assert!(store.take_pending_diff("diff-request").is_none());
    }

    #[tokio::test]
    async fn 프로젝트가_닫히면_해당_pending_diff만_tabclosed로_해소된다() {
        let store = IdeStore::default();
        assert!(store.mark_started(TEST_SERVER_PORT, "token".to_string(), PathBuf::from("/tmp/ide"), tokio::spawn(async {})).is_some());

        let open_project = ProjectId::from("open".to_string());
        let closed_project = ProjectId::from("closed".to_string());

        let (open_tx, mut open_rx) = oneshot::channel();
        store.insert_pending_diff(
            "open-req".to_string(),
            PendingDiff { project_id: open_project.clone(), new_path: PathBuf::from("/a"), responder: open_tx },
        );
        let (closed_tx, closed_rx) = oneshot::channel();
        store.insert_pending_diff(
            "closed-req".to_string(),
            PendingDiff { project_id: closed_project, new_path: PathBuf::from("/b"), responder: closed_tx },
        );

        let still_open: HashSet<ProjectId> = HashSet::from([open_project]);
        store.resolve_pending_for_missing_projects(&still_open);

        let (outcome, _) = closed_rx.await.unwrap();
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

        let empty_selection = IdeSelectionSnapshot { is_empty: true, text: String::new(), ..selection };
        store.set_selection(empty_selection.clone());
        assert_eq!(store.current_selection(), Some(empty_selection));
        assert_eq!(store.latest_selection().unwrap().text, "hello");

        store.clear_selection();
        assert!(store.current_selection().is_none());
        assert!(store.latest_selection().is_some());
    }

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

    #[tokio::test]
    async fn 복제본은_진단_pending_응답과_알림_채널을_공유한다() {
        let store = IdeStore::default();
        let clone = store.clone();
        let project = ProjectId::new();
        let mut notifications = store.subscribe();
        let (sender, receiver) = oneshot::channel();
        clone.publish_diagnostics(project, Vec::new());
        assert_eq!(store.diagnostics(None), Some(Vec::new()));
        clone.insert_pending_save("save".to_string(), PendingSave { responder: sender });
        store.take_pending_save("save").expect("공유 pending save").responder.send(true).expect("응답 수신자");
        assert!(receiver.await.expect("공유 pending 응답"));

        clone.broadcast("notification".to_string());
        assert_eq!(notifications.recv().await.expect("공유 알림"), "notification");
        drop(store);
        clone.broadcast("after-drop".to_string());
        assert_eq!(notifications.recv().await.expect("복제본이 채널 수명 유지"), "after-drop");
    }
}

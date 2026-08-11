use std::collections::HashSet;

use parking_lot::Mutex;
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;
use tokio::sync::{broadcast, watch};

use super::server;
use super::service;
use super::types::{
    RemoteLinkInfo, RemoteStatus, REMOTE_BROADCAST_CHANNEL_CAPACITY, REMOTE_LINK_TOKEN_QUERY_KEY, REMOTE_SHUTDOWN_GRACE_MS,
};
use crate::error::{AppError, AppResult};
use crate::events::RemoteStateChanged;

#[derive(Default)]
struct RemoteStoreInner {
    running: bool,
    port: u32,
    client_count: u32,
    pending_link_token_digest: Option<Vec<u8>>,
    session_token_digests: HashSet<String>,
    server_handle: Option<tauri::async_runtime::JoinHandle<()>>,
    shutdown_tx: Option<watch::Sender<()>>,
}

pub struct RemoteShutdownState {
    pub port: u32,
    pub server_handle: Option<tauri::async_runtime::JoinHandle<()>>,
    pub shutdown_tx: Option<watch::Sender<()>>,
}

pub struct RemoteStore {
    inner: Mutex<RemoteStoreInner>,
    event_tx: broadcast::Sender<String>,
}

impl Default for RemoteStore {
    fn default() -> Self {
        let (event_tx, _rx) = broadcast::channel(REMOTE_BROADCAST_CHANNEL_CAPACITY);
        Self {
            inner: Mutex::new(RemoteStoreInner::default()),
            event_tx,
        }
    }
}

impl RemoteStore {
    pub fn status(&self) -> RemoteStatus {
        let inner = self.inner.lock();
        RemoteStatus {
            running: inner.running,
            port: inner.port,
            client_count: inner.client_count,
        }
    }

    pub fn is_running(&self) -> bool {
        self.inner.lock().running
    }

    pub fn mark_started(
        &self,
        port: u32,
        shutdown_tx: watch::Sender<()>,
        server_handle: tauri::async_runtime::JoinHandle<()>,
    ) -> RemoteStatus {
        let mut inner = self.inner.lock();
        inner.running = true;
        inner.port = port;
        inner.client_count = 0;
        inner.shutdown_tx = Some(shutdown_tx);
        inner.server_handle = Some(server_handle);
        RemoteStatus {
            running: true,
            port,
            client_count: 0,
        }
    }

    pub fn take_shutdown_state(&self) -> Option<RemoteShutdownState> {
        let mut inner = self.inner.lock();
        if !inner.running {
            return None;
        }
        inner.running = false;
        let port = inner.port;
        inner.port = 0;
        inner.client_count = 0;
        inner.pending_link_token_digest = None;
        inner.session_token_digests.clear();

        Some(RemoteShutdownState {
            port,
            server_handle: inner.server_handle.take(),
            shutdown_tx: inner.shutdown_tx.take(),
        })
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

    pub fn issue_link_token(&self) -> String {
        let token = service::generate_link_token();
        self.inner.lock().pending_link_token_digest = Some(service::digest_bytes(&token));
        token
    }

    pub fn consume_link_token(&self, candidate: &str) -> Option<String> {
        let mut inner = self.inner.lock();
        let expected_digest = inner.pending_link_token_digest.clone()?;
        let candidate_digest = service::digest_bytes(candidate);
        if !crate::infra::crypto::constant_time_eq(&candidate_digest, &expected_digest) {
            return None;
        }
        inner.pending_link_token_digest = None;
        let session_token = service::generate_session_token();
        inner.session_token_digests.insert(service::digest_hex(&session_token));
        Some(session_token)
    }

    pub fn has_active_session(&self, candidate: &str) -> bool {
        self.inner.lock().session_token_digests.contains(&service::digest_hex(candidate))
    }

    pub fn revoke_all_sessions(&self) {
        self.inner.lock().session_token_digests.clear();
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<String> {
        self.event_tx.subscribe()
    }

    pub fn has_event_subscribers(&self) -> bool {
        self.event_tx.receiver_count() > 0
    }

    pub fn broadcast_event(&self, frame: String) {
        let _ = self.event_tx.send(frame);
    }
}

async fn bind_and_start(app: &AppHandle) -> AppResult<RemoteStatus> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0u16)).await.map_err(AppError::from)?;
    let port = listener.local_addr().map_err(AppError::from)?.port() as u32;

    let (shutdown_tx, shutdown_rx) = watch::channel(());
    let router = server::build_router(app.clone());
    let server_handle = tauri::async_runtime::spawn(async move {
        server::serve(listener, router, shutdown_rx).await;
    });

    let remote = app.state::<RemoteStore>();
    let status = remote.mark_started(port, shutdown_tx, server_handle);
    log::info!("원격 접속 서버 기동: port={port}");
    let _ = RemoteStateChanged { status }.emit(app);
    Ok(status)
}

pub fn stop_server(app: &AppHandle, remote: &RemoteStore) {
    let Some(mut shutdown) = remote.take_shutdown_state() else {
        return;
    };

    if let Some(shutdown_tx) = shutdown.shutdown_tx.take() {
        let _ = shutdown_tx.send(());
    }
    if let Some(mut handle) = shutdown.server_handle.take() {
        tauri::async_runtime::spawn(async move {
            tokio::select! {
                _ = &mut handle => {}
                _ = tokio::time::sleep(std::time::Duration::from_millis(REMOTE_SHUTDOWN_GRACE_MS)) => {
                    handle.abort();
                }
            }
        });
    }

    let _ = RemoteStateChanged {
        status: RemoteStatus::default(),
    }
    .emit(app);
}

#[tauri::command]
#[specta::specta]
pub async fn remote_status(remote: State<'_, RemoteStore>) -> AppResult<RemoteStatus> {
    Ok(remote.status())
}

#[tauri::command]
#[specta::specta]
pub async fn remote_start(app: AppHandle, remote: State<'_, RemoteStore>) -> AppResult<RemoteStatus> {
    if remote.is_running() {
        return Ok(remote.status());
    }
    bind_and_start(&app).await
}

#[tauri::command]
#[specta::specta]
pub async fn remote_stop(app: AppHandle, remote: State<'_, RemoteStore>) -> AppResult<()> {
    stop_server(&app, &remote);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn remote_issue_link(remote: State<'_, RemoteStore>) -> AppResult<RemoteLinkInfo> {
    if !remote.is_running() {
        return Err(AppError::InvalidArgument("원격 접속 서버가 실행 중이 아닙니다".to_string()));
    }
    let token = remote.issue_link_token();
    let port = remote.status().port;
    let url = format!("http://127.0.0.1:{port}/?{REMOTE_LINK_TOKEN_QUERY_KEY}={token}");
    Ok(RemoteLinkInfo { url })
}

#[tauri::command]
#[specta::specta]
pub async fn remote_revoke_sessions(remote: State<'_, RemoteStore>) -> AppResult<()> {
    remote.revoke_all_sessions();
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
        let store = RemoteStore::default();
        let status = store.status();
        assert!(!status.running);
        assert_eq!(status.port, 0);
    }

    #[test]
    fn mark_started는_실행_상태로_전환한다() {
        let store = RemoteStore::default();
        let (shutdown_tx, _rx) = watch::channel(());
        let status = store.mark_started(51_300, shutdown_tx, dummy_handle());
        assert!(status.running);
        assert_eq!(status.port, 51_300);
        assert!(store.is_running());
    }

    #[test]
    fn 실행중이_아니면_shutdown_상태가_없다() {
        let store = RemoteStore::default();
        assert!(store.take_shutdown_state().is_none());
    }

    #[test]
    fn 올바른_링크_토큰만_세션으로_승격된다() {
        let store = RemoteStore::default();
        let token = store.issue_link_token();

        assert!(store.consume_link_token("wrong-token").is_none());

        let session = store.consume_link_token(&token).expect("올바른 토큰은 세션을 발급해야 한다");
        assert!(store.has_active_session(&session));
    }

    #[test]
    fn 링크_토큰은_한_번만_소모된다() {
        let store = RemoteStore::default();
        let token = store.issue_link_token();

        assert!(store.consume_link_token(&token).is_some());
        assert!(store.consume_link_token(&token).is_none());
    }

    #[test]
    fn 세션을_전체_해제하면_더_이상_유효하지_않다() {
        let store = RemoteStore::default();
        let token = store.issue_link_token();
        let session = store.consume_link_token(&token).expect("세션 발급");

        store.revoke_all_sessions();

        assert!(!store.has_active_session(&session));
    }

    #[test]
    fn 클라이언트_연결_해제_카운트가_오간다() {
        let store = RemoteStore::default();
        assert_eq!(store.client_connected(), 1);
        assert_eq!(store.client_connected(), 2);
        assert_eq!(store.client_disconnected(), 1);
        assert_eq!(store.client_disconnected(), 0);
        assert_eq!(store.client_disconnected(), 0);
    }
}

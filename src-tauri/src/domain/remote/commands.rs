use std::collections::HashMap;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;
use tokio::sync::{broadcast, watch};

use super::server;
use super::service;
use super::types::{
    RemoteLinkInfo, RemoteStatus, REMOTE_BROADCAST_CHANNEL_CAPACITY, REMOTE_LOGIN_LOCKOUT_BASE_MS, REMOTE_LOGIN_LOCKOUT_MAX_MS,
    REMOTE_LOGIN_MAX_ATTEMPTS, REMOTE_LOGIN_NONCE_TTL_MS, REMOTE_PASSWORD_MIN_LEN, REMOTE_SESSION_TTL_MS, REMOTE_SHUTDOWN_GRACE_MS,
};
use crate::error::{AppError, AppResult};
use crate::events::RemoteStateChanged;
use crate::infra::crypto::constant_time_eq;
use crate::infra::secret::{SecretAccount, SecretStoreState};
use crate::state::AppState;

#[derive(Default)]
struct RemoteStoreInner {
    running: bool,
    port: u32,
    client_count: u32,
    password_configured: bool,
    pending_link_token_digest: Option<Vec<u8>>,
    pending_login_nonces: HashMap<String, Instant>,
    session_token_digests: HashMap<String, Instant>,
    failed_login_attempts_nonce: u32,
    login_locked_until_nonce: Option<Instant>,
    failed_login_attempts_anonymous: u32,
    login_locked_until_anonymous: Option<Instant>,
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
    session_epoch_tx: watch::Sender<u64>,
}

impl Default for RemoteStore {
    fn default() -> Self {
        let (event_tx, _rx) = broadcast::channel(REMOTE_BROADCAST_CHANNEL_CAPACITY);
        let (session_epoch_tx, _rx) = watch::channel(0u64);
        Self {
            inner: Mutex::new(RemoteStoreInner::default()),
            event_tx,
            session_epoch_tx,
        }
    }
}

impl RemoteStore {
    /// Reports the `password_configured` value last cached via
    /// `set_password_configured` rather than reading the OS keyring on every
    /// call. `remote_status` is polled by the settings UI every
    /// `REMOTE_STATUS_REFETCH_MS` (5s) — a keyring round-trip on each tick is
    /// wasteful and, on backends that can prompt for access (e.g. a locked
    /// Linux Secret Service), can make the status poll itself slow or fail.
    /// The cache is refreshed once at app boot and again on every
    /// `remote_set_password`/`remote_clear_password` call — the only two
    /// points that can actually change the keyring entry from within this app.
    pub fn status(&self) -> RemoteStatus {
        let inner = self.inner.lock();
        RemoteStatus {
            running: inner.running,
            port: inner.port,
            client_count: inner.client_count,
            password_configured: inner.password_configured,
        }
    }

    pub fn set_password_configured(&self, configured: bool) {
        self.inner.lock().password_configured = configured;
    }

    pub fn port(&self) -> u32 {
        self.inner.lock().port
    }

    pub fn is_running(&self) -> bool {
        self.inner.lock().running
    }

    pub fn mark_started(&self, port: u32, shutdown_tx: watch::Sender<()>, server_handle: tauri::async_runtime::JoinHandle<()>) {
        let mut inner = self.inner.lock();
        inner.running = true;
        inner.port = port;
        inner.client_count = 0;
        inner.shutdown_tx = Some(shutdown_tx);
        inner.server_handle = Some(server_handle);
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
        inner.pending_login_nonces.clear();
        inner.session_token_digests.clear();

        let shutdown_state = RemoteShutdownState {
            port,
            server_handle: inner.server_handle.take(),
            shutdown_tx: inner.shutdown_tx.take(),
        };
        drop(inner);
        self.bump_session_epoch();
        Some(shutdown_state)
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

    /// Verifies and consumes the one-time link token (single-use, as before).
    /// On success this no longer issues a session directly — it mints a
    /// short-lived login nonce (`REMOTE_LOGIN_NONCE_TTL_MS`) that grants a
    /// single visit to the login form. When no password is configured, the
    /// caller is expected to immediately hand the nonce to
    /// [`Self::promote_nonce_to_session`] to reproduce the previous
    /// "link click = instant session" behavior.
    ///
    /// Nonces are kept in a map (one entry per still-pending login), not a
    /// single slot — issuing a fresh link for a second device must not
    /// invalidate a first device's nonce that's still mid-login, which would
    /// otherwise make a *correct* password submission look like a failure
    /// (and count against the shared login lockout).
    pub fn consume_link_token(&self, candidate: &str) -> Option<String> {
        let mut inner = self.inner.lock();
        let expected_digest = inner.pending_link_token_digest.clone()?;
        let candidate_digest = service::digest_bytes(candidate);
        if !constant_time_eq(&candidate_digest, &expected_digest) {
            return None;
        }
        inner.pending_link_token_digest = None;
        let nonce = service::generate_login_nonce();
        let expires_at = Instant::now() + Duration::from_millis(REMOTE_LOGIN_NONCE_TTL_MS);
        let now = Instant::now();
        inner.pending_login_nonces.retain(|_, nonce_expires_at| *nonce_expires_at > now);
        inner.pending_login_nonces.insert(service::digest_hex(&nonce), expires_at);
        Some(nonce)
    }

    /// Verifies (TTL-checked) and consumes a login nonce, then mints a new
    /// session (`REMOTE_SESSION_TTL_MS`). Password verification happens in
    /// the caller (`server.rs`) before this is invoked — this method only
    /// enforces that the nonce itself is genuine, unexpired, and still
    /// pending (one of possibly several concurrent logins-in-progress).
    /// Non-consuming check for whether `candidate` is a currently pending, unexpired login nonce
    /// — i.e. whether [`Self::promote_nonce_to_session`] would have a chance of succeeding for it.
    /// Used by `server.rs`'s `login_post_route` to reject a forged/stale nonce cookie *before*
    /// looking at the submitted password at all. Without this gate, a request carrying an
    /// arbitrary (never-issued, already-consumed, or expired) nonce cookie still reached password
    /// verification, and the wrong-password response (`failed: true`) vs. the link-expired response
    /// (`link_expired: true`, taken when `promote_nonce_to_session` returns `None` for that same
    /// forged nonce) differ in their rendered text — turning the pair into a password-correctness
    /// oracle an attacker could probe without ever having a real link. See
    /// `docs/acknowledge/2026-08-15-wave-b-hardening-contract.md`.
    pub fn has_pending_nonce(&self, candidate: &str) -> bool {
        let inner = self.inner.lock();
        let digest = service::digest_hex(candidate);
        match inner.pending_login_nonces.get(&digest) {
            Some(expires_at) => *expires_at > Instant::now(),
            None => false,
        }
    }

    pub fn promote_nonce_to_session(&self, candidate: &str) -> Option<String> {
        {
            let mut inner = self.inner.lock();
            let digest = service::digest_hex(candidate);
            let expires_at = inner.pending_login_nonces.remove(&digest)?;
            if Instant::now() >= expires_at {
                return None;
            }
        }
        Some(self.mint_session())
    }

    /// Mints a session with no nonce involved — used for "password-only"
    /// logins where the caller has already verified the password and the
    /// request never carried a link token.
    pub fn issue_session_without_nonce(&self) -> String {
        self.mint_session()
    }

    fn mint_session(&self) -> String {
        let session_token = service::generate_session_token();
        let expires_at = Instant::now() + Duration::from_millis(REMOTE_SESSION_TTL_MS);
        self.inner
            .lock()
            .session_token_digests
            .insert(service::digest_hex(&session_token), expires_at);
        session_token
    }

    pub fn has_active_session(&self, candidate: &str) -> bool {
        self.has_active_session_digest(&service::digest_hex(candidate))
    }

    /// Same check as [`Self::has_active_session`] but takes an
    /// already-computed digest — used by [`super::ws::handle_socket`], which
    /// only ever holds the digest `ws_upgrade_route` extracted at upgrade
    /// time (the raw session token never crosses into the WS layer).
    pub fn has_active_session_digest(&self, digest: &str) -> bool {
        let mut inner = self.inner.lock();
        match inner.session_token_digests.get(digest) {
            Some(expires_at) if *expires_at > Instant::now() => true,
            Some(_) => {
                inner.session_token_digests.remove(digest);
                false
            }
            None => false,
        }
    }

    /// The `Instant` a given session digest's `REMOTE_SESSION_TTL_MS` window
    /// closes, or `None` if the digest isn't a currently-tracked session
    /// (already expired-and-swept, revoked, or never existed). Consumed by
    /// [`super::ws::handle_socket`] to race a `sleep_until` deadline against
    /// the socket's own read loop — see that function's doc comment for why
    /// this per-session deadline is necessary on top of the bulk
    /// `subscribe_session_epoch` mechanism.
    pub fn session_expires_at(&self, digest: &str) -> Option<Instant> {
        self.inner.lock().session_token_digests.get(digest).copied()
    }

    /// Drops every session entry whose TTL has already elapsed. Session
    /// expiry is otherwise only ever discovered lazily — by
    /// [`Self::has_active_session_digest`] on the specific digest being
    /// checked — so a session that mints a cookie and is never checked again
    /// (no live WebSocket, no further HTTP requests) would sit in the map
    /// forever. Called once per new WebSocket upgrade
    /// ([`super::ws::handle_socket`]), which is frequent enough in practice
    /// to keep the map from growing unbounded without needing a dedicated
    /// background timer.
    pub fn sweep_expired_sessions(&self) {
        let now = Instant::now();
        self.inner.lock().session_token_digests.retain(|_, expires_at| *expires_at > now);
    }

    pub fn revoke_all_sessions(&self) {
        self.inner.lock().session_token_digests.clear();
        self.bump_session_epoch();
    }

    /// Subscribes to the session epoch — a counter bumped by
    /// [`Self::revoke_all_sessions`] (and, via that, `remote_set_password`/
    /// `remote_clear_password`, which already call it) and by
    /// [`Self::take_shutdown_state`] on server stop. `ws.rs` subscribes once
    /// per connection and races the returned receiver's `changed()` against
    /// the socket's own read loop, so an already-upgraded WebSocket closes
    /// itself the moment sessions are revoked instead of keeping the
    /// privileges it captured at upgrade time for the rest of its lifetime.
    ///
    /// This only covers *bulk* revocation — it does not fire for a single
    /// session's own `REMOTE_SESSION_TTL_MS` (7-day) expiry, since that
    /// happens on its own schedule per session rather than as a shared
    /// event. `ws.rs::handle_socket` covers that gap separately with a
    /// `sleep_until` deadline computed from [`Self::session_expires_at`] —
    /// see that function's doc comment. The two mechanisms are independent:
    /// this epoch is "something changed for everyone, re-check", the
    /// deadline is "this specific socket's own clock ran out".
    pub fn subscribe_session_epoch(&self) -> watch::Receiver<u64> {
        self.session_epoch_tx.subscribe()
    }

    fn bump_session_epoch(&self) {
        self.session_epoch_tx.send_modify(|epoch| *epoch = epoch.wrapping_add(1));
    }

    /// Remaining lockout duration for the given axis, or `None` if login
    /// attempts on that axis are currently allowed.
    ///
    /// The lockout is tracked as two independent counters selected by
    /// `has_nonce` — whether the request carried a login nonce cookie (i.e.
    /// came through the two-factor link-then-password flow) vs. not (the
    /// anonymous `remote_password_only_login` flow). Without this split, an
    /// attacker who can reach the login form anonymously (no link token
    /// needed once password-only mode is on) could exhaust the shared
    /// attempt budget and lock out a legitimate device that's mid-login with
    /// a valid nonce — a denial-of-service an *unauthenticated* visitor
    /// shouldn't be able to inflict on someone who already passed the
    /// link-token step. See `docs/acknowledge/2026-08-15-wave-b-hardening-contract.md` §3.1.
    pub fn login_lockout_remaining_ms(&self, has_nonce: bool) -> Option<u64> {
        let mut inner = self.inner.lock();
        let locked_until_slot = if has_nonce {
            &mut inner.login_locked_until_nonce
        } else {
            &mut inner.login_locked_until_anonymous
        };
        let locked_until = (*locked_until_slot)?;
        let now = Instant::now();
        if now >= locked_until {
            *locked_until_slot = None;
            return None;
        }
        Some((locked_until - now).as_millis() as u64)
    }

    /// Records a failed login attempt on the given axis (see
    /// [`Self::login_lockout_remaining_ms`]) and, once
    /// `REMOTE_LOGIN_MAX_ATTEMPTS` is exceeded on that axis, engages an
    /// exponential-backoff lockout capped at `REMOTE_LOGIN_LOCKOUT_MAX_MS`.
    pub fn record_login_failure(&self, has_nonce: bool) {
        let mut guard = self.inner.lock();
        let inner: &mut RemoteStoreInner = &mut guard;
        let (attempts, locked_until_slot) = if has_nonce {
            (&mut inner.failed_login_attempts_nonce, &mut inner.login_locked_until_nonce)
        } else {
            (&mut inner.failed_login_attempts_anonymous, &mut inner.login_locked_until_anonymous)
        };
        *attempts = attempts.saturating_add(1);
        if *attempts <= REMOTE_LOGIN_MAX_ATTEMPTS {
            return;
        }
        let overage = (*attempts - REMOTE_LOGIN_MAX_ATTEMPTS).min(16);
        let backoff_ms = REMOTE_LOGIN_LOCKOUT_BASE_MS
            .saturating_mul(1u64 << overage)
            .min(REMOTE_LOGIN_LOCKOUT_MAX_MS);
        *locked_until_slot = Some(Instant::now() + Duration::from_millis(backoff_ms));
    }

    pub fn record_login_success(&self, has_nonce: bool) {
        let mut inner = self.inner.lock();
        if has_nonce {
            inner.failed_login_attempts_nonce = 0;
            inner.login_locked_until_nonce = None;
        } else {
            inner.failed_login_attempts_anonymous = 0;
            inner.login_locked_until_anonymous = None;
        }
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
    remote.mark_started(port, shutdown_tx, server_handle);
    let status = remote.status();
    log::info!("원격 접속 서버 기동: port={port}");
    let _ = RemoteStateChanged { status }.emit(app);
    Ok(status)
}

/// Refreshes `RemoteStore`'s cached `password_configured` flag from the OS
/// keyring. Called once at app boot — `remote_status` polling afterward reads
/// the cache (see `RemoteStore::status`) instead of hitting the keyring
/// itself on every tick. Fail-closed on a keyring error: treated as
/// "configured" so a transient keyring failure can't make the settings UI
/// report "no password set" when one may well be.
pub fn refresh_password_configured_cache(app: &AppHandle) {
    let secret = app.state::<SecretStoreState>();
    let remote = app.state::<RemoteStore>();
    match secret.0.get(SecretAccount::RemoteAccess) {
        Ok(value) => remote.set_password_configured(value.is_some()),
        Err(error) => {
            log::warn!("원격 접속 비밀번호 설정 여부를 확인하지 못했습니다: {error}");
            remote.set_password_configured(true);
        }
    }
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

/// Starts the local remote-access HTTP/WS server if it isn't already running — no longer a
/// `#[tauri::command]` (X1#13, `docs/acknowledge/2026-08-19-xa-wiring-cleanup-contract.md` §1.2): the
/// only reachable path to it was already `settings_update`'s `remoteAccessEnabled` toggle
/// (`settings::commands::apply_integration_toggles`) and this module's own boot-time auto-start
/// (`lib.rs`), so the separate `remote_start` IPC surface duplicated that without adding a distinct
/// capability. Kept as a plain internal function since both of those call sites still need it.
pub async fn remote_start(app: AppHandle, remote: State<'_, RemoteStore>) -> AppResult<RemoteStatus> {
    if remote.is_running() {
        return Ok(remote.status());
    }
    bind_and_start(&app).await
}

/// Issues a one-time link token and formats it into a URL the user shares
/// with another device. When `Settings::remote_allowed_hosts` has at least
/// one registered tunnel hostname, the link points at that hostname over
/// `https` instead of the loopback address — a device reached only through
/// the tunnel (not on the same machine/network as the loopback bind) could
/// never resolve `http://127.0.0.1:{port}` (`docs/acknowledge/
/// 2026-08-15-wave-b-hardening-contract.md` §6). The first registered host is
/// used; `auth_middleware`/`is_allowed_host` accept a request addressed to
/// *any* registered host regardless of which one the link happened to name.
#[tauri::command]
#[specta::specta]
pub async fn remote_issue_link(app: AppHandle, remote: State<'_, RemoteStore>) -> AppResult<RemoteLinkInfo> {
    if !remote.is_running() {
        return Err(AppError::InvalidArgument("원격 접속 서버가 실행 중이 아닙니다".to_string()));
    }
    let token = remote.issue_link_token();
    let allowed_hosts = app.state::<AppState>().settings.read().remote_allowed_hosts.clone();
    let url = service::format_issue_link_url(&allowed_hosts, remote.port(), &token);
    Ok(RemoteLinkInfo { url })
}

#[tauri::command]
#[specta::specta]
pub async fn remote_revoke_sessions(remote: State<'_, RemoteStore>) -> AppResult<()> {
    remote.revoke_all_sessions();
    Ok(())
}

/// Sets (or replaces) the remote-access password: hashed with a fresh salt
/// and stored in the OS keyring (never in `settings.json` — see
/// `docs/acknowledge/2026-08-14-hotexit-remote-password-contract.md` §3.2).
/// Every existing session is invalidated so devices authenticated under the
/// old password (or under no-password mode) must re-authenticate.
///
/// The password is trimmed before both the length check and the hash (see
/// [`service::validate_and_trim_password`]): the settings UI already trims
/// client-side, so this makes the backend the single source of truth instead
/// of trusting that every caller does the same. `REMOTE_PASSWORD_MIN_LEN` is
/// only enforced here, on write — a password already stored below that
/// length (set before this check existed) keeps working until the user
/// changes it, per `docs/acknowledge/2026-08-15-wave-b-hardening-contract.md`
/// §3.1. The login form itself deliberately does *not* trim (see
/// `server.rs`'s `login_post_route`) since the stored value is already
/// trimmed.
#[tauri::command]
#[specta::specta]
pub async fn remote_set_password(remote: State<'_, RemoteStore>, secret: State<'_, SecretStoreState>, password: String) -> AppResult<()> {
    let Some(trimmed) = service::validate_and_trim_password(&password) else {
        return Err(AppError::InvalidArgument(format!(
            "비밀번호는 최소 {REMOTE_PASSWORD_MIN_LEN}자 이상이어야 합니다"
        )));
    };
    secret.0.set(SecretAccount::RemoteAccess, &service::hash_password(trimmed))?;
    remote.set_password_configured(true);
    remote.revoke_all_sessions();
    Ok(())
}

/// Removes the remote-access password, reverting to link-only access
/// (backward-compatible legacy mode). Invalidates every existing session.
#[tauri::command]
#[specta::specta]
pub async fn remote_clear_password(remote: State<'_, RemoteStore>, secret: State<'_, SecretStoreState>) -> AppResult<()> {
    secret.0.delete(SecretAccount::RemoteAccess)?;
    remote.set_password_configured(false);
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
        store.mark_started(51_300, shutdown_tx, dummy_handle());
        let status = store.status();
        assert!(status.running);
        assert_eq!(status.port, 51_300);
        assert!(store.is_running());
    }

    #[test]
    fn set_password_configured는_status에_반영된다() {
        let store = RemoteStore::default();
        assert!(!store.status().password_configured);

        store.set_password_configured(true);
        assert!(store.status().password_configured);

        store.set_password_configured(false);
        assert!(!store.status().password_configured);
    }

    #[test]
    fn 실행중이_아니면_shutdown_상태가_없다() {
        let store = RemoteStore::default();
        assert!(store.take_shutdown_state().is_none());
    }

    #[test]
    fn 올바른_링크_토큰은_노출로_이어지고_노출은_세션으로_승격된다() {
        let store = RemoteStore::default();
        let token = store.issue_link_token();

        assert!(store.consume_link_token("wrong-token").is_none());

        let nonce = store.consume_link_token(&token).expect("올바른 토큰은 nonce 를 발급해야 한다");
        let session = store
            .promote_nonce_to_session(&nonce)
            .expect("올바른 nonce 는 세션을 발급해야 한다");
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
    fn nonce는_한_번만_세션으로_승격된다() {
        let store = RemoteStore::default();
        let token = store.issue_link_token();
        let nonce = store.consume_link_token(&token).expect("nonce 발급");

        assert!(store.promote_nonce_to_session(&nonce).is_some());
        assert!(store.promote_nonce_to_session(&nonce).is_none());
    }

    #[test]
    fn 잘못된_nonce는_세션으로_승격되지_않는다() {
        let store = RemoteStore::default();
        let token = store.issue_link_token();
        store.consume_link_token(&token).expect("nonce 발급");

        assert!(store.promote_nonce_to_session("wrong-nonce").is_none());
    }

    #[test]
    fn 발급된_nonce는_소모_전까지_pending으로_보고된다() {
        let store = RemoteStore::default();
        let token = store.issue_link_token();
        let nonce = store.consume_link_token(&token).expect("nonce 발급");

        assert!(store.has_pending_nonce(&nonce));
    }

    #[test]
    fn 위조된_nonce는_pending으로_보고되지_않는다() {
        let store = RemoteStore::default();
        let token = store.issue_link_token();
        store.consume_link_token(&token).expect("nonce 발급");

        assert!(!store.has_pending_nonce("forged-nonce"));
    }

    #[test]
    fn 이미_승격된_nonce는_더_이상_pending이_아니다() {
        let store = RemoteStore::default();
        let token = store.issue_link_token();
        let nonce = store.consume_link_token(&token).expect("nonce 발급");
        store.promote_nonce_to_session(&nonce).expect("세션 승격");

        assert!(!store.has_pending_nonce(&nonce));
    }

    #[test]
    fn has_pending_nonce는_승격과_달리_nonce를_소모하지_않는다() {
        let store = RemoteStore::default();
        let token = store.issue_link_token();
        let nonce = store.consume_link_token(&token).expect("nonce 발급");

        assert!(store.has_pending_nonce(&nonce));
        assert!(store.has_pending_nonce(&nonce), "peek 는 반복 호출해도 상태를 바꾸지 않아야 한다");
        assert!(
            store.promote_nonce_to_session(&nonce).is_some(),
            "peek 이후에도 정상적으로 승격되어야 한다"
        );
    }

    #[test]
    fn 두_기기가_동시에_링크를_소모해도_두_nonce_모두_세션으로_승격된다() {
        let store = RemoteStore::default();

        let first_token = store.issue_link_token();
        let first_nonce = store.consume_link_token(&first_token).expect("첫 기기 nonce 발급");

        let second_token = store.issue_link_token();
        let second_nonce = store.consume_link_token(&second_token).expect("두 번째 기기 nonce 발급");

        assert!(
            store.promote_nonce_to_session(&first_nonce).is_some(),
            "두 번째 링크 발급이 첫 기기의 진행 중인 nonce를 무효화하면 안 된다"
        );
        assert!(store.promote_nonce_to_session(&second_nonce).is_some());
    }

    #[test]
    fn 세션을_전체_해제하면_더_이상_유효하지_않다() {
        let store = RemoteStore::default();
        let token = store.issue_link_token();
        let nonce = store.consume_link_token(&token).expect("nonce 발급");
        let session = store.promote_nonce_to_session(&nonce).expect("세션 발급");

        store.revoke_all_sessions();

        assert!(!store.has_active_session(&session));
    }

    #[test]
    fn 비밀번호_없이도_세션을_직접_발급할_수_있다() {
        let store = RemoteStore::default();
        let session = store.issue_session_without_nonce();
        assert!(store.has_active_session(&session));
    }

    #[test]
    fn 다이제스트로도_세션_활성_여부를_확인할_수_있다() {
        let store = RemoteStore::default();
        let session = store.issue_session_without_nonce();
        let digest = service::digest_hex(&session);

        assert!(store.has_active_session_digest(&digest));
        assert!(!store.has_active_session_digest("not-a-real-digest"));
    }

    #[test]
    fn session_expires_at은_존재하지_않는_다이제스트에_none을_반환한다() {
        let store = RemoteStore::default();
        assert!(store.session_expires_at("not-a-real-digest").is_none());
    }

    #[test]
    fn session_expires_at은_발급된_세션의_만료_시각을_반환한다() {
        let store = RemoteStore::default();
        let before = Instant::now();
        let session = store.issue_session_without_nonce();
        let digest = service::digest_hex(&session);

        let expires_at = store.session_expires_at(&digest).expect("발급된 세션은 만료 시각을 가져야 한다");

        assert!(expires_at > before, "만료 시각은 발급 시점보다 미래여야 한다");
    }

    #[test]
    fn sweep_expired_sessions는_전체_해제된_세션을_지운다() {
        let store = RemoteStore::default();
        let session = store.issue_session_without_nonce();
        let digest = service::digest_hex(&session);
        store.revoke_all_sessions();

        store.sweep_expired_sessions();

        assert!(store.session_expires_at(&digest).is_none());
    }

    #[test]
    fn sweep_expired_sessions는_아직_유효한_세션은_남긴다() {
        let store = RemoteStore::default();
        let session = store.issue_session_without_nonce();

        store.sweep_expired_sessions();

        assert!(store.has_active_session(&session), "만료되지 않은 세션은 스윕 후에도 남아야 한다");
    }

    #[test]
    fn 로그인_실패가_임계치를_넘으면_잠긴다() {
        let store = RemoteStore::default();
        assert!(store.login_lockout_remaining_ms(true).is_none());

        for _ in 0..REMOTE_LOGIN_MAX_ATTEMPTS {
            store.record_login_failure(true);
        }
        assert!(store.login_lockout_remaining_ms(true).is_none(), "임계치까지는 잠기지 않는다");

        store.record_login_failure(true);
        assert!(store.login_lockout_remaining_ms(true).is_some(), "임계치를 넘으면 잠긴다");
    }

    #[test]
    fn 로그인_성공은_잠금_상태를_초기화한다() {
        let store = RemoteStore::default();
        for _ in 0..=REMOTE_LOGIN_MAX_ATTEMPTS {
            store.record_login_failure(true);
        }
        assert!(store.login_lockout_remaining_ms(true).is_some());

        store.record_login_success(true);

        assert!(store.login_lockout_remaining_ms(true).is_none());
    }

    #[test]
    fn nonce_경로와_익명_경로의_잠금_카운터는_서로_독립적이다() {
        let store = RemoteStore::default();

        for _ in 0..=REMOTE_LOGIN_MAX_ATTEMPTS {
            store.record_login_failure(false);
        }

        assert!(store.login_lockout_remaining_ms(false).is_some(), "익명 축은 잠겨야 한다");
        assert!(
            store.login_lockout_remaining_ms(true).is_none(),
            "익명 축의 실패가 nonce 축을 잠그면 안 된다 — 정당한 2요소 로그인 중인 기기가 익명 공격에 잠기는 것을 막는다"
        );
    }

    #[test]
    fn 한_축의_로그인_성공은_다른_축의_잠금을_해제하지_않는다() {
        let store = RemoteStore::default();
        for _ in 0..=REMOTE_LOGIN_MAX_ATTEMPTS {
            store.record_login_failure(false);
        }
        assert!(store.login_lockout_remaining_ms(false).is_some());

        store.record_login_success(true);

        assert!(
            store.login_lockout_remaining_ms(false).is_some(),
            "nonce 축의 성공이 익명 축의 잠금까지 풀면 안 된다"
        );
    }

    #[test]
    fn 세션_전체_해제는_세션_epoch를_증가시킨다() {
        let store = RemoteStore::default();
        let mut epoch_rx = store.subscribe_session_epoch();
        assert_eq!(*epoch_rx.borrow(), 0, "초기 epoch는 0이어야 한다");

        store.revoke_all_sessions();

        assert!(
            epoch_rx.has_changed().expect("송신 측이 살아있어야 한다"),
            "revoke_all_sessions 는 구독자가 감지할 수 있는 epoch 변화를 만들어야 한다"
        );
        assert_eq!(*epoch_rx.borrow_and_update(), 1);
    }

    #[test]
    fn 서버_정지도_세션_epoch를_증가시킨다() {
        let store = RemoteStore::default();
        let (shutdown_tx, _rx) = watch::channel(());
        store.mark_started(51_301, shutdown_tx, dummy_handle());

        let epoch_rx = store.subscribe_session_epoch();
        store.take_shutdown_state();

        assert!(
            epoch_rx.has_changed().expect("송신 측이 살아있어야 한다"),
            "서버 정지는 이미 연결된 소켓도 끊어내야 하므로 epoch를 증가시켜야 한다"
        );
    }

    #[test]
    fn 구독_이전의_epoch_변화는_새_구독자에게_변화로_보고되지_않는다() {
        let store = RemoteStore::default();
        store.revoke_all_sessions();

        let epoch_rx = store.subscribe_session_epoch();

        assert!(
            !epoch_rx.has_changed().expect("송신 측이 살아있어야 한다"),
            "구독 시점의 최신 epoch는 이미 반영된 상태이므로 변화로 보고되면 안 된다"
        );
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

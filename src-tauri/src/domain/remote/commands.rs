use taide_model::app_event::AppEvent;
use taide_runtime::{EventSink, TaskSupervisor};
use tauri::{AppHandle, Manager, State};
use tokio::sync::watch;

pub use taide_remote::store::{RemoteShutdownState, RemoteStore};
pub use taide_runtime::RemoteDispatchLimiter;

use super::server;
use super::service;
use super::types::{RemoteLinkInfo, RemoteStatus, REMOTE_PASSWORD_MIN_LEN, REMOTE_SHUTDOWN_GRACE_MS};
use crate::error::{AppError, AppErrorKind, AppResult};
use crate::infra::secret::{SecretAccount, SecretStoreState};
use crate::platform::event_sink::TauriEventSink;
use crate::state::AppState;

async fn bind_and_start(app: &AppHandle) -> AppResult<RemoteStatus> {
    if app.state::<AppState>().is_shutting_down() {
        return Err(AppError::Internal("remote server unavailable during shutdown".to_string()));
    }
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0u16)).await.map_err(AppError::from)?;
    let port = listener.local_addr().map_err(AppError::from)?.port() as u32;

    let (shutdown_tx, shutdown_rx) = watch::channel(());
    let router = server::build_router(app.clone());
    let server_handle = app.state::<TaskSupervisor>().spawn_transient_handle("remote-server", async move {
        server::serve(listener, router, shutdown_rx).await;
    });
    let Some(server_handle) = server_handle else {
        return Err(AppError::Internal("remote server task supervisor stopped".to_string()));
    };
    if app.state::<AppState>().is_shutting_down() {
        server_handle.abort();
        return Err(AppError::Internal("remote server unavailable during shutdown".to_string()));
    }

    let remote = app.state::<RemoteStore>();
    if !remote.mark_started(port, shutdown_tx, server_handle) {
        return Ok(remote.status());
    }
    let status = remote.status();
    log::info!("원격 접속 서버 기동: port={port}");
    TauriEventSink(app).publish(AppEvent::RemoteStateChanged { status });
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
        let abort_handle = handle.abort_handle();
        if !app.state::<TaskSupervisor>().spawn_transient("remote-server-stop", async move {
            tokio::select! {
                _ = &mut handle => {}
                _ = tokio::time::sleep(std::time::Duration::from_millis(REMOTE_SHUTDOWN_GRACE_MS)) => {
                    handle.abort();
                }
            }
        }) {
            abort_handle.abort();
        }
    }

    TauriEventSink(app).publish(AppEvent::RemoteStateChanged {
        status: RemoteStatus::default(),
    });
}

#[tauri::command]
#[specta::specta]
pub async fn remote_status(remote: State<'_, RemoteStore>) -> AppResult<RemoteStatus> {
    Ok(remote.status())
}

/// Starts the local remote-access HTTP/WS server if it isn't already running — no longer a
/// `#[tauri::command]` (X1#13, `docs/acknowledge/2026-08-19-xa-wiring-cleanup-contract.md` §1.2): the
/// only reachable path to it was already `settings_update`'s `remoteAccessEnabled` toggle
/// ([`apply_remote_access_toggle`], registered as a settings toggle observer) and this module's
/// own boot-time auto-start (`lib.rs`), so the separate `remote_start` IPC surface duplicated that
/// without adding a distinct capability. Kept as a plain internal function since both of those
/// call sites still need it.
pub async fn remote_start(app: AppHandle, remote: State<'_, RemoteStore>) -> AppResult<RemoteStatus> {
    if remote.is_running() {
        return Ok(remote.status());
    }
    bind_and_start(&app).await
}

/// Reconciles a flipped `remote_access_enabled` settings value against the live server — starts
/// it when the toggle turns on, stops it when it turns off, no-op when the value didn't change.
/// Registered into `settings::commands::SettingsToggleObservers` by `lib.rs`'s assembly so the
/// settings domain never calls into this one directly (audit R5#6, T1-I §1.4).
pub async fn apply_remote_access_toggle(app: &AppHandle, was_enabled: bool, enabled: bool) {
    if was_enabled == enabled {
        return;
    }
    let remote = app.state::<RemoteStore>();
    if enabled {
        if let Err(error) = remote_start(app.clone(), remote).await {
            log::warn!("원격 접속 서버 시작 실패: {error}");
        }
    } else {
        stop_server(app, &remote);
    }
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
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.remote.serverNotRunning",
            "the remote-access server is not running",
        ));
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
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.remote.passwordTooShort",
            format!("password must be at least {REMOTE_PASSWORD_MIN_LEN} characters"),
        )
        .with_arg("min", REMOTE_PASSWORD_MIN_LEN));
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
    use std::sync::Arc;

    use super::*;

    /// §1-c 의 핵심 계약: 상한을 넘는 요청은 거부되지 않고 **대기**한다 — permit 을 전부 소진한
    /// 상태에서 하나를 반환하면 대기 중이던 호출이 그제서야 진행되는지를 확인한다.
    #[tokio::test]
    async fn 상한을_넘는_요청은_거부되지_않고_permit_반환을_대기한다() {
        let limiter = Arc::new(RemoteDispatchLimiter::new(1));

        let first_permit = limiter.acquire().await.expect("permit 이 존재해야 한다");

        let waiter_limiter = limiter.clone();
        let waiter = tokio::spawn(async move {
            let _permit = waiter_limiter.acquire().await.expect("permit 이 존재해야 한다");
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(
            !waiter.is_finished(),
            "permit 이 모두 소진된 동안에는 대기해야 하고, 거부되어 즉시 끝나면 안 된다"
        );

        drop(first_permit);
        waiter.await.expect("대기 중이던 태스크가 패닉했다");
    }

    #[tokio::test]
    async fn 서로_다른_permit은_동시에_진행된다() {
        let limiter = RemoteDispatchLimiter::new(crate::domain::remote::types::REMOTE_DISPATCH_MAX_CONCURRENT);

        let first = limiter.acquire().await;
        let second = tokio::time::timeout(std::time::Duration::from_millis(50), limiter.acquire()).await;

        assert!(first.is_some());
        assert!(
            matches!(second, Ok(Some(_))),
            "기본 상한({}) 미만으로 동시 요청 시 대기 없이 즉시 permit 을 받아야 한다",
            crate::domain::remote::types::REMOTE_DISPATCH_MAX_CONCURRENT
        );
    }
}

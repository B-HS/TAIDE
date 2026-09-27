use taide_infra::secret::{SecretAccount, SecretStoreState};
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::remote::{RemoteLinkInfo, RemoteStatus};
use taide_remote::service;
use taide_remote::store::RemoteStore;
use taide_remote::types::REMOTE_PASSWORD_MIN_LEN;

use crate::AppState;

/// Applies the shared remote status policy.
pub async fn remote_status(remote: &RemoteStore) -> AppResult<RemoteStatus> {
    Ok(remote.status())
}

/// Applies the shared remote issue link policy.
pub async fn remote_issue_link(state: &AppState, remote: &RemoteStore) -> AppResult<RemoteLinkInfo> {
    if !remote.is_running() {
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.remote.serverNotRunning",
            "the remote-access server is not running",
        ));
    }
    let token = remote.issue_link_token();
    let allowed_hosts = state.settings.read().remote_allowed_hosts.clone();
    let url = service::format_issue_link_url(&allowed_hosts, remote.port(), &token);
    Ok(RemoteLinkInfo { url })
}

/// Applies the shared remote revoke sessions policy.
pub async fn remote_revoke_sessions(remote: &RemoteStore) -> AppResult<()> {
    remote.revoke_all_sessions();
    Ok(())
}

/// Applies the shared remote set password policy.
pub async fn remote_set_password(remote: &RemoteStore, secret: &SecretStoreState, password: String) -> AppResult<()> {
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

/// Applies the shared remote clear password policy.
pub async fn remote_clear_password(remote: &RemoteStore, secret: &SecretStoreState) -> AppResult<()> {
    secret.0.delete(SecretAccount::RemoteAccess)?;
    remote.set_password_configured(false);
    remote.revoke_all_sessions();
    Ok(())
}

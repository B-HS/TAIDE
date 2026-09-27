use std::future::Future;

use taide_infra::secret::{SecretAccount, SecretStore};
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::settings::Settings;
use taide_model::sync::{SyncDownloadResult, SyncStatus};
use taide_sync::service;

use crate::{AppState, EventSink};

/// Supplies the existing gist operations without a toolkit or credential-store implementation.
pub trait SyncGistPort: Send + Sync {
    fn discover_sync_gist(
        &self,
        token: &str,
        preferred_id: Option<&str>,
    ) -> impl Future<Output = AppResult<Option<(String, String)>>> + Send;
    fn create_gist(&self, token: &str, payload_json: &str) -> impl Future<Output = AppResult<(String, String)>> + Send;
    fn update_gist(&self, token: &str, gist_id: &str, payload_json: &str) -> impl Future<Output = AppResult<String>> + Send;
    fn fetch_gist(&self, token: &str, gist_id: &str) -> impl Future<Output = AppResult<(String, String)>> + Send;
}

/// Builds the existing sync status snapshot without host state.
pub fn current_status_snapshot(settings: &Settings, connected: bool) -> SyncStatus {
    SyncStatus {
        connected,
        has_gist: settings.sync_gist_id.is_some(),
        last_synced_at: settings.sync_last_synced_at.clone(),
        remote_newer: None,
    }
}

/// Loads the sync credential through the supplied store using the existing missing-token error.
pub fn load_token(secret: &dyn SecretStore) -> AppResult<String> {
    secret
        .get(SecretAccount::GithubSync)?
        .ok_or_else(|| AppError::InvalidArgument("GitHub sync is not connected".to_string()))
}

/// Phase-③ write-back decision of [`sync_upload`]: overlays the sync bookkeeping fields onto the
/// live settings only while the live `sync_gist_id` still matches the phase-① snapshot. A
/// mismatch means a `sync_disconnect` (live went `None`) or a gist repoint landed while the
/// round-trip ran with the guard dropped — the write-back is skipped (`None`) so the interleaved
/// command's outcome survives instead of being resurrected by stale upload bookkeeping (Phase E
/// SYNC-1). On a match, every non-bookkeeping field comes from the live settings, so a
/// `settings_update` that landed mid-round-trip is never rolled back to the snapshot.
pub fn overlay_sync_bookkeeping(
    live_settings: &Settings,
    snapshot_gist_id: Option<&str>,
    gist_id: &str,
    remote_updated_at: &str,
) -> Option<Settings> {
    if live_settings.sync_gist_id.as_deref() != snapshot_gist_id {
        return None;
    }
    Some(Settings {
        sync_gist_id: Some(gist_id.to_string()),
        sync_last_synced_at: Some(remote_updated_at.to_string()),
        ..live_settings.clone()
    })
}

/// Describes the existing guard-side retry, conflict or apply outcome.
#[derive(Debug, PartialEq, Eq)]
pub enum DownloadApplyDecision {
    RetryGistChanged,
    RetrySyncCompleted,
    Conflict,
    Apply,
}

/// Guard-side decision of [`sync_download`], evaluated against the **live** settings after the
/// guard is re-acquired. The ordering preserves the pre-split command's semantics: the retry
/// aborts and the conflict verdict are decided before the payload is parsed, so an input that is
/// both conflicting and malformed still reports the conflict exactly as the old code did. The two
/// retry aborts cover what the old full-span lock excluded by construction: the configured gist
/// changing mid-fetch (`RetryGistChanged`), and another sync completing mid-fetch and moving
/// `sync_last_synced_at` off the pre-fetch snapshot (`RetrySyncCompleted`, Phase E SYNC-2) —
/// without the latter, a concurrent upload's newer bookkeeping would flip the conflict check to
/// "not newer" and let the stale fetched payload silently overwrite the settings that upload had
/// just pushed, while rolling `sync_last_synced_at` backwards.
pub fn decide_download_apply(
    live_gist_id: Option<&str>,
    live_last_synced_at: Option<&str>,
    fetched_gist_id: &str,
    pre_fetch_last_synced_at: Option<&str>,
    remote_updated_at: &str,
    force: bool,
) -> DownloadApplyDecision {
    if live_gist_id != Some(fetched_gist_id) {
        return DownloadApplyDecision::RetryGistChanged;
    }
    if live_last_synced_at != pre_fetch_last_synced_at {
        return DownloadApplyDecision::RetrySyncCompleted;
    }
    if !force && service::is_remote_newer(remote_updated_at, live_last_synced_at) {
        return DownloadApplyDecision::Conflict;
    }
    DownloadApplyDecision::Apply
}

/// Executes the existing sync application policy with explicit host ports.
pub async fn sync_status<G: SyncGistPort>(
    state: &AppState,
    secret: &dyn SecretStore,
    create_client: impl FnOnce() -> G,
) -> AppResult<SyncStatus> {
    let connected = secret.get(SecretAccount::GithubSync)?.is_some();
    let settings = state.settings.read().clone();
    let mut status = current_status_snapshot(&settings, connected);

    if let (true, Some(token), Some(gist_id)) = (connected, secret.get(SecretAccount::GithubSync)?, settings.sync_gist_id.clone()) {
        let gist_client = create_client();
        if let Ok((remote_updated_at, _)) = gist_client.fetch_gist(&token, &gist_id).await {
            status.remote_newer = Some(service::is_remote_newer(
                &remote_updated_at,
                settings.sync_last_synced_at.as_deref(),
            ));
        }
    }

    Ok(status)
}
/// Executes the existing sync application policy with explicit host ports.
pub async fn sync_connect<G: SyncGistPort>(
    state: &AppState,
    secret: &dyn SecretStore,
    pat: String,
    create_client: impl FnOnce() -> G,
    events: &dyn EventSink,
) -> AppResult<SyncStatus> {
    if pat.trim().is_empty() {
        return Err(AppError::InvalidArgument("token must not be empty".to_string()));
    }

    let gist_client = create_client();
    let previous_gist_id = state.settings.read().sync_gist_id.clone();
    let discovered = gist_client.discover_sync_gist(&pat, previous_gist_id.as_deref()).await?;

    let _guard = state.begin_mutation().await;
    let current = state.settings.read().clone();
    if current.sync_gist_id != previous_gist_id {
        return Err(AppError::InvalidArgument(
            "sync configuration changed while connecting — retry the connection".to_string(),
        ));
    }
    secret.set(SecretAccount::GithubSync, &pat)?;
    let gist_id = discovered.as_ref().map(|(id, _)| id.clone());
    let last_synced_at = if current.sync_gist_id == gist_id {
        current.sync_last_synced_at.clone()
    } else {
        None
    };
    let updated = Settings {
        sync_gist_id: gist_id,
        sync_last_synced_at: last_synced_at,
        ..current
    };
    taide_settings::service::save_settings(&state.paths, &updated)?;
    *state.settings.write() = updated.clone();
    let mut status = current_status_snapshot(&updated, true);
    status.remote_newer = discovered.map(|(_, updated_at)| service::is_remote_newer(&updated_at, updated.sync_last_synced_at.as_deref()));
    events.publish(AppEvent::SyncStateChanged { status: status.clone() });
    Ok(status)
}
/// Executes the existing sync application policy with explicit host ports.
pub async fn sync_disconnect(state: &AppState, secret: &dyn SecretStore, events: &dyn EventSink) -> AppResult<SyncStatus> {
    let _guard = state.begin_mutation().await;
    secret.delete(SecretAccount::GithubSync)?;

    let current = state.settings.read().clone();
    let updated = Settings {
        sync_gist_id: None,
        sync_last_synced_at: None,
        ..current
    };
    taide_settings::service::save_settings(&state.paths, &updated)?;
    *state.settings.write() = updated.clone();

    let status = current_status_snapshot(&updated, false);
    events.publish(AppEvent::SyncStateChanged { status: status.clone() });
    Ok(status)
}
/// Executes the existing sync application policy with explicit host ports.
pub async fn sync_upload<G: SyncGistPort>(
    state: &AppState,
    secret: &dyn SecretStore,
    create_client: impl FnOnce() -> G,
    events: &dyn EventSink,
) -> AppResult<SyncStatus> {
    let guard = state.begin_mutation().await;
    let token = load_token(secret)?;
    let settings_snapshot = state.settings.read().clone();
    let themes = service::collect_theme_entries(&state.paths);
    let locales = service::collect_locale_entries(&state.paths);
    let payload = service::assemble_payload(&settings_snapshot, themes, locales, service::now_utc_iso8601());
    let payload_json = serde_json::to_string_pretty(&payload)?;

    let gist_client = create_client();

    let snapshot_gist_id = settings_snapshot.sync_gist_id.clone();
    let (_guard, gist_id, remote_updated_at) = match snapshot_gist_id.clone() {
        Some(id) => {
            drop(guard);
            let updated_at = gist_client.update_gist(&token, &id, &payload_json).await?;
            (state.begin_mutation().await, id, updated_at)
        }
        None => {
            let (id, updated_at) = gist_client.create_gist(&token, &payload_json).await?;
            (guard, id, updated_at)
        }
    };

    let connected = secret.get(SecretAccount::GithubSync)?.is_some();
    let live_settings = state.settings.read().clone();
    let Some(updated_settings) = overlay_sync_bookkeeping(&live_settings, snapshot_gist_id.as_deref(), &gist_id, &remote_updated_at) else {
        return Ok(current_status_snapshot(&live_settings, connected));
    };
    taide_settings::service::save_settings(&state.paths, &updated_settings)?;
    *state.settings.write() = updated_settings.clone();

    let status = current_status_snapshot(&updated_settings, connected);
    events.publish(AppEvent::SyncStateChanged { status: status.clone() });
    Ok(status)
}
/// Executes the existing sync application policy with explicit host ports.
pub async fn sync_download<G, F, Fut>(
    state: &AppState,
    secret: &dyn SecretStore,
    create_client: impl FnOnce() -> G,
    apply_settings: F,
    force: bool,
    events: &dyn EventSink,
) -> AppResult<SyncDownloadResult>
where
    G: SyncGistPort,
    F: FnOnce(Settings) -> Fut,
    Fut: Future<Output = AppResult<Settings>>,
{
    let token = load_token(secret)?;
    let (gist_id, pre_fetch_last_synced_at) = {
        let settings = state.settings.read();
        let gist_id = settings
            .sync_gist_id
            .clone()
            .ok_or_else(|| AppError::InvalidArgument("no sync gist is configured yet — upload once first".to_string()))?;
        (gist_id, settings.sync_last_synced_at.clone())
    };

    let gist_client = create_client();
    let (remote_updated_at, content) = gist_client.fetch_gist(&token, &gist_id).await?;

    let _guard = state.begin_mutation().await;
    let current = state.settings.read().clone();
    match decide_download_apply(
        current.sync_gist_id.as_deref(),
        current.sync_last_synced_at.as_deref(),
        &gist_id,
        pre_fetch_last_synced_at.as_deref(),
        &remote_updated_at,
        force,
    ) {
        DownloadApplyDecision::RetryGistChanged => {
            return Err(AppError::InvalidArgument(
                "the configured sync gist changed while downloading — retry the download".to_string(),
            ))
        }
        DownloadApplyDecision::RetrySyncCompleted => {
            return Err(AppError::InvalidArgument(
                "another sync completed while downloading — retry the download".to_string(),
            ))
        }
        DownloadApplyDecision::Conflict => return Ok(SyncDownloadResult::Conflict { remote_updated_at }),
        DownloadApplyDecision::Apply => {}
    }

    let payload = service::parse_synced_payload(&content)
        .ok_or_else(|| AppError::Internal("sync payload from the gist was malformed".to_string()))?;
    service::ensure_supported_schema_version(payload.schema_version)?;

    let applied = service::apply_payload_settings(&current, &payload);
    let final_settings = Settings {
        sync_gist_id: Some(gist_id),
        sync_last_synced_at: Some(remote_updated_at),
        ..applied
    };
    let final_settings = apply_settings(final_settings).await?;

    service::apply_theme_entries(&state.paths, &payload.themes);
    service::apply_locale_entries(&state.paths, &payload.locales);

    let status = current_status_snapshot(&final_settings, true);
    events.publish(AppEvent::SyncStateChanged { status: status.clone() });
    Ok(SyncDownloadResult::Applied { status })
}

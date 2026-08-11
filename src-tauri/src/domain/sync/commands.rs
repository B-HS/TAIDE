use tauri::State;
use tauri_specta::Event;

use crate::domain::settings::service as settings_service;
use crate::domain::settings::types::Settings;
use crate::domain::sync::github::GistClient;
use crate::domain::sync::service;
use crate::domain::sync::types::{SyncDownloadResult, SyncPayload, SyncStatus};
use crate::error::{AppError, AppResult};
use crate::events::SyncStateChanged;
use crate::infra::http::create_outbound_http_client;
use crate::infra::secret::{SecretAccount, SecretStore, SecretStoreState};
use crate::state::AppState;

fn current_status_snapshot(settings: &Settings, connected: bool) -> SyncStatus {
    SyncStatus {
        connected,
        has_gist: settings.sync_gist_id.is_some(),
        last_synced_at: settings.sync_last_synced_at.clone(),
        remote_newer: None,
    }
}

fn load_token(secret: &dyn SecretStore) -> AppResult<String> {
    secret
        .get(SecretAccount::GithubSync)?
        .ok_or_else(|| AppError::InvalidArgument("GitHub sync is not connected".to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn sync_status(state: State<'_, AppState>, secret: State<'_, SecretStoreState>) -> AppResult<SyncStatus> {
    let secret = secret.0.as_ref();
    let connected = secret.get(SecretAccount::GithubSync)?.is_some();
    let settings = state.settings.read().clone();
    let mut status = current_status_snapshot(&settings, connected);

    if let (true, Some(token), Some(gist_id)) = (connected, secret.get(SecretAccount::GithubSync)?, settings.sync_gist_id.clone()) {
        let client = create_outbound_http_client();
        let gist_client = GistClient {
            client: &client,
            token: &token,
        };
        if let Ok((remote_updated_at, _)) = gist_client.fetch_gist(&gist_id).await {
            status.remote_newer = Some(service::is_remote_newer(
                &remote_updated_at,
                settings.sync_last_synced_at.as_deref(),
            ));
        }
    }

    Ok(status)
}

#[tauri::command]
#[specta::specta]
pub async fn sync_connect(state: State<'_, AppState>, secret: State<'_, SecretStoreState>, pat: String) -> AppResult<SyncStatus> {
    if pat.trim().is_empty() {
        return Err(AppError::InvalidArgument("token must not be empty".to_string()));
    }

    let client = create_outbound_http_client();
    GistClient {
        client: &client,
        token: &pat,
    }
    .verify_token()
    .await?;
    secret.0.as_ref().set(SecretAccount::GithubSync, &pat)?;

    let settings = state.settings.read().clone();
    Ok(current_status_snapshot(&settings, true))
}

#[tauri::command]
#[specta::specta]
pub async fn sync_disconnect(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    secret: State<'_, SecretStoreState>,
) -> AppResult<SyncStatus> {
    let _guard = state.begin_mutation().await;
    secret.0.as_ref().delete(SecretAccount::GithubSync)?;

    let current = state.settings.read().clone();
    let updated = Settings {
        sync_gist_id: None,
        sync_last_synced_at: None,
        ..current
    };
    settings_service::save_settings(&state.paths, &updated)?;
    *state.settings.write() = updated.clone();

    let status = current_status_snapshot(&updated, false);
    let _ = SyncStateChanged { status: status.clone() }.emit(&app);
    Ok(status)
}

#[tauri::command]
#[specta::specta]
pub async fn sync_upload(app: tauri::AppHandle, state: State<'_, AppState>, secret: State<'_, SecretStoreState>) -> AppResult<SyncStatus> {
    let _guard = state.begin_mutation().await;
    let token = load_token(secret.0.as_ref())?;
    let settings = state.settings.read().clone();

    let themes = service::collect_theme_entries(&state.paths);
    let locales = service::collect_locale_entries(&state.paths);
    let payload = service::assemble_payload(&settings, themes, locales, service::now_utc_iso8601());
    let payload_json = serde_json::to_string_pretty(&payload)?;

    let client = create_outbound_http_client();
    let gist_client = GistClient {
        client: &client,
        token: &token,
    };

    let (gist_id, remote_updated_at) = match settings.sync_gist_id.clone() {
        Some(id) => {
            let updated_at = gist_client.update_gist(&id, &payload_json).await?;
            (id, updated_at)
        }
        None => gist_client.create_gist(&payload_json).await?,
    };

    let updated_settings = Settings {
        sync_gist_id: Some(gist_id),
        sync_last_synced_at: Some(remote_updated_at),
        ..settings
    };
    settings_service::save_settings(&state.paths, &updated_settings)?;
    *state.settings.write() = updated_settings.clone();

    let status = current_status_snapshot(&updated_settings, true);
    let _ = SyncStateChanged { status: status.clone() }.emit(&app);
    Ok(status)
}

#[tauri::command]
#[specta::specta]
pub async fn sync_download(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    secret: State<'_, SecretStoreState>,
    force: bool,
) -> AppResult<SyncDownloadResult> {
    let _guard = state.begin_mutation().await;
    let token = load_token(secret.0.as_ref())?;
    let settings = state.settings.read().clone();
    let gist_id = settings
        .sync_gist_id
        .clone()
        .ok_or_else(|| AppError::InvalidArgument("no sync gist is configured yet — upload once first".to_string()))?;

    let client = create_outbound_http_client();
    let gist_client = GistClient {
        client: &client,
        token: &token,
    };
    let (remote_updated_at, content) = gist_client.fetch_gist(&gist_id).await?;

    if !force && service::is_remote_newer(&remote_updated_at, settings.sync_last_synced_at.as_deref()) {
        return Ok(SyncDownloadResult::Conflict { remote_updated_at });
    }

    let payload: SyncPayload =
        serde_json::from_str(&content).map_err(|_| AppError::Internal("sync payload from the gist was malformed".to_string()))?;
    service::ensure_supported_schema_version(payload.schema_version)?;

    let applied = service::apply_payload_settings(&settings, &payload);
    let final_settings = Settings {
        sync_gist_id: Some(gist_id),
        sync_last_synced_at: Some(remote_updated_at),
        ..applied
    };
    settings_service::save_settings(&state.paths, &final_settings)?;
    *state.settings.write() = final_settings.clone();

    service::apply_theme_entries(&state.paths, &payload.themes);
    service::apply_locale_entries(&state.paths, &payload.locales);

    let status = current_status_snapshot(&final_settings, true);
    let _ = SyncStateChanged { status: status.clone() }.emit(&app);
    Ok(SyncDownloadResult::Applied { status })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 연결_안됐고_gist도_없으면_상태는_전부_비어있다() {
        let status = current_status_snapshot(&Settings::default(), false);
        assert!(!status.connected);
        assert!(!status.has_gist);
        assert_eq!(status.last_synced_at, None);
        assert_eq!(status.remote_newer, None);
    }

    #[test]
    fn gist_아이디와_마지막_동기화_시각이_상태에_반영된다() {
        let settings = Settings {
            sync_gist_id: Some("gist-1".to_string()),
            sync_last_synced_at: Some("2026-08-11T00:00:00Z".to_string()),
            ..Settings::default()
        };
        let status = current_status_snapshot(&settings, true);

        assert!(status.connected);
        assert!(status.has_gist);
        assert_eq!(status.last_synced_at, Some("2026-08-11T00:00:00Z".to_string()));
    }

    #[test]
    fn 토큰이_없으면_업로드_다운로드는_연결_필요_에러를_반환한다() {
        use crate::infra::secret::test_support::InMemorySecretStore;
        let store = InMemorySecretStore::default();
        let result = load_token(&store);
        assert!(matches!(result, Err(AppError::InvalidArgument(_))));
    }
}

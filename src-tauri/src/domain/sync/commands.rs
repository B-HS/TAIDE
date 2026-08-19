use tauri::State;
use tauri_specta::Event;

use crate::domain::settings::service as settings_service;
use crate::domain::settings::types::Settings;
use crate::domain::sync::github::GistClient;
use crate::domain::sync::service;
use crate::domain::sync::types::{SyncDownloadResult, SyncStatus};
use crate::error::{AppError, AppResult};
use crate::events::SyncStateChanged;
use crate::infra::http::{outbound_http_client, HttpClientProfile};
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

/// Phase-③ write-back decision of [`sync_upload`]: overlays the sync bookkeeping fields onto the
/// live settings only while the live `sync_gist_id` still matches the phase-① snapshot. A
/// mismatch means a `sync_disconnect` (live went `None`) or a gist repoint landed while the
/// round-trip ran with the guard dropped — the write-back is skipped (`None`) so the interleaved
/// command's outcome survives instead of being resurrected by stale upload bookkeeping (Phase E
/// SYNC-1). On a match, every non-bookkeeping field comes from the live settings, so a
/// `settings_update` that landed mid-round-trip is never rolled back to the snapshot.
fn overlay_sync_bookkeeping(
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

#[derive(Debug, PartialEq, Eq)]
enum DownloadApplyDecision {
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
fn decide_download_apply(
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

#[tauri::command]
#[specta::specta]
pub async fn sync_status(state: State<'_, AppState>, secret: State<'_, SecretStoreState>) -> AppResult<SyncStatus> {
    let secret = secret.0.as_ref();
    let connected = secret.get(SecretAccount::GithubSync)?.is_some();
    let settings = state.settings.read().clone();
    let mut status = current_status_snapshot(&settings, connected);

    if let (true, Some(token), Some(gist_id)) = (connected, secret.get(SecretAccount::GithubSync)?, settings.sync_gist_id.clone()) {
        let client = outbound_http_client(HttpClientProfile::Api);
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

    let client = outbound_http_client(HttpClientProfile::Api);
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

/// Runs in three phases so the GitHub round-trip (60s client timeout) no longer holds the
/// app-wide mutation lock for its whole duration (audit R5#7, C11 axis A). What the old full-span
/// guard actually protected, and how each protection is preserved:
/// ① a `begin_mutation` hold reads the token and snapshots settings + theme/locale files — the
/// same point-in-time payload consistency the full-span hold gave, and the same "a disconnect
/// that already landed fails the upload before any network write" ordering (the token read sits
/// under the guard exactly as it originally did); ② updating an **existing** gist runs with the
/// guard dropped — freezing every other mutation (file saves included) for the round-trip was
/// cost, not protection. The **first-ever gist creation keeps the phase-① guard across the
/// round-trip** (Phase E F5): dropping it there would let two racing uploads each create a gist
/// and strand one — holding possibly secret-bearing settings content — orphaned on GitHub with
/// no UI able to delete it, so the once-per-account creation pays the old full-span cost and the
/// race is excluded by construction; ③ the guard is (re-)held and [`overlay_sync_bookkeeping`]
/// revalidates the live `sync_gist_id` against the phase-① snapshot before writing back — a
/// `sync_disconnect` or gist repoint that landed during the round-trip wins and the write-back is
/// skipped (Phase E SYNC-1), while a `settings_update` that landed mid-round-trip is never rolled
/// back because every non-bookkeeping field comes from the live settings. The emitted `connected`
/// is measured from the secret store under the same guard, never hardcoded.
///
/// Consistency regime (contract 2026-08-19 §1.1): `sync_gist_id`/`sync_last_synced_at` are sync
/// bookkeeping owned by the last still-valid sync write-back; every other field is owned by the
/// live settings. The uploaded content is the phase-① snapshot — a change made mid-upload rides
/// the next upload.
#[tauri::command]
#[specta::specta]
pub async fn sync_upload(app: tauri::AppHandle, state: State<'_, AppState>, secret: State<'_, SecretStoreState>) -> AppResult<SyncStatus> {
    let guard = state.begin_mutation().await;
    let token = load_token(secret.0.as_ref())?;
    let settings_snapshot = state.settings.read().clone();
    let themes = service::collect_theme_entries(&state.paths);
    let locales = service::collect_locale_entries(&state.paths);
    let payload = service::assemble_payload(&settings_snapshot, themes, locales, service::now_utc_iso8601());
    let payload_json = serde_json::to_string_pretty(&payload)?;

    let client = outbound_http_client(HttpClientProfile::Api);
    let gist_client = GistClient {
        client: &client,
        token: &token,
    };

    let snapshot_gist_id = settings_snapshot.sync_gist_id.clone();
    let (_guard, gist_id, remote_updated_at) = match snapshot_gist_id.clone() {
        Some(id) => {
            drop(guard);
            let updated_at = gist_client.update_gist(&id, &payload_json).await?;
            (state.begin_mutation().await, id, updated_at)
        }
        None => {
            let (id, updated_at) = gist_client.create_gist(&payload_json).await?;
            (guard, id, updated_at)
        }
    };

    let connected = secret.0.as_ref().get(SecretAccount::GithubSync)?.is_some();
    let live_settings = state.settings.read().clone();
    let Some(updated_settings) = overlay_sync_bookkeeping(&live_settings, snapshot_gist_id.as_deref(), &gist_id, &remote_updated_at) else {
        return Ok(current_status_snapshot(&live_settings, connected));
    };
    settings_service::save_settings(&state.paths, &updated_settings)?;
    *state.settings.write() = updated_settings.clone();

    let status = current_status_snapshot(&updated_settings, connected);
    let _ = SyncStateChanged { status: status.clone() }.emit(&app);
    Ok(status)
}

/// Fetches the gist **outside** `AppState::begin_mutation` and takes the guard only for the local
/// apply (audit R5#7, C11 axis A). What the old full-span guard actually protected, and how each
/// protection is preserved: the fetch phase reads no app state beyond a snapshot of
/// `sync_gist_id` + `sync_last_synced_at`, so holding the lock across the round-trip protected
/// nothing local; the check-then-apply phase (retry/conflict decision → payload parse/schema gate
/// → settings apply → theme/locale file writes) runs entirely under the re-acquired guard. The
/// old lock also made two things true by construction, both preserved by
/// [`decide_download_apply`]'s revalidation against the live settings: the configured gist can't
/// change while a download is in flight (cleared by `sync_disconnect` or repointed → retry
/// abort), and no other sync can complete while a download is in flight (live
/// `sync_last_synced_at` moved off the pre-fetch snapshot → retry abort, so a stale fetched
/// payload can never overwrite what a concurrent upload just pushed — Phase E SYNC-2). Both
/// aborts reuse the pre-existing `AppError::InvalidArgument` retry shape (wire unchanged), and
/// the decision runs before the parse/schema gates so the conflict-vs-error outcome for any given
/// input matches the pre-split command (Phase E T1H-C3).
#[tauri::command]
#[specta::specta]
pub async fn sync_download(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    secret: State<'_, SecretStoreState>,
    force: bool,
) -> AppResult<SyncDownloadResult> {
    let token = load_token(secret.0.as_ref())?;
    let (gist_id, pre_fetch_last_synced_at) = {
        let settings = state.settings.read();
        let gist_id = settings
            .sync_gist_id
            .clone()
            .ok_or_else(|| AppError::InvalidArgument("no sync gist is configured yet — upload once first".to_string()))?;
        (gist_id, settings.sync_last_synced_at.clone())
    };

    let client = outbound_http_client(HttpClientProfile::Api);
    let gist_client = GistClient {
        client: &client,
        token: &token,
    };
    let (remote_updated_at, content) = gist_client.fetch_gist(&gist_id).await?;

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
    let final_settings = crate::domain::settings::commands::apply_and_broadcast(&app, &state, final_settings).await?;

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

    fn settings_with_sync(gist_id: Option<&str>, last_synced_at: Option<&str>) -> Settings {
        Settings {
            sync_gist_id: gist_id.map(str::to_string),
            sync_last_synced_at: last_synced_at.map(str::to_string),
            ..Settings::default()
        }
    }

    #[test]
    fn 업로드_되쓰기는_라운드트립_중_disconnect가_지운_gist를_되살리지_않는다() {
        let live = settings_with_sync(None, None);
        assert_eq!(
            overlay_sync_bookkeeping(&live, Some("gist-1"), "gist-1", "2026-08-19T01:00:00Z"),
            None,
            "disconnect 가 라이브 gist id 를 지웠으면 되쓰기를 건너뛰어야 한다"
        );
    }

    #[test]
    fn 업로드_되쓰기는_라운드트립_중_재지정된_gist를_덮지_않는다() {
        let live = settings_with_sync(Some("gist-2"), Some("2026-08-19T00:00:00Z"));
        assert_eq!(
            overlay_sync_bookkeeping(&live, Some("gist-1"), "gist-1", "2026-08-19T01:00:00Z"),
            None,
            "라이브 gist 가 다른 대상으로 바뀌었으면 되쓰기를 건너뛰어야 한다"
        );
    }

    #[test]
    fn 업로드_되쓰기는_신규_생성_경합으로_이미_기록된_gist를_덮지_않는다() {
        let live = settings_with_sync(Some("gist-other"), Some("2026-08-19T00:30:00Z"));
        assert_eq!(
            overlay_sync_bookkeeping(&live, None, "gist-mine", "2026-08-19T01:00:00Z"),
            None,
            "스냅샷이 None 이었는데 라이브에 이미 gist 가 기록됐으면 되쓰기를 건너뛰어야 한다"
        );
    }

    #[test]
    fn 업로드_되쓰기는_스냅샷과_라이브가_일치하면_북키핑만_갱신하고_라이브_필드를_보존한다() {
        let live = Settings {
            editor_font_size: 19,
            ..settings_with_sync(Some("gist-1"), Some("2026-08-19T00:00:00Z"))
        };
        let updated = overlay_sync_bookkeeping(&live, Some("gist-1"), "gist-1", "2026-08-19T01:00:00Z").expect("일치하면 되써야 한다");

        assert_eq!(updated.sync_gist_id.as_deref(), Some("gist-1"));
        assert_eq!(updated.sync_last_synced_at.as_deref(), Some("2026-08-19T01:00:00Z"));
        assert_eq!(updated.editor_font_size, 19, "북키핑 외 필드는 라이브 값을 보존해야 한다");
    }

    #[test]
    fn 다운로드는_라운드트립_중_gist가_바뀌면_재시도를_요구한다() {
        assert_eq!(
            decide_download_apply(
                Some("gist-2"),
                Some("2026-08-19T00:00:00Z"),
                "gist-1",
                Some("2026-08-19T00:00:00Z"),
                "2026-08-19T00:00:00Z",
                false,
            ),
            DownloadApplyDecision::RetryGistChanged
        );
    }

    #[test]
    fn 다운로드는_라운드트립_중_다른_sync가_완료되면_stale_적용_대신_재시도를_요구한다() {
        assert_eq!(
            decide_download_apply(
                Some("gist-1"),
                Some("2026-08-19T01:00:00Z"),
                "gist-1",
                Some("2026-08-19T00:00:00Z"),
                "2026-08-19T00:00:00Z",
                false,
            ),
            DownloadApplyDecision::RetrySyncCompleted,
            "동시 업로드가 last_synced_at 을 전진시켰으면 fetch 시점 payload 는 stale 이다"
        );
    }

    #[test]
    fn 다운로드는_원격이_더_새로우면_충돌을_보고한다() {
        assert_eq!(
            decide_download_apply(
                Some("gist-1"),
                Some("2026-08-19T00:00:00Z"),
                "gist-1",
                Some("2026-08-19T00:00:00Z"),
                "2026-08-19T02:00:00Z",
                false,
            ),
            DownloadApplyDecision::Conflict
        );
    }

    #[test]
    fn 다운로드는_force면_충돌_검사를_건너뛰고_적용한다() {
        assert_eq!(
            decide_download_apply(
                Some("gist-1"),
                Some("2026-08-19T00:00:00Z"),
                "gist-1",
                Some("2026-08-19T00:00:00Z"),
                "2026-08-19T02:00:00Z",
                true,
            ),
            DownloadApplyDecision::Apply
        );
    }

    #[test]
    fn 다운로드는_스냅샷과_라이브가_일치하고_원격이_새롭지_않으면_적용한다() {
        assert_eq!(
            decide_download_apply(
                Some("gist-1"),
                Some("2026-08-19T02:00:00Z"),
                "gist-1",
                Some("2026-08-19T02:00:00Z"),
                "2026-08-19T02:00:00Z",
                false,
            ),
            DownloadApplyDecision::Apply
        );
    }
}

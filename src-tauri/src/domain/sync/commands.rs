use taide_runtime::{sync_actions, TaskSupervisor};
use tauri::State;

#[cfg(test)]
use crate::domain::settings::types::Settings;
use crate::domain::sync::github::SyncGistHttpPort;
use crate::domain::sync::types::{SyncDownloadResult, SyncStatus};
#[cfg(test)]
use crate::error::AppError;
use crate::error::AppResult;
use crate::infra::secret::SecretStoreState;
use crate::platform::event_sink::TauriEventSink;
use crate::settings_port::SettingsApplyPort;
use crate::state::AppState;

#[cfg(test)]
use taide_runtime::sync_actions::load_token;
pub use taide_runtime::sync_actions::{current_status_snapshot, decide_download_apply, overlay_sync_bookkeeping, DownloadApplyDecision};

#[tauri::command]
#[specta::specta]
pub async fn sync_status(state: State<'_, AppState>, secret: State<'_, SecretStoreState>) -> AppResult<SyncStatus> {
    sync_actions::sync_status(&state, secret.0.as_ref(), SyncGistHttpPort::new).await
}

#[tauri::command]
#[specta::specta]
pub async fn sync_connect(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    secret: State<'_, SecretStoreState>,
    pat: String,
) -> AppResult<SyncStatus> {
    sync_actions::sync_connect(&state, secret.0.as_ref(), pat, SyncGistHttpPort::new, &TauriEventSink(&app)).await
}

#[tauri::command]
#[specta::specta]
pub async fn sync_disconnect(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    secret: State<'_, SecretStoreState>,
) -> AppResult<SyncStatus> {
    sync_actions::sync_disconnect(&state, secret.0.as_ref(), &TauriEventSink(&app)).await
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
    sync_actions::sync_upload(&state, secret.0.as_ref(), SyncGistHttpPort::new, &TauriEventSink(&app)).await
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
    tasks: State<'_, TaskSupervisor>,
    apply_settings: State<'_, SettingsApplyPort>,
    force: bool,
) -> AppResult<SyncDownloadResult> {
    let prepared = sync_actions::prepare_sync_download(&state, secret.0.as_ref(), SyncGistHttpPort::new).await?;
    let state = state.inner().clone();
    let apply_settings = apply_settings.0;
    tasks
        .run_nonabortable_result("sync-download-apply", async move {
            sync_actions::apply_sync_download(
                &state,
                prepared,
                |settings| apply_settings(&app, &state, settings),
                force,
                &TauriEventSink(&app),
            )
            .await
        })
        .await
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

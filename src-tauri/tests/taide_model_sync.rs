use taide_lib::domain::sync::types::{
    SyncDownloadResult, SyncLocaleEntry, SyncPayload, SyncStatus, SyncThemeEntry, SYNC_GIST_DESCRIPTION, SYNC_GIST_FILENAME,
};
use taide_model::sync::{
    SyncDownloadResult as ModelSyncDownloadResult, SyncLocaleEntry as ModelSyncLocaleEntry, SyncPayload as ModelSyncPayload,
    SyncStatus as ModelSyncStatus, SyncThemeEntry as ModelSyncThemeEntry,
};

#[test]
fn sync_payload의_기존_wire와_기본값을_유지한다() {
    let payload: ModelSyncPayload = serde_json::from_value(serde_json::json!({
        "schemaVersion": 1,
        "updatedAt": "2026-09-24T00:00:00Z",
        "settings": {
            "themeId": "taide-dark"
        }
    }))
    .expect("구버전 sync payload");
    let facade: SyncPayload = payload;
    assert_eq!(facade.schema_version, 1);
    assert_eq!(facade.settings.theme_id.as_deref(), Some("taide-dark"));
    assert!(facade.themes.is_empty());
    assert!(facade.locales.is_empty());

    let theme: SyncThemeEntry = ModelSyncThemeEntry {
        id: "theme".to_string(),
        json: "{}".to_string(),
    };
    let locale: SyncLocaleEntry = ModelSyncLocaleEntry {
        id: "ko".to_string(),
        json: "{}".to_string(),
    };
    assert_eq!(serde_json::to_value(theme).expect("theme 직렬화")["id"], "theme");
    assert_eq!(serde_json::to_value(locale).expect("locale 직렬화")["id"], "ko");
    assert_eq!(SYNC_GIST_FILENAME, "taide-settings.json");
    assert!(SYNC_GIST_DESCRIPTION.starts_with("TAIDE sync payload"));
}

#[test]
fn sync_status와_download_result의_기존_wire를_유지한다() {
    let status: ModelSyncStatus = serde_json::from_value(serde_json::json!({
        "connected": true,
        "hasGist": true,
        "lastSyncedAt": null,
        "remoteNewer": false
    }))
    .expect("sync status");
    let facade: SyncStatus = status;
    assert!(facade.connected);
    assert_eq!(facade.remote_newer, Some(false));

    let result: ModelSyncDownloadResult = serde_json::from_value(serde_json::json!({
        "kind": "conflict",
        "remoteUpdatedAt": "2026-09-24T01:00:00Z"
    }))
    .expect("sync conflict");
    let facade: SyncDownloadResult = result;
    assert_eq!(
        serde_json::to_value(facade).expect("download result 직렬화"),
        serde_json::json!({
            "kind": "conflict",
            "remoteUpdatedAt": "2026-09-24T01:00:00Z"
        })
    );
}

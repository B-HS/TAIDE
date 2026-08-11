use serde::{Deserialize, Serialize};
use specta::Type;

use crate::domain::settings::service::SettingsPatch;

pub const SYNC_GIST_FILENAME: &str = "taide-settings.json";
pub const SYNC_GIST_DESCRIPTION: &str = "TAIDE sync payload — managed by TAIDE, do not edit manually";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncThemeEntry {
    pub id: String,
    pub json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncLocaleEntry {
    pub id: String,
    pub json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPayload {
    pub schema_version: u32,
    pub updated_at: String,
    pub settings: SettingsPatch,
    #[serde(default)]
    pub themes: Vec<SyncThemeEntry>,
    #[serde(default)]
    pub locales: Vec<SyncLocaleEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub connected: bool,
    pub has_gist: bool,
    pub last_synced_at: Option<String>,
    pub remote_newer: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SyncDownloadResult {
    #[serde(rename_all = "camelCase")]
    Applied { status: SyncStatus },
    #[serde(rename_all = "camelCase")]
    Conflict { remote_updated_at: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_status는_카멜케이스로_직렬화된다() {
        let status = SyncStatus {
            connected: true,
            has_gist: false,
            last_synced_at: None,
            remote_newer: Some(true),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#"{"connected":true,"hasGist":false,"lastSyncedAt":null,"remoteNewer":true}"#);
    }

    #[test]
    fn sync_download_result은_kind_태그로_구분된다() {
        let conflict = SyncDownloadResult::Conflict {
            remote_updated_at: "2026-08-11T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&conflict).unwrap();
        assert_eq!(json, r#"{"kind":"conflict","remoteUpdatedAt":"2026-08-11T00:00:00Z"}"#);
    }
}

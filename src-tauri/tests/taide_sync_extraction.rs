use taide_lib::domain::sync::service as legacy_service;
use taide_model::settings::{Settings, SettingsPatch};
use taide_model::sync::SyncPayload;

#[test]
fn 동기화_서비스는_독립_crate와_기존_경로에서_같다() {
    let extracted_time: fn(u64) -> String = taide_sync::service::format_unix_utc_iso8601;
    let legacy_time: fn(u64) -> String = legacy_service::format_unix_utc_iso8601;
    assert!(std::ptr::fn_addr_eq(extracted_time, legacy_time));
    assert_eq!(extracted_time(0), "1970-01-01T00:00:00Z");

    let extracted_patch: fn(&Settings) -> SettingsPatch = taide_sync::service::settings_to_sync_patch;
    let legacy_patch: fn(&Settings) -> SettingsPatch = legacy_service::settings_to_sync_patch;
    assert!(std::ptr::fn_addr_eq(extracted_patch, legacy_patch));
    let patch = extracted_patch(&Settings::default());
    assert!(patch.remote_access_enabled.is_none());
    assert!(patch.remote_allowed_hosts.is_none());
    assert!(patch.shell_override.is_none());

    let extracted_parse: fn(&str) -> Option<SyncPayload> = taide_sync::service::parse_synced_payload;
    let legacy_parse: fn(&str) -> Option<SyncPayload> = legacy_service::parse_synced_payload;
    assert!(std::ptr::fn_addr_eq(extracted_parse, legacy_parse));
    assert!(extracted_parse("{").is_none());
}

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::locale::service as locale_service;
use crate::domain::locale::types::LocalePack;
use crate::domain::settings::service as settings_service;
use crate::domain::settings::types::{Settings, SettingsPatch, SETTINGS_SCHEMA_VERSION};
use crate::domain::sync::types::{SyncLocaleEntry, SyncPayload, SyncThemeEntry};
use crate::domain::theme::service as theme_service;
use crate::domain::theme::types::Theme;
use crate::error::{AppError, AppResult};
use crate::paths::AppPaths;

const SECONDS_PER_DAY: i64 = 86_400;
const DAYS_FROM_CIVIL_EPOCH_OFFSET: i64 = 719_468;
const DAYS_PER_ERA: i64 = 146_097;
const YEARS_PER_ERA: i64 = 400;

fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + DAYS_FROM_CIVIL_EPOCH_OFFSET;
    let era = if z >= 0 { z } else { z - DAYS_PER_ERA + 1 } / DAYS_PER_ERA;
    let day_of_era = (z - era * DAYS_PER_ERA) as u64;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i64 + era * YEARS_PER_ERA;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
    let month = if month_index < 10 { month_index + 3 } else { month_index - 9 } as u32;
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}

pub fn format_unix_utc_iso8601(unix_seconds: u64) -> String {
    let unix_seconds = unix_seconds as i64;
    let days = unix_seconds.div_euclid(SECONDS_PER_DAY);
    let seconds_of_day = unix_seconds.rem_euclid(SECONDS_PER_DAY);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

pub fn now_utc_iso8601() -> String {
    let unix_seconds = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    format_unix_utc_iso8601(unix_seconds)
}

pub fn is_remote_newer(remote_updated_at: &str, last_synced_at: Option<&str>) -> bool {
    match last_synced_at {
        Some(last) => remote_updated_at > last,
        None => true,
    }
}

/// Fields on [`SettingsPatch`] that must never cross the sync gist boundary in
/// either direction. Uploads exclude these so a leaked/shared gist can't leak
/// the local shell override or gate state; downloads exclude the same fields
/// so a hand-edited or attacker-controlled gist can't remotely flip
/// `remoteAccessEnabled`/`remotePasswordOnlyLogin` (bypassing the link-gate),
/// grow `remoteAllowedHosts` (self-expanding the Host allowlist), or override
/// `shellOverride` (arbitrary executable spawned the next time a terminal
/// opens). [`settings_to_sync_patch`] (upload) and [`apply_payload_settings`]
/// (download) both funnel through this single function so the two directions
/// can never drift apart — see `docs/acknowledge/2026-08-15-wave-b-hardening-contract.md`
/// §3.1. This is distinct from `remote::dispatch::strip_remote_gated_settings_patch`,
/// which guards the live remote-session `settings_update` path, not the gist
/// round-trip.
fn strip_non_syncable(patch: &SettingsPatch) -> SettingsPatch {
    SettingsPatch {
        shell_override: None,
        remote_access_enabled: None,
        remote_password_only_login: None,
        remote_allowed_hosts: None,
        ..patch.clone()
    }
}

pub fn settings_to_sync_patch(settings: &Settings) -> SettingsPatch {
    strip_non_syncable(&SettingsPatch {
        theme_id: Some(settings.theme_id.clone()),
        editor_font_size: Some(settings.editor_font_size),
        terminal_font_size: Some(settings.terminal_font_size),
        shell_override: settings.shell_override.clone(),
        follow_system_theme: Some(settings.follow_system_theme),
        language: Some(settings.language.clone()),
        toast_position: Some(settings.toast_position.clone()),
        resizer_thickness: Some(settings.resizer_thickness),
        editor_font_family: settings.editor_font_family.clone(),
        terminal_font_family: settings.terminal_font_family.clone(),
        ui_font_family: settings.ui_font_family.clone(),
        format_on_save: Some(settings.format_on_save),
        auto_save_delay_ms: Some(settings.auto_save_delay_ms),
        keymap_overrides: settings.keymap_overrides.clone(),
        editor_minimap: Some(settings.editor_minimap),
        show_system_usage: Some(settings.show_system_usage),
        agent_status_badge_enabled: Some(settings.agent_status_badge_enabled),
        agent_hooks_enabled: Some(settings.agent_hooks_enabled),
        ide_integration_enabled: Some(settings.ide_integration_enabled),
        ide_auto_open_diff: Some(settings.ide_auto_open_diff),
        editor_word_wrap: Some(settings.editor_word_wrap),
        editor_line_numbers: Some(settings.editor_line_numbers),
        editor_tab_size: Some(settings.editor_tab_size),
        editor_insert_spaces: Some(settings.editor_insert_spaces),
        editor_detect_indentation: Some(settings.editor_detect_indentation),
        editor_render_whitespace: Some(settings.editor_render_whitespace),
        editor_bracket_pair_colorization: Some(settings.editor_bracket_pair_colorization),
        editor_font_ligatures: Some(settings.editor_font_ligatures),
        editor_cursor_style: Some(settings.editor_cursor_style),
        editor_cursor_blinking: Some(settings.editor_cursor_blinking),
        editor_scroll_beyond_last_line: Some(settings.editor_scroll_beyond_last_line),
        editor_sticky_scroll_enabled: Some(settings.editor_sticky_scroll_enabled),
        terminal_scrollback: Some(settings.terminal_scrollback),
        terminal_cursor_style: Some(settings.terminal_cursor_style),
        terminal_cursor_blink: Some(settings.terminal_cursor_blink),
        enable_preview_tabs: Some(settings.enable_preview_tabs),
        ai_auto_tab_enabled: Some(settings.ai_auto_tab_enabled),
        ai_provider: settings.ai_provider,
        ai_model: settings.ai_model.clone(),
        ai_omlx_base_url: settings.ai_omlx_base_url.clone(),
        remote_access_enabled: Some(settings.remote_access_enabled),
        remote_password_only_login: Some(settings.remote_password_only_login),
        remote_allowed_hosts: Some(settings.remote_allowed_hosts.clone()),
        organize_imports_on_save: Some(settings.organize_imports_on_save),
        fix_all_on_save: Some(settings.fix_all_on_save),
        editor_code_lens_enabled: Some(settings.editor_code_lens_enabled),
        editor_semantic_highlighting: Some(settings.editor_semantic_highlighting),
        editor_format_on_type: Some(settings.editor_format_on_type),
        editor_format_on_paste: Some(settings.editor_format_on_paste),
        emmet_enabled: Some(settings.emmet_enabled),
        recent_searches: Some(settings.recent_searches.clone()),
        zen_fullscreen: Some(settings.zen_fullscreen),
        zen_hide_status_bar: Some(settings.zen_hide_status_bar),
    })
}

pub fn assemble_payload(
    settings: &Settings,
    themes: Vec<SyncThemeEntry>,
    locales: Vec<SyncLocaleEntry>,
    updated_at: String,
) -> SyncPayload {
    SyncPayload {
        schema_version: SETTINGS_SCHEMA_VERSION,
        updated_at,
        settings: settings_to_sync_patch(settings),
        themes,
        locales,
    }
}

fn read_json_entries(dir: &Path) -> Vec<(String, String)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut result = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Some(id) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        result.push((id.to_string(), content));
    }
    result
}

pub fn collect_theme_entries(paths: &AppPaths) -> Vec<SyncThemeEntry> {
    read_json_entries(&paths.themes_dir())
        .into_iter()
        .map(|(id, json)| SyncThemeEntry { id, json })
        .collect()
}

pub fn collect_locale_entries(paths: &AppPaths) -> Vec<SyncLocaleEntry> {
    read_json_entries(&paths.locales_dir())
        .into_iter()
        .map(|(id, json)| SyncLocaleEntry { id, json })
        .collect()
}

fn parse_theme_entry(entry: &SyncThemeEntry) -> AppResult<Theme> {
    serde_json::from_str(&entry.json).map_err(|_| AppError::Internal(format!("theme '{}' in the sync payload is malformed", entry.id)))
}

fn parse_locale_entry(entry: &SyncLocaleEntry) -> AppResult<LocalePack> {
    serde_json::from_str(&entry.json).map_err(|_| AppError::Internal(format!("locale '{}' in the sync payload is malformed", entry.id)))
}

pub fn apply_theme_entries(paths: &AppPaths, entries: &[SyncThemeEntry]) {
    for entry in entries {
        match parse_theme_entry(entry) {
            Ok(theme) => {
                if let Err(error) = theme_service::save_theme(paths, &theme) {
                    log::warn!("동기화 테마 적용 실패 ({}): {error}", entry.id);
                }
            }
            Err(error) => log::warn!("동기화 테마 파싱 실패 ({}): {error}", entry.id),
        }
    }
}

pub fn apply_locale_entries(paths: &AppPaths, entries: &[SyncLocaleEntry]) {
    for entry in entries {
        match parse_locale_entry(entry) {
            Ok(pack) => {
                if let Err(error) = locale_service::save_locale(paths, &pack) {
                    log::warn!("동기화 로케일 적용 실패 ({}): {error}", entry.id);
                }
            }
            Err(error) => log::warn!("동기화 로케일 파싱 실패 ({}): {error}", entry.id),
        }
    }
}

/// Rejects a downloaded payload whose `schemaVersion` is explicitly *greater* than this build's
/// `SETTINGS_SCHEMA_VERSION`. Mirrors the version-gate `layout::service::load_layout` already
/// applies to `LAYOUT_SCHEMA_VERSION`.
///
/// `X1#6` — this comparison alone does **not** detect "a payload has fields this build doesn't
/// recognize," despite the doc that motivated it (below) describing exactly that scenario:
/// `SETTINGS_SCHEMA_VERSION` is policy-frozen at `1` (`ipc-contract.md` — new `Settings` fields are
/// added without bumping it, since every field is `#[serde(default)]` and additive), so a payload
/// from a newer TAIDE build carries the *same* `schemaVersion` this build has, passes this check
/// unconditionally, and any field it added that this build doesn't know still gets silently dropped
/// on deserialize rather than rejected. The gate only ever fires for a payload claiming a schema
/// version number *higher* than `1` — which, under the current freeze policy, no build ever writes.
/// See `스키마_버전이_같아도_알수없는_필드는_거부없이_조용히_버려진다` below, which pins this as
/// current, intentional-until-changed behavior rather than a bug to silently patch here. A real
/// unknown-field detector would need a field whitelist compared against the payload's raw JSON keys
/// — a policy change (the codebase would start rejecting instead of silently dropping unrecognized
/// settings), so it wants an explicit decision before being added, not just a gate tightening.
pub fn ensure_supported_schema_version(schema_version: u32) -> AppResult<()> {
    if schema_version > SETTINGS_SCHEMA_VERSION {
        return Err(AppError::InvalidArgument(format!(
            "sync payload schema version {schema_version} is newer than the supported version {SETTINGS_SCHEMA_VERSION} — update TAIDE to sync"
        )));
    }
    Ok(())
}

pub fn apply_payload_settings(current: &Settings, payload: &SyncPayload) -> Settings {
    settings_service::apply_patch(current, &strip_non_syncable(&payload.settings))
}

/// Parses a downloaded gist body into a [`SyncPayload`], migrating any pre-rename
/// `aiAutoTabProvider`/`aiAutoTabModel` keys and normalizing any out-of-range bounded-enum value
/// (`editorRenderWhitespace`/`editorCursorStyle`/`editorCursorBlinking`/`terminalCursorStyle`/
/// `aiProvider` — e.g. from a gist written by a different TAIDE build) in its nested `settings`
/// object first — see [`settings_service::migrate_legacy_ai_provider_keys`]'s and
/// [`settings_service::sanitize_legacy_settings_values`]'s doc comments for why these are raw-JSON
/// passes rather than `#[serde(alias = ...)]`/`deserialize_with`. `None` covers both an
/// unparseable JSON body and a body that doesn't match [`SyncPayload`]'s shape — `sync_download`
/// reports either the same way to the caller ("sync payload from the gist was malformed"), so the
/// two aren't distinguished here.
pub fn parse_synced_payload(content: &str) -> Option<SyncPayload> {
    let mut raw: serde_json::Value = serde_json::from_str(content).ok()?;
    if let Some(settings_object) = raw.get_mut("settings").and_then(|value| value.as_object_mut()) {
        settings_service::migrate_legacy_ai_provider_keys(settings_object);
        settings_service::sanitize_legacy_settings_values(settings_object);
    }
    serde_json::from_value(raw).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ai::types::AiProviderId;

    #[test]
    fn 유닉스_epoch는_1970_1_1로_변환된다() {
        assert_eq!(format_unix_utc_iso8601(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn 하루_끝_초는_날짜가_넘어가지_않는다() {
        assert_eq!(format_unix_utc_iso8601(86_399), "1970-01-01T23:59:59Z");
        assert_eq!(format_unix_utc_iso8601(86_400), "1970-01-02T00:00:00Z");
    }

    #[test]
    fn 알려진_기준_시각들을_정확히_변환한다() {
        assert_eq!(format_unix_utc_iso8601(946_684_800), "2000-01-01T00:00:00Z");
        assert_eq!(format_unix_utc_iso8601(1_000_000_000), "2001-09-09T01:46:40Z");
        assert_eq!(format_unix_utc_iso8601(1_700_000_000), "2023-11-14T22:13:20Z");
        assert_eq!(format_unix_utc_iso8601(1_735_689_599), "2024-12-31T23:59:59Z");
        assert_eq!(format_unix_utc_iso8601(1_738_368_000), "2025-02-01T00:00:00Z");
    }

    #[test]
    fn now_utc_iso8601은_고정폭_형식을_반환한다() {
        let value = now_utc_iso8601();
        assert_eq!(value.len(), "2026-08-11T00:00:00Z".len());
        assert!(value.ends_with('Z'));
    }

    #[test]
    fn 한번도_동기화하지_않았으면_원격이_항상_더_새롭다() {
        assert!(is_remote_newer("2026-01-01T00:00:00Z", None));
    }

    #[test]
    fn 문자열_비교로_시각_선후를_판단한다() {
        assert!(is_remote_newer("2026-08-11T12:00:00Z", Some("2026-08-11T11:59:59Z")));
        assert!(!is_remote_newer("2026-08-11T11:00:00Z", Some("2026-08-11T11:59:59Z")));
        assert!(!is_remote_newer("2026-08-11T11:59:59Z", Some("2026-08-11T11:59:59Z")));
    }

    #[test]
    fn settings_to_sync_patch는_shell_override를_제외한다() {
        let settings = Settings {
            shell_override: Some("/bin/zsh".to_string()),
            theme_id: "taide-light".to_string(),
            ..Settings::default()
        };

        let patch = settings_to_sync_patch(&settings);

        assert_eq!(patch.shell_override, None);
        assert_eq!(patch.theme_id, Some("taide-light".to_string()));
    }

    #[test]
    fn settings_to_sync_patch는_recent_searches를_포함한다() {
        let settings = Settings {
            recent_searches: vec!["needle".to_string(), "haystack".to_string()],
            ..Settings::default()
        };

        let patch = settings_to_sync_patch(&settings);

        assert_eq!(patch.recent_searches, Some(vec!["needle".to_string(), "haystack".to_string()]));
    }

    #[test]
    fn settings_to_sync_patch는_sync_gist_id를_페이로드에_담지_않는다() {
        let settings = Settings {
            sync_gist_id: Some("gist-123".to_string()),
            ..Settings::default()
        };

        let payload = assemble_payload(&settings, Vec::new(), Vec::new(), now_utc_iso8601());
        let serialized = serde_json::to_string(&payload).expect("serialize payload");

        assert!(!serialized.contains("gist-123"));
        assert!(!serialized.contains("syncGistId"));
    }

    #[test]
    fn 지원하는_스키마_버전은_통과한다() {
        assert!(ensure_supported_schema_version(SETTINGS_SCHEMA_VERSION).is_ok());
    }

    #[test]
    fn 낮은_스키마_버전도_통과한다() {
        assert!(ensure_supported_schema_version(0).is_ok());
    }

    #[test]
    fn 지원보다_높은_스키마_버전은_거부된다() {
        let result = ensure_supported_schema_version(SETTINGS_SCHEMA_VERSION + 1);
        assert!(matches!(result, Err(AppError::InvalidArgument(_))));
    }

    /// `X1#6` — pins `ensure_supported_schema_version`'s actual reach (see that function's doc
    /// comment): a gist payload with the *same* `schemaVersion` this build has, but a field this
    /// build has never heard of, passes the gate unrejected and the unknown field is silently
    /// dropped rather than causing any error. This documents that as current, deliberate-until-a
    /// policy-decision behavior — not a gap this test is meant to close.
    #[test]
    fn 스키마_버전이_같아도_알수없는_필드는_거부없이_조용히_버려진다() {
        let payload_from_a_newer_build = r#"{
            "schemaVersion": 1,
            "updatedAt": "2026-08-15T00:00:00Z",
            "settings": {
                "themeId": "taide-light",
                "aBrandNewFieldThisBuildHasNeverHeardOf": "some-future-value"
            }
        }"#;

        let payload: SyncPayload =
            serde_json::from_str(payload_from_a_newer_build).expect("알 수 없는 필드가 있어도 파싱은 성공해야 함(serde default)");

        assert!(
            ensure_supported_schema_version(payload.schema_version).is_ok(),
            "schemaVersion 이 같으므로 게이트를 통과해야 한다(현재 동작)"
        );
        assert_eq!(
            payload.settings.theme_id,
            Some("taide-light".to_string()),
            "인식하는 필드는 정상 반영된다"
        );
    }

    /// Regresses the gist-inbound filter gap the contract calls out explicitly:
    /// `apply_payload_settings는_settings_update와_동일한_apply_patch를_재사용한다`
    /// below only ever exercises payloads built through [`assemble_payload`],
    /// which already excludes `shellOverride`/the remote gate fields at the
    /// source — it can never reach the code path a hand-edited or
    /// attacker-controlled gist takes. This test deserializes a raw JSON
    /// string (as `sync_download` does with the gist body) so the filter is
    /// exercised on the actual untrusted-input boundary, not just on payloads
    /// this build already knows how to produce safely.
    #[test]
    fn 손으로_만든_gist_페이로드의_shell_override와_원격_게이트_필드는_적용되지_않는다() {
        let malicious_json = r#"{
            "schemaVersion": 1,
            "updatedAt": "2026-08-15T00:00:00Z",
            "settings": {
                "shellOverride": "/tmp/evil.sh",
                "remoteAccessEnabled": true,
                "remotePasswordOnlyLogin": true,
                "remoteAllowedHosts": ["attacker.example.com"]
            }
        }"#;
        let payload: SyncPayload = serde_json::from_str(malicious_json).expect("손으로 작성한 페이로드가 파싱되어야 함");

        let current = Settings::default();
        let applied = apply_payload_settings(&current, &payload);

        assert_eq!(applied.shell_override, current.shell_override);
        assert_eq!(applied.remote_access_enabled, current.remote_access_enabled);
        assert_eq!(applied.remote_password_only_login, current.remote_password_only_login);
        assert_eq!(applied.remote_allowed_hosts, current.remote_allowed_hosts);
    }

    /// A gist uploaded before the `ai_auto_tab_provider`/`ai_auto_tab_model` → `ai_provider`/
    /// `ai_model` rename (`docs/acknowledge/2026-08-16-wave-g-ai-contract.md` §2-2/§3.1) still
    /// carries the old field names — [`parse_synced_payload`] (the same parse path
    /// `sync_download` uses) must migrate them to the renamed fields, exercised here the same way
    /// as the shell-override/remote-gate regression above: a raw hand-written JSON string, not a
    /// payload this build already knows how to produce safely.
    #[test]
    fn 구_필드명의_gist_페이로드도_ai_provider_ai_model로_읽힌다() {
        let legacy_json = r#"{
            "schemaVersion": 1,
            "updatedAt": "2026-08-15T00:00:00Z",
            "settings": {
                "aiAutoTabProvider": "ollamaCloud",
                "aiAutoTabModel": "qwen2.5-coder"
            }
        }"#;
        let payload = parse_synced_payload(legacy_json).expect("구 필드명 gist 페이로드가 파싱되어야 함");

        let current = Settings::default();
        let applied = apply_payload_settings(&current, &payload);

        assert_eq!(applied.ai_provider, Some(AiProviderId::OllamaCloud));
        assert_eq!(applied.ai_model, Some("qwen2.5-coder".to_string()));
    }

    #[test]
    fn parse_synced_payload는_신_필드명_gist_페이로드도_그대로_파싱한다() {
        let json = r#"{
            "schemaVersion": 1,
            "updatedAt": "2026-08-15T00:00:00Z",
            "settings": {
                "aiProvider": "codex",
                "aiModel": "gpt-5.6-sol"
            }
        }"#;
        let payload = parse_synced_payload(json).expect("신 필드명 gist 페이로드가 파싱되어야 함");

        assert_eq!(payload.settings.ai_provider, Some(AiProviderId::Codex));
        assert_eq!(payload.settings.ai_model, Some("gpt-5.6-sol".to_string()));
    }

    #[test]
    fn parse_synced_payload는_깨진_json에_none을_반환한다() {
        assert!(parse_synced_payload("{not json").is_none());
    }

    #[test]
    fn strip_non_syncable은_그_외_필드는_그대로_통과시킨다() {
        let patch = SettingsPatch {
            theme_id: Some("taide-light".to_string()),
            editor_font_size: Some(18),
            shell_override: Some("/bin/zsh".to_string()),
            remote_access_enabled: Some(true),
            remote_password_only_login: Some(true),
            remote_allowed_hosts: Some(vec!["tunnel.example.com".to_string()]),
            ..SettingsPatch::default()
        };

        let stripped = strip_non_syncable(&patch);

        assert_eq!(stripped.theme_id, Some("taide-light".to_string()));
        assert_eq!(stripped.editor_font_size, Some(18));
        assert_eq!(stripped.shell_override, None);
        assert_eq!(stripped.remote_access_enabled, None);
        assert_eq!(stripped.remote_password_only_login, None);
        assert_eq!(stripped.remote_allowed_hosts, None);
    }

    #[test]
    fn settings_to_sync_patch는_원격_게이트와_허용_호스트를_제외한다() {
        let settings = Settings {
            remote_access_enabled: true,
            remote_password_only_login: true,
            remote_allowed_hosts: vec!["tunnel.example.com".to_string()],
            ..Settings::default()
        };

        let patch = settings_to_sync_patch(&settings);

        assert_eq!(patch.remote_access_enabled, None);
        assert_eq!(patch.remote_password_only_login, None);
        assert_eq!(patch.remote_allowed_hosts, None);
    }

    #[test]
    fn apply_payload_settings는_settings_update와_동일한_apply_patch를_재사용한다() {
        let current = Settings::default();
        let mut synced = Settings {
            editor_font_size: 42,
            ..Settings::default()
        };
        synced.shell_override = Some("/bin/fish".to_string());

        let payload = assemble_payload(&synced, Vec::new(), Vec::new(), now_utc_iso8601());
        let applied = apply_payload_settings(&current, &payload);

        assert_eq!(applied.editor_font_size, 42);
        assert_eq!(applied.shell_override, current.shell_override);
    }

    fn temp_data_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("taide-sync-{name}-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn collect_theme_entries는_사용자_테마_파일만_수집한다() {
        let paths = AppPaths::new(temp_data_dir("collect-themes"));
        std::fs::create_dir_all(paths.themes_dir()).expect("create themes dir");
        let mut theme = theme_service::builtin_dark();
        theme.id = "my-theme".to_string();
        theme_service::save_theme(&paths, &theme).expect("save theme");

        let entries = collect_theme_entries(&paths);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "my-theme");
        assert!(entries[0].json.contains("my-theme"));

        std::fs::remove_dir_all(paths.themes_dir()).ok();
    }

    #[test]
    fn apply_theme_entries는_다운로드한_테마를_저장한다() {
        let paths = AppPaths::new(temp_data_dir("apply-themes"));
        std::fs::create_dir_all(paths.themes_dir()).expect("create themes dir");
        let mut theme = theme_service::builtin_dark();
        theme.id = "synced-theme".to_string();
        let json = serde_json::to_string(&theme).expect("serialize theme");

        apply_theme_entries(
            &paths,
            &[SyncThemeEntry {
                id: theme.id.clone(),
                json,
            }],
        );

        assert!(theme_service::theme_exists(&paths, "synced-theme"));

        std::fs::remove_dir_all(paths.themes_dir()).ok();
    }

    #[test]
    fn apply_theme_entries는_손상된_엔트리를_건너뛰고_계속한다() {
        let paths = AppPaths::new(temp_data_dir("apply-themes-broken"));
        std::fs::create_dir_all(paths.themes_dir()).expect("create themes dir");
        let mut good_theme = theme_service::builtin_dark();
        good_theme.id = "good-theme".to_string();
        let good_json = serde_json::to_string(&good_theme).expect("serialize theme");

        apply_theme_entries(
            &paths,
            &[
                SyncThemeEntry {
                    id: "broken".to_string(),
                    json: "{not json".to_string(),
                },
                SyncThemeEntry {
                    id: good_theme.id.clone(),
                    json: good_json,
                },
            ],
        );

        assert!(theme_service::theme_exists(&paths, "good-theme"));
        assert!(!theme_service::theme_exists(&paths, "broken"));

        std::fs::remove_dir_all(paths.themes_dir()).ok();
    }

    #[test]
    fn apply_locale_entries는_다운로드한_로케일을_저장한다() {
        let paths = AppPaths::new(temp_data_dir("apply-locales"));
        std::fs::create_dir_all(paths.locales_dir()).expect("create locales dir");
        let pack = LocalePack {
            version: crate::domain::locale::types::LOCALE_SCHEMA_VERSION,
            id: "synced-locale".to_string(),
            name: "Synced".to_string(),
            extends: Some(locale_service::BUILTIN_EN_ID.to_string()),
            messages: std::collections::BTreeMap::new(),
        };
        let json = serde_json::to_string(&pack).expect("serialize locale");

        apply_locale_entries(&paths, &[SyncLocaleEntry { id: pack.id.clone(), json }]);

        assert!(locale_service::locale_exists(&paths, "synced-locale"));

        std::fs::remove_dir_all(paths.locales_dir()).ok();
    }
}

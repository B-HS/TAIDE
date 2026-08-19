use serde::Serialize;

use crate::domain::ai::types::AiProviderId;
use crate::domain::remote::types::ALLOWED_HOST_WILDCARD_PREFIX;
use crate::domain::settings::types::{
    EditorCursorBlinking, EditorCursorStyle, EditorRenderWhitespace, Settings, SettingsPatch, TerminalCursorStyle,
};
use crate::domain::theme::service as theme_service;
use crate::error::{AppError, AppResult};
use crate::infra::persist;
use crate::paths::AppPaths;

/// Legacy → current key renames applied by [`migrate_legacy_ai_provider_keys`] — see that
/// function's doc comment for why this is a raw-JSON migration rather than `#[serde(alias)]`.
const LEGACY_AI_PROVIDER_KEY_RENAMES: &[(&str, &str)] = &[("aiAutoTabProvider", "aiProvider"), ("aiAutoTabModel", "aiModel")];

/// Renames pre-rename `aiAutoTabProvider`/`aiAutoTabModel` JSON keys (the `Settings`/
/// `SettingsPatch` field names before
/// `docs/acknowledge/2026-08-16-wave-g-ai-contract.md` §2-2/§3.1) to their current `aiProvider`/
/// `aiModel` names, in place, on an untyped JSON object — applied before deserializing into
/// `Settings`/`SettingsPatch` at every boundary a pre-rename value could still arrive from (a
/// hand-edited/legacy `settings.json` via [`load_settings`], a gist payload synced before the
/// rename via `sync::commands::sync_download`).
///
/// This is a `serde_json::Value` pre-pass rather than `#[serde(alias = "aiAutoTabProvider")]` on
/// the struct fields directly, because with the `specta` version this project pins, a field alias
/// makes `Settings`'/`SettingsPatch`'s Serialize and Deserialize shapes diverge — which splits
/// every TS consumer of those types (including ones with no relation to these two fields) into
/// `_Serialize`/`_Deserialize`/union type variants.
pub fn migrate_legacy_ai_provider_keys(object: &mut serde_json::Map<String, serde_json::Value>) {
    for (legacy_key, current_key) in LEGACY_AI_PROVIDER_KEY_RENAMES {
        if object.contains_key(*current_key) {
            continue;
        }
        if let Some(legacy_value) = object.remove(*legacy_key) {
            object.insert((*current_key).to_string(), legacy_value);
        }
    }
}

/// Normalizes a present-but-out-of-range (or empty-string) value at `object[field_name]` back to
/// `fallback` by attempting `T`'s own [`serde::Deserialize`] impl — the allowed-value check is
/// derived from the real enum type (`EditorRenderWhitespace`/`EditorCursorStyle`/
/// `EditorCursorBlinking`/`TerminalCursorStyle`/`Option<AiProviderId>`) rather than a
/// hand-maintained `&[&str]` list, so a variant added to the enum never needs a second place kept
/// in sync. Only touches a key that is *present* — an absent key is left for the containing
/// struct's own `#[serde(default)]` to fill in, matching [`migrate_legacy_ai_provider_keys`]'s
/// scope of "touch present-but-wrong values only". Used by [`sanitize_legacy_settings_values`].
fn sanitize_legacy_enum_field<T: serde::de::DeserializeOwned>(
    object: &mut serde_json::Map<String, serde_json::Value>,
    field_name: &str,
    fallback: serde_json::Value,
) {
    let Some(value) = object.get(field_name) else { return };
    if serde_json::from_value::<T>(value.clone()).is_err() {
        object.insert(field_name.to_string(), fallback);
    }
}

/// Normalizes every bounded-enum `Settings`/`SettingsPatch` field a hand-edited `settings.json`, an
/// `AppFile` "Settings" tab save, or a synced gist's nested `settings` object might still carry an
/// out-of-range or pre-narrowing string value for: `editorRenderWhitespace`/`editorCursorStyle`/
/// `editorCursorBlinking`/`terminalCursorStyle` fall back to the field's own default, and
/// `aiProvider` (no meaningful default provider) falls back to `null` — the same fallback each had
/// under the old `sanitize_enum`/`sanitize_optional_enum` runtime checks before these fields were
/// `specta`-narrowed enums. Applied to the raw JSON object *before* typed deserialization — the same boundary
/// [`migrate_legacy_ai_provider_keys`] uses and for the same reason: a `deserialize_with` on the
/// typed field would make `Settings`' Serialize and Deserialize shapes diverge, splitting every TS
/// consumer into `_Serialize`/`_Deserialize` variants (see `Settings::ai_provider`'s doc comment).
/// See `docs/acknowledge/2026-08-18-audit-t1-batch1-contract.md` T1-B. `pub` so
/// `crate::domain::sync::service::parse_synced_payload` can apply the same normalization to a
/// downloaded gist's nested `settings` object, mirroring how it already calls
/// [`migrate_legacy_ai_provider_keys`] there.
pub fn sanitize_legacy_settings_values(object: &mut serde_json::Map<String, serde_json::Value>) {
    sanitize_legacy_enum_field::<EditorRenderWhitespace>(
        object,
        "editorRenderWhitespace",
        default_value_json(EditorRenderWhitespace::default()),
    );
    sanitize_legacy_enum_field::<EditorCursorStyle>(object, "editorCursorStyle", default_value_json(EditorCursorStyle::default()));
    sanitize_legacy_enum_field::<EditorCursorBlinking>(object, "editorCursorBlinking", default_value_json(EditorCursorBlinking::default()));
    sanitize_legacy_enum_field::<TerminalCursorStyle>(object, "terminalCursorStyle", default_value_json(TerminalCursorStyle::default()));
    sanitize_legacy_enum_field::<Option<AiProviderId>>(object, "aiProvider", serde_json::Value::Null);
}

/// `T` is always one of the fieldless unit-variant enums [`sanitize_legacy_settings_values`] passes
/// (`EditorRenderWhitespace`/`EditorCursorStyle`/`EditorCursorBlinking`/`TerminalCursorStyle`), whose
/// serialization cannot fail — but this is still a production code path (settings-file load, `AppFile`
/// save, gist sync), so it resolves that impossibility with a harmless fallback rather than an
/// `.expect()`/`.unwrap()` that would turn a future refactor's mistake into a panic instead of the
/// ordinary `AppResult` error [`sanitize_legacy_enum_field`]'s caller already propagates.
fn default_value_json<T: Serialize>(value: T) -> serde_json::Value {
    serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
}

/// Applies [`migrate_legacy_ai_provider_keys`] and [`sanitize_legacy_settings_values`] to a raw
/// JSON object before converting it into `Settings` — the shared core of both
/// [`read_settings_file`] (loading `settings.json` from disk) and [`parse_settings_json`]
/// (validating a hand-edited `AppFile` save), so a pre-rename or otherwise legacy-shaped payload is
/// migrated identically through either entry point.
fn settings_from_value(mut raw: serde_json::Value) -> AppResult<Settings> {
    if let Some(object) = raw.as_object_mut() {
        migrate_legacy_ai_provider_keys(object);
        sanitize_legacy_settings_values(object);
    }
    Ok(serde_json::from_value(raw)?)
}

/// Reads and type-checks `settings.json`, applying [`migrate_legacy_ai_provider_keys`] to its raw
/// JSON object first.
fn read_settings_file(path: &std::path::Path) -> AppResult<Option<Settings>> {
    let Some(raw) = persist::read_json::<serde_json::Value>(path)? else {
        return Ok(None);
    };
    Ok(Some(settings_from_value(raw)?))
}

/// Parses a hand-edited `settings.json` save (the `AppFile` tab's "Settings" target) into
/// `Settings`, applying the same legacy-key migration [`read_settings_file`] applies. Kept separate
/// from `read_settings_file` because it parses a `String` already in memory (an unsaved editor
/// buffer), not a file on disk, and reports a parse/schema failure as
/// [`AppError::InvalidArgument`] — a save-time error the caller should surface to the user and
/// reject, not the load-time "fall back to defaults and back up the corrupt file" behavior
/// [`load_settings`] uses for a `settings.json` the app itself can no longer make sense of.
pub fn parse_settings_json(content: &str) -> AppResult<Settings> {
    let raw: serde_json::Value = serde_json::from_str(content)
        .map_err(|error| AppError::InvalidArgument(format!("settings.json이 유효한 JSON이 아닙니다: {error}")))?;
    settings_from_value(raw).map_err(|error| AppError::InvalidArgument(format!("settings.json 스키마가 올바르지 않습니다: {error}")))
}

pub fn load_settings(paths: &AppPaths) -> Settings {
    let path = paths.settings_file();
    match read_settings_file(&path) {
        Ok(Some(settings)) => {
            let sanitized = sanitize(settings.clone());
            if sanitized != settings {
                let _ = persist::write_json(&path, &sanitized);
            }
            sanitized
        }
        Ok(None) => Settings::default(),
        Err(_) => {
            backup_corrupted(&path);
            let defaults = Settings::default();
            let _ = persist::write_json(&path, &defaults);
            defaults
        }
    }
}

pub fn save_settings(paths: &AppPaths, settings: &Settings) -> AppResult<()> {
    persist::write_json(&paths.settings_file(), settings)
}

fn backup_corrupted(path: &std::path::Path) {
    let backup_path = path.with_extension("json.bak");
    let _ = std::fs::rename(path, backup_path);
}

const EDITOR_TAB_SIZE_MIN: u32 = 1;
const EDITOR_TAB_SIZE_MAX: u32 = 8;
const DNS_HOSTNAME_MAX_LEN: usize = 253;
const DNS_LABEL_MAX_LEN: usize = 63;
const TERMINAL_SCROLLBACK_MIN: u32 = 100;
const TERMINAL_SCROLLBACK_MAX: u32 = 100_000;
const RESIZER_THICKNESS_MIN: u32 = 0;
const RESIZER_THICKNESS_MAX: u32 = 8;
/// Shared by `editor_font_size` and `terminal_font_size` — both faced the same three-way drift
/// (Rust left them unclamped; the frontend had two different ranges of its own,
/// `settings-view.tsx`'s local `MIN_FONT_SIZE`/`MAX_FONT_SIZE` at 8–32 and
/// `shared/constants/code-font-size.ts`'s `MIN_CODE_FONT_SIZE`/`MAX_CODE_FONT_SIZE` at 6–48 — audit
/// `docs/quality-assurance/2026-08-18-architecture-audit.md` R5#4). 6–48 was chosen as the single
/// range because it's the one actually wired into a live control path
/// (`widgets/window-chrome/status-bar-content.tsx`'s zoom stepper imports `code-font-size.ts`
/// directly), while `settings-view.tsx`'s 8–32 pair is a local, unimported literal the slider alone
/// used — Wave F reconciles the frontend side onto this same pair. Kept as one shared pair (not a
/// separate `EDITOR_FONT_SIZE_MIN`/`TERMINAL_FONT_SIZE_MIN` pair with the same values) so the two
/// settings can't drift apart the same way again.
const FONT_SIZE_MIN: u32 = 6;
const FONT_SIZE_MAX: u32 = 48;

/// Mirrors the frontend's `SEARCH_HISTORY_LIMIT`
/// (`src/entities/search/search-history.ts`). The frontend already caps
/// `recent_searches` at this length before every `settings_update`, but this
/// field also arrives via [`apply_payload_settings`] (sync gist download),
/// which the frontend cap can't guard — a malformed or hand-edited gist
/// payload could otherwise store an unbounded array. Enforced here so every
/// entry point is covered, the same defense-in-depth as [`sanitize_allowed_hosts`].
const RECENT_SEARCHES_MAX: usize = 20;

fn sanitize_optional_url(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let authority = v.strip_prefix("http://").or_else(|| v.strip_prefix("https://"))?;
        let host = authority.split(['/', '?', '#']).next().unwrap_or("");
        if host.is_empty() {
            return None;
        }
        Some(v.trim_end_matches('/').to_string())
    })
}

fn is_valid_dns_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= DNS_LABEL_MAX_LEN
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Accepts only a bare hostname or IPv4 literal for a `remote_allowed_hosts`
/// entry — no scheme, userinfo, path/query, whitespace, or port — with one
/// exception: a leading `*.` marks the entry as an RFC 6125 single-label
/// wildcard (`remote::service::host_matches_allowed_entry` is the matcher).
/// `remote::service::is_allowed_host` matches entries against the `Host`
/// header's hostname alone, and `remote::service::format_issue_link_url`
/// interpolates the first non-wildcard entry verbatim into `https://{host}/...`;
/// anything else either never matches (silently disabling the tunnel) or
/// corrupts the issued link (a scheme yields `https://https://...`, and
/// `real.example.com@attacker.example` would make a browser send the link's
/// one-time token to `attacker.example` as if `real.example.com` were
/// userinfo). See `docs/acknowledge/2026-08-15-wave-b-hardening-contract.md` §6.
///
/// The wildcard's remainder is validated the same way a bare hostname is,
/// plus one extra requirement: it must itself contain at least one more `.`
/// (i.e. two or more labels), so a bare `*` (no `*.` prefix, falls through to
/// the non-wildcard branch and fails there since `*` isn't a valid DNS label)
/// and `*.com` (single-label remainder) are both rejected — the latter would
/// otherwise register a wildcard broad enough to match almost any TLD-only
/// hostname. A wildcard elsewhere than the very front (`foo.*.com`) or fused
/// into a label (`*foo.com`) never matches the literal `*.` prefix check at
/// all, so both fall through to the non-wildcard branch and are rejected
/// there too (`*` isn't a valid DNS label).
fn is_valid_allowed_host(value: &str) -> bool {
    match value.strip_prefix(ALLOWED_HOST_WILDCARD_PREFIX) {
        Some(remainder) => {
            remainder.contains('.') && remainder.len() <= DNS_HOSTNAME_MAX_LEN && remainder.split('.').all(is_valid_dns_label)
        }
        None => !value.is_empty() && value.len() <= DNS_HOSTNAME_MAX_LEN && value.split('.').all(is_valid_dns_label),
    }
}

/// Trims, lower-cases, and drops malformed entries from a
/// `remote_allowed_hosts` patch/disk value. Applied inside [`sanitize`] so
/// every entry point (patch or hand-edited `settings.json`) is covered, the
/// same defense-in-depth already used for `ai_omlx_base_url`
/// ([`sanitize_optional_url`]).
fn sanitize_allowed_hosts(hosts: Vec<String>) -> Vec<String> {
    hosts
        .into_iter()
        .map(|host| host.trim().to_ascii_lowercase())
        .filter(|host| is_valid_allowed_host(host))
        .collect()
}

fn sanitize_recent_searches(mut searches: Vec<String>) -> Vec<String> {
    searches.truncate(RECENT_SEARCHES_MAX);
    searches
}

/// Merges a patch value for a "clearable" `Option<String>` settings field: `None` leaves the field
/// untouched (the patch omitted it), `Some("")` clears it back to `None`, and any other `Some(value)`
/// replaces it. A plain `Option<String>` patch field can otherwise only express two states (touch /
/// don't touch), which collapses "clear this back to the system default" and "leave it alone" into
/// the same `None` on the wire — `ai_omlx_base_url` was the one field that already worked around
/// this with its own empty-string convention; this generalizes that convention to every other
/// clearable string field (font family pickers' "System Default", the shell override picker's
/// "Default shell", the AI model picker) instead of leaving them all with the same gap
/// `font-picker.tsx`'s "System Default" selection hit. `ai_provider` no longer goes through this —
/// it's `Option<AiProviderId>` now (T1-B), an enum with no "empty" variant to carry a clear
/// sentinel, so its merge is a plain `Option::or` (see [`apply_patch`]). See
/// `docs/acknowledge/2026-08-18-audit-t0-fix-contract.md` §1 결정 7.
fn merge_clearable_string(patch_value: Option<&String>, existing: Option<&String>) -> Option<String> {
    match patch_value {
        None => existing.cloned(),
        Some(value) if value.is_empty() => None,
        Some(value) => Some(value.clone()),
    }
}

fn merge_ai_omlx_base_url(patch_value: Option<&String>, existing: Option<&String>) -> Option<String> {
    match patch_value {
        Some(value) if !value.is_empty() => sanitize_optional_url(Some(value.clone())).or_else(|| existing.cloned()),
        _ => merge_clearable_string(patch_value, existing),
    }
}

/// 숫자 범위 필드를 clamp 로 보정한다. `Settings` 가 만들어지는 모든 출구(`apply_patch` · 디스크
/// 로드)에서 항상 거친다 — patch 든 손으로 편집한 settings.json 이든 검증되지 않은 값이 Monaco/xterm
/// 런타임까지 그대로 흘러가는 것을 막는다. `editorRenderWhitespace`/`editorCursorStyle`/
/// `editorCursorBlinking`/`terminalCursorStyle`/`aiProvider` 는 더 이상 여기서 허용목록을 검사하지
/// 않는다 — specta 로 좁힌 실제 enum 타입이 유효하지 않은 값을 애초에 표현할 수 없게 됐고(런타임
/// patch·프로그램적 구성 경로), 디스크/동기화처럼 타입 밖에서 오는 원시 JSON 값은
/// [`sanitize_legacy_settings_values`]가 타입 파싱 이전에 정규화한다.
pub fn sanitize(settings: Settings) -> Settings {
    Settings {
        editor_font_size: settings.editor_font_size.clamp(FONT_SIZE_MIN, FONT_SIZE_MAX),
        terminal_font_size: settings.terminal_font_size.clamp(FONT_SIZE_MIN, FONT_SIZE_MAX),
        editor_tab_size: settings.editor_tab_size.clamp(EDITOR_TAB_SIZE_MIN, EDITOR_TAB_SIZE_MAX),
        terminal_scrollback: settings.terminal_scrollback.clamp(TERMINAL_SCROLLBACK_MIN, TERMINAL_SCROLLBACK_MAX),
        resizer_thickness: settings.resizer_thickness.clamp(RESIZER_THICKNESS_MIN, RESIZER_THICKNESS_MAX),
        ai_omlx_base_url: sanitize_optional_url(settings.ai_omlx_base_url),
        remote_allowed_hosts: sanitize_allowed_hosts(settings.remote_allowed_hosts),
        recent_searches: sanitize_recent_searches(settings.recent_searches),
        ..settings
    }
}

pub fn apply_patch(settings: &Settings, patch: &SettingsPatch) -> Settings {
    sanitize(Settings {
        version: settings.version,
        theme_id: patch.theme_id.clone().unwrap_or_else(|| settings.theme_id.clone()),
        editor_font_size: patch.editor_font_size.unwrap_or(settings.editor_font_size),
        terminal_font_size: patch.terminal_font_size.unwrap_or(settings.terminal_font_size),
        shell_override: merge_clearable_string(patch.shell_override.as_ref(), settings.shell_override.as_ref()),
        follow_system_theme: patch.follow_system_theme.unwrap_or(settings.follow_system_theme),
        language: patch.language.clone().unwrap_or_else(|| settings.language.clone()),
        toast_position: patch.toast_position.clone().unwrap_or_else(|| settings.toast_position.clone()),
        resizer_thickness: patch.resizer_thickness.unwrap_or(settings.resizer_thickness),
        editor_font_family: merge_clearable_string(patch.editor_font_family.as_ref(), settings.editor_font_family.as_ref()),
        terminal_font_family: merge_clearable_string(patch.terminal_font_family.as_ref(), settings.terminal_font_family.as_ref()),
        ui_font_family: merge_clearable_string(patch.ui_font_family.as_ref(), settings.ui_font_family.as_ref()),
        format_on_save: patch.format_on_save.unwrap_or(settings.format_on_save),
        auto_save_delay_ms: patch.auto_save_delay_ms.unwrap_or(settings.auto_save_delay_ms),
        keymap_overrides: patch.keymap_overrides.clone().or_else(|| settings.keymap_overrides.clone()),
        editor_minimap: patch.editor_minimap.unwrap_or(settings.editor_minimap),
        show_system_usage: patch.show_system_usage.unwrap_or(settings.show_system_usage),
        agent_status_badge_enabled: patch.agent_status_badge_enabled.unwrap_or(settings.agent_status_badge_enabled),
        agent_hooks_enabled: patch.agent_hooks_enabled.unwrap_or(settings.agent_hooks_enabled),
        ide_integration_enabled: patch.ide_integration_enabled.unwrap_or(settings.ide_integration_enabled),
        ide_auto_open_diff: patch.ide_auto_open_diff.unwrap_or(settings.ide_auto_open_diff),
        editor_word_wrap: patch.editor_word_wrap.unwrap_or(settings.editor_word_wrap),
        editor_line_numbers: patch.editor_line_numbers.unwrap_or(settings.editor_line_numbers),
        editor_tab_size: patch.editor_tab_size.unwrap_or(settings.editor_tab_size),
        editor_insert_spaces: patch.editor_insert_spaces.unwrap_or(settings.editor_insert_spaces),
        editor_detect_indentation: patch.editor_detect_indentation.unwrap_or(settings.editor_detect_indentation),
        editor_render_whitespace: patch.editor_render_whitespace.unwrap_or(settings.editor_render_whitespace),
        editor_bracket_pair_colorization: patch
            .editor_bracket_pair_colorization
            .unwrap_or(settings.editor_bracket_pair_colorization),
        editor_font_ligatures: patch.editor_font_ligatures.unwrap_or(settings.editor_font_ligatures),
        editor_cursor_style: patch.editor_cursor_style.unwrap_or(settings.editor_cursor_style),
        editor_cursor_blinking: patch.editor_cursor_blinking.unwrap_or(settings.editor_cursor_blinking),
        editor_scroll_beyond_last_line: patch
            .editor_scroll_beyond_last_line
            .unwrap_or(settings.editor_scroll_beyond_last_line),
        editor_sticky_scroll_enabled: patch.editor_sticky_scroll_enabled.unwrap_or(settings.editor_sticky_scroll_enabled),
        terminal_scrollback: patch.terminal_scrollback.unwrap_or(settings.terminal_scrollback),
        terminal_cursor_style: patch.terminal_cursor_style.unwrap_or(settings.terminal_cursor_style),
        terminal_cursor_blink: patch.terminal_cursor_blink.unwrap_or(settings.terminal_cursor_blink),
        enable_preview_tabs: patch.enable_preview_tabs.unwrap_or(settings.enable_preview_tabs),
        ai_auto_tab_enabled: patch.ai_auto_tab_enabled.unwrap_or(settings.ai_auto_tab_enabled),
        ai_provider: patch.ai_provider.or(settings.ai_provider),
        ai_model: merge_clearable_string(patch.ai_model.as_ref(), settings.ai_model.as_ref()),
        ai_omlx_base_url: merge_ai_omlx_base_url(patch.ai_omlx_base_url.as_ref(), settings.ai_omlx_base_url.as_ref()),
        sync_gist_id: settings.sync_gist_id.clone(),
        sync_last_synced_at: settings.sync_last_synced_at.clone(),
        remote_access_enabled: patch.remote_access_enabled.unwrap_or(settings.remote_access_enabled),
        remote_password_only_login: patch.remote_password_only_login.unwrap_or(settings.remote_password_only_login),
        remote_allowed_hosts: patch
            .remote_allowed_hosts
            .clone()
            .unwrap_or_else(|| settings.remote_allowed_hosts.clone()),
        organize_imports_on_save: patch.organize_imports_on_save.unwrap_or(settings.organize_imports_on_save),
        fix_all_on_save: patch.fix_all_on_save.unwrap_or(settings.fix_all_on_save),
        editor_code_lens_enabled: patch.editor_code_lens_enabled.unwrap_or(settings.editor_code_lens_enabled),
        editor_semantic_highlighting: patch.editor_semantic_highlighting.unwrap_or(settings.editor_semantic_highlighting),
        editor_format_on_type: patch.editor_format_on_type.unwrap_or(settings.editor_format_on_type),
        editor_format_on_paste: patch.editor_format_on_paste.unwrap_or(settings.editor_format_on_paste),
        emmet_enabled: patch.emmet_enabled.unwrap_or(settings.emmet_enabled),
        recent_searches: patch.recent_searches.clone().unwrap_or_else(|| settings.recent_searches.clone()),
        zen_fullscreen: patch.zen_fullscreen.unwrap_or(settings.zen_fullscreen),
        zen_hide_status_bar: patch.zen_hide_status_bar.unwrap_or(settings.zen_hide_status_bar),
    })
}

/// Explicitly picking a theme also turns `follow_system_theme` off. Without this, a theme chosen
/// from the picker while that flag was on would be silently ignored — `theme::commands::theme_get_current`
/// keeps resolving the OS theme every time, so the picker's own selection never actually shows,
/// with no indication in the UI that the pick was overridden (see
/// `docs/acknowledge/2026-08-18-hand-qa-fix-contract.md` §2.2). Does **not** persist — unlike every
/// other settings mutation in this module, the caller (`commands::settings_set_theme`) routes the
/// returned value through `commands::apply_and_broadcast` for the save, so this second visible
/// field flip reaches other windows/remote sessions/the `settings.json` `AppFile` tab the same way
/// `settings_update` does, instead of only the narrower `ThemeChanged` event this command also
/// emits for its own theme-CSS-application purpose.
pub fn set_theme(paths: &AppPaths, settings: &Settings, theme_id: &str) -> AppResult<Settings> {
    if !theme_service::theme_exists(paths, theme_id) {
        return Err(AppError::NotFound(format!("theme not found: {theme_id}")));
    }

    Ok(Settings {
        theme_id: theme_id.to_string(),
        follow_system_theme: false,
        ..settings.clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_data_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("taide-settings-{name}-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn patch의_none_필드는_기존값을_보존한다() {
        let settings = Settings {
            editor_font_size: 20,
            ..Settings::default()
        };
        let patch = SettingsPatch {
            terminal_font_size: Some(16),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.editor_font_size, 20);
        assert_eq!(updated.terminal_font_size, 16);
        assert_eq!(updated.theme_id, settings.theme_id);
        assert_eq!(updated.follow_system_theme, settings.follow_system_theme);
    }

    #[test]
    fn patch의_some_필드는_값을_덮어쓴다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            theme_id: Some("taide-light".to_string()),
            follow_system_theme: Some(true),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.theme_id, "taide-light");
        assert!(updated.follow_system_theme);
    }

    #[test]
    fn patch로_저장시_포맷과_자동저장_지연을_설정한다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            format_on_save: Some(true),
            auto_save_delay_ms: Some(1_500),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.format_on_save);
        assert_eq!(updated.auto_save_delay_ms, 1_500);
    }

    #[test]
    fn patch로_미니맵과_리소스_표시_agent_ide_설정을_변경한다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            editor_minimap: Some(false),
            show_system_usage: Some(false),
            agent_status_badge_enabled: Some(false),
            agent_hooks_enabled: Some(true),
            ide_integration_enabled: Some(false),
            ide_auto_open_diff: Some(false),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(!updated.editor_minimap);
        assert!(!updated.show_system_usage);
        assert!(!updated.agent_status_badge_enabled);
        assert!(updated.agent_hooks_enabled);
        assert!(!updated.ide_integration_enabled);
        assert!(!updated.ide_auto_open_diff);
    }

    #[test]
    fn patch로_키맵_오버라이드_json을_설정한다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            keymap_overrides: Some("[]".to_string()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.keymap_overrides, Some("[]".to_string()));
    }

    #[test]
    fn patch로_에디터와_터미널_신규_설정_15필드를_변경한다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            editor_word_wrap: Some(true),
            editor_line_numbers: Some(false),
            editor_tab_size: Some(2),
            editor_insert_spaces: Some(false),
            editor_detect_indentation: Some(false),
            editor_render_whitespace: Some(EditorRenderWhitespace::All),
            editor_bracket_pair_colorization: Some(false),
            editor_font_ligatures: Some(true),
            editor_cursor_style: Some(EditorCursorStyle::Block),
            editor_cursor_blinking: Some(EditorCursorBlinking::Smooth),
            editor_scroll_beyond_last_line: Some(false),
            terminal_scrollback: Some(5_000),
            terminal_cursor_style: Some(TerminalCursorStyle::Underline),
            terminal_cursor_blink: Some(false),
            enable_preview_tabs: Some(false),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.editor_word_wrap);
        assert!(!updated.editor_line_numbers);
        assert_eq!(updated.editor_tab_size, 2);
        assert!(!updated.editor_insert_spaces);
        assert!(!updated.editor_detect_indentation);
        assert_eq!(updated.editor_render_whitespace, EditorRenderWhitespace::All);
        assert!(!updated.editor_bracket_pair_colorization);
        assert!(updated.editor_font_ligatures);
        assert_eq!(updated.editor_cursor_style, EditorCursorStyle::Block);
        assert_eq!(updated.editor_cursor_blinking, EditorCursorBlinking::Smooth);
        assert!(!updated.editor_scroll_beyond_last_line);
        assert_eq!(updated.terminal_scrollback, 5_000);
        assert_eq!(updated.terminal_cursor_style, TerminalCursorStyle::Underline);
        assert!(!updated.terminal_cursor_blink);
        assert!(!updated.enable_preview_tabs);
    }

    /// `editorCursorStyle`/`editorCursorBlinking`/`editorRenderWhitespace`/`terminalCursorStyle`
    /// patch values are now `EditorCursorStyle`/`EditorCursorBlinking`/`EditorRenderWhitespace`/
    /// `TerminalCursorStyle` (T1-B) — an out-of-range *string* for them can no longer be
    /// constructed as a `SettingsPatch` at all (that's now a compile error, not a runtime value to
    /// sanitize), so this test only covers the remaining un-typed numeric ranges. The JSON-level
    /// equivalent for the four enum fields (a hand-edited `settings.json` carrying an invalid
    /// string) is covered by
    /// [`손으로_편집된_settings_파일의_잘못된_값은_로드시_보정되어_재저장된다`] below, which exercises
    /// [`sanitize_legacy_settings_values`] instead.
    #[test]
    fn 범위를_벗어난_숫자는_보정된다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            editor_tab_size: Some(100),
            terminal_scrollback: Some(1),
            resizer_thickness: Some(999),
            editor_font_size: Some(999),
            terminal_font_size: Some(0),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.editor_tab_size, 8);
        assert_eq!(updated.terminal_scrollback, 100);
        assert_eq!(updated.resizer_thickness, 8);
        assert_eq!(updated.editor_font_size, FONT_SIZE_MAX);
        assert_eq!(updated.terminal_font_size, FONT_SIZE_MIN);
    }

    #[test]
    fn patch로_auto_tab_설정을_변경한다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            ai_auto_tab_enabled: Some(true),
            ai_provider: Some(AiProviderId::OllamaCloud),
            ai_model: Some("qwen2.5-coder".to_string()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.ai_auto_tab_enabled);
        assert_eq!(updated.ai_provider, Some(AiProviderId::OllamaCloud));
        assert_eq!(updated.ai_model, Some("qwen2.5-coder".to_string()));
    }

    /// `aiProvider` is `Option<AiProviderId>` now (T1-B), so an out-of-range provider id can no
    /// longer reach `apply_patch` as a typed `SettingsPatch` value at all — the equivalent
    /// scenario is a *raw JSON* value from a hand-edited `settings.json`/older synced gist, which
    /// [`sanitize_legacy_settings_values`] normalizes before typed parse.
    #[test]
    fn 허용목록_밖의_ai_provider는_raw_json_단계에서_null로_보정된다() {
        let mut value: serde_json::Value = serde_json::json!({ "aiProvider": "anthropic" });
        sanitize_legacy_settings_values(value.as_object_mut().expect("object"));

        assert_eq!(value.get("aiProvider"), Some(&serde_json::Value::Null));
    }

    #[test]
    fn 구_필드명_객체는_마이그레이션_후_ai_provider_키로_파싱된다() {
        let mut value: serde_json::Value = serde_json::from_str(r#"{"aiAutoTabProvider":"ollamaCloud","aiAutoTabModel":"qwen2.5-coder"}"#)
            .expect("구 필드명 JSON이 파싱되어야 함");
        migrate_legacy_ai_provider_keys(value.as_object_mut().expect("object"));

        let patch: SettingsPatch = serde_json::from_value(value).expect("마이그레이션된 JSON이 SettingsPatch로 파싱되어야 함");

        assert_eq!(patch.ai_provider, Some(AiProviderId::OllamaCloud));
        assert_eq!(patch.ai_model, Some("qwen2.5-coder".to_string()));
    }

    #[test]
    fn 신_필드명이_이미_있으면_구_필드명_마이그레이션은_덮어쓰지_않는다() {
        let mut value: serde_json::Value =
            serde_json::from_str(r#"{"aiAutoTabProvider":"codex","aiProvider":"omlx"}"#).expect("JSON이 파싱되어야 함");
        migrate_legacy_ai_provider_keys(value.as_object_mut().expect("object"));

        let patch: SettingsPatch = serde_json::from_value(value).expect("마이그레이션된 JSON이 SettingsPatch로 파싱되어야 함");

        assert_eq!(patch.ai_provider, Some(AiProviderId::Omlx));
    }

    #[test]
    fn 구_필드명_ai_auto_tab_provider로_저장된_settings_json도_ai_provider로_읽힌다() {
        let paths = AppPaths::new(temp_data_dir("legacy-ai-provider-field"));
        let path = paths.settings_file();
        std::fs::create_dir_all(path.parent().unwrap()).expect("create dir");
        std::fs::write(
            &path,
            br#"{"version":1,"aiAutoTabProvider":"codex","aiAutoTabModel":"gpt-5.6-sol"}"#,
        )
        .expect("write legacy settings file");

        let settings = load_settings(&paths);

        assert_eq!(settings.ai_provider, Some(AiProviderId::Codex));
        assert_eq!(settings.ai_model, Some("gpt-5.6-sol".to_string()));

        std::fs::remove_dir_all(paths.data_dir).ok();
    }

    #[test]
    fn patch로_omlx_base_url을_설정하면_trailing_slash가_제거된다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            ai_omlx_base_url: Some("http://localhost:8000/".to_string()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.ai_omlx_base_url, Some("http://localhost:8000".to_string()));
    }

    #[test]
    fn http_https로_시작하지_않는_omlx_base_url은_none으로_보정된다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            ai_omlx_base_url: Some("ftp://localhost:8000".to_string()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.ai_omlx_base_url, None);
    }

    #[test]
    fn 스킴만_있고_host가_없는_omlx_base_url은_none으로_보정된다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            ai_omlx_base_url: Some("http://".to_string()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.ai_omlx_base_url, None);
    }

    #[test]
    fn 빈_문자열_패치는_기존_omlx_base_url을_해제한다() {
        let settings = Settings {
            ai_omlx_base_url: Some("http://localhost:8000".to_string()),
            ..Settings::default()
        };
        let patch = SettingsPatch {
            ai_omlx_base_url: Some(String::new()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.ai_omlx_base_url, None);
    }

    #[test]
    fn 유효하지_않은_omlx_base_url_패치는_기존_값을_보존한다() {
        let settings = Settings {
            ai_omlx_base_url: Some("http://localhost:8000".to_string()),
            ..Settings::default()
        };
        let patch = SettingsPatch {
            ai_omlx_base_url: Some("localhost:8123".to_string()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.ai_omlx_base_url, Some("http://localhost:8000".to_string()));
    }

    #[test]
    fn patch에_omlx_base_url이_없으면_기존_값을_유지한다() {
        let settings = Settings {
            ai_omlx_base_url: Some("http://localhost:8000".to_string()),
            ..Settings::default()
        };
        let patch = SettingsPatch {
            editor_font_size: Some(16),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.ai_omlx_base_url, Some("http://localhost:8000".to_string()));
    }

    #[test]
    fn 빈_문자열_패치는_해제_가능한_옵션_문자열_필드를_모두_해제한다() {
        let settings = Settings {
            shell_override: Some("/bin/zsh".to_string()),
            editor_font_family: Some("Fira Code".to_string()),
            terminal_font_family: Some("Fira Code".to_string()),
            ui_font_family: Some("Fira Code".to_string()),
            ai_model: Some("gpt-5".to_string()),
            ..Settings::default()
        };
        let patch = SettingsPatch {
            shell_override: Some(String::new()),
            editor_font_family: Some(String::new()),
            terminal_font_family: Some(String::new()),
            ui_font_family: Some(String::new()),
            ai_model: Some(String::new()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.shell_override, None);
        assert_eq!(updated.editor_font_family, None);
        assert_eq!(updated.terminal_font_family, None);
        assert_eq!(updated.ui_font_family, None);
        assert_eq!(updated.ai_model, None);
    }

    /// `ai_provider` (`Option<AiProviderId>`, T1-B) has no "empty" variant to carry a clear
    /// sentinel through, unlike the `Option<String>` fields the test above covers — an omitted
    /// patch value keeps the existing provider (`Option::or`, see [`apply_patch`]), and there is no
    /// way to explicitly clear it back to "unconfigured" via `SettingsPatch` (same gap R5#1
    /// documents for other fields; unchanged by this narrowing).
    #[test]
    fn patch에_ai_provider가_없으면_기존_provider를_유지한다() {
        let settings = Settings {
            ai_provider: Some(AiProviderId::Codex),
            ..Settings::default()
        };

        let updated = apply_patch(&settings, &SettingsPatch::default());

        assert_eq!(updated.ai_provider, Some(AiProviderId::Codex));
    }

    #[test]
    fn 패치를_생략한_옵션_문자열_필드는_구_클라이언트처럼_기존값을_그대로_보존한다() {
        let settings = Settings {
            shell_override: Some("/bin/zsh".to_string()),
            editor_font_family: Some("Fira Code".to_string()),
            terminal_font_family: Some("Fira Code".to_string()),
            ui_font_family: Some("Fira Code".to_string()),
            ai_provider: Some(AiProviderId::Codex),
            ai_model: Some("gpt-5".to_string()),
            ..Settings::default()
        };
        let patch = SettingsPatch {
            editor_font_size: Some(16),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.shell_override, settings.shell_override);
        assert_eq!(updated.editor_font_family, settings.editor_font_family);
        assert_eq!(updated.terminal_font_family, settings.terminal_font_family);
        assert_eq!(updated.ui_font_family, settings.ui_font_family);
        assert_eq!(updated.ai_provider, settings.ai_provider);
        assert_eq!(updated.ai_model, settings.ai_model);
    }

    #[test]
    fn patch로_원격_접속_활성화를_변경한다() {
        let settings = Settings::default();
        assert!(!settings.remote_access_enabled);

        let patch = SettingsPatch {
            remote_access_enabled: Some(true),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.remote_access_enabled);
    }

    #[test]
    fn patch로_원격_비밀번호만_로그인_허용을_변경한다() {
        let settings = Settings::default();
        assert!(!settings.remote_password_only_login);

        let patch = SettingsPatch {
            remote_password_only_login: Some(true),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.remote_password_only_login);
    }

    #[test]
    fn patch로_원격_허용_호스트_목록을_변경한다() {
        let settings = Settings::default();
        assert!(settings.remote_allowed_hosts.is_empty());

        let patch = SettingsPatch {
            remote_allowed_hosts: Some(vec!["tunnel.example.com".to_string()]),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.remote_allowed_hosts, vec!["tunnel.example.com".to_string()]);
    }

    #[test]
    fn 스킴이_포함된_허용_호스트는_걸러진다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            remote_allowed_hosts: Some(vec!["https://tunnel.example.com".to_string(), "tunnel.example.com".to_string()]),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.remote_allowed_hosts, vec!["tunnel.example.com".to_string()]);
    }

    #[test]
    fn userinfo가_포함된_허용_호스트는_걸러진다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            remote_allowed_hosts: Some(vec!["real.example.com@attacker.example".to_string()]),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(
            updated.remote_allowed_hosts.is_empty(),
            "userinfo 형태의 허용 호스트가 링크 토큰 탈취 경로로 저장되면 안 된다"
        );
    }

    #[test]
    fn 포트나_공백이_포함된_허용_호스트는_걸러진다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            remote_allowed_hosts: Some(vec!["tunnel.example.com:8080".to_string(), "tunnel example.com".to_string()]),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.remote_allowed_hosts.is_empty());
    }

    #[test]
    fn 유효한_호스트명과_ipv4는_소문자로_정규화되어_저장된다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            remote_allowed_hosts: Some(vec!["Tunnel.Example.Com".to_string(), " 192.168.1.5 ".to_string()]),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(
            updated.remote_allowed_hosts,
            vec!["tunnel.example.com".to_string(), "192.168.1.5".to_string()]
        );
    }

    #[test]
    fn 접미사가_두_레이블_이상인_와일드카드_허용_호스트는_그대로_저장된다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            remote_allowed_hosts: Some(vec!["*.Trycloudflare.Com".to_string()]),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.remote_allowed_hosts, vec!["*.trycloudflare.com".to_string()]);
    }

    #[test]
    fn 맨몸_별표는_허용_호스트에서_걸러진다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            remote_allowed_hosts: Some(vec!["*".to_string()]),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.remote_allowed_hosts.is_empty());
    }

    #[test]
    fn 접미사가_한_레이블뿐인_와일드카드는_허용_호스트에서_걸러진다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            remote_allowed_hosts: Some(vec!["*.com".to_string()]),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(
            updated.remote_allowed_hosts.is_empty(),
            "*.com 처럼 TLD 하나만 남는 와일드카드는 폭이 너무 넓다"
        );
    }

    #[test]
    fn 중간에_별표가_있는_허용_호스트는_걸러진다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            remote_allowed_hosts: Some(vec!["foo.*.com".to_string()]),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.remote_allowed_hosts.is_empty());
    }

    #[test]
    fn 레이블에_별표가_섞인_허용_호스트는_걸러진다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            remote_allowed_hosts: Some(vec!["*foo.com".to_string()]),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.remote_allowed_hosts.is_empty());
    }

    #[test]
    fn 와일드카드가_여러_개_섞인_허용_호스트는_걸러진다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            remote_allowed_hosts: Some(vec!["*.*.com".to_string()]),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.remote_allowed_hosts.is_empty());
    }

    #[test]
    fn patch로_organize_imports_fix_all_code_lens_설정을_변경한다() {
        let settings = Settings::default();
        assert!(!settings.organize_imports_on_save);
        assert!(!settings.fix_all_on_save);
        assert!(settings.editor_code_lens_enabled);

        let patch = SettingsPatch {
            organize_imports_on_save: Some(true),
            fix_all_on_save: Some(true),
            editor_code_lens_enabled: Some(false),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.organize_imports_on_save);
        assert!(updated.fix_all_on_save);
        assert!(!updated.editor_code_lens_enabled);
    }

    #[test]
    fn patch로_semantic_하이라이팅_포매팅_emmet_설정을_변경한다() {
        let settings = Settings::default();
        assert!(settings.editor_semantic_highlighting);
        assert!(!settings.editor_format_on_type);
        assert!(!settings.editor_format_on_paste);
        assert!(settings.emmet_enabled);

        let patch = SettingsPatch {
            editor_semantic_highlighting: Some(false),
            editor_format_on_type: Some(true),
            editor_format_on_paste: Some(true),
            emmet_enabled: Some(false),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(!updated.editor_semantic_highlighting);
        assert!(updated.editor_format_on_type);
        assert!(updated.editor_format_on_paste);
        assert!(!updated.emmet_enabled);
    }

    #[test]
    fn patch로_recent_searches를_변경한다() {
        let settings = Settings::default();
        assert!(settings.recent_searches.is_empty());

        let patch = SettingsPatch {
            recent_searches: Some(vec!["needle".to_string(), "haystack".to_string()]),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.recent_searches, vec!["needle".to_string(), "haystack".to_string()]);
    }

    #[test]
    fn patch에_recent_searches가_없으면_기존_값을_유지한다() {
        let settings = Settings {
            recent_searches: vec!["needle".to_string()],
            ..Settings::default()
        };

        let updated = apply_patch(&settings, &SettingsPatch::default());

        assert_eq!(updated.recent_searches, vec!["needle".to_string()]);
    }

    #[test]
    fn recent_searches는_상한을_초과하면_잘려나간다() {
        let settings = Settings::default();
        let oversized: Vec<String> = (0..(RECENT_SEARCHES_MAX + 10)).map(|index| format!("term-{index}")).collect();
        let patch = SettingsPatch {
            recent_searches: Some(oversized.clone()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.recent_searches.len(), RECENT_SEARCHES_MAX);
        assert_eq!(updated.recent_searches, oversized[..RECENT_SEARCHES_MAX]);
    }

    #[test]
    fn resizer_thickness는_0을_허용한다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            resizer_thickness: Some(0),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.resizer_thickness, 0);
    }

    #[test]
    fn 없는_settings_파일은_기본값을_반환한다() {
        let paths = AppPaths::new(temp_data_dir("missing"));
        let settings = load_settings(&paths);
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn 파손된_settings_파일은_bak로_밀리고_기본값을_반환한다() {
        let paths = AppPaths::new(temp_data_dir("corrupt"));
        let path = paths.settings_file();
        std::fs::create_dir_all(path.parent().unwrap()).expect("create dir");
        std::fs::write(&path, b"{not json at all").expect("write corrupt file");

        let settings = load_settings(&paths);

        assert_eq!(settings, Settings::default());
        assert!(path.with_extension("json.bak").exists());

        std::fs::remove_dir_all(paths.data_dir).ok();
    }

    /// `T1-B` backward-compat regression: a `settings.json` written before
    /// `editorRenderWhitespace`/`editorCursorStyle`/`editorCursorBlinking`/`terminalCursorStyle`/
    /// `aiProvider` were narrowed to real enums (or simply hand-edited with a typo/out-of-range
    /// value for any of them) must still load — normalized field-by-field back to defaults/`null`
    /// by [`sanitize_legacy_settings_values`] before typed `Settings` parse — instead of the whole
    /// file being treated as corrupt (backed up, every other field lost).
    #[test]
    fn 손으로_편집된_settings_파일의_잘못된_값은_로드시_보정되어_재저장된다() {
        let paths = AppPaths::new(temp_data_dir("hand-edited"));
        let path = paths.settings_file();
        std::fs::create_dir_all(path.parent().unwrap()).expect("create dir");
        std::fs::write(
            &path,
            br#"{
                "version": 1,
                "terminalCursorStyle": "banana",
                "editorCursorStyle": "vertical",
                "editorCursorBlinking": "",
                "editorRenderWhitespace": "trailing",
                "aiProvider": "anthropic",
                "editorTabSize": 0,
                "terminalScrollback": 99999999,
                "editorFontSize": 0,
                "terminalFontSize": 999
            }"#,
        )
        .expect("write settings file");

        let settings = load_settings(&paths);

        assert_eq!(settings.terminal_cursor_style, TerminalCursorStyle::default());
        assert_eq!(settings.editor_cursor_style, EditorCursorStyle::default());
        assert_eq!(settings.editor_cursor_blinking, EditorCursorBlinking::default());
        assert_eq!(settings.editor_render_whitespace, EditorRenderWhitespace::default());
        assert_eq!(settings.ai_provider, None);
        assert_eq!(settings.editor_tab_size, EDITOR_TAB_SIZE_MIN);
        assert_eq!(settings.terminal_scrollback, TERMINAL_SCROLLBACK_MAX);
        assert_eq!(settings.editor_font_size, FONT_SIZE_MIN);
        assert_eq!(settings.terminal_font_size, FONT_SIZE_MAX);

        let reloaded = persist::read_json::<Settings>(&path).expect("reread").expect("some");
        assert_eq!(reloaded, settings);

        std::fs::remove_dir_all(paths.data_dir).ok();
    }

    #[test]
    fn patch로_zen_설정을_변경한다() {
        let settings = Settings::default();
        assert!(!settings.zen_fullscreen);
        assert!(settings.zen_hide_status_bar);

        let patch = SettingsPatch {
            zen_fullscreen: Some(true),
            zen_hide_status_bar: Some(false),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.zen_fullscreen);
        assert!(!updated.zen_hide_status_bar);
    }

    #[test]
    fn set_theme는_존재하는_테마만_허용한다() {
        let paths = AppPaths::new(temp_data_dir("set-theme"));
        std::fs::create_dir_all(paths.themes_dir()).expect("create themes dir");
        let settings = Settings::default();

        let ok = set_theme(&paths, &settings, theme_service::BUILTIN_LIGHT_ID);
        assert!(ok.is_ok());
        assert_eq!(ok.unwrap().theme_id, theme_service::BUILTIN_LIGHT_ID);

        let err = set_theme(&paths, &settings, "does-not-exist");
        assert!(matches!(err, Err(AppError::NotFound(_))));

        std::fs::remove_dir_all(paths.data_dir).ok();
    }

    #[test]
    fn set_theme는_시스템_테마_따라가기를_자동으로_해제한다() {
        let paths = AppPaths::new(temp_data_dir("set-theme-follow-system"));
        std::fs::create_dir_all(paths.themes_dir()).expect("create themes dir");
        let settings = Settings {
            follow_system_theme: true,
            ..Settings::default()
        };

        let updated = set_theme(&paths, &settings, theme_service::BUILTIN_LIGHT_ID).expect("set_theme 성공");

        assert!(!updated.follow_system_theme, "테마를 명시적으로 고르면 시스템 팔로우가 꺼져야 한다");
        assert_eq!(updated.theme_id, theme_service::BUILTIN_LIGHT_ID);

        std::fs::remove_dir_all(paths.data_dir).ok();
    }
}

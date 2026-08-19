import type { SettingsPatch } from '@shared/api/bindings'

/**
 * `SettingsPatch` fields whose changed value can flip what `theme_get_current` resolves to
 * (`src-tauri/src/domain/theme/commands.rs`'s `theme_get_current` reads only these two off
 * `state.settings`) — the set `useUpdateSettings` (contract F1#3) checks before invalidating
 * `QUERY_KEY.THEME.ALL`, so an unrelated patch (e.g. `editorFontSize`) no longer force-refetches
 * the theme.
 */
export const THEME_AFFECTING_SETTINGS_FIELDS = ['themeId', 'followSystemTheme'] as const satisfies ReadonlyArray<keyof SettingsPatch>

/**
 * `SettingsPatch` fields whose changed value can flip what `locale_get_current` resolves to
 * (`src-tauri/src/domain/locale/commands.rs`'s `locale_get_current` reads only `settings.language`)
 * — see {@link THEME_AFFECTING_SETTINGS_FIELDS} for the same contract applied to locale.
 */
export const LOCALE_AFFECTING_SETTINGS_FIELDS = ['language'] as const satisfies ReadonlyArray<keyof SettingsPatch>

/**
 * `SettingsPatch` fields whose changed value can flip what `remote_status` reports (remote access
 * being turned on/off) — kept alongside the theme/locale field maps so `useUpdateSettings` owns
 * every mutation-triggered invalidation the settings screen previously did itself (contract F5#10).
 */
export const REMOTE_STATUS_AFFECTING_SETTINGS_FIELDS = ['remoteAccessEnabled'] as const satisfies ReadonlyArray<keyof SettingsPatch>

/**
 * Whether `patch` actually changes any of `fields` — every `SettingsPatch` field is `null` unless
 * a caller explicitly set it (every call site spreads `emptySettingsPatch()` first), so `null`
 * reliably means "not part of this update".
 */
export const settingsPatchTouchesFields = (patch: SettingsPatch, fields: ReadonlyArray<keyof SettingsPatch>) =>
    fields.some((field) => patch[field] !== null)

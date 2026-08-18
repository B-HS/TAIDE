import type { Page } from '@playwright/test'
import { invokeIpc } from './ipc'

export type SettingsSnapshot = Record<string, unknown>

export const readSettingsSnapshot = (page: Page) => invokeIpc<SettingsSnapshot>(page, 'settings_get')

/**
 * Builds a `SettingsPatch` that re-applies every field present in `snapshot` verbatim, restoring
 * them to that exact value regardless of what a test changed in between. Fields the backend omits
 * from `settings_get` (never configured) stay omitted here too, which the `Option<T>` shape of
 * every `SettingsPatch` field treats as "leave unchanged".
 */
export const buildFullRestorePatch = (snapshot: SettingsSnapshot) =>
    Object.fromEntries(Object.entries(snapshot).map(([key, value]) => [key, value ?? null]))

/**
 * Reads current settings and re-applies them with `overrides` merged on top, as a full-field patch.
 * Used to steer settings the tests depend on (e.g. pinning `language: 'en'` for deterministic
 * locale-text assertions) without guessing whether the backend accepts a sparse patch.
 */
export const applySettingsOverride = async (page: Page, overrides: SettingsSnapshot) => {
    const current = await readSettingsSnapshot(page)
    const merged = { ...current, ...overrides }
    await invokeIpc(page, 'settings_update', { patch: buildFullRestorePatch(merged) })
    return merged
}

import type { ThemeType } from '@shared/api/bindings'
import { BUILTIN_THEME_ID } from '@entities/theme/theme-tokens'

export const builtinThemeIdForType = (themeType: ThemeType) => (themeType === 'dark' ? BUILTIN_THEME_ID.DARK : BUILTIN_THEME_ID.LIGHT)

type ThemeIdAfterDeleteInput = {
    deletedThemeId: string
    deletedThemeType: ThemeType
    activeThemeId: string | null | undefined
}

/**
 * The theme id that must become active before `theme_delete` removes `deletedThemeId` — the
 * same-type builtin when the theme being deleted is the active one, `null` when nothing needs to
 * change. `settings.themeId` is a plain string the backend never rewrites on delete, so deleting the
 * active theme left it pointing at a file that no longer exists: `theme_get_current` fails with
 * `NotFound`, `ThemeProvider` applies no theme at all (no CSS variables, no shiki theme), and the
 * next launch comes up with the error banner and a completely unhighlighted editor (audit §4-B B5).
 * Returning the id instead of performing the switch keeps the decision testable and lets the caller
 * order it *before* the delete — `settings_set_theme` validates that the target exists, so switching
 * first is also the only ordering with no window where the active id is dangling.
 */
export const resolveThemeIdAfterDelete = ({ deletedThemeId, deletedThemeType, activeThemeId }: ThemeIdAfterDeleteInput) =>
    deletedThemeId === activeThemeId ? builtinThemeIdForType(deletedThemeType) : null

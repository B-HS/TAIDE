import { useQuery } from '@tanstack/react-query'
import { CODE_FONT_SIZE_STEP, DEFAULT_CODE_FONT_SIZE, MAX_CODE_FONT_SIZE, MIN_CODE_FONT_SIZE } from '@shared/constants/code-font-size'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import { settingsQueryOptions, useUpdateSettings } from '@entities/settings/settings.query'

const clampEditorFontSize = (value: number) => Math.min(MAX_CODE_FONT_SIZE, Math.max(MIN_CODE_FONT_SIZE, value))

/**
 * The editor font size and the three ways to move it, in one place because its keyboard owner and
 * its on-screen owner are not in the same window. ⌘=/⌘− (`font-size-up`/`font-size-down`) used to be
 * registered only by `StatusBarContent`, which lives in `AppShell` — so an auxiliary editor window,
 * which mounts no status bar at all, had neither the shortcut nor any button and could only change
 * the size by walking over to the main window (audit #14/#15; the same class `docs/features/keymap.md`
 * §5 records for ⌘K ⌘S in d-51 F4). The registration now sits in `KeybindingsRuntimeProvider`, which
 * `app.tsx` mounts in both window branches, and the status bar's own buttons call back into this hook
 * so the two entry points cannot drift into different clamping or step arithmetic.
 *
 * Lives in `entities` rather than `shared/hooks` because it reads and writes settings through
 * `entities/settings` — `shared` may not import `entities` (eslint layer rule), and the consumers are
 * an `app` provider and a `widgets` status bar, both of which may.
 */
export const useEditorFontSize = () => {
    const { data: settings } = useQuery(settingsQueryOptions())
    const { mutate: updateSettings } = useUpdateSettings()

    const editorFontSize = settings?.editorFontSize ?? DEFAULT_CODE_FONT_SIZE

    return {
        editorFontSize,
        increaseEditorFontSize: () =>
            updateSettings({ ...emptySettingsPatch(), editorFontSize: clampEditorFontSize(editorFontSize + CODE_FONT_SIZE_STEP) }),
        decreaseEditorFontSize: () =>
            updateSettings({ ...emptySettingsPatch(), editorFontSize: clampEditorFontSize(editorFontSize - CODE_FONT_SIZE_STEP) }),
        resetEditorFontSize: () => updateSettings({ ...emptySettingsPatch(), editorFontSize: DEFAULT_CODE_FONT_SIZE }),
    }
}

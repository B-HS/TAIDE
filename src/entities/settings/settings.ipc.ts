import { commands } from '@shared/api/bindings'
import type { SettingsPatch } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'

export const emptySettingsPatch = (): SettingsPatch => ({
    themeId: null,
    editorFontSize: null,
    terminalFontSize: null,
    shellOverride: null,
    followSystemTheme: null,
    language: null,
    toastPosition: null,
    resizerThickness: null,
    editorFontFamily: null,
    terminalFontFamily: null,
    uiFontFamily: null,
    formatOnSave: null,
    autoSaveDelayMs: null,
    keymapOverrides: null,
    editorMinimap: null,
    showSystemUsage: null,
    agentStatusBadgeEnabled: null,
    agentHooksEnabled: null,
    ideIntegrationEnabled: null,
    ideAutoOpenDiff: null,
})

export const getSettings = () => unwrapResult(commands.settingsGet())

export const updateSettings = (patch: SettingsPatch) => unwrapResult(commands.settingsUpdate(patch))

export const setThemeId = (themeId: string) => unwrapResult(commands.settingsSetTheme(themeId))

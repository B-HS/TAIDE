import { describe, expect, test } from 'bun:test'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import {
    LOCALE_AFFECTING_SETTINGS_FIELDS,
    REMOTE_STATUS_AFFECTING_SETTINGS_FIELDS,
    THEME_AFFECTING_SETTINGS_FIELDS,
    settingsPatchTouchesFields,
} from '@entities/settings/settings.constant'

describe('settingsPatchTouchesFields', () => {
    test('빈 patch 는 어떤 필드 그룹도 건드리지 않는다', () => {
        const patch = emptySettingsPatch()
        expect(settingsPatchTouchesFields(patch, THEME_AFFECTING_SETTINGS_FIELDS)).toBe(false)
        expect(settingsPatchTouchesFields(patch, LOCALE_AFFECTING_SETTINGS_FIELDS)).toBe(false)
        expect(settingsPatchTouchesFields(patch, REMOTE_STATUS_AFFECTING_SETTINGS_FIELDS)).toBe(false)
    })

    test('themeId 변경은 THEME 그룹만 건드린다', () => {
        const patch = { ...emptySettingsPatch(), themeId: 'dark' }
        expect(settingsPatchTouchesFields(patch, THEME_AFFECTING_SETTINGS_FIELDS)).toBe(true)
        expect(settingsPatchTouchesFields(patch, LOCALE_AFFECTING_SETTINGS_FIELDS)).toBe(false)
        expect(settingsPatchTouchesFields(patch, REMOTE_STATUS_AFFECTING_SETTINGS_FIELDS)).toBe(false)
    })

    test('followSystemTheme 변경도 THEME 그룹을 건드린다', () => {
        const patch = { ...emptySettingsPatch(), followSystemTheme: true }
        expect(settingsPatchTouchesFields(patch, THEME_AFFECTING_SETTINGS_FIELDS)).toBe(true)
    })

    test('language 변경은 LOCALE 그룹만 건드린다', () => {
        const patch = { ...emptySettingsPatch(), language: 'ko' }
        expect(settingsPatchTouchesFields(patch, THEME_AFFECTING_SETTINGS_FIELDS)).toBe(false)
        expect(settingsPatchTouchesFields(patch, LOCALE_AFFECTING_SETTINGS_FIELDS)).toBe(true)
    })

    test('remoteAccessEnabled 변경은 REMOTE_STATUS 그룹만 건드린다', () => {
        const patch = { ...emptySettingsPatch(), remoteAccessEnabled: true }
        expect(settingsPatchTouchesFields(patch, REMOTE_STATUS_AFFECTING_SETTINGS_FIELDS)).toBe(true)
        expect(settingsPatchTouchesFields(patch, THEME_AFFECTING_SETTINGS_FIELDS)).toBe(false)
        expect(settingsPatchTouchesFields(patch, LOCALE_AFFECTING_SETTINGS_FIELDS)).toBe(false)
    })

    test('무관한 필드(editorFontSize) 변경은 세 그룹 모두 건드리지 않는다 — 과소 무효화 회귀 방지', () => {
        const patch = { ...emptySettingsPatch(), editorFontSize: 16 }
        expect(settingsPatchTouchesFields(patch, THEME_AFFECTING_SETTINGS_FIELDS)).toBe(false)
        expect(settingsPatchTouchesFields(patch, LOCALE_AFFECTING_SETTINGS_FIELDS)).toBe(false)
        expect(settingsPatchTouchesFields(patch, REMOTE_STATUS_AFFECTING_SETTINGS_FIELDS)).toBe(false)
    })
})

import type { FC } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import type { useUpdateSettings } from '@entities/settings/settings.query'
import { fontListQueryOptions } from '@entities/font/font.query'
import { shellProfilesQueryOptions } from '@entities/terminal/terminal.query'
import { FontPicker } from '@features/settings/font-picker'
import { NumericField } from '@features/settings/numeric-field'
import { OptionPicker } from '@features/settings/option-picker'
import { SettingsSection } from '@features/settings/settings-section'
import { ShellProfileList } from '@features/settings/shell-profile-list'
import { TextField } from '@features/settings/text-field'
import { DEFAULT_CODE_FONT_SIZE, MAX_CODE_FONT_SIZE, MIN_CODE_FONT_SIZE } from '@shared/constants/code-font-size'
import type { Settings } from '@shared/api/bindings'
import { Switch } from '@shared/ui/switch'

const MIN_SCROLLBACK = 100
const MAX_SCROLLBACK = 100_000
const DEFAULT_SCROLLBACK = 10_000
const DEFAULT_TERMINAL_CURSOR_STYLE = 'bar'

const TERMINAL_CURSOR_STYLE_OPTIONS = [
    { id: 'bar', labelKey: 'settings.cursorStyleBar' },
    { id: 'block', labelKey: 'settings.cursorStyleBlock' },
    { id: 'underline', labelKey: 'settings.cursorStyleUnderline' },
] as const

type SettingsTerminalSectionProps = {
    id: string
    settings: Settings
    updateSettings: ReturnType<typeof useUpdateSettings>['mutate']
}

export const SettingsTerminalSection: FC<SettingsTerminalSectionProps> = ({ id, settings, updateSettings }) => {
    const { data: fonts = [], isPending: isFontsPending } = useQuery(fontListQueryOptions())
    const { data: shellProfiles = [], isPending: isShellPending } = useQuery(shellProfilesQueryOptions())

    const { t } = useTranslation()

    return (
        <SettingsSection id={id} title={t('settings.terminal')}>
            <NumericField
                label={t('settings.terminalFontSize')}
                value={settings.terminalFontSize ?? DEFAULT_CODE_FONT_SIZE}
                min={MIN_CODE_FONT_SIZE}
                max={MAX_CODE_FONT_SIZE}
                onCommit={(value) => updateSettings({ ...emptySettingsPatch(), terminalFontSize: value })}
            />
            {isFontsPending ? (
                <span className='text-app-sidebar-icon-default text-xs'>{t('settings.loading')}</span>
            ) : (
                <FontPicker
                    label={t('settings.terminalFontFamily')}
                    fonts={fonts}
                    value={settings.terminalFontFamily ?? null}
                    onSelect={(terminalFontFamily) => updateSettings({ ...emptySettingsPatch(), terminalFontFamily: terminalFontFamily ?? '' })}
                />
            )}
            <TextField
                label={t('settings.shell')}
                value={settings.shellOverride ?? ''}
                placeholder='/bin/zsh'
                onCommit={(value) => updateSettings({ ...emptySettingsPatch(), shellOverride: value.trim() })}
            />
            {isShellPending ? (
                <span className='text-app-sidebar-icon-default text-xs'>{t('settings.loading')}</span>
            ) : (
                <ShellProfileList
                    profiles={shellProfiles}
                    activePath={settings.shellOverride ?? null}
                    onSelect={(path) => updateSettings({ ...emptySettingsPatch(), shellOverride: path })}
                />
            )}
            <NumericField
                label={t('settings.terminalScrollback')}
                value={settings.terminalScrollback ?? DEFAULT_SCROLLBACK}
                min={MIN_SCROLLBACK}
                max={MAX_SCROLLBACK}
                onCommit={(value) => updateSettings({ ...emptySettingsPatch(), terminalScrollback: value })}
            />
            <OptionPicker
                label={t('settings.terminalCursorStyle')}
                options={TERMINAL_CURSOR_STYLE_OPTIONS.map((option) => ({ id: option.id, label: t(option.labelKey) }))}
                value={settings.terminalCursorStyle ?? DEFAULT_TERMINAL_CURSOR_STYLE}
                onSelect={(terminalCursorStyle) => updateSettings({ ...emptySettingsPatch(), terminalCursorStyle })}
            />
            <label className='flex items-center justify-between gap-3 text-xs'>
                <span className='text-app-foreground'>{t('settings.terminalCursorBlink')}</span>
                <Switch
                    checked={settings.terminalCursorBlink ?? true}
                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), terminalCursorBlink: checked })}
                />
            </label>
        </SettingsSection>
    )
}

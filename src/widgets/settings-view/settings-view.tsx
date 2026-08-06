import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { lspServersQueryOptions } from '@entities/lsp/lsp.query'
import { settingsQueryOptions, useSetThemeId, useUpdateSettings } from '@entities/settings/settings.query'
import { shellProfilesQueryOptions } from '@entities/terminal/terminal.query'
import { themeListQueryOptions } from '@entities/theme/theme.query'
import { localeListQueryOptions } from '@entities/locale/locale.query'
import { LspServerStatusList } from '@features/settings/lsp-server-status-list'
import { NumericField } from '@features/settings/numeric-field'
import { PluginSectionPlaceholder } from '@features/settings/plugin-section-placeholder'
import { SettingsSection } from '@features/settings/settings-section'
import { ShellProfileList } from '@features/settings/shell-profile-list'
import { TextField } from '@features/settings/text-field'
import { LanguagePicker, SYSTEM_LANGUAGE_ID } from '@features/settings/language-picker'
import { ThemePicker } from '@features/settings/theme-picker'

const MIN_FONT_SIZE = 8
const MAX_FONT_SIZE = 32
const DEFAULT_FONT_SIZE = 13

export const SettingsView = () => {
    const { data: settings, isPending: isSettingsPending } = useQuery(settingsQueryOptions())
    const { data: themes = [], isPending: isThemesPending } = useQuery(themeListQueryOptions())
    const { data: lspServers = [], isPending: isLspPending } = useQuery(lspServersQueryOptions())
    const { data: shellProfiles = [], isPending: isShellPending } = useQuery(shellProfilesQueryOptions())
    const { data: locales = [], isPending: isLocalesPending } = useQuery(localeListQueryOptions())
    const { mutate: setThemeId } = useSetThemeId()
    const { mutate: updateSettings } = useUpdateSettings()

    const { t } = useTranslation()

    if (isSettingsPending || !settings) return <div className='bg-panel-background h-full w-full' />

    return (
        <div className='bg-panel-background text-app-foreground h-full w-full overflow-y-auto'>
            <div className='mx-auto flex max-w-2xl flex-col gap-6 px-6 py-8'>
                <h1 className='text-lg font-semibold'>{t('settings.title')}</h1>

                <SettingsSection title={t('settings.theme')}>
                    {isThemesPending ? (
                        <span className='text-app-sidebar-icon-default text-xs'>{t('settings.loading')}</span>
                    ) : (
                        <ThemePicker themes={themes} activeThemeId={settings.themeId ?? ''} onSelect={setThemeId} />
                    )}
                    <label className='flex items-center gap-2 text-xs'>
                        <input
                            type='checkbox'
                            defaultChecked={settings.followSystemTheme ?? false}
                            onChange={(event) => updateSettings({ ...emptyPatch(), followSystemTheme: event.currentTarget.checked })}
                        />
                        <span>{t('settings.followSystemTheme')}</span>
                    </label>
                </SettingsSection>

                <SettingsSection title={t('settings.language')}>
                    {isLocalesPending ? (
                        <span className='text-app-sidebar-icon-default text-xs'>{t('settings.loading')}</span>
                    ) : (
                        <LanguagePicker
                            locales={locales}
                            activeLanguage={settings.language ?? SYSTEM_LANGUAGE_ID}
                            systemLabel={t('settings.systemLanguage')}
                            onSelect={(language) => updateSettings({ ...emptyPatch(), language })}
                        />
                    )}
                </SettingsSection>

                <SettingsSection title={t('settings.editor')}>
                    <NumericField
                        label={t('settings.editorFontSize')}
                        value={settings.editorFontSize ?? DEFAULT_FONT_SIZE}
                        min={MIN_FONT_SIZE}
                        max={MAX_FONT_SIZE}
                        onCommit={(value) => updateSettings({ ...emptyPatch(), editorFontSize: value })}
                    />
                </SettingsSection>

                <SettingsSection title={t('settings.terminal')}>
                    <NumericField
                        label={t('settings.terminalFontSize')}
                        value={settings.terminalFontSize ?? DEFAULT_FONT_SIZE}
                        min={MIN_FONT_SIZE}
                        max={MAX_FONT_SIZE}
                        onCommit={(value) => updateSettings({ ...emptyPatch(), terminalFontSize: value })}
                    />
                    <TextField
                        label={t('settings.shell')}
                        value={settings.shellOverride ?? ''}
                        placeholder='/bin/zsh'
                        onCommit={(value) => updateSettings({ ...emptyPatch(), shellOverride: value.trim() || null })}
                    />
                    {isShellPending ? (
                        <span className='text-app-sidebar-icon-default text-xs'>{t('settings.loading')}</span>
                    ) : (
                        <ShellProfileList
                            profiles={shellProfiles}
                            activePath={settings.shellOverride ?? null}
                            onSelect={(path) => updateSettings({ ...emptyPatch(), shellOverride: path })}
                        />
                    )}
                </SettingsSection>

                <SettingsSection title={t('settings.lspStatus')} description={t('settings.lspDescription')}>
                    {isLspPending ? (
                        <span className='text-app-sidebar-icon-default text-xs'>{t('settings.loading')}</span>
                    ) : (
                        <LspServerStatusList servers={lspServers} />
                    )}
                </SettingsSection>

                <SettingsSection title={t('settings.plugins')}>
                    <PluginSectionPlaceholder />
                </SettingsSection>
            </div>
        </div>
    )
}

const emptyPatch = () => ({
    themeId: null,
    editorFontSize: null,
    terminalFontSize: null,
    shellOverride: null,
    followSystemTheme: null,
    language: null,
})

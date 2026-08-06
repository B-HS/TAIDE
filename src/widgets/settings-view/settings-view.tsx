import { useQuery } from '@tanstack/react-query'
import { lspServersQueryOptions } from '@entities/lsp/lsp.query'
import { settingsQueryOptions, useSetThemeId, useUpdateSettings } from '@entities/settings/settings.query'
import { shellProfilesQueryOptions } from '@entities/terminal/terminal.query'
import { themeListQueryOptions } from '@entities/theme/theme.query'
import { LspServerStatusList } from '@features/settings/lsp-server-status-list'
import { NumericField } from '@features/settings/numeric-field'
import { PluginSectionPlaceholder } from '@features/settings/plugin-section-placeholder'
import { SettingsSection } from '@features/settings/settings-section'
import { ShellProfileList } from '@features/settings/shell-profile-list'
import { TextField } from '@features/settings/text-field'
import { ThemePicker } from '@features/settings/theme-picker'

const MIN_FONT_SIZE = 8
const MAX_FONT_SIZE = 32
const DEFAULT_FONT_SIZE = 13

export const SettingsView = () => {
    const { data: settings, isPending: isSettingsPending } = useQuery(settingsQueryOptions())
    const { data: themes = [], isPending: isThemesPending } = useQuery(themeListQueryOptions())
    const { data: lspServers = [], isPending: isLspPending } = useQuery(lspServersQueryOptions())
    const { data: shellProfiles = [], isPending: isShellPending } = useQuery(shellProfilesQueryOptions())
    const { mutate: setThemeId } = useSetThemeId()
    const { mutate: updateSettings } = useUpdateSettings()

    if (isSettingsPending || !settings) return <div className='bg-panel-background h-full w-full' />

    return (
        <div className='bg-panel-background text-app-foreground h-full w-full overflow-y-auto'>
            <div className='mx-auto flex max-w-2xl flex-col gap-6 px-6 py-8'>
                <h1 className='text-lg font-semibold'>설정</h1>

                <SettingsSection title='테마'>
                    {isThemesPending ? (
                        <span className='text-app-sidebar-icon-default text-xs'>불러오는 중...</span>
                    ) : (
                        <ThemePicker themes={themes} activeThemeId={settings.themeId ?? ''} onSelect={setThemeId} />
                    )}
                    <label className='flex items-center gap-2 text-xs'>
                        <input
                            type='checkbox'
                            defaultChecked={settings.followSystemTheme ?? false}
                            onChange={(event) => updateSettings({ ...emptyPatch(), followSystemTheme: event.currentTarget.checked })}
                        />
                        <span>시스템 테마를 따라간다</span>
                    </label>
                </SettingsSection>

                <SettingsSection title='에디터'>
                    <NumericField
                        label='폰트 크기'
                        value={settings.editorFontSize ?? DEFAULT_FONT_SIZE}
                        min={MIN_FONT_SIZE}
                        max={MAX_FONT_SIZE}
                        onCommit={(value) => updateSettings({ ...emptyPatch(), editorFontSize: value })}
                    />
                </SettingsSection>

                <SettingsSection title='터미널'>
                    <NumericField
                        label='폰트 크기'
                        value={settings.terminalFontSize ?? DEFAULT_FONT_SIZE}
                        min={MIN_FONT_SIZE}
                        max={MAX_FONT_SIZE}
                        onCommit={(value) => updateSettings({ ...emptyPatch(), terminalFontSize: value })}
                    />
                    <TextField
                        label='셸 오버라이드'
                        value={settings.shellOverride ?? ''}
                        placeholder='/bin/zsh'
                        onCommit={(value) => updateSettings({ ...emptyPatch(), shellOverride: value.trim() || null })}
                    />
                    {isShellPending ? (
                        <span className='text-app-sidebar-icon-default text-xs'>불러오는 중...</span>
                    ) : (
                        <ShellProfileList
                            profiles={shellProfiles}
                            activePath={settings.shellOverride ?? null}
                            onSelect={(path) => updateSettings({ ...emptyPatch(), shellOverride: path })}
                        />
                    )}
                </SettingsSection>

                <SettingsSection title='LSP 서버' description='시스템에 설치된 언어 서버 감지 상태입니다.'>
                    {isLspPending ? (
                        <span className='text-app-sidebar-icon-default text-xs'>불러오는 중...</span>
                    ) : (
                        <LspServerStatusList servers={lspServers} />
                    )}
                </SettingsSection>

                <SettingsSection title='플러그인'>
                    <PluginSectionPlaceholder />
                </SettingsSection>
            </div>
        </div>
    )
}

const emptyPatch = () => ({ themeId: null, editorFontSize: null, terminalFontSize: null, shellOverride: null, followSystemTheme: null })

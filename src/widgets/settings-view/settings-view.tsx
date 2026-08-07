import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import { fontListQueryOptions } from '@entities/font/font.query'
import { lspServersQueryOptions } from '@entities/lsp/lsp.query'
import { projectListQueryOptions } from '@entities/project/project.query'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import { settingsQueryOptions, useSetThemeId, useUpdateSettings } from '@entities/settings/settings.query'
import { shellProfilesQueryOptions } from '@entities/terminal/terminal.query'
import { themeListQueryOptions } from '@entities/theme/theme.query'
import { localeListQueryOptions } from '@entities/locale/locale.query'
import { AgentHooksProjectList } from '@features/settings/agent-hooks-project-list'
import { AgentHooksToggle } from '@features/settings/agent-hooks-toggle'
import { FontPicker } from '@features/settings/font-picker'
import { KeymapList } from '@features/settings/keymap-list'
import { LspServerStatusList } from '@features/settings/lsp-server-status-list'
import { NumericField } from '@features/settings/numeric-field'
import { PluginSectionPlaceholder } from '@features/settings/plugin-section-placeholder'
import { SettingsSection } from '@features/settings/settings-section'
import { ToastPositionPicker } from '@features/settings/toast-position-picker'
import { DEFAULT_RESIZER_THICKNESS, MAX_RESIZER_THICKNESS, MIN_RESIZER_THICKNESS } from '@shared/constants/layout'
import { DEFAULT_TOAST_POSITION } from '@shared/constants/toast'
import type { KeymapActionId, KeymapModifier } from '@shared/lib/keymap'
import { APP_KEYMAP, applyKeymapOverrides, findKeymapConflict, parseKeymapOverrides, serializeKeymapOverrides } from '@shared/lib/keymap'
import { SettingsToc } from '@features/settings/settings-toc'
import { ShellProfileList } from '@features/settings/shell-profile-list'
import { TextField } from '@features/settings/text-field'
import { LanguagePicker, SYSTEM_LANGUAGE_ID } from '@features/settings/language-picker'
import { ThemePicker } from '@features/settings/theme-picker'
import { CustomThemeList } from '@features/theme/custom-theme-list'
import { BUILTIN_THEME_ID } from '@entities/theme/theme-tokens'
import { ThemeEditor } from '@widgets/theme-editor/theme-editor'
import { Switch } from '@shared/ui/switch'
import { Button } from '@shared/ui/button'

const SETTINGS_SCROLL_OFFSET_PX = 32

const MIN_FONT_SIZE = 8
const MAX_FONT_SIZE = 32
const DEFAULT_FONT_SIZE = 13
const MIN_AUTO_SAVE_DELAY_MS = 0
const MAX_AUTO_SAVE_DELAY_MS = 60_000
const DEFAULT_AUTO_SAVE_DELAY_MS = 0

type ThemeEditorState = { mode: 'create' | 'edit'; sourceThemeId: string }

const SETTINGS_SECTION_ID = {
    APPEARANCE: 'settings-section-appearance',
    LANGUAGE: 'settings-section-language',
    INTERFACE: 'settings-section-interface',
    EDITOR: 'settings-section-editor',
    TERMINAL: 'settings-section-terminal',
    KEYMAP: 'settings-section-keymap',
    LSP: 'settings-section-lsp',
    PLUGINS: 'settings-section-plugins',
} as const

const SETTINGS_TOC_ITEMS = [
    { id: SETTINGS_SECTION_ID.APPEARANCE, labelKey: 'settings.appearance' },
    { id: SETTINGS_SECTION_ID.LANGUAGE, labelKey: 'settings.language' },
    { id: SETTINGS_SECTION_ID.INTERFACE, labelKey: 'settings.interface' },
    { id: SETTINGS_SECTION_ID.EDITOR, labelKey: 'settings.editor' },
    { id: SETTINGS_SECTION_ID.TERMINAL, labelKey: 'settings.terminal' },
    { id: SETTINGS_SECTION_ID.KEYMAP, labelKey: 'settings.keymap' },
    { id: SETTINGS_SECTION_ID.LSP, labelKey: 'settings.lspStatus' },
    { id: SETTINGS_SECTION_ID.PLUGINS, labelKey: 'settings.plugins' },
]

export const SettingsView = () => {
    const scrollContainerRef = useRef<HTMLDivElement>(null)

    const [activeSectionId, setActiveSectionId] = useState<string>(SETTINGS_TOC_ITEMS[0].id)
    const [themeEditorState, setThemeEditorState] = useState<ThemeEditorState | null>(null)

    const { data: settings, isPending: isSettingsPending } = useQuery(settingsQueryOptions())
    const { data: themes = [], isPending: isThemesPending } = useQuery(themeListQueryOptions())
    const { data: lspServers = [], isPending: isLspPending } = useQuery(lspServersQueryOptions())
    const { data: shellProfiles = [], isPending: isShellPending } = useQuery(shellProfilesQueryOptions())
    const { data: locales = [], isPending: isLocalesPending } = useQuery(localeListQueryOptions())
    const { data: fonts = [], isPending: isFontsPending } = useQuery(fontListQueryOptions())
    const { data: projects = [] } = useQuery(projectListQueryOptions())
    const { mutate: setThemeId } = useSetThemeId()
    const { mutate: updateSettings } = useUpdateSettings()

    const { t } = useTranslation()

    const handleTocSelect = (id: string) => {
        setActiveSectionId(id)
        const container = scrollContainerRef.current
        const target = container?.querySelector(`#${CSS.escape(id)}`)
        if (!container || !target) return
        const top = target.getBoundingClientRect().top - container.getBoundingClientRect().top + container.scrollTop - SETTINGS_SCROLL_OFFSET_PX
        container.scrollTo({ top, behavior: 'smooth' })
    }

    if (isSettingsPending || !settings) return <div className='bg-panel-background h-full w-full' />

    const keymapOverrides = parseKeymapOverrides(settings.keymapOverrides ?? null)
    const effectiveKeymapEntries = applyKeymapOverrides(APP_KEYMAP, keymapOverrides)

    const handleKeymapChange = (actionId: KeymapActionId, key: string, mods: KeymapModifier[]) => {
        const conflict = findKeymapConflict(effectiveKeymapEntries, { key, mods }, actionId)
        if (conflict) toast.warning(t('settings.keymapConflictWarning', { action: t(conflict.descriptionKey) }))

        const nextOverrides = [...keymapOverrides.filter((override) => override.actionId !== actionId), { actionId, key, mods }]
        updateSettings({ ...emptySettingsPatch(), keymapOverrides: serializeKeymapOverrides(nextOverrides) })
    }

    const handleKeymapResetAll = () => updateSettings({ ...emptySettingsPatch(), keymapOverrides: serializeKeymapOverrides([]) })

    if (themeEditorState)
        return (
            <ThemeEditor
                sourceThemeId={themeEditorState.sourceThemeId}
                mode={themeEditorState.mode}
                existingThemeIds={themes.map((theme) => theme.id)}
                onClose={() => setThemeEditorState(null)}
            />
        )

    return (
        <div ref={scrollContainerRef} className='bg-panel-background text-app-foreground h-full w-full overflow-x-hidden overflow-y-auto'>
            <div className='flex flex-col gap-6 px-8 py-8'>
                <h1 className='text-lg font-semibold'>{t('settings.title')}</h1>

                <div className='flex w-full items-start gap-8'>
                    <div className='sticky top-8 self-start'>
                        <SettingsToc
                            items={SETTINGS_TOC_ITEMS.map((item) => ({ id: item.id, label: t(item.labelKey) }))}
                            activeId={activeSectionId}
                            onSelect={handleTocSelect}
                        />
                    </div>

                    <div className='flex min-w-0 flex-1 flex-col gap-6'>
                        <SettingsSection id={SETTINGS_SECTION_ID.APPEARANCE} title={t('settings.appearance')}>
                            {isThemesPending ? (
                                <span className='text-app-sidebar-icon-default text-xs'>{t('settings.loading')}</span>
                            ) : (
                                <ThemePicker themes={themes} activeThemeId={settings.themeId ?? ''} onSelect={setThemeId} />
                            )}
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='text-app-foreground'>{t('settings.followSystemTheme')}</span>
                                <Switch
                                    checked={settings.followSystemTheme ?? false}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), followSystemTheme: checked })}
                                />
                            </label>

                            <div className='flex flex-col gap-2 pt-2'>
                                <div className='flex items-center justify-between gap-3'>
                                    <span className='text-app-sidebar-icon-default text-xs'>{t('themeEditor.customThemes')}</span>
                                    <Button
                                        variant='outline'
                                        size='xs'
                                        onClick={() =>
                                            setThemeEditorState({
                                                mode: 'create',
                                                sourceThemeId: settings.themeId ?? BUILTIN_THEME_ID.DARK,
                                            })
                                        }>
                                        {t('themeEditor.createNew')}
                                    </Button>
                                </div>
                                {isThemesPending ? (
                                    <span className='text-app-sidebar-icon-default text-xs'>{t('settings.loading')}</span>
                                ) : (
                                    <CustomThemeList
                                        themes={themes.filter((theme) => !theme.builtin)}
                                        onEdit={(sourceThemeId) => setThemeEditorState({ mode: 'edit', sourceThemeId })}
                                        onDuplicate={(sourceThemeId) => setThemeEditorState({ mode: 'create', sourceThemeId })}
                                    />
                                )}
                            </div>
                        </SettingsSection>

                        <SettingsSection id={SETTINGS_SECTION_ID.LANGUAGE} title={t('settings.language')}>
                            {isLocalesPending ? (
                                <span className='text-app-sidebar-icon-default text-xs'>{t('settings.loading')}</span>
                            ) : (
                                <LanguagePicker
                                    locales={locales}
                                    activeLanguage={settings.language ?? SYSTEM_LANGUAGE_ID}
                                    systemLabel={t('settings.systemLanguage')}
                                    onSelect={(language) => updateSettings({ ...emptySettingsPatch(), language })}
                                />
                            )}
                        </SettingsSection>

                        <SettingsSection id={SETTINGS_SECTION_ID.INTERFACE} title={t('settings.interface')}>
                            <div className='flex flex-col gap-2'>
                                <span className='text-app-sidebar-icon-default text-xs'>{t('settings.toastPosition')}</span>
                                <ToastPositionPicker
                                    value={settings.toastPosition ?? DEFAULT_TOAST_POSITION}
                                    translateLabel={t}
                                    onSelect={(toastPosition) => updateSettings({ ...emptySettingsPatch(), toastPosition })}
                                />
                            </div>
                            <NumericField
                                label={t('settings.resizerThickness')}
                                value={settings.resizerThickness ?? DEFAULT_RESIZER_THICKNESS}
                                min={MIN_RESIZER_THICKNESS}
                                max={MAX_RESIZER_THICKNESS}
                                onCommit={(value) => updateSettings({ ...emptySettingsPatch(), resizerThickness: value })}
                            />
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='text-app-foreground'>{t('settings.showSystemUsage')}</span>
                                <Switch
                                    checked={settings.showSystemUsage ?? false}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), showSystemUsage: checked })}
                                />
                            </label>
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='text-app-foreground'>{t('settings.editorMinimap')}</span>
                                <Switch
                                    checked={settings.editorMinimap ?? false}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorMinimap: checked })}
                                />
                            </label>
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='text-app-foreground'>{t('settings.agentStatusBadge')}</span>
                                <Switch
                                    checked={settings.agentStatusBadgeEnabled ?? false}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), agentStatusBadgeEnabled: checked })}
                                />
                            </label>
                            <div className='flex flex-col gap-2'>
                                <AgentHooksToggle
                                    checked={settings.agentHooksEnabled ?? false}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), agentHooksEnabled: checked })}
                                />
                                {(settings.agentHooksEnabled ?? false) && <AgentHooksProjectList projects={projects} />}
                            </div>
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='flex flex-col gap-0.5'>
                                    <span className='text-app-foreground'>{t('settings.ideIntegration')}</span>
                                    <span className='text-app-sidebar-icon-default'>{t('settings.ideIntegrationHint')}</span>
                                </span>
                                <Switch
                                    checked={settings.ideIntegrationEnabled ?? true}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), ideIntegrationEnabled: checked })}
                                />
                            </label>
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='text-app-foreground'>{t('settings.ideAutoOpenDiff')}</span>
                                <Switch
                                    checked={settings.ideAutoOpenDiff ?? false}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), ideAutoOpenDiff: checked })}
                                />
                            </label>
                        </SettingsSection>

                        <SettingsSection id={SETTINGS_SECTION_ID.EDITOR} title={t('settings.editor')}>
                            <NumericField
                                label={t('settings.editorFontSize')}
                                value={settings.editorFontSize ?? DEFAULT_FONT_SIZE}
                                min={MIN_FONT_SIZE}
                                max={MAX_FONT_SIZE}
                                onCommit={(value) => updateSettings({ ...emptySettingsPatch(), editorFontSize: value })}
                            />
                            {isFontsPending ? (
                                <span className='text-app-sidebar-icon-default text-xs'>{t('settings.loading')}</span>
                            ) : (
                                <FontPicker
                                    label={t('settings.editorFontFamily')}
                                    fonts={fonts}
                                    value={settings.editorFontFamily ?? null}
                                    onSelect={(editorFontFamily) => updateSettings({ ...emptySettingsPatch(), editorFontFamily })}
                                />
                            )}
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='text-app-foreground'>{t('settings.formatOnSave')}</span>
                                <Switch
                                    checked={settings.formatOnSave ?? false}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), formatOnSave: checked })}
                                />
                            </label>
                            <div className='flex flex-col gap-1'>
                                <NumericField
                                    label={t('settings.autoSaveDelayMs')}
                                    value={settings.autoSaveDelayMs ?? DEFAULT_AUTO_SAVE_DELAY_MS}
                                    min={MIN_AUTO_SAVE_DELAY_MS}
                                    max={MAX_AUTO_SAVE_DELAY_MS}
                                    onCommit={(value) => updateSettings({ ...emptySettingsPatch(), autoSaveDelayMs: value })}
                                />
                                <span className='text-app-sidebar-icon-default text-xs'>{t('settings.autoSaveDelayHint')}</span>
                            </div>
                        </SettingsSection>

                        <SettingsSection id={SETTINGS_SECTION_ID.TERMINAL} title={t('settings.terminal')}>
                            <NumericField
                                label={t('settings.terminalFontSize')}
                                value={settings.terminalFontSize ?? DEFAULT_FONT_SIZE}
                                min={MIN_FONT_SIZE}
                                max={MAX_FONT_SIZE}
                                onCommit={(value) => updateSettings({ ...emptySettingsPatch(), terminalFontSize: value })}
                            />
                            {isFontsPending ? (
                                <span className='text-app-sidebar-icon-default text-xs'>{t('settings.loading')}</span>
                            ) : (
                                <FontPicker
                                    label={t('settings.terminalFontFamily')}
                                    fonts={fonts}
                                    value={settings.terminalFontFamily ?? null}
                                    onSelect={(terminalFontFamily) => updateSettings({ ...emptySettingsPatch(), terminalFontFamily })}
                                />
                            )}
                            <TextField
                                label={t('settings.shell')}
                                value={settings.shellOverride ?? ''}
                                placeholder='/bin/zsh'
                                onCommit={(value) => updateSettings({ ...emptySettingsPatch(), shellOverride: value.trim() || null })}
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
                        </SettingsSection>

                        <SettingsSection id={SETTINGS_SECTION_ID.KEYMAP} title={t('settings.keymap')} description={t('settings.keymapDescription')}>
                            <div className='flex items-center justify-end'>
                                <Button type='button' variant='outline' size='xs' onClick={handleKeymapResetAll}>
                                    {t('settings.keymapReset')}
                                </Button>
                            </div>
                            <KeymapList
                                entries={effectiveKeymapEntries}
                                overriddenActionIds={keymapOverrides.map((override) => override.actionId)}
                                onChangeBinding={handleKeymapChange}
                            />
                        </SettingsSection>

                        <SettingsSection id={SETTINGS_SECTION_ID.LSP} title={t('settings.lspStatus')} description={t('settings.lspDescription')}>
                            {isLspPending ? (
                                <span className='text-app-sidebar-icon-default text-xs'>{t('settings.loading')}</span>
                            ) : (
                                <LspServerStatusList servers={lspServers} />
                            )}
                        </SettingsSection>

                        <SettingsSection id={SETTINGS_SECTION_ID.PLUGINS} title={t('settings.plugins')}>
                            <PluginSectionPlaceholder />
                        </SettingsSection>

                        <div aria-hidden className='h-[50vh] shrink-0' />
                    </div>
                </div>
            </div>
        </div>
    )
}

import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { FolderOpen } from 'lucide-react'
import { toast } from 'sonner'
import { fontListQueryOptions } from '@entities/font/font.query'
import { lspServersQueryOptions } from '@entities/lsp/lsp.query'
import { projectListQueryOptions } from '@entities/project/project.query'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import { settingsQueryOptions, useSetThemeId, useUpdateSettings } from '@entities/settings/settings.query'
import { systemOpenAppDataPath } from '@entities/system/system.ipc'
import { shellProfilesQueryOptions } from '@entities/terminal/terminal.query'
import { themeListQueryOptions } from '@entities/theme/theme.query'
import { localeListQueryOptions } from '@entities/locale/locale.query'
import { AgentHooksProjectList } from '@features/settings/agent-hooks-project-list'
import { AgentHooksToggle } from '@features/settings/agent-hooks-toggle'
import { FontPicker } from '@features/settings/font-picker'
import { KeymapList } from '@features/settings/keymap-list'
import { LspServerStatusList } from '@features/settings/lsp-server-status-list'
import { NumericField } from '@features/settings/numeric-field'
import { OptionPicker } from '@features/settings/option-picker'
import { PluginList } from '@features/settings/plugin-list'
import { SettingsSection } from '@features/settings/settings-section'
import { ToastPositionPicker } from '@features/settings/toast-position-picker'
import { DEFAULT_RESIZER_THICKNESS, MAX_RESIZER_THICKNESS, MIN_RESIZER_THICKNESS } from '@shared/constants/layout'
import { DEFAULT_TOAST_POSITION } from '@shared/constants/toast'
import type { AppDataPathKind } from '@shared/api/bindings'
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
import { ScrollContainer } from '@shared/scroll/scroll-container'

const SETTINGS_SCROLL_OFFSET_PX = 32

const MIN_FONT_SIZE = 8
const MAX_FONT_SIZE = 32
const DEFAULT_FONT_SIZE = 13
const MIN_AUTO_SAVE_DELAY_MS = 0
const MAX_AUTO_SAVE_DELAY_MS = 60_000
const DEFAULT_AUTO_SAVE_DELAY_MS = 0

const MIN_TAB_SIZE = 1
const MAX_TAB_SIZE = 8
const DEFAULT_TAB_SIZE = 4
const MIN_SCROLLBACK = 100
const MAX_SCROLLBACK = 100_000
const DEFAULT_SCROLLBACK = 10_000
const DEFAULT_CURSOR_STYLE = 'line'
const DEFAULT_CURSOR_BLINKING = 'blink'
const DEFAULT_RENDER_WHITESPACE = 'selection'
const DEFAULT_TERMINAL_CURSOR_STYLE = 'bar'

const EDITOR_CURSOR_STYLE_OPTIONS = [
    { id: 'line', labelKey: 'settings.cursorStyleLine' },
    { id: 'block', labelKey: 'settings.cursorStyleBlock' },
    { id: 'underline', labelKey: 'settings.cursorStyleUnderline' },
] as const

const EDITOR_CURSOR_BLINKING_OPTIONS = [
    { id: 'blink', labelKey: 'settings.cursorBlinkingBlink' },
    { id: 'smooth', labelKey: 'settings.cursorBlinkingSmooth' },
    { id: 'phase', labelKey: 'settings.cursorBlinkingPhase' },
    { id: 'expand', labelKey: 'settings.cursorBlinkingExpand' },
    { id: 'solid', labelKey: 'settings.cursorBlinkingSolid' },
] as const

const EDITOR_RENDER_WHITESPACE_OPTIONS = [
    { id: 'none', labelKey: 'settings.renderWhitespaceNone' },
    { id: 'boundary', labelKey: 'settings.renderWhitespaceBoundary' },
    { id: 'selection', labelKey: 'settings.renderWhitespaceSelection' },
    { id: 'all', labelKey: 'settings.renderWhitespaceAll' },
] as const

const TERMINAL_CURSOR_STYLE_OPTIONS = [
    { id: 'bar', labelKey: 'settings.cursorStyleBar' },
    { id: 'block', labelKey: 'settings.cursorStyleBlock' },
    { id: 'underline', labelKey: 'settings.cursorStyleUnderline' },
] as const

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

    if (isSettingsPending || !settings) return <div className='bg-app-background h-full w-full' />

    const keymapOverrides = parseKeymapOverrides(settings.keymapOverrides ?? null)
    const effectiveKeymapEntries = applyKeymapOverrides(APP_KEYMAP, keymapOverrides)

    const handleKeymapChange = (actionId: KeymapActionId, key: string, mods: KeymapModifier[]) => {
        const conflict = findKeymapConflict(effectiveKeymapEntries, { key, mods }, actionId)
        if (conflict) toast.warning(t('settings.keymapConflictWarning', { action: t(conflict.descriptionKey) }))

        const nextOverrides = [...keymapOverrides.filter((override) => override.actionId !== actionId), { actionId, key, mods }]
        updateSettings({ ...emptySettingsPatch(), keymapOverrides: serializeKeymapOverrides(nextOverrides) })
    }

    const handleKeymapResetAll = () => updateSettings({ ...emptySettingsPatch(), keymapOverrides: serializeKeymapOverrides([]) })

    const handleOpenAppDataFolder = (kind: AppDataPathKind) => void systemOpenAppDataPath(kind).catch((error: Error) => toast.error(error.message))

    if (themeEditorState)
        return (
            <ThemeEditor
                sourceThemeId={themeEditorState.sourceThemeId}
                mode={themeEditorState.mode}
                themes={themes}
                onClose={() => setThemeEditorState(null)}
            />
        )

    return (
        <ScrollContainer viewportRef={scrollContainerRef} className='bg-app-background text-app-foreground h-full w-full'>
            <div className='flex flex-col gap-6 px-4 py-8'>
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
                                <ThemePicker
                                    themes={themes}
                                    activeThemeId={settings.themeId ?? ''}
                                    onSelect={setThemeId}
                                    onDuplicate={(sourceThemeId) => setThemeEditorState({ mode: 'create', sourceThemeId })}
                                />
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
                                    <div className='flex items-center gap-2'>
                                        <Button variant='outline' size='xs' onClick={() => handleOpenAppDataFolder('themes')}>
                                            <FolderOpen className='size-3.5' />
                                            {t('settings.themesOpenFolder')}
                                        </Button>
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
                            <div className='flex justify-end'>
                                <Button variant='outline' size='xs' onClick={() => handleOpenAppDataFolder('locales')}>
                                    <FolderOpen className='size-3.5' />
                                    {t('settings.localesOpenFolder')}
                                </Button>
                            </div>
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
                                    checked={settings.showSystemUsage ?? true}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), showSystemUsage: checked })}
                                />
                            </label>
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='text-app-foreground'>{t('settings.editorMinimap')}</span>
                                <Switch
                                    checked={settings.editorMinimap ?? true}
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
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='flex flex-col gap-0.5'>
                                    <span className='text-app-foreground'>{t('settings.enablePreviewTabs')}</span>
                                    <span className='text-app-sidebar-icon-default'>{t('settings.enablePreviewTabsHint')}</span>
                                </span>
                                <Switch
                                    checked={settings.enablePreviewTabs ?? true}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), enablePreviewTabs: checked })}
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
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='text-app-foreground'>{t('settings.editorWordWrap')}</span>
                                <Switch
                                    checked={settings.editorWordWrap ?? false}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorWordWrap: checked })}
                                />
                            </label>
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='text-app-foreground'>{t('settings.editorLineNumbers')}</span>
                                <Switch
                                    checked={settings.editorLineNumbers ?? true}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorLineNumbers: checked })}
                                />
                            </label>
                            <NumericField
                                label={t('settings.editorTabSize')}
                                value={settings.editorTabSize ?? DEFAULT_TAB_SIZE}
                                min={MIN_TAB_SIZE}
                                max={MAX_TAB_SIZE}
                                onCommit={(value) => updateSettings({ ...emptySettingsPatch(), editorTabSize: value })}
                            />
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='text-app-foreground'>{t('settings.editorInsertSpaces')}</span>
                                <Switch
                                    checked={settings.editorInsertSpaces ?? true}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorInsertSpaces: checked })}
                                />
                            </label>
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='flex flex-col gap-0.5'>
                                    <span className='text-app-foreground'>{t('settings.editorDetectIndentation')}</span>
                                    <span className='text-app-sidebar-icon-default'>{t('settings.editorDetectIndentationHint')}</span>
                                </span>
                                <Switch
                                    checked={settings.editorDetectIndentation ?? true}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorDetectIndentation: checked })}
                                />
                            </label>
                            <OptionPicker
                                label={t('settings.editorRenderWhitespace')}
                                options={EDITOR_RENDER_WHITESPACE_OPTIONS.map((option) => ({ id: option.id, label: t(option.labelKey) }))}
                                value={settings.editorRenderWhitespace ?? DEFAULT_RENDER_WHITESPACE}
                                onSelect={(editorRenderWhitespace) => updateSettings({ ...emptySettingsPatch(), editorRenderWhitespace })}
                            />
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='text-app-foreground'>{t('settings.editorBracketPairColorization')}</span>
                                <Switch
                                    checked={settings.editorBracketPairColorization ?? true}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorBracketPairColorization: checked })}
                                />
                            </label>
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='text-app-foreground'>{t('settings.editorFontLigatures')}</span>
                                <Switch
                                    checked={settings.editorFontLigatures ?? false}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorFontLigatures: checked })}
                                />
                            </label>
                            <OptionPicker
                                label={t('settings.editorCursorStyle')}
                                options={EDITOR_CURSOR_STYLE_OPTIONS.map((option) => ({ id: option.id, label: t(option.labelKey) }))}
                                value={settings.editorCursorStyle ?? DEFAULT_CURSOR_STYLE}
                                onSelect={(editorCursorStyle) => updateSettings({ ...emptySettingsPatch(), editorCursorStyle })}
                            />
                            <OptionPicker
                                label={t('settings.editorCursorBlinking')}
                                options={EDITOR_CURSOR_BLINKING_OPTIONS.map((option) => ({ id: option.id, label: t(option.labelKey) }))}
                                value={settings.editorCursorBlinking ?? DEFAULT_CURSOR_BLINKING}
                                onSelect={(editorCursorBlinking) => updateSettings({ ...emptySettingsPatch(), editorCursorBlinking })}
                            />
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='text-app-foreground'>{t('settings.editorScrollBeyondLastLine')}</span>
                                <Switch
                                    checked={settings.editorScrollBeyondLastLine ?? true}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorScrollBeyondLastLine: checked })}
                                />
                            </label>
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
                            <PluginList />
                        </SettingsSection>

                        <div aria-hidden className='h-[50vh] shrink-0' />
                    </div>
                </div>
            </div>
        </ScrollContainer>
    )
}

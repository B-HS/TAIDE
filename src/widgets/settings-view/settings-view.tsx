import type { FC } from 'react'
import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { FileJson, FolderOpen, Pencil } from 'lucide-react'
import { toast } from 'sonner'
import { aiModelsQueryOptions, aiTokenStatusQueryOptions, useClearAiToken, useSetAiToken } from '@entities/ai/ai.query'
import { fontListQueryOptions } from '@entities/font/font.query'
import { layoutQueryOptions, useOpenTab } from '@entities/layout/layout.query'
import {
    lspInstallProgressQueryOptions,
    lspServersQueryOptions,
    useCancelLspInstall,
    useInstallLspServer,
    useLspInstallProgressSync,
} from '@entities/lsp/lsp.query'
import { projectListQueryOptions } from '@entities/project/project.query'
import {
    remoteStatusQueryOptions,
    useClearRemotePassword,
    useIssueRemoteLink,
    useRevokeRemoteSessions,
    useSetRemotePassword,
} from '@entities/remote/remote.query'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import { settingsQueryOptions, useSetThemeId, useUpdateSettings } from '@entities/settings/settings.query'
import { systemOpenAppDataPath } from '@entities/system/system.ipc'
import { shellProfilesQueryOptions } from '@entities/terminal/terminal.query'
import { syncStatusQueryOptions, useConnectSync, useDisconnectSync, useDownloadSync, useUploadSync } from '@entities/sync/sync.query'
import { themeListQueryOptions } from '@entities/theme/theme.query'
import { localeListQueryOptions } from '@entities/locale/locale.query'
import { AgentCliStatusRow } from '@features/settings/agent-cli-status-row'
import { AgentHooksProjectList } from '@features/settings/agent-hooks-project-list'
import { AgentHooksToggle } from '@features/settings/agent-hooks-toggle'
import { FontPicker } from '@features/settings/font-picker'
import { LspServerStatusList } from '@features/settings/lsp-server-status-list'
import { NumericField } from '@features/settings/numeric-field'
import { OptionPicker } from '@features/settings/option-picker'
import { RemoteSection } from '@features/settings/remote-section'
import { SettingsSection } from '@features/settings/settings-section'
import { ToastPositionPicker } from '@features/settings/toast-position-picker'
import { SETTINGS_JSON_TAB_TITLE } from '@shared/constants/app-file'
import { DEFAULT_RESIZER_THICKNESS, MAX_RESIZER_THICKNESS, MIN_RESIZER_THICKNESS } from '@shared/constants/layout'
import { QUERY_KEY } from '@shared/constants/query-key'
import { DEFAULT_TOAST_POSITION } from '@shared/constants/toast'
import type { AiProviderId, AppDataPathKind, LspServerId, ProjectId, PromptTemplateId } from '@shared/api/bindings'
import { requestOpenKeybindingsEditor } from '@shared/lib/keybindings-bridge'
import { currentWindowFocusedPane } from '@shared/lib/pane-tree'
import { AiAutoTabToggle } from '@features/settings/ai-auto-tab-toggle'
import { AiOmlxRow } from '@features/settings/ai-omlx-row'
import { AiProviderTokenRow } from '@features/settings/ai-provider-token-row'
import { SettingsToc } from '@features/settings/settings-toc'
import { ShellProfileList } from '@features/settings/shell-profile-list'
import { SyncConflictDialog } from '@features/settings/sync-conflict-dialog'
import { SyncSection } from '@features/settings/sync-section'
import { TextField } from '@features/settings/text-field'
import { LanguagePicker, SYSTEM_LANGUAGE_ID } from '@features/settings/language-picker'
import { ThemePicker } from '@features/settings/theme-picker'
import { CustomThemeList } from '@features/theme/custom-theme-list'
import { BUILTIN_THEME_ID } from '@entities/theme/theme-tokens'
import { PluginManager } from '@widgets/plugin-manager/plugin-manager'
import { ThemeEditor } from '@widgets/theme-editor/theme-editor'
import { SnippetEditor } from '@widgets/snippet-editor/snippet-editor'
import { Switch } from '@shared/ui/switch'
import { Button } from '@shared/ui/button'
import { IconButton } from '@shared/ui/icon-button'
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

/** Default for `Settings.aiProvider` — shared by auto-tab, Inline Edit, and AI commit messages (not auto-tab-only, despite the field's Wave G predecessor name). */
const DEFAULT_AI_PROVIDER: AiProviderId = 'ollamaCloud'

const AI_PROVIDER_OPTIONS: { id: AiProviderId; labelKey: string }[] = [
    { id: 'ollamaCloud', labelKey: 'settings.aiProviderOllamaCloud' },
    { id: 'codex', labelKey: 'settings.aiProviderCodex' },
    { id: 'omlx', labelKey: 'settings.aiProviderOmlx' },
]

const PROMPT_ROWS: { id: PromptTemplateId; labelKey: string }[] = [
    { id: 'auto-tab-default', labelKey: 'prompts.autoTabTitle' },
    { id: 'inline-edit-default', labelKey: 'prompts.inlineEditTitle' },
    { id: 'commit-message-default', labelKey: 'prompts.commitMessageTitle' },
]

type ThemeEditorState = { mode: 'create' | 'edit'; sourceThemeId: string }

const SETTINGS_SECTION_ID = {
    APPEARANCE: 'settings-section-appearance',
    LANGUAGE: 'settings-section-language',
    INTERFACE: 'settings-section-interface',
    EDITOR: 'settings-section-editor',
    SNIPPETS: 'settings-section-snippets',
    TERMINAL: 'settings-section-terminal',
    KEYMAP: 'settings-section-keymap',
    LSP: 'settings-section-lsp',
    AI: 'settings-section-ai',
    PLUGINS: 'settings-section-plugins',
    SYNC: 'settings-section-sync',
    REMOTE: 'settings-section-remote',
} as const

const SETTINGS_TOC_ITEMS = [
    { id: SETTINGS_SECTION_ID.APPEARANCE, labelKey: 'settings.appearance' },
    { id: SETTINGS_SECTION_ID.LANGUAGE, labelKey: 'settings.language' },
    { id: SETTINGS_SECTION_ID.INTERFACE, labelKey: 'settings.interface' },
    { id: SETTINGS_SECTION_ID.EDITOR, labelKey: 'settings.editor' },
    { id: SETTINGS_SECTION_ID.SNIPPETS, labelKey: 'settings.snippetsSectionTitle' },
    { id: SETTINGS_SECTION_ID.TERMINAL, labelKey: 'settings.terminal' },
    { id: SETTINGS_SECTION_ID.KEYMAP, labelKey: 'settings.keymap' },
    { id: SETTINGS_SECTION_ID.LSP, labelKey: 'settings.lspStatus' },
    { id: SETTINGS_SECTION_ID.AI, labelKey: 'settings.aiSectionTitle' },
    { id: SETTINGS_SECTION_ID.PLUGINS, labelKey: 'settings.plugins' },
    { id: SETTINGS_SECTION_ID.SYNC, labelKey: 'settings.syncSectionTitle' },
    { id: SETTINGS_SECTION_ID.REMOTE, labelKey: 'remote.title' },
]

type SettingsViewProps = {
    projectId: ProjectId
}

/**
 * `projectId` is the project whose layout owns this tab — not necessarily the app's globally
 * active project. It only ever leaves this window's pane tree once `layoutMoveTabToWindow` moves
 * this "Settings" tab elsewhere (Wave I contract §3.2), which is exactly when the two can diverge:
 * an auxiliary window renders this component for its own fixed project regardless of which
 * project the main window currently has active. New tabs opened from here (`settings.json`, prompt
 * templates) must land in *this* project's layout, so every settings-view-owned mutation below
 * threads `projectId` through instead of re-deriving it from the global active-project query.
 */
export const SettingsView: FC<SettingsViewProps> = ({ projectId }) => {
    const scrollContainerRef = useRef<HTMLDivElement>(null)

    const [activeSectionId, setActiveSectionId] = useState<string>(SETTINGS_TOC_ITEMS[0].id)
    const [themeEditorState, setThemeEditorState] = useState<ThemeEditorState | null>(null)
    const [isSnippetEditorOpen, setIsSnippetEditorOpen] = useState(false)
    const [isSyncConflictOpen, setIsSyncConflictOpen] = useState(false)
    const [issuedRemoteUrl, setIssuedRemoteUrl] = useState<string | null>(null)

    const { data: settings, isPending: isSettingsPending } = useQuery(settingsQueryOptions())
    const { data: themes = [], isPending: isThemesPending } = useQuery(themeListQueryOptions())
    const { data: lspServers = [], isPending: isLspPending } = useQuery(lspServersQueryOptions())
    const { data: lspInstallProgressByServerId = {} } = useQuery(lspInstallProgressQueryOptions())
    const { data: shellProfiles = [], isPending: isShellPending } = useQuery(shellProfilesQueryOptions())
    const { data: locales = [], isPending: isLocalesPending } = useQuery(localeListQueryOptions())
    const { data: fonts = [], isPending: isFontsPending } = useQuery(fontListQueryOptions())
    const { data: projects = [] } = useQuery(projectListQueryOptions())
    const { data: aiTokenStatus } = useQuery(aiTokenStatusQueryOptions())
    const { data: syncStatus } = useQuery(syncStatusQueryOptions())
    const { data: remoteStatus } = useQuery(remoteStatusQueryOptions())
    const { mutate: setThemeId } = useSetThemeId()
    const { mutate: updateSettings, isPending: isUpdatingSettings } = useUpdateSettings()
    const { mutate: installLspServer } = useInstallLspServer()
    const { mutate: cancelLspInstall } = useCancelLspInstall()
    const { mutate: setAiToken, isPending: isSettingAiToken, variables: settingAiTokenVariables } = useSetAiToken()
    const { mutate: clearAiToken } = useClearAiToken()
    const { mutate: connectSync, isPending: isConnectingSync } = useConnectSync()
    const { mutate: disconnectSync, isPending: isDisconnectingSync } = useDisconnectSync()
    const { mutate: uploadSync, isPending: isUploadingSync } = useUploadSync()
    const { mutate: downloadSync, isPending: isDownloadingSync } = useDownloadSync()
    const { mutate: issueRemoteLink, isPending: isIssuingRemoteLink } = useIssueRemoteLink()
    const { mutate: revokeRemoteSessions, isPending: isRevokingRemoteSessions } = useRevokeRemoteSessions()
    const { mutate: setRemotePassword, isPending: isSettingRemotePassword } = useSetRemotePassword()
    const { mutate: clearRemotePassword, isPending: isClearingRemotePassword } = useClearRemotePassword()
    const { mutate: openTab } = useOpenTab(projectId)
    const queryClient = useQueryClient()
    const { data: layout } = useQuery(layoutQueryOptions(projectId))

    const selectedAiProvider = (settings?.aiProvider ?? DEFAULT_AI_PROVIDER) as AiProviderId
    const isSelectedAiProviderConfigured = aiTokenStatus?.[selectedAiProvider] ?? false
    const {
        data: aiModels = [],
        isPending: isAiModelsPending,
        isError: isAiModelsError,
    } = useQuery(aiModelsQueryOptions(isSelectedAiProviderConfigured ? selectedAiProvider : null))

    const { t } = useTranslation()

    useLspInstallProgressSync()

    const handleInstallLspServer = (serverId: LspServerId) => installLspServer(serverId, { onError: (error: Error) => toast.error(error.message) })
    const handleCancelLspInstall = (serverId: LspServerId) => cancelLspInstall(serverId, { onError: (error: Error) => toast.error(error.message) })

    const handleSaveAiToken = (provider: AiProviderId, token: string) =>
        setAiToken({ provider, token }, { onError: () => toast.error(t('settings.aiTokenSaveFailed')) })
    const handleClearAiToken = (provider: AiProviderId) => clearAiToken(provider)
    const handleOmlxBaseUrlCommit = (value: string) => updateSettings({ ...emptySettingsPatch(), aiOmlxBaseUrl: value.trim() })

    const handleOpenSettingsFile = () =>
        openTab(
            {
                projectId,
                kind: { kind: 'appFile', target: { kind: 'settings' } },
                title: SETTINGS_JSON_TAB_TITLE,
                target: currentWindowFocusedPane(layout),
                preview: false,
            },
            { onError: (error) => toast.error(error.message) },
        )

    const handleOpenPromptFile = (id: PromptTemplateId, labelKey: string) =>
        openTab(
            {
                projectId,
                kind: { kind: 'appFile', target: { kind: 'prompt', id } },
                title: t(labelKey),
                target: currentWindowFocusedPane(layout),
                preview: false,
            },
            { onError: (error) => toast.error(error.message) },
        )

    const handleConnectSync = (pat: string) => connectSync(pat, { onError: () => toast.error(t('settings.syncConnectFailed')) })
    const handleDisconnectSync = () =>
        disconnectSync(undefined, {
            onSuccess: () => toast.success(t('settings.syncDisconnected')),
            onError: () => toast.error(t('settings.syncDisconnectFailed')),
        })
    const handleUploadSync = () =>
        uploadSync(undefined, {
            onSuccess: () => toast.success(t('settings.syncUploadSuccess')),
            onError: () => toast.error(t('settings.syncUploadFailed')),
        })
    const handleDownloadSync = () =>
        downloadSync(false, {
            onSuccess: (result) => (result.kind === 'conflict' ? setIsSyncConflictOpen(true) : toast.success(t('settings.syncDownloadSuccess'))),
            onError: () => toast.error(t('settings.syncDownloadFailed')),
        })
    const handleSyncConflictKeepLocal = () => {
        setIsSyncConflictOpen(false)
        handleUploadSync()
    }
    const handleSyncConflictPullRemote = () => {
        setIsSyncConflictOpen(false)
        downloadSync(true, {
            onSuccess: () => toast.success(t('settings.syncDownloadSuccess')),
            onError: () => toast.error(t('settings.syncDownloadFailed')),
        })
    }

    const handleToggleRemote = (enabled: boolean) => {
        setIssuedRemoteUrl(null)
        updateSettings(
            { ...emptySettingsPatch(), remoteAccessEnabled: enabled },
            { onSettled: () => queryClient.invalidateQueries({ queryKey: QUERY_KEY.REMOTE.STATUS }) },
        )
    }
    const handleIssueRemoteLink = () =>
        issueRemoteLink(undefined, {
            onSuccess: (info) => {
                setIssuedRemoteUrl(info.url)
                void navigator.clipboard.writeText(info.url).then(
                    () => toast.success(t('remote.linkCopied')),
                    () => undefined,
                )
            },
            onError: () => toast.error(t('remote.startFailed')),
        })
    const handleRevokeRemoteSessions = () => revokeRemoteSessions(undefined, { onSuccess: () => toast.success(t('remote.sessionsRevoked')) })
    const handleSaveRemotePassword = (password: string) => setRemotePassword(password)
    const handleClearRemotePassword = () => clearRemotePassword()
    const handleTogglePasswordOnlyLogin = (checked: boolean) => updateSettings({ ...emptySettingsPatch(), remotePasswordOnlyLogin: checked })
    const handleChangeRemoteAllowedHosts = (remoteAllowedHosts: string[]) =>
        updateSettings({ ...emptySettingsPatch(), remoteAllowedHosts }, { onError: () => toast.error(t('remote.allowedHostsSaveFailed')) })

    const handleTocSelect = (id: string) => {
        setActiveSectionId(id)
        const container = scrollContainerRef.current
        const target = container?.querySelector(`#${CSS.escape(id)}`)
        if (!container || !target) return
        const top = target.getBoundingClientRect().top - container.getBoundingClientRect().top + container.scrollTop - SETTINGS_SCROLL_OFFSET_PX
        container.scrollTo({ top, behavior: 'smooth' })
    }

    if (isSettingsPending || !settings) return <div className='bg-app-background h-full w-full' />

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

    if (isSnippetEditorOpen) return <SnippetEditor onClose={() => setIsSnippetEditorOpen(false)} />

    return (
        <ScrollContainer viewportRef={scrollContainerRef} className='bg-app-background text-app-foreground h-full w-full'>
            <div className='flex flex-col gap-6 px-4 py-8'>
                <div className='flex items-center justify-between gap-3'>
                    <h1 className='text-lg font-semibold'>{t('settings.title')}</h1>
                    <Button type='button' variant='outline' size='xs' onClick={handleOpenSettingsFile}>
                        <FileJson className='size-3.5' />
                        {t('app.openSettingsFile')}
                    </Button>
                </div>

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
                            <AgentCliStatusRow />
                            <div className='flex flex-col gap-2'>
                                <AgentHooksToggle
                                    label={t('settings.agentHooks')}
                                    hint={t('settings.agentHooksHint')}
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
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='flex flex-col gap-0.5'>
                                    <span className='text-app-foreground'>{t('settings.zenFullscreen')}</span>
                                    <span className='text-app-sidebar-icon-default'>{t('settings.zenFullscreenDescription')}</span>
                                </span>
                                <Switch
                                    checked={settings.zenFullscreen ?? false}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), zenFullscreen: checked })}
                                />
                            </label>
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='flex flex-col gap-0.5'>
                                    <span className='text-app-foreground'>{t('settings.zenHideStatusBar')}</span>
                                    <span className='text-app-sidebar-icon-default'>{t('settings.zenHideStatusBarDescription')}</span>
                                </span>
                                <Switch
                                    checked={settings.zenHideStatusBar ?? true}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), zenHideStatusBar: checked })}
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
                                    onSelect={(editorFontFamily) =>
                                        updateSettings({ ...emptySettingsPatch(), editorFontFamily: editorFontFamily ?? '' })
                                    }
                                />
                            )}
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='text-app-foreground'>{t('settings.formatOnSave')}</span>
                                <Switch
                                    checked={settings.formatOnSave ?? false}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), formatOnSave: checked })}
                                />
                            </label>
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='flex flex-col gap-0.5'>
                                    <span className='text-app-foreground'>{t('settings.organizeImportsOnSave')}</span>
                                    <span className='text-app-sidebar-icon-default'>{t('settings.organizeImportsOnSaveDescription')}</span>
                                </span>
                                <Switch
                                    checked={settings.organizeImportsOnSave ?? false}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), organizeImportsOnSave: checked })}
                                />
                            </label>
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='flex flex-col gap-0.5'>
                                    <span className='text-app-foreground'>{t('settings.fixAllOnSave')}</span>
                                    <span className='text-app-sidebar-icon-default'>{t('settings.fixAllOnSaveDescription')}</span>
                                </span>
                                <Switch
                                    checked={settings.fixAllOnSave ?? false}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), fixAllOnSave: checked })}
                                />
                            </label>
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='flex flex-col gap-0.5'>
                                    <span className='text-app-foreground'>{t('settings.editorCodeLens')}</span>
                                    <span className='text-app-sidebar-icon-default'>{t('settings.editorCodeLensDescription')}</span>
                                </span>
                                <Switch
                                    checked={settings.editorCodeLensEnabled ?? true}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorCodeLensEnabled: checked })}
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
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='flex flex-col gap-0.5'>
                                    <span className='text-app-foreground'>{t('settings.editorStickyScroll')}</span>
                                    <span className='text-app-sidebar-icon-default'>{t('settings.editorStickyScrollDescription')}</span>
                                </span>
                                <Switch
                                    checked={settings.editorStickyScrollEnabled ?? true}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorStickyScrollEnabled: checked })}
                                />
                            </label>
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='flex flex-col gap-0.5'>
                                    <span className='text-app-foreground'>{t('settings.editorSemanticHighlighting')}</span>
                                    <span className='text-app-sidebar-icon-default'>{t('settings.editorSemanticHighlightingDescription')}</span>
                                </span>
                                <Switch
                                    checked={settings.editorSemanticHighlighting ?? true}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorSemanticHighlighting: checked })}
                                />
                            </label>
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='flex flex-col gap-0.5'>
                                    <span className='text-app-foreground'>{t('settings.editorFormatOnType')}</span>
                                    <span className='text-app-sidebar-icon-default'>{t('settings.editorFormatOnTypeDescription')}</span>
                                </span>
                                <Switch
                                    checked={settings.editorFormatOnType ?? false}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorFormatOnType: checked })}
                                />
                            </label>
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='flex flex-col gap-0.5'>
                                    <span className='text-app-foreground'>{t('settings.editorFormatOnPaste')}</span>
                                    <span className='text-app-sidebar-icon-default'>{t('settings.editorFormatOnPasteDescription')}</span>
                                </span>
                                <Switch
                                    checked={settings.editorFormatOnPaste ?? false}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), editorFormatOnPaste: checked })}
                                />
                            </label>
                            <label className='flex items-center justify-between gap-3 text-xs'>
                                <span className='flex flex-col gap-0.5'>
                                    <span className='text-app-foreground'>{t('settings.emmetEnabled')}</span>
                                    <span className='text-app-sidebar-icon-default'>{t('settings.emmetEnabledDescription')}</span>
                                </span>
                                <Switch
                                    checked={settings.emmetEnabled ?? true}
                                    onCheckedChange={(checked) => updateSettings({ ...emptySettingsPatch(), emmetEnabled: checked })}
                                />
                            </label>
                        </SettingsSection>

                        <SettingsSection id={SETTINGS_SECTION_ID.SNIPPETS} title={t('settings.snippetsSectionTitle')}>
                            <div className='flex items-center gap-2'>
                                <Button type='button' variant='outline' size='sm' onClick={() => setIsSnippetEditorOpen(true)}>
                                    {t('settings.snippetsManage')}
                                </Button>
                                <Button type='button' variant='outline' size='xs' onClick={() => handleOpenAppDataFolder('snippets')}>
                                    <FolderOpen className='size-3.5' />
                                    {t('settings.snippetsOpenFolder')}
                                </Button>
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
                                    onSelect={(terminalFontFamily) =>
                                        updateSettings({ ...emptySettingsPatch(), terminalFontFamily: terminalFontFamily ?? '' })
                                    }
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

                        <SettingsSection id={SETTINGS_SECTION_ID.KEYMAP} title={t('settings.keymap')} description={t('settings.keymapDescription')}>
                            <Button type='button' variant='outline' size='sm' onClick={() => requestOpenKeybindingsEditor()}>
                                {t('settings.keymapOpenEditor')}
                            </Button>
                        </SettingsSection>

                        <SettingsSection id={SETTINGS_SECTION_ID.LSP} title={t('settings.lspStatus')} description={t('settings.lspDescription')}>
                            {isLspPending ? (
                                <span className='text-app-sidebar-icon-default text-xs'>{t('settings.loading')}</span>
                            ) : (
                                <LspServerStatusList
                                    servers={lspServers}
                                    installProgressByServerId={lspInstallProgressByServerId}
                                    onInstall={handleInstallLspServer}
                                    onCancelInstall={handleCancelLspInstall}
                                />
                            )}
                        </SettingsSection>

                        <SettingsSection id={SETTINGS_SECTION_ID.AI} title={t('settings.aiSectionTitle')}>
                            <ul className='flex flex-col gap-1.5'>
                                <AiProviderTokenRow
                                    label={t('settings.aiProviderOllamaCloud')}
                                    configured={aiTokenStatus?.ollamaCloud ?? false}
                                    saving={isSettingAiToken && settingAiTokenVariables?.provider === 'ollamaCloud'}
                                    onSave={(token) => handleSaveAiToken('ollamaCloud', token)}
                                    onClear={() => handleClearAiToken('ollamaCloud')}
                                />
                                <AiProviderTokenRow
                                    label={t('settings.aiProviderCodex')}
                                    warning={t('settings.aiCodexUnofficialWarning')}
                                    configured={aiTokenStatus?.codex ?? false}
                                    saving={isSettingAiToken && settingAiTokenVariables?.provider === 'codex'}
                                    onSave={(token) => handleSaveAiToken('codex', token)}
                                    onClear={() => handleClearAiToken('codex')}
                                />
                                <AiOmlxRow
                                    baseUrl={settings.aiOmlxBaseUrl ?? ''}
                                    onBaseUrlCommit={handleOmlxBaseUrlCommit}
                                    apiKeySaving={isSettingAiToken && settingAiTokenVariables?.provider === 'omlx'}
                                    onSaveApiKey={(token) => handleSaveAiToken('omlx', token)}
                                    onClearApiKey={() => handleClearAiToken('omlx')}
                                />
                            </ul>
                            <OptionPicker
                                label={t('settings.aiProviderLabel')}
                                options={AI_PROVIDER_OPTIONS.map((option) => ({ id: option.id, label: t(option.labelKey) }))}
                                value={selectedAiProvider}
                                onSelect={(providerId) => updateSettings({ ...emptySettingsPatch(), aiProvider: providerId, aiModel: '' })}
                            />
                            {isSelectedAiProviderConfigured && isAiModelsError && (
                                <span className='text-status-error text-xs'>
                                    {t(selectedAiProvider === 'omlx' ? 'settings.aiOmlxConnectFailed' : 'settings.aiModelLoadFailed')}
                                </span>
                            )}
                            {isSelectedAiProviderConfigured && !isAiModelsError && !isAiModelsPending && aiModels.length === 0 && (
                                <span className='text-app-sidebar-icon-default text-xs'>{t('settings.aiModelSelectPlaceholder')}</span>
                            )}
                            {isSelectedAiProviderConfigured && !isAiModelsError && aiModels.length > 0 && (
                                <OptionPicker
                                    label={t('settings.aiModelLabel')}
                                    options={aiModels.map((model) => ({ id: model.modelId, label: model.displayName ?? model.modelId }))}
                                    value={settings.aiModel ?? ''}
                                    onSelect={(modelId) =>
                                        updateSettings({ ...emptySettingsPatch(), aiProvider: selectedAiProvider, aiModel: modelId })
                                    }
                                />
                            )}
                            <AiAutoTabToggle
                                checked={settings.aiAutoTabEnabled ?? false}
                                disabled={!isSelectedAiProviderConfigured}
                                onCheckedChange={(checked) =>
                                    updateSettings({ ...emptySettingsPatch(), aiProvider: selectedAiProvider, aiAutoTabEnabled: checked })
                                }
                            />
                            <ul className='border-app-border flex flex-col gap-1.5 border-t pt-3'>
                                {PROMPT_ROWS.map((row) => (
                                    <li key={row.id} className='flex items-center justify-between gap-3 text-xs'>
                                        <span className='text-app-foreground'>{t(row.labelKey)}</span>
                                        <IconButton
                                            label={t('prompts.editEntry')}
                                            icon={<Pencil className='size-3.5' />}
                                            onClick={() => handleOpenPromptFile(row.id, row.labelKey)}
                                        />
                                    </li>
                                ))}
                            </ul>
                        </SettingsSection>

                        <SettingsSection id={SETTINGS_SECTION_ID.PLUGINS} title={t('settings.plugins')}>
                            <PluginManager />
                        </SettingsSection>

                        <SettingsSection id={SETTINGS_SECTION_ID.SYNC} title={t('settings.syncSectionTitle')}>
                            <SyncSection
                                status={syncStatus}
                                gistId={settings.syncGistId ?? null}
                                connecting={isConnectingSync}
                                disconnecting={isDisconnectingSync}
                                uploading={isUploadingSync}
                                downloading={isDownloadingSync}
                                onConnect={handleConnectSync}
                                onDisconnect={handleDisconnectSync}
                                onUpload={handleUploadSync}
                                onDownload={handleDownloadSync}
                            />
                        </SettingsSection>

                        <SettingsSection id={SETTINGS_SECTION_ID.REMOTE} title={t('remote.title')} description={t('remote.description')}>
                            <RemoteSection
                                status={remoteStatus}
                                enabled={settings.remoteAccessEnabled ?? false}
                                issuedUrl={issuedRemoteUrl}
                                issuing={isIssuingRemoteLink}
                                revoking={isRevokingRemoteSessions}
                                passwordSaving={isSettingRemotePassword || isClearingRemotePassword}
                                passwordOnlyLogin={settings.remotePasswordOnlyLogin ?? false}
                                allowedHosts={settings.remoteAllowedHosts ?? []}
                                allowedHostsSaving={isUpdatingSettings}
                                onToggle={handleToggleRemote}
                                onIssueLink={handleIssueRemoteLink}
                                onRevokeSessions={handleRevokeRemoteSessions}
                                onSavePassword={handleSaveRemotePassword}
                                onClearPassword={handleClearRemotePassword}
                                onTogglePasswordOnlyLogin={handleTogglePasswordOnlyLogin}
                                onChangeAllowedHosts={handleChangeRemoteAllowedHosts}
                            />
                        </SettingsSection>

                        <div aria-hidden className='h-[50vh] shrink-0' />
                    </div>
                </div>
            </div>

            <SyncConflictDialog
                open={isSyncConflictOpen}
                onCancel={() => setIsSyncConflictOpen(false)}
                onKeepLocal={handleSyncConflictKeepLocal}
                onPullRemote={handleSyncConflictPullRemote}
            />
        </ScrollContainer>
    )
}

import type { ComponentProps, FC } from 'react'
import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { FileJson } from 'lucide-react'
import { toast } from 'sonner'
import { useLspInstallProgressSync } from '@entities/lsp/lsp.query'
import { useOpenAppFileTab } from '@entities/layout/layout.query'
import { useIssueRemoteLink } from '@entities/remote/remote.query'
import { settingsQueryOptions, useUpdateSettings } from '@entities/settings/settings.query'
import { systemOpenAppDataPath } from '@entities/system/system.ipc'
import { useConnectSync, useDisconnectSync, useDownloadSync, useUploadSync } from '@entities/sync/sync.query'
import { themeListQueryOptions } from '@entities/theme/theme.query'
import { SettingsAiSection } from '@widgets/settings-view/settings-ai-section'
import { SettingsAppearanceSection } from '@widgets/settings-view/settings-appearance-section'
import { SettingsEditorSection } from '@widgets/settings-view/settings-editor-section'
import { SettingsInterfaceSection } from '@widgets/settings-view/settings-interface-section'
import { SettingsKeymapSection } from '@widgets/settings-view/settings-keymap-section'
import { SettingsLanguageSection } from '@widgets/settings-view/settings-language-section'
import { SettingsLspSection } from '@widgets/settings-view/settings-lsp-section'
import { SettingsPluginsSection } from '@widgets/settings-view/settings-plugins-section'
import { SettingsRemoteSection } from '@widgets/settings-view/settings-remote-section'
import { SettingsSnippetsSection } from '@widgets/settings-view/settings-snippets-section'
import { SettingsSyncSection } from '@widgets/settings-view/settings-sync-section'
import { SettingsTerminalSection } from '@widgets/settings-view/settings-terminal-section'
import { SettingsToc } from '@features/settings/settings-toc'
import { SETTINGS_JSON_TAB_TITLE } from '@shared/constants/app-file'
import type { AppDataPathKind, ProjectId } from '@shared/api/bindings'
import { ThemeEditor } from '@widgets/theme-editor/theme-editor'
import { SnippetEditor } from '@widgets/snippet-editor/snippet-editor'
import { Button } from '@shared/ui/button'
import { ScrollContainer } from '@shared/scroll/scroll-container'

export type ThemeEditorState = Pick<ComponentProps<typeof ThemeEditor>, 'mode' | 'sourceThemeId'>

/**
 * Pure — no closure over container state, so it lives at module scope instead of being
 * recreated (and re-threaded through props) on every SettingsView render.
 */
const handleOpenAppDataFolder = (kind: AppDataPathKind) => void systemOpenAppDataPath(kind).catch((error: Error) => toast.error(error.message))

const SETTINGS_SCROLL_OFFSET_PX = 32

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
    const [issuedRemoteUrl, setIssuedRemoteUrl] = useState<string | null>(null)
    const [isSyncConflictOpen, setIsSyncConflictOpen] = useState(false)

    const { data: settings, isPending: isSettingsPending } = useQuery(settingsQueryOptions())
    const { data: themes = [] } = useQuery(themeListQueryOptions())
    const { mutate: updateSettings, isPending: isUpdatingSettings } = useUpdateSettings()
    const { mutate: issueRemoteLink, isPending: isIssuingRemoteLink } = useIssueRemoteLink()
    const { mutate: connectSync, isPending: isConnectingSync } = useConnectSync()
    const { mutate: disconnectSync, isPending: isDisconnectingSync } = useDisconnectSync()
    const { mutate: uploadSync, isPending: isUploadingSync } = useUploadSync()
    const { mutate: downloadSync, isPending: isDownloadingSync } = useDownloadSync()
    const openAppFileTab = useOpenAppFileTab(projectId)

    const { t } = useTranslation()

    useLspInstallProgressSync()

    const handleOpenSettingsFile = () => openAppFileTab({ kind: 'settings' }, SETTINGS_JSON_TAB_TITLE)

    const handleTocSelect = (id: string) => {
        setActiveSectionId(id)
        const container = scrollContainerRef.current
        const target = container?.querySelector(`#${CSS.escape(id)}`)
        if (!container || !target) return
        const top = target.getBoundingClientRect().top - container.getBoundingClientRect().top + container.scrollTop - SETTINGS_SCROLL_OFFSET_PX
        container.scrollTo({ top, behavior: 'smooth' })
    }

    if (isSettingsPending || !settings) return <div className='bg-app-background h-full w-full' />

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
                        <SettingsAppearanceSection
                            id={SETTINGS_SECTION_ID.APPEARANCE}
                            settings={settings}
                            updateSettings={updateSettings}
                            onOpenAppDataFolder={handleOpenAppDataFolder}
                            onOpenThemeEditor={setThemeEditorState}
                        />

                        <SettingsLanguageSection
                            id={SETTINGS_SECTION_ID.LANGUAGE}
                            settings={settings}
                            updateSettings={updateSettings}
                            onOpenAppDataFolder={handleOpenAppDataFolder}
                        />

                        <SettingsInterfaceSection id={SETTINGS_SECTION_ID.INTERFACE} settings={settings} updateSettings={updateSettings} />

                        <SettingsEditorSection id={SETTINGS_SECTION_ID.EDITOR} settings={settings} updateSettings={updateSettings} />

                        <SettingsSnippetsSection
                            id={SETTINGS_SECTION_ID.SNIPPETS}
                            onManage={() => setIsSnippetEditorOpen(true)}
                            onOpenAppDataFolder={handleOpenAppDataFolder}
                        />

                        <SettingsTerminalSection id={SETTINGS_SECTION_ID.TERMINAL} settings={settings} updateSettings={updateSettings} />

                        <SettingsKeymapSection id={SETTINGS_SECTION_ID.KEYMAP} />

                        <SettingsLspSection id={SETTINGS_SECTION_ID.LSP} />

                        <SettingsAiSection id={SETTINGS_SECTION_ID.AI} projectId={projectId} settings={settings} updateSettings={updateSettings} />

                        <SettingsPluginsSection id={SETTINGS_SECTION_ID.PLUGINS} />

                        <SettingsSyncSection
                            id={SETTINGS_SECTION_ID.SYNC}
                            settings={settings}
                            isSyncConflictOpen={isSyncConflictOpen}
                            onSyncConflictOpenChange={setIsSyncConflictOpen}
                            connectSync={connectSync}
                            isConnectingSync={isConnectingSync}
                            disconnectSync={disconnectSync}
                            isDisconnectingSync={isDisconnectingSync}
                            uploadSync={uploadSync}
                            isUploadingSync={isUploadingSync}
                            downloadSync={downloadSync}
                            isDownloadingSync={isDownloadingSync}
                        />

                        <SettingsRemoteSection
                            id={SETTINGS_SECTION_ID.REMOTE}
                            settings={settings}
                            updateSettings={updateSettings}
                            isUpdatingSettings={isUpdatingSettings}
                            issuedUrl={issuedRemoteUrl}
                            onIssuedUrlChange={setIssuedRemoteUrl}
                            issueRemoteLink={issueRemoteLink}
                            isIssuingRemoteLink={isIssuingRemoteLink}
                        />

                        <div aria-hidden className='h-[50vh] shrink-0' />
                    </div>
                </div>
            </div>
        </ScrollContainer>
    )
}

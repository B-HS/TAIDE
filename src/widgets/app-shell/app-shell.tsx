import { useEffect, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { open } from '@tauri-apps/plugin-dialog'
import type { EventCallback } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import type { DragDropEvent } from '@tauri-apps/api/webview'
import { useTranslation } from 'react-i18next'
import { Group, Panel, usePanelRef } from 'react-resizable-panels'
import { toast } from 'sonner'
import { openTab as openTabRaw } from '@entities/layout/layout.ipc'
import { useOpenTab } from '@entities/layout/layout.query'
import { activeProjectQueryOptions, projectListQueryOptions, useActivateProject, useOpenProject } from '@entities/project/project.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { PaneSeparator } from '@features/split/pane-separator'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { DEFAULT_RESIZER_THICKNESS, RESIZE_HIT_TARGET_SIZE } from '@shared/constants/layout'
import { IS_MAC } from '@shared/constants/platform'
import { QUERY_KEY } from '@shared/constants/query-key'
import { subscribeOpenSearchPanel } from '@shared/lib/search-panel-bridge'
import { DragDropOverlay } from '@features/window/drag-drop-overlay'
import { WelcomeScreen } from '@features/welcome/welcome-screen'
import { AppSidebar } from '@widgets/app-sidebar/app-sidebar'
import { EditorArea } from '@widgets/editor-area/editor-area'
import { ExplorerContainer } from '@widgets/explorer/explorer-container'
import { StatusBarContent } from '@widgets/window-chrome/status-bar-content'
import { TitleBarContent } from '@widgets/window-chrome/title-bar-content'

const PATH_SEPARATOR = '/'

const dragDropEventSource = { listen: (handler: EventCallback<DragDropEvent>) => getCurrentWebview().onDragDropEvent(handler) }

export const AppShell = () => {
    const explorerPanelRef = usePanelRef()
    const [isDragActive, setIsDragActive] = useState(false)
    const [isProblemsOpen, setIsProblemsOpen] = useState(false)

    const { t } = useTranslation()
    const queryClient = useQueryClient()
    const { data: projects = [], isPending } = useQuery(projectListQueryOptions())
    const { data: activeProjectId = null } = useQuery(activeProjectQueryOptions())
    const { data: settings } = useQuery(settingsQueryOptions())
    const { mutate: openProject, mutateAsync: openProjectAsync } = useOpenProject()
    const { mutate: activateProject } = useActivateProject()
    const { mutate: openTab } = useOpenTab(activeProjectId)

    const handleOpenProject = async () => {
        const selected = await open({ directory: true, multiple: false })
        if (typeof selected !== 'string') return
        openProject(selected, { onError: (error) => toast.error(error.message) })
    }

    const handleOpenSettings = () => {
        if (!activeProjectId) return toast.info(t('app.openProjectFirst'))
        openTab(
            { projectId: activeProjectId, kind: { kind: 'settings' }, title: t('settings.title'), target: null, preview: false },
            { onError: (error) => toast.error(error.message) },
        )
    }

    const openDroppedFile = async (targetProjectId: string, path: string) => {
        const name = path.slice(path.lastIndexOf(PATH_SEPARATOR) + 1)
        try {
            const layout = await openTabRaw({ projectId: targetProjectId, kind: { kind: 'file', path }, title: name, target: null, preview: true })
            queryClient.setQueryData(QUERY_KEY.LAYOUT.DETAIL(targetProjectId), layout)
        } catch (error) {
            toast.error(error instanceof Error ? error.message : String(error))
        }
    }

    const handleDroppedPaths = async (paths: string[]) => {
        let targetProjectId = activeProjectId

        for (const path of paths) {
            try {
                const result = await openProjectAsync(path)
                targetProjectId = result.project.id
                continue
            } catch {
                if (!targetProjectId) {
                    toast.info(t('app.openProjectFirst'))
                    continue
                }
                await openDroppedFile(targetProjectId, path)
            }
        }
    }

    const handleDragDropEvent: EventCallback<DragDropEvent> = ({ payload }) => {
        if (payload.type === 'leave') {
            setIsDragActive(false)
            return
        }
        if (payload.type === 'drop') {
            setIsDragActive(false)
            void handleDroppedPaths(payload.paths)
            return
        }
        setIsDragActive(true)
    }

    const handleNativeContextMenu = (event: MouseEvent) => {
        const target = event.target as HTMLElement | null
        if (target?.closest('input, textarea, [contenteditable="true"]')) return
        event.preventDefault()
    }

    useTauriEvent(dragDropEventSource, handleDragDropEvent)

    useEffect(() => {
        document.addEventListener('contextmenu', handleNativeContextMenu)
        return () => document.removeEventListener('contextmenu', handleNativeContextMenu)
    }, [])

    useEffect(() => subscribeOpenSearchPanel(() => explorerPanelRef.current?.expand()), [explorerPanelRef])

    if (isPending) return <div className='bg-app-background h-full w-full' />

    return (
        <div className='bg-app-background text-app-foreground relative flex h-full w-full flex-col'>
            <DragDropOverlay visible={isDragActive} label={t('app.dropToOpen')} />
            {IS_MAC && <TitleBarContent />}
            {projects.length === 0 ? (
                <div className='min-h-0 flex-1'>
                    <WelcomeScreen recentProjects={[]} onOpenProject={() => void handleOpenProject()} onSelectRecent={(id) => activateProject(id)} />
                </div>
            ) : (
                <div className='flex min-h-0 flex-1'>
                    <AppSidebar activeProjectId={activeProjectId} onOpenSettings={handleOpenSettings} />
                    <main className='flex min-w-0 flex-1'>
                        {activeProjectId ? (
                            <Group orientation='horizontal' resizeTargetMinimumSize={RESIZE_HIT_TARGET_SIZE} className='min-h-0 min-w-0 flex-1'>
                                <Panel
                                    id='explorer'
                                    panelRef={explorerPanelRef}
                                    defaultSize='240px'
                                    minSize='180px'
                                    maxSize='40%'
                                    collapsible
                                    collapsedSize={0}>
                                    <ExplorerContainer projectId={activeProjectId} />
                                </Panel>
                                <PaneSeparator orientation='horizontal' thickness={settings?.resizerThickness ?? DEFAULT_RESIZER_THICKNESS} />
                                <Panel id='editor' minSize='30%'>
                                    <EditorArea
                                        projectId={activeProjectId}
                                        isProblemsOpen={isProblemsOpen}
                                        onCloseProblems={() => setIsProblemsOpen(false)}
                                    />
                                </Panel>
                            </Group>
                        ) : (
                            <span className='text-app-sidebar-icon-default m-auto'>{t('app.selectProject')}</span>
                        )}
                    </main>
                </div>
            )}
            <StatusBarContent isProblemsOpen={isProblemsOpen} onToggleProblems={() => setIsProblemsOpen((open) => !open)} />
        </div>
    )
}

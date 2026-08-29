import { Suspense, useEffect, useEffectEvent, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { EventCallback } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import type { DragDropEvent } from '@tauri-apps/api/webview'
import { useTranslation } from 'react-i18next'
import { Group, Panel, usePanelRef } from 'react-resizable-panels'
import type { Layout, LayoutChangedMeta } from 'react-resizable-panels'
import { toast } from 'sonner'
import { useOpenTab, useOpenTabInProject, useSetShellView } from '@entities/layout/layout.query'
import { activeProjectQueryOptions, projectListQueryOptions, useOpenProject } from '@entities/project/project.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { PaneSeparator } from '@features/split/pane-separator'
import { useZenMode } from '@widgets/app-shell/use-zen-mode'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { DEFAULT_RESIZER_THICKNESS, RESIZE_HIT_TARGET_SIZE } from '@shared/constants/layout'
import { IS_MAC } from '@shared/constants/platform'
import {
    requestShowExplorerView,
    requestToggleExplorerSidebar,
    subscribeShowExplorerView,
    subscribeToggleExplorerSidebar,
} from '@shared/lib/bridge/explorer-panel-bridge'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { subscribeOpenSearchPanel } from '@shared/lib/bridge/search-panel-bridge'
import { fileNameOf } from '@shared/lib/relative-path'
import { DragDropOverlay } from '@features/window/drag-drop-overlay'
import { ZenModeHint } from '@features/window/zen-mode-hint'
import { ErrorBoundary } from '@shared/ui/error-boundary'
import { AppSidebar } from '@widgets/app-sidebar/app-sidebar'
import { EditorArea } from '@widgets/editor-area/editor-area'
import { ExplorerContainer } from '@widgets/explorer/explorer-container'
import { StatusBarContent } from '@widgets/window-chrome/status-bar-content'
import { TitleBarContent } from '@widgets/window-chrome/title-bar-content'
import { WelcomeContainerLazy } from '@widgets/welcome/welcome-container-lazy'

const dragDropEventSource = { listen: (handler: EventCallback<DragDropEvent>) => getCurrentWebview().onDragDropEvent(handler) }

export const AppShell = () => {
    const explorerPanelRef = usePanelRef()
    const [isDragActive, setIsDragActive] = useState(false)
    const [isProblemsOpen, setIsProblemsOpen] = useState(false)

    const { t } = useTranslation()
    const { data: projects = [], isPending } = useQuery(projectListQueryOptions())
    const { data: activeProjectId = null } = useQuery(activeProjectQueryOptions())
    const { data: settings } = useQuery(settingsQueryOptions())
    const { mutateAsync: openProjectAsync } = useOpenProject()
    const { mutate: openTab } = useOpenTab(activeProjectId)
    const { mutateAsync: openTabInProject } = useOpenTabInProject()
    const { mutate: setShellView } = useSetShellView(activeProjectId)
    const { zen, sidebarCollapsed, hideStatusBar } = useZenMode(activeProjectId)

    const handleOpenSettings = () => {
        if (!activeProjectId) return toast.info(t('app.openProjectFirst'))
        openTab(
            { projectId: activeProjectId, kind: { kind: 'settings' }, title: t('settings.title'), target: null, preview: false },
            { onError: (error) => toast.error(describeIpcError(error)) },
        )
    }

    const openDroppedFile = async (targetProjectId: string, path: string) => {
        await openTabInProject(
            { projectId: targetProjectId, kind: { kind: 'file', path }, title: fileNameOf(path), target: null, preview: true },
            { onError: (error) => toast.error(describeIpcError(error)) },
        ).catch(() => undefined)
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

    /** Promotes the explorer panel's collapsed/expanded state from view-local (pre-Wave-I) to Rust-owned `shell_view.sidebarCollapsed` (ADR-0004) — the panel's *width* stays a view-local default, per contract §3.2. */
    const persistSidebarCollapsed = (collapsed: boolean) => {
        if (!activeProjectId) return
        setShellView({ projectId: activeProjectId, patch: { zen: null, sidebarCollapsed: collapsed } })
    }

    /** Catches a *drag*-driven collapse/expand (dragging the separator past `minSize`) — imperative `.collapse()`/`.expand()` calls (below, and in `subscribeToggleExplorerSidebar`'s handler) report `isUserInteraction: false` and persist explicitly at their own call site instead. */
    const handleShellLayoutChanged = (_layout: Layout, meta: LayoutChangedMeta) => {
        if (!meta.isUserInteraction) return
        persistSidebarCollapsed(explorerPanelRef.current?.isCollapsed() ?? false)
    }

    useGlobalKeymap({
        'toggle-sidebar': () => requestToggleExplorerSidebar(),
        explorer: () => requestShowExplorerView('files'),
        git: () => requestShowExplorerView('git'),
    })

    useTauriEvent(dragDropEventSource, handleDragDropEvent)

    useEffect(() => {
        document.addEventListener('contextmenu', handleNativeContextMenu)
        return () => document.removeEventListener('contextmenu', handleNativeContextMenu)
    }, [])

    useEffect(() => subscribeOpenSearchPanel(() => explorerPanelRef.current?.expand()), [explorerPanelRef])
    useEffect(() => subscribeShowExplorerView(() => explorerPanelRef.current?.expand()), [explorerPanelRef])

    /**
     * A no-op while Zen mode holds the panel force-collapsed — toggling it mid-Zen would desync
     * from the `shouldCollapse` sync effect below (no separator to drag it back with either, since
     * that's also hidden in Zen), so the manual toggle is suppressed until the user leaves Zen
     * mode. `useEffectEvent` (not a dependency array) so the effect subscribes exactly once while
     * still always reading the *latest* `zen`/`activeProjectId` at the moment the bridge fires.
     */
    const handleToggleSidebarRequested = useEffectEvent(() => {
        if (zen) return
        const panel = explorerPanelRef.current
        if (!panel) return
        const collapsed = panel.isCollapsed()
        if (collapsed) panel.expand()
        else panel.collapse()
        persistSidebarCollapsed(!collapsed)
    })
    useEffect(() => subscribeToggleExplorerSidebar(handleToggleSidebarRequested), [])

    /** Applies Zen mode (always collapsed) and the persisted `sidebarCollapsed` preference (otherwise) to the panel's actual imperative state — an external-widget sync, not a derived render value, since `Panel` has no controlled "collapsed" prop. */
    useEffect(() => {
        const panel = explorerPanelRef.current
        if (!panel) return
        const shouldCollapse = zen || sidebarCollapsed
        if (panel.isCollapsed() === shouldCollapse) return
        if (shouldCollapse) panel.collapse()
        else panel.expand()
    }, [explorerPanelRef, zen, sidebarCollapsed])

    if (isPending) return <div className='bg-app-background h-full w-full' />

    return (
        <div className='bg-app-background text-app-foreground relative flex h-full w-full flex-col'>
            <DragDropOverlay visible={isDragActive} label={t('app.dropToOpen')} />
            <ZenModeHint zen={zen} />
            {IS_MAC && (
                <div className='border-tab-bar-tab-border shrink-0 border-b'>
                    <TitleBarContent />
                </div>
            )}
            {projects.length === 0 ? (
                <div className='min-h-0 flex-1'>
                    <ErrorBoundary labelKey='errorBoundary.welcome' labelFallback='Welcome'>
                        <Suspense fallback={<div className='bg-app-background h-full w-full' />}>
                            <WelcomeContainerLazy projectId={null} />
                        </Suspense>
                    </ErrorBoundary>
                </div>
            ) : (
                <div className='flex min-h-0 flex-1'>
                    {!zen && (
                        <ErrorBoundary labelKey='errorBoundary.sidebar' labelFallback='Activity Bar' fallbackSizeClassName='h-full w-14 shrink-0'>
                            <AppSidebar activeProjectId={activeProjectId} onOpenSettings={handleOpenSettings} />
                        </ErrorBoundary>
                    )}
                    <main className='flex min-w-0 flex-1'>
                        {activeProjectId ? (
                            <Group
                                orientation='horizontal'
                                onLayoutChanged={handleShellLayoutChanged}
                                resizeTargetMinimumSize={RESIZE_HIT_TARGET_SIZE}
                                className='min-h-0 min-w-0 flex-1'>
                                <Panel
                                    id='explorer'
                                    panelRef={explorerPanelRef}
                                    defaultSize='240px'
                                    minSize='180px'
                                    maxSize='40%'
                                    collapsible
                                    collapsedSize={0}>
                                    <ErrorBoundary labelKey='errorBoundary.sidebarPanel' labelFallback='Sidebar Panel'>
                                        <ExplorerContainer projectId={activeProjectId} />
                                    </ErrorBoundary>
                                </Panel>
                                {!zen && (
                                    <PaneSeparator orientation='horizontal' thickness={settings?.resizerThickness ?? DEFAULT_RESIZER_THICKNESS} />
                                )}
                                <Panel id='editor' minSize='30%'>
                                    <ErrorBoundary labelKey='errorBoundary.editorArea' labelFallback='Editor'>
                                        <EditorArea
                                            projectId={activeProjectId}
                                            isProblemsOpen={isProblemsOpen}
                                            onCloseProblems={() => setIsProblemsOpen(false)}
                                        />
                                    </ErrorBoundary>
                                </Panel>
                            </Group>
                        ) : (
                            <span className='text-app-sidebar-icon-default m-auto'>{t('app.selectProject')}</span>
                        )}
                    </main>
                </div>
            )}
            {!(zen && hideStatusBar) && (
                <ErrorBoundary labelKey='errorBoundary.statusBar' labelFallback='Status Bar' fallbackSizeClassName='h-6 w-full shrink-0'>
                    <StatusBarContent isProblemsOpen={isProblemsOpen} onToggleProblems={() => setIsProblemsOpen((open) => !open)} />
                </ErrorBoundary>
            )}
        </div>
    )
}

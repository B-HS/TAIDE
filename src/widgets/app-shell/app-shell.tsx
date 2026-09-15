import { Suspense, useEffect, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { EventCallback } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import type { DragDropEvent } from '@tauri-apps/api/webview'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { ShellSlotId } from '@shared/api/bindings'
import { useOpenTab, useOpenTabInProject } from '@entities/layout/layout.query'
import { projectListQueryOptions, useOpenProject } from '@entities/project/project.query'
import { shellStateQueryOptions, useCloseShellSlot, useSetShellSlotSizes } from '@entities/session/session.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { useWindowChrome } from '@widgets/app-shell/use-window-chrome'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { DEFAULT_RESIZER_THICKNESS } from '@shared/constants/layout'
import { IS_MAC } from '@shared/constants/platform'
import { requestShowExplorerView, requestToggleExplorerSidebar } from '@shared/lib/bridge/explorer-panel-bridge'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { PERF_MARK, perfMark } from '@shared/lib/perf-mark'
import { fileNameOf } from '@shared/lib/relative-path'
import { withShellSlotToggled } from '@shared/lib/shell-slot'
import { useShellSlotFocus } from '@shared/lib/shell-slot-context'
import { DragDropOverlay } from '@features/window/drag-drop-overlay'
import { ZenModeHint } from '@features/window/zen-mode-hint'
import { ErrorBoundary } from '@shared/ui/error-boundary'
import { AppSidebar } from '@widgets/app-sidebar/app-sidebar'
import { ShellSlotTreeView } from '@widgets/app-shell/shell-slot-tree-view'
import { StatusBarContent } from '@widgets/window-chrome/status-bar-content'
import { TitleBarContent } from '@widgets/window-chrome/title-bar-content'
import { WelcomeContainerLazy } from '@widgets/welcome/welcome-container-lazy'

const dragDropEventSource = { listen: (handler: EventCallback<DragDropEvent>) => getCurrentWebview().onDragDropEvent(handler) }

/**
 * The main window's chrome: the project rail, the shell-slot tree (one `ProjectShell` per slot,
 * contract §1.B), and the title/status bars that act on whichever slot has focus.
 *
 * The three panel shortcuts stay registered *here*, once, rather than inside each slot: they publish
 * to the per-realm panel bridges, and a registration per slot would publish once per open slot. The
 * subscribing side is what picks the focused slot out (`project-shell.tsx`), which is contract
 * §0.1 U-1's "gate the subscriber, leave the publisher alone" in both directions.
 *
 * Problems-panel visibility is tracked per slot here rather than inside each `ProjectShell` because
 * the status bar's toggle is window-level: it has to both read and flip the *focused* slot's flag,
 * which means one owner above all the slots. Every other slot-local piece of state stays inside the
 * slot. A closed slot's entry is dropped once `shell_slot_close` reports success — slot ids are
 * per-session uuids so nothing would inherit the flag, but the list lives as long as the window and
 * would otherwise only ever grow.
 */
export const AppShell = () => {
    const [isDragActive, setIsDragActive] = useState(false)
    const [problemsOpenSlotIds, setProblemsOpenSlotIds] = useState<readonly ShellSlotId[]>([])

    const { t } = useTranslation()
    const { data: projects = [], isPending: isProjectListPending } = useQuery(projectListQueryOptions())
    const { data: shellState, isPending: isShellStatePending } = useQuery(shellStateQueryOptions())
    const { data: settings } = useQuery(settingsQueryOptions())
    const { focusedShellSlotId, focusedProjectId } = useShellSlotFocus()
    const { mutateAsync: openProjectAsync } = useOpenProject()
    const { mutate: openTab } = useOpenTab(focusedProjectId)
    const { mutateAsync: openTabInProject } = useOpenTabInProject()
    const { mutate: closeShellSlot } = useCloseShellSlot()
    const { mutate: setShellSlotSizes } = useSetShellSlotSizes()
    const { zen, sidebarRailCollapsed, hideStatusBar } = useWindowChrome()

    const tree = shellState?.tree ?? null
    const isProblemsOpenInFocusedSlot = !!focusedShellSlotId && problemsOpenSlotIds.includes(focusedShellSlotId)

    const toggleProblemsInFocusedSlot = () => {
        if (!focusedShellSlotId) return
        setProblemsOpenSlotIds((slotIds) => withShellSlotToggled(slotIds, focusedShellSlotId))
    }

    const forgetSlotProblems = (slotId: ShellSlotId) => setProblemsOpenSlotIds((slotIds) => slotIds.filter((current) => current !== slotId))

    const handleCloseSlot = (slotId: ShellSlotId) =>
        closeShellSlot(slotId, { onSuccess: () => forgetSlotProblems(slotId), onError: (error) => toast.error(describeIpcError(error)) })

    const handleOpenSettings = () => {
        if (!focusedProjectId) return toast.info(t('app.openProjectFirst'))
        openTab(
            { projectId: focusedProjectId, kind: { kind: 'settings' }, title: t('settings.title'), target: null, preview: false },
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
        let targetProjectId = focusedProjectId

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

    /**
     * Opens the project-switch span (metric 2 in `docs/quality-assurance/2026-09-04-perf-baseline.md`),
     * closed by `explorer-container.tsx` when the new project's first tree page lands. The shell is
     * where the switch becomes observable — the sidebar only fires the mutation, and the focused
     * slot's project is what every panel below actually reacts to.
     *
     * A switch whose tree page is already cached is not measured at all: `ExplorerContainer` is a
     * descendant, so its effects run *before* this one in that single commit and find no start mark.
     * That is the honest outcome — there was no round trip to measure — and it keeps a stale mark from
     * being charged to an unrelated later render (`perf-mark.ts`, consume-on-measure).
     */
    useEffect(() => {
        if (!focusedProjectId) return
        perfMark(PERF_MARK.PROJECT_SWITCH_REQUESTED)
    }, [focusedProjectId])

    if (isProjectListPending || isShellStatePending) return <div className='bg-app-background h-full w-full' />

    return (
        <div className='bg-app-background text-app-foreground relative flex h-full w-full flex-col'>
            <DragDropOverlay visible={isDragActive} label={t('app.dropToOpen')} />
            <ZenModeHint zen={zen} />
            {IS_MAC && (
                <div className='border-tab-bar-tab-border shrink-0 border-b'>
                    <TitleBarContent projectId={focusedProjectId} />
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
                    {!zen && !sidebarRailCollapsed && (
                        <ErrorBoundary labelKey='errorBoundary.sidebar' labelFallback='Activity Bar' fallbackSizeClassName='h-full w-14 shrink-0'>
                            <AppSidebar activeProjectId={focusedProjectId} onOpenSettings={handleOpenSettings} />
                        </ErrorBoundary>
                    )}
                    <main className='flex min-w-0 flex-1'>
                        {tree ? (
                            <ShellSlotTreeView
                                tree={tree}
                                projects={projects}
                                focusedShellSlotId={focusedShellSlotId}
                                zen={zen}
                                resizerThickness={settings?.resizerThickness ?? DEFAULT_RESIZER_THICKNESS}
                                problemsOpenSlotIds={problemsOpenSlotIds}
                                onCloseProblems={forgetSlotProblems}
                                onCloseSlot={handleCloseSlot}
                                onCommitSizes={(path, sizes) => setShellSlotSizes({ path, sizes })}
                            />
                        ) : (
                            <span className='text-app-sidebar-icon-default m-auto'>{t('app.selectProject')}</span>
                        )}
                    </main>
                </div>
            )}
            {!(zen && hideStatusBar) && (
                <ErrorBoundary labelKey='errorBoundary.statusBar' labelFallback='Status Bar' fallbackSizeClassName='h-6 w-full shrink-0'>
                    <StatusBarContent
                        projectId={focusedProjectId}
                        isProblemsOpen={isProblemsOpenInFocusedSlot}
                        onToggleProblems={toggleProblemsInFocusedSlot}
                    />
                </ErrorBoundary>
            )}
        </div>
    )
}

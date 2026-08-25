import type { FC, ReactNode, WheelEvent } from 'react'
import { useEffect, useRef } from 'react'
import { useDroppable } from '@dnd-kit/core'
import { SortableContext, horizontalListSortingStrategy } from '@dnd-kit/sortable'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { FileDiff, FileSearch2, Settings, Sparkles, Terminal } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { AgentActivity, DetectedAgent, PaneId, ProjectId, Tab, TabId, TabKind, TabWindowTarget } from '@shared/api/bindings'
import { cn } from '@shared/lib/cn'
import { QUERY_KEY } from '@shared/constants/query-key'
import { FileTypeIcon } from '@shared/icons/file-type-icon'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { collectAllPaneTabs, currentWindowFocusedPane } from '@shared/lib/pane-tree'
import { resolvePreviewKind } from '@shared/lib/preview-kind'
import { fileNameOf, toRelativePath } from '@shared/lib/relative-path'
import { requestOpenFileHistory } from '@shared/lib/bridge/file-history-panel-bridge'
import { requestRevealInExplorer } from '@shared/lib/bridge/explorer-reveal-bridge'
import { getWindowContext } from '@shared/lib/window-context'
import { setOpenWithOverride } from '@entities/editor/open-with-registry'
import { disposeModel, toUntitledModelPath } from '@entities/editor/model-registry'
import { pruneUntitledContents } from '@entities/editor/untitled-registry'
import { clearUntitledMirror } from '@entities/file/file.ipc'
import { projectAgentsQueryOptions } from '@entities/agent/agent.query'
import { projectQueryOptions } from '@entities/project/project.query'
import {
    layoutQueryOptions,
    useActivateTab,
    useCloseTab,
    useFocusPane,
    useMoveTabToWindow,
    useOpenTab,
    useOpenUntitledTab,
    usePinTab,
    useSetTabPreview,
    useSplitPane,
} from '@entities/layout/layout.query'
import { systemRevealPath } from '@entities/system/system.ipc'
import { OverlayScrollbar } from '@shared/scroll/overlay-scrollbar'
import type { SplitEdge } from '@features/tab/tab-context-menu'
import { SortableTab } from '@features/tab/sortable-tab'
import { TabBarAddMenu } from '@features/tab/tab-bar-add-menu'

const TAB_ICON_SIZE_CLASS = 'size-3.5'

export type TabContainerDropData = { type: 'tab-container'; paneId: PaneId }

const ICON_AGENT_ACTIVITY_CLASS: Record<AgentActivity, string> = {
    working: 'text-app-sidebar-icon-agent-working',
    awaitingInput: 'text-app-sidebar-icon-agent-awaiting',
    idle: 'text-app-sidebar-icon-agent-idle',
    unknown: 'text-app-sidebar-icon-agent-unknown',
}

export const getTabIcon = (kind: TabKind, agent?: DetectedAgent): ReactNode => {
    if (kind.kind === 'file') return <FileTypeIcon fileName={fileNameOf(kind.path)} className={TAB_ICON_SIZE_CLASS} />
    if (kind.kind === 'terminal' && agent) return <Sparkles className={cn(TAB_ICON_SIZE_CLASS, ICON_AGENT_ACTIVITY_CLASS[agent.activity])} />
    if (kind.kind === 'terminal') return <Terminal className={TAB_ICON_SIZE_CLASS} />
    if (kind.kind === 'settings') return <Settings className={TAB_ICON_SIZE_CLASS} />
    if (kind.kind === 'appFile') return <Settings className={TAB_ICON_SIZE_CLASS} />
    if (kind.kind === 'diff') return <FileDiff className={TAB_ICON_SIZE_CLASS} />
    if (kind.kind === 'searchEditor') return <FileSearch2 className={TAB_ICON_SIZE_CLASS} />
    if (kind.kind === 'untitled') return <FileTypeIcon fileName='untitled' className={TAB_ICON_SIZE_CLASS} />
    return <Sparkles className={TAB_ICON_SIZE_CLASS} />
}

type PaneTabBarProps = {
    projectId: ProjectId
    paneId: PaneId
    tabs: Tab[]
    activeTabId: TabId | null
    focused: boolean
}

export const PaneTabBar: FC<PaneTabBarProps> = ({ projectId, paneId, tabs, activeTabId, focused }) => {
    const scrollRef = useRef<HTMLDivElement>(null)

    const queryClient = useQueryClient()
    const { data: project } = useQuery(projectQueryOptions(projectId))
    const { data: projectAgents } = useQuery(projectAgentsQueryOptions(projectId))
    const { data: layout } = useQuery(layoutQueryOptions(projectId))
    const { mutate: activateTab } = useActivateTab(projectId)
    const { mutate: closeTab } = useCloseTab(projectId)
    const { mutateAsync: closeTabAsync } = useCloseTab(projectId)
    const { mutate: pinTab } = usePinTab(projectId)
    const { mutate: setTabPreview } = useSetTabPreview(projectId)
    const { mutate: splitPane } = useSplitPane(projectId)
    const { mutate: focusPane } = useFocusPane(projectId)
    const { mutate: openTab } = useOpenTab(projectId)
    const { mutate: openUntitledTab } = useOpenUntitledTab(projectId)
    const { mutate: moveTabToWindow } = useMoveTabToWindow(projectId)
    const { setNodeRef: setContainerRef } = useDroppable({
        id: `pane-container:${paneId}`,
        data: { type: 'tab-container', paneId } satisfies TabContainerDropData,
    })
    const { t } = useTranslation()

    const pinnedTabs = tabs.filter((tab) => tab.pinned)
    const unpinnedTabs = tabs.filter((tab) => !tab.pinned)
    const agentBySessionId = new Map((projectAgents?.agents ?? []).map((agent) => [agent.sessionId, agent] as const))

    const notifyError = (error: Error) => toast.error(describeIpcError(error))

    /**
     * "Move into New Window"/"Move back to Main Window"/"Move to Window N" (contract §3.2) all
     * dispatch through `layout_move_tab_to_window` — the only branching needed here is which
     * targets to *offer*: `windowContext` (this OS window's own identity, not the tab's project)
     * decides whether "back to Main Window" applies, and `layout.auxiliaryWindows` — read off the
     * same `ProjectLayout` this pane bar already queries — lists every other open auxiliary window
     * to offer as an "existing window" destination, excluding this window's own slot.
     */
    const windowContext = getWindowContext()
    const otherAuxiliaryWindowSlots = (layout?.auxiliaryWindows ?? [])
        .map((window) => window.slot)
        .filter((slot) => windowContext.kind !== 'auxiliary' || slot !== windowContext.windowSlot)
    const handleMoveToWindow = (tabId: TabId, target: TabWindowTarget) => moveTabToWindow({ tabId, target }, { onError: notifyError })

    const handleWheel = (event: WheelEvent<HTMLDivElement>) => {
        if (event.deltaY === 0) return
        event.currentTarget.scrollLeft += event.deltaY
    }

    const handleNewUntitledFile = () => openUntitledTab({ projectId, target: paneId }, { onError: notifyError })

    const handleNewTerminal = () =>
        openTab(
            { projectId, kind: { kind: 'terminal', sessionId: '' }, title: t('terminal.title'), target: paneId, preview: false },
            { onError: notifyError },
        )

    const handleCloseOthers = async (keepId: TabId) => {
        for (const tab of tabs) {
            if (tab.id === keepId || tab.pinned) continue
            await closeTabAsync(tab.id)
        }
    }

    const handleCloseToRight = async (fromId: TabId) => {
        const fromIndex = tabs.findIndex((tab) => tab.id === fromId)
        if (fromIndex < 0) return
        for (const tab of tabs.slice(fromIndex + 1)) {
            if (tab.pinned) continue
            await closeTabAsync(tab.id)
        }
    }

    const handleCloseSaved = async () => {
        for (const tab of tabs) {
            if (tab.pinned || tab.dirty) continue
            await closeTabAsync(tab.id)
        }
    }

    const handleCloseAll = async () => {
        for (const tab of tabs) {
            if (tab.pinned) continue
            await closeTabAsync(tab.id)
        }
    }

    const renderTab = (tab: Tab) => {
        const filePath = tab.kind.kind === 'file' ? tab.kind.path : null
        const relativePath = filePath && project ? toRelativePath(project.root, filePath) : null
        const fileName = filePath ? fileNameOf(filePath) : null
        const canReopenWith = fileName ? resolvePreviewKind(fileName) !== null : false
        const agent = tab.kind.kind === 'terminal' && tab.kind.sessionId ? agentBySessionId.get(tab.kind.sessionId) : undefined
        const agentTooltip = agent ? t('agent.sessionTooltip', { name: agent.name, status: t(`agent.status.${agent.activity}`) }) : undefined

        return (
            <SortableTab
                key={tab.id}
                tab={tab}
                paneId={paneId}
                active={tab.id === activeTabId}
                icon={getTabIcon(tab.kind, agent)}
                agentTooltip={agentTooltip}
                onActivate={() => activateTab(tab.id)}
                onClose={() => closeTab(tab.id)}
                onCloseOthers={() => void handleCloseOthers(tab.id)}
                onCloseToRight={() => void handleCloseToRight(tab.id)}
                onCloseSaved={() => void handleCloseSaved()}
                onCloseAll={() => void handleCloseAll()}
                onTogglePin={() => pinTab({ tabId: tab.id, pinned: !tab.pinned })}
                onSplit={(edge: SplitEdge) => splitPane({ paneId, edge, tabId: tab.id })}
                onCopyPath={filePath ? () => void navigator.clipboard.writeText(filePath) : undefined}
                onCopyRelativePath={relativePath ? () => void navigator.clipboard.writeText(relativePath) : undefined}
                onRevealInFinder={filePath ? () => void systemRevealPath(filePath).catch(notifyError) : undefined}
                onOpenChanges={
                    filePath
                        ? () =>
                              openTab(
                                  {
                                      projectId,
                                      kind: { kind: 'diff', path: filePath, staged: false },
                                      title: `${tab.title} (diff)`,
                                      target: currentWindowFocusedPane(layout),
                                      preview: true,
                                  },
                                  { onError: notifyError },
                              )
                        : undefined
                }
                onFileHistory={filePath ? () => requestOpenFileHistory(filePath) : undefined}
                onKeepOpen={() => setTabPreview({ tabId: tab.id, preview: false })}
                onRevealInExplorerView={filePath ? () => requestRevealInExplorer(filePath) : undefined}
                onReopenWithEditor={filePath && canReopenWith ? () => setOpenWithOverride(filePath, 'editor') : undefined}
                onReopenWithPreview={filePath && canReopenWith ? () => setOpenWithOverride(filePath, null) : undefined}
                onMoveToNewWindow={() => handleMoveToWindow(tab.id, { kind: 'newAuxiliary' })}
                onMoveToMainWindow={windowContext.kind === 'auxiliary' ? () => handleMoveToWindow(tab.id, { kind: 'main' }) : undefined}
                moveToWindowSlots={otherAuxiliaryWindowSlots}
                onMoveToWindow={(slot) => handleMoveToWindow(tab.id, { kind: 'existing', slot })}
            />
        )
    }

    /**
     * Keeps every tree in the project (main plus every auxiliary window's own tree — see
     * `collectAllPaneTabs`'s doc comment) in the keep set, not just this pane's own `layout.root` —
     * this effect runs once per rendered pane, in whichever window that pane lives in, so scoping it
     * to a single tree would GC an untitled tab's draft the moment it's open only in a *different*
     * window.
     */
    useEffect(() => {
        if (!layout) return
        const keepTabIds = [...collectAllPaneTabs(layout).map((tab) => tab.id), ...(layout.closedTabs ?? []).map((closed) => closed.tab.id)]
        const removedTabIds = pruneUntitledContents(projectId, keepTabIds)
        for (const removedTabId of removedTabIds) {
            disposeModel(toUntitledModelPath(removedTabId))
            void clearUntitledMirror({ projectId, tabId: removedTabId }).catch(() => undefined)
        }
        if (removedTabIds.length > 0) void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.UNTITLED_MIRRORS(projectId) })
    }, [layout, projectId, queryClient])

    return (
        <div className='relative flex shrink-0 items-stretch'>
            <div
                ref={scrollRef}
                role='tablist'
                onMouseDown={() => focusPane(paneId)}
                onWheel={handleWheel}
                className={cn(
                    'bg-tab-bar-background border-tab-bar-tab-border scrollbar-hidden flex h-9 min-w-0 flex-1 items-stretch overflow-x-auto overflow-y-hidden border-b',
                    focused && 'border-b-ring',
                )}>
                {pinnedTabs.length > 0 && (
                    <SortableContext items={pinnedTabs.map((tab) => tab.id)} strategy={horizontalListSortingStrategy}>
                        <div className='flex shrink-0 items-stretch'>{pinnedTabs.map(renderTab)}</div>
                    </SortableContext>
                )}
                <SortableContext items={unpinnedTabs.map((tab) => tab.id)} strategy={horizontalListSortingStrategy}>
                    <div className='flex min-w-0 shrink-0 items-stretch'>{unpinnedTabs.map(renderTab)}</div>
                </SortableContext>
                <div ref={setContainerRef} onDoubleClick={handleNewUntitledFile} className='min-w-8 flex-1' />
            </div>
            <div
                className={cn(
                    'bg-tab-bar-background border-tab-bar-tab-border flex h-9 shrink-0 items-center border-b px-1',
                    focused && 'border-b-ring',
                )}>
                <TabBarAddMenu onNewFile={handleNewUntitledFile} onNewTerminal={handleNewTerminal} />
            </div>
            <OverlayScrollbar viewportRef={scrollRef} orientation='horizontal' trackClassName='h-[3px]' />
        </div>
    )
}

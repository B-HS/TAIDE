import type { FC, ReactNode, WheelEvent } from 'react'
import { useDroppable } from '@dnd-kit/core'
import { SortableContext, horizontalListSortingStrategy } from '@dnd-kit/sortable'
import { File, FileDiff, Settings, Sparkles, Terminal } from 'lucide-react'
import type { PaneId, ProjectId, Tab, TabId, TabKind } from '@shared/api/bindings'
import { cn } from '@shared/lib/cn'
import { useActivateTab, useCloseTab, useFocusPane, usePinTab, useSplitPane } from '@entities/layout/layout.query'
import type { SplitEdge } from '@widgets/editor-area/tab-context-menu'
import { SortableTab } from '@widgets/editor-area/sortable-tab'

const TAB_ICON_SIZE_CLASS = 'size-3.5'

export type TabContainerDropData = { type: 'tab-container'; paneId: PaneId }

export const getTabIcon = (kind: TabKind): ReactNode => {
    if (kind.kind === 'file') return <File className={TAB_ICON_SIZE_CLASS} />
    if (kind.kind === 'terminal') return <Terminal className={TAB_ICON_SIZE_CLASS} />
    if (kind.kind === 'settings') return <Settings className={TAB_ICON_SIZE_CLASS} />
    if (kind.kind === 'diff') return <FileDiff className={TAB_ICON_SIZE_CLASS} />
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
    const { mutate: activateTab } = useActivateTab(projectId)
    const { mutate: closeTab } = useCloseTab(projectId)
    const { mutateAsync: closeTabAsync } = useCloseTab(projectId)
    const { mutate: pinTab } = usePinTab(projectId)
    const { mutate: splitPane } = useSplitPane(projectId)
    const { mutate: focusPane } = useFocusPane(projectId)
    const { setNodeRef: setContainerRef } = useDroppable({
        id: `pane-container:${paneId}`,
        data: { type: 'tab-container', paneId } satisfies TabContainerDropData,
    })

    const pinnedTabs = tabs.filter((tab) => tab.pinned)
    const unpinnedTabs = tabs.filter((tab) => !tab.pinned)

    const handleWheel = (event: WheelEvent<HTMLDivElement>) => {
        if (event.deltaY === 0) return
        event.currentTarget.scrollLeft += event.deltaY
    }

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

    const renderTab = (tab: Tab) => {
        const filePath = tab.kind.kind === 'file' ? tab.kind.path : null

        return (
            <SortableTab
                key={tab.id}
                tab={tab}
                paneId={paneId}
                active={tab.id === activeTabId}
                icon={getTabIcon(tab.kind)}
                onActivate={() => activateTab(tab.id)}
                onClose={() => closeTab(tab.id)}
                onCloseOthers={() => void handleCloseOthers(tab.id)}
                onCloseToRight={() => void handleCloseToRight(tab.id)}
                onTogglePin={() => pinTab({ tabId: tab.id, pinned: !tab.pinned })}
                onSplit={(edge: SplitEdge) => splitPane({ paneId, edge, tabId: tab.id })}
                onCopyPath={filePath ? () => void navigator.clipboard.writeText(filePath) : undefined}
            />
        )
    }

    return (
        <div
            role='tablist'
            onMouseDown={() => focusPane(paneId)}
            onWheel={handleWheel}
            className={cn(
                'bg-tab-bar-background border-tab-bar-tab-border flex h-9 min-w-0 shrink-0 items-stretch overflow-x-auto overflow-y-hidden border-b',
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
            <div ref={setContainerRef} className='min-w-8 flex-1' />
        </div>
    )
}

import type { FC } from 'react'
import { useState } from 'react'
import { DndContext, DragOverlay, PointerSensor, pointerWithin, useSensor, useSensors } from '@dnd-kit/core'
import type { DragEndEvent, DragOverEvent, DragStartEvent } from '@dnd-kit/core'
import { useQuery } from '@tanstack/react-query'
import type { DropEdge, PaneId, PaneNode, ProjectId, Tab, TabId, TabKind } from '@shared/api/bindings'
import { layoutQueryOptions, useCloseTab, useMoveTab, useSplitPane } from '@entities/layout/layout.query'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'
import { TabItem } from '@features/tab/tab-item'
import type { TabContainerDropData } from '@widgets/editor-area/pane-tab-bar'
import { getTabIcon } from '@widgets/editor-area/pane-tab-bar'
import type { SplitDropData } from '@widgets/editor-area/pane-node-view'
import { PaneNodeView } from '@widgets/editor-area/pane-node-view'
import type { TabDragData } from '@widgets/editor-area/sortable-tab'

const DRAG_ACTIVATION_DISTANCE_PX = 4

type OverDropData = SplitDropData | TabContainerDropData

type DragTabState = {
    id: TabId
    title: string
    kind: TabKind
    pinned: boolean
    preview: boolean
    dirty: boolean
}

type EditorAreaProps = {
    projectId: ProjectId
}

const findLeaf = (node: PaneNode, paneId: PaneId): Extract<PaneNode, { node: 'leaf' }> | null => {
    if (node.node === 'leaf') return node.id === paneId ? node : null
    for (const child of node.children) {
        const found = findLeaf(child, paneId)
        if (found) return found
    }
    return null
}

const findTab = (node: PaneNode, tabId: TabId): Tab | null => {
    if (node.node === 'leaf') return node.tabs.find((tab) => tab.id === tabId) ?? null
    for (const child of node.children) {
        const found = findTab(child, tabId)
        if (found) return found
    }
    return null
}

export const EditorArea: FC<EditorAreaProps> = ({ projectId }) => {
    const [dragTab, setDragTab] = useState<DragTabState | null>(null)
    const [overTarget, setOverTarget] = useState<{ paneId: PaneId; edge: DropEdge } | null>(null)

    const { data: layout } = useQuery(layoutQueryOptions(projectId))
    const { mutate: moveTab } = useMoveTab(projectId)
    const { mutate: splitPane } = useSplitPane(projectId)
    const { mutate: closeTab } = useCloseTab(projectId)

    const closeFocusedTab = () => {
        if (!layout) return
        const leaf = findLeaf(layout.root, layout.focusedPane)
        if (!leaf?.active) return
        closeTab(leaf.active)
    }

    useGlobalKeymap({ 'close-tab': closeFocusedTab })

    const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: DRAG_ACTIVATION_DISTANCE_PX } }))

    const handleDragStart = ({ active }: DragStartEvent) => {
        if (!layout) return
        const tab = findTab(layout.root, active.id as TabId)
        if (!tab) return
        setDragTab({
            id: tab.id,
            title: tab.title,
            kind: tab.kind,
            pinned: tab.pinned ?? false,
            preview: tab.preview ?? false,
            dirty: tab.dirty ?? false,
        })
    }

    const handleDragOver = ({ over }: DragOverEvent) => {
        const data = over?.data.current as OverDropData | TabDragData | undefined
        setOverTarget(data?.type === 'split' ? { paneId: data.paneId, edge: data.edge } : null)
    }

    const handleDragCancel = () => {
        setDragTab(null)
        setOverTarget(null)
    }

    const handleDragEnd = ({ active, over }: DragEndEvent) => {
        setDragTab(null)
        setOverTarget(null)
        if (!over || !layout) return

        const tabId = active.id as TabId
        const activeData = active.data.current as TabDragData | undefined
        const overData = over.data.current as OverDropData | TabDragData | undefined
        if (!activeData || !overData) return

        if (overData.type === 'split') {
            if (overData.edge === 'center') {
                const leaf = findLeaf(layout.root, overData.paneId)
                if (leaf) moveTab({ tabId, paneId: overData.paneId, index: leaf.tabs.length })
                return
            }
            splitPane({ paneId: overData.paneId, edge: overData.edge, tabId })
            return
        }

        if (overData.type === 'tab-container') {
            const leaf = findLeaf(layout.root, overData.paneId)
            if (leaf) moveTab({ tabId, paneId: overData.paneId, index: leaf.tabs.length })
            return
        }

        const leaf = findLeaf(layout.root, overData.paneId)
        if (!leaf) return
        const rawIndex = leaf.tabs.findIndex((tab) => tab.id === over.id)
        if (rawIndex < 0) return
        const pinnedCount = leaf.tabs.filter((tab) => tab.pinned).length
        const index = activeData.pinned ? Math.min(rawIndex, pinnedCount) : Math.max(rawIndex, pinnedCount)
        moveTab({ tabId, paneId: overData.paneId, index })
    }

    if (!layout) return <div className='bg-editor-background h-full w-full' />

    return (
        <DndContext
            sensors={sensors}
            collisionDetection={pointerWithin}
            onDragStart={handleDragStart}
            onDragOver={handleDragOver}
            onDragEnd={handleDragEnd}
            onDragCancel={handleDragCancel}>
            <div className='relative flex h-full min-h-0 w-full min-w-0 flex-1 overflow-hidden'>
                <PaneNodeView
                    node={layout.root}
                    projectId={projectId}
                    focusedPaneId={layout.focusedPane}
                    isDragging={!!dragTab}
                    overTarget={overTarget}
                />
            </div>
            <DragOverlay>
                {dragTab && (
                    <div className='pointer-events-none opacity-90'>
                        <TabItem
                            title={dragTab.title}
                            icon={getTabIcon(dragTab.kind)}
                            active
                            dirty={dragTab.dirty}
                            pinned={dragTab.pinned}
                            preview={dragTab.preview}
                            onActivate={() => {}}
                            onClose={() => {}}
                        />
                    </div>
                )}
            </DragOverlay>
        </DndContext>
    )
}

import type { FC, ReactNode } from 'react'
import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import type { PaneId, Tab } from '@shared/api/bindings'
import { TabItem } from '@features/tab/tab-item'
import type { SplitEdge } from '@widgets/editor-area/tab-context-menu'
import { TabContextMenu } from '@widgets/editor-area/tab-context-menu'

const DRAGGING_OPACITY = 0.4

export type TabDragData = { type: 'tab'; paneId: PaneId; pinned: boolean }

type SortableTabProps = {
    tab: Tab
    paneId: PaneId
    active: boolean
    icon: ReactNode
    onActivate: () => void
    onClose: () => void
    onCloseOthers: () => void
    onCloseToRight: () => void
    onCloseSaved: () => void
    onCloseAll: () => void
    onTogglePin: () => void
    onSplit: (edge: SplitEdge) => void
    onCopyPath?: () => void
    onCopyRelativePath?: () => void
    onRevealInFinder?: () => void
    onOpenChanges?: () => void
}

export const SortableTab: FC<SortableTabProps> = ({
    tab,
    paneId,
    active,
    icon,
    onActivate,
    onClose,
    onCloseOthers,
    onCloseToRight,
    onCloseSaved,
    onCloseAll,
    onTogglePin,
    onSplit,
    onCopyPath,
    onCopyRelativePath,
    onRevealInFinder,
    onOpenChanges,
}) => {
    const dragData: TabDragData = { type: 'tab', paneId, pinned: tab.pinned ?? false }
    const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: tab.id, data: dragData })

    return (
        <div
            ref={setNodeRef}
            style={{ transform: CSS.Translate.toString(transform), transition, opacity: isDragging ? DRAGGING_OPACITY : 1 }}
            {...attributes}
            {...listeners}>
            <TabContextMenu
                tab={tab}
                onClose={onClose}
                onCloseOthers={onCloseOthers}
                onCloseToRight={onCloseToRight}
                onCloseSaved={onCloseSaved}
                onCloseAll={onCloseAll}
                onTogglePin={onTogglePin}
                onSplit={onSplit}
                onCopyPath={onCopyPath}
                onCopyRelativePath={onCopyRelativePath}
                onRevealInFinder={onRevealInFinder}
                onOpenChanges={onOpenChanges}>
                <TabItem
                    title={tab.title}
                    icon={icon}
                    active={active}
                    dirty={tab.dirty ?? false}
                    pinned={tab.pinned ?? false}
                    preview={tab.preview ?? false}
                    onActivate={onActivate}
                    onClose={onClose}
                />
            </TabContextMenu>
        </div>
    )
}

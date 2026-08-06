import type { FC, ReactNode } from 'react'
import { GitCompare, PanelBottom, PanelLeft, PanelRight, PanelTop, Pin, PinOff, X } from 'lucide-react'
import type { DropEdge, Tab } from '@shared/api/bindings'
import {
    ContextMenu,
    ContextMenuContent,
    ContextMenuItem,
    ContextMenuSeparator,
    ContextMenuSub,
    ContextMenuSubContent,
    ContextMenuSubTrigger,
    ContextMenuTrigger,
} from '@shared/ui/context-menu'

export type SplitEdge = Extract<DropEdge, 'left' | 'right' | 'top' | 'bottom'>

const SPLIT_EDGE_OPTIONS: { edge: SplitEdge; label: string; icon: ReactNode }[] = [
    { edge: 'left', label: '왼쪽으로 분할', icon: <PanelLeft className='size-4' /> },
    { edge: 'right', label: '오른쪽으로 분할', icon: <PanelRight className='size-4' /> },
    { edge: 'top', label: '위로 분할', icon: <PanelTop className='size-4' /> },
    { edge: 'bottom', label: '아래로 분할', icon: <PanelBottom className='size-4' /> },
]

type TabContextMenuProps = {
    tab: Tab
    children: ReactNode
    onClose: () => void
    onCloseOthers: () => void
    onCloseToRight: () => void
    onTogglePin: () => void
    onSplit: (edge: SplitEdge) => void
    onCopyPath?: () => void
}

export const TabContextMenu: FC<TabContextMenuProps> = ({
    tab,
    children,
    onClose,
    onCloseOthers,
    onCloseToRight,
    onTogglePin,
    onSplit,
    onCopyPath,
}) => (
    <ContextMenu>
        <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
        <ContextMenuContent>
            <ContextMenuItem onSelect={onClose}>
                <X className='size-4' />
                닫기
            </ContextMenuItem>
            <ContextMenuItem onSelect={onCloseOthers}>다른 탭 모두 닫기</ContextMenuItem>
            <ContextMenuItem onSelect={onCloseToRight}>오른쪽 탭 닫기</ContextMenuItem>
            <ContextMenuSeparator />
            <ContextMenuItem onSelect={onTogglePin}>
                {tab.pinned ? <PinOff className='size-4' /> : <Pin className='size-4' />}
                {tab.pinned ? '고정 해제' : '고정'}
            </ContextMenuItem>
            <ContextMenuSub>
                <ContextMenuSubTrigger>
                    <GitCompare className='size-4' />
                    스플릿으로 이동
                </ContextMenuSubTrigger>
                <ContextMenuSubContent>
                    {SPLIT_EDGE_OPTIONS.map((option) => (
                        <ContextMenuItem key={option.edge} onSelect={() => onSplit(option.edge)}>
                            {option.icon}
                            {option.label}
                        </ContextMenuItem>
                    ))}
                </ContextMenuSubContent>
            </ContextMenuSub>
            {tab.kind.kind === 'file' && onCopyPath && (
                <>
                    <ContextMenuSeparator />
                    <ContextMenuItem onSelect={onCopyPath}>경로 복사</ContextMenuItem>
                </>
            )}
        </ContextMenuContent>
    </ContextMenu>
)

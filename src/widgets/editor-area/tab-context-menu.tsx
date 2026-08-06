import type { FC, ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
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

const SPLIT_EDGE_OPTIONS: { edge: SplitEdge; labelKey: string; icon: ReactNode }[] = [
    { edge: 'left', labelKey: 'editorArea.splitLeft', icon: <PanelLeft className='size-4' /> },
    { edge: 'right', labelKey: 'editorArea.splitRight', icon: <PanelRight className='size-4' /> },
    { edge: 'top', labelKey: 'editorArea.splitTop', icon: <PanelTop className='size-4' /> },
    { edge: 'bottom', labelKey: 'editorArea.splitBottom', icon: <PanelBottom className='size-4' /> },
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
}) => {
    const { t } = useTranslation()

    return (
        <ContextMenu>
            <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
            <ContextMenuContent>
                <ContextMenuItem onSelect={onClose}>
                    <X className='size-4' />
                    {t('tab.close')}
                </ContextMenuItem>
                <ContextMenuItem onSelect={onCloseOthers}>{t('tab.closeAll')}</ContextMenuItem>
                <ContextMenuItem onSelect={onCloseToRight}>{t('tab.closeToRight')}</ContextMenuItem>
                <ContextMenuSeparator />
                <ContextMenuItem onSelect={onTogglePin}>
                    {tab.pinned ? <PinOff className='size-4' /> : <Pin className='size-4' />}
                    {tab.pinned ? t('tab.unpin') : t('tab.pin')}
                </ContextMenuItem>
                <ContextMenuSub>
                    <ContextMenuSubTrigger>
                        <GitCompare className='size-4' />
                        {t('tab.split')}
                    </ContextMenuSubTrigger>
                    <ContextMenuSubContent>
                        {SPLIT_EDGE_OPTIONS.map((option) => (
                            <ContextMenuItem key={option.edge} onSelect={() => onSplit(option.edge)}>
                                {option.icon}
                                {t(option.labelKey)}
                            </ContextMenuItem>
                        ))}
                    </ContextMenuSubContent>
                </ContextMenuSub>
                {tab.kind.kind === 'file' && onCopyPath && (
                    <>
                        <ContextMenuSeparator />
                        <ContextMenuItem onSelect={onCopyPath}>{t('explorer.copyPath')}</ContextMenuItem>
                    </>
                )}
            </ContextMenuContent>
        </ContextMenu>
    )
}

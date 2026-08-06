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
    onCloseSaved: () => void
    onCloseAll: () => void
    onTogglePin: () => void
    onSplit: (edge: SplitEdge) => void
    onCopyPath?: () => void
    onCopyRelativePath?: () => void
    onRevealInFinder?: () => void
    onOpenChanges?: () => void
}

export const TabContextMenu: FC<TabContextMenuProps> = ({
    tab,
    children,
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
    const { t } = useTranslation()
    const isFileTab = tab.kind.kind === 'file'

    return (
        <ContextMenu>
            <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
            <ContextMenuContent>
                <ContextMenuItem onSelect={onClose}>
                    <X className='size-4' />
                    {t('tab.close')}
                </ContextMenuItem>
                <ContextMenuItem onSelect={onCloseOthers}>{t('tab.closeOthers')}</ContextMenuItem>
                <ContextMenuItem onSelect={onCloseToRight}>{t('tab.closeToRight')}</ContextMenuItem>
                <ContextMenuItem onSelect={onCloseSaved}>{t('tab.closeSaved')}</ContextMenuItem>
                <ContextMenuItem onSelect={onCloseAll}>{t('tab.closeAll')}</ContextMenuItem>
                <ContextMenuSeparator />
                <ContextMenuItem onSelect={onTogglePin}>
                    {tab.pinned ? <PinOff className='size-4' /> : <Pin className='size-4' />}
                    {tab.pinned ? t('tab.unpin') : t('tab.pin')}
                </ContextMenuItem>
                {isFileTab && (onCopyPath || onCopyRelativePath) && (
                    <>
                        <ContextMenuSeparator />
                        {onCopyPath && <ContextMenuItem onSelect={onCopyPath}>{t('explorer.copyPath')}</ContextMenuItem>}
                        {onCopyRelativePath && <ContextMenuItem onSelect={onCopyRelativePath}>{t('tab.copyRelativePath')}</ContextMenuItem>}
                    </>
                )}
                {isFileTab && (onRevealInFinder || onOpenChanges) && (
                    <>
                        <ContextMenuSeparator />
                        {onRevealInFinder && <ContextMenuItem onSelect={onRevealInFinder}>{t('explorer.reveal')}</ContextMenuItem>}
                        {onOpenChanges && <ContextMenuItem onSelect={onOpenChanges}>{t('tab.openChanges')}</ContextMenuItem>}
                    </>
                )}
                <ContextMenuSeparator />
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
            </ContextMenuContent>
        </ContextMenu>
    )
}

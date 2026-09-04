import type { FC, ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { ClipboardPaste, Copy, Eraser, GitCompare, SquareDashedMousePointer, SquareTerminal, Trash2 } from 'lucide-react'
import type { SplitEdge } from '@features/tab/tab-context-menu'
import { SPLIT_EDGE_OPTIONS } from '@features/tab/tab-context-menu'
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

type TerminalContextMenuProps = {
    children: ReactNode
    canCopy: boolean
    canPaste: boolean
    /** Per-direction verdict from `resolveSplitAvailability` — a direction that does not fit is shown disabled, never hidden. */
    splitAvailability: Record<SplitEdge, boolean>
    onOpenChange: (open: boolean) => void
    onRestoreFocus: () => void
    onCopy: () => void
    onPaste: () => void
    onSelectAll: () => void
    onClear: () => void
    onSplit: (edge: SplitEdge) => void
    onNewTerminal: () => void
    onKill: () => void
}

/**
 * The terminal's right-click menu (batch 4 contract §F.2-6) — a props-only view, like every other
 * `*-context-menu.tsx`: it owns no query, no IPC and no measurement.
 *
 * Two behaviours are deliberate. Split directions are `disabled` rather than hidden, against the
 * "hide entries that do not apply" rule the tab menu follows (`docs/features/tabs.md` §3.1),
 * because a direction that vanishes reads as "this app cannot split downwards" while a greyed one
 * reads as "not at this size" — which is the truth. And `onCloseAutoFocus` is defaulted so focus
 * returns to the terminal instead of the trigger element: Radix would otherwise leave the pane
 * focused but the xterm textarea blurred, and the next keystroke would go nowhere.
 */
export const TerminalContextMenu: FC<TerminalContextMenuProps> = ({
    children,
    canCopy,
    canPaste,
    splitAvailability,
    onOpenChange,
    onRestoreFocus,
    onCopy,
    onPaste,
    onSelectAll,
    onClear,
    onSplit,
    onNewTerminal,
    onKill,
}) => {
    const { t } = useTranslation()

    const handleCloseAutoFocus = (event: Event) => {
        event.preventDefault()
        onRestoreFocus()
    }

    return (
        <ContextMenu onOpenChange={onOpenChange}>
            <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
            <ContextMenuContent onCloseAutoFocus={handleCloseAutoFocus}>
                <ContextMenuItem disabled={!canCopy} onSelect={onCopy}>
                    <Copy className='size-4' />
                    {t('terminal.copy')}
                </ContextMenuItem>
                <ContextMenuItem disabled={!canPaste} onSelect={onPaste}>
                    <ClipboardPaste className='size-4' />
                    {t('terminal.paste')}
                </ContextMenuItem>
                <ContextMenuItem onSelect={onSelectAll}>
                    <SquareDashedMousePointer className='size-4' />
                    {t('terminal.selectAll')}
                </ContextMenuItem>
                <ContextMenuSeparator />
                <ContextMenuItem onSelect={onClear}>
                    <Eraser className='size-4' />
                    {t('terminal.clear')}
                </ContextMenuItem>
                <ContextMenuSeparator />
                <ContextMenuSub>
                    <ContextMenuSubTrigger>
                        <GitCompare className='size-4' />
                        {t('tab.split')}
                    </ContextMenuSubTrigger>
                    <ContextMenuSubContent>
                        {SPLIT_EDGE_OPTIONS.map((option) => (
                            <ContextMenuItem key={option.edge} disabled={!splitAvailability[option.edge]} onSelect={() => onSplit(option.edge)}>
                                {option.icon}
                                {t(option.labelKey)}
                            </ContextMenuItem>
                        ))}
                    </ContextMenuSubContent>
                </ContextMenuSub>
                <ContextMenuItem onSelect={onNewTerminal}>
                    <SquareTerminal className='size-4' />
                    {t('tab.newTerminal')}
                </ContextMenuItem>
                <ContextMenuSeparator />
                <ContextMenuItem onSelect={onKill}>
                    <Trash2 className='size-4' />
                    {t('terminal.kill')}
                </ContextMenuItem>
            </ContextMenuContent>
        </ContextMenu>
    )
}

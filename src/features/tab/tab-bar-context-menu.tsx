import type { FC, ReactNode } from 'react'
import { Fragment } from 'react'
import { useTranslation } from 'react-i18next'
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
import type { SplitEdge } from '@features/tab/tab-context-menu'
import { SPLIT_EDGE_OPTIONS } from '@features/tab/tab-context-menu'
import type { TabBarMenuActionId } from '@features/tab/tab-bar-menu-items'
import { buildTabBarMenuItems } from '@features/tab/tab-bar-menu-items'

type TabBarContextMenuProps = {
    children: ReactNode
    hasTabs: boolean
    hasActiveTab: boolean
    hasClosedTabs: boolean
    onNewFile: () => void
    onNewTerminal: () => void
    onReopenClosedTab: () => void
    onCloseSaved: () => void
    onCloseAll: () => void
    onOpenWelcome: () => void
    onSplit: (edge: SplitEdge) => void
}

/**
 * The tab bar's *empty space* menu — the tabs themselves keep their own `TabContextMenu`. Radix's
 * trigger only calls `preventDefault()` on `contextmenu` and lets the event keep bubbling, so a
 * trigger wrapped around the whole bar would open both menus at once on a tab; this one is mounted
 * on the filler and on the `+` action area, which are siblings of the tabs rather than ancestors.
 *
 * Which entries appear is decided by `buildTabBarMenuItems`, shared with the `+` dropdown so the
 * two surfaces cannot drift; this component only maps ids to the handlers the pane bar owns.
 */
export const TabBarContextMenu: FC<TabBarContextMenuProps> = ({
    children,
    hasTabs,
    hasActiveTab,
    hasClosedTabs,
    onNewFile,
    onNewTerminal,
    onReopenClosedTab,
    onCloseSaved,
    onCloseAll,
    onOpenWelcome,
    onSplit,
}) => {
    const { t } = useTranslation()
    const items = buildTabBarMenuItems({ surface: 'contextMenu', hasTabs, hasActiveTab, hasClosedTabs })
    const handlerById: Record<TabBarMenuActionId, () => void> = {
        newFile: onNewFile,
        newTerminal: onNewTerminal,
        reopenClosed: onReopenClosedTab,
        closeSaved: onCloseSaved,
        closeAll: onCloseAll,
        openWelcome: onOpenWelcome,
    }

    return (
        <ContextMenu>
            <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
            <ContextMenuContent>
                {items.map((item, index) => {
                    const Icon = item.icon

                    return (
                        <Fragment key={item.id}>
                            {index > 0 && items[index - 1].group !== item.group && <ContextMenuSeparator />}
                            {item.id === 'split' ? (
                                <ContextMenuSub>
                                    <ContextMenuSubTrigger>
                                        {Icon && <Icon className='size-4' />}
                                        {t(item.labelKey)}
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
                            ) : (
                                <ContextMenuItem onSelect={handlerById[item.id]}>
                                    {Icon && <Icon className='size-4' />}
                                    {t(item.labelKey)}
                                </ContextMenuItem>
                            )}
                        </Fragment>
                    )
                })}
            </ContextMenuContent>
        </ContextMenu>
    )
}

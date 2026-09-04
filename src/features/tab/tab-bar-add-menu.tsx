import type { FC } from 'react'
import { Plus } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@shared/ui/dropdown-menu'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'
import { buildTabBarMenuItems } from '@features/tab/tab-bar-menu-items'

const ADD_MENU_TRIGGER_CLASS =
    'text-app-sidebar-icon-default hover:bg-app-sidebar-item-hover hover:text-app-foreground flex size-6 items-center justify-center rounded-sm'

type TabBarAddMenuProps = {
    onNewFile: () => void
    onNewTerminal: () => void
}

/**
 * Renders the `addMenu` slice of `buildTabBarMenuItems` — the same builder the tab bar's
 * empty-space context menu consumes — so the `+` button and the right-click menu always agree on
 * the label, icon and order of the two creation entries.
 */
export const TabBarAddMenu: FC<TabBarAddMenuProps> = ({ onNewFile, onNewTerminal }) => {
    const { t } = useTranslation()

    return (
        <DropdownMenu>
            <Tooltip>
                <TooltipTrigger asChild>
                    <DropdownMenuTrigger aria-label={t('tab.newTabMenu')} className={ADD_MENU_TRIGGER_CLASS}>
                        <Plus className='size-3.5' />
                    </DropdownMenuTrigger>
                </TooltipTrigger>
                <TooltipContent side='bottom'>{t('tab.newTabMenu')}</TooltipContent>
            </Tooltip>
            <DropdownMenuContent align='end'>
                {buildTabBarMenuItems({ surface: 'addMenu' }).map(({ id, labelKey, icon: Icon }) => (
                    <DropdownMenuItem key={id} onSelect={id === 'newTerminal' ? onNewTerminal : onNewFile}>
                        {Icon && <Icon className='size-4' />}
                        {t(labelKey)}
                    </DropdownMenuItem>
                ))}
            </DropdownMenuContent>
        </DropdownMenu>
    )
}

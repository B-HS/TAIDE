import type { FC } from 'react'
import { FilePlus, Plus, Terminal } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@shared/ui/dropdown-menu'

const ADD_MENU_TRIGGER_CLASS =
    'text-app-sidebar-icon-default hover:bg-app-sidebar-item-hover hover:text-app-foreground flex size-6 items-center justify-center rounded-sm'

type TabBarAddMenuProps = {
    onNewFile: () => void
    onNewTerminal: () => void
}

export const TabBarAddMenu: FC<TabBarAddMenuProps> = ({ onNewFile, onNewTerminal }) => {
    const { t } = useTranslation()

    return (
        <DropdownMenu>
            <DropdownMenuTrigger aria-label={t('tab.newTabMenu')} className={ADD_MENU_TRIGGER_CLASS}>
                <Plus className='size-3.5' />
            </DropdownMenuTrigger>
            <DropdownMenuContent align='end'>
                <DropdownMenuItem onSelect={onNewFile}>
                    <FilePlus className='size-4' />
                    {t('tab.newUntitledFile')}
                </DropdownMenuItem>
                <DropdownMenuItem onSelect={onNewTerminal}>
                    <Terminal className='size-4' />
                    {t('tab.newTerminal')}
                </DropdownMenuItem>
            </DropdownMenuContent>
        </DropdownMenu>
    )
}

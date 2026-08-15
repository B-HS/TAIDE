import type { FC } from 'react'
import { History } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@shared/ui/dropdown-menu'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'

const HISTORY_TRIGGER_CLASS =
    'text-app-sidebar-icon-default hover:bg-explorer-item-hover flex size-6 shrink-0 items-center justify-center rounded-sm disabled:pointer-events-none disabled:opacity-40'

type SearchHistoryDropdownProps = {
    history: string[]
    onSelect: (term: string) => void
}

export const SearchHistoryDropdown: FC<SearchHistoryDropdownProps> = ({ history, onSelect }) => {
    const { t } = useTranslation()

    return (
        <DropdownMenu>
            <Tooltip>
                <TooltipTrigger asChild>
                    <DropdownMenuTrigger aria-label={t('search.recentSearches')} disabled={history.length === 0} className={HISTORY_TRIGGER_CLASS}>
                        <History className='size-3.5' />
                    </DropdownMenuTrigger>
                </TooltipTrigger>
                <TooltipContent side='bottom'>{t('search.recentSearches')}</TooltipContent>
            </Tooltip>
            <DropdownMenuContent align='start'>
                {history.map((term) => (
                    <DropdownMenuItem key={term} onSelect={() => onSelect(term)}>
                        {term}
                    </DropdownMenuItem>
                ))}
            </DropdownMenuContent>
        </DropdownMenu>
    )
}

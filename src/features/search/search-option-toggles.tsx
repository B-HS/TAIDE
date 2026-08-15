import type { FC } from 'react'
import { CaseSensitive, EyeOff, Regex, WholeWord } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { cn } from '@shared/lib/cn'
import { IconButton } from '@shared/ui/icon-button'

type SearchOptionTogglesProps = {
    caseSensitive: boolean
    onCaseSensitiveChange: (value: boolean) => void
    wholeWord: boolean
    onWholeWordChange: (value: boolean) => void
    regex: boolean
    onRegexChange: (value: boolean) => void
    respectGitignore: boolean
    onRespectGitignoreChange: (value: boolean) => void
}

const toggleButtonClass = (active: boolean, className?: string) =>
    cn(
        'flex size-6 shrink-0 items-center justify-center rounded-sm',
        active ? 'bg-explorer-item-selected text-app-foreground' : 'text-app-sidebar-icon-default hover:bg-explorer-item-hover',
        className,
    )

export const SearchOptionToggles: FC<SearchOptionTogglesProps> = ({
    caseSensitive,
    onCaseSensitiveChange,
    wholeWord,
    onWholeWordChange,
    regex,
    onRegexChange,
    respectGitignore,
    onRespectGitignoreChange,
}) => {
    const { t } = useTranslation()

    return (
        <>
            <IconButton
                label={t('search.caseSensitive')}
                icon={<CaseSensitive className='size-3.5' />}
                aria-pressed={caseSensitive}
                onClick={() => onCaseSensitiveChange(!caseSensitive)}
                side='bottom'
                className={toggleButtonClass(caseSensitive)}
            />
            <IconButton
                label={t('search.wholeWord')}
                icon={<WholeWord className='size-3.5' />}
                aria-pressed={wholeWord}
                onClick={() => onWholeWordChange(!wholeWord)}
                side='bottom'
                className={toggleButtonClass(wholeWord)}
            />
            <IconButton
                label={t('search.regex')}
                icon={<Regex className='size-3.5' />}
                aria-pressed={regex}
                onClick={() => onRegexChange(!regex)}
                side='bottom'
                className={toggleButtonClass(regex)}
            />
            <IconButton
                label={t('search.respectGitignore')}
                icon={<EyeOff className='size-3.5' />}
                aria-pressed={respectGitignore}
                onClick={() => onRespectGitignoreChange(!respectGitignore)}
                side='bottom'
                className={toggleButtonClass(respectGitignore, 'mr-1')}
            />
        </>
    )
}

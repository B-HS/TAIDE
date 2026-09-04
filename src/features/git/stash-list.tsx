import type { FC } from 'react'
import { Archive, Trash2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { GitStashEntry } from '@shared/api/bindings'
import { cn } from '@shared/lib/cn'
import { GIT_SECTION_ROW_INDENT_CLASS } from '@features/git/git-section-header'
import { IconButton } from '@shared/ui/icon-button'

type StashListProps = {
    stashes: GitStashEntry[]
    disabled: boolean
    onApply: (index: number) => void
    onDrop: (index: number) => void
}

/**
 * Renders only what a caller has already decided to show: the panel drops the whole stash section
 * when there is nothing stashed, so this list never has an empty state to draw. It used to, and the
 * section it lived in was rendered whenever the working tree was dirty — which put an empty stash
 * header above the changes on nearly every repository and is what made the two areas blur together.
 */
export const StashList: FC<StashListProps> = ({ stashes, disabled, onApply, onDrop }) => {
    const { t } = useTranslation()

    return (
        <ul className={cn('flex flex-col', GIT_SECTION_ROW_INDENT_CLASS)}>
            {stashes.map((stash) => (
                <li key={stash.index} className='hover:bg-explorer-item-hover flex items-center gap-1.5 px-2 py-1 text-xs'>
                    <Archive className='size-3.5 shrink-0' />
                    <span className='min-w-0 flex-1 truncate'>{stash.message}</span>
                    <button
                        type='button'
                        disabled={disabled}
                        aria-label={t('git.stashApply')}
                        onClick={() => onApply(stash.index)}
                        className='hover:bg-app-sidebar-item-active shrink-0 rounded-sm px-1.5 py-0.5 disabled:opacity-50'>
                        {t('git.stashApply')}
                    </button>
                    <IconButton
                        label={t('git.stashDrop')}
                        icon={<Trash2 className='size-3.5' />}
                        disabled={disabled}
                        onClick={() => onDrop(stash.index)}
                        side='bottom'
                        className='hover:bg-app-sidebar-item-active text-status-error shrink-0 rounded-sm p-1 disabled:opacity-50'
                    />
                </li>
            ))}
        </ul>
    )
}

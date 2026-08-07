import type { FC } from 'react'
import { Archive, Trash2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { GitStashEntry } from '@shared/api/bindings'

type StashListProps = {
    stashes: GitStashEntry[]
    disabled: boolean
    onApply: (index: number) => void
    onDrop: (index: number) => void
}

export const StashList: FC<StashListProps> = ({ stashes, disabled, onApply, onDrop }) => {
    const { t } = useTranslation()

    if (stashes.length === 0) return <span className='text-app-sidebar-icon-default px-2 py-1 text-xs'>{t('git.stashEmpty')}</span>

    return (
        <ul className='flex flex-col'>
            {stashes.map((stash) => (
                <li key={stash.index} className='hover:bg-explorer-item-hover flex items-center gap-1.5 px-2 py-1 text-xs'>
                    <Archive className='size-3.5 shrink-0' />
                    <span className='min-w-0 flex-1 truncate'>{stash.message}</span>
                    <button
                        type='button'
                        disabled={disabled}
                        onClick={() => onApply(stash.index)}
                        className='hover:bg-app-sidebar-item-active shrink-0 rounded-sm px-1.5 py-0.5 disabled:opacity-50'>
                        {t('git.stashApply')}
                    </button>
                    <button
                        type='button'
                        disabled={disabled}
                        aria-label={t('git.stashDrop')}
                        onClick={() => onDrop(stash.index)}
                        className='hover:bg-app-sidebar-item-active text-status-error shrink-0 rounded-sm p-1 disabled:opacity-50'>
                        <Trash2 className='size-3.5' />
                    </button>
                </li>
            ))}
        </ul>
    )
}

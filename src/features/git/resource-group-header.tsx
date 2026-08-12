import type { FC, ReactNode } from 'react'
import { IconButton } from '@shared/ui/icon-button'

type ResourceGroupHeaderProps = {
    title: string
    count: number
    actionLabel?: string
    actionIcon?: ReactNode
    onAction?: () => void
}

export const ResourceGroupHeader: FC<ResourceGroupHeaderProps> = ({ title, count, actionLabel, actionIcon, onAction }) => (
    <div className='group text-panel-section-header hover:bg-explorer-item-hover flex h-6 items-center gap-1.5 px-2 text-[11px] font-semibold tracking-wide uppercase'>
        <span className='truncate'>{title}</span>
        <span className='bg-app-sidebar-item-active flex h-4 min-w-4 items-center justify-center rounded-full px-1 text-[10px] font-normal normal-case'>
            {count}
        </span>
        {onAction && actionLabel && (
            <IconButton
                label={actionLabel}
                icon={actionIcon}
                onClick={onAction}
                side='bottom'
                className='hover:bg-explorer-item-selected ml-auto hidden size-4 items-center justify-center rounded-sm group-hover:flex'
            />
        )}
    </div>
)

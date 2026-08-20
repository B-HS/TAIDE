import type { FC } from 'react'
import { ChevronRight, File } from 'lucide-react'
import { cn } from '@shared/lib/cn'
import { createActivationKeyDownHandler } from '@shared/lib/activation-key'
import { fileNameOf } from '@shared/lib/relative-path'
import { Checkbox } from '@shared/ui/checkbox'

type FileGroupHeaderProps = {
    path: string
    count: number
    expanded: boolean
    onToggle: () => void
    selected?: boolean
    onToggleSelect?: () => void
    selectAriaLabel?: string
}

export const FileGroupHeader: FC<FileGroupHeaderProps> = ({ path, count, expanded, onToggle, selected, onToggleSelect, selectAriaLabel }) => (
    <div className={cn('hover:bg-explorer-item-hover flex cursor-default items-center text-xs select-none', onToggleSelect && 'pl-2')}>
        {onToggleSelect && <Checkbox checked={selected} aria-label={selectAriaLabel} onCheckedChange={onToggleSelect} />}
        <div
            role='button'
            tabIndex={0}
            aria-expanded={expanded}
            onClick={onToggle}
            onKeyDown={createActivationKeyDownHandler(onToggle)}
            className={cn('flex flex-1 items-center gap-1 py-0.5 pr-2', onToggleSelect ? 'pl-1' : 'pl-2')}>
            <ChevronRight className={cn('size-3 shrink-0', expanded && 'rotate-90')} />
            <File className='size-3.5 shrink-0 opacity-80' />
            <span className='truncate font-medium'>{fileNameOf(path)}</span>
            <span className='text-app-sidebar-icon-default truncate'>{path}</span>
            <span className='text-app-sidebar-icon-default ml-auto shrink-0 tabular-nums'>{count}</span>
        </div>
    </div>
)

import type { FC } from 'react'
import { ChevronRight, File } from 'lucide-react'
import { cn } from '@shared/lib/cn'
import { createActivationKeyDownHandler } from '@shared/lib/activation-key'
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

const fileNameOf = (path: string) => path.slice(path.lastIndexOf('/') + 1)

export const FileGroupHeader: FC<FileGroupHeaderProps> = ({ path, count, expanded, onToggle, selected, onToggleSelect, selectAriaLabel }) => (
    <div className='hover:bg-explorer-item-hover flex cursor-default items-center gap-1 py-0.5 pl-2 text-xs select-none'>
        {onToggleSelect && (
            <Checkbox checked={selected} aria-label={selectAriaLabel} onClick={(event) => event.stopPropagation()} onCheckedChange={onToggleSelect} />
        )}
        <div
            role='button'
            tabIndex={0}
            onClick={onToggle}
            onKeyDown={createActivationKeyDownHandler(onToggle)}
            className='flex flex-1 items-center gap-1 pr-2'>
            <ChevronRight className={cn('size-3 shrink-0', expanded && 'rotate-90')} />
            <File className='size-3.5 shrink-0 opacity-80' />
            <span className='truncate font-medium'>{fileNameOf(path)}</span>
            <span className='text-app-sidebar-icon-default truncate'>{path}</span>
            <span className='text-app-sidebar-icon-default ml-auto shrink-0 tabular-nums'>{count}</span>
        </div>
    </div>
)

import type { FC } from 'react'
import { ChevronRight, File } from 'lucide-react'
import { cn } from '@shared/lib/cn'

type SearchFileGroupHeaderProps = {
    path: string
    matchCount: number
    expanded: boolean
    onToggle: () => void
}

const fileNameOf = (path: string) => path.slice(path.lastIndexOf('/') + 1)

export const SearchFileGroupHeader: FC<SearchFileGroupHeaderProps> = ({ path, matchCount, expanded, onToggle }) => (
    <div
        role='button'
        tabIndex={0}
        onClick={onToggle}
        onKeyDown={(event) => event.key === 'Enter' && onToggle()}
        className='hover:bg-explorer-item-hover flex cursor-default items-center gap-1 py-0.5 pr-2 pl-2 text-xs select-none'>
        <ChevronRight className={cn('size-3 shrink-0', expanded && 'rotate-90')} />
        <File className='size-3.5 shrink-0 opacity-80' />
        <span className='truncate font-medium'>{fileNameOf(path)}</span>
        <span className='text-app-sidebar-icon-default truncate'>{path}</span>
        <span className='text-app-sidebar-icon-default ml-auto shrink-0 tabular-nums'>{matchCount}</span>
    </div>
)

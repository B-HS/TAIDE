import type { CSSProperties, FC } from 'react'
import { ChevronRight, File, Folder, FolderOpen } from 'lucide-react'
import { cn } from '@shared/lib/cn'

export type FileTreeNodeKind = 'file' | 'directory'

export type FileTreeGitStatus = 'added' | 'modified' | 'deleted' | 'conflicted' | 'ignored' | null

export type FileTreeRow = {
    id: string
    path: string
    name: string
    depth: number
    kind: FileTreeNodeKind
    expanded: boolean
    gitStatus: FileTreeGitStatus
}

const ROW_INDENT_PX = 12
const ROW_ICON_SIZE_CLASS = 'size-3.5'
const CHEVRON_SIZE_CLASS = 'size-3'

const GIT_STATUS_TEXT_CLASS: Record<Exclude<FileTreeGitStatus, null>, string> = {
    added: 'text-git-added',
    modified: 'text-git-modified',
    deleted: 'text-git-deleted',
    conflicted: 'text-git-conflicted',
    ignored: 'opacity-50',
}

type FileTreeRowItemProps = {
    row: FileTreeRow
    selected: boolean
    style: CSSProperties
    onClick: () => void
    onDoubleClick: () => void
}

export const FileTreeRowItem: FC<FileTreeRowItemProps> = ({ row, selected, style, onClick, onDoubleClick }) => (
    <div
        role='treeitem'
        aria-selected={selected}
        aria-expanded={row.kind === 'directory' ? row.expanded : undefined}
        onClick={onClick}
        onDoubleClick={onDoubleClick}
        style={{ ...style, paddingLeft: row.depth * ROW_INDENT_PX }}
        className={cn(
            'hover:bg-explorer-item-hover flex cursor-default items-center gap-1 pr-2 text-xs select-none',
            selected && 'bg-explorer-item-selected',
            row.gitStatus && GIT_STATUS_TEXT_CLASS[row.gitStatus],
        )}>
        <span className={cn('flex size-4 shrink-0 items-center justify-center', row.kind === 'file' && 'invisible')}>
            <ChevronRight className={cn(CHEVRON_SIZE_CLASS, row.expanded && 'rotate-90')} />
        </span>
        <span className='flex shrink-0 items-center justify-center opacity-80'>
            {row.kind === 'directory' ? (
                row.expanded ? (
                    <FolderOpen className={ROW_ICON_SIZE_CLASS} />
                ) : (
                    <Folder className={ROW_ICON_SIZE_CLASS} />
                )
            ) : (
                <File className={ROW_ICON_SIZE_CLASS} />
            )}
        </span>
        <span className='truncate'>{row.name}</span>
    </div>
)

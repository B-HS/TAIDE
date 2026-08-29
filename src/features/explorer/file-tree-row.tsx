import type { CSSProperties, FC } from 'react'
import { ChevronRight } from 'lucide-react'
import { cn } from '@shared/lib/cn'
import { FileTypeIcon } from '@shared/icons/file-type-icon'
import { FolderTypeIcon } from '@shared/icons/folder-type-icon'

export type FileTreeNodeKind = 'file' | 'directory'

export type FileTreeGitStatus = 'added' | 'modified' | 'deleted' | 'renamed' | 'untracked' | 'conflicted' | 'ignored' | null

export type FileTreeRow = {
    id: string
    path: string
    name: string
    depth: number
    kind: FileTreeNodeKind
    expanded: boolean
    gitStatus: FileTreeGitStatus
}

export const ROW_INDENT_PX = 12
export const ROW_ICON_SIZE_CLASS = 'size-3.5'
const CHEVRON_SIZE_CLASS = 'size-3'

const GIT_STATUS_TEXT_CLASS: Record<Exclude<FileTreeGitStatus, null>, string> = {
    added: 'text-git-added',
    modified: 'text-git-modified',
    deleted: 'text-git-deleted',
    renamed: 'text-git-renamed',
    untracked: 'text-explorer-git-untracked',
    conflicted: 'text-git-conflicted',
    ignored: 'text-explorer-git-ignored',
}

const GIT_STATUS_BADGE: Record<Exclude<FileTreeGitStatus, null | 'ignored'>, string> = {
    added: 'A',
    modified: 'M',
    deleted: 'D',
    renamed: 'R',
    untracked: 'U',
    conflicted: '!',
}

type FileTreeRowItemProps = {
    row: FileTreeRow
    selected: boolean
    focused: boolean
    style: CSSProperties
    onClick: () => void
    onDoubleClick: () => void
}

export const FileTreeRowItem: FC<FileTreeRowItemProps> = ({ row, selected, focused, style, onClick, onDoubleClick }) => (
    <div
        role='treeitem'
        aria-selected={selected}
        aria-expanded={row.kind === 'directory' ? row.expanded : undefined}
        onClick={onClick}
        onDoubleClick={onDoubleClick}
        style={{ ...style, paddingLeft: row.depth * ROW_INDENT_PX }}
        className={cn(
            'hover:bg-explorer-item-hover flex cursor-default items-center gap-1 pr-2 text-xs transition-colors select-none',
            selected && (focused ? 'bg-explorer-item-selected' : 'bg-explorer-item-focused'),
            row.gitStatus && GIT_STATUS_TEXT_CLASS[row.gitStatus],
        )}>
        <span className={cn('flex size-4 shrink-0 items-center justify-center', row.kind === 'file' && 'invisible')}>
            <ChevronRight className={cn(CHEVRON_SIZE_CLASS, row.expanded && 'rotate-90')} />
        </span>
        <span className='flex shrink-0 items-center justify-center'>
            {row.kind === 'directory' ? (
                <FolderTypeIcon folderName={row.name} expanded={row.expanded} className={ROW_ICON_SIZE_CLASS} />
            ) : (
                <FileTypeIcon fileName={row.name} className={ROW_ICON_SIZE_CLASS} />
            )}
        </span>
        <span className='truncate'>{row.name}</span>
        {row.gitStatus && row.gitStatus !== 'ignored' && (
            <span aria-label={row.gitStatus} className='ml-auto flex shrink-0 items-center pl-1'>
                {row.kind === 'directory' ? <span className='size-1.5 rounded-full bg-current' /> : GIT_STATUS_BADGE[row.gitStatus]}
            </span>
        )}
    </div>
)

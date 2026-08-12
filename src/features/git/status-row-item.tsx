import type { FC, ReactNode } from 'react'
import { cn } from '@shared/lib/cn'
import { ICON_BUTTON_CLASS } from '@shared/constants/ui-class'
import { IconButton } from '@shared/ui/icon-button'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'

export type GitStatusChangeKind = 'modified' | 'added' | 'deleted' | 'renamed' | 'untracked' | 'typeChange' | 'conflicted'

const STATUS_LETTER: Record<GitStatusChangeKind, string> = {
    modified: 'M',
    added: 'A',
    deleted: 'D',
    renamed: 'R',
    untracked: 'U',
    typeChange: 'T',
    conflicted: '!',
}

const STATUS_TEXT_CLASS: Record<GitStatusChangeKind, string> = {
    modified: 'text-git-modified',
    added: 'text-git-added',
    deleted: 'text-git-deleted',
    renamed: 'text-git-renamed',
    untracked: 'text-git-untracked',
    typeChange: 'text-git-modified',
    conflicted: 'text-git-conflicted',
}

export type StatusRowAction = {
    id: string
    label: string
    icon: ReactNode
    onClick: () => void
}

type StatusRowItemProps = {
    path: string
    origPath: string | null
    kind: GitStatusChangeKind
    selected: boolean
    actions: StatusRowAction[]
    onClick: () => void
}

export const StatusRowItem: FC<StatusRowItemProps> = ({ path, origPath, kind, selected, actions, onClick }) => {
    const lastSlashIndex = path.lastIndexOf('/')
    const fileName = lastSlashIndex === -1 ? path : path.slice(lastSlashIndex + 1)
    const dirPath = lastSlashIndex === -1 ? '' : path.slice(0, lastSlashIndex)

    const rowContent = (
        <div
            role='button'
            tabIndex={0}
            onClick={onClick}
            className={cn(
                'group hover:bg-explorer-item-hover flex h-6 w-full cursor-default items-center gap-1.5 px-2 text-xs select-none',
                selected && 'bg-explorer-item-selected',
            )}>
            <span className='truncate'>{fileName}</span>
            {dirPath && <span className='text-app-sidebar-icon-default truncate opacity-70'>{dirPath}</span>}
            <span className='ml-auto flex shrink-0 items-center gap-0.5'>
                <span className={cn('flex w-3 items-center justify-center font-semibold group-hover:hidden', STATUS_TEXT_CLASS[kind])}>
                    {STATUS_LETTER[kind]}
                </span>
                <span className='hidden items-center gap-0.5 group-hover:flex'>
                    {actions.map((action) => (
                        <IconButton
                            key={action.id}
                            label={action.label}
                            icon={action.icon}
                            onClick={(event) => {
                                event.stopPropagation()
                                action.onClick()
                            }}
                            side='bottom'
                            className={cn(ICON_BUTTON_CLASS, 'size-4')}
                        />
                    ))}
                </span>
            </span>
        </div>
    )

    return (
        <Tooltip>
            <TooltipTrigger asChild>{rowContent}</TooltipTrigger>
            <TooltipContent side='bottom'>{origPath ? `${origPath} → ${path}` : path}</TooltipContent>
        </Tooltip>
    )
}

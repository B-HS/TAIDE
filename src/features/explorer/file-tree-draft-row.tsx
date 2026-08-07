import type { CSSProperties, FC, KeyboardEvent } from 'react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { FileTreeNodeKind } from '@features/explorer/file-tree-row'
import { cn } from '@shared/lib/cn'
import { FileTypeIcon } from '@shared/icons/file-type-icon'
import { FolderTypeIcon } from '@shared/icons/folder-type-icon'

const ROW_INDENT_PX = 12
const ROW_ICON_SIZE_CLASS = 'size-3.5'

type FileTreeDraftRowItemProps = {
    depth: number
    kind: FileTreeNodeKind
    error: string | null
    style: CSSProperties
    onCommit: (name: string) => void
    onCancel: () => void
}

export const FileTreeDraftRowItem: FC<FileTreeDraftRowItemProps> = ({ depth, kind, error, style, onCommit, onCancel }) => {
    const { t } = useTranslation()
    const [name, setName] = useState('')

    const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
        event.stopPropagation()
        if (event.key === 'Enter') {
            event.preventDefault()
            onCommit(name)
            return
        }
        if (event.key === 'Escape') {
            event.preventDefault()
            onCancel()
        }
    }

    return (
        <div
            role='treeitem'
            aria-selected
            style={{ ...style, paddingLeft: depth * ROW_INDENT_PX }}
            className='flex items-center gap-1 pr-2 text-xs select-none'>
            <span className='flex size-4 shrink-0 items-center justify-center' />
            <span className='flex shrink-0 items-center justify-center'>
                {kind === 'directory' ? (
                    <FolderTypeIcon folderName={name} expanded={false} className={ROW_ICON_SIZE_CLASS} />
                ) : (
                    <FileTypeIcon fileName={name} className={ROW_ICON_SIZE_CLASS} />
                )}
            </span>
            <input
                autoFocus
                value={name}
                onChange={(event) => setName(event.target.value)}
                onKeyDown={handleKeyDown}
                onBlur={onCancel}
                placeholder={t('explorer.entryNamePlaceholder')}
                aria-invalid={error !== null}
                title={error ?? undefined}
                className={cn(
                    'bg-panel-input-background border-panel-input-border focus:border-app-focus-border text-app-foreground box-border h-full min-w-0 flex-1 rounded-sm border px-1 py-0 leading-none outline-none',
                    error !== null && 'border-status-error',
                )}
            />
        </div>
    )
}

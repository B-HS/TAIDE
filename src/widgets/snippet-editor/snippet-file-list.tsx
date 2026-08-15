import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import type { SnippetFile } from '@shared/api/bindings'
import { cn } from '@shared/lib/cn'

type SnippetFileListProps = {
    files: readonly SnippetFile[]
    selectedFileName: string | null
    onSelect: (fileName: string) => void
}

export const SnippetFileList: FC<SnippetFileListProps> = ({ files, selectedFileName, onSelect }) => {
    const { t } = useTranslation()

    if (files.length === 0) return <span className='text-app-sidebar-icon-default text-xs'>{t('snippetEditor.noFiles')}</span>

    return (
        <ul className='flex flex-col gap-1'>
            {files.map((file) => (
                <li key={file.fileName}>
                    <button
                        type='button'
                        onClick={() => onSelect(file.fileName)}
                        aria-pressed={file.fileName === selectedFileName}
                        className={cn(
                            'border-app-border hover:bg-app-sidebar-item-hover w-full truncate rounded-md border px-3 py-1.5 text-left text-xs',
                            file.fileName === selectedFileName && 'border-app-focus-border bg-app-sidebar-item-active',
                        )}>
                        {file.fileName}
                    </button>
                </li>
            ))}
        </ul>
    )
}

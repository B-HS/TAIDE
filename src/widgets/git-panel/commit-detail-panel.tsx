import type { FC } from 'react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { ArrowLeft, X } from 'lucide-react'
import type { CommitFile, GitChangeKind, ProjectId } from '@shared/api/bindings'
import { cn } from '@shared/lib/cn'
import { gitCommitFilesQueryOptions } from '@entities/git/git.query'
import { ResourceGroupHeader } from '@features/git/resource-group-header'
import { CommitFileDiff } from '@widgets/commit-file-diff/commit-file-diff'
import type { GraphLogEntry } from '@widgets/git-panel/commit-graph'
import { IconButton } from '@shared/ui/icon-button'

type CommitDetailPanelProps = {
    projectId: ProjectId
    commit: GraphLogEntry
    onClose: () => void
}

const STATUS_LETTER: Record<GitChangeKind, string> = {
    modified: 'M',
    added: 'A',
    deleted: 'D',
    renamed: 'R',
    untracked: 'U',
    typeChange: 'T',
    conflicted: '!',
}

const STATUS_TEXT_CLASS: Record<GitChangeKind, string> = {
    modified: 'text-git-modified',
    added: 'text-git-added',
    deleted: 'text-git-deleted',
    renamed: 'text-git-renamed',
    untracked: 'text-git-untracked',
    typeChange: 'text-git-modified',
    conflicted: 'text-git-conflicted',
}

const beforePathOf = (file: CommitFile) => (file.kind === 'renamed' ? (file.origPath ?? file.path) : file.path)

export const CommitDetailPanel: FC<CommitDetailPanelProps> = ({ projectId, commit, onClose }) => {
    const { t } = useTranslation()
    const [selectedFile, setSelectedFile] = useState<CommitFile | null>(null)

    const { data: files = [], isPending, isError } = useQuery(gitCommitFilesQueryOptions({ projectId, rev: commit.id }))
    const parentRev = commit.parents[0] ?? null

    return (
        <div className='border-app-border bg-panel-background mt-1 border-t pt-1'>
            <div className='flex items-center gap-1.5 px-2 py-1 text-xs'>
                <span className='truncate font-medium'>{commit.summary}</span>
                <span className='text-app-sidebar-icon-default shrink-0 font-mono text-[10px]'>{commit.id.slice(0, 7)}</span>
                <IconButton
                    label={t('common.close')}
                    icon={<X className='size-3.5' />}
                    onClick={onClose}
                    side='bottom'
                    containerClassName='ml-auto shrink-0'
                    className='hover:bg-explorer-item-hover flex size-5 items-center justify-center rounded-sm'
                />
            </div>

            {selectedFile ? (
                <>
                    <div className='flex items-center gap-1.5 px-2 py-1 text-xs'>
                        <IconButton
                            label={t('git.changedFiles')}
                            icon={<ArrowLeft className='size-3.5' />}
                            onClick={() => setSelectedFile(null)}
                            side='bottom'
                            className='hover:bg-explorer-item-hover flex size-5 shrink-0 items-center justify-center rounded-sm'
                        />
                        <span className='truncate'>{selectedFile.path}</span>
                    </div>
                    <div className='h-80 w-full'>
                        <CommitFileDiff
                            projectId={projectId}
                            rev={commit.id}
                            parentRev={parentRev}
                            path={selectedFile.path}
                            beforePath={beforePathOf(selectedFile)}
                            renderSideBySide={false}
                        />
                    </div>
                </>
            ) : (
                <>
                    <ResourceGroupHeader title={t('git.changedFiles')} count={files.length} />
                    {isError && <div className='text-status-error px-2 py-1.5 text-xs'>{t('editor.diffLoadFailed')}</div>}
                    {!isError && isPending && <div className='text-app-sidebar-icon-default px-2 py-1.5 text-xs'>{t('common.loading')}</div>}
                    {files.map((file) => (
                        <div
                            key={file.path}
                            role='button'
                            tabIndex={0}
                            onClick={() => setSelectedFile(file)}
                            className='hover:bg-explorer-item-hover flex h-6 w-full cursor-default items-center gap-1.5 px-2 text-xs select-none'>
                            <span className='truncate'>{file.path}</span>
                            <span className={cn('ml-auto w-3 shrink-0 text-center font-semibold', STATUS_TEXT_CLASS[file.kind])}>
                                {STATUS_LETTER[file.kind]}
                            </span>
                        </div>
                    ))}
                </>
            )}
        </div>
    )
}

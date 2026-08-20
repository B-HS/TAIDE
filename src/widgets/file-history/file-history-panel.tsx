import type { FC } from 'react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { ArrowLeft, X } from 'lucide-react'
import type { LogEntry, ProjectId } from '@shared/api/bindings'
import { relativeTimeToken } from '@shared/lib/relative-time'
import { subscribeOpenFileHistory } from '@shared/lib/file-history-panel-bridge'
import { createActivationKeyDownHandler } from '@shared/lib/activation-key'
import { fileNameOf } from '@shared/lib/relative-path'
import { COMMIT_SHORT_HASH_LENGTH } from '@entities/git/git.constant'
import { gitFileLogQueryOptions } from '@entities/git/git.query'
import { CommitFileDiff } from '@widgets/commit-file-diff/commit-file-diff'
import { Dialog, DialogContent, DialogTitle } from '@shared/ui/dialog'
import { IconButton } from '@shared/ui/icon-button'
import { ScrollContainer } from '@shared/scroll/scroll-container'

type FileHistoryPanelProps = {
    projectId: ProjectId
}

export const FileHistoryPanel: FC<FileHistoryPanelProps> = ({ projectId }) => {
    const { t } = useTranslation()
    const [path, setPath] = useState<string | null>(null)
    const [selectedEntry, setSelectedEntry] = useState<LogEntry | null>(null)

    const { data: entries = [], isPending, isError } = useQuery(gitFileLogQueryOptions({ projectId, path }))

    const handleOpenChange = (open: boolean) => {
        if (open) return
        setPath(null)
        setSelectedEntry(null)
    }

    useEffect(
        () =>
            subscribeOpenFileHistory((requestedPath) => {
                setPath(requestedPath)
                setSelectedEntry(null)
            }),
        [],
    )

    return (
        <Dialog open={path !== null} onOpenChange={handleOpenChange}>
            <DialogContent
                showCloseButton={false}
                className='top-0 right-0 left-auto flex h-full max-h-full w-96 max-w-full translate-x-0 translate-y-0 flex-col gap-0 rounded-none rounded-l-lg p-0 sm:max-w-96'>
                <DialogTitle className='sr-only'>{t('git.fileHistory')}</DialogTitle>
                <div className='border-app-border flex h-9 shrink-0 items-center gap-1.5 border-b px-2'>
                    {selectedEntry && (
                        <IconButton
                            label={t('git.fileHistory')}
                            icon={<ArrowLeft className='size-3.5' />}
                            onClick={() => setSelectedEntry(null)}
                            side='bottom'
                            className='hover:bg-explorer-item-hover flex size-5 shrink-0 items-center justify-center rounded-sm'
                        />
                    )}
                    <span className='truncate text-xs font-medium'>{selectedEntry ? fileNameOf(path ?? '') : t('git.fileHistory')}</span>
                    <IconButton
                        label={t('common.close')}
                        icon={<X className='size-3.5' />}
                        onClick={() => handleOpenChange(false)}
                        side='bottom'
                        containerClassName='ml-auto shrink-0'
                        className='hover:bg-explorer-item-hover flex size-5 items-center justify-center rounded-sm'
                    />
                </div>

                <div className='min-h-0 flex-1'>
                    {selectedEntry ? (
                        <CommitFileDiff
                            projectId={projectId}
                            rev={selectedEntry.id}
                            parentRev={selectedEntry.parents[0] ?? null}
                            path={path ?? ''}
                            beforePath={path ?? ''}
                            renderSideBySide={false}
                        />
                    ) : (
                        <ScrollContainer className='h-full'>
                            {isError && <div className='text-status-error px-2 py-2 text-xs'>{t('editor.diffLoadFailed')}</div>}
                            {!isError && isPending && <div className='text-app-sidebar-icon-default px-2 py-2 text-xs'>{t('common.loading')}</div>}
                            {!isError && !isPending && entries.length === 0 && (
                                <div className='text-app-sidebar-icon-default px-2 py-2 text-xs'>{t('git.noFileHistory')}</div>
                            )}
                            {entries.map((entry) => {
                                const relativeTime = relativeTimeToken(entry.timeUnix ?? 0)
                                return (
                                    <div
                                        key={entry.id}
                                        role='button'
                                        tabIndex={0}
                                        onClick={() => setSelectedEntry(entry)}
                                        onKeyDown={createActivationKeyDownHandler(() => setSelectedEntry(entry))}
                                        className='hover:bg-explorer-item-hover flex h-10 w-full cursor-default flex-col justify-center gap-0.5 px-2 text-xs select-none'>
                                        <span className='truncate'>{entry.summary}</span>
                                        <span className='text-app-sidebar-icon-default flex shrink-0 gap-1.5 text-[10px]'>
                                            <span className='truncate'>{entry.author}</span>
                                            <span className='shrink-0'>{t(relativeTime.key, relativeTime.params)}</span>
                                            <span className='shrink-0 font-mono'>{entry.id.slice(0, COMMIT_SHORT_HASH_LENGTH)}</span>
                                        </span>
                                    </div>
                                )
                            })}
                        </ScrollContainer>
                    )}
                </div>
            </DialogContent>
        </Dialog>
    )
}

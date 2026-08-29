import type { FC } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import { IpcError } from '@shared/api/unwrap-result'
import { getLanguageIdFromPath } from '@shared/lib/language-from-path'
import { resolveDiffViewSettingsProps } from '@shared/lib/diff-view-settings'
import { gitShowFileQueryOptions } from '@entities/git/git.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { DiffView } from '@features/git/diff-view'

type CommitFileDiffProps = {
    projectId: ProjectId
    rev: string
    parentRev: string | null
    path: string
    beforePath: string
    renderSideBySide: boolean
}

const isNotFoundError = (error: unknown) => error instanceof IpcError && error.code === 'NotFound'

export const CommitFileDiff: FC<CommitFileDiffProps> = ({ projectId, rev, parentRev, path, beforePath, renderSideBySide }) => {
    const { t } = useTranslation()

    const originalQuery = useQuery({
        ...gitShowFileQueryOptions({ projectId, rev: parentRev, path: beforePath }),
        enabled: parentRev !== null,
    })
    const modifiedQuery = useQuery(gitShowFileQueryOptions({ projectId, rev, path }))
    const { data: settings } = useQuery(settingsQueryOptions())

    const originalFailed = originalQuery.isError && !isNotFoundError(originalQuery.error)
    const modifiedFailed = modifiedQuery.isError && !isNotFoundError(modifiedQuery.error)
    if (originalFailed || modifiedFailed) {
        return (
            <div className='bg-editor-background text-status-error flex h-full w-full items-center justify-center text-sm'>
                {t('editor.diffLoadFailed')}
            </div>
        )
    }

    const originalReady = parentRev === null || originalQuery.isSuccess || isNotFoundError(originalQuery.error)
    const modifiedReady = modifiedQuery.isSuccess || isNotFoundError(modifiedQuery.error)
    if (!originalReady || !modifiedReady) return <div className='bg-editor-background h-full w-full' />

    const original = parentRev === null || originalQuery.isError ? '' : (originalQuery.data ?? '')
    const modified = modifiedQuery.isError ? '' : (modifiedQuery.data ?? '')

    return (
        <DiffView
            original={original}
            modified={modified}
            languageId={getLanguageIdFromPath(path)}
            renderSideBySide={renderSideBySide}
            {...resolveDiffViewSettingsProps(settings)}
        />
    )
}

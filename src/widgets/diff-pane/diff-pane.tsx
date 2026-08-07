import type { FC } from 'react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import { gitDiffFileQueryOptions } from '@entities/git/git.query'
import { fileQueryOptions } from '@entities/file/file.query'
import { DiffView } from '@features/git/diff-view'

const RENDER_SIDE_BY_SIDE_TOGGLE_CODE = 'Backslash'

type DiffPaneProps = {
    projectId: ProjectId
    path: string
    staged: boolean
    compareWith: string | null
}

export const DiffPane: FC<DiffPaneProps> = ({ projectId, path, staged, compareWith }) => {
    const { t } = useTranslation()
    const [renderSideBySide, setRenderSideBySide] = useState(true)

    const {
        data: gitData,
        isPending: isGitPending,
        isError: isGitError,
    } = useQuery({ ...gitDiffFileQueryOptions({ projectId, path, mode: staged ? 'indexVsHead' : 'workdirVsIndex' }), enabled: compareWith === null })
    const { data: originalFile, isPending: isOriginalPending, isError: isOriginalError } = useQuery(fileQueryOptions(compareWith))
    const {
        data: modifiedFile,
        isPending: isModifiedPending,
        isError: isModifiedError,
    } = useQuery({ ...fileQueryOptions(path), enabled: compareWith !== null })

    const isPending = compareWith === null ? isGitPending : isOriginalPending || isModifiedPending
    const isError = compareWith === null ? isGitError : isOriginalError || isModifiedError
    const diffContent =
        compareWith !== null && originalFile && modifiedFile
            ? { original: originalFile.content, modified: modifiedFile.content, languageId: modifiedFile.languageId }
            : gitData

    useEffect(() => {
        const handleKeyDown = (event: KeyboardEvent) => {
            if (!event.altKey || event.code !== RENDER_SIDE_BY_SIDE_TOGGLE_CODE) return
            event.preventDefault()
            setRenderSideBySide((current) => !current)
        }

        window.addEventListener('keydown', handleKeyDown)
        return () => window.removeEventListener('keydown', handleKeyDown)
    }, [])

    if (isError) {
        return (
            <div className='bg-editor-background text-status-error flex h-full w-full items-center justify-center text-sm'>
                {t('editor.diffLoadFailed')}
            </div>
        )
    }

    if (isPending || !diffContent) return <div className='bg-editor-background h-full w-full' />

    return (
        <DiffView
            original={diffContent.original}
            modified={diffContent.modified}
            languageId={diffContent.languageId}
            renderSideBySide={renderSideBySide}
        />
    )
}

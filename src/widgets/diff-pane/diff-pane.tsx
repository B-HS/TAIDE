import type { FC } from 'react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import type { PaneNode, ProjectId, Tab } from '@shared/api/bindings'
import { layoutQueryOptions } from '@entities/layout/layout.query'
import { gitDiffFileQueryOptions } from '@entities/git/git.query'
import { fileQueryOptions } from '@entities/file/file.query'
import { DiffView } from '@features/git/diff-view'

const RENDER_SIDE_BY_SIDE_TOGGLE_CODE = 'Backslash'

const findDiffTabByPath = (node: PaneNode, path: string, staged: boolean): Tab | null => {
    if (node.node === 'leaf') return node.tabs.find((tab) => tab.kind.kind === 'diff' && tab.kind.path === path && tab.kind.staged === staged) ?? null
    for (const child of node.children) {
        const found = findDiffTabByPath(child, path, staged)
        if (found) return found
    }
    return null
}

type DiffPaneProps = {
    projectId: ProjectId
    path: string
    staged: boolean
}

export const DiffPane: FC<DiffPaneProps> = ({ projectId, path, staged }) => {
    const { t } = useTranslation()
    const [renderSideBySide, setRenderSideBySide] = useState(true)

    const { data: layout } = useQuery(layoutQueryOptions(projectId))
    const diffTabKind = layout ? findDiffTabByPath(layout.root, path, staged)?.kind : null
    const compareWith = diffTabKind?.kind === 'diff' ? (diffTabKind.compareWith ?? null) : null

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

    if (isPending || !diffContent) return <div className='bg-editor-background h-full w-full' />

    if (isError) {
        return (
            <div className='bg-editor-background text-status-error flex h-full w-full items-center justify-center text-sm'>
                {t('editor.diffLoadFailed')}
            </div>
        )
    }

    return (
        <DiffView
            original={diffContent.original}
            modified={diffContent.modified}
            languageId={diffContent.languageId}
            renderSideBySide={renderSideBySide}
        />
    )
}

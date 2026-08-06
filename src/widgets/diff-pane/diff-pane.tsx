import type { FC } from 'react'
import { useEffect, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import { gitDiffFileQueryOptions } from '@entities/git/git.query'
import { DiffView } from '@features/git/diff-view'

const RENDER_SIDE_BY_SIDE_TOGGLE_CODE = 'Backslash'

type DiffPaneProps = {
    projectId: ProjectId
    path: string
    staged: boolean
}

export const DiffPane: FC<DiffPaneProps> = ({ projectId, path, staged }) => {
    const [renderSideBySide, setRenderSideBySide] = useState(true)

    const { data, isPending, isError } = useQuery(gitDiffFileQueryOptions({ projectId, path, mode: staged ? 'indexVsHead' : 'workdirVsIndex' }))

    useEffect(() => {
        const handleKeyDown = (event: KeyboardEvent) => {
            if (!event.altKey || event.code !== RENDER_SIDE_BY_SIDE_TOGGLE_CODE) return
            event.preventDefault()
            setRenderSideBySide((current) => !current)
        }

        window.addEventListener('keydown', handleKeyDown)
        return () => window.removeEventListener('keydown', handleKeyDown)
    }, [])

    if (isPending) return <div className='bg-editor-background h-full w-full' />

    if (isError) {
        return (
            <div className='bg-editor-background text-status-error flex h-full w-full items-center justify-center text-sm'>
                diff 를 불러오지 못했습니다
            </div>
        )
    }

    return <DiffView original={data.original} modified={data.modified} languageId={data.languageId} renderSideBySide={renderSideBySide} />
}

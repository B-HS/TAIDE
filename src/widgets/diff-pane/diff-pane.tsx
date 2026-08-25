import type { FC } from 'react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { ProjectId } from '@shared/api/bindings'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { monaco } from '@shared/lib/monaco/setup'
import { gitDiffFileQueryOptions, gitStatusQueryOptions, useStageGitHunk, useUnstageGitHunk } from '@entities/git/git.query'
import { fileQueryOptions } from '@entities/file/file.query'
import { DiffView } from '@features/git/diff-view'
import { toHunkRange, type HunkRange } from '@widgets/diff-pane/diff-hunk-range'

const RENDER_SIDE_BY_SIDE_TOGGLE_CODE = 'Backslash'

type DiffPaneProps = {
    projectId: ProjectId
    path: string
    staged: boolean
    compareWith: string | null
}

export const DiffPane: FC<DiffPaneProps> = ({ projectId, path, staged, compareWith }) => {
    const [renderSideBySide, setRenderSideBySide] = useState(true)
    const [diffEditor, setDiffEditor] = useState<monaco.editor.IStandaloneDiffEditor | null>(null)
    const [hunkRanges, setHunkRanges] = useState<HunkRange[]>([])

    const { t } = useTranslation()

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
    const { data: gitStatus } = useQuery({ ...gitStatusQueryOptions(projectId), enabled: compareWith === null })
    const { mutate: stageHunk } = useStageGitHunk(projectId)
    const { mutate: unstageHunk } = useUnstageGitHunk(projectId)

    const isPending = compareWith === null ? isGitPending : isOriginalPending || isModifiedPending
    const isError = compareWith === null ? isGitError : isOriginalError || isModifiedError
    const diffContent =
        compareWith !== null && originalFile && modifiedFile
            ? { original: originalFile.content, modified: modifiedFile.content, languageId: modifiedFile.languageId }
            : gitData
    /**
     * Gutter hunk stage/unstage only makes sense against a real git diff (workdirVsIndex or
     * indexVsHead) of a conflict-free file — a manual file-vs-file compare has no git-stage
     * concept, and an unresolved conflict's raw marker text has no meaningful hunk-level stage
     * action (see the inline conflict decorator in `editor-pane.tsx` for that flow instead).
     */
    const isStageable = compareWith === null && !(gitStatus?.rows ?? []).some((row) => row.path === path && row.isConflicted)

    useEffect(() => {
        const handleKeyDown = (event: KeyboardEvent) => {
            if (!event.altKey || event.code !== RENDER_SIDE_BY_SIDE_TOGGLE_CODE) return
            event.preventDefault()
            setRenderSideBySide((current) => !current)
        }

        window.addEventListener('keydown', handleKeyDown)
        return () => window.removeEventListener('keydown', handleKeyDown)
    }, [])

    /**
     * `onDidUpdateDiff` alone keeps `hunkRanges` current across content changes (staged toggle,
     * path switch, a stage/unstage mutation's refetch all swap `DiffView`'s underlying models,
     * which monaco re-diffs and re-fires this event) — no need to depend on `diffContent` itself.
     * The initial read is deferred to a microtask rather than called synchronously in the effect
     * body, matching `editor-pane.tsx`'s established pattern for the same constraint (an effect
     * body must not call `setState` synchronously).
     */
    useEffect(() => {
        if (!diffEditor || !isStageable) return
        const updateHunkRanges = () => setHunkRanges((diffEditor.getLineChanges() ?? []).map(toHunkRange))
        queueMicrotask(updateHunkRanges)
        const subscription = diffEditor.onDidUpdateDiff(updateHunkRanges)
        return () => subscription.dispose()
    }, [diffEditor, isStageable])

    useEffect(() => {
        if (!diffEditor) return
        const modifiedEditor = diffEditor.getModifiedEditor()
        const decorations = isStageable
            ? hunkRanges.map((range) => ({
                  range: new monaco.Range(range.start, 1, range.end, 1),
                  options: { linesDecorationsClassName: 'taide-diff-gutter-stage', isWholeLine: true },
              }))
            : []
        const collection = modifiedEditor.createDecorationsCollection(decorations)
        return () => collection.clear()
    }, [diffEditor, hunkRanges, isStageable])

    useEffect(() => {
        if (!diffEditor || !isStageable) return
        const modifiedEditor = diffEditor.getModifiedEditor()
        const subscription = modifiedEditor.onMouseDown((event) => {
            if (event.target.type !== monaco.editor.MouseTargetType.GUTTER_LINE_DECORATIONS) return
            const line = event.target.position?.lineNumber
            if (!line) return
            const hunk = hunkRanges.find((candidate) => line >= candidate.start && line <= candidate.end)
            if (!hunk) return
            const mutate = staged ? unstageHunk : stageHunk
            mutate(
                { projectId, path, hunkStart: hunk.start, hunkEnd: hunk.end },
                { onError: (mutationError) => toast.error(describeIpcError(mutationError)) },
            )
        })
        return () => subscription.dispose()
    }, [diffEditor, hunkRanges, isStageable, staged, projectId, path, stageHunk, unstageHunk])

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
            onDiffEditorMount={setDiffEditor}
        />
    )
}

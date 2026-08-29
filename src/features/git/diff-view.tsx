import type { FC } from 'react'
import { useEffect, useRef } from 'react'
import { monaco } from '@shared/lib/monaco/setup'

export type DiffViewProps = {
    original: string
    modified: string
    languageId: string
    renderSideBySide: boolean
    hideUnchangedRegions: boolean
    showMoves: boolean
    onDiffEditorMount?: (diffEditor: monaco.editor.IStandaloneDiffEditor | null) => void
}

/**
 * Monaco's diff editor defaults to `ignoreTrimWhitespace: true`, which treats two lines differing
 * only in leading/trailing whitespace as equal. Every consumer here renders a *git* diff, where
 * that is wrong twice over: a whitespace-only change is real (git will commit it) yet drew no
 * change at all, and a whitespace-only line *inside* a changed run turned one git hunk into two
 * monaco ranges. The second one broke gutter staging outright — `git_stage_hunk`/`git_unstage_hunk`
 * match the submitted `(start, end)` against libgit2's hunk boundaries by exact equality, so
 * neither split range existed on the backend and the click failed with `error.git.hunkNotFound`
 * (audit §4-B D8). Turning the option off makes the rendered diff byte-faithful, the same
 * comparison the backend performs.
 */
export const DiffView: FC<DiffViewProps> = ({
    original,
    modified,
    languageId,
    renderSideBySide,
    hideUnchangedRegions,
    showMoves,
    onDiffEditorMount,
}) => {
    const containerRef = useRef<HTMLDivElement>(null)
    const diffEditorRef = useRef<monaco.editor.IStandaloneDiffEditor | null>(null)
    const onDiffEditorMountRef = useRef(onDiffEditorMount)

    useEffect(() => {
        onDiffEditorMountRef.current = onDiffEditorMount
    })

    useEffect(() => {
        if (!containerRef.current) return

        const diffEditor = monaco.editor.createDiffEditor(containerRef.current, {
            automaticLayout: true,
            readOnly: true,
            ignoreTrimWhitespace: false,
        })
        diffEditorRef.current = diffEditor
        onDiffEditorMountRef.current?.(diffEditor)

        return () => {
            onDiffEditorMountRef.current?.(null)
            diffEditor.dispose()
            diffEditorRef.current = null
        }
    }, [])

    useEffect(() => {
        const diffEditor = diffEditorRef.current
        if (!diffEditor) return

        const originalModel = monaco.editor.createModel(original, languageId)
        const modifiedModel = monaco.editor.createModel(modified, languageId)
        diffEditor.setModel({ original: originalModel, modified: modifiedModel })

        return () => {
            originalModel.dispose()
            modifiedModel.dispose()
        }
    }, [original, modified, languageId])

    useEffect(() => {
        diffEditorRef.current?.updateOptions({ renderSideBySide })
    }, [renderSideBySide])

    useEffect(() => {
        diffEditorRef.current?.updateOptions({ hideUnchangedRegions: { enabled: hideUnchangedRegions }, experimental: { showMoves } })
    }, [hideUnchangedRegions, showMoves])

    return <div ref={containerRef} className='h-full w-full' />
}

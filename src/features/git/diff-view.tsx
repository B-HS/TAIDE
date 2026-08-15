import type { FC } from 'react'
import { useEffect, useRef } from 'react'
import { monaco } from '@shared/lib/monaco/setup'

export type DiffViewProps = {
    original: string
    modified: string
    languageId: string
    renderSideBySide: boolean
    onDiffEditorMount?: (diffEditor: monaco.editor.IStandaloneDiffEditor | null) => void
}

export const DiffView: FC<DiffViewProps> = ({ original, modified, languageId, renderSideBySide, onDiffEditorMount }) => {
    const containerRef = useRef<HTMLDivElement>(null)
    const diffEditorRef = useRef<monaco.editor.IStandaloneDiffEditor | null>(null)
    const onDiffEditorMountRef = useRef(onDiffEditorMount)

    useEffect(() => {
        onDiffEditorMountRef.current = onDiffEditorMount
    })

    useEffect(() => {
        if (!containerRef.current) return

        const diffEditor = monaco.editor.createDiffEditor(containerRef.current, { automaticLayout: true, readOnly: true })
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

    return <div ref={containerRef} className='h-full w-full' />
}

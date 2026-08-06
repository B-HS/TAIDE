import type { FC } from 'react'
import { useEffect, useRef } from 'react'
import { monaco } from '@shared/lib/monaco/setup'
import { createModel, getModel, restoreViewState, saveViewState } from '@entities/editor/model-registry'

export type CodeEditorProps = {
    path: string
    language: string
    value: string
    readOnly: boolean
    largeFile: boolean
    onChange: (value: string) => void
    onSave: () => void
    onCursorLineChange: (line: number) => void
    onEditorMount?: (editor: monaco.editor.IStandaloneCodeEditor | null) => void
}

const LARGE_FILE_EDITOR_OPTIONS: monaco.editor.IEditorOptions = {
    minimap: { enabled: false },
    folding: false,
    bracketPairColorization: { enabled: false },
}

const DEFAULT_EDITOR_OPTIONS: monaco.editor.IEditorOptions = {
    minimap: { enabled: true },
    folding: true,
    bracketPairColorization: { enabled: true },
}

export const CodeEditor: FC<CodeEditorProps> = ({
    path,
    language,
    value,
    readOnly,
    largeFile,
    onChange,
    onSave,
    onCursorLineChange,
    onEditorMount,
}) => {
    const containerRef = useRef<HTMLDivElement>(null)
    const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null)
    const activePathRef = useRef<string | null>(null)
    const valueRef = useRef(value)
    const onChangeRef = useRef(onChange)
    const onSaveRef = useRef(onSave)
    const onCursorLineChangeRef = useRef(onCursorLineChange)
    const onEditorMountRef = useRef(onEditorMount)

    useEffect(() => {
        valueRef.current = value
        onChangeRef.current = onChange
        onSaveRef.current = onSave
        onCursorLineChangeRef.current = onCursorLineChange
        onEditorMountRef.current = onEditorMount
    })

    useEffect(() => {
        if (!containerRef.current) return

        const editor = monaco.editor.create(containerRef.current, { automaticLayout: true, largeFileOptimizations: true })
        editorRef.current = editor
        onEditorMountRef.current?.(editor)

        editor.addAction({
            id: 'taide.saveFile',
            label: 'Save File',
            keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS],
            run: () => onSaveRef.current(),
        })

        const changeSubscription = editor.onDidChangeModelContent(() => {
            const model = editor.getModel()
            if (model) onChangeRef.current(model.getValue())
        })
        const cursorSubscription = editor.onDidChangeCursorPosition((event) => onCursorLineChangeRef.current(event.position.lineNumber))

        return () => {
            changeSubscription.dispose()
            cursorSubscription.dispose()
            editor.dispose()
            editorRef.current = null
            onEditorMountRef.current?.(null)
        }
    }, [])

    useEffect(() => {
        editorRef.current?.updateOptions(largeFile ? LARGE_FILE_EDITOR_OPTIONS : DEFAULT_EDITOR_OPTIONS)
    }, [largeFile])

    useEffect(() => {
        editorRef.current?.updateOptions({ readOnly })
    }, [readOnly])

    useEffect(() => {
        const editor = editorRef.current
        if (!editor) return

        if (activePathRef.current && activePathRef.current !== path) saveViewState(activePathRef.current, editor)

        const model = getModel(path) ?? createModel(path, valueRef.current, language)
        editor.setModel(model)
        restoreViewState(path, editor)
        editor.focus()
        activePathRef.current = path
    }, [path, language])

    return <div ref={containerRef} className='h-full w-full' />
}

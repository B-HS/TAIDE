import type { FC } from 'react'
import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import type { EditorCursorBlinking, EditorCursorStyle, EditorRenderWhitespace } from '@shared/api/bindings'
import type { AiInlineCompletionClient, AiInlineCompletionConfig } from '@shared/lib/ai/inline-completion'
import { acquireAiInlineCompletionProvider } from '@shared/lib/ai/inline-completion'
import { attachAiInlineEditAction } from '@features/editor/ai-inline-edit'
import { monaco } from '@shared/lib/monaco/setup'
import { cancelAiRequest, completeAiInline } from '@entities/ai/ai.ipc'
import { getOrCreateModel, restoreViewState, saveViewState } from '@entities/editor/model-registry'

const AI_INLINE_COMPLETION_CLIENT: AiInlineCompletionClient = { complete: completeAiInline, cancel: cancelAiRequest }

export type { EditorCursorStyle, EditorRenderWhitespace }
export type EditorCursorBlinkingStyle = EditorCursorBlinking

export type CodeEditorProps = {
    path: string
    language: string
    value: string
    readOnly: boolean
    largeFile: boolean
    minimap: boolean
    fontFamily: string
    fontSize: number
    wordWrap: boolean
    lineNumbers: boolean
    tabSize: number
    insertSpaces: boolean
    detectIndentation: boolean
    renderWhitespace: EditorRenderWhitespace
    bracketPairColorization: boolean
    fontLigatures: boolean
    cursorStyle: EditorCursorStyle
    cursorBlinking: EditorCursorBlinkingStyle
    scrollBeyondLastLine: boolean
    stickyScroll: boolean
    formatOnType: boolean
    formatOnPaste: boolean
    aiAutoTabEnabled: boolean
    aiCompletionConfig: AiInlineCompletionConfig | null
    onChange: (value: string) => void
    onSave: () => void
    onCursorLineChange: (line: number) => void
    onEditorMount?: (editor: monaco.editor.IStandaloneCodeEditor | null) => void
    onMinimapToggle: (enabled: boolean) => void
}

const TOGGLE_MINIMAP_ACTION_ID = 'taide.toggleMinimap'
const TOGGLE_MINIMAP_CONTEXT_MENU_ORDER = 1.5
const LINE_NUMBERS_MIN_CHARS = 3

export const CodeEditor: FC<CodeEditorProps> = ({
    path,
    language,
    value,
    readOnly,
    largeFile,
    minimap,
    fontFamily,
    fontSize,
    wordWrap,
    lineNumbers,
    tabSize,
    insertSpaces,
    detectIndentation,
    renderWhitespace,
    bracketPairColorization,
    fontLigatures,
    cursorStyle,
    cursorBlinking,
    scrollBeyondLastLine,
    stickyScroll,
    formatOnType,
    formatOnPaste,
    aiAutoTabEnabled,
    aiCompletionConfig,
    onChange,
    onSave,
    onCursorLineChange,
    onEditorMount,
    onMinimapToggle,
}) => {
    const { t } = useTranslation()
    const containerRef = useRef<HTMLDivElement>(null)
    const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null)
    const activePathRef = useRef<string | null>(null)
    const valueRef = useRef(value)
    const initialFontFamilyRef = useRef(fontFamily)
    const initialFontSizeRef = useRef(fontSize)
    const minimapRef = useRef(minimap)
    const aiCompletionConfigRef = useRef(aiCompletionConfig)
    const onChangeRef = useRef(onChange)
    const onSaveRef = useRef(onSave)
    const onCursorLineChangeRef = useRef(onCursorLineChange)
    const onEditorMountRef = useRef(onEditorMount)
    const onMinimapToggleRef = useRef(onMinimapToggle)

    useEffect(() => {
        valueRef.current = value
        minimapRef.current = minimap
        aiCompletionConfigRef.current = aiCompletionConfig
        onChangeRef.current = onChange
        onSaveRef.current = onSave
        onCursorLineChangeRef.current = onCursorLineChange
        onEditorMountRef.current = onEditorMount
        onMinimapToggleRef.current = onMinimapToggle
    })

    useEffect(() => {
        if (!containerRef.current) return

        const editor = monaco.editor.create(containerRef.current, {
            automaticLayout: true,
            largeFileOptimizations: true,
            fontFamily: initialFontFamilyRef.current,
            fontSize: initialFontSizeRef.current,
            glyphMargin: false,
            lineNumbersMinChars: LINE_NUMBERS_MIN_CHARS,
            /**
             * Monaco's semantic-highlighting styling pass (`semanticTokensProviderStyling.js`) never
             * runs unless this is set — the option defaults to `'configuredByTheme'`, and every
             * `StandaloneTheme` hardcodes `semanticHighlighting: false` (contract §2-2). Set once at
             * construction and left `true` for the editor's lifetime — `settings.editorSemanticHighlighting`
             * on/off is instead enforced at the semantic-tokens provider itself (a request-time getter
             * gate, the `isCodeLensEnabled` precedent), which fires the provider's own `onDidChange`
             * on toggle (`use-lsp-session.ts`'s `attachLspSession`) so monaco recomputes immediately
             * without needing this option flipped. (`StandaloneEditor.create` and `.updateOptions` both
             * resolve through the same `updateConfigurationService` call, so toggling this option via
             * `updateOptions` *would* work too — TAIDE just doesn't route the setting through it, to
             * keep one enforcement point instead of two.)
             */
            'semanticHighlighting.enabled': true,
        })
        editorRef.current = editor
        onEditorMountRef.current?.(editor)

        editor.addAction({
            id: 'taide.saveFile',
            label: 'Save File',
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
        const editor = editorRef.current
        if (!editor) return
        const toggleMinimapAction = editor.addAction({
            id: TOGGLE_MINIMAP_ACTION_ID,
            label: t('editor.toggleMinimap'),
            contextMenuGroupId: 'navigation',
            contextMenuOrder: TOGGLE_MINIMAP_CONTEXT_MENU_ORDER,
            run: () => onMinimapToggleRef.current(!minimapRef.current),
        })
        return () => toggleMinimapAction.dispose()
    }, [t])

    useEffect(() => {
        const editor = editorRef.current
        if (!editor) return
        const aiInlineEdit = attachAiInlineEditAction(editor, t)
        return () => aiInlineEdit.dispose()
    }, [t])

    useEffect(() => {
        editorRef.current?.updateOptions({ folding: !largeFile, bracketPairColorization: { enabled: bracketPairColorization && !largeFile } })
    }, [largeFile, bracketPairColorization])

    useEffect(() => {
        editorRef.current?.updateOptions({ minimap: { enabled: minimap && !largeFile } })
    }, [minimap, largeFile])

    useEffect(() => {
        editorRef.current?.updateOptions({ stickyScroll: { enabled: stickyScroll } })
    }, [stickyScroll])

    useEffect(() => {
        editorRef.current?.updateOptions({
            wordWrap: wordWrap ? 'on' : 'off',
            lineNumbers: lineNumbers ? 'on' : 'off',
            tabSize,
            insertSpaces,
            detectIndentation,
            renderWhitespace,
            fontLigatures,
            cursorStyle,
            cursorBlinking,
            scrollBeyondLastLine,
            formatOnType,
            formatOnPaste,
        })
    }, [
        wordWrap,
        lineNumbers,
        tabSize,
        insertSpaces,
        detectIndentation,
        renderWhitespace,
        fontLigatures,
        cursorStyle,
        cursorBlinking,
        scrollBeyondLastLine,
        formatOnType,
        formatOnPaste,
    ])

    useEffect(() => {
        editorRef.current?.updateOptions({ readOnly })
    }, [readOnly])

    useEffect(() => {
        editorRef.current?.updateOptions({ inlineSuggest: { enabled: aiAutoTabEnabled } })
    }, [aiAutoTabEnabled])

    useEffect(() => {
        if (!aiAutoTabEnabled) return
        return acquireAiInlineCompletionProvider(monaco, () => aiCompletionConfigRef.current, AI_INLINE_COMPLETION_CLIENT)
    }, [aiAutoTabEnabled])

    useEffect(() => {
        editorRef.current?.updateOptions({ fontFamily })
    }, [fontFamily])

    useEffect(() => {
        editorRef.current?.updateOptions({ fontSize })
    }, [fontSize])

    useEffect(() => {
        const editor = editorRef.current
        if (!editor) return

        if (activePathRef.current && activePathRef.current !== path) saveViewState(activePathRef.current, editor)

        const model = getOrCreateModel(path, valueRef.current, language)
        editor.setModel(model)
        restoreViewState(path, editor)
        editor.focus()
        activePathRef.current = path
    }, [path, language])

    return <div ref={containerRef} className='h-full w-full' />
}

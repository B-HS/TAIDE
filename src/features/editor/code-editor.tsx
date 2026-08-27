import type { FC } from 'react'
import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import type { EditorCursorBlinking, EditorCursorStyle, EditorRenderWhitespace, TabId } from '@shared/api/bindings'
import type { AiInlineCompletionClient, AiInlineCompletionConfig } from '@shared/lib/ai/inline-completion'
import { acquireAiInlineCompletionProvider } from '@shared/lib/ai/inline-completion'
import { attachAiInlineEditAction } from '@features/editor/ai-inline-edit'
import { monaco } from '@shared/lib/monaco/setup'
import { cancelAiRequest, completeAiInline } from '@entities/ai/ai.ipc'
import { registerEditorInstance, unregisterEditorInstance } from '@entities/editor/editor-instance-registry'
import { getOrCreateModel, restoreViewState, saveViewState } from '@entities/editor/model-registry'

const AI_INLINE_COMPLETION_CLIENT: AiInlineCompletionClient = { complete: completeAiInline, cancel: cancelAiRequest }

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
    registryTabId?: TabId
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
    registryTabId,
}) => {
    const { t } = useTranslation()
    const containerRef = useRef<HTMLDivElement>(null)
    const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null)
    const registryTabIdRef = useRef<TabId | null>(null)
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

        /**
         * Unregisters `registryTabIdRef.current` (see the `[registryTabId]` effect below) FIRST,
         * before any of the dispose calls that follow — this is the fiber's first cleanup to run on
         * a full unmount (verified: `commitHookEffectListUnmount` walks a fiber's effect list
         * forward, so a cleanup declared earlier always runs before one declared later), and nothing
         * runs ahead of it in this function that could throw and skip it. That makes the registry
         * unregister unconditional even if `editor.dispose()`, the minimap-action cleanup, the
         * `aiInlineEdit` cleanup, or the AI inline-completion provider release below all throw —
         * React aborts a fiber's ENTIRE destroy pass (a single try/catch around the whole loop) on
         * the first cleanup that throws, so anything relying on running "later" in this same fiber
         * would otherwise be skipped (crash-class-seal-contract.md §4, lifecycle-1).
         */
        return () => {
            if (registryTabIdRef.current) {
                unregisterEditorInstance(registryTabIdRef.current)
                registryTabIdRef.current = null
            }
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

    /**
     * Registers this instance's own live monaco editor under `registryTabId` in the shared
     * `editor-instance-registry` (`breadcrumbs-bar.tsx`, `editor-area.tsx`, and
     * `status-bar-content.tsx` all read it) — moved here from `EditorPane`'s former
     * `[tabId, editor]` effect (crash-class-seal-contract.md §1-1) so the registered value can
     * never be anything but THIS component's own `editorRef.current`, never a snapshot of a
     * parent's `editor` state that could still reference a sibling instance already torn down in
     * the same commit. `registryTabId` is optional — `untitled-pane.tsx` still never passes it, so
     * this effect no-ops there. `app-file-pane.tsx` now does pass it, so `⌘S` can reach its editor
     * instance (contract `2026-08-25-d42-e2e-defects-contract.md` §3, item a).
     *
     * Declared as the LAST effect in this component (after the creation effect and every
     * option-sync effect above) so that whenever this effect's own setup runs, `editorRef.current`
     * is not just non-null but fully configured for `registryTabId`'s tab — model attached, view
     * state restored, focused, every `addAction` already registered above — before any registry
     * subscriber's synchronous `notifyTabListeners` callback can observe it. `registryTabIdRef`
     * mirrors the currently-registered id so the creation effect's cleanup (below) can reach it —
     * see that cleanup's own comment for why.
     *
     * `registryTabId` can only change while this same `CodeEditor` instance keeps running (the
     * creation effect's `[]` deps mean it is torn down only on full unmount), so a re-key here is
     * always cleanup(old id) then setup(new id) against the SAME live instance — it can never
     * register a disposed instance under a new key, because there is no "new key" to register
     * under once this component is gone: on full unmount this effect only runs its cleanup (no
     * following setup), same as every other effect torn down alongside it. That structurally
     * rules out the corpse RE-REGISTRATION this registry used to be exposed to when the parent
     * owned it (blank-window-hotfix-contract.md §1) — this is an invariant about SETUP, and holds
     * unconditionally regardless of what any cleanup does.
     *
     * The *unregistration* side needed a second device (crash-class-seal-contract.md §4,
     * lifecycle-1): React wraps a whole fiber's destroy pass in a SINGLE try/catch
     * (`commitHookEffectListUnmount`, verified against the installed `react-dom@19.2.8` source) —
     * if any effect declared before this one throws while cleaning up (`editor.dispose()`, the
     * minimap-action/`aiInlineEdit` cleanups above, or the AI inline-completion provider release),
     * the whole destroy loop aborts and this effect's own cleanup never runs, leaving a disposed
     * instance registered. So the actual unregister call for a full unmount does not live here at
     * all — it lives at the very top of the creation effect's cleanup (the fiber's FIRST cleanup,
     * with nothing ahead of it that could throw and skip it), reading `registryTabIdRef.current`
     * instead of the `registryTabId` closure. This effect's own cleanup below still calls
     * `unregisterEditorInstance` for the ordinary re-keying case (component stays mounted,
     * `registryTabId` just changes) — on a full unmount it becomes a harmless redundant no-op
     * (`unregisterEditorInstance` deletes-then-notifies, both idempotent on an already-removed key).
     */
    useEffect(() => {
        if (!registryTabId) return
        const editor = editorRef.current
        if (!editor) return
        registryTabIdRef.current = registryTabId
        registerEditorInstance(registryTabId, editor)
        return () => {
            registryTabIdRef.current = null
            unregisterEditorInstance(registryTabId)
        }
    }, [registryTabId])

    return <div ref={containerRef} className='h-full w-full' />
}

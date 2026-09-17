import type { FC } from 'react'
import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import type { EditorCursorBlinking, EditorCursorStyle, EditorRenderWhitespace, TabId } from '@shared/api/bindings'
import type { AiInlineCompletionClient, AiInlineCompletionConfig } from '@shared/lib/ai/inline-completion'
import { acquireAiInlineCompletionProvider } from '@shared/lib/ai/inline-completion'
import { attachAiInlineEditAction } from '@features/editor/ai-inline-edit'
import { attachEditorGroupShortcutActions } from '@features/editor/editor-group-shortcut-actions'
import { monaco } from '@shared/lib/monaco/setup'
import { useKeymapOverridesJson } from '@shared/hooks/use-global-keymap'
import { resolveEditorConfigModelIndent } from '@shared/lib/editorconfig'
import { cancelAiRequest, completeAiInline } from '@entities/ai/ai.ipc'
import { registerEditorInstance, unregisterEditorInstance } from '@entities/editor/editor-instance-registry'
import { getModel, getOrCreateModel, isApplyingExternalContentTo, restoreViewState, saveViewState } from '@entities/editor/model-registry'

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
    /**
     * This file's `.editorconfig` indent override, `null` for each axis it does not specify (and for
     * every host that has no file behind it — `untitled-pane`, `app-file-pane`). Deliberately
     * separate from `tabSize`/`insertSpaces` above rather than folded into them: those three are
     * monaco's *global* editor options (`IGlobalEditorOptions`), shared by every editor and model in
     * the app, so a per-file value pushed through them would be the last-mounted pane's value for
     * every open model. The override is applied to this editor's own model instead — see the
     * `[editorConfigTabSize, editorConfigInsertSpaces]` effect.
     */
    editorConfigTabSize?: number | null
    editorConfigInsertSpaces?: boolean | null
    renderWhitespace: EditorRenderWhitespace
    bracketPairColorization: boolean
    fontLigatures: boolean
    cursorStyle: EditorCursorStyle
    cursorBlinking: EditorCursorBlinkingStyle
    scrollBeyondLastLine: boolean
    stickyScroll: boolean
    bracketPairGuides: boolean
    smoothScrolling: boolean
    cursorSmoothCaretAnimation: boolean
    suggestPreview: boolean
    rulers: readonly number[]
    formatOnType: boolean
    formatOnPaste: boolean
    aiAutoTabEnabled: boolean
    aiCompletionConfig: AiInlineCompletionConfig | null
    /**
     * Whether this editor's pane is the focused one — the counterpart of `TerminalViewProps.autoFocus`,
     * and read for the same reason: every pane in the tree mounts its own editor at once when a session
     * is restored or a split is opened, so an unconditional `focus()` in the model-attach effect below
     * handed the keyboard to whichever pane happened to commit last, overriding the focused pane the
     * layout actually persisted. Optional and defaulting to `true` so the hosts that own the *only*
     * editor in their subtree (`untitled-pane`, `app-file-pane`) keep today's behavior untouched.
     */
    autoFocus?: boolean
    /**
     * Handed a lazy reader for the model's text rather than the text itself: every host debounces
     * what it does with a keystroke (hot-exit mirror, auto-save, markdown preview), so materializing
     * the whole document as a string on each `onDidChangeModelContent` builds a full copy of the file
     * per character typed only for it to be discarded before the debounce fires. Hosts that genuinely
     * need the text now call the reader immediately; the ones that don't hold onto it and read at
     * their own fire time, which also gets them the freshest text rather than the keystroke's.
     */
    onChange: (readContent: () => string) => void
    onSave: () => void
    onCursorLineChange: (line: number) => void
    /**
     * Reports each time this editor binds to a DIFFERENT path's model (the mount, and every tab
     * switch inside the same pane) — never a mere re-language re-attach, which rebinds the same
     * buffer. `hadLiveModel` is whether `model-registry` already held a model for that path *before*
     * this attach, i.e. whether the buffer handed over is one somebody else has been editing (the
     * other half of a split view, or the same pane returning to a tab it left dirty) rather than a
     * fresh one this attach just created from `value`.
     *
     * Only this component can answer that: by the time the host's own effects run, `getOrCreateModel`
     * has already created the model, so a `getModel(path)` there always says "existed". Hosts use it
     * to decide whether the buffer on screen is theirs to reconcile against disk or an unsaved draft
     * they must adopt — see `use-editor-file-persistence.ts`'s `noteModelAttach`.
     */
    onModelAttach?: (attach: { path: string; hadLiveModel: boolean }) => void
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
    editorConfigTabSize = null,
    editorConfigInsertSpaces = null,
    renderWhitespace,
    bracketPairColorization,
    fontLigatures,
    cursorStyle,
    cursorBlinking,
    scrollBeyondLastLine,
    stickyScroll,
    bracketPairGuides,
    smoothScrolling,
    cursorSmoothCaretAnimation,
    suggestPreview,
    rulers,
    formatOnType,
    formatOnPaste,
    aiAutoTabEnabled,
    aiCompletionConfig,
    autoFocus = true,
    onChange,
    onSave,
    onCursorLineChange,
    onModelAttach,
    onEditorMount,
    onMinimapToggle,
    registryTabId,
}) => {
    const { t } = useTranslation()
    const keymapOverridesJson = useKeymapOverridesJson()
    const containerRef = useRef<HTMLDivElement>(null)
    const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null)
    const registryTabIdRef = useRef<TabId | null>(null)
    const activePathRef = useRef<string | null>(null)
    const autoFocusRef = useRef(autoFocus)
    const valueRef = useRef(value)
    const initialFontFamilyRef = useRef(fontFamily)
    const initialFontSizeRef = useRef(fontSize)
    const minimapRef = useRef(minimap)
    const aiCompletionConfigRef = useRef(aiCompletionConfig)
    const onChangeRef = useRef(onChange)
    const onSaveRef = useRef(onSave)
    const onCursorLineChangeRef = useRef(onCursorLineChange)
    const onModelAttachRef = useRef(onModelAttach)
    const onEditorMountRef = useRef(onEditorMount)
    const onMinimapToggleRef = useRef(onMinimapToggle)

    useEffect(() => {
        valueRef.current = value
        minimapRef.current = minimap
        aiCompletionConfigRef.current = aiCompletionConfig
        autoFocusRef.current = autoFocus
        onChangeRef.current = onChange
        onSaveRef.current = onSave
        onCursorLineChangeRef.current = onCursorLineChange
        onModelAttachRef.current = onModelAttach
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
            if (!model || isApplyingExternalContentTo(model)) return
            onChangeRef.current(() => model.getValue())
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
        const editor = editorRef.current
        if (!editor) return
        const groupShortcuts = attachEditorGroupShortcutActions(editor, keymapOverridesJson, t)
        return () => groupShortcuts.dispose()
    }, [t, keymapOverridesJson])

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
            guides: { bracketPairs: bracketPairGuides },
            smoothScrolling,
            cursorSmoothCaretAnimation: cursorSmoothCaretAnimation ? 'on' : 'off',
            suggest: { preview: suggestPreview },
            rulers: [...rulers],
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
        bracketPairGuides,
        smoothScrolling,
        cursorSmoothCaretAnimation,
        suggestPreview,
        rulers,
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

    /**
     * Attaches this path's model, and takes keyboard focus only when both halves of "the user just
     * arrived at this buffer" hold.
     *
     * `autoFocusRef` gates on the pane: read through a ref, and deliberately absent from the
     * dependency list, so that a pane merely *becoming* focused (⌘K ⌘→, a tab-bar mousedown) does not
     * re-run the whole attach — `focusPane` and `activateTab` are separate layout mutations, and the
     * activation that changes `path` always carries the focus change with it (Rust serializes the two
     * writes and every layout response is a full snapshot), so the focused pane's own tab switches
     * still focus exactly as before.
     *
     * `activePathRef.current !== path` gates on the buffer: `language` is in the dependency list
     * because a re-language (`applyModelLanguage` after a rename changed the extension) has to re-attach,
     * but that re-run is not navigation — focusing there yanked the caret out of the terminal or the
     * search box the user was actually typing in. Tab-switch returns inside one pane keep their focus,
     * which is the documented behavior (`docs/features/editor.md` §restore).
     */
    useEffect(() => {
        const editor = editorRef.current
        if (!editor) return

        const isPathChange = activePathRef.current !== path
        if (activePathRef.current && isPathChange) saveViewState(activePathRef.current, editor)

        const hadLiveModel = !!getModel(path)
        const model = getOrCreateModel(path, valueRef.current, language)
        editor.setModel(model)
        restoreViewState(path, editor)
        if (autoFocusRef.current && isPathChange) editor.focus()
        activePathRef.current = path
        if (isPathChange) onModelAttachRef.current?.({ path, hadLiveModel })
    }, [path, language])

    /**
     * Applies this file's `.editorconfig` indentation to the *model* — the only place a per-file
     * value can live, since `tabSize`/`insertSpaces`/`detectIndentation` are global editor options
     * that monaco funnels into one shared configuration service (`updateConfigurationService` in
     * `standaloneCodeEditor.js`) and from there into every model at once.
     *
     * Declared after the model-attach effect above so `getModel()` is already this `path`'s model,
     * and before the registry effect below so a registry subscriber never observes the editor
     * mid-configuration. No-ops entirely when the file has no `.editorconfig` override, which is
     * what keeps the settings/`detectIndentation` path byte-for-byte unchanged for every other file.
     *
     * The dependency list carries more than it reads on purpose. Whenever one of the *global*
     * options this component pushes changes value, monaco's `ModelService._updateModelOptions`
     * re-derives model options for every live model from that global configuration — which would
     * silently drop this override. Its own early-out ("same indent opts") covers every other option
     * change, so the ones that can actually reach the model are exactly the four listed here beyond
     * `path`/`language`: the indent triple, and `bracketPairColorization` (with `largeFile`, which
     * is folded into the value pushed for it) since bracket colorization is itself a model creation
     * option. Re-running this effect in the same commit re-asserts the override right after that
     * happens — the option effect above is declared earlier, so it always runs first. A future
     * global option that maps to a model creation option has to be added here too (contract §5).
     *
     * What this cannot see is a *sibling* editor writing one of those globals: `bracketPairColorization`
     * is pushed as `bracketPairColorization && !largeFile`, and `largeFile` is per-pane, so another
     * pane opening or closing a large file flips the shared value and makes `ModelService` re-derive
     * every model — including this one — while none of this pane's own props changed. Known limit,
     * recorded in contract §5 rather than papered over with an `onDidChangeOptions` self-heal, which
     * would fight the keymap's manual `indentUsingSpaces`/`indentUsingTabs`/`detectIndentation`.
     */
    useEffect(() => {
        const modelIndent = resolveEditorConfigModelIndent({ editorConfigTabSize, editorConfigInsertSpaces })
        if (!modelIndent) return
        editorRef.current?.getModel()?.updateOptions(modelIndent)
    }, [path, language, tabSize, insertSpaces, detectIndentation, bracketPairColorization, largeFile, editorConfigTabSize, editorConfigInsertSpaces])

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

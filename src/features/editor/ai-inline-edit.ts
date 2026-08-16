import type { TFunction } from 'i18next'
import { toast } from 'sonner'
import { monaco } from '@shared/lib/monaco/setup'
import { stripCodeFence } from '@shared/lib/inline-edit-fence'
import type { AiInlineEditPreviewState } from '@shared/lib/inline-edit-preview-state'
import { AI_INLINE_EDIT_PREVIEW_IDLE_STATE, advanceAiInlineEditPreview } from '@shared/lib/inline-edit-preview-state'
import { AI_INLINE_EDIT_MONACO_ACTION_ID } from '@entities/ai/ai.constant'
import { cancelAiRequest } from '@entities/ai/ai.ipc'
import { requestAiInlineEdit } from '@entities/ai/ai-inline-edit.ipc'

/** Must match Rust's `INLINE_EDIT_CONTEXT_CHAR_LIMIT` (contract §3.1) — the client-side clamp only saves IPC payload size, since the backend clamps again regardless. */
const INLINE_EDIT_CONTEXT_CHAR_LIMIT = 2_000

/** Reuses Wave C's conflict-decoration family (`--taide-diff-removed-line-background`) rather than introducing a parallel style constant for the same "this text is going away" meaning. */
const INLINE_EDIT_DELETE_DECORATION_CLASS = 'taide-conflict-incoming-background'
const INLINE_EDIT_WIDGET_ID = 'taide.aiInlineEditWidget'

/** `escape()`'s status → preview-event lookup (common.md §3.4 — replaces a nested ternary). */
const ESCAPE_EVENT_BY_STATUS = { idle: null, loading: 'cancel', preview: 'reject' } as const

const buildInlineEditContext = (model: monaco.editor.ITextModel, range: monaco.Range) => {
    const startOffset = model.getOffsetAt(range.getStartPosition())
    const endOffset = model.getOffsetAt(range.getEndPosition())
    const prefixStart = model.getPositionAt(Math.max(0, startOffset - INLINE_EDIT_CONTEXT_CHAR_LIMIT))
    const suffixEnd = model.getPositionAt(Math.min(model.getValueLength(), endOffset + INLINE_EDIT_CONTEXT_CHAR_LIMIT))

    return {
        prefix: model.getValueInRange({
            startLineNumber: prefixStart.lineNumber,
            startColumn: prefixStart.column,
            endLineNumber: range.startLineNumber,
            endColumn: range.startColumn,
        }),
        suffix: model.getValueInRange({
            startLineNumber: range.endLineNumber,
            startColumn: range.endColumn,
            endLineNumber: suffixEnd.lineNumber,
            endColumn: suffixEnd.column,
        }),
    }
}

/** Empty/collapsed selection promotes to the current line (same fallback `shared/lib/editor-selection.ts`'s `resolveSelectedTextOrCurrentLine` uses), returning the range itself — needed for the decoration/edit target, not just its text. */
const resolveInlineEditTargetRange = (editorInstance: monaco.editor.IStandaloneCodeEditor): monaco.Range | null => {
    const model = editorInstance.getModel()
    if (!model) return null

    const selection = editorInstance.getSelection()
    if (selection && !selection.isEmpty()) return monaco.Range.lift(selection)

    const line = editorInstance.getPosition()?.lineNumber
    if (!line) return null
    return new monaco.Range(line, 1, line, model.getLineMaxColumn(line))
}

const createElement = <K extends keyof HTMLElementTagNameMap>(tag: K, className: string) => {
    const element = document.createElement(tag)
    element.className = className
    return element
}

/**
 * One open ⌘I session: the {@link monaco.editor.IContentWidget} input/status bar, and — once a
 * response lands — the deletion decoration + preview {@link monaco.editor.IViewZone}. The model
 * itself is never touched until {@link accept} (contract §3.2's "model unchanged until accept"
 * invariant) — everything before that is decorations/zones/DOM, all disposable without an undo step.
 */
const createInlineEditSession = (editorInstance: monaco.editor.IStandaloneCodeEditor, t: TFunction, onEnd: () => void) => {
    const model = editorInstance.getModel()
    const targetRange = model ? resolveInlineEditTargetRange(editorInstance) : null
    if (!model || !targetRange) return null

    let disposed = false
    let previewState: AiInlineEditPreviewState = AI_INLINE_EDIT_PREVIEW_IDLE_STATE
    let currentRequestId: string | null = null
    let decorationCollection: monaco.editor.IEditorDecorationsCollection | null = null
    let viewZoneId: string | null = null

    const input = createElement(
        'input',
        'w-72 min-w-0 flex-1 bg-transparent text-[13px] text-editor-foreground placeholder:text-input-placeholder outline-none',
    )
    input.type = 'text'
    input.placeholder = t('ai.inlineEditPlaceholder')
    input.autocomplete = 'off'
    input.spellcheck = false
    input.setAttribute('aria-label', t('ai.inlineEditLabel'))

    const spinner = createElement(
        'div',
        'size-3.5 shrink-0 animate-spin rounded-full border-2 border-editor-foreground/30 border-t-editor-foreground',
    )
    const loadingLabel = createElement('span', 'text-[12px] whitespace-nowrap text-editor-foreground/80')
    loadingLabel.textContent = t('ai.inlineEditGenerating')
    const cancelButton = createElement(
        'button',
        'shrink-0 rounded px-1.5 py-0.5 text-[12px] text-editor-foreground/70 hover:bg-list-hover-background hover:text-editor-foreground',
    )
    cancelButton.type = 'button'
    cancelButton.textContent = '×'
    cancelButton.title = t('common.cancel')
    cancelButton.setAttribute('aria-label', t('common.cancel'))

    const rejectButton = createElement('button', 'shrink-0 rounded px-2 py-0.5 text-[12px] text-editor-foreground/80 hover:bg-list-hover-background')
    rejectButton.type = 'button'
    rejectButton.textContent = t('ai.inlineEditReject')
    const acceptButton = createElement('button', 'shrink-0 rounded bg-primary px-2 py-0.5 text-[12px] text-primary-foreground hover:bg-primary/90')
    acceptButton.type = 'button'
    acceptButton.textContent = t('ai.inlineEditAccept')

    const editingGroup = createElement('div', 'flex flex-1 items-center')
    editingGroup.append(input)
    const loadingGroup = createElement('div', 'flex flex-1 items-center gap-2')
    loadingGroup.append(spinner, loadingLabel, createElement('div', 'flex-1'), cancelButton)
    loadingGroup.hidden = true
    const previewGroup = createElement('div', 'flex flex-1 items-center justify-end gap-2')
    previewGroup.append(rejectButton, acceptButton)
    previewGroup.hidden = true

    const container = createElement(
        'div',
        'flex min-w-72 items-center gap-2 rounded-md border border-editor-widget-border bg-editor-widget-background px-2 py-1.5 shadow-lg',
    )
    container.append(editingGroup, loadingGroup, previewGroup)

    const contentWidget: monaco.editor.IContentWidget = {
        getId: () => INLINE_EDIT_WIDGET_ID,
        getDomNode: () => container,
        getPosition: () => ({
            position: { lineNumber: targetRange.startLineNumber, column: targetRange.startColumn },
            preference: [monaco.editor.ContentWidgetPositionPreference.ABOVE, monaco.editor.ContentWidgetPositionPreference.BELOW],
        }),
        allowEditorOverflow: true,
    }
    editorInstance.addContentWidget(contentWidget)

    /**
     * Toggling `hidden` on `editingGroup`/`loadingGroup`/`previewGroup` can move DOM focus off
     * whichever control currently holds it (a hidden element can't hold focus) out to
     * `document.body` — which would break the container-level keydown listener below (Esc/⌘Enter
     * would never reach it again, since they'd fire on `body`, outside `container`). Explicitly
     * re-focusing the newly-visible group's primary control keeps focus inside `container` through
     * every status transition.
     */
    const applyStatusVisibility = () => {
        editingGroup.hidden = previewState.status !== 'idle'
        loadingGroup.hidden = previewState.status !== 'loading'
        previewGroup.hidden = previewState.status !== 'preview'
        editorInstance.layoutContentWidget(contentWidget)
        if (previewState.status === 'loading') cancelButton.focus()
        else if (previewState.status === 'preview') acceptButton.focus()
    }

    const clearPreviewVisuals = () => {
        decorationCollection?.clear()
        decorationCollection = null
        if (viewZoneId === null) return
        const zoneId = viewZoneId
        viewZoneId = null
        editorInstance.changeViewZones((accessor) => accessor.removeZone(zoneId))
    }

    const teardownWidgetAndListeners = () => {
        modelChangeSubscription.dispose()
        clearPreviewVisuals()
        editorInstance.removeContentWidget(contentWidget)
    }

    /** Cancels whatever request is in flight — a no-op if none is (idle/preview already have nothing running). */
    const cancelInFlightRequest = () => {
        if (previewState.status !== 'loading' || !currentRequestId) return
        void cancelAiRequest(currentRequestId).catch(() => undefined)
        toast.info(t('ai.inlineEditCancelled'))
    }

    /**
     * Runs on every session-ending path — Esc/reject/accept, a document edit invalidating the
     * session, and (unlike those) tab switch (`onDidChangeModel`) or the editor being disposed,
     * which skip straight here without going through {@link escape}/{@link invalidate} first.
     * Cancelling here too (idempotent — {@link cancelInFlightRequest} no-ops once the status has
     * already left `'loading'`) ensures a generating request never keeps running server-side after
     * its UI has silently vanished.
     */
    const endSession = () => {
        if (disposed) return
        disposed = true
        cancelInFlightRequest()
        teardownWidgetAndListeners()
        onEnd()
    }

    const escape = () => {
        if (disposed) return
        cancelInFlightRequest()
        const event = ESCAPE_EVENT_BY_STATUS[previewState.status]
        if (event) previewState = advanceAiInlineEditPreview(previewState, { type: event })
        endSession()
    }

    /**
     * Renders the accept/reject preview once a response lands. The {@link monaco.editor.IViewZone}
     * is sized in pixels from the editor's own live font metrics (`lineHeight`/`fontSize` options),
     * not a fixed Tailwind text/leading class — a fixed size drifts from whatever font size the
     * user has configured, clipping the zone's last line or leaving a gap under it.
     */
    const renderPreview = async (proposedText: string) => {
        if (disposed) return
        applyStatusVisibility()

        decorationCollection = editorInstance.createDecorationsCollection([
            { range: targetRange, options: { className: INLINE_EDIT_DELETE_DECORATION_CLASS } },
        ])

        const lineHeight = editorInstance.getOption(monaco.editor.EditorOption.lineHeight)
        const fontSize = editorInstance.getOption(monaco.editor.EditorOption.fontSize)
        const lineCount = proposedText.replace(/\n$/, '').split('\n').length

        const pre = createElement('pre', 'm-0 overflow-x-auto bg-editor-background px-3 font-mono whitespace-pre text-editor-foreground')
        pre.style.fontSize = `${fontSize}px`
        pre.style.lineHeight = `${lineHeight}px`
        try {
            pre.innerHTML = await monaco.editor.colorize(proposedText, model.getLanguageId(), {})
        } catch {
            pre.textContent = proposedText
        }
        if (disposed) return

        editorInstance.changeViewZones((accessor) => {
            viewZoneId = accessor.addZone({
                afterLineNumber: targetRange.endLineNumber,
                heightInPx: lineHeight * lineCount,
                domNode: pre,
            })
        })
    }

    const submit = () => {
        const instruction = input.value.trim()
        if (!instruction || previewState.status !== 'idle') return
        previewState = advanceAiInlineEditPreview(previewState, { type: 'submit' })
        applyStatusVisibility()

        const { prefix, suffix } = buildInlineEditContext(model, targetRange)
        const requestId = crypto.randomUUID()
        currentRequestId = requestId

        requestAiInlineEdit({
            requestId,
            selection: model.getValueInRange(targetRange),
            instruction,
            language: model.getLanguageId(),
            filePath: model.uri.path,
            prefix,
            suffix,
        })
            .then((response) => {
                if (disposed) return
                const strippedText = response.text ? stripCodeFence(response.text) : ''
                if (!strippedText) {
                    previewState = advanceAiInlineEditPreview(previewState, { type: 'rejectResponse' })
                    toast.error(t('ai.inlineEditEmptyResponse'))
                    endSession()
                    return
                }
                previewState = advanceAiInlineEditPreview(previewState, { type: 'resolve', text: strippedText })
                void renderPreview(strippedText)
            })
            .catch((error: unknown) => {
                if (disposed) return
                previewState = advanceAiInlineEditPreview(previewState, { type: 'rejectResponse' })
                toast.error(t('ai.inlineEditFailed'), { description: error instanceof Error ? error.message : undefined })
                endSession()
            })
    }

    const accept = () => {
        if (disposed || previewState.status !== 'preview') return
        const proposedText = previewState.text
        previewState = advanceAiInlineEditPreview(previewState, { type: 'accept' })
        disposed = true
        teardownWidgetAndListeners()
        onEnd()

        model.pushStackElement()
        editorInstance.executeEdits('taide.aiInlineEdit', [{ range: targetRange, text: proposedText }])
        model.pushStackElement()
        editorInstance.focus()
    }

    /** A document edit while a request is in flight or a preview is showing makes both stale (contract §3.2) — discarded rather than risking an edit landing against content that has since moved. */
    const invalidate = () => {
        if (disposed) return
        cancelInFlightRequest()
        previewState = advanceAiInlineEditPreview(previewState, { type: 'invalidate' })
        endSession()
    }

    const modelChangeSubscription = editorInstance.onDidChangeModelContent(invalidate)

    container.addEventListener('keydown', (event) => {
        if (event.isComposing) return
        if (event.key === 'Escape') {
            event.preventDefault()
            escape()
            return
        }
        if (event.key === 'Enter' && (event.metaKey || event.ctrlKey) && previewState.status === 'preview') {
            event.preventDefault()
            accept()
            return
        }
        if (event.key === 'Enter' && !event.metaKey && !event.ctrlKey && !event.shiftKey && previewState.status === 'idle') {
            event.preventDefault()
            submit()
        }
    })
    cancelButton.addEventListener('click', escape)
    rejectButton.addEventListener('click', escape)
    acceptButton.addEventListener('click', accept)

    requestAnimationFrame(() => input.focus())

    return { dispose: endSession, focusInput: () => input.focus() }
}

/**
 * Registers the `taide.aiInlineEdit` monaco action (⌘I) on `editorInstance` and returns a
 * disposable that unregisters it — mirrors `CodeEditor`'s existing `editor.addAction` call sites
 * (`taide.saveFile`, `taide.toggleMinimap`). At most one session is open per editor at a time;
 * re-triggering while one is already open just refocuses its input instead of stacking widgets.
 *
 * Scoped to a focused editor via `keybindingContext` (not `precondition`) deliberately — monaco's
 * `isSupported()` gates `run()` itself on `precondition`, so putting `editorTextFocus` there would
 * make the command-palette trigger a silent no-op (the palette, not the editor, holds DOM focus
 * when it dispatches this action). `keybindingContext` scopes only the ⌘I keybinding to a focused
 * editor while leaving `run()` reachable from the palette — the same split monaco's own built-in
 * actions use (e.g. `editor.action.formatDocument` puts `editorTextFocus` in `kbOpts.kbExpr`, not
 * `precondition`).
 */
export const attachAiInlineEditAction = (editorInstance: monaco.editor.IStandaloneCodeEditor, t: TFunction): monaco.IDisposable => {
    let activeSession: ReturnType<typeof createInlineEditSession> = null

    const action = editorInstance.addAction({
        id: AI_INLINE_EDIT_MONACO_ACTION_ID,
        label: t('ai.inlineEditLabel'),
        keybindingContext: 'editorTextFocus',
        keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyI],
        run: () => {
            if (activeSession) {
                activeSession.focusInput()
                return
            }
            activeSession = createInlineEditSession(editorInstance, t, () => {
                activeSession = null
            })
        },
    })

    const modelSubscription = editorInstance.onDidChangeModel(() => {
        activeSession?.dispose()
        activeSession = null
    })

    return {
        dispose: () => {
            activeSession?.dispose()
            activeSession = null
            modelSubscription.dispose()
            action.dispose()
        },
    }
}

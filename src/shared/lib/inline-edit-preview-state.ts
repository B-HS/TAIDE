export type AiInlineEditPreviewState = { status: 'idle' } | { status: 'loading' } | { status: 'preview'; text: string }

export type AiInlineEditPreviewEvent =
    | { type: 'submit' }
    | { type: 'resolve'; text: string }
    | { type: 'rejectResponse' }
    | { type: 'accept' }
    | { type: 'reject' }
    | { type: 'invalidate' }
    | { type: 'cancel' }

export const AI_INLINE_EDIT_PREVIEW_IDLE_STATE: AiInlineEditPreviewState = { status: 'idle' }

/**
 * Pure transition table for the inline-edit lifecycle: submit → loading → preview →
 * accept/reject/invalidate (back to idle). Every event not valid for the current status is a
 * no-op (state returned unchanged) rather than throwing — callers dispatch from UI/IPC callbacks
 * that can race (e.g. a stale response arriving after the user already cancelled), and silently
 * ignoring the stale transition is simpler and safer than requiring every call site to pre-check
 * the current status.
 */
export const advanceAiInlineEditPreview = (state: AiInlineEditPreviewState, event: AiInlineEditPreviewEvent): AiInlineEditPreviewState => {
    if (state.status === 'idle') return event.type === 'submit' ? { status: 'loading' } : state

    if (state.status === 'loading') {
        if (event.type === 'resolve') return { status: 'preview', text: event.text }
        if (event.type === 'rejectResponse' || event.type === 'cancel' || event.type === 'invalidate') return AI_INLINE_EDIT_PREVIEW_IDLE_STATE
        return state
    }

    if (event.type === 'accept' || event.type === 'reject' || event.type === 'invalidate') return AI_INLINE_EDIT_PREVIEW_IDLE_STATE
    return state
}

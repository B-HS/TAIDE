import { monaco } from '@shared/lib/monaco/setup'

type RevealTarget = { line: number; column: number }
type PendingReveal = { target: RevealTarget; timeoutId: ReturnType<typeof setTimeout> }

/**
 * How long a reveal request waits for its target path to open in some editor before being
 * discarded. Without an expiry, a request for a path the user never actually opens (or opens much
 * later, for an unrelated reason — e.g. clicking it in the explorer long after the search/symbol
 * navigation that originally requested the reveal) would sit in `pendingReveals` for the rest of
 * the window's session and then silently hijack the cursor the moment that path finally opens.
 */
export const REVEAL_PENDING_TTL_MS = 5_000

const pendingReveals = new Map<string, PendingReveal>()

const toKey = (path: string) => monaco.Uri.file(path).toString()

const findEditorForPath = (path: string) => {
    const key = toKey(path)
    return monaco.editor.getEditors().find((editor) => editor.getModel()?.uri.toString() === key) ?? null
}

const applyReveal = (editor: monaco.editor.ICodeEditor, target: RevealTarget) => {
    const position = { lineNumber: target.line, column: target.column }
    editor.setPosition(position)
    editor.revealPositionInCenter(position)
    editor.focus()
}

const clearPendingReveal = (key: string, pending: PendingReveal) => {
    clearTimeout(pending.timeoutId)
    pendingReveals.delete(key)
}

/**
 * `ttlMs` defaults to {@link REVEAL_PENDING_TTL_MS} and is only ever overridden by tests — every
 * real caller (`command-palette.tsx`, `editor-area.tsx`, `search-panel-container.tsx`, etc.) relies
 * on the production default.
 */
export const requestReveal = (path: string, line: number, column = 1, ttlMs: number = REVEAL_PENDING_TTL_MS) => {
    const editor = findEditorForPath(path)
    if (editor) {
        applyReveal(editor, { line, column })
        return
    }

    const key = toKey(path)
    const existing = pendingReveals.get(key)
    if (existing) clearTimeout(existing.timeoutId)
    const timeoutId = setTimeout(() => pendingReveals.delete(key), ttlMs)
    pendingReveals.set(key, { target: { line, column }, timeoutId })
}

export const consumePendingReveal = (path: string, editor: monaco.editor.ICodeEditor) => {
    const key = toKey(path)
    const pending = pendingReveals.get(key)
    if (!pending) return
    clearPendingReveal(key, pending)
    applyReveal(editor, pending.target)
}

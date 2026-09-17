import type { TabId } from '@shared/api/bindings'
import type { monaco } from '@shared/lib/monaco/setup'
import { getEditorInstance } from '@entities/editor/editor-instance-registry'

export type RevealTarget = { line: number; column: number }

type PendingReveal = { target: RevealTarget; timeoutId: ReturnType<typeof setTimeout> }

/**
 * How long a reveal request waits for its target tab to mount an editor before being discarded.
 * Without an expiry, a request for a tab that never actually mounts (the open failed, or the user
 * closed/moved the tab in the same beat) would sit in `pendingReveals` for the rest of the
 * window's session and then silently hijack the cursor the moment that tab id happens to mount
 * later.
 */
export const REVEAL_PENDING_TTL_MS = 5_000

const pendingReveals = new Map<TabId, PendingReveal>()

const applyReveal = (editor: monaco.editor.ICodeEditor, target: RevealTarget) => {
    const position = { lineNumber: target.line, column: target.column }
    editor.setPosition(position)
    editor.revealPositionInCenter(position)
    editor.focus()
}

const takePendingReveal = (tabId: TabId) => {
    const pending = pendingReveals.get(tabId)
    if (!pending) return null
    clearTimeout(pending.timeoutId)
    pendingReveals.delete(tabId)
    return pending
}

/**
 * Points a navigation (search hit, diagnostic, symbol, go-to-definition, breadcrumb) at ONE tab's
 * editor — the tab the navigation itself just opened or is already sitting in — instead of at a
 * path. The path-keyed predecessor asked monaco for "any editor in this webview whose model is
 * that file" and moved whichever one it found first, so in a split with the same file open twice
 * the wrong pane jumped (and stole focus, since {@link applyReveal} focuses) while the tab the user
 * actually opened stayed on line 1 (audit #9). A `TabId` is unique across every pane and every
 * window of a project, so there is no "first match" to get wrong.
 *
 * Both arrival orders have to work, because a caller cannot tell them apart up front:
 * `layout_open_tab` dedupes inside the target pane, so an open either returns an ALREADY MOUNTED
 * tab (nothing remounts — {@link getEditorInstance} answers right here) or activates/creates a tab
 * whose `EditorPane` mounts a commit or two later (queued until that pane calls
 * {@link consumePendingReveal}). Deliberately not `subscribeEditorInstance`: `CodeEditor` registers
 * its instance from its own effect, one commit BEFORE `EditorPane`'s `useEditorViewState` restores
 * that tab's persisted cursor/scroll, so applying from a registry subscription would let the
 * restore overwrite the reveal for any tab that has a persisted `viewState`. Consuming from
 * `EditorPane`, after that hook, keeps the reveal the last word on the cursor.
 *
 * `ttlMs` defaults to {@link REVEAL_PENDING_TTL_MS} and is only ever overridden by tests — every
 * real caller relies on the production default.
 */
export const revealInTab = (tabId: TabId, target: RevealTarget, ttlMs: number = REVEAL_PENDING_TTL_MS) => {
    takePendingReveal(tabId)

    const editor = getEditorInstance(tabId)
    if (editor) {
        applyReveal(editor, target)
        return
    }

    pendingReveals.set(tabId, { target, timeoutId: setTimeout(() => pendingReveals.delete(tabId), ttlMs) })
}

export const consumePendingReveal = (tabId: TabId, editor: monaco.editor.ICodeEditor) => {
    const pending = takePendingReveal(tabId)
    if (!pending) return
    applyReveal(editor, pending.target)
}

import type { TabId } from '@shared/api/bindings'

/**
 * Runs one tab's full save pipeline and answers whether the file is now on disk with that pane's
 * current draft. `false` means the write did not happen — a rejected `file_save`, or a refusal the
 * pipeline makes on its own (a read-only file) — and is the caller's signal that the draft is still
 * the only copy of the edit.
 */
type SaveRequest = () => Promise<boolean>

const requestsByTabId = new Map<TabId, SaveRequest>()

/**
 * The mounted counterpart of the draft-based write `use-request-close-tab.tsx` performs for a tab
 * with no pane: a way to ask the pane that owns `tabId` to save *through its own pipeline* — the
 * only path that honours the read-only refusal, Code Actions on Save, format-on-save and the
 * on-save whitespace cleanups — and to learn the outcome.
 *
 * It exists because `CodeEditor`'s `taide.saveFile` monaco action cannot report one: monaco types
 * `IEditorAction.run()` as `Promise<void>`, so a caller awaiting it can see that the handler
 * finished but never whether the write landed. Closing a dirty tab has to know the difference —
 * `useCloseTab`'s `releaseClosedFileTabPath` clears the file's hot-exit mirror and disposes its
 * monaco model, so a close that follows a failed save destroys the edit *and* both ways back
 * (audit wave 2 #8).
 *
 * Keyed by tab rather than by path because the answer belongs to a pane: a split view holds two
 * tabs on one path, and each registers its own.
 */
export const registerSaveRequest = (tabId: TabId, save: SaveRequest) => {
    requestsByTabId.set(tabId, save)
}

export const unregisterSaveRequest = (tabId: TabId) => {
    requestsByTabId.delete(tabId)
}

/** The registered save for `tabId`, or `null` when no pane is mounted for it. */
export const getSaveRequest = (tabId: TabId) => requestsByTabId.get(tabId) ?? null

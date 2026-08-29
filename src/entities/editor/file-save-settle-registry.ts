type FileSaveSettleListener = (content: string) => void

/**
 * Path-keyed listeners notified whenever a disk write for that exact path is known to have landed —
 * a ⌘S/auto-save from any pane, a Claude Code `ide:save-requested` save (`ide-sync-provider.tsx`),
 * or a merge-conflict resolution. Keyed by path rather than by tab because "saved" is a property of
 * the file, not of whichever pane happened to issue the write: a split view can hold two tabs on the
 * same path (two `EditorPane` instances sharing one monaco model through
 * `entities/editor/model-registry.ts`), and an external save has no owning pane at all. Each of
 * those panes keeps its own `dirty` / `syncedContent` / hot-exit-mirror bookkeeping in React state,
 * so a save that settled only the issuing pane left every other pane on that path still marked
 * dirty, still holding the pre-save content as its "last known disk content" (which then reads as a
 * spurious "changed on disk" conflict once the refetch lands), and still holding an armed mirror
 * write that resurrects a mirror for a file that is already clean.
 *
 * Listeners run synchronously in registration order and are isolated from each other: one that
 * throws cannot stop the rest from settling. The set is snapshotted before iterating so a listener
 * that unsubscribes itself (or another pane) mid-notification cannot corrupt the walk.
 */
const listenersByPath = new Map<string, Set<FileSaveSettleListener>>()

/** Subscribes `listener` to disk-write settlements for `path`; returns the unsubscribe function. */
export const subscribeFileSaveSettle = (path: string, listener: FileSaveSettleListener) => {
    const listeners = listenersByPath.get(path) ?? new Set<FileSaveSettleListener>()
    listeners.add(listener)
    listenersByPath.set(path, listeners)

    return () => {
        const current = listenersByPath.get(path)
        if (!current) return
        current.delete(listener)
        if (current.size === 0) listenersByPath.delete(path)
    }
}

const notify = (listener: FileSaveSettleListener, content: string) => {
    try {
        listener(content)
    } catch {
        return undefined
    }
}

/** Announces that `content` is now the on-disk text of `path`, so every pane watching it can settle. */
export const publishFileSaveSettle = (path: string, content: string) => {
    const listeners = listenersByPath.get(path)
    if (!listeners) return
    for (const listener of [...listeners]) notify(listener, content)
}

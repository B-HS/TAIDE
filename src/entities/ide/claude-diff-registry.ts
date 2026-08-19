type PendingClaudeDiff = {
    oldPath: string
    newContents: string
    tabName: string
}

const registry = new Map<string, PendingClaudeDiff>()

export const setPendingClaudeDiff = (requestId: string, pending: PendingClaudeDiff) => {
    registry.set(requestId, pending)
}

export const getPendingClaudeDiff = (requestId: string) => registry.get(requestId)

export const removePendingClaudeDiff = (requestId: string) => {
    registry.delete(requestId)
}

/**
 * Atomically reads and clears one pending entry — `null` when nothing is pending for `requestId`,
 * which callers use to tell "still needs resolving" (the diff tab closed without the user ever
 * accepting/rejecting) apart from "already resolved" (`removePendingClaudeDiff` already ran, e.g.
 * from `claude-diff-pane.tsx`'s own accept/reject handlers) without a separate existence check.
 */
export const takePendingClaudeDiffIfUnresolved = (requestId: string): PendingClaudeDiff | null => {
    const pending = registry.get(requestId)
    if (!pending) return null
    registry.delete(requestId)
    return pending
}

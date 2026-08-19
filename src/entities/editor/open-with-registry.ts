export type OpenWithOverride = 'editor'

type Listener = () => void

/**
 * Caps how many distinct file paths can hold a sticky "reopen with" override at once, as a backstop
 * behind the close-event pruning below (`setOpenWithOverride(path, null)` on tab close via
 * `useCloseTab`, `pruneOpenWithOverrides` on project close via `IpcSyncProvider`'s `projectClosed`
 * handler) — a path can still outlive every tab/project that ever held it (the same path can be
 * reopened, in a different pane or a different project entirely, long after those close events
 * fired), so this `Map` needs a capacity bound in addition to those event-driven hooks, not instead
 * of them. `setOpenWithOverride` always re-inserts the touched path last, so `Map` iteration order
 * keeps the *least recently written* path first — eviction on overflow drops that one, meaning only
 * genuinely stale entries (paths nobody has revisited in a while) are ever dropped, never the one
 * the user is actively toggling.
 */
export const OPEN_WITH_OVERRIDE_MAX_ENTRIES = 200

const overrideByPath = new Map<string, OpenWithOverride>()
const listeners = new Set<Listener>()

const notify = () => {
    for (const listener of listeners) listener()
}

const evictOldestIfOverCapacity = () => {
    if (overrideByPath.size <= OPEN_WITH_OVERRIDE_MAX_ENTRIES) return
    const oldestKey = overrideByPath.keys().next().value
    if (oldestKey !== undefined) overrideByPath.delete(oldestKey)
}

export const setOpenWithOverride = (path: string, override: OpenWithOverride | null) => {
    overrideByPath.delete(path)
    if (override !== null) {
        overrideByPath.set(path, override)
        evictOldestIfOverCapacity()
    }
    notify()
}

export const getOpenWithOverride = (path: string): OpenWithOverride | null => overrideByPath.get(path) ?? null

/**
 * Drops every override whose path is not in `keepPaths` — the eager counterpart to
 * {@link OPEN_WITH_OVERRIDE_MAX_ENTRIES}'s capacity-based eviction, for a caller that actually knows
 * which paths are still relevant rather than waiting for the LRU cap to eventually reclaim a path
 * nobody revisits. Called from `IpcSyncProvider`'s `projectClosed` handler with every path still open
 * in every *other* cached project's layout (`collectOpenFilePathsOutsideProject`), so it prunes
 * exactly the overrides that belonged only to the project that just closed. A no-op call (nothing
 * pruned) does not notify subscribers.
 */
export const pruneOpenWithOverrides = (keepPaths: readonly string[]) => {
    const keep = new Set(keepPaths)
    let pruned = false
    for (const path of overrideByPath.keys()) {
        if (keep.has(path)) continue
        overrideByPath.delete(path)
        pruned = true
    }
    if (pruned) notify()
}

export const subscribeOpenWithOverride = (listener: Listener) => {
    listeners.add(listener)
    return () => {
        listeners.delete(listener)
    }
}

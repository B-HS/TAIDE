export type OpenWithOverride = 'editor' | 'preview'

type Listener = () => void

/**
 * Caps how many distinct file paths can hold a sticky "reopen with" override at once. Without a
 * bound this `Map` grows for the entire life of the window — every path a user ever right-clicked
 * "Reopen With Editor" on (`pane-tab-bar.tsx`, `explorer-container.tsx`) stays here even after its
 * tab, and the project it belonged to, closed; a path is not itself tied to any tab/pane/project
 * lifecycle event this registry can observe (the same path can be reopened, in a different pane or
 * a different project entirely, long after every tab that once held it is gone), so eviction is by
 * capacity rather than by a specific close event. `setOpenWithOverride` always re-inserts the
 * touched path last, so `Map` iteration order keeps the *least recently written* path first —
 * eviction on overflow drops that one, meaning only genuinely stale entries (paths nobody has
 * revisited in a while) are ever dropped, never the one the user is actively toggling.
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

export const subscribeOpenWithOverride = (listener: Listener) => {
    listeners.add(listener)
    return () => {
        listeners.delete(listener)
    }
}

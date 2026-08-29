import type { ProjectId, TabId } from '@shared/api/bindings'
import type { SearchRunSnapshot } from '@entities/search/search-run-state'

export type SearchEditorFormState = {
    queryText: string
    caseSensitive: boolean
    wholeWord: boolean
    regex: boolean
    respectGitignore: boolean
    excludeGlob: string
    contextLines: number
}

export type SearchEditorMemoryEntry = {
    projectId: ProjectId
    form: SearchEditorFormState
    run: SearchRunSnapshot
}

/**
 * How many Search Editor tabs keep their remembered state. Entries are only dropped by this cap
 * (a closed tab leaves its entry behind, since nothing in the pane observes tab closure), so the
 * cap is what keeps a long session from accumulating result sets forever.
 */
export const SEARCH_EDITOR_MEMORY_LIMIT = 16

/**
 * Per-tab memory of a Search Editor's inputs and results, keyed by tab id.
 *
 * A Search Editor pane only renders while its tab is the *active* one in its pane, so clicking a
 * match — which opens the matched file in that same pane — unmounts it outright. Everything lived
 * in component state, so coming back rebuilt the pane from the tab's originally-persisted
 * `SearchQuery`: typed-but-not-yet-run query edits, toggles, context-line count and the entire
 * result list were gone, and a mount-time effect immediately re-ran the *original* query, throwing
 * away a paid-for scan and overwriting the results the user had just been looking at (audit §4-B
 * B8). Holding it here instead of in the tab's persisted `kind` keeps the fix inside the frontend —
 * persisting edited query/toggles into the tab record is a backend surface change, deferred.
 *
 * Insertion order doubles as recency (a rewrite deletes before setting), so the oldest entry is the
 * first key when the cap is exceeded.
 */
const entriesByTabId = new Map<TabId, SearchEditorMemoryEntry>()

/**
 * The remembered state for `tabId`, or `null` when this tab has never been left. `projectId` is
 * verified rather than assumed: a stale entry from a project that is no longer open must never
 * seed a pane with another project's paths (the same cross-project bleed as audit §4-B B9), and a
 * mismatch drops the entry rather than leaving it to be found again by a later read.
 */
export const readSearchEditorMemory = (tabId: TabId, projectId: ProjectId) => {
    const entry = entriesByTabId.get(tabId)
    if (!entry) return null
    if (entry.projectId !== projectId) {
        entriesByTabId.delete(tabId)
        return null
    }
    return entry
}

export const writeSearchEditorMemory = (tabId: TabId, entry: SearchEditorMemoryEntry) => {
    entriesByTabId.delete(tabId)
    entriesByTabId.set(tabId, entry)

    while (entriesByTabId.size > SEARCH_EDITOR_MEMORY_LIMIT) {
        const oldest = entriesByTabId.keys().next()
        if (oldest.done) return
        entriesByTabId.delete(oldest.value)
    }
}

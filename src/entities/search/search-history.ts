import { useRef } from 'react'
import { useQuery } from '@tanstack/react-query'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import { settingsQueryOptions, useUpdateSettings } from '@entities/settings/settings.query'

export const SEARCH_HISTORY_LIMIT = 20

export const addRecentSearchTerm = (history: string[], term: string) => {
    const trimmed = term.trim()
    if (!trimmed) return history
    if (history[0] === trimmed) return history
    return [trimmed, ...history.filter((entry) => entry !== trimmed)].slice(0, SEARCH_HISTORY_LIMIT)
}

/**
 * Sequencing state for recent-search writes, so consecutive searches compose instead of racing.
 *
 * Recording a term is a read-modify-write of the whole settings document, and the read used to be
 * the render-time query snapshot. Two searches started inside one settings round trip therefore
 * both derived from the *pre-first-search* list, and the second write overwrote the first — the
 * earlier term vanished from history (audit §4-B D4). `pending` carries the list this hook last
 * wrote so the next term builds on it, and `inFlight` decides when the settled server value is
 * authoritative again.
 *
 * Deliberately mutable: it is scratch state owned by a single ref, never React state.
 */
export type RecentSearchQueue = {
    pending: string[] | null
    inFlight: number
}

export const createRecentSearchQueue = (): RecentSearchQueue => ({ pending: null, inFlight: 0 })

/**
 * The list to persist for `term`, or `null` when the term is already at the head (nothing to
 * write). `settled` is the last value the settings query actually returned; it is only the base
 * while no write of this queue's own is still outstanding.
 */
export const enqueueRecentSearch = (queue: RecentSearchQueue, settled: string[], term: string) => {
    const base = queue.pending ?? settled
    const next = addRecentSearchTerm(base, term)
    if (next === base) return null

    queue.pending = next
    queue.inFlight += 1
    return next
}

/** Marks one enqueued write as finished; the settled value takes over once none are outstanding. */
export const settleRecentSearch = (queue: RecentSearchQueue) => {
    queue.inFlight = Math.max(0, queue.inFlight - 1)
    if (queue.inFlight === 0) queue.pending = null
}

export const useRecentSearches = () => {
    const { data: settings } = useQuery(settingsQueryOptions())
    return settings?.recentSearches ?? []
}

export const useAddRecentSearch = () => {
    const queueRef = useRef(createRecentSearchQueue())

    const recentSearches = useRecentSearches()
    const { mutate: updateSettings } = useUpdateSettings()

    return (term: string) => {
        const next = enqueueRecentSearch(queueRef.current, recentSearches, term)
        if (!next) return
        updateSettings({ ...emptySettingsPatch(), recentSearches: next }, { onSettled: () => settleRecentSearch(queueRef.current) })
    }
}

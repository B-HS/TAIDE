import { useRef, useState } from 'react'
import { toast } from 'sonner'
import type { ProjectId, SearchMatch, SearchQuery } from '@shared/api/bindings'
import { groupSearchMatches } from '@entities/search/search-result'
import type { SearchResultGroup } from '@entities/search/search-result'
import { useAddRecentSearch } from '@entities/search/search-history'
import { cancelSearch, runSearch } from '@entities/search/search.ipc'
import { describeIpcError } from '@shared/lib/ipc-error-message'

type RunSearchOptions = {
    recordHistory: boolean
}

const DEFAULT_RUN_SEARCH_OPTIONS: RunSearchOptions = { recordHistory: true }

/**
 * Orchestrates a single search surface's streaming search run (panel or
 * Search Editor tab): collects `SearchMatch` events into grouped results,
 * tracks the searching/total-match state, and records history. `sessionId`
 * must be a value stable and unique to this surface's mount (a generated id
 * for the panel, the tab id for a Search Editor tab) — the backend cancels
 * only the previous run from the same session, so distinct surfaces never
 * truncate each other's in-flight search.
 *
 * A monotonically increasing generation ref guards against a superseded
 * run's late-arriving `onMatch`/`then`/`catch`/`finally` callbacks
 * overwriting the state of a newer run started on the same surface.
 */
export const useSearchRun = (projectId: ProjectId, sessionId: string) => {
    const generationRef = useRef(0)

    const [results, setResults] = useState<SearchResultGroup[]>([])
    const [totalMatches, setTotalMatches] = useState(0)
    const [isSearching, setIsSearching] = useState(false)

    const addRecentSearch = useAddRecentSearch()

    const run = (query: SearchQuery, options: RunSearchOptions = DEFAULT_RUN_SEARCH_OPTIONS) => {
        const generation = ++generationRef.current
        const collected: SearchMatch[] = []
        setResults([])
        setTotalMatches(0)
        setIsSearching(true)
        if (options.recordHistory) addRecentSearch(query.text)

        void cancelSearch(sessionId).catch(() => undefined)

        void runSearch({
            projectId,
            sessionId,
            query,
            onMatch: (match) => {
                if (generationRef.current !== generation) return
                collected.push(match)
                setResults(groupSearchMatches(collected))
                setTotalMatches(collected.length)
            },
        })
            .then((total) => {
                if (generationRef.current === generation) setTotalMatches(total)
            })
            .catch((error: unknown) => {
                if (generationRef.current === generation) toast.error(describeIpcError(error))
            })
            .finally(() => {
                if (generationRef.current === generation) setIsSearching(false)
            })
    }

    return { results, totalMatches, isSearching, run }
}

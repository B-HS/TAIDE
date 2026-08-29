import { useEffect, useRef, useState } from 'react'
import { toast } from 'sonner'
import type { ProjectId, SearchFileMatches, SearchQuery } from '@shared/api/bindings'
import type { SearchResultAccumulator, SearchResultGroup } from '@entities/search/search-result'
import { appendSearchFileMatches, createSearchResultAccumulator, restoreSearchResultAccumulator } from '@entities/search/search-result'
import type { SearchRunSnapshot, SearchRunStatus } from '@entities/search/search-run-state'
import { isSearchResultTruncated } from '@entities/search/search-run-state'
import { useAddRecentSearch } from '@entities/search/search-history'
import { cancelSearch, runSearch } from '@entities/search/search.ipc'
import { describeIpcError } from '@shared/lib/ipc-error-message'

type RunSearchOptions = {
    recordHistory: boolean
}

const DEFAULT_RUN_SEARCH_OPTIONS: RunSearchOptions = { recordHistory: true }

/**
 * How long arriving batches are buffered before one state update publishes them all. A timer
 * rather than `requestAnimationFrame` because a search can legitimately run in a window that is
 * not currently painting (a background auxiliary window), where rAF callbacks are throttled to a
 * standstill and results would stop appearing until the run finished.
 */
const RESULT_FLUSH_INTERVAL_MS = 50

/**
 * Orchestrates a single search surface's streaming search run (panel or
 * Search Editor tab): collects the backend's per-file `SearchFileMatches`
 * batches into grouped results, tracks the run's lifecycle status and total
 * match count, and records history. `sessionId` must be a value stable and
 * unique to this surface's mount (a generated id for the panel, the tab id for
 * a Search Editor tab) — the backend keys its cancellation token by
 * `(owner, sessionId)`, so distinct surfaces never truncate each other's
 * in-flight search.
 *
 * Arriving batches are buffered and published on a [`RESULT_FLUSH_INTERVAL_MS`] timer instead of
 * synchronously, and grouped by incremental append rather than a full rebuild. Before that, every
 * single match re-grouped the whole accumulated list and set state — 10,000 renders and O(n²)
 * grouping work for one project-wide search (audit §1-3).
 *
 * A monotonically increasing generation ref guards against a superseded
 * run's late-arriving batch/`then`/`catch` callbacks overwriting the state of a
 * newer run started on the same surface.
 *
 * Starting a run issues no explicit `searchCancel` first: `search_run`'s own `begin_search` already
 * flags the previous token for this `(owner, sessionId)` pair before registering the new one. The
 * fire-and-forget cancel that used to precede it raced that supersede — both commands await the
 * same global mutation guard, and when `search_run` won the race the trailing cancel flagged the
 * *new* token, so the run ended with zero results and no error at all (audit §4-B D4). Unmounting,
 * by contrast, does cancel: nothing supersedes a run whose surface is gone, and the backend would
 * otherwise keep scanning the whole project for a pane nobody can see.
 *
 * `initialSnapshot` seeds the hook from a previous mount's [`readSnapshot`] result, which is how a
 * Search Editor tab survives being unmounted while its pane shows another tab (audit §4-B B8).
 */
export const useSearchRun = (projectId: ProjectId, sessionId: string, initialSnapshot?: SearchRunSnapshot | null) => {
    const generationRef = useRef(0)
    const accumulatorRef = useRef<SearchResultAccumulator | null>(null)
    const pendingRef = useRef<SearchFileMatches[]>([])
    const flushTimerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    const statusRef = useRef<SearchRunStatus>(initialSnapshot?.status ?? 'idle')
    const ranQueryRef = useRef<SearchQuery | null>(initialSnapshot?.query ?? null)

    const [results, setResults] = useState<SearchResultGroup[]>(initialSnapshot?.groups ?? [])
    const [totalMatches, setTotalMatches] = useState(initialSnapshot?.totalMatches ?? 0)
    const [status, setStatusState] = useState<SearchRunStatus>(initialSnapshot?.status ?? 'idle')
    const [ranQuery, setRanQueryState] = useState<SearchQuery | null>(initialSnapshot?.query ?? null)

    const addRecentSearch = useAddRecentSearch()

    const readAccumulator = () =>
        (accumulatorRef.current ??= initialSnapshot
            ? restoreSearchResultAccumulator(initialSnapshot.groups, initialSnapshot.totalMatches)
            : createSearchResultAccumulator())

    const setStatus = (next: SearchRunStatus) => {
        statusRef.current = next
        setStatusState(next)
    }

    const setRanQuery = (next: SearchQuery | null) => {
        ranQueryRef.current = next
        setRanQueryState(next)
    }

    const cancelPendingFlush = () => {
        clearTimeout(flushTimerRef.current)
        flushTimerRef.current = undefined
    }

    const drainPendingBatches = () => {
        if (pendingRef.current.length === 0) return null
        const batches = pendingRef.current
        pendingRef.current = []
        return appendSearchFileMatches(readAccumulator(), batches)
    }

    const flushPendingBatches = () => {
        flushTimerRef.current = undefined
        const groups = drainPendingBatches()
        if (!groups) return
        setResults(groups)
        setTotalMatches(readAccumulator().totalMatches)
    }

    /**
     * The run state as it stands right now, read from refs so an unmount cleanup sees the live
     * values rather than the first render's. A still-running run reports as `completed` because
     * the unmount cancels it: what the caller stores is exactly what arrived.
     */
    const readSnapshot = (): SearchRunSnapshot => {
        cancelPendingFlush()
        drainPendingBatches()
        const accumulator = readAccumulator()

        return {
            groups: accumulator.groups,
            totalMatches: accumulator.totalMatches,
            status: statusRef.current === 'running' ? 'completed' : statusRef.current,
            query: ranQueryRef.current,
        }
    }

    const run = (query: SearchQuery, options: RunSearchOptions = DEFAULT_RUN_SEARCH_OPTIONS) => {
        const generation = ++generationRef.current
        cancelPendingFlush()
        pendingRef.current = []
        accumulatorRef.current = createSearchResultAccumulator()
        setResults([])
        setTotalMatches(0)
        setStatus('running')
        setRanQuery(query)
        if (options.recordHistory) addRecentSearch(query.text)

        void runSearch({
            projectId,
            sessionId,
            query,
            onFileMatches: (batch) => {
                if (generationRef.current !== generation) return
                pendingRef.current.push(batch)
                if (flushTimerRef.current === undefined) flushTimerRef.current = setTimeout(flushPendingBatches, RESULT_FLUSH_INTERVAL_MS)
            },
        })
            .then((total) => {
                if (generationRef.current !== generation) return
                cancelPendingFlush()
                flushPendingBatches()
                setTotalMatches(total)
                setStatus('completed')
            })
            .catch((error: unknown) => {
                if (generationRef.current !== generation) return
                toast.error(describeIpcError(error))
                setStatus('failed')
            })
    }

    useEffect(
        () => () => {
            clearTimeout(flushTimerRef.current)
            generationRef.current += 1
            if (statusRef.current !== 'running') return
            statusRef.current = 'completed'
            void cancelSearch(sessionId).catch(() => undefined)
        },
        [sessionId],
    )

    return {
        results,
        totalMatches,
        status,
        ranQuery,
        isTruncated: isSearchResultTruncated(totalMatches),
        run,
        readSnapshot,
    }
}

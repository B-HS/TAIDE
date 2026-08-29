import type { SearchQuery } from '@shared/api/bindings'
import type { SearchResultGroup } from '@entities/search/search-result'
import { SEARCH_MATCH_LIMIT } from '@shared/constants/search'

export const SEARCH_RUN_STATUSES = ['idle', 'running', 'completed', 'failed'] as const

/**
 * Lifecycle of one search surface's most recent run. Before this existed a surface only knew
 * `isSearching`, so "no search has been typed in yet", "the run failed and its results never
 * arrived", and "the project genuinely contains zero matches" all rendered the same empty-result
 * message (audit §4-B C9).
 */
export type SearchRunStatus = (typeof SEARCH_RUN_STATUSES)[number]

/**
 * Everything a surface needs to be rebuilt exactly as it was left, so a Search Editor tab that is
 * unmounted (clicking a match swaps the pane's active tab to the opened file) can come back with
 * its results and its run status intact instead of re-running from scratch — audit §4-B B8.
 */
export type SearchRunSnapshot = {
    groups: SearchResultGroup[]
    totalMatches: number
    status: SearchRunStatus
    /** The query the results in `groups` actually came from, not whatever the inputs now hold. */
    query: SearchQuery | null
}

export const SEARCH_RESULTS_VIEWS = ['hint', 'searching', 'failed', 'empty', 'results'] as const

export type SearchResultsView = (typeof SEARCH_RESULTS_VIEWS)[number]

/**
 * Which body a search surface should render. Results win over every status because matches stream
 * in while the run is still going, and a finished run with matches is the ordinary case; the
 * remaining states are the three the old `!isSearching && !hasResults` condition collapsed into one
 * "no results" message.
 */
export const resolveSearchResultsView = ({ status, hasResults }: { status: SearchRunStatus; hasResults: boolean }): SearchResultsView => {
    if (hasResults) return 'results'
    if (status === 'idle') return 'hint'
    if (status === 'running') return 'searching'
    if (status === 'failed') return 'failed'
    return 'empty'
}

/**
 * Whether the backend stopped at its shared match budget rather than at the end of the project.
 * Equality is the test rather than `>`: [`SEARCH_MATCH_LIMIT`] is an exact cap, never overshot.
 */
export const isSearchResultTruncated = (totalMatches: number) => totalMatches >= SEARCH_MATCH_LIMIT

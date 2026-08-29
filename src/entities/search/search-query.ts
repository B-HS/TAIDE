import type { SearchQuery } from '@shared/api/bindings'
import { DEFAULT_SEARCH_OPTIONS } from '@entities/search/search.type'

/**
 * Fills in every option the backend would have defaulted, so two queries built at different times
 * (one from a Search Editor tab's persisted `SearchQuery`, where the optional fields may simply be
 * absent, one from live panel inputs, where they are always explicit) can be compared field by
 * field instead of `undefined` reading as a difference.
 */
export const normalizeSearchQuery = (query: SearchQuery) => ({
    text: query.text,
    caseSensitive: query.caseSensitive ?? DEFAULT_SEARCH_OPTIONS.caseSensitive,
    wholeWord: query.wholeWord ?? DEFAULT_SEARCH_OPTIONS.wholeWord,
    regex: query.regex ?? DEFAULT_SEARCH_OPTIONS.regex,
    includeGlob: query.includeGlob ?? DEFAULT_SEARCH_OPTIONS.includeGlob,
    excludeGlob: query.excludeGlob ?? DEFAULT_SEARCH_OPTIONS.excludeGlob,
    contextLines: query.contextLines ?? DEFAULT_SEARCH_OPTIONS.contextLines,
    respectGitignore: query.respectGitignore ?? DEFAULT_SEARCH_OPTIONS.respectGitignore,
})

/**
 * Whether two queries select the same set of matches — the test "may these displayed results be
 * handed to Replace All?" reduces to (audit §4-B A5). `contextLines` is deliberately excluded: it
 * only widens the preview around a match, so changing it cannot make the replace hit a different
 * occurrence.
 */
export const isSameSearchQuery = (left: SearchQuery, right: SearchQuery) => {
    const a = normalizeSearchQuery(left)
    const b = normalizeSearchQuery(right)

    return (
        a.text === b.text &&
        a.caseSensitive === b.caseSensitive &&
        a.wholeWord === b.wholeWord &&
        a.regex === b.regex &&
        a.includeGlob === b.includeGlob &&
        a.excludeGlob === b.excludeGlob &&
        a.respectGitignore === b.respectGitignore
    )
}

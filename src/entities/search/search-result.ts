import type { SearchFileMatches, SearchLineMatch } from '@shared/api/bindings'

export type SearchMatchRowData = SearchLineMatch

export type SearchResultGroup = {
    path: string
    matches: SearchMatchRowData[]
}

/**
 * Running state of one search run's grouping, so each arriving batch costs only its own size.
 *
 * The previous shape re-grouped the entire accumulated match list from scratch on every single
 * arriving match — O(n²) over a run, up to 10,000 full rebuilds (audit §1-3). Here `indexByPath`
 * lets [`appendSearchFileMatches`] find the group a batch belongs to in O(1), and untouched group
 * objects keep their identity across appends so React skips re-rendering their rows.
 *
 * Deliberately mutable: it is per-run scratch state owned by a single ref, never React state.
 */
export type SearchResultAccumulator = {
    groups: SearchResultGroup[]
    indexByPath: Map<string, number>
    totalMatches: number
}

export const createSearchResultAccumulator = (): SearchResultAccumulator => ({ groups: [], indexByPath: new Map(), totalMatches: 0 })

/**
 * Rebuilds an accumulator around groups that were already produced by an earlier run, so a surface
 * restored from memory (a Search Editor tab returning to the foreground — audit §4-B B8) can keep
 * appending to its existing groups instead of starting a second, parallel result list. `groups` is
 * adopted as-is: the caller's array is the one the accumulator will copy from on the next append,
 * never mutated in place.
 */
export const restoreSearchResultAccumulator = (groups: SearchResultGroup[], totalMatches: number): SearchResultAccumulator => ({
    groups,
    indexByPath: new Map(groups.map((group, index) => [group.path, index])),
    totalMatches,
})

/**
 * Trims each match's `before`/`after` context lines so a source line already
 * shown by a neighboring match in the same file (as its own match line or as
 * that match's context) is never rendered twice. Matches must already be in
 * ascending source-line order (the order the backend emits them in per
 * file). A no-op when `contextLines` is `0` (every caller before Search
 * Editor), since empty `before`/`after` arrays never overlap.
 */
const dedupeAdjacentContext = (matches: SearchMatchRowData[]) => {
    let coveredThroughLine = -Infinity

    return matches.map((match, index) => {
        const beforeStartLine = match.line - match.before.length
        const beforeSkipCount = Math.max(0, coveredThroughLine + 1 - beforeStartLine)
        const before = match.before.slice(beforeSkipCount)

        coveredThroughLine = Math.max(coveredThroughLine, match.line)

        const nextLine = matches[index + 1]?.line ?? Infinity
        const afterKeepCount = Math.max(0, Math.min(match.after.length, nextLine - match.line - 1))
        const after = match.after.slice(0, afterKeepCount)

        coveredThroughLine = Math.max(coveredThroughLine, match.line + after.length)

        return { ...match, before, after }
    })
}

/**
 * Folds one flush's worth of backend batches into `accumulator` and returns the new group list.
 *
 * The backend sends one batch per file (`SearchFileMatches`), so the common path is a plain
 * append of an already-ordered group. A path arriving twice is still merged and re-deduplicated
 * rather than appended as a second group — `dedupeAdjacentContext` is idempotent, so re-running it
 * over an already-trimmed group plus the new matches is safe.
 *
 * The returned array is a fresh identity (so React re-renders the list) while every group object
 * that did not change keeps its own identity (so the rows inside it do not).
 */
export const appendSearchFileMatches = (accumulator: SearchResultAccumulator, batches: SearchFileMatches[]) => {
    if (batches.length === 0) return accumulator.groups

    const next = [...accumulator.groups]

    for (const batch of batches) {
        accumulator.totalMatches += batch.matches.length
        const existingIndex = accumulator.indexByPath.get(batch.path)
        const existing = existingIndex === undefined ? undefined : next[existingIndex]

        if (existingIndex === undefined || !existing) {
            accumulator.indexByPath.set(batch.path, next.length)
            next.push({ path: batch.path, matches: dedupeAdjacentContext(batch.matches) })
            continue
        }

        next[existingIndex] = { path: existing.path, matches: dedupeAdjacentContext([...existing.matches, ...batch.matches]) }
    }

    accumulator.groups = next
    return next
}

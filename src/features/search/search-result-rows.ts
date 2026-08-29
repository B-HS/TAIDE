import type { SearchMatchRowData, SearchResultGroup } from '@entities/search/search-result'

export type SearchResultRow =
    | { kind: 'group'; id: string; path: string; matchCount: number; collapsed: boolean }
    | { kind: 'match'; id: string; path: string; match: SearchMatchRowData }

/**
 * Height of one file group header — `text-xs` (16px line box) plus its `py-0.5` (2px each side).
 */
const GROUP_HEADER_HEIGHT_PX = 20

/** The match line itself: a 16px `text-xs` line box with no vertical padding of its own. */
const MATCH_LINE_HEIGHT_PX = 16

/** One context line, which unlike the match line carries `py-0.5`. */
const CONTEXT_LINE_HEIGHT_PX = 20

/** `py-0.5` on the match row's own wrapper, paid once no matter how many context lines it holds. */
const MATCH_ROW_VERTICAL_PADDING_PX = 4

/**
 * Flattens grouped results into one linear row list — the shape a virtualizer needs, since it
 * addresses items by a single index and cannot descend into nested per-file children.
 *
 * A collapsed group keeps its header row and contributes none of its matches, so collapsing is the
 * same operation as before (audit §1-4: the list rendered up to 10,000 match rows plus every
 * context line into the DOM at once).
 */
export const buildSearchResultRows = (groups: SearchResultGroup[], collapsedPaths: ReadonlySet<string>): SearchResultRow[] => {
    const rows: SearchResultRow[] = []

    for (const group of groups) {
        const collapsed = collapsedPaths.has(group.path)
        rows.push({ kind: 'group', id: `group:${group.path}`, path: group.path, matchCount: group.matches.length, collapsed })
        if (collapsed) continue

        for (const match of group.matches) {
            rows.push({ kind: 'match', id: `match:${group.path}:${match.line}:${match.column}`, path: group.path, match })
        }
    }

    return rows
}

/**
 * The row's expected height. Match rows vary because Search Editor results carry context lines, so
 * this is computed from the line count rather than being one flat estimate; the virtualizer still
 * re-measures mounted rows, so this only has to be close enough for the scroll range before a row
 * has ever been on screen.
 */
export const estimateSearchResultRowHeight = (row: SearchResultRow) => {
    if (row.kind === 'group') return GROUP_HEADER_HEIGHT_PX

    const contextLineCount = row.match.before.length + row.match.after.length
    return MATCH_ROW_VERTICAL_PADDING_PX + MATCH_LINE_HEIGHT_PX + contextLineCount * CONTEXT_LINE_HEIGHT_PX
}

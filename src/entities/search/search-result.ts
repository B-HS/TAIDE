import type { SearchMatch } from '@shared/api/bindings'

export type SearchMatchRowData = Omit<SearchMatch, 'path'>

export type SearchResultGroup = {
    path: string
    matches: SearchMatchRowData[]
}

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

export const groupSearchMatches = (matches: SearchMatch[]) => {
    const byPath = new Map<string, SearchResultGroup>()
    for (const match of matches) {
        const group = byPath.get(match.path) ?? { path: match.path, matches: [] }
        group.matches.push({
            line: match.line,
            column: match.column,
            preview: match.preview,
            matchStart: match.matchStart,
            matchEnd: match.matchEnd,
            before: match.before,
            after: match.after,
        })
        byPath.set(match.path, group)
    }
    return [...byPath.values()].map((group) => ({ ...group, matches: dedupeAdjacentContext(group.matches) }))
}

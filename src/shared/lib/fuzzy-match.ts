const NOT_FOUND_INDEX = -1
const CONSECUTIVE_MATCH_BONUS = 5
const MATCH_BASE_SCORE = 1
const MAX_FUZZY_QUERY_TOKENS = 8
const QUERY_TOKEN_SEPARATOR_PATTERN = /\s+/

export type FuzzyMatch = {
    score: number
    indices: number[]
}

/**
 * Greedily matches `query` against `target` character by character (in order, not necessarily
 * consecutive) and returns the UTF-16 code-unit offsets of the matched characters in `indices` —
 * every code unit of a matched character, so a matched astral character (e.g. an emoji) contributes
 * both surrogate halves rather than splitting across a highlight boundary. Comparison lowercases
 * `target` one code point at a time (not the whole string) so a character whose lowercase form spans
 * more code units than the original (e.g. Turkish 'İ') cannot shift the indices of characters after
 * it; such a character simply won't match a single-character query instead of desyncing every
 * following offset.
 *
 * The scan is a single forward pass over `target`: greedy matching never looks back, so each target
 * code point is visited — and lowercased — exactly once however long the query is, and the pass
 * stops the moment the last query character matches. The shape this replaced (contract
 * `2026-09-04-usability-batch4-contract.md` §C.2-4 H3) paid two costs this one does not: it
 * materialized one `{ char, unitIndex }` object per code point for *every* candidate, including the
 * candidates that fail on the first query character, and its `findIndex` restarted at index 0 for
 * each query character, re-walking the already-consumed prefix to have the callback's guard reject
 * it. Measured over this app's own path corpus: 50,000 candidates went 26.3ms → 6.6ms per keystroke.
 */
export const fuzzyMatch = (query: string, target: string): FuzzyMatch | null => {
    if (query.length === 0) return { score: 0, indices: [] }

    const queryCodePoints = [...query]
    const indices: number[] = []
    let score = 0
    let queryPosition = 0
    let normalizedQueryChar = queryCodePoints[0].toLowerCase()
    let targetPosition = 0
    let targetUnitIndex = 0
    let previousMatchedPosition = NOT_FOUND_INDEX

    for (const targetChar of target) {
        if (targetChar.toLowerCase() === normalizedQueryChar) {
            const isConsecutive = previousMatchedPosition !== NOT_FOUND_INDEX && targetPosition === previousMatchedPosition + 1
            score += isConsecutive ? MATCH_BASE_SCORE + CONSECUTIVE_MATCH_BONUS : MATCH_BASE_SCORE

            for (let unitOffset = 0; unitOffset < targetChar.length; unitOffset += 1) indices.push(targetUnitIndex + unitOffset)
            previousMatchedPosition = targetPosition
            queryPosition += 1
            if (queryPosition === queryCodePoints.length) return { score, indices }
            normalizedQueryChar = queryCodePoints[queryPosition].toLowerCase()
        }
        targetPosition += 1
        targetUnitIndex += targetChar.length
    }

    return null
}

export type FuzzyRankedItem<T> = { item: T; match: FuzzyMatch }

const splitQueryIntoTokens = (query: string) =>
    query
        .trim()
        .split(QUERY_TOKEN_SEPARATOR_PATTERN)
        .filter((token) => token.length > 0)
        .slice(0, MAX_FUZZY_QUERY_TOKENS)

const matchQueryTokens = (queryTokens: string[], target: string) => {
    if (queryTokens.length <= 1) return fuzzyMatch(queryTokens[0] ?? '', target)

    const matchedIndices = new Set<number>()
    let score = 0

    for (const token of queryTokens) {
        const tokenMatch = fuzzyMatch(token, target)
        if (!tokenMatch) return null
        score += tokenMatch.score
        for (const index of tokenMatch.indices) matchedIndices.add(index)
    }

    return { score, indices: [...matchedIndices].toSorted((left, right) => left - right) }
}

/**
 * Ranks `items` by fuzzy relevance to `query`, reading each item's label exactly once. The query is
 * split on whitespace into at most {@link MAX_FUZZY_QUERY_TOKENS} tokens (surplus tokens are
 * ignored): an empty or whitespace-only query keeps every item in its original order, a single token
 * behaves exactly like {@link fuzzyMatch}, and two or more tokens must *each* match the label on
 * their own — in any order — scoring the sum of their matches and highlighting the union of their
 * indices. Without this split a spaced query can never match the palette's file mode, whose labels
 * are project-relative paths: {@link fuzzyMatch} consumes the space as an ordinary character, so
 * `search panel` looks for a literal space that no path contains.
 */
export const fuzzyFilter = <T>(query: string, items: T[], getLabel: (item: T) => string): FuzzyRankedItem<T>[] => {
    const queryTokens = splitQueryIntoTokens(query)

    return items
        .map((item) => {
            const match = matchQueryTokens(queryTokens, getLabel(item))
            return match ? { item, match } : null
        })
        .filter((ranked): ranked is FuzzyRankedItem<T> => ranked !== null)
        .toSorted((a, b) => b.match.score - a.match.score)
}

export type FuzzyHighlightSegment = { text: string; matched: boolean }

/**
 * Groups `text` into runs of consecutive matched/unmatched characters per `indices` ({@link
 * FuzzyMatch.indices}), so a renderer can wrap each matched run in a single highlight element
 * instead of one per character.
 */
export const buildFuzzyHighlightSegments = (text: string, indices: number[]): FuzzyHighlightSegment[] => {
    if (!text) return []

    const matchedIndexSet = new Set(indices)
    const segments: FuzzyHighlightSegment[] = []
    let segmentStart = 0
    let segmentMatched = matchedIndexSet.has(0)

    for (let index = 1; index < text.length; index += 1) {
        const isMatched = matchedIndexSet.has(index)
        if (isMatched === segmentMatched) continue
        segments.push({ text: text.slice(segmentStart, index), matched: segmentMatched })
        segmentStart = index
        segmentMatched = isMatched
    }
    segments.push({ text: text.slice(segmentStart), matched: segmentMatched })

    return segments
}

const NOT_FOUND_INDEX = -1
const CONSECUTIVE_MATCH_BONUS = 5
const MATCH_BASE_SCORE = 1

export type FuzzyMatch = {
    score: number
    indices: number[]
}

type TargetCodePoint = { char: string; unitIndex: number }

const toTargetCodePoints = (target: string): TargetCodePoint[] => {
    const codePoints: TargetCodePoint[] = []
    let unitIndex = 0
    for (const char of target) {
        codePoints.push({ char, unitIndex })
        unitIndex += char.length
    }
    return codePoints
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
 */
export const fuzzyMatch = (query: string, target: string): FuzzyMatch | null => {
    if (query.length === 0) return { score: 0, indices: [] }

    const targetCodePoints = toTargetCodePoints(target)

    const indices: number[] = []
    let score = 0
    let searchFromPosition = 0
    let previousMatchedPosition = NOT_FOUND_INDEX

    for (const queryChar of query) {
        const normalizedQueryChar = queryChar.toLowerCase()
        const matchedPosition = targetCodePoints.findIndex(
            (codePoint, position) => position >= searchFromPosition && codePoint.char.toLowerCase() === normalizedQueryChar,
        )
        if (matchedPosition === NOT_FOUND_INDEX) return null

        const matchedCodePoint = targetCodePoints[matchedPosition]
        const isConsecutive = previousMatchedPosition !== NOT_FOUND_INDEX && matchedPosition === previousMatchedPosition + 1
        score += isConsecutive ? MATCH_BASE_SCORE + CONSECUTIVE_MATCH_BONUS : MATCH_BASE_SCORE

        for (let unitOffset = 0; unitOffset < matchedCodePoint.char.length; unitOffset += 1) indices.push(matchedCodePoint.unitIndex + unitOffset)
        previousMatchedPosition = matchedPosition
        searchFromPosition = matchedPosition + 1
    }

    return { score, indices }
}

export type FuzzyRankedItem<T> = { item: T; match: FuzzyMatch }

export const fuzzyFilter = <T>(query: string, items: T[], getLabel: (item: T) => string): FuzzyRankedItem<T>[] =>
    items
        .map((item) => {
            const match = fuzzyMatch(query, getLabel(item))
            return match ? { item, match } : null
        })
        .filter((ranked): ranked is FuzzyRankedItem<T> => ranked !== null)
        .toSorted((a, b) => b.match.score - a.match.score)

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

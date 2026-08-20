const NOT_FOUND_INDEX = -1
const CONSECUTIVE_MATCH_BONUS = 5
const MATCH_BASE_SCORE = 1

export type FuzzyMatch = {
    score: number
    indices: number[]
}

export const fuzzyMatch = (query: string, target: string): FuzzyMatch | null => {
    if (query.length === 0) return { score: 0, indices: [] }

    const normalizedQuery = query.toLowerCase()
    const normalizedTarget = target.toLowerCase()

    const indices: number[] = []
    let score = 0
    let targetIndex = 0
    let previousMatchedIndex = NOT_FOUND_INDEX

    for (const queryChar of normalizedQuery) {
        const foundIndex = normalizedTarget.indexOf(queryChar, targetIndex)
        if (foundIndex === NOT_FOUND_INDEX) return null

        const isConsecutive = previousMatchedIndex !== NOT_FOUND_INDEX && foundIndex === previousMatchedIndex + 1
        score += isConsecutive ? MATCH_BASE_SCORE + CONSECUTIVE_MATCH_BONUS : MATCH_BASE_SCORE

        indices.push(foundIndex)
        previousMatchedIndex = foundIndex
        targetIndex = foundIndex + 1
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

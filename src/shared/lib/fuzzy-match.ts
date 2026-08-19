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

export type ListSelectionInput = {
    orderedIds: readonly string[]
    selectedIds: ReadonlySet<string>
    anchorId: string | null
    clickedId: string
    shiftKey: boolean
    additiveKey: boolean
}

export type ListSelectionResult = {
    selectedIds: Set<string>
    anchorId: string
    primaryId: string | null
}

export const resolveListSelection = ({ orderedIds, selectedIds, anchorId, clickedId, shiftKey, additiveKey }: ListSelectionInput) => {
    const visibleSelectedIds = orderedIds.filter((id) => selectedIds.has(id))
    const clickedIndex = orderedIds.indexOf(clickedId)
    const anchorIndex = anchorId ? orderedIds.indexOf(anchorId) : -1

    if (clickedIndex < 0) return { selectedIds: new Set(visibleSelectedIds), anchorId: clickedId, primaryId: null }

    if (shiftKey && anchorIndex >= 0) {
        const start = Math.min(anchorIndex, clickedIndex)
        const end = Math.max(anchorIndex, clickedIndex)
        const rangeIds = orderedIds.slice(start, end + 1)
        const nextSelectedIds = new Set(additiveKey ? [...visibleSelectedIds, ...rangeIds] : rangeIds)
        return { selectedIds: nextSelectedIds, anchorId: anchorId ?? clickedId, primaryId: clickedId }
    }

    if (additiveKey) {
        const nextSelectedIds = new Set(visibleSelectedIds.filter((id) => id !== clickedId))
        if (!selectedIds.has(clickedId)) nextSelectedIds.add(clickedId)
        return {
            selectedIds: nextSelectedIds,
            anchorId: clickedId,
            primaryId: nextSelectedIds.has(clickedId) ? clickedId : (orderedIds.findLast((id) => nextSelectedIds.has(id)) ?? null),
        }
    }

    return { selectedIds: new Set([clickedId]), anchorId: clickedId, primaryId: clickedId }
}

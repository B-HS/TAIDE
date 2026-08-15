export const SEARCH_EDITOR_MAX_CONTEXT_LINES = 10
export const SEARCH_EDITOR_MIN_CONTEXT_LINES = 0

export const clampContextLines = (value: number) => {
    if (Number.isNaN(value)) return SEARCH_EDITOR_MIN_CONTEXT_LINES
    return Math.min(SEARCH_EDITOR_MAX_CONTEXT_LINES, Math.max(SEARCH_EDITOR_MIN_CONTEXT_LINES, Math.trunc(value)))
}

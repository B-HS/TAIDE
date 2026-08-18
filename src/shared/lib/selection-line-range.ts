export type LineSelectionRange = { startLineNumber: number; endLineNumber: number; endColumn: number }

/**
 * Resolves an editor text selection to the inclusive 1-based line range it actually covers, for
 * git line-level stage/unstage. A selection that ends at column 1 of a line (the common case when
 * a user drags/shift-clicks to the very start of the line *after* the content they meant to
 * select) has selected none of that trailing line's characters, so it is excluded — matching how
 * editors visually present "selected lines" to the user. Single-line selections are left as-is
 * regardless of `endColumn`.
 */
export const resolveSelectedLineRange = (selection: LineSelectionRange): { start: number; end: number } => {
    const endLineHasNoSelectedContent = selection.endColumn === 1 && selection.endLineNumber > selection.startLineNumber
    return { start: selection.startLineNumber, end: endLineHasNoSelectedContent ? selection.endLineNumber - 1 : selection.endLineNumber }
}

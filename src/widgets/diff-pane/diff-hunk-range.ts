export type HunkLineChange = { modifiedStartLineNumber: number; modifiedEndLineNumber: number }

export type HunkRange = { start: number; end: number }

/**
 * Converts a monaco diff editor's `ILineChange` (modified side) into the 1-based inclusive line
 * range `git_stage_hunk`/`git_unstage_hunk` expect. Monaco marks a pure deletion (nothing added
 * on the modified side) with `modifiedEndLineNumber: 0` and `modifiedStartLineNumber` pointing at
 * the insertion anchor — mirrors the backend's own `gutter_range` convention for the same
 * zero-line case (`new_start.max(1)` as both start and end).
 */
export const toHunkRange = (change: HunkLineChange): HunkRange => {
    if (change.modifiedEndLineNumber === 0) {
        const marker = Math.max(change.modifiedStartLineNumber, 1)
        return { start: marker, end: marker }
    }
    return { start: change.modifiedStartLineNumber, end: change.modifiedEndLineNumber }
}

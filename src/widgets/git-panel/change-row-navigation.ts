/**
 * Resolves where ArrowDown/ArrowUp should move the roving focus inside the git changes list.
 * `activeIndex` is the row currently owning focus (`-1` when focus sits outside any row — e.g. the
 * scroll container or a group header): ArrowDown then enters at the top and ArrowUp at the bottom,
 * mirroring the file tree's behavior. Movement clamps at both ends instead of wrapping. Returns
 * `-1` when there is nothing to focus so the caller can leave the event untouched.
 */
export const resolveNextChangeRowIndex = (key: 'ArrowDown' | 'ArrowUp', activeIndex: number, rowCount: number) => {
    if (rowCount <= 0) return -1
    if (activeIndex < 0) return key === 'ArrowDown' ? 0 : rowCount - 1
    return key === 'ArrowDown' ? Math.min(activeIndex + 1, rowCount - 1) : Math.max(activeIndex - 1, 0)
}

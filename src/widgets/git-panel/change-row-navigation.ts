/**
 * Every element ArrowDown/ArrowUp stops on inside the SCM panel, in document order: the collapsible
 * section headers and the change rows they contain. Headers joined the sequence when they became
 * collapsible — leaving them out would have made their Stage All / Unstage All actions reachable by
 * mouse only, since those actions only reveal on hover or focus-within.
 */
export const GIT_SECTION_ROVING_SELECTOR = '[data-git-change-row], [data-git-section-header]'

/**
 * Resolves where ArrowDown/ArrowUp should move the roving focus inside the git changes list.
 * `activeIndex` is the header or row currently owning focus (`-1` when focus sits outside every
 * {@link GIT_SECTION_ROVING_SELECTOR} match — e.g. the scroll container itself): ArrowDown then
 * enters at the top and ArrowUp at the bottom, mirroring the file tree's behavior. Movement clamps
 * at both ends instead of wrapping. Returns `-1` when there is nothing to focus so the caller can
 * leave the event untouched.
 */
export const resolveNextChangeRowIndex = (key: 'ArrowDown' | 'ArrowUp', activeIndex: number, rowCount: number) => {
    if (rowCount <= 0) return -1
    if (activeIndex < 0) return key === 'ArrowDown' ? 0 : rowCount - 1
    return key === 'ArrowDown' ? Math.min(activeIndex + 1, rowCount - 1) : Math.max(activeIndex - 1, 0)
}

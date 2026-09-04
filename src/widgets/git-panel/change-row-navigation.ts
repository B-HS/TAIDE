import type { GitSectionId } from '@entities/git/git-section-collapse-memory'

/** The resource groups whose headers and rows make up the panel's one virtualized change list. */
export type GitChangeSectionId = Extract<GitSectionId, 'merge' | 'staged' | 'changes'>

export type GitChangeListSection = {
    id: GitChangeSectionId
    visible: boolean
    collapsed: boolean
    rowKeys: readonly string[]
}

export type GitChangeListRow =
    { kind: 'header'; id: string; section: GitChangeSectionId } | { kind: 'row'; id: string; section: GitChangeSectionId; rowIndex: number }

/**
 * Flattens the three resource groups into one linear row list — the shape a virtualizer needs, since
 * it addresses items by a single index and cannot descend into per-group children. It is the same
 * move `search-result-rows.ts` and `problem-list-rows.ts` already make for their panels; the SCM
 * panel was the last list that still poured every row into the DOM at once, which on a branch switch
 * or a generated-file commit means thousands of rows plus one Radix context menu per row.
 *
 * An invisible section contributes nothing at all (an empty group is not drawn — `git-sections.ts`),
 * and a collapsed one keeps its header and drops its rows, so collapsing stays exactly the operation
 * it was. Row ids carry the group because the same path legitimately appears in both "Staged
 * Changes" and "Changes" when a file has staged and unstaged hunks.
 */
export const buildGitChangeListRows = (sections: readonly GitChangeListSection[]): GitChangeListRow[] => {
    const rows: GitChangeListRow[] = []

    for (const section of sections) {
        if (!section.visible) continue
        rows.push({ kind: 'header', id: `header:${section.id}`, section: section.id })
        if (section.collapsed) continue

        section.rowKeys.forEach((key, rowIndex) => rows.push({ kind: 'row', id: `row:${section.id}:${key}`, section: section.id, rowIndex }))
    }

    return rows
}

/** Indexes of the section headers inside a {@link buildGitChangeListRows} result, ascending. */
export const gitChangeListHeaderIndexes = (rows: readonly GitChangeListRow[]) => rows.flatMap((row, index) => (row.kind === 'header' ? [index] : []))

/**
 * Which header has to render as the sticky one for a window that starts at `firstVisibleIndex`: the
 * last header at or above it, or `-1` when the list starts with rows only (it cannot — a group's
 * rows always follow its header — but an empty list has no header either).
 *
 * A virtualized row is absolutely positioned, so `position: sticky` on it can never leave its own
 * 24px box; the top-most header therefore has to be pulled into the rendered range even when it has
 * scrolled out of it, and rendered in flow instead of translated. This is the same function the
 * virtualizer's `rangeExtractor` uses, so what gets mounted and what gets drawn sticky agree by
 * construction.
 */
export const resolveStickyHeaderIndex = (headerIndexes: readonly number[], firstVisibleIndex: number) =>
    headerIndexes.filter((index) => index <= firstVisibleIndex).at(-1) ?? -1

/**
 * Marks one stop in the SCM panel's roving focus order and, at the same time, says where that stop
 * sits in the panel's item sequence. A `querySelectorAll` of the mounted rows cannot answer that any
 * more: only the rows inside the virtual window are in the DOM, so document order stopped being the
 * item order the moment the change list was virtualized (research 3a L1).
 */
export const GIT_ROVING_INDEX_ATTRIBUTE = 'data-git-roving-index'

export const GIT_ROVING_ITEM_SELECTOR = `[${GIT_ROVING_INDEX_ATTRIBUTE}]`

export const gitRovingItemSelector = (index: number) => `[${GIT_ROVING_INDEX_ATTRIBUTE}="${index}"]`

/** Reads {@link GIT_ROVING_INDEX_ATTRIBUTE} back, answering `-1` for anything that is not one. */
export const parseGitRovingIndex = (value: string | null | undefined) => {
    if (!value) return -1
    const index = Number(value)
    return Number.isInteger(index) && index >= 0 ? index : -1
}

/**
 * Resolves where ArrowDown/ArrowUp should move the roving focus inside the SCM panel.
 * `activeIndex` is the header or row currently owning focus (`-1` when focus sits outside every
 * roving item — e.g. the scroll container itself): ArrowDown then enters at the top and ArrowUp at
 * the bottom, mirroring the file tree's behavior. Movement clamps at both ends instead of wrapping.
 * Returns `-1` when there is nothing to focus so the caller can leave the event untouched.
 *
 * `rowCount` counts every roving item, not just the change rows: the virtualized change list first,
 * then the stash and graph section headers that follow it in the panel. Headers joined the sequence
 * when they became collapsible — leaving them out would have made their Stage All / Unstage All
 * actions reachable by mouse only, since those actions only reveal on hover or focus-within.
 */
export const resolveNextChangeRowIndex = (key: 'ArrowDown' | 'ArrowUp', activeIndex: number, rowCount: number) => {
    if (rowCount <= 0) return -1
    if (activeIndex < 0) return key === 'ArrowDown' ? 0 : rowCount - 1
    return key === 'ArrowDown' ? Math.min(activeIndex + 1, rowCount - 1) : Math.max(activeIndex - 1, 0)
}

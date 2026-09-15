/**
 * The collapsible sections of the SCM panel, in render order: the three resource groups, the stash
 * list, then the commit graph.
 */
export const GIT_SECTION_IDS = ['merge', 'staged', 'changes', 'stashes', 'graph'] as const

export type GitSectionId = (typeof GIT_SECTION_IDS)[number]

/**
 * Which sections are collapsed before the user has ever touched one. Only `stashes` is: a stash is
 * a side channel the user leaves and comes back to, while the resource groups and the graph are
 * what the panel exists to show. Annotating the record (instead of inferring it) is what forces a
 * newly added {@link GitSectionId} to declare its default here rather than silently defaulting to
 * expanded.
 */
export const GIT_SECTION_DEFAULT_COLLAPSED: Record<GitSectionId, boolean> = {
    merge: false,
    staged: false,
    changes: false,
    stashes: true,
    graph: false,
}

/**
 * Reads `Settings.gitSectionsCollapsed` into the flag-per-section shape the panel renders from.
 *
 * The stored value is a list of collapsed ids that Rust keeps as an opaque passthrough — this union
 * owns the value set, so an id this build does not know is simply ignored here rather than rejected
 * there (`docs/ipc-contract.md`, d-58 §1.H). An absent list — settings not loaded yet, or a
 * `settings.json` written before the field existed — falls back to
 * {@link GIT_SECTION_DEFAULT_COLLAPSED} rather than to "nothing is collapsed": the two disagree
 * about `stashes`, and the fallback is what the panel shows on a cold start.
 */
export const toGitSectionCollapsedMap = (collapsedIds: readonly string[] | null | undefined) => {
    if (!collapsedIds) return GIT_SECTION_DEFAULT_COLLAPSED

    const isCollapsed = (id: GitSectionId) => collapsedIds.includes(id)

    return {
        merge: isCollapsed('merge'),
        staged: isCollapsed('staged'),
        changes: isCollapsed('changes'),
        stashes: isCollapsed('stashes'),
        graph: isCollapsed('graph'),
    } satisfies Record<GitSectionId, boolean>
}

/** The inverse, in {@link GIT_SECTION_IDS} order so the same collapse state always writes the same list. */
export const toGitSectionCollapsedIds = (collapsed: Record<GitSectionId, boolean>) => GIT_SECTION_IDS.filter((id) => collapsed[id])

/**
 * The collapsible sections of the SCM panel, in render order: the three resource groups, the stash
 * list, then the commit graph.
 */
export type GitSectionId = 'merge' | 'staged' | 'changes' | 'stashes' | 'graph'

/**
 * Which sections start collapsed on a cold start. Only `stashes` does: a stash is a side channel
 * the user leaves and comes back to, while the resource groups and the graph are what the panel
 * exists to show. Annotating the record (instead of inferring it) is what forces a newly added
 * {@link GitSectionId} to declare its default here rather than silently defaulting to expanded.
 */
export const GIT_SECTION_DEFAULT_COLLAPSED: Record<GitSectionId, boolean> = {
    merge: false,
    staged: false,
    changes: false,
    stashes: true,
    graph: false,
}

/**
 * Which SCM sections the user has collapsed.
 *
 * Ownership scope (`docs/architecture.md` §6.4): the process — one record for the whole window, not
 * per project. Collapsing "Staged Changes" is a view preference about the panel's shape, not a fact
 * about a repository, so it deliberately follows the user across projects the way a sidebar width
 * would. Lifetime: the app run. There is nothing to release — the key set is a closed union of five
 * entries, so the map cannot grow, and no TTL or cap is needed to bound it.
 *
 * It lives here rather than in `GitPanel`'s component state for the same reason
 * `commit-message-memory.ts` does: the git panel only renders while the sidebar's git view is
 * selected, so switching to the files or search view unmounts `GitPanelContainer` and would throw
 * every collapse away. Persisting it across restarts would mean a `Settings` field (Rust type +
 * patch + bindings) or a `ProjectLayout` migration; a preference this cheap to redo is not worth
 * either, so a restart intentionally returns to {@link GIT_SECTION_DEFAULT_COLLAPSED}.
 */
const collapsedBySectionId = new Map<GitSectionId, boolean>()

const readGitSectionCollapsed = (id: GitSectionId) => collapsedBySectionId.get(id) ?? GIT_SECTION_DEFAULT_COLLAPSED[id]

/**
 * Snapshot of every section's collapse flag, shaped for `useState`'s initializer so a remount of
 * the panel restores what the user last collapsed.
 */
export const readGitSectionCollapseState = () => ({
    merge: readGitSectionCollapsed('merge'),
    staged: readGitSectionCollapsed('staged'),
    changes: readGitSectionCollapsed('changes'),
    stashes: readGitSectionCollapsed('stashes'),
    graph: readGitSectionCollapsed('graph'),
})

export const writeGitSectionCollapsed = (id: GitSectionId, collapsed: boolean) => {
    collapsedBySectionId.set(id, collapsed)
}

/**
 * Test-only: forgets every recorded collapse so the next read falls back to
 * {@link GIT_SECTION_DEFAULT_COLLAPSED}. `collapsedBySectionId` is a process-wide singleton and
 * `bun test` does not isolate modules per test file, so a section collapsed by one test would
 * otherwise leak into every test that runs after it.
 */
export const resetGitSectionCollapseMemoryForTests = () => {
    collapsedBySectionId.clear()
}

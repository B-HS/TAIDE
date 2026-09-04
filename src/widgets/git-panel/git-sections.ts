import type { StatusRow } from '@shared/api/bindings'
import type { GitSectionId } from '@entities/git/git-section-collapse-memory'
import { isStagedRow, isUnstagedRow } from '@widgets/git-panel/commit-gate'

export type GitSectionState = {
    count: number
    visible: boolean
    collapsed: boolean
}

type BuildGitSectionsInput = {
    rows: readonly StatusRow[]
    stashCount: number
    graphCount: number
    collapsed: Record<GitSectionId, boolean>
}

/**
 * Everything the SCM panel needs to decide *what* to draw, kept out of the component so the one
 * part of this UX that is testable without a DOM harness actually is.
 *
 * Two rules it encodes. An empty section is not drawn at all — the previous panel rendered the
 * stash section whenever the working tree was dirty, so a repository with zero stashes still showed
 * an empty "Stash" header above the changes, which is what made the two areas read as one blurred
 * list. And a section that is collapsed still reports its `count`, because the header badge has to
 * keep showing it: a user who collapsed "Staged Changes" and then reads "0 staged" would reach for
 * the stage-all confirmation path and commit the whole working tree.
 *
 * `showNoChanges` is deliberately derived from the three resource groups rather than from
 * `rows.length`, so a row that belongs to none of them can never make the panel claim the
 * repository is clean.
 */
export const buildGitSections = ({ rows, stashCount, graphCount, collapsed }: BuildGitSectionsInput) => {
    const mergeRows = rows.filter((row) => row.isConflicted)
    const stagedRows = rows.filter(isStagedRow)
    const unstagedRows = rows.filter(isUnstagedRow)

    const sectionOf = (id: GitSectionId, count: number): GitSectionState => ({ count, visible: count > 0, collapsed: collapsed[id] })

    return {
        mergeRows,
        stagedRows,
        unstagedRows,
        sections: {
            merge: sectionOf('merge', mergeRows.length),
            staged: sectionOf('staged', stagedRows.length),
            changes: sectionOf('changes', unstagedRows.length),
            stashes: sectionOf('stashes', stashCount),
            graph: sectionOf('graph', graphCount),
        },
        showNoChanges: mergeRows.length === 0 && stagedRows.length === 0 && unstagedRows.length === 0,
    }
}

import type { StatusRow } from '@shared/api/bindings'
import type { GitStatusChangeKind } from '@features/git/status-row-item'

export type CommitGate = 'blockedByConflicts' | 'confirmStageAll' | 'commit'

/**
 * A row the "Staged Changes" group owns. Conflicted rows are excluded on purpose — an unmerged
 * path's index holds the three conflict stages, not a stageable change — which is exactly why
 * {@link resolveCommitGate} has to look at `isConflicted` separately instead of inferring "nothing
 * to commit" from these two predicates alone.
 */
export const isStagedRow = (row: StatusRow): row is StatusRow & { staged: GitStatusChangeKind } => !row.isConflicted && row.staged !== null

export const isUnstagedRow = (row: StatusRow): row is StatusRow & { unstaged: GitStatusChangeKind } => !row.isConflicted && row.unstaged !== null

/**
 * What a commit click must do for a given status listing.
 *
 * `blockedByConflicts` exists because both predicates above drop conflicted rows: with a merge in
 * progress and nothing else staged, the panel saw "0 staged, 0 unstaged", skipped the stage-all
 * confirmation, and asked for a commit — and the commit path stages everything when nothing is
 * staged (`git_commit`'s `stage_all`), so the raw `<<<<<<<`/`=======`/`>>>>>>>` markers were added
 * and committed as the merge resolution (audit §4-B A4). git itself refuses to commit with
 * unmerged paths, so blocking here matches what the underlying tool would do anyway — resolve the
 * conflict (which clears `isConflicted` and moves the file into the staged group) first.
 */
export const resolveCommitGate = (rows: readonly StatusRow[]): CommitGate => {
    if (rows.some((row) => row.isConflicted)) return 'blockedByConflicts'
    if (!rows.some(isStagedRow) && rows.some(isUnstagedRow)) return 'confirmStageAll'
    return 'commit'
}

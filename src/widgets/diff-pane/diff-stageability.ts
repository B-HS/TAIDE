import type { StatusRow } from '@shared/api/bindings'

/**
 * Whether the diff tab at `path` may offer gutter hunk stage/unstage.
 *
 * Two conditions have to hold. A manual file-vs-file compare (`compareWith`) has no git-stage
 * concept at all, and an unresolved conflict's raw marker text has no meaningful hunk-level stage
 * action (the inline conflict decorator in `editor-pane.tsx` serves that flow instead).
 *
 * The conflict lookup accepts either of a `StatusRow`'s two path representations. Every producer of
 * a `TabKind::Diff` now writes the absolute one, but this check previously compared only against
 * the repo-relative `StatusRow.path`, so it recognized a conflict *only* for tabs opened from the
 * git panel (the single producer that passed a relative path) and silently missed it for every
 * other one — the tab bar's "Open Changes" in particular offered stage gutters over conflict-marker
 * text (audit §4-B B10). Keeping the relative arm covers layouts persisted before the producers
 * were unified, whose restored diff tabs still carry a repo-relative path.
 */
export const isDiffHunkStageable = (input: { path: string; compareWith: string | null; rows: readonly StatusRow[] }) =>
    input.compareWith === null && !input.rows.some((row) => (row.absPath === input.path || row.path === input.path) && row.isConflicted)

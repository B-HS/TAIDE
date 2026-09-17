import type { OpenTabInSplitRequest, ProjectId, ProjectLayout } from '@shared/api/bindings'
import type { FileTreeRow } from '@features/explorer/file-tree-row'
import { currentWindowFocusedPane } from '@shared/lib/pane-tree'

/**
 * "Open to the Side" as a single `layout_open_tab_in_split` request — the file is opened *into a new
 * pane* instead of being opened in the focused pane first and dragged out of it afterwards.
 *
 * The two-step form (`layout_open_tab` then `layout_split`) collapsed back to one pane whenever the
 * focused group ended up holding only the tab being moved: `split` extracts the tab, leaves an empty
 * leaf behind, and `normalize` then folds a split with one surviving child back into that child
 * (d-66 #19). An empty source pane never exists here, so there is nothing for `normalize` to fold,
 * and the window sees one `layout:changed` instead of two.
 *
 * `open_tab_in_split` deliberately performs no kind dedupe, so a file that is already open is opened
 * *again* in the new pane rather than merely activating the existing tab — "to the side" is a
 * request for a split, and that is the reading this returns.
 *
 * `targetPane` is this window's focused pane ({@link currentWindowFocusedPane}) rather than
 * `ProjectLayout.focusedPane`, which always names the main tree's pane: the explorer is mounted in
 * auxiliary windows too (d-62 §1.D), and splitting the main window from one of them is not what the
 * menu says. A layout that has not loaded (or an auxiliary window whose slot is already gone) has no
 * pane to split and yields `null`.
 */
export const planOpenToTheSide = (projectId: ProjectId, row: FileTreeRow, layout: ProjectLayout | null | undefined): OpenTabInSplitRequest | null => {
    const targetPane = currentWindowFocusedPane(layout)
    if (row.kind !== 'file' || !targetPane) return null
    return { projectId, targetPane, edge: 'right', kind: { kind: 'file', path: row.path }, title: row.name, preview: false }
}

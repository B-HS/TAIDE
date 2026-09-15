import type { PaneId, TabId } from '@shared/api/bindings'
import type { PaneFocusTarget } from '@shared/lib/bridge/editor-pane-command-bridge'
import type { WindowPaneTree } from '@shared/lib/pane-tree'
import { findAdjacentPaneLeaf, findPaneLeaf, paneLeafAtPosition } from '@shared/lib/pane-tree'

/**
 * Which pane ⌘K ⌘←/→/↑/↓ or ⌘1..⌘9 should focus, or `null` for the cases that must stay a no-op:
 * no pane tree yet, no neighbour at that edge, fewer groups than the pressed number, or a target
 * that resolves to the group already focused. Pure so `EditorArea`'s handler is one `focusPane`
 * call and every one of those no-op branches is unit-testable without mounting the widget.
 */
export const resolveFocusGroupPaneId = (paneTree: WindowPaneTree | null, target: PaneFocusTarget): PaneId | null => {
    if (!paneTree) return null
    const leaf =
        target.kind === 'direction'
            ? findAdjacentPaneLeaf(paneTree.root, paneTree.focusedPane, target.direction)
            : paneLeafAtPosition(paneTree.root, target.position)
    if (!leaf || leaf.id === paneTree.focusedPane) return null
    return leaf.id
}

/**
 * The tabs ⌘K ⌘W closes, in strip order. Pinned tabs survive, matching the tab bar's own
 * "Close All" (`pane-tab-bar.tsx`) and ⌘W's single-tab rule. The list is the snapshot taken before
 * the first close — each close returns a fresh layout, so the caller cannot re-read the tree
 * between iterations and must serialize over these ids.
 */
export const collectClosableTabIdsInFocusedGroup = (paneTree: WindowPaneTree | null): TabId[] => {
    if (!paneTree) return []
    const leaf = findPaneLeaf(paneTree.root, paneTree.focusedPane)
    if (!leaf) return []
    return leaf.tabs.filter((tab) => !tab.pinned).map((tab) => tab.id)
}

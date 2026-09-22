import type { PaneId, PaneNode, ProjectLayout, SplitDir, Tab, TabId } from '@shared/api/bindings'
import { getWindowContext, type WindowContext } from '@shared/lib/window-context'

export type PaneDirection = 'left' | 'right' | 'up' | 'down'

/** A `horizontal` split lays its children out left-to-right, a `vertical` one top-to-bottom (Rust's `split_dir_of_edge`), so only a split on the matching axis can hold a neighbour in a given direction. */
const SPLIT_DIR_BY_PANE_DIRECTION: Record<PaneDirection, SplitDir> = { left: 'horizontal', right: 'horizontal', up: 'vertical', down: 'vertical' }

/** Which way to step through a split's `children` to reach the neighbour subtree — earlier for left/up, later for right/down. */
const SIBLING_STEP_BY_PANE_DIRECTION: Record<PaneDirection, number> = { left: -1, right: 1, up: -1, down: 1 }

/** Which end of the neighbour subtree actually touches the pane we came from: moving right lands on that subtree's first leaf, moving left on its last. */
const ENTRY_LEAF_BY_PANE_DIRECTION: Record<PaneDirection, 'first' | 'last'> = { left: 'last', right: 'first', up: 'last', down: 'first' }

export const findPaneLeaf = (node: PaneNode, paneId: PaneId): Extract<PaneNode, { node: 'leaf' }> | null => {
    if (node.node === 'leaf') return node.id === paneId ? node : null
    for (const child of node.children) {
        const found = findPaneLeaf(child, paneId)
        if (found) return found
    }
    return null
}

export const findPaneTab = (node: PaneNode, tabId: TabId): Tab | null => {
    if (node.node === 'leaf') return node.tabs.find((tab) => tab.id === tabId) ?? null
    for (const child of node.children) {
        const found = findPaneTab(child, tabId)
        if (found) return found
    }
    return null
}

export const findActiveTab = (node: PaneNode, paneId: PaneId): Tab | null => {
    const leaf = findPaneLeaf(node, paneId)
    if (!leaf?.active) return null
    return leaf.tabs.find((tab) => tab.id === leaf.active) ?? null
}

export const collectPaneTabs = (node: PaneNode): Tab[] => (node.node === 'leaf' ? node.tabs : node.children.flatMap(collectPaneTabs))

/**
 * Every leaf pane (editor group) in depth-first order — left-to-right inside a `horizontal` split,
 * top-to-bottom inside a `vertical` one, because `PaneNode.children` is already stored in visual
 * order. This is the order the ⌘1..⌘9 group-focus shortcuts count in.
 */
export const collectPaneLeaves = (node: PaneNode): Extract<PaneNode, { node: 'leaf' }>[] =>
    node.node === 'leaf' ? [node] : node.children.flatMap(collectPaneLeaves)

/** The group at `position` in {@link collectPaneLeaves} order — 1-based, so it reads the same as the ⌘1..⌘9 label the user presses. `null` when the tree has fewer groups than that. */
export const paneLeafAtPosition = (root: PaneNode, position: number) => collectPaneLeaves(root)[position - 1] ?? null

/** Root-to-leaf chain ending at `paneId` (the leaf itself last), or `null` when the tree has no such pane. Node identity is preserved so callers can locate a child inside its own parent's `children`. */
const findPanePath = (node: PaneNode, paneId: PaneId): PaneNode[] | null => {
    if (node.node === 'leaf') return node.id === paneId ? [node] : null
    for (const child of node.children) {
        const path = findPanePath(child, paneId)
        if (path) return [node, ...path]
    }
    return null
}

const siblingSubtreeOf = (ancestor: PaneNode, child: PaneNode, direction: PaneDirection) => {
    if (ancestor.node !== 'split' || ancestor.dir !== SPLIT_DIR_BY_PANE_DIRECTION[direction]) return null
    return ancestor.children[ancestor.children.indexOf(child) + SIBLING_STEP_BY_PANE_DIRECTION[direction]] ?? null
}

/**
 * The group immediately `direction` of `paneId`, or `null` when there is none (the caller then does
 * nothing rather than wrapping around — VS Code's own behaviour for ⌘K ⌘←/→/↑/↓ at an edge).
 *
 * Walks the ancestor chain outward and takes the *nearest* split on the requested axis that still
 * has a sibling subtree on that side, then descends into whichever end of that subtree faces back
 * at us ({@link ENTRY_LEAF_BY_PANE_DIRECTION}). Deliberately no geometric/diagonal reasoning: pane
 * sizes live in `sizes` as ratios, not pixels, so there is no true "the pane physically above this
 * one" to compute here — the tree's own nesting is the only ordering both this and the rendered
 * layout agree on.
 */
export const findAdjacentPaneLeaf = (root: PaneNode, paneId: PaneId, direction: PaneDirection) => {
    const path = findPanePath(root, paneId)
    if (!path) return null

    const sibling = path
        .slice(0, -1)
        .map((ancestor, index) => siblingSubtreeOf(ancestor, path[index + 1], direction))
        .findLast((node) => node !== null)
    if (!sibling) return null

    const leaves = collectPaneLeaves(sibling)
    return (ENTRY_LEAF_BY_PANE_DIRECTION[direction] === 'first' ? leaves.at(0) : leaves.at(-1)) ?? null
}

export type WindowPaneTree = { root: PaneNode; focusedPane: PaneId }

/**
 * The absolute path of the focused pane's active tab when that tab is a file, `null` for every
 * other tab kind (terminal, settings, diff, …) and for a tree that hasn't loaded yet. Takes the
 * tree — not a `ProjectLayout` — so each caller keeps deciding *which* tree it means: a widget that
 * only ever renders in the main window passes the `ProjectLayout` itself (structurally a
 * `WindowPaneTree`), while one that can render inside an auxiliary window passes its own
 * `resolveWindowPaneTree` result, the same split those call sites already had inline.
 */
export const activeFilePathOf = (tree: WindowPaneTree | null | undefined): string | null => {
    const activeTab = tree ? findActiveTab(tree.root, tree.focusedPane) : null
    return activeTab?.kind.kind === 'file' ? activeTab.kind.path : null
}

export const fileTabPaneIdOf = (tree: WindowPaneTree | null | undefined, path: string): PaneId | null => {
    if (!tree) return null
    const focusedPane = findPaneLeaf(tree.root, tree.focusedPane)
    if (focusedPane?.tabs.some((tab) => tab.kind.kind === 'file' && tab.kind.path === path)) return focusedPane.id
    return collectPaneLeaves(tree.root).find((pane) => pane.tabs.some((tab) => tab.kind.kind === 'file' && tab.kind.path === path))?.id ?? null
}

/**
 * A window's tree with its `focusedPane` narrowed to a pane that actually exists in `root`, falling
 * back to the first leaf ({@link collectPaneLeaves} order) — the frontend mirror of Rust's
 * `ensure_focused_pane_valid` (d-65 contract §1.R1). Rust seals the invariant on its own side, but
 * every window renders from whatever `ProjectLayout` snapshot its cache currently holds, so the
 * window between a mutation pruning the focused pane (⌘W on a split half's last tab, a deleted
 * file's tabs closing, another OS window's mutation) and this window applying the `layout:changed`
 * echo would otherwise hand consumers a dangling id: `withCurrentWindowTarget` would promote it to
 * an explicit `target` and get `pane_not_found` back, and `editor-area.tsx`'s keymap handlers all
 * bail silently when `findPaneLeaf` returns `null`. A tree with no leaves at all (a split emptied
 * of children) keeps the original id — there is nothing truthful to point at instead.
 */
const withExistingFocusedPane = (root: PaneNode, focusedPane: PaneId) => {
    if (findPaneLeaf(root, focusedPane)) return { root, focusedPane }
    return { root, focusedPane: collectPaneLeaves(root).at(0)?.id ?? focusedPane }
}

/**
 * Resolves which of a `ProjectLayout`'s pane trees the calling window owns — the main tree for the
 * main window, or the matching `AuxWindowLayout` entry for an auxiliary window's own `windowSlot`
 * (Wave I contract §3.1/§3.2 — every pane/tab mutation on the Rust side locates the same tree by
 * `PaneTreeRef` before touching it, this is that lookup's frontend mirror). Returns `null` for an
 * auxiliary window whose slot isn't (yet, or no longer) present in the layout — a brief boot race,
 * or the window's own slot having just been cleaned up by `layout_move_tab_to_window`'s
 * `cleanup_emptied_auxiliary_windows` — callers should render an empty state rather than falling
 * back to the main tree, which would silently show/mutate the wrong window's tabs.
 *
 * Whichever tree it lands on, the returned `focusedPane` is validated against that tree by
 * {@link withExistingFocusedPane}, so no consumer of this function — every call site reading
 * `focusedPane`, {@link currentWindowFocusedPane} included — ever sees a pane id the tree lost.
 */
export const resolveWindowPaneTree = (layout: ProjectLayout, windowContext: WindowContext): WindowPaneTree | null => {
    if (windowContext.kind === 'main') return withExistingFocusedPane(layout.root, layout.focusedPane)
    const auxiliaryWindow = (layout.auxiliaryWindows ?? []).find((window) => window.slot === windowContext.windowSlot)
    return auxiliaryWindow ? withExistingFocusedPane(auxiliaryWindow.root, auxiliaryWindow.focusedPane) : null
}

/**
 * A pane tree with no tabs anywhere in it — mirrors the Rust-side `is_layout_tree_empty` check
 * `layout::service` uses to decide an auxiliary window's entry is pointless, so the frontend can
 * recognize the same condition to close its own now-empty OS window (contract §3.2's "마지막 탭
 * 이동/닫기 시 창 정리" — the close half, which unlike the move-to-window path Rust leaves to the
 * frontend; see `auxiliary-window-shell.tsx`).
 */
export const isPaneTreeEmpty = (root: PaneNode): boolean => root.node === 'leaf' && root.tabs.length === 0

/**
 * Every open tab across every tree in the project — the main tree plus every auxiliary window's
 * own tree. Hot-exit mirror GC (`editor-area.tsx`'s prune sweep) must keep a file's/untitled tab's
 * mirror alive if it's open in *any* window, not just the window running the sweep — both the main
 * and every auxiliary window mount their own `EditorArea` and independently run that sweep, so each
 * has to see the full cross-window picture or it would prune mirrors for tabs merely living in a
 * different OS window.
 */
export const collectAllPaneTabs = (layout: ProjectLayout): Tab[] => [
    ...collectPaneTabs(layout.root),
    ...(layout.auxiliaryWindows ?? []).flatMap((window) => collectPaneTabs(window.root)),
]

/**
 * The focused pane of *this* OS window — `layout_open_tab`/`layout_open_untitled` fall back to
 * `layout.focusedPane` (the main tree's) whenever a caller passes `target: null`, so a widget that
 * can render inside an auxiliary window (settings, breadcrumbs, search results, "Open Changes") must
 * resolve its own window's focused pane explicitly and pass it as `target` — passing `null` from
 * inside an auxiliary window would silently open the new tab in the *main* window instead, however
 * the pane a call site derives its own tab list from.
 */
export const currentWindowFocusedPane = (layout: ProjectLayout | null | undefined): PaneId | null =>
    layout ? (resolveWindowPaneTree(layout, getWindowContext())?.focusedPane ?? null) : null

export const currentWindowFileTabPane = (layout: ProjectLayout | null | undefined, path: string): PaneId | null =>
    fileTabPaneIdOf(layout ? resolveWindowPaneTree(layout, getWindowContext()) : null, path)

/**
 * The active file path of *this* OS window — {@link activeFilePathOf} over whichever tree
 * {@link resolveWindowPaneTree} hands the calling realm, and the symmetric counterpart of
 * {@link currentWindowFocusedPane}. Every widget that can render inside an auxiliary window and
 * asks "which file is the user looking at" wants this, not `activeFilePathOf(layout)`: a raw
 * `ProjectLayout` is structurally a `WindowPaneTree`, so passing it type-checks and silently
 * answers with the *main* window's active file (audit #5/#6 — the palette's `@`/`:` modes and the
 * outline panel both did, and acted on a file that window was not showing). `null` for a layout
 * that has not loaded yet and for an auxiliary window whose slot is no longer in the layout, which
 * every consumer already treats as "no active file".
 */
export const currentWindowActiveFilePath = (layout: ProjectLayout | null | undefined): string | null =>
    activeFilePathOf(layout ? resolveWindowPaneTree(layout, getWindowContext()) : null)

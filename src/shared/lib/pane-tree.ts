import type { PaneId, PaneNode, ProjectLayout, Tab, TabId } from '@shared/api/bindings'
import { getWindowContext, type WindowContext } from '@shared/lib/window-context'

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

export type WindowPaneTree = { root: PaneNode; focusedPane: PaneId }

/**
 * Resolves which of a `ProjectLayout`'s pane trees the calling window owns — the main tree for the
 * main window, or the matching `AuxWindowLayout` entry for an auxiliary window's own `windowSlot`
 * (Wave I contract §3.1/§3.2 — every pane/tab mutation on the Rust side locates the same tree by
 * `PaneTreeRef` before touching it, this is that lookup's frontend mirror). Returns `null` for an
 * auxiliary window whose slot isn't (yet, or no longer) present in the layout — a brief boot race,
 * or the window's own slot having just been cleaned up by `layout_move_tab_to_window`'s
 * `cleanup_emptied_auxiliary_windows` — callers should render an empty state rather than falling
 * back to the main tree, which would silently show/mutate the wrong window's tabs.
 */
export const resolveWindowPaneTree = (layout: ProjectLayout, windowContext: WindowContext): WindowPaneTree | null => {
    if (windowContext.kind === 'main') return { root: layout.root, focusedPane: layout.focusedPane }
    const auxiliaryWindow = (layout.auxiliaryWindows ?? []).find((window) => window.slot === windowContext.windowSlot)
    return auxiliaryWindow ? { root: auxiliaryWindow.root, focusedPane: auxiliaryWindow.focusedPane } : null
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

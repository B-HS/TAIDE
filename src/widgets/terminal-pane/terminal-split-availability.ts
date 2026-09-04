import type { SplitEdge } from '@features/tab/tab-context-menu'
import { isWithinRoot } from '@shared/lib/path-root'

type PaneSplitMetrics = {
    /** Width of the pane's terminal area, in CSS pixels. */
    paneWidthPx: number
    /** Height of the pane's terminal area, in CSS pixels — the tab bar is already excluded. */
    paneHeightPx: number
    minPaneSizePx: number
    resizerThicknessPx: number
}

/**
 * Which of the four split directions the terminal's context menu may offer, given how much room
 * the pane has left.
 *
 * `layout_open_tab_in_split` itself never refuses a direction — the pane tree can always be split
 * — so the only real limit is the renderer's: `pane-node-view.tsx` gives every `Panel` a pixel
 * `minSize`, and react-resizable-panels converts those to percentages of the group and *normalizes*
 * them when they no longer fit rather than rejecting the layout. A split of a pane too small to
 * hold two minimum panes plus their separator therefore succeeds and silently squashes both halves,
 * which is why this pre-check exists at all.
 *
 * Left/right halve the pane's width and top/bottom its height, so the two axes are decided
 * independently. A pane that measures zero on an axis (the element is not laid out yet, or the
 * measurement failed) disables that axis: offering a split whose outcome is unknown is worse than
 * offering one direction fewer, and the user can always re-open the menu.
 */
export const resolveSplitAvailability = ({ paneWidthPx, paneHeightPx, minPaneSizePx, resizerThicknessPx }: PaneSplitMetrics) => {
    const requiredPx = minPaneSizePx * 2 + resizerThicknessPx
    const fitsHorizontally = paneWidthPx > 0 && paneWidthPx >= requiredPx
    const fitsVertically = paneHeightPx > 0 && paneHeightPx >= requiredPx

    return { left: fitsHorizontally, right: fitsHorizontally, top: fitsVertically, bottom: fitsVertically } satisfies Record<SplitEdge, boolean>
}

type SplitTerminalCwdInput = {
    /** OSC 7 cwd reported by the running shell, if it has reported one. */
    liveCwd: string | null
    /** cwd the session was spawned in, from the terminal session roster. */
    persistedCwd: string | null
    /** cwd recorded on the tab itself. */
    tabCwd: string | null
    projectRoot: string | null
}

/**
 * The cwd a terminal opened by "split" should inherit from the terminal it was split off.
 *
 * The live OSC 7 cwd is the one the user actually sees, so it wins — but `pty_default_options`
 * validates whatever it is handed against the project root (`root_guard::ensure_within_root`) and
 * fails the spawn outright when it points elsewhere, and a shell that has `cd`-ed to `/tmp` is
 * exactly that case. Rather than surface a `Forbidden` for an action that has an obvious sane
 * outcome, a cwd outside the root (or an unknown root, before the project query resolves) falls
 * back to `null`, which the backend reads as "the project root".
 */
export const resolveSplitTerminalCwd = ({ liveCwd, persistedCwd, tabCwd, projectRoot }: SplitTerminalCwdInput) => {
    const candidate = liveCwd ?? persistedCwd ?? tabCwd
    if (!candidate || !projectRoot) return null
    return isWithinRoot(candidate, projectRoot) ? candidate : null
}

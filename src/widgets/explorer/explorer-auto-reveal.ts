export type AutoRevealDecision = 'skip' | 'select-only' | 'reveal-then-select'

type AutoRevealInput = {
    enabled: boolean
    activePath: string | null
    visiblePaths: ReadonlySet<string>
    sidebarVisible: boolean
    explorerViewActive: boolean
    lastRevealedPath: string | null
}

/**
 * VS Code's `explorer.autoReveal` parity, expressed as a pure decision so the gating rules are
 * testable without a tree, a layout or an IPC round trip.
 *
 * Three gates come first and are all "the user cannot see the tree right now": the setting is off,
 * the sidebar is collapsed (or Zen mode hides it), or the sidebar is showing search/git/outline
 * instead of the file tree. Revealing under any of those spends a `tree_reveal` round trip — which
 * replaces the whole cached row page — on something nobody is looking at, and silently expands
 * folders the user finds already open the next time they look.
 *
 * `lastRevealedPath` suppresses the repeat: the decision is recomputed on every layout/rows change,
 * and without it the reveal's own row-page replacement would immediately ask for the same reveal
 * again. Note only an *acted-on* decision records a path, so a file that became active while the
 * sidebar was collapsed is still revealed when the sidebar comes back.
 *
 * `select-only` is the whole point of taking `visiblePaths`: a row already on screen needs nothing
 * from Rust, just the existing `selectPathRequest` scroll-and-select the explorer already performs.
 */
export const decideAutoReveal = ({
    enabled,
    activePath,
    visiblePaths,
    sidebarVisible,
    explorerViewActive,
    lastRevealedPath,
}: AutoRevealInput): AutoRevealDecision => {
    if (!enabled || !activePath || !sidebarVisible || !explorerViewActive) return 'skip'
    if (activePath === lastRevealedPath) return 'skip'
    return visiblePaths.has(activePath) ? 'select-only' : 'reveal-then-select'
}

import type { PaneNode, Tab } from '@shared/api/bindings'

type PaneLeaf = Extract<PaneNode, { node: 'leaf' }>

const activeTabOf = (leaf: PaneLeaf) => leaf.tabs.find((tab) => tab.id === leaf.active) ?? null

/**
 * Which terminal a task ("Run Task", "Run Selected Text in Terminal") writes into: the one the user
 * is looking at when that is a terminal, and only otherwise the pane's leftmost one.
 *
 * `leaf.tabs` is the tab strip in painted order, so the plain `find` this replaced always picked the
 * leftmost terminal — which, once a pane held two, meant the command was typed into whatever shell
 * happened to be first. If that shell has a foreground process (`npm run dev`, a REPL, a pager) the
 * text is swallowed as *its* stdin instead of being run, silently, while the view jumps away from
 * the terminal the user had chosen (`docs/features/tasks.md` §3).
 *
 * `null` when the pane holds no terminal at all — the caller opens one.
 */
export const resolveRunTargetTerminalTab = (leaf: PaneLeaf): Tab | null => {
    const activeTab = activeTabOf(leaf)
    if (activeTab?.kind.kind === 'terminal') return activeTab
    return leaf.tabs.find((tab) => tab.kind.kind === 'terminal') ?? null
}

/**
 * Where ⌃` goes back to when the active tab *is* a terminal: the first tab in the pane that is not
 * one. A fallback picked by position alone cycled between terminals instead of toggling the panel
 * away, so the key could never leave a pane holding two of them. `null` — the caller then does
 * nothing — is the honest answer for a pane made only of terminals; staying put beats bouncing.
 */
export const resolveTerminalToggleFallbackTab = (leaf: PaneLeaf): Tab | null =>
    leaf.tabs.find((tab) => tab.id !== leaf.active && tab.kind.kind !== 'terminal') ?? null

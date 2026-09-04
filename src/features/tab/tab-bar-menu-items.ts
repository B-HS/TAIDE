import { FilePlus, GitCompare, RotateCcw, Sparkles, Terminal } from 'lucide-react'

/**
 * Every tab bar menu entry that fires a single handler. `split` is deliberately not part of this
 * union: it opens a submenu of four edges instead of dispatching one action, so the menu component
 * renders it through its own branch and keeps a handler record that is exhaustive over the rest.
 */
export type TabBarMenuActionId = 'newFile' | 'newTerminal' | 'reopenClosed' | 'closeSaved' | 'closeAll' | 'openWelcome'

export type TabBarMenuItemId = TabBarMenuActionId | 'split'

/**
 * Consecutive items of the same group render with no separator between them; a group change draws
 * one. Expressing the grouping as data (rather than hard-coding separator positions in the menu
 * markup) keeps it correct after visibility filtering, which can drop a whole group's items.
 */
type TabBarMenuGroup = 'create' | 'reopen' | 'close' | 'split' | 'welcome'

type TabBarMenuIcon = typeof FilePlus

export type TabBarMenuItem = {
    id: TabBarMenuItemId
    group: TabBarMenuGroup
    labelKey: string
    icon: TabBarMenuIcon | null
}

/**
 * Which surface is asking for the items. `addMenu` is the tab bar's `+` dropdown, which offers the
 * two creation entries only — the destructive ones (close saved / close all) stay behind an
 * explicit right-click, as they were never on that button.
 */
export type TabBarMenuSurface = 'contextMenu' | 'addMenu'

type TabBarMenuConditions = {
    hasTabs: boolean
    hasActiveTab: boolean
    hasClosedTabs: boolean
}

export type TabBarMenuInput = { surface: TabBarMenuSurface } & Partial<TabBarMenuConditions>

const TAB_BAR_MENU_ITEMS: TabBarMenuItem[] = [
    { id: 'newFile', group: 'create', labelKey: 'tab.newUntitledFile', icon: FilePlus },
    { id: 'newTerminal', group: 'create', labelKey: 'tab.newTerminal', icon: Terminal },
    { id: 'reopenClosed', group: 'reopen', labelKey: 'keymap.reopenClosedTab', icon: RotateCcw },
    { id: 'closeSaved', group: 'close', labelKey: 'tab.closeSaved', icon: null },
    { id: 'closeAll', group: 'close', labelKey: 'tab.closeAll', icon: null },
    { id: 'split', group: 'split', labelKey: 'tab.split', icon: GitCompare },
    { id: 'openWelcome', group: 'welcome', labelKey: 'tab.openWelcome', icon: Sparkles },
]

/**
 * Hidden — not disabled — when the precondition fails, matching the tab context menu's own policy
 * (`tabs.md` §3.1). `split` is the one entry that would actually fail in Rust rather than no-op:
 * `layout_split` moves a named tab into the new leaf, so a pane with no active tab has nothing to
 * hand it and would answer `NotFound`.
 */
const isTabBarMenuItemVisible = (id: TabBarMenuItemId, { hasTabs, hasActiveTab, hasClosedTabs }: TabBarMenuConditions) => {
    if (id === 'reopenClosed') return hasClosedTabs
    if (id === 'closeSaved' || id === 'closeAll') return hasTabs
    if (id === 'split') return hasActiveTab
    return true
}

/**
 * The single source of truth for what the tab bar's empty-space context menu and its `+` dropdown
 * offer, so the two surfaces can never drift apart. Pure by design: the menu components are not
 * unit-testable in this repo (no testing-library), so the visibility rules live here where
 * `bun test` can reach them.
 */
export const buildTabBarMenuItems = ({ surface, hasTabs = false, hasActiveTab = false, hasClosedTabs = false }: TabBarMenuInput) => {
    if (surface === 'addMenu') return TAB_BAR_MENU_ITEMS.filter((item) => item.group === 'create')
    return TAB_BAR_MENU_ITEMS.filter((item) => isTabBarMenuItemVisible(item.id, { hasTabs, hasActiveTab, hasClosedTabs }))
}

import type { FC, PropsWithChildren } from 'react'
import { createContext, use } from 'react'
import type { ProjectId, ShellSlotId } from '@shared/api/bindings'
import type { ShellSlotLeaf } from '@shared/lib/shell-slot'

export type ShellSlotFocusValue = {
    focusedShellSlotId: ShellSlotId | null
    focusedProjectId: ProjectId | null
}

const NO_FOCUS: ShellSlotFocusValue = { focusedShellSlotId: null, focusedProjectId: null }

const ShellSlotFocusContext = createContext<ShellSlotFocusValue | null>(null)

const ShellSlotScopeContext = createContext<ShellSlotLeaf | null>(null)

/**
 * Window-wide "which shell slot has the user's attention" (contract §0.1 U-7). Mounted once at the
 * main window's app root by `app/providers/shell-slot-provider.tsx`, which owns the DOM capture that
 * keeps it current; this half is deliberately pure so `shared` carries no IPC.
 */
export const ShellSlotFocusProvider: FC<PropsWithChildren<ShellSlotFocusValue>> = ({ focusedShellSlotId, focusedProjectId, children }) => (
    <ShellSlotFocusContext value={{ focusedShellSlotId, focusedProjectId }}>{children}</ShellSlotFocusContext>
)

export const useShellSlotFocus = () => use(ShellSlotFocusContext) ?? NO_FOCUS

/** Names the slot one subtree belongs to. Rendered by `ShellSlotTreeView` around each leaf's `ProjectShell`, so everything below a leaf can ask whether it is the focused one. */
export const ShellSlotScope: FC<PropsWithChildren<ShellSlotLeaf>> = ({ slotId, projectId, children }) => (
    <ShellSlotScopeContext value={{ slotId, projectId }}>{children}</ShellSlotScopeContext>
)

export const useShellSlotScope = () => use(ShellSlotScopeContext)

/**
 * The gate every duplicated global listener is wrapped in (contract §0.1 S-4/U-1): a keymap handler
 * or a broadcast-bridge subscription inside an unfocused slot must not fire, or ⌘S and ⌘W would run
 * once per open slot.
 *
 * `true` outside any scope is the load-bearing default, not a convenience: auxiliary windows and
 * component tests mount the very same widgets with no slot tree above them, and those realms have
 * exactly one shell — so "no slot scope" means "nothing to compete with", and their behavior is
 * unchanged by this feature.
 */
export const useIsShellSlotFocused = () => {
    const scope = use(ShellSlotScopeContext)
    const { focusedShellSlotId } = useShellSlotFocus()
    return !scope || scope.slotId === focusedShellSlotId
}

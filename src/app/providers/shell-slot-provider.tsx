import type { FC, PropsWithChildren } from 'react'
import { useEffect, useEffectEvent, useRef, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { ShellSlotId } from '@shared/api/bindings'
import { activeProjectQueryOptions } from '@entities/project/project.query'
import { shellStateQueryOptions, useFocusShellSlot } from '@entities/session/session.query'
import { ShellSlotFocusProvider, useShellSlotFocus } from '@shared/lib/shell-slot-context'
import { projectOfShellSlot, resolveShellSlotFocus, resolveShellSlotIdFromEventTarget } from '@shared/lib/shell-slot'

type FocusOverride = { serverFocused: ShellSlotId | null; slotId: ShellSlotId }

/**
 * Owns which shell slot the main window treats as focused (contract §0.1 U-7). Rust persists the
 * value, but the *source* is this window's DOM: one capture-phase pointerdown/focusin listener
 * resolves the event target to the slot that contains it, which is the only signal that covers
 * clicking into an editor, tabbing between panes, and focus following a newly opened tab alike.
 *
 * A click that lands outside every slot — a Radix portal's dialog/menu content, the sidebar, the
 * status bar — resolves to `null` and deliberately changes nothing, so a palette or a context menu
 * keeps acting on the slot the user opened it from.
 *
 * `override` is the local answer that beats the server's until the server catches up, so focus moves
 * within the same frame rather than after an IPC round trip. It carries the server value it was
 * taken against, which is what makes it self-clearing: once `session_focus_shell_slot` lands (or
 * something else — a sidebar click, a slot close — moves focus server-side) the recorded value no
 * longer matches and the server's answer wins again, with no effect and no stale-override window.
 *
 * `overrideRef` is that same override read *synchronously*. One click produces two signals —
 * `pointerdown` and then the `focusin` the browser fires for the focus it moves — and both land
 * before React has committed the state update, so a guard reading rendered state would see the old
 * slot twice and send `session_focus_shell_slot` twice for one click. The ref is written on the way
 * out, so the second signal of a pair resolves to the slot the first one already claimed and stops
 * there. It is discarded by the same `serverFocused` comparison, so focus moved by anything other
 * than this listener still wins the next click.
 */
export const ShellSlotProvider: FC<PropsWithChildren> = ({ children }) => {
    const overrideRef = useRef<FocusOverride | null>(null)

    const [override, setOverride] = useState<FocusOverride | null>(null)

    const { data: shellState } = useQuery(shellStateQueryOptions())
    const { mutate: focusShellSlot } = useFocusShellSlot()

    const tree = shellState?.tree ?? null
    const serverFocused = shellState?.focused ?? null

    const focusedSlotOf = (pending: FocusOverride | null) =>
        resolveShellSlotFocus(tree, pending && pending.serverFocused === serverFocused ? pending.slotId : serverFocused)

    const focusedShellSlotId = focusedSlotOf(override)
    const focusedProjectId = projectOfShellSlot(tree, focusedShellSlotId)

    const handleFocusWithin = useEffectEvent((event: Event) => {
        const slotId = resolveShellSlotIdFromEventTarget(event.target)
        if (!slotId || slotId === focusedSlotOf(overrideRef.current)) return
        overrideRef.current = { serverFocused, slotId }
        setOverride(overrideRef.current)
        focusShellSlot(slotId)
    })

    useEffect(() => {
        const listener = (event: Event) => handleFocusWithin(event)
        document.addEventListener('pointerdown', listener, true)
        document.addEventListener('focusin', listener, true)
        return () => {
            document.removeEventListener('pointerdown', listener, true)
            document.removeEventListener('focusin', listener, true)
        }
    }, [])

    return (
        <ShellSlotFocusProvider focusedShellSlotId={focusedShellSlotId} focusedProjectId={focusedProjectId}>
            {children}
        </ShellSlotFocusProvider>
    )
}

/**
 * The project the main window's window-level chrome acts on. The slot tree is the answer once it has
 * loaded; `project_get_active` is the same value by definition (Rust redefined it as "the focused
 * slot's project" in d-62 2a) and stands in for the one paint before the tree arrives, so ⌘P at boot
 * still opens on a project instead of on nothing.
 */
export const useFocusedProjectId = () => {
    const { focusedProjectId } = useShellSlotFocus()
    const { data: activeProjectId = null } = useQuery(activeProjectQueryOptions())
    return focusedProjectId ?? activeProjectId
}

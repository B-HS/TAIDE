import { useEffect, useEffectEvent, useRef } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { ProjectId } from '@shared/api/bindings'
import { layoutQueryOptions, useSetShellView } from '@entities/layout/layout.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { setWindowFullscreen } from '@entities/window/window.ipc'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'
import { subscribeToggleZenMode } from '@shared/lib/zen-mode-bridge'

export type ZenModeState = {
    zen: boolean
    sidebarCollapsed: boolean
    hideStatusBar: boolean
}

/**
 * Owns every cross-cutting Zen-mode concern that isn't tied to a specific DOM widget (the shell's
 * imperative sidebar-panel collapse lives in `app-shell.tsx` instead, since it needs the panel
 * ref): the `layout_set_shell_view` mutation, the ⌘K Z chord, the palette-command bridge, the
 * Escape exit, and the opt-in OS-fullscreen side effect. `projectId === null` (no active project,
 * or a project whose layout hasn't loaded yet) degrades to "never in zen" rather than throwing —
 * every consumer already renders a project-less state (welcome screen) that has no shell chrome to
 * hide in the first place.
 */
export const useZenMode = (projectId: ProjectId | null): ZenModeState => {
    const { data: layout } = useQuery(layoutQueryOptions(projectId))
    const { data: settings } = useQuery(settingsQueryOptions())
    const { mutate: setShellView } = useSetShellView(projectId)

    const zen = layout?.shellView?.zen ?? false
    const sidebarCollapsed = layout?.shellView?.sidebarCollapsed ?? false
    const hideStatusBar = settings?.zenHideStatusBar ?? true
    const fullscreenOnZen = settings?.zenFullscreen ?? false
    const desiredFullscreen = zen && fullscreenOnZen
    const appliedFullscreenRef = useRef(desiredFullscreen)

    const toggleZen = () => {
        if (!projectId) return
        setShellView({ projectId, patch: { zen: !zen, sidebarCollapsed: null } })
    }

    useGlobalKeymap({ 'toggle-zen-mode': toggleZen })

    const handleToggleRequested = useEffectEvent(() => toggleZen())
    useEffect(() => subscribeToggleZenMode(handleToggleRequested), [])

    /**
     * Bubble-phase `window` listener, not `useGlobalKeymap`'s capture-phase chord engine —
     * deliberately. A capture-phase listener would run *before* Radix's own Escape-to-dismiss
     * handling (`DismissableLayer` also listens in capture phase, but on `document`, which fires
     * after `window` in capture order), so it would steal Escape out from under an open dialog/
     * palette instead of letting it close first. Bubble phase on `window` means this only runs
     * *after* every other Escape handler (monaco's own included) has had a chance to call
     * `preventDefault()` — `event.defaultPrevented` is the signal that something else already
     * claimed this keystroke, in which case Zen mode does not also exit. Only attached while `zen`
     * is actually true, so it costs nothing the rest of the time.
     */
    const handleEscape = useEffectEvent((event: KeyboardEvent) => {
        if (event.key !== 'Escape' || event.defaultPrevented || !projectId) return
        setShellView({ projectId, patch: { zen: false, sidebarCollapsed: null } })
    })

    useEffect(() => {
        if (!zen) return
        window.addEventListener('keydown', handleEscape)
        return () => window.removeEventListener('keydown', handleEscape)
    }, [zen])

    /**
     * `Settings::zen_fullscreen` is opt-in (contract §3.2) — fires `window_set_fullscreen` with
     * `zen && fullscreenOnZen` on every *transition* of either input, not just when `zen` itself
     * flips. That means toggling the setting *while already in Zen mode* takes effect immediately
     * (enters/exits fullscreen without needing to leave and re-enter Zen), and — just as
     * importantly — exiting Zen mode always evaluates to `false` regardless of what the setting is
     * at that moment, so a window fullscreened by a since-disabled setting still un-fullscreens on
     * exit instead of getting stuck.
     *
     * The `appliedFullscreenRef` comparison is load-bearing: this hook (and its query data) mounts
     * fresh on every project switch and at boot, so a naive `[projectId, zen, fullscreenOnZen]`
     * effect would fire `setWindowFullscreen(false)` on that very first render too — forcibly
     * exiting a window the user put into native fullscreen themselves, or one
     * `tauri-plugin-window-state` just restored to fullscreen from a previous session, even though
     * `zen` was never involved. Seeding the ref with the *current* render's `desiredFullscreen`
     * means the first render is always a no-op match; only a later render where the computed value
     * actually differs from what was last applied issues the Rust call.
     */
    useEffect(() => {
        if (!projectId) return
        if (appliedFullscreenRef.current === desiredFullscreen) return
        appliedFullscreenRef.current = desiredFullscreen
        void setWindowFullscreen(desiredFullscreen).catch(() => undefined)
    }, [projectId, desiredFullscreen])

    return { zen, sidebarCollapsed, hideStatusBar }
}

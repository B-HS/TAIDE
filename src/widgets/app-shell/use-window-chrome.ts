import { useEffect, useEffectEvent, useRef } from 'react'
import { useQuery } from '@tanstack/react-query'
import { emptyWindowChromePatch } from '@entities/session/session.ipc'
import { shellStateQueryOptions, useSetWindowChrome } from '@entities/session/session.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { setWindowFullscreen } from '@entities/window/window.ipc'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'
import { subscribeToggleZenMode } from '@shared/lib/bridge/zen-mode-bridge'
import { isImeCompositionKeydown } from '@shared/lib/ime-composition'

export type WindowChromeState = {
    zen: boolean
    sidebarRailCollapsed: boolean
    hideStatusBar: boolean
}

/**
 * Owns every cross-cutting window-chrome concern that isn't tied to a specific DOM widget (the
 * shell's imperative explorer-panel collapse lives in `project-shell.tsx` instead, since it needs the
 * panel ref): the `session_set_window_chrome` mutation, the ⌘K Z chord, the palette-command bridge,
 * the Escape exit, and the opt-in OS-fullscreen side effect.
 *
 * Zen and the sidebar icon rail read from `SessionState::window_chrome`, not from the per-project
 * `ProjectLayout::shell_view` they used to live in (contract §0.1 S-6): with several project shells
 * side by side in one window these are properties of the *window*, so keeping them per project made
 * them flip as the focused slot changed. The slot-local axes (the explorer panel's collapsed state)
 * stay on the per-project struct.
 */
export const useWindowChrome = (): WindowChromeState => {
    const { data: shellState } = useQuery(shellStateQueryOptions())
    const { data: settings } = useQuery(settingsQueryOptions())
    const { mutate: setWindowChrome } = useSetWindowChrome()

    const zen = shellState?.windowChrome.zen ?? false
    const sidebarRailCollapsed = shellState?.windowChrome.sidebarRailCollapsed ?? false
    const hideStatusBar = settings?.zenHideStatusBar ?? true
    const fullscreenOnZen = settings?.zenFullscreen ?? false
    const desiredFullscreen = zen && fullscreenOnZen
    const appliedFullscreenRef = useRef(desiredFullscreen)

    const toggleZen = () => setWindowChrome({ ...emptyWindowChromePatch(), zen: !zen })

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
     *
     * The IME guard is separate from `defaultPrevented`: the Escape that cancels an in-flight
     * composition (in any focused input — the commit box, a search field, monaco) is consumed by
     * the input method itself, which leaves no `preventDefault` behind for this listener to read,
     * so without it a Korean typo correction would drop the user out of Zen mode.
     */
    const handleEscape = useEffectEvent((event: KeyboardEvent) => {
        if (isImeCompositionKeydown(event) || event.key !== 'Escape' || event.defaultPrevented) return
        setWindowChrome({ ...emptyWindowChromePatch(), zen: false })
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
     * The `appliedFullscreenRef` comparison is load-bearing: this hook's query data is empty on the
     * first paint of every boot, so a naive `[zen, fullscreenOnZen]` effect would fire
     * `setWindowFullscreen(false)` on that very first render too — forcibly exiting a window the user
     * put into native fullscreen themselves, or one `tauri-plugin-window-state` just restored to
     * fullscreen from a previous session, even though `zen` was never involved. Seeding the ref with
     * the *current* render's `desiredFullscreen` means the first render is always a no-op match; only
     * a later render where the computed value actually differs from what was last applied issues the
     * Rust call.
     */
    useEffect(() => {
        if (appliedFullscreenRef.current === desiredFullscreen) return
        appliedFullscreenRef.current = desiredFullscreen
        void setWindowFullscreen(desiredFullscreen).catch(() => undefined)
    }, [desiredFullscreen])

    return { zen, sidebarRailCollapsed, hideStatusBar }
}

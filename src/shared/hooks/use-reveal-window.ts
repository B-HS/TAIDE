import { useEffect } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'

export const useRevealWindow = (ready: boolean) => {
    useEffect(() => {
        if (!ready) return
        void getCurrentWindow().show()
    }, [ready])
}

/**
 * Mirrors the CSS reveal gate at `html[data-theme-ready][data-locale-ready] body` (global.css) —
 * the OS-level window `show()` triggered by {@link useRevealWindow} must wait on the same
 * theme+locale signals as that visibility gate, or the window can appear while the body is still
 * hidden, flashing the bare background color. Both `isFetched` flags flip to `true` on a failed
 * fetch just as on a successful one, so gating reveal on them can never deadlock waiting for a
 * query that already settled into an error state.
 */
export const isWindowReadyToReveal = (isThemeFetched: boolean, isLocaleFetched: boolean) => isThemeFetched && isLocaleFetched

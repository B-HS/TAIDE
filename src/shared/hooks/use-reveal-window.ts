import { useEffect } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { PERF_MARK, PERF_MEASURE, perfMeasure } from '@shared/lib/perf-mark'

/**
 * Closes the boot span opened in `main.tsx` at the moment this window is actually shown — the
 * front-end half of metric 1 in `docs/quality-assurance/2026-09-04-perf-baseline.md`, paired with the
 * Rust `setup.*` spans that cover everything before the webview existed. The start mark is consumed
 * by the first measure, so the second window a session opens (an auxiliary editor window, which
 * runs this same hook) records nothing rather than reporting its own reveal as a boot time.
 */
export const useRevealWindow = (ready: boolean) => {
    useEffect(() => {
        if (!ready) return
        perfMeasure(PERF_MEASURE.BOOT_REVEAL, PERF_MARK.BOOT_MODULE_EVALUATED)
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

import '@shared/lib/error-log-forwarding-install'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { App } from '@app/app'
import { syncNativePerfGate } from '@entities/app/perf.ipc'
import { PERF_MARK, perfMark } from '@shared/lib/perf-mark'
import { installRemoteInternalsShim } from '@shared/lib/remote/tauri-internals-shim'
import { ErrorBoundary } from '@shared/ui/error-boundary'
import '@shared/styles/global.css'

installRemoteInternalsShim()

/**
 * Boot instrumentation's start point (metric 1 in `docs/quality-assurance/2026-09-04-perf-baseline.md`).
 * It sits after the shim — which must stay the first thing that runs in a remote mirror — and
 * before `createRoot`, so the span it opens covers React's own mount and everything the reveal gate
 * waits on, while its `performance.now()` stamp already carries the cost of evaluating this entry
 * point's whole import graph (measured against `performance.timeOrigin`).
 *
 * The gate query is fired here rather than inside a provider so it overlaps the first render
 * instead of queueing behind it; `perfMark` records its timestamp whether or not the answer has
 * arrived yet, which is what keeps this measurement honest in a release build started with
 * `TAIDE_PERF=1` (see `perf-mark.ts`).
 */
perfMark(PERF_MARK.BOOT_MODULE_EVALUATED)
void syncNativePerfGate()

const container = document.getElementById('root')

if (!container) throw new Error('root container not found')

const revealWindowOnRootCrash = () => {
    document.documentElement.dataset.themeReady = ''
    document.documentElement.dataset.localeReady = ''
    void getCurrentWindow().show()
}

createRoot(container).render(
    <StrictMode>
        <ErrorBoundary labelKey='errorBoundary.app' labelFallback='Application' onCaught={revealWindowOnRootCrash}>
            <App />
        </ErrorBoundary>
    </StrictMode>,
)

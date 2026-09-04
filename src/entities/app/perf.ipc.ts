import { commands } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'
import { applyNativePerfGate } from '@shared/lib/perf-mark'
import { isRemoteMirrorRuntime } from '@shared/lib/remote/runtime-environment'

/**
 * Adopts the process-wide `TAIDE_PERF` gate for this window's front-end instrumentation, so both
 * halves of a measurement session are on or off together — a dev build started with `TAIDE_PERF=0`
 * (comparing against the instrumentation's own overhead, `docs/debugging.md` §4.1) must not leave
 * the front end recording on its own.
 *
 * `perf_snapshot` is the reader for a number the front end has no other way to learn: the gate is
 * resolved once from the environment at process start, and no boot payload carries it into the
 * webview. Its `enabled` field is the whole point of this call; the counters it also returns are
 * ignored here.
 *
 * Two paths deliberately leave the build default in place instead of reporting a problem. The
 * remote mirror is refused by policy (`RemoteDenialPolicy::DesktopProcessDiagnostics` — the
 * registry belongs to the desktop user who turned it on), so it is skipped before the IPC rather
 * than after a guaranteed rejection. Any other failure is swallowed because instrumentation is a
 * diagnostic side channel: a toast or an `error` log for it would be noise in the very log
 * (`docs/debugging.md` §1) a measurement session is trying to read.
 */
export const syncNativePerfGate = async () => {
    if (isRemoteMirrorRuntime()) return
    try {
        const snapshot = await unwrapResult(commands.perfSnapshot())
        applyNativePerfGate(snapshot.enabled)
    } catch {
        return
    }
}

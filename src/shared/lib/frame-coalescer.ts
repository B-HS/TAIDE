export type FrameCoalescerScheduler = {
    requestFrame: (callback: () => void) => number
    cancelFrame: (frameId: number) => void
}

const browserFrameScheduler: FrameCoalescerScheduler = {
    requestFrame: (callback) => requestAnimationFrame(callback),
    cancelFrame: (frameId) => cancelAnimationFrame(frameId),
}

export type FrameCoalescer<Value> = {
    push: (value: Value) => void
    cancel: () => void
}

/**
 * Collapses a burst of same-frame `push` calls into a single `flush` per animation frame, keeping
 * only the most recently pushed value. A color-picker drag fires `pointermove` far faster than the
 * theme preview pipeline (full draft re-resolve + CSS variable re-apply + subscriber re-render) can
 * keep up with — applying every intermediate draft floods the main thread the same way the per-move
 * native window-appearance IPC did before it was gated (contract d-45 §0, §1#2). `cancel` drops any
 * value that has not been flushed yet and un-schedules the pending frame, so a caller that tore down
 * mid-drag (e.g. closing the theme editor) can never have a late frame apply a stale value after the
 * fact (contract d-45 §1 review question 2).
 *
 * Kept as a standalone `shared/lib` utility with a single consumer today (`theme.query.ts`) so its
 * frame-scheduling semantics — same-frame collapsing, cancel-before-schedule — are unit-testable on
 * their own rather than only reachable through the theme preview pipeline's much larger surface
 * (contract d-45 F-10). This is a readability/testability split (common.md §3.3), not a bet on future
 * reuse; promote its placement decision to `shared` in more than name only once a second consumer
 * actually shows up.
 */
export const createFrameCoalescer = <Value>(
    flush: (value: Value) => void,
    scheduler: FrameCoalescerScheduler = browserFrameScheduler,
): FrameCoalescer<Value> => {
    let frameId: number | null = null
    let pendingBox: { value: Value } | null = null

    const runFlush = () => {
        frameId = null
        if (!pendingBox) return
        const { value } = pendingBox
        pendingBox = null
        flush(value)
    }

    const push = (value: Value) => {
        pendingBox = { value }
        if (frameId !== null) return
        frameId = scheduler.requestFrame(runFlush)
    }

    const cancel = () => {
        if (frameId !== null) scheduler.cancelFrame(frameId)
        frameId = null
        pendingBox = null
    }

    return { push, cancel }
}

export type PaneResizeCommitScheduler<Handle = unknown> = {
    schedule(callback: () => void, delayMs: number): Handle
    cancel(timerId: Handle): void
}

const timeoutScheduler: PaneResizeCommitScheduler<ReturnType<typeof setTimeout>> = {
    schedule: (callback, delayMs) => setTimeout(callback, delayMs),
    cancel: (timerId) => clearTimeout(timerId),
}

/**
 * Long enough to swallow a held arrow key's auto-repeat (~30/s on macOS at the fastest key-repeat
 * setting) and short enough that a released key persists before the user can plausibly quit or
 * switch windows.
 */
export const PANE_RESIZE_COMMIT_DEBOUNCE_MS = 120

const pendingTimerByPane = new Map<string, unknown>()

/**
 * Trailing-only debounce of one pane's resize persistence, keyed by `paneKey` so sibling panes
 * never cancel each other.
 *
 * `react-resizable-panels` already withholds `onLayoutChanged` until a pointer drag ends, so a mouse
 * drag costs one commit either way. Keyboard resizing does not go through that gate: every arrow-key
 * press reports a completed layout change, and a *held* arrow key therefore fired one
 * `layout_resize_pane` per repeat — each taking the app-wide mutation guard and rewriting the layout
 * file, each answering with a full `ProjectLayout`, and each emitting a `layout:changed` event on top
 * (audit §1-5). Trailing-only (no leading edge) because the intermediate widths of a resize are
 * meaningless: only where the separator comes to rest is worth storing.
 *
 * The pending timer lives in module state rather than in a `PaneNodeView` ref so a commit still lands
 * when the component re-renders (a layout refetch replaces the node object on every keystroke) and so
 * the same pane cannot end up with two competing timers. `run` is supplied per call, so each fire
 * uses the latest mutation binding rather than one captured when the burst started.
 */
export const schedulePaneResizeCommit = (
    paneKey: string,
    run: () => void,
    options: { delayMs?: number; scheduler?: PaneResizeCommitScheduler } = {},
) => {
    const { delayMs = PANE_RESIZE_COMMIT_DEBOUNCE_MS, scheduler = timeoutScheduler } = options
    const pending = pendingTimerByPane.get(paneKey)
    if (pending !== undefined) scheduler.cancel(pending)

    pendingTimerByPane.set(
        paneKey,
        scheduler.schedule(() => {
            pendingTimerByPane.delete(paneKey)
            run()
        }, delayMs),
    )
}

/** Test-only reset so one test's pending timers cannot leak into the next. */
export const clearPendingPaneResizeCommits = () => pendingTimerByPane.clear()

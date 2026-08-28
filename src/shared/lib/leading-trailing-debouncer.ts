/**
 * `schedule`/`cancel` are declared with method shorthand, not as function-typed properties, so this
 * type opts into TypeScript's bivariant method parameter checking: `createLeadingTrailingDebouncer`'s
 * default `scheduler` needs a concretely `ReturnType<typeof setTimeout>`-handled scheduler to be
 * assignable where a `Handle = unknown`-handled one is expected, and a test's `number`-handled fake
 * scheduler needs the reverse — neither direction holds under the stricter, invariance-leaning
 * property-function check (contract d-45 F-09).
 */
export type LeadingTrailingDebouncerScheduler<Handle = unknown> = {
    schedule(callback: () => void, delayMs: number): Handle
    cancel(timerId: Handle): void
}

const timeoutScheduler: LeadingTrailingDebouncerScheduler<ReturnType<typeof setTimeout>> = {
    schedule: (callback, delayMs) => setTimeout(callback, delayMs),
    cancel: (timerId) => clearTimeout(timerId),
}

export type LeadingTrailingDebouncer<Result> = {
    trigger: () => Result | undefined
}

/**
 * Leading+trailing debounce: a `trigger()` call while idle runs `run` immediately (leading edge), so
 * a one-off call arriving while idle (boot, a manual theme switch) stays instant, without waiting out
 * a cooldown it has no burst behind it. Every `trigger()` call that arrives before `delayMs` has
 * passed since the previous one resets the wait window (classic debounce, not a fixed-interval
 * throttle) and is folded into a single pending trailing run, so a continuous flood of calls produces
 * exactly one more `run` after the flood actually stops — never one per call while it is still
 * ongoing, and never immediately if it arrives mid-cooldown rather than while idle (contract d-45
 * F-06). Built for `applyShikiTheme` (contract d-45 §0, §1#3): a color-picker drag can call it once
 * per `pointermove`, and each call's monaco retokenize (`highlighter.loadTheme` + tokens-provider
 * repatch) is expensive enough that running it per-move — even once the preview pipeline itself is
 * capped — still floods the main thread.
 *
 * Kept as a standalone `shared/lib` utility with a single consumer today (`shiki-monaco.ts`) so its
 * timer semantics — leading/trailing edges, wait-window reset, injected scheduler — are unit-testable
 * on their own rather than only reachable through shiki's much larger surface (contract d-45 F-10).
 * This is a readability/testability split (common.md §3.3), not a bet on future reuse; promote its
 * placement decision to `shared` in more than name only once a second consumer actually shows up.
 */
export const createLeadingTrailingDebouncer = <Result>(
    run: () => Result,
    delayMs: number,
    scheduler: LeadingTrailingDebouncerScheduler = timeoutScheduler,
): LeadingTrailingDebouncer<Result> => {
    let timerId: unknown = null
    let hasTrailingCall = false

    const clearTimer = () => {
        if (timerId === null) return
        scheduler.cancel(timerId)
        timerId = null
    }

    const armTimer = () => {
        timerId = scheduler.schedule(() => {
            timerId = null
            if (!hasTrailingCall) return
            hasTrailingCall = false
            run()
        }, delayMs)
    }

    const trigger = (): Result | undefined => {
        if (timerId === null) {
            const result = run()
            armTimer()
            return result
        }
        hasTrailingCall = true
        clearTimer()
        armTimer()
        return undefined
    }

    return { trigger }
}

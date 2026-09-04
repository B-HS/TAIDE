const IS_DEV_BUILD = import.meta.env.DEV === true

/**
 * Session cap on the User Timing entries this module hands to the browser. Marks and measures both
 * draw from it. The app's own diagnostics channel is a 40KB rotating file log
 * (`docs/debugging.md` §1) and the devtools timeline is finite, so a runaway instrumentation site
 * (a mark inside a render that loops, a measure on a high-frequency path) must degrade into
 * silence rather than crowd out the signal it was added to find. Reaching the cap stops emission
 * only — the in-memory aggregates below keep accumulating, so a long session still reports honest
 * totals.
 */
export const PERF_MARK_LIMIT = 500

/**
 * Start points. Names are a closed set for the same reason `infra::perf`'s Rust slots are: a typo
 * would silently produce a measurement nobody reads, and the names are a contract — renaming one
 * invalidates every baseline recorded against it (`docs/quality-assurance/2026-09-04-perf-baseline.md`).
 */
export const PERF_MARK = {
    BOOT_MODULE_EVALUATED: 'boot.module-evaluated',
    PROJECT_SWITCH_REQUESTED: 'project.switch-requested',
    FILE_OPEN_REQUESTED: 'file.open-requested',
    TREE_TOGGLE_REQUESTED: 'tree.toggle-requested',
    SEARCH_RUN_REQUESTED: 'search.run-requested',
    PALETTE_OPEN_REQUESTED: 'palette.open-requested',
    PALETTE_QUERY_CHANGED: 'palette.query-changed',
} as const

export type PerfMarkName = (typeof PERF_MARK)[keyof typeof PERF_MARK]

/** Durations, each measured from the {@link PERF_MARK} its call site names. */
export const PERF_MEASURE = {
    BOOT_REVEAL: 'boot.reveal',
    PROJECT_SWITCH: 'project.switch',
    FILE_OPEN: 'file.open',
    TREE_TOGGLE: 'tree.toggle',
    SEARCH_RESULTS: 'search.results',
    PALETTE_OPEN: 'palette.open',
    PALETTE_FILTER: 'palette.filter',
} as const

export type PerfMeasureName = (typeof PERF_MEASURE)[keyof typeof PERF_MEASURE]

/**
 * Running totals with no duration attached — the shape every high-frequency site must use, mirroring
 * `infra::perf::CounterSlot`'s rationale: timing each terminal chunk would cost two clock reads per
 * chunk and distort the throughput it is measuring. Throughput is derived outside the app by
 * dividing two snapshots by wall time.
 */
export const PERF_COUNTER = {
    TERMINAL_OUTPUT_BYTES: 'terminal.output-bytes',
    TERMINAL_OUTPUT_CHUNKS: 'terminal.output-chunks',
} as const

export type PerfCounterName = (typeof PERF_COUNTER)[keyof typeof PERF_COUNTER]

export type PerfMeasureStat = { count: number; totalMs: number; maxMs: number; lastMs: number }

/**
 * The gate rule, shared with the Rust side: `TAIDE_PERF` decides it for the whole process
 * (`infra::perf::resolve_enabled`), and the front end adopts that answer as soon as it can read it
 * (`entities/app/perf.ipc.ts`). Until then — the window's first frames, before any IPC has
 * completed — the build default stands, which is why `nativeGate` is nullable rather than a plain
 * boolean: "not read yet" and "read as off" must not collapse into the same value, or a dev build
 * would start with instrumentation off and lose its own boot measurement.
 */
export const resolvePerfEnabled = (isDevBuild: boolean, nativeGate: boolean | null) => nativeGate ?? isDevBuild

const markTimestamps = new Map<PerfMarkName, number>()
const measureStats = new Map<PerfMeasureName, PerfMeasureStat>()
const counterTotals = new Map<PerfCounterName, number>()

let isEnabled = resolvePerfEnabled(IS_DEV_BUILD, null)
let emittedEntryCount = 0

export const applyNativePerfGate = (nativeGate: boolean) => {
    isEnabled = resolvePerfEnabled(IS_DEV_BUILD, nativeGate)
}

export const isPerfEnabled = () => isEnabled

const claimEntryBudget = () => {
    if (!isEnabled) return false
    if (emittedEntryCount >= PERF_MARK_LIMIT) return false
    emittedEntryCount += 1
    return true
}

/**
 * Records a start point. The timestamp is kept in this module whether or not instrumentation is on,
 * so a measure taken *after* the gate flips on (the boot case: the entry point marks before any IPC
 * could have told the front end that `TAIDE_PERF=1` in a release build) still reports a real
 * duration instead of vanishing. That unconditional half costs one clock read and one `Map.set` at
 * seven low-frequency sites; the `performance.mark` that makes the point visible in the devtools
 * timeline is the part the gate — and {@link PERF_MARK_LIMIT} — actually withhold. High-frequency
 * paths must use {@link perfCount}, which is a true no-op while off.
 *
 * A second mark of the same name replaces the first: every start point here is consumed by exactly
 * one measure ({@link perfMeasure}), so an unconsumed one is a stale start whose only correct fate
 * is to be overwritten by the next attempt.
 */
export const perfMark = (name: PerfMarkName) => {
    markTimestamps.set(name, performance.now())
    if (!claimEntryBudget()) return
    performance.clearMarks(name)
    performance.mark(name)
}

/**
 * Closes the span opened by `from` and returns its duration in milliseconds, or `null` when there
 * is nothing to close.
 *
 * The start point is **consumed**. Instrumented call sites are effects that also run for unrelated
 * reasons — a tree page arriving from a refresh rather than the expand that was marked, a search
 * result list re-rendering with no new run — and consuming makes those extra runs measure nothing
 * instead of reporting the age of a stale mark as if it were the operation's cost. It also bounds
 * the timestamp map without any expiry logic.
 */
export const perfMeasure = (name: PerfMeasureName, from: PerfMarkName) => {
    const start = markTimestamps.get(from)
    if (start === undefined) return null
    markTimestamps.delete(from)
    if (!isEnabled) return null

    const durationMs = performance.now() - start
    const previous = measureStats.get(name)
    measureStats.set(name, {
        count: (previous?.count ?? 0) + 1,
        totalMs: (previous?.totalMs ?? 0) + durationMs,
        maxMs: Math.max(previous?.maxMs ?? 0, durationMs),
        lastMs: durationMs,
    })

    if (claimEntryBudget()) performance.measure(name, { start, duration: durationMs })
    if (IS_DEV_BUILD) console.debug(`[perf] ${name} ${durationMs.toFixed(1)}ms`)

    return durationMs
}

/**
 * Adds to a running total. Unlike {@link perfMark} this is a true no-op while instrumentation is
 * off, because its call sites are per-chunk (terminal output), not per-interaction.
 */
export const perfCount = (name: PerfCounterName, amount = 1) => {
    if (!isEnabled) return
    counterTotals.set(name, (counterTotals.get(name) ?? 0) + amount)
}

export type PerfReport = {
    enabled: boolean
    emittedEntryCount: number
    measures: ({ name: PerfMeasureName } & PerfMeasureStat)[]
    counters: { name: PerfCounterName; total: number }[]
}

/**
 * Everything this module has accumulated. `enabled` is part of the report for the same reason
 * `perf_snapshot`'s reply carries it: an empty report means "instrumentation is off" or "nothing
 * happened", and only this flag separates the two.
 */
export const buildPerfReport = (): PerfReport => ({
    enabled: isEnabled,
    emittedEntryCount,
    measures: [...measureStats].map(([name, stat]) => ({ name, ...stat })),
    counters: [...counterTotals].map(([name, total]) => ({ name, total })),
})

/**
 * Prints the report to the devtools console — what the `app.showPerfSnapshot` command runs. Console
 * output rather than a UI surface on purpose: this is a developer instrument, and a panel for it
 * would have to be translated, themed, and kept working in every window kind. The Rust half is read
 * next to it with `invoke('perf_snapshot')` (`docs/debugging.md` §4.1); this module cannot call that
 * command itself, since `shared` may not reach the `entities` layer that owns IPC.
 */
export const printPerfReport = () => {
    const report = buildPerfReport()
    console.info(`[perf] enabled=${report.enabled} entries=${report.emittedEntryCount}/${PERF_MARK_LIMIT}`)
    console.table(report.measures)
    console.table(report.counters)
}

/** Starts a fresh measurement window. Leaves the gate alone, mirroring `perf_reset`. */
export const resetPerfMetrics = () => {
    markTimestamps.clear()
    measureStats.clear()
    counterTotals.clear()
    emittedEntryCount = 0
}

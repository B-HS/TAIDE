/**
 * A `Map` that records how often it was read and written, for the operation-count budget tests
 * (`docs/quality-assurance/2026-09-04-perf-baseline.md` §5).
 *
 * Those tests assert *algorithmic* cost — "one lookup per batch, whatever the accumulated size" —
 * without measuring time, so they stay deterministic on a shared CI runner where a wall-clock
 * threshold would be flaky. Counting the map operations a function performs is the cheapest honest
 * proxy for that cost, and works on production code that carries no instrumentation of its own.
 *
 * Always constructed empty: `Map`'s own constructor calls `this.set` for every entry of an iterable
 * argument, which under `useDefineForClassFields` would run before `setCount` exists. Fill it with
 * `set` calls and use {@link resetCounts} to start the measurement window.
 */
export class CountingMap<K, V> extends Map<K, V> {
    getCount = 0
    setCount = 0

    get(key: K) {
        this.getCount += 1
        return super.get(key)
    }

    set(key: K, value: V) {
        this.setCount += 1
        return super.set(key, value)
    }

    resetCounts() {
        this.getCount = 0
        this.setCount = 0
    }
}

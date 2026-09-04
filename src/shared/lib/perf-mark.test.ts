import { beforeEach, describe, expect, spyOn, test } from 'bun:test'
import {
    applyNativePerfGate,
    buildPerfReport,
    isPerfEnabled,
    PERF_COUNTER,
    PERF_MARK,
    PERF_MARK_LIMIT,
    PERF_MEASURE,
    perfCount,
    perfMark,
    perfMeasure,
    printPerfReport,
    resetPerfMetrics,
    resolvePerfEnabled,
} from '@shared/lib/perf-mark'

beforeEach(() => {
    resetPerfMetrics()
    applyNativePerfGate(false)
})

describe('resolvePerfEnabled', () => {
    test('네이티브 게이트를 아직 못 읽었으면 빌드 기본값을 따른다', () => {
        expect(resolvePerfEnabled(true, null)).toBe(true)
        expect(resolvePerfEnabled(false, null)).toBe(false)
    })

    test('네이티브 게이트를 읽었으면 빌드 기본값을 덮어쓴다 — 양방향', () => {
        expect(resolvePerfEnabled(false, true)).toBe(true)
        expect(resolvePerfEnabled(true, false)).toBe(false)
    })
})

describe('perfMark · perfMeasure', () => {
    test('마크를 찍고 측정하면 밀리초 duration 을 돌려준다', () => {
        applyNativePerfGate(true)
        perfMark(PERF_MARK.FILE_OPEN_REQUESTED)

        const durationMs = perfMeasure(PERF_MEASURE.FILE_OPEN, PERF_MARK.FILE_OPEN_REQUESTED)

        expect(durationMs).not.toBeNull()
        expect(durationMs ?? -1).toBeGreaterThanOrEqual(0)
    })

    test('시작 마크가 없으면 null 을 돌려주고 아무것도 기록하지 않는다', () => {
        applyNativePerfGate(true)

        expect(perfMeasure(PERF_MEASURE.TREE_TOGGLE, PERF_MARK.TREE_TOGGLE_REQUESTED)).toBeNull()
        expect(buildPerfReport().measures).toEqual([])
    })

    test('시작 마크는 첫 측정에 소비된다 — 두 번째 측정은 null', () => {
        applyNativePerfGate(true)
        perfMark(PERF_MARK.TREE_TOGGLE_REQUESTED)

        expect(perfMeasure(PERF_MEASURE.TREE_TOGGLE, PERF_MARK.TREE_TOGGLE_REQUESTED)).not.toBeNull()
        expect(perfMeasure(PERF_MEASURE.TREE_TOGGLE, PERF_MARK.TREE_TOGGLE_REQUESTED)).toBeNull()
        expect(buildPerfReport().measures[0]?.count).toBe(1)
    })

    test('같은 이름의 마크를 다시 찍으면 최신 시작점만 남는다', () => {
        applyNativePerfGate(true)
        perfMark(PERF_MARK.SEARCH_RUN_REQUESTED)
        perfMark(PERF_MARK.SEARCH_RUN_REQUESTED)

        expect(perfMeasure(PERF_MEASURE.SEARCH_RESULTS, PERF_MARK.SEARCH_RUN_REQUESTED)).not.toBeNull()
        expect(perfMeasure(PERF_MEASURE.SEARCH_RESULTS, PERF_MARK.SEARCH_RUN_REQUESTED)).toBeNull()
    })

    test('반복 측정은 count·total·max·last 로 누적된다', () => {
        applyNativePerfGate(true)
        perfMark(PERF_MARK.PROJECT_SWITCH_REQUESTED)
        const first = perfMeasure(PERF_MEASURE.PROJECT_SWITCH, PERF_MARK.PROJECT_SWITCH_REQUESTED) ?? 0
        perfMark(PERF_MARK.PROJECT_SWITCH_REQUESTED)
        const second = perfMeasure(PERF_MEASURE.PROJECT_SWITCH, PERF_MARK.PROJECT_SWITCH_REQUESTED) ?? 0

        const stat = buildPerfReport().measures.find((entry) => entry.name === PERF_MEASURE.PROJECT_SWITCH)

        expect(stat?.count).toBe(2)
        expect(stat?.totalMs).toBeCloseTo(first + second, 6)
        expect(stat?.maxMs).toBeCloseTo(Math.max(first, second), 6)
        expect(stat?.lastMs).toBeCloseTo(second, 6)
    })

    test('게이트가 꺼져 있으면 측정하지 않는다', () => {
        perfMark(PERF_MARK.FILE_OPEN_REQUESTED)

        expect(perfMeasure(PERF_MEASURE.FILE_OPEN, PERF_MARK.FILE_OPEN_REQUESTED)).toBeNull()
        expect(buildPerfReport()).toMatchObject({ enabled: false, measures: [], counters: [], emittedEntryCount: 0 })
    })

    /**
     * The boot contract: `main.tsx` marks before any IPC could have told the front end that
     * `TAIDE_PERF=1` in a release build, so a start point recorded while the gate was still off must
     * still produce a real duration once the gate opens.
     */
    test('게이트가 꺼진 동안 찍은 마크도 게이트가 열린 뒤에는 측정된다 — 부팅 계측 계약', () => {
        perfMark(PERF_MARK.BOOT_MODULE_EVALUATED)
        applyNativePerfGate(true)

        const durationMs = perfMeasure(PERF_MEASURE.BOOT_REVEAL, PERF_MARK.BOOT_MODULE_EVALUATED)

        expect(durationMs).not.toBeNull()
        expect(buildPerfReport().measures.map((entry) => entry.name)).toEqual([PERF_MEASURE.BOOT_REVEAL])
    })

    test('상한을 넘기면 User Timing 발행만 멈추고 누적 통계는 계속 쌓인다', () => {
        applyNativePerfGate(true)
        for (let index = 0; index < PERF_MARK_LIMIT; index += 1) perfMark(PERF_MARK.PALETTE_QUERY_CHANGED)

        expect(buildPerfReport().emittedEntryCount).toBe(PERF_MARK_LIMIT)

        perfMark(PERF_MARK.PALETTE_QUERY_CHANGED)
        const durationMs = perfMeasure(PERF_MEASURE.PALETTE_FILTER, PERF_MARK.PALETTE_QUERY_CHANGED)
        const report = buildPerfReport()

        expect(report.emittedEntryCount).toBe(PERF_MARK_LIMIT)
        expect(durationMs).not.toBeNull()
        expect(report.measures.find((entry) => entry.name === PERF_MEASURE.PALETTE_FILTER)?.count).toBe(1)
    })

    test('발행된 마크는 User Timing 타임라인에도 남는다', () => {
        applyNativePerfGate(true)
        perfMark(PERF_MARK.PALETTE_OPEN_REQUESTED)

        expect(performance.getEntriesByName(PERF_MARK.PALETTE_OPEN_REQUESTED, 'mark').length).toBeGreaterThan(0)
        performance.clearMarks(PERF_MARK.PALETTE_OPEN_REQUESTED)
    })
})

describe('perfCount', () => {
    test('게이트가 켜져 있으면 누적하고, 인자를 생략하면 1 씩 센다', () => {
        applyNativePerfGate(true)
        perfCount(PERF_COUNTER.TERMINAL_OUTPUT_BYTES, 120)
        perfCount(PERF_COUNTER.TERMINAL_OUTPUT_BYTES, 80)
        perfCount(PERF_COUNTER.TERMINAL_OUTPUT_CHUNKS)
        perfCount(PERF_COUNTER.TERMINAL_OUTPUT_CHUNKS)

        expect(buildPerfReport().counters).toEqual([
            { name: PERF_COUNTER.TERMINAL_OUTPUT_BYTES, total: 200 },
            { name: PERF_COUNTER.TERMINAL_OUTPUT_CHUNKS, total: 2 },
        ])
    })

    test('게이트가 꺼져 있으면 완전한 no-op 이다 — 고빈도 경로 계약', () => {
        perfCount(PERF_COUNTER.TERMINAL_OUTPUT_BYTES, 4096)

        expect(buildPerfReport().counters).toEqual([])
    })
})

describe('resetPerfMetrics · printPerfReport · isPerfEnabled', () => {
    test('reset 은 누적치와 발행 예산을 비우되 게이트는 유지한다', () => {
        applyNativePerfGate(true)
        perfMark(PERF_MARK.FILE_OPEN_REQUESTED)
        perfMeasure(PERF_MEASURE.FILE_OPEN, PERF_MARK.FILE_OPEN_REQUESTED)
        perfCount(PERF_COUNTER.TERMINAL_OUTPUT_BYTES, 10)

        resetPerfMetrics()

        expect(buildPerfReport()).toMatchObject({ enabled: true, measures: [], counters: [], emittedEntryCount: 0 })
        expect(isPerfEnabled()).toBe(true)
    })

    test('reset 은 소비되지 않은 시작 마크도 비운다', () => {
        applyNativePerfGate(true)
        perfMark(PERF_MARK.TREE_TOGGLE_REQUESTED)

        resetPerfMetrics()

        expect(perfMeasure(PERF_MEASURE.TREE_TOGGLE, PERF_MARK.TREE_TOGGLE_REQUESTED)).toBeNull()
    })

    test('printPerfReport 는 게이트 상태와 두 표를 콘솔에 낸다', () => {
        const info = spyOn(console, 'info').mockImplementation(() => {})
        const table = spyOn(console, 'table').mockImplementation(() => {})

        printPerfReport()

        expect(info).toHaveBeenCalledTimes(1)
        expect(table).toHaveBeenCalledTimes(2)
    })
})

import { describe, expect, test } from 'bun:test'
import type { SearchFileMatches } from '@shared/api/bindings'
import type { SearchResultAccumulator } from '@entities/search/search-result'
import { appendSearchFileMatches } from '@entities/search/search-result'
import { CountingMap } from '@shared/testing/counting-map'

/**
 * Operation-count guard for the regression audit §1-3 fixed: grouping used to be rebuilt from
 * scratch for every arriving match, so a run's cost grew with everything already accumulated. The
 * budget here is per *batch*, never per accumulated group, and it is asserted by counting map
 * operations rather than time (`docs/quality-assurance/2026-09-04-perf-baseline.md` §5).
 */
const buildBatches = (count: number, offset: number): SearchFileMatches[] =>
    Array.from({ length: count }, (_, index) => ({
        path: `/repo/src/file-${offset + index}.ts`,
        matches: [{ line: 1, column: 1, preview: 'const value = 1', matchStart: 6, matchEnd: 11, before: [], after: [] }],
    }))

const createCountingAccumulator = () => {
    const indexByPath = new CountingMap<string, number>()
    const accumulator: SearchResultAccumulator = { groups: [], indexByPath, totalMatches: 0 }
    return { accumulator, indexByPath }
}

describe('appendSearchFileMatches 연산 예산', () => {
    test('새 경로 배치는 배치당 조회 1회·기록 1회만 쓴다', () => {
        const { accumulator, indexByPath } = createCountingAccumulator()

        appendSearchFileMatches(accumulator, buildBatches(100, 0))

        expect(indexByPath.getCount).toBe(100)
        expect(indexByPath.setCount).toBe(100)
    })

    test('이미 100개 그룹이 쌓인 뒤에도 다음 100배치의 비용은 그대로다 — O(n²) 재그룹 회귀 가드', () => {
        const { accumulator, indexByPath } = createCountingAccumulator()
        appendSearchFileMatches(accumulator, buildBatches(100, 0))
        indexByPath.resetCounts()

        appendSearchFileMatches(accumulator, buildBatches(100, 100))

        expect(indexByPath.getCount).toBe(100)
        expect(indexByPath.setCount).toBe(100)
        expect(accumulator.groups.length).toBe(200)
    })

    test('같은 경로가 다시 오면 조회만 하고 인덱스를 다시 쓰지 않는다', () => {
        const { accumulator, indexByPath } = createCountingAccumulator()
        appendSearchFileMatches(accumulator, buildBatches(10, 0))
        indexByPath.resetCounts()

        appendSearchFileMatches(accumulator, buildBatches(3, 0))

        expect(indexByPath.getCount).toBe(3)
        expect(indexByPath.setCount).toBe(0)
        expect(accumulator.groups.length).toBe(10)
    })

    test('빈 배치 목록은 맵을 건드리지 않는다', () => {
        const { accumulator, indexByPath } = createCountingAccumulator()

        appendSearchFileMatches(accumulator, [])

        expect(indexByPath.getCount).toBe(0)
        expect(indexByPath.setCount).toBe(0)
    })
})

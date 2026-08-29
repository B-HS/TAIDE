import { describe, expect, test } from 'bun:test'
import type { SearchFileMatches, SearchLineMatch } from '@shared/api/bindings'
import { appendSearchFileMatches, createSearchResultAccumulator, restoreSearchResultAccumulator } from '@entities/search/search-result'

const makeMatch = (overrides: Partial<SearchLineMatch> = {}): SearchLineMatch => ({
    line: 1,
    column: 1,
    preview: 'hello',
    matchStart: 0,
    matchEnd: 5,
    before: [],
    after: [],
    ...overrides,
})

const makeBatch = (path: string, matches: SearchLineMatch[]): SearchFileMatches => ({ path, matches })

const groupAll = (batches: SearchFileMatches[]) => appendSearchFileMatches(createSearchResultAccumulator(), batches)

describe('appendSearchFileMatches', () => {
    test('배치 하나가 그룹 하나가 된다', () => {
        const groups = groupAll([makeBatch('src/a.ts', [makeMatch({ line: 1 }), makeMatch({ line: 2 })]), makeBatch('src/b.ts', [makeMatch()])])

        expect(groups).toHaveLength(2)
        expect(groups[0]).toMatchObject({ path: 'src/a.ts' })
        expect(groups[0]?.matches).toHaveLength(2)
        expect(groups[1]).toMatchObject({ path: 'src/b.ts' })
        expect(groups[1]?.matches).toHaveLength(1)
    })

    test('그룹은 배치가 도착한 순서를 유지한다', () => {
        const groups = groupAll([makeBatch('src/z.ts', [makeMatch()]), makeBatch('src/a.ts', [makeMatch()])])

        expect(groups.map((group) => group.path)).toEqual(['src/z.ts', 'src/a.ts'])
    })

    test('같은 경로가 두 번 도착하면 새 그룹을 만들지 않고 기존 그룹에 이어붙인다', () => {
        const accumulator = createSearchResultAccumulator()

        appendSearchFileMatches(accumulator, [makeBatch('src/a.ts', [makeMatch({ line: 1 })])])
        const groups = appendSearchFileMatches(accumulator, [makeBatch('src/a.ts', [makeMatch({ line: 5 })])])

        expect(groups).toHaveLength(1)
        expect(groups[0]?.matches.map((match) => match.line)).toEqual([1, 5])
    })

    test('여러 번 append 해도 이전 배치의 그룹 객체 정체성이 유지된다', () => {
        const accumulator = createSearchResultAccumulator()

        const first = appendSearchFileMatches(accumulator, [makeBatch('src/a.ts', [makeMatch()])])
        const second = appendSearchFileMatches(accumulator, [makeBatch('src/b.ts', [makeMatch()])])

        expect(second).not.toBe(first)
        expect(second[0]).toBe(first[0])
    })

    test('누적 매치 수를 세어둔다', () => {
        const accumulator = createSearchResultAccumulator()

        appendSearchFileMatches(accumulator, [makeBatch('src/a.ts', [makeMatch({ line: 1 }), makeMatch({ line: 2 })])])
        appendSearchFileMatches(accumulator, [makeBatch('src/b.ts', [makeMatch()])])

        expect(accumulator.totalMatches).toBe(3)
    })

    test('컨텍스트 줄(before/after)을 그대로 보존한다', () => {
        const groups = groupAll([makeBatch('src/a.ts', [makeMatch({ before: ['line before'], after: ['line after'] })])])

        expect(groups[0]?.matches[0]).toMatchObject({ before: ['line before'], after: ['line after'] })
    })

    test('빈 배치 목록이면 기존 그룹을 그대로 돌려준다', () => {
        const accumulator = createSearchResultAccumulator()

        expect(appendSearchFileMatches(accumulator, [])).toEqual([])
        expect(accumulator.totalMatches).toBe(0)
    })

    test('인접한 매치의 컨텍스트 줄이 겹치면 겹치는 부분을 제거해 중복 렌더링을 막는다', () => {
        const groups = groupAll([
            makeBatch('src/a.ts', [
                makeMatch({ line: 10, before: ['line 8', 'line 9'], after: ['line 11', 'line 12', 'line 13'] }),
                makeMatch({ line: 12, before: ['line 10', 'line 11'], after: ['line 13', 'line 14'] }),
            ]),
        ])

        expect(groups[0]?.matches[0]).toMatchObject({ line: 10, before: ['line 8', 'line 9'], after: ['line 11'] })
        expect(groups[0]?.matches[1]).toMatchObject({ line: 12, before: [], after: ['line 13', 'line 14'] })
    })

    test('같은 줄에 여러 매치가 있으면 컨텍스트 줄을 한 번만 표시한다', () => {
        const groups = groupAll([
            makeBatch('src/a.ts', [
                makeMatch({ line: 5, column: 1, before: ['line 4'], after: ['line 6'] }),
                makeMatch({ line: 5, column: 10, before: ['line 4'], after: ['line 6'] }),
            ]),
        ])

        expect(groups[0]?.matches[0]).toMatchObject({ before: ['line 4'], after: [] })
        expect(groups[0]?.matches[1]).toMatchObject({ before: [], after: ['line 6'] })
    })

    test('겹치지 않는 매치의 컨텍스트 줄은 그대로 보존한다', () => {
        const groups = groupAll([
            makeBatch('src/a.ts', [
                makeMatch({ line: 5, before: ['line 3', 'line 4'], after: ['line 6', 'line 7'] }),
                makeMatch({ line: 20, before: ['line 18', 'line 19'], after: ['line 21', 'line 22'] }),
            ]),
        ])

        expect(groups[0]?.matches[0]).toMatchObject({ before: ['line 3', 'line 4'], after: ['line 6', 'line 7'] })
        expect(groups[0]?.matches[1]).toMatchObject({ before: ['line 18', 'line 19'], after: ['line 21', 'line 22'] })
    })

    test('이어붙인 그룹도 컨텍스트 중복 제거를 다시 적용한다', () => {
        const accumulator = createSearchResultAccumulator()

        appendSearchFileMatches(accumulator, [makeBatch('src/a.ts', [makeMatch({ line: 10, before: [], after: ['line 11', 'line 12'] })])])
        const groups = appendSearchFileMatches(accumulator, [
            makeBatch('src/a.ts', [makeMatch({ line: 12, before: ['line 10', 'line 11'], after: [] })]),
        ])

        expect(groups[0]?.matches[0]).toMatchObject({ line: 10, after: ['line 11'] })
        expect(groups[0]?.matches[1]).toMatchObject({ line: 12, before: [] })
    })
})

describe('restoreSearchResultAccumulator', () => {
    test('복원한 누적기는 기존 그룹과 누적 수를 그대로 이어받는다', () => {
        const accumulator = restoreSearchResultAccumulator([{ path: 'src/a.ts', matches: [makeMatch({ line: 1 })] }], 1)

        expect(accumulator.groups).toHaveLength(1)
        expect(accumulator.totalMatches).toBe(1)
    })

    test('복원한 뒤 같은 경로가 다시 도착하면 새 그룹을 만들지 않는다', () => {
        const accumulator = restoreSearchResultAccumulator([{ path: 'src/a.ts', matches: [makeMatch({ line: 1 })] }], 1)

        const groups = appendSearchFileMatches(accumulator, [makeBatch('src/a.ts', [makeMatch({ line: 9 })])])

        expect(groups).toHaveLength(1)
        expect(groups[0]?.matches.map((match) => match.line)).toEqual([1, 9])
        expect(accumulator.totalMatches).toBe(2)
    })
})

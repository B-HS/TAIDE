import { describe, expect, test } from 'bun:test'
import type { SearchMatch } from '@shared/api/bindings'
import { groupSearchMatches } from '@entities/search/search-result'

const makeMatch = (overrides: Partial<SearchMatch> = {}): SearchMatch => ({
    path: 'src/a.ts',
    line: 1,
    column: 1,
    preview: 'hello',
    matchStart: 0,
    matchEnd: 5,
    before: [],
    after: [],
    ...overrides,
})

describe('groupSearchMatches', () => {
    test('같은 경로의 매치를 하나의 그룹으로 묶는다', () => {
        const matches = [makeMatch({ path: 'src/a.ts', line: 1 }), makeMatch({ path: 'src/a.ts', line: 2 }), makeMatch({ path: 'src/b.ts', line: 1 })]

        const groups = groupSearchMatches(matches)

        expect(groups).toHaveLength(2)
        expect(groups[0]).toMatchObject({ path: 'src/a.ts' })
        expect(groups[0]?.matches).toHaveLength(2)
        expect(groups[1]).toMatchObject({ path: 'src/b.ts' })
        expect(groups[1]?.matches).toHaveLength(1)
    })

    test('그룹은 처음 등장한 경로 순서를 유지한다', () => {
        const matches = [makeMatch({ path: 'src/z.ts' }), makeMatch({ path: 'src/a.ts' }), makeMatch({ path: 'src/z.ts' })]

        const groups = groupSearchMatches(matches)

        expect(groups.map((group) => group.path)).toEqual(['src/z.ts', 'src/a.ts'])
    })

    test('컨텍스트 줄(before/after)을 그대로 보존한다', () => {
        const matches = [makeMatch({ before: ['line before'], after: ['line after'] })]

        const groups = groupSearchMatches(matches)

        expect(groups[0]?.matches[0]).toMatchObject({ before: ['line before'], after: ['line after'] })
    })

    test('빈 배열이면 빈 그룹 목록을 반환한다', () => {
        expect(groupSearchMatches([])).toEqual([])
    })

    test('인접한 매치의 컨텍스트 줄이 겹치면 겹치는 부분을 제거해 중복 렌더링을 막는다', () => {
        const matches = [
            makeMatch({ line: 10, before: ['line 8', 'line 9'], after: ['line 11', 'line 12', 'line 13'] }),
            makeMatch({ line: 12, before: ['line 10', 'line 11'], after: ['line 13', 'line 14'] }),
        ]

        const groups = groupSearchMatches(matches)

        expect(groups[0]?.matches[0]).toMatchObject({ line: 10, before: ['line 8', 'line 9'], after: ['line 11'] })
        expect(groups[0]?.matches[1]).toMatchObject({ line: 12, before: [], after: ['line 13', 'line 14'] })
    })

    test('같은 줄에 여러 매치가 있으면 컨텍스트 줄을 한 번만 표시한다', () => {
        const matches = [
            makeMatch({ line: 5, column: 1, before: ['line 4'], after: ['line 6'] }),
            makeMatch({ line: 5, column: 10, before: ['line 4'], after: ['line 6'] }),
        ]

        const groups = groupSearchMatches(matches)

        expect(groups[0]?.matches[0]).toMatchObject({ before: ['line 4'], after: [] })
        expect(groups[0]?.matches[1]).toMatchObject({ before: [], after: ['line 6'] })
    })

    test('겹치지 않는 매치의 컨텍스트 줄은 그대로 보존한다', () => {
        const matches = [
            makeMatch({ line: 5, before: ['line 3', 'line 4'], after: ['line 6', 'line 7'] }),
            makeMatch({ line: 20, before: ['line 18', 'line 19'], after: ['line 21', 'line 22'] }),
        ]

        const groups = groupSearchMatches(matches)

        expect(groups[0]?.matches[0]).toMatchObject({ before: ['line 3', 'line 4'], after: ['line 6', 'line 7'] })
        expect(groups[0]?.matches[1]).toMatchObject({ before: ['line 18', 'line 19'], after: ['line 21', 'line 22'] })
    })
})

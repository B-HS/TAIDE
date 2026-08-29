import { describe, expect, test } from 'bun:test'
import type { SearchResultGroup } from '@entities/search/search-result'
import { buildSearchResultRows, estimateSearchResultRowHeight } from '@features/search/search-result-rows'

const match = (line: number, before: string[] = [], after: string[] = []) => ({
    line,
    column: 1,
    preview: 'needle',
    matchStart: 0,
    matchEnd: 6,
    before,
    after,
})

const group = (path: string, lines: number[]): SearchResultGroup => ({ path, matches: lines.map((line) => match(line)) })

describe('buildSearchResultRows', () => {
    test('그룹 헤더와 매치 행을 하나의 목록으로 펼친다', () => {
        const rows = buildSearchResultRows([group('src/a.ts', [1, 2]), group('src/b.ts', [7])], new Set())

        expect(rows.map((row) => row.kind)).toEqual(['group', 'match', 'match', 'group', 'match'])
        expect(rows[0]).toMatchObject({ kind: 'group', path: 'src/a.ts', matchCount: 2, collapsed: false })
        expect(rows[1]).toMatchObject({ kind: 'match', path: 'src/a.ts' })
    })

    test('접힌 그룹은 헤더만 남기고 매치 행을 내보내지 않는다', () => {
        const rows = buildSearchResultRows([group('src/a.ts', [1, 2]), group('src/b.ts', [7])], new Set(['src/a.ts']))

        expect(rows.map((row) => row.kind)).toEqual(['group', 'group', 'match'])
        expect(rows[0]).toMatchObject({ collapsed: true, matchCount: 2 })
    })

    test('행 id 는 경로 기준이라 파일 도착 순서가 달라도 안정적이다', () => {
        const ascending = buildSearchResultRows([group('src/a.ts', [1]), group('src/b.ts', [1])], new Set())
        const reversed = buildSearchResultRows([group('src/b.ts', [1]), group('src/a.ts', [1])], new Set())

        expect(new Set(ascending.map((row) => row.id))).toEqual(new Set(reversed.map((row) => row.id)))
    })

    test('같은 파일의 서로 다른 매치는 서로 다른 id 를 가진다', () => {
        const rows = buildSearchResultRows([group('src/a.ts', [1, 2, 3])], new Set())

        expect(new Set(rows.map((row) => row.id)).size).toBe(rows.length)
    })
})

describe('estimateSearchResultRowHeight', () => {
    test('컨텍스트 줄이 많은 매치는 더 높게 잡는다', () => {
        const [, plain] = buildSearchResultRows([group('src/a.ts', [1])], new Set())
        const [, withContext] = buildSearchResultRows([{ path: 'src/b.ts', matches: [match(5, ['a', 'b'], ['c'])] }], new Set())

        expect(estimateSearchResultRowHeight(withContext)).toBeGreaterThan(estimateSearchResultRowHeight(plain))
    })

    test('그룹 헤더 높이는 매치 개수와 무관하다', () => {
        const [oneMatch] = buildSearchResultRows([group('src/a.ts', [1])], new Set())
        const [manyMatches] = buildSearchResultRows([group('src/b.ts', [1, 2, 3, 4])], new Set())

        expect(estimateSearchResultRowHeight(oneMatch)).toBe(estimateSearchResultRowHeight(manyMatches))
    })
})

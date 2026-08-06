import { describe, expect, test } from 'bun:test'
import { fuzzyFilter, fuzzyMatch } from '@shared/lib/fuzzy-match'

describe('fuzzyMatch', () => {
    test('빈 쿼리는 항상 매칭되고 점수 0 을 반환한다', () => {
        expect(fuzzyMatch('', 'anything.ts')).toEqual({ score: 0, indices: [] })
    })

    test('부분 문자가 순서대로 존재하면 매칭된다', () => {
        const result = fuzzyMatch('pnv', 'pane-node-view.tsx')
        expect(result).not.toBeNull()
        expect(result?.indices).toEqual([0, 2, 10])
    })

    test('순서가 어긋나면 매칭되지 않는다', () => {
        expect(fuzzyMatch('vnp', 'pane-node-view.tsx')).toBeNull()
    })

    test('타겟에 없는 문자가 있으면 매칭되지 않는다', () => {
        expect(fuzzyMatch('xyz', 'pane-node-view.tsx')).toBeNull()
    })

    test('대소문자를 구분하지 않는다', () => {
        expect(fuzzyMatch('PNV', 'pane-node-view.tsx')).not.toBeNull()
        expect(fuzzyMatch('pnv', 'Pane-Node-View.tsx')).not.toBeNull()
    })

    test('연속 매칭은 비연속 매칭보다 점수가 높다', () => {
        const consecutive = fuzzyMatch('pan', 'pane.ts')
        const scattered = fuzzyMatch('pts', 'pane.ts')

        expect(consecutive).not.toBeNull()
        expect(scattered).not.toBeNull()
        expect(consecutive!.score).toBeGreaterThan(scattered!.score)
    })

    test('완전한 연속 일치가 부분 연속 일치보다 점수가 높다', () => {
        const fullWord = fuzzyMatch('editor', 'editor-pane.tsx')
        const scatteredChars = fuzzyMatch('edtr', 'editor-pane.tsx')

        expect(fullWord).not.toBeNull()
        expect(scatteredChars).not.toBeNull()
        expect(fullWord!.score).toBeGreaterThan(scatteredChars!.score)
    })
})

describe('fuzzyFilter', () => {
    type Row = { path: string }
    const rows: Row[] = [
        { path: 'src/widgets/editor-area/pane-node-view.tsx' },
        { path: 'src/entities/plugin/plugin.query.ts' },
        { path: 'src/shared/lib/fuzzy-match.ts' },
    ]

    test('매칭되지 않는 항목은 제외한다', () => {
        const result = fuzzyFilter('zzzz', rows, (row) => row.path)
        expect(result).toEqual([])
    })

    test('점수 높은 순으로 정렬한다', () => {
        const result = fuzzyFilter('fuzzy', rows, (row) => row.path)
        expect(result[0]?.item.path).toBe('src/shared/lib/fuzzy-match.ts')
    })

    test('빈 쿼리는 전체 항목을 원래 순서로 반환한다', () => {
        const result = fuzzyFilter('', rows, (row) => row.path)
        expect(result.map((r) => r.item.path)).toEqual(rows.map((r) => r.path))
    })
})

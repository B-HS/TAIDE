import { describe, expect, test } from 'bun:test'
import { buildFuzzyHighlightSegments, fuzzyFilter, fuzzyMatch } from '@shared/lib/fuzzy-match'

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

    test('타겟이 비어 있으면 매칭되지 않는다', () => {
        expect(fuzzyMatch('a', '')).toBeNull()
        expect(fuzzyMatch('', '')).toEqual({ score: 0, indices: [] })
    })

    test('쿼리가 타겟보다 길면 매칭되지 않는다', () => {
        expect(fuzzyMatch('abcd', 'abc')).toBeNull()
    })

    test('같은 문자가 반복되면 항상 가장 앞의 남은 위치를 집는다', () => {
        expect(fuzzyMatch('aa', 'abaca')?.indices).toEqual([0, 2])
    })

    test('매칭 인덱스는 항상 증가하고 타겟 코드유닛 오프셋으로 그대로 슬라이스된다', () => {
        const target = 'src/widgets/editor-area/pane-node-view.tsx'
        const result = fuzzyMatch('pnv', target)
        expect(result).not.toBeNull()
        expect(result!.indices.every((index, position) => position === 0 || index > result!.indices[position - 1])).toBe(true)
        expect(result!.indices.map((index) => target[index]).join('')).toBe('pnv')
    })

    test('서로게이트 쌍(이모지)에 매칭되면 두 코드유닛 인덱스가 모두 담긴다', () => {
        const result = fuzzyMatch('🚀', 'src/🚀rocket.ts')
        expect(result).not.toBeNull()
        expect(result?.indices).toEqual([4, 5])
    })

    test('서로게이트 쌍이 타겟 끝에 있어도 두 코드유닛이 모두 담긴다', () => {
        const target = 'ship🚀'
        const result = fuzzyMatch('s🚀', target)
        expect(result?.indices).toEqual([0, 4, 5])
        expect(target.slice(4, 6)).toBe('🚀')
    })

    test('서로게이트 쌍이 여러 개면 각각의 코드유닛 오프셋을 정확히 가리킨다', () => {
        const result = fuzzyMatch('🚀🎉', '🚀a🎉')
        expect(result?.indices).toEqual([0, 1, 3, 4])
    })

    test('서로게이트 쌍도 코드포인트 1칸으로 세어 연속 매칭 보너스를 받는다', () => {
        const consecutive = fuzzyMatch('a🚀', 'a🚀b')
        const scattered = fuzzyMatch('a🚀', 'axx🚀')
        expect(consecutive!.score).toBeGreaterThan(scattered!.score)
    })

    test('서로게이트 쌍 뒤의 문자 인덱스가 코드유닛 기준으로 밀린다', () => {
        const target = '🚀ab'
        const result = fuzzyMatch('b', target)
        expect(result?.indices).toEqual([3])
        expect(target[3]).toBe('b')
    })

    test('서로게이트 쌍 매칭 인덱스로 세그먼트를 나누면 문자가 쪼개지지 않는다', () => {
        const result = fuzzyMatch('🚀', 'src/🚀rocket.ts')
        expect(result).not.toBeNull()
        expect(buildFuzzyHighlightSegments('src/🚀rocket.ts', result!.indices)).toEqual([
            { text: 'src/', matched: false },
            { text: '🚀', matched: true },
            { text: 'rocket.ts', matched: false },
        ])
    })

    test('소문자화 시 길이가 늘어나는 문자(İ) 뒤의 인덱스가 밀리지 않는다', () => {
        const result = fuzzyMatch('t', 'İstanbul.ts')
        expect(result).not.toBeNull()
        expect(result?.indices).toEqual([2])
        expect('İstanbul.ts'[2]).toBe('t')
    })

    test("İ 는 소문자형이 2코드유닛이라 단일 문자 'i' 쿼리와 매칭되지 않고 뒤의 진짜 i 로 넘어간다", () => {
        expect(fuzzyMatch('i', 'İstanbul')).toBeNull()

        const target = 'İstanbul.min.ts'
        const result = fuzzyMatch('i', target)
        expect(result?.indices).toEqual([10])
        expect(target[10]).toBe('i')
    })

    test('İ 자신을 쿼리로 주면 코드포인트끼리 비교되어 매칭된다', () => {
        expect(fuzzyMatch('İ', 'İstanbul.ts')?.indices).toEqual([0])
    })

    test('İ 가 중간에 있어도 뒤 문자들의 코드유닛 오프셋이 그대로 유지된다', () => {
        const target = 'src/İd.ts'
        const result = fuzzyMatch('sd', target)
        expect(result?.indices).toEqual([0, 5])
        expect(target[5]).toBe('d')
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

describe('buildFuzzyHighlightSegments', () => {
    test('매칭 인덱스가 없으면 전체를 비매칭 세그먼트 하나로 반환한다', () => {
        expect(buildFuzzyHighlightSegments('index.ts', [])).toEqual([{ text: 'index.ts', matched: false }])
    })

    test('빈 문자열은 빈 배열을 반환한다', () => {
        expect(buildFuzzyHighlightSegments('', [0, 1])).toEqual([])
    })

    test('연속된 매칭 인덱스를 하나의 세그먼트로 묶는다', () => {
        expect(buildFuzzyHighlightSegments('index.ts', [0, 1, 2])).toEqual([
            { text: 'ind', matched: true },
            { text: 'ex.ts', matched: false },
        ])
    })

    test('흩어진 매칭 인덱스마다 별도 세그먼트를 만든다', () => {
        const result = fuzzyMatch('pnv', 'pane-node-view.tsx')
        expect(result).not.toBeNull()
        expect(buildFuzzyHighlightSegments('pane-node-view.tsx', result!.indices)).toEqual([
            { text: 'p', matched: true },
            { text: 'a', matched: false },
            { text: 'n', matched: true },
            { text: 'e-node-', matched: false },
            { text: 'v', matched: true },
            { text: 'iew.tsx', matched: false },
        ])
    })

    test('전체 문자가 매칭되면 세그먼트 하나로 반환한다', () => {
        expect(buildFuzzyHighlightSegments('abc', [0, 1, 2])).toEqual([{ text: 'abc', matched: true }])
    })

    test('마지막 문자만 매칭되면 마지막 세그먼트만 매칭 표시한다', () => {
        expect(buildFuzzyHighlightSegments('abc', [2])).toEqual([
            { text: 'ab', matched: false },
            { text: 'c', matched: true },
        ])
    })
})

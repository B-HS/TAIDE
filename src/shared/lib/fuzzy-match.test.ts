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

    test('공백으로 나뉜 토큰이 각각 매칭되면 통과한다', () => {
        const result = fuzzyFilter('widgets pane', rows, (row) => row.path)
        expect(result.map((ranked) => ranked.item.path)).toEqual(['src/widgets/editor-area/pane-node-view.tsx'])
    })

    test('토큰 순서가 라벨 등장 순서와 달라도 매칭된다', () => {
        const forward = fuzzyFilter('widgets pane', rows, (row) => row.path)
        const reversed = fuzzyFilter('pane widgets', rows, (row) => row.path)
        expect(reversed).toEqual(forward)

        const backwardTokens = fuzzyFilter('view editor', rows, (row) => row.path)
        expect(backwardTokens.map((ranked) => ranked.item.path)).toEqual(['src/widgets/editor-area/pane-node-view.tsx'])
    })

    test('토큰이 하나라도 매칭되지 않으면 탈락한다', () => {
        expect(fuzzyFilter('widgets zzzz', rows, (row) => row.path)).toEqual([])
    })

    test('여러 토큰의 점수는 각 토큰 점수의 합이다', () => {
        const path = 'src/widgets/editor-area/pane-node-view.tsx'
        const result = fuzzyFilter('widgets pane', rows, (row) => row.path)
        expect(result[0]?.match.score).toBe(fuzzyMatch('widgets', path)!.score + fuzzyMatch('pane', path)!.score)
    })

    test('여러 토큰의 인덱스는 정렬된 합집합이며 겹치는 토큰도 중복되지 않는다', () => {
        expect(fuzzyFilter('widgets pane', rows, (row) => row.path)[0]?.match.indices).toEqual([4, 5, 6, 7, 8, 9, 10, 24, 25, 26, 27])

        const overlappingPath = 'src/shared/lib/fuzzy-match.ts'
        expect(fuzzyFilter('fuzzy fu', rows, (row) => row.path)[0]?.match.indices).toEqual(fuzzyMatch('fuzzy', overlappingPath)!.indices)
    })

    test('앞뒤·연속 공백은 무시한다', () => {
        expect(fuzzyFilter('  widgets   pane  ', rows, (row) => row.path)).toEqual(fuzzyFilter('widgets pane', rows, (row) => row.path))
        expect(fuzzyFilter(' fuzzy ', rows, (row) => row.path)).toEqual(fuzzyFilter('fuzzy', rows, (row) => row.path))
        expect(fuzzyFilter('   ', rows, (row) => row.path).map((ranked) => ranked.item.path)).toEqual(rows.map((row) => row.path))
    })

    test('토큰 상한(8개)을 넘는 토큰은 무시한다', () => {
        expect(fuzzyFilter('s r c h l i b zzzz', rows, (row) => row.path)).toEqual([])
        expect(fuzzyFilter('s r c h l i b f zzzz', rows, (row) => row.path).map((ranked) => ranked.item.path)).toEqual([
            'src/shared/lib/fuzzy-match.ts',
        ])
    })

    test('단일 토큰 결과는 fuzzyMatch 결과 그대로다', () => {
        const path = 'src/shared/lib/fuzzy-match.ts'
        const result = fuzzyFilter('fuzzy', rows, (row) => row.path)
        expect(result.map((ranked) => ranked.item.path)).toEqual([path])
        expect(result[0]?.match).toEqual(fuzzyMatch('fuzzy', path)!)
    })
})

describe('fuzzyFilter 동점 정렬', () => {
    const rank = (query: string, paths: string[]) => fuzzyFilter(query, paths, (path) => path).map((ranked) => ranked.item)

    test('점수가 같으면 파일명 매치가 디렉토리 매치보다 앞선다', () => {
        const dirMatch = 'src/ab/x.ts'
        const fileNameMatch = 'src/x/ab.ts'

        expect(rank('ab', [dirMatch, fileNameMatch])).toEqual([fileNameMatch, dirMatch])
        expect(rank('ab', [fileNameMatch, dirMatch])).toEqual([fileNameMatch, dirMatch])
    })

    test('점수·매치 위치가 같으면 짧은 라벨이 앞선다', () => {
        expect(rank('ab', ['ab.tsx', 'ab.ts'])).toEqual(['ab.ts', 'ab.tsx'])
        expect(rank('ab', ['ab.ts', 'ab.tsx'])).toEqual(['ab.ts', 'ab.tsx'])
    })

    test('길이까지 같으면 라벨 사전순으로 고정된다', () => {
        expect(rank('ab', ['ab.ts', 'ab.js'])).toEqual(['ab.js', 'ab.ts'])
        expect(rank('ab', ['ab.js', 'ab.ts'])).toEqual(['ab.js', 'ab.ts'])
    })

    test('같은 후보 집합은 입력 순서가 달라도 같은 순위를 낸다 — 파일 순회 순서가 상한 커트라인을 바꾸지 않는다', () => {
        const paths = ['src/ab/x.ts', 'src/x/ab.ts', 'ab.ts', 'ab.js', 'a/b.ts']

        expect(rank('ab', paths)).toEqual(rank('ab', paths.toReversed()))
    })

    test('빈 질의는 정렬하지 않고 입력 순서를 그대로 돌려준다', () => {
        const paths = ['zzz.ts', 'a.ts', 'src/m.ts']

        expect(rank('', paths)).toEqual(paths)
        expect(rank('   ', paths)).toEqual(paths)
    })
})

describe('fuzzyFilter 유니코드 정규화', () => {
    const composedPath = 'src/한글/파일.ts'
    const decomposedPath = composedPath.normalize('NFD')

    test('NFD 라벨도 NFC 질의로 매칭된다', () => {
        expect(decomposedPath).not.toBe(composedPath)
        expect(fuzzyFilter('파일', [decomposedPath], (path) => path)).toHaveLength(1)
    })

    test('NFD 질의도 NFC 라벨을 찾는다', () => {
        expect(fuzzyFilter('파일'.normalize('NFD'), [composedPath], (path) => path)).toHaveLength(1)
    })

    test('item 은 원본 그대로 두고 label 만 NFC 정규화본이다 — 열기는 디스크가 준 경로로 한다', () => {
        const [ranked] = fuzzyFilter('파일', [decomposedPath], (path) => path)

        expect(ranked.item).toBe(decomposedPath)
        expect(ranked.label).toBe(composedPath)
    })

    test('매칭 인덱스는 label(정규화본) 기준이라 강조가 질의와 정확히 겹친다', () => {
        const [ranked] = fuzzyFilter('파일', [decomposedPath], (path) => path)

        expect(buildFuzzyHighlightSegments(ranked.label, ranked.match.indices)).toEqual([
            { text: 'src/한글/', matched: false },
            { text: '파일', matched: true },
            { text: '.ts', matched: false },
        ])
    })

    test('ASCII 라벨은 정규화를 거쳐도 동일 문자열이다', () => {
        const [ranked] = fuzzyFilter('pnv', ['pane-node-view.tsx'], (path) => path)

        expect(ranked.label).toBe('pane-node-view.tsx')
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

import { describe, expect, test } from 'bun:test'
import type { languages } from 'monaco-editor'
import { buildOutlineRows, findOutlineParentIndex } from '@features/outline/outline-rows'

const RANGE = { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 1 }

const symbol = (name: string, children: languages.DocumentSymbol[] = []): languages.DocumentSymbol => ({
    name,
    detail: '',
    kind: 4,
    tags: [],
    range: RANGE,
    selectionRange: RANGE,
    children,
})

const TREE = [symbol('A', [symbol('A1'), symbol('A2', [symbol('A2a')])]), symbol('B')]

describe('buildOutlineRows', () => {
    test('트리를 깊이 우선 순서의 평탄 배열로 만든다', () => {
        const rows = buildOutlineRows(TREE, new Set())

        expect(rows.map((row) => row.symbol.name)).toEqual(['A', 'A1', 'A2', 'A2a', 'B'])
        expect(rows.map((row) => row.depth)).toEqual([0, 1, 1, 2, 0])
    })

    test('자식 유무를 행에 담는다', () => {
        expect(buildOutlineRows(TREE, new Set()).map((row) => row.hasChildren)).toEqual([true, false, true, false, false])
    })

    test('접힌 행은 자기 자신만 남기고 자손 전체를 뺀다', () => {
        const rows = buildOutlineRows(TREE, new Set(['/0']))

        expect(rows.map((row) => row.symbol.name)).toEqual(['A', 'B'])
        expect(rows[0].collapsed).toBe(true)
    })

    test('중간 깊이를 접으면 그 아래만 빠진다', () => {
        expect(buildOutlineRows(TREE, new Set(['/0/1'])).map((row) => row.symbol.name)).toEqual(['A', 'A1', 'A2', 'B'])
    })

    test('id 는 트리 위치라서 이름이 같은 형제도 서로 다르다', () => {
        const rows = buildOutlineRows([symbol('overload'), symbol('overload')], new Set())

        expect(rows.map((row) => row.id)).toEqual(['/0', '/1'])
    })

    test('children 이 없는 심볼도 안전하게 처리한다', () => {
        const bare = { name: 'C', detail: '', kind: 4, tags: [], range: RANGE, selectionRange: RANGE } satisfies languages.DocumentSymbol

        expect(buildOutlineRows([bare], new Set())).toEqual([{ id: '/0', symbol: bare, depth: 0, hasChildren: false, collapsed: false }])
    })

    test('심볼이 없으면 빈 배열이다', () => {
        expect(buildOutlineRows([], new Set())).toEqual([])
    })
})

describe('findOutlineParentIndex', () => {
    test('한 단계 바깥의 가장 가까운 행을 찾는다', () => {
        const rows = buildOutlineRows(TREE, new Set())

        expect(findOutlineParentIndex(rows, 3)).toBe(2)
        expect(findOutlineParentIndex(rows, 2)).toBe(0)
        expect(findOutlineParentIndex(rows, 1)).toBe(0)
    })

    test('최상위 행은 부모가 없다', () => {
        const rows = buildOutlineRows(TREE, new Set())

        expect(findOutlineParentIndex(rows, 0)).toBe(-1)
        expect(findOutlineParentIndex(rows, 4)).toBe(-1)
    })

    test('범위를 벗어난 인덱스는 -1 이다', () => {
        expect(findOutlineParentIndex(buildOutlineRows(TREE, new Set()), 99)).toBe(-1)
    })
})

import { describe, expect, test } from 'bun:test'
import {
    buildGitChangeListRows,
    GIT_ROVING_INDEX_ATTRIBUTE,
    gitChangeListHeaderIndexes,
    gitRovingItemSelector,
    parseGitRovingIndex,
    resolveNextChangeRowIndex,
    resolveStickyHeaderIndex,
} from '@widgets/git-panel/change-row-navigation'

const section = (id: 'merge' | 'staged' | 'changes', rowKeys: string[], overrides: { visible?: boolean; collapsed?: boolean } = {}) => ({
    id,
    visible: overrides.visible ?? rowKeys.length > 0,
    collapsed: overrides.collapsed ?? false,
    rowKeys,
})

describe('buildGitChangeListRows', () => {
    test('보이는 섹션마다 헤더 1행 + 행들을 순서대로 평탄화한다', () => {
        const rows = buildGitChangeListRows([section('staged', ['a.ts']), section('changes', ['b.ts', 'c.ts'])])

        expect(rows.map((row) => row.kind)).toEqual(['header', 'row', 'header', 'row', 'row'])
        expect(rows.map((row) => row.section)).toEqual(['staged', 'staged', 'changes', 'changes', 'changes'])
        expect(rows.filter((row) => row.kind === 'row').map((row) => row.rowIndex)).toEqual([0, 0, 1])
    })

    test('보이지 않는 섹션은 헤더도 내지 않는다', () => {
        expect(buildGitChangeListRows([section('merge', []), section('changes', ['b.ts'])]).map((row) => row.id)).toEqual([
            'header:changes',
            'row:changes:b.ts',
        ])
    })

    test('접힌 섹션은 헤더만 남기고 행을 빼지만 뒤 섹션은 그대로 이어진다', () => {
        const rows = buildGitChangeListRows([section('staged', ['a.ts'], { collapsed: true }), section('changes', ['b.ts'])])

        expect(rows.map((row) => row.id)).toEqual(['header:staged', 'header:changes', 'row:changes:b.ts'])
    })

    test('같은 경로가 두 그룹에 있어도 행 id 가 겹치지 않는다', () => {
        const rows = buildGitChangeListRows([section('staged', ['a.ts']), section('changes', ['a.ts'])])

        expect(new Set(rows.map((row) => row.id)).size).toBe(rows.length)
    })

    test('행이 하나도 없으면 빈 배열이다', () => {
        expect(buildGitChangeListRows([section('merge', []), section('staged', []), section('changes', [])])).toEqual([])
    })
})

describe('gitChangeListHeaderIndexes', () => {
    test('헤더 위치만 오름차순으로 모은다', () => {
        const rows = buildGitChangeListRows([section('merge', ['m.ts']), section('staged', ['a.ts', 'b.ts']), section('changes', ['c.ts'])])

        expect(gitChangeListHeaderIndexes(rows)).toEqual([0, 2, 5])
    })
})

describe('resolveStickyHeaderIndex', () => {
    test('첫 보이는 행 이상에서 가장 가까운 헤더를 고른다', () => {
        expect(resolveStickyHeaderIndex([0, 2, 5], 0)).toBe(0)
        expect(resolveStickyHeaderIndex([0, 2, 5], 1)).toBe(0)
        expect(resolveStickyHeaderIndex([0, 2, 5], 2)).toBe(2)
        expect(resolveStickyHeaderIndex([0, 2, 5], 4)).toBe(2)
        expect(resolveStickyHeaderIndex([0, 2, 5], 5)).toBe(5)
        expect(resolveStickyHeaderIndex([0, 2, 5], 9)).toBe(5)
    })

    test('헤더가 없으면 -1 이다', () => {
        expect(resolveStickyHeaderIndex([], 3)).toBe(-1)
    })
})

describe('parseGitRovingIndex', () => {
    test('숫자 문자열만 인덱스로 인정한다', () => {
        expect(parseGitRovingIndex('0')).toBe(0)
        expect(parseGitRovingIndex('12')).toBe(12)
    })

    test('없거나 인덱스가 아닌 값은 -1 이다', () => {
        expect(parseGitRovingIndex(undefined)).toBe(-1)
        expect(parseGitRovingIndex(null)).toBe(-1)
        expect(parseGitRovingIndex('')).toBe(-1)
        expect(parseGitRovingIndex('abc')).toBe(-1)
        expect(parseGitRovingIndex('-1')).toBe(-1)
        expect(parseGitRovingIndex('1.5')).toBe(-1)
    })
})

describe('gitRovingItemSelector', () => {
    test('인덱스 속성 셀렉터를 만든다', () => {
        expect(gitRovingItemSelector(3)).toBe(`[${GIT_ROVING_INDEX_ATTRIBUTE}="3"]`)
    })
})

describe('resolveNextChangeRowIndex', () => {
    test('행이 없으면 -1 을 반환한다', () => {
        expect(resolveNextChangeRowIndex('ArrowDown', -1, 0)).toBe(-1)
        expect(resolveNextChangeRowIndex('ArrowUp', 2, 0)).toBe(-1)
    })

    test('포커스가 행 밖이면 ArrowDown 은 첫 행, ArrowUp 은 마지막 행으로 진입한다', () => {
        expect(resolveNextChangeRowIndex('ArrowDown', -1, 3)).toBe(0)
        expect(resolveNextChangeRowIndex('ArrowUp', -1, 3)).toBe(2)
    })

    test('ArrowDown 은 다음 행으로, ArrowUp 은 이전 행으로 이동한다', () => {
        expect(resolveNextChangeRowIndex('ArrowDown', 0, 3)).toBe(1)
        expect(resolveNextChangeRowIndex('ArrowUp', 2, 3)).toBe(1)
    })

    test('양 끝에서는 순환하지 않고 멈춘다', () => {
        expect(resolveNextChangeRowIndex('ArrowDown', 2, 3)).toBe(2)
        expect(resolveNextChangeRowIndex('ArrowUp', 0, 3)).toBe(0)
    })

    test('헤더와 행이 섞인 시퀀스에서도 한 칸씩 이동한다 — 헤더 다음은 그 섹션의 첫 행', () => {
        expect(resolveNextChangeRowIndex('ArrowDown', 0, 5)).toBe(1)
        expect(resolveNextChangeRowIndex('ArrowDown', 2, 5)).toBe(3)
        expect(resolveNextChangeRowIndex('ArrowUp', 3, 5)).toBe(2)
    })

    test('가상화 목록 뒤의 정적 헤더(스태시·그래프)까지 인덱스가 이어진다', () => {
        const changeListLength = 4
        const rovingItemCount = changeListLength + 2

        expect(resolveNextChangeRowIndex('ArrowDown', changeListLength - 1, rovingItemCount)).toBe(4)
        expect(resolveNextChangeRowIndex('ArrowDown', 4, rovingItemCount)).toBe(5)
        expect(resolveNextChangeRowIndex('ArrowDown', 5, rovingItemCount)).toBe(5)
        expect(resolveNextChangeRowIndex('ArrowUp', 4, rovingItemCount)).toBe(3)
    })
})

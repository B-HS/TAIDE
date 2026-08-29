import { describe, expect, test } from 'bun:test'
import { resolveNextChangeRowIndex } from '@widgets/git-panel/change-row-navigation'

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
})

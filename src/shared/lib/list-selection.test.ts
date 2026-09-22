import { describe, expect, test } from 'bun:test'
import { resolveListSelection } from '@shared/lib/list-selection'

const IDS = ['a', 'b', 'c', 'd']

describe('resolveListSelection', () => {
    test('일반 클릭은 기존 선택을 클릭한 항목 하나로 바꾼다', () => {
        const result = resolveListSelection({
            orderedIds: IDS,
            selectedIds: new Set(['a', 'c']),
            anchorId: 'a',
            clickedId: 'b',
            shiftKey: false,
            additiveKey: false,
        })

        expect([...result.selectedIds]).toEqual(['b'])
        expect(result.anchorId).toBe('b')
        expect(result.primaryId).toBe('b')
    })

    test('Command/Ctrl 클릭은 항목을 기존 선택에 추가하거나 해제한다', () => {
        const added = resolveListSelection({
            orderedIds: IDS,
            selectedIds: new Set(['a']),
            anchorId: 'a',
            clickedId: 'c',
            shiftKey: false,
            additiveKey: true,
        })
        const removed = resolveListSelection({
            orderedIds: IDS,
            selectedIds: added.selectedIds,
            anchorId: added.anchorId,
            clickedId: 'c',
            shiftKey: false,
            additiveKey: true,
        })

        expect([...added.selectedIds]).toEqual(['a', 'c'])
        expect([...removed.selectedIds]).toEqual(['a'])
        expect(removed.primaryId).toBe('a')
    })

    test('Shift 클릭은 anchor 와 클릭 항목 사이의 연속 범위로 바꾼다', () => {
        const result = resolveListSelection({
            orderedIds: IDS,
            selectedIds: new Set(['a']),
            anchorId: 'b',
            clickedId: 'd',
            shiftKey: true,
            additiveKey: false,
        })

        expect([...result.selectedIds]).toEqual(['b', 'c', 'd'])
        expect(result.anchorId).toBe('b')
        expect(result.primaryId).toBe('d')
    })

    test('Command/Ctrl+Shift 클릭은 기존 선택에 범위를 더한다', () => {
        const result = resolveListSelection({
            orderedIds: IDS,
            selectedIds: new Set(['a']),
            anchorId: 'c',
            clickedId: 'd',
            shiftKey: true,
            additiveKey: true,
        })

        expect([...result.selectedIds]).toEqual(['a', 'c', 'd'])
    })

    test('목록에서 사라진 선택은 다음 선택 계산에서 제거한다', () => {
        const result = resolveListSelection({
            orderedIds: IDS,
            selectedIds: new Set(['gone', 'a']),
            anchorId: 'gone',
            clickedId: 'c',
            shiftKey: true,
            additiveKey: false,
        })

        expect([...result.selectedIds]).toEqual(['c'])
        expect(result.anchorId).toBe('c')
    })
})

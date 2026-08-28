import { describe, expect, test } from 'bun:test'
import { commitDragHsv, nextDragHsvFromHuePointer, nextDragHsvFromSquarePointer } from '@features/theme/color-picker-drag'

describe('nextDragHsvFromSquarePointer', () => {
    test('드래그 첫 샘플(prev 없음)에는 마지막 커밋된 휴를 사용한다', () => {
        expect(nextDragHsvFromSquarePointer(null, 200, 0.5, 0.5)).toEqual({ h: 200, s: 0.5, v: 0.5 })
    })

    test('드래그 도중에는 커밋된 휴 대신 드래그-로컬 휴를 유지한다', () => {
        const prev = { h: 120, s: 0.2, v: 0.2 }
        expect(nextDragHsvFromSquarePointer(prev, 200, 0.7, 0.3)).toEqual({ h: 120, s: 0.7, v: 0.3 })
    })
})

describe('nextDragHsvFromHuePointer', () => {
    test('드래그 첫 샘플(prev 없음)에는 마지막 커밋된 채도/명도를 사용한다', () => {
        expect(nextDragHsvFromHuePointer(null, 0.4, 0.6, 90)).toEqual({ h: 90, s: 0.4, v: 0.6 })
    })

    test('드래그 도중에는 커밋된 채도/명도 대신 드래그-로컬 값을 유지한다', () => {
        const prev = { h: 10, s: 0.9, v: 0.1 }
        expect(nextDragHsvFromHuePointer(prev, 0.4, 0.6, 300)).toEqual({ h: 300, s: 0.9, v: 0.1 })
    })
})

describe('commitDragHsv', () => {
    test('드래그-로컬 HSV 를 헥스로 커밋한다', () => {
        expect(commitDragHsv({ h: 0, s: 1, v: 1 })).toBe('#ff0000')
    })

    test('드래그 중이 아니면(null) 커밋하지 않는다', () => {
        expect(commitDragHsv(null)).toBeNull()
    })
})

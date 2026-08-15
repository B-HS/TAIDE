import { describe, expect, test } from 'bun:test'
import { toggleInSet } from '@shared/lib/set'

describe('toggleInSet', () => {
    test('없는 값이면 추가한다', () => {
        const result = toggleInSet(new Set(['a']), 'b')
        expect(result.has('b')).toBe(true)
        expect(result.size).toBe(2)
    })

    test('있는 값이면 제거한다', () => {
        const result = toggleInSet(new Set(['a', 'b']), 'b')
        expect(result.has('b')).toBe(false)
        expect(result.size).toBe(1)
    })

    test('원본 Set을 변경하지 않는다', () => {
        const original = new Set(['a'])
        toggleInSet(original, 'b')
        expect(original.has('b')).toBe(false)
    })
})

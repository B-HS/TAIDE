import { describe, expect, test } from 'bun:test'
import { cn } from '@shared/lib/cn'

describe('cn', () => {
    test('falsy 값을 제거하고 클래스를 합친다', () => {
        expect(cn('a', false, undefined, 'b')).toBe('a b')
    })

    test('같은 그룹의 tailwind 클래스는 뒤엣것이 이긴다', () => {
        expect(cn('px-2', 'px-4')).toBe('px-4')
    })

    test('조건부 객체 표기를 지원한다', () => {
        expect(cn({ 'text-red-500': true, 'text-blue-500': false })).toBe('text-red-500')
    })
})

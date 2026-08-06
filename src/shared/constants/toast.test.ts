import { describe, expect, test } from 'bun:test'
import { isMiddleToastPosition, parseToastPosition, toSonnerPosition } from '@shared/constants/toast'

describe('parseToastPosition', () => {
    test('9분할 값을 세로·가로로 나눈다', () => {
        expect(parseToastPosition('top-left')).toEqual({ vertical: 'top', horizontal: 'left' })
        expect(parseToastPosition('middle-center')).toEqual({ vertical: 'middle', horizontal: 'center' })
    })

    test('알 수 없는 값은 기본값으로 폴백한다', () => {
        expect(parseToastPosition('nowhere')).toEqual({ vertical: 'bottom', horizontal: 'right' })
    })
})

describe('toSonnerPosition', () => {
    test('sonner 가 지원하는 6종은 그대로 넘긴다', () => {
        expect(toSonnerPosition('top-center')).toBe('top-center')
        expect(toSonnerPosition('bottom-left')).toBe('bottom-left')
    })

    test('중간 행은 top 으로 앵커하고 CSS 로 보정한다', () => {
        expect(toSonnerPosition('middle-right')).toBe('top-right')
        expect(isMiddleToastPosition('middle-right')).toBe(true)
        expect(isMiddleToastPosition('bottom-right')).toBe(false)
    })
})

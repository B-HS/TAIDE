import { describe, expect, test } from 'bun:test'
import { isWindowReadyToReveal } from '@shared/hooks/use-reveal-window'

describe('isWindowReadyToReveal', () => {
    test('테마·로케일 쿼리가 모두 fetch 완료되면 true 를 반환한다', () => {
        expect(isWindowReadyToReveal(true, true)).toBe(true)
    })

    test('테마만 fetch 완료되고 로케일이 아직이면 false 를 반환한다(CSS 게이트와 정합)', () => {
        expect(isWindowReadyToReveal(true, false)).toBe(false)
    })

    test('로케일만 fetch 완료되고 테마가 아직이면 false 를 반환한다', () => {
        expect(isWindowReadyToReveal(false, true)).toBe(false)
    })

    test('둘 다 아직이면 false 를 반환한다', () => {
        expect(isWindowReadyToReveal(false, false)).toBe(false)
    })

    test('쿼리가 에러로 확정돼도 isFetched 는 true 이므로 reveal 데드락이 없다', () => {
        const isThemeFetchedAfterError = true
        const isLocaleFetchedAfterError = true
        expect(isWindowReadyToReveal(isThemeFetchedAfterError, isLocaleFetchedAfterError)).toBe(true)
    })
})

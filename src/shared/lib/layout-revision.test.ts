import { describe, expect, test } from 'bun:test'
import { isStaleLayoutRevision } from '@shared/lib/layout-revision'

describe('isStaleLayoutRevision', () => {
    test('아직 관측된 적 없으면(undefined) 무엇이 와도 stale 이 아니다', () => {
        expect(isStaleLayoutRevision(undefined, 1)).toBe(false)
        expect(isStaleLayoutRevision(undefined, 0)).toBe(false)
    })

    test('마지막 관측보다 큰 revision 은 stale 이 아니다', () => {
        expect(isStaleLayoutRevision(5, 6)).toBe(false)
    })

    test('마지막 관측보다 낮거나 같은 revision 은 stale 이다 (중복·역전 이벤트 무시)', () => {
        expect(isStaleLayoutRevision(5, 5)).toBe(true)
        expect(isStaleLayoutRevision(5, 4)).toBe(true)
    })
})

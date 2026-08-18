import { describe, expect, test } from 'bun:test'
import { isValidAllowedHost } from '@features/settings/remote-allowed-hosts-row'

describe('isValidAllowedHost', () => {
    test('일반 호스트명을 허용한다', () => {
        expect(isValidAllowedHost('tunnel.example.com')).toBe(true)
    })

    test('단일 레이블 와일드카드를 허용한다', () => {
        expect(isValidAllowedHost('*.trycloudflare.com')).toBe(true)
    })

    test('맨몸 와일드카드는 거부한다', () => {
        expect(isValidAllowedHost('*')).toBe(false)
    })

    test('접미사가 한 레이블뿐인 와일드카드는 거부한다', () => {
        expect(isValidAllowedHost('*.com')).toBe(false)
    })

    test('선두가 아닌 와일드카드는 거부한다', () => {
        expect(isValidAllowedHost('foo.*.com')).toBe(false)
    })

    test('레이블에 유사 결합된 와일드카드는 거부한다', () => {
        expect(isValidAllowedHost('*foo.com')).toBe(false)
    })
})

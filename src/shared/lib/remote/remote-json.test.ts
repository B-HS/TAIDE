import { describe, expect, test } from 'bun:test'
import { isRecord, numberOf, parseJson, stringOf } from '@shared/lib/remote/remote-json'

/**
 * The remote page decodes websocket frames it did not type-check, so these four are the only place
 * an unknown value is narrowed. Each test pins the *lenient* half of the contract as much as the
 * strict one: a frame that is malformed must degrade to a neutral value rather than throw, because
 * a throw inside the socket's message handler tears down the whole remote session.
 */
describe('isRecord', () => {
    test('객체는 true, null 은 false 다 (typeof null === object 함정)', () => {
        expect(isRecord({})).toBe(true)
        expect(isRecord({ a: 1 })).toBe(true)
        expect(isRecord(null)).toBe(false)
    })

    test('배열도 객체이므로 true 다 — 이 가드는 "인덱싱 가능" 만 보장한다', () => {
        expect(isRecord([])).toBe(true)
    })

    test('원시값과 함수는 false 다', () => {
        expect(isRecord('{}')).toBe(false)
        expect(isRecord(0)).toBe(false)
        expect(isRecord(undefined)).toBe(false)
        expect(isRecord(() => undefined)).toBe(false)
    })
})

describe('parseJson', () => {
    test('유효한 JSON 은 그대로 파싱한다', () => {
        expect(parseJson('{"status":"ok"}')).toEqual({ status: 'ok' })
        expect(parseJson('[1,2]')).toEqual([1, 2])
        expect(parseJson('3')).toBe(3)
    })

    test('깨진 JSON 은 throw 하지 않고 null 을 돌려준다', () => {
        expect(parseJson('{')).toBeNull()
        expect(parseJson('')).toBeNull()
        expect(parseJson('undefined')).toBeNull()
    })

    test('리터럴 null 과 파싱 실패는 같은 값으로 수렴한다 (호출부가 둘을 구분하지 않는다)', () => {
        expect(parseJson('null')).toBeNull()
    })
})

describe('numberOf', () => {
    test('number 는 그대로, 그 외 타입은 0 이다', () => {
        expect(numberOf(12)).toBe(12)
        expect(numberOf(0)).toBe(0)
        expect(numberOf(-1.5)).toBe(-1.5)
        expect(numberOf('12')).toBe(0)
        expect(numberOf(null)).toBe(0)
        expect(numberOf(undefined)).toBe(0)
        expect(numberOf(true)).toBe(0)
    })

    test('NaN·Infinity 도 typeof number 이므로 통과한다 (가드는 타입만 본다)', () => {
        expect(Number.isNaN(numberOf(Number.NaN))).toBe(true)
        expect(numberOf(Number.POSITIVE_INFINITY)).toBe(Number.POSITIVE_INFINITY)
    })
})

describe('stringOf', () => {
    test('string 은 그대로, 그 외 타입은 빈 문자열이다', () => {
        expect(stringOf('ok')).toBe('ok')
        expect(stringOf('')).toBe('')
        expect(stringOf(12)).toBe('')
        expect(stringOf(null)).toBe('')
        expect(stringOf(undefined)).toBe('')
        expect(stringOf({ toString: () => 'coerced' })).toBe('')
    })
})

import { describe, expect, test } from 'bun:test'
import { isOpaqueForegroundCandidate } from '@shared/lib/theme-convert/mapping-tables'

describe('isOpaqueForegroundCandidate', () => {
    test('4자리 #rgba 축약형은 마지막 알파 니블이 f 미만이면 반투명으로 배제하고, f 면 불투명으로 통과시킨다', () => {
        expect(isOpaqueForegroundCandidate('#fd03')).toBe(false)
        expect(isOpaqueForegroundCandidate('#fd0f')).toBe(true)
    })
})

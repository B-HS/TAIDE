import { describe, expect, test } from 'bun:test'
import { isDistinctFromBodyForeground, isOpaqueForegroundCandidate } from '@shared/lib/theme-convert/mapping-tables'

describe('isOpaqueForegroundCandidate', () => {
    test('4자리 #rgba 축약형은 마지막 알파 니블이 f 미만이면 반투명으로 배제하고, f 면 불투명으로 통과시킨다', () => {
        expect(isOpaqueForegroundCandidate('#fd03')).toBe(false)
        expect(isOpaqueForegroundCandidate('#fd0f')).toBe(true)
    })
})

describe('isDistinctFromBodyForeground', () => {
    test('본문 전경과 완전히 동일한 색(ΔE 0, monokai/night-owl-light/palenight 실사례)은 구별되지 않는다고 판정한다', () => {
        expect(isDistinctFromBodyForeground('#f8f8f2', '#f8f8f2')).toBe(false)
        expect(isDistinctFromBodyForeground('#403f53', '#403f53')).toBe(false)
        expect(isDistinctFromBodyForeground('#ffffff', '#ffffff')).toBe(false)
    })

    test('본문 전경과 지각적으로 가까운 값(ΔE 5.4, one-monokai 실사례)은 구별된다고 판정한다 — WCAG 대비비가 아니라 ΔE 임계 기준', () => {
        expect(isDistinctFromBodyForeground('#C5C5C5', '#D4D4D4')).toBe(true)
    })

    test('WCAG 대비비로는 거의 동일해 보이지만(github-dark 실사례, 대비비 ~1.03) 색상이 뚜렷이 다르면(ΔE 77.7) 구별된다고 판정한다', () => {
        expect(isDistinctFromBodyForeground('#ffd33d', '#d1d5da')).toBe(true)
    })
})

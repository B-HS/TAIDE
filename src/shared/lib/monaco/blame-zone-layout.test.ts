import { describe, expect, test } from 'bun:test'
import { computeBlameZoneAfterLineNumber, computeBlameZoneFontSize, computeBlameZoneHeightPx } from '@shared/lib/monaco/blame-zone-layout'

describe('computeBlameZoneFontSize', () => {
    test('에디터 폰트 크기의 90%를 정수로 내림한다', () => {
        expect(computeBlameZoneFontSize(14)).toBe(12)
        expect(computeBlameZoneFontSize(13)).toBe(11)
    })
})

describe('computeBlameZoneHeightPx', () => {
    test('줄 높이 비율이 최소값(1.3)보다 크면 그 비율을 그대로 사용한다', () => {
        expect(computeBlameZoneHeightPx(14, 21)).toBe(18)
    })

    test('줄 높이 비율이 최소값(1.3)보다 작으면 1.3으로 하한한다', () => {
        expect(computeBlameZoneHeightPx(14, 14)).toBe(15)
    })

    test('폰트 크기가 커지면 존 높이도 함께 커진다', () => {
        expect(computeBlameZoneHeightPx(20, 30)).toBe(27)
    })
})

describe('computeBlameZoneAfterLineNumber', () => {
    test('일반 줄은 line - 1을 반환한다', () => {
        expect(computeBlameZoneAfterLineNumber(5)).toBe(4)
    })

    test('첫 줄(line=1)은 0을 반환해 첫 줄 앞에 배치한다', () => {
        expect(computeBlameZoneAfterLineNumber(1)).toBe(0)
    })

    test('음수·0 입력이 들어와도 0 밑으로 내려가지 않는다', () => {
        expect(computeBlameZoneAfterLineNumber(0)).toBe(0)
        expect(computeBlameZoneAfterLineNumber(-3)).toBe(0)
    })
})

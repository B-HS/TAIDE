import { describe, expect, test } from 'bun:test'
import { hexToHsv, hexToRgb, hsvToHex, isValidThemeColorValue, normalizeHexColor, rgbToHex, rgbToHsv } from '@shared/lib/color'

describe('isValidThemeColorValue', () => {
    test('3자리 hex 를 허용한다', () => {
        expect(isValidThemeColorValue('#fff')).toBe(true)
    })

    test('6자리 hex 를 허용한다', () => {
        expect(isValidThemeColorValue('#1e1e2e')).toBe(true)
    })

    test('알파 채널 포함 8자리 hex 를 허용한다', () => {
        expect(isValidThemeColorValue('#a6e3a133')).toBe(true)
    })

    test('transparent 키워드를 허용한다', () => {
        expect(isValidThemeColorValue('transparent')).toBe(true)
        expect(isValidThemeColorValue('Transparent')).toBe(true)
    })

    test('해시가 없거나 형식이 틀리면 거부한다', () => {
        expect(isValidThemeColorValue('1e1e2e')).toBe(false)
        expect(isValidThemeColorValue('#12345')).toBe(false)
        expect(isValidThemeColorValue('#gggggg')).toBe(false)
        expect(isValidThemeColorValue('')).toBe(false)
    })
})

describe('normalizeHexColor', () => {
    test('대문자 hex 를 소문자로 정규화한다', () => {
        expect(normalizeHexColor('#ABCDEF')).toBe('#abcdef')
    })

    test('유효하지 않으면 null 을 반환한다', () => {
        expect(normalizeHexColor('not-a-color')).toBeNull()
    })
})

describe('hexToRgb / rgbToHex', () => {
    test('6자리 hex 를 rgb 로 변환한다', () => {
        expect(hexToRgb('#89b4fa')).toEqual({ r: 0x89, g: 0xb4, b: 0xfa })
    })

    test('3자리 hex 를 확장해서 변환한다', () => {
        expect(hexToRgb('#fff')).toEqual({ r: 255, g: 255, b: 255 })
    })

    test('rgb 를 hex 로 되돌린다', () => {
        expect(rgbToHex({ r: 137, g: 180, b: 250 })).toBe('#89b4fa')
    })

    test('유효하지 않은 값은 null 을 반환한다', () => {
        expect(hexToRgb('nope')).toBeNull()
    })
})

describe('rgbToHsv / hsvToHex 원점 왕복', () => {
    test('여러 색상이 hex 왕복 변환 후 동일하다', () => {
        for (const hex of ['#89b4fa', '#1e1e2e', '#ffffff', '#000000', '#f38ba8']) {
            const hsv = hexToHsv(hex)
            expect(hsv).not.toBeNull()
            expect(hsvToHex(hsv!)).toBe(hex)
        }
    })

    test('빨강의 hsv 값이 기대대로 나온다', () => {
        expect(rgbToHsv({ r: 255, g: 0, b: 0 })).toEqual({ h: 0, s: 1, v: 1 })
    })

    test('검정은 채도·명도 모두 0이다', () => {
        expect(rgbToHsv({ r: 0, g: 0, b: 0 })).toEqual({ h: 0, s: 0, v: 0 })
    })
})

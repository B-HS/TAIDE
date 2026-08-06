import { describe, expect, test } from 'bun:test'
import type { ResolvedTheme } from '@shared/api/bindings'
import {
    buildMonacoThemeData,
    buildThemeColors,
    buildTokenRules,
    InvalidHexColorError,
    toMonacoFontStyle,
    toRuleForeground,
    toThemeColor,
} from '@shared/lib/monaco/theme'

const baseTheme: ResolvedTheme = {
    id: 'taide-dark',
    name: 'TAIDE Dark',
    type: 'dark',
    colors: {
        'editor.background': '#1e1e2e',
        'editor.foreground': '#cdd6f4',
        'editor.findMatchHighlight': '#fab38766',
        'appSidebar.background': '#181825',
    },
    syntax: {
        keyword: { fg: '#cba6f7' },
        comment: { fg: '#6c7086', italic: true },
        constant: { fg: '#fab387', bold: true, italic: true },
    },
    terminal: {},
}

describe('toRuleForeground', () => {
    test('6자리 hex 를 # 없이 반환한다', () => {
        expect(toRuleForeground('#cba6f7')).toBe('cba6f7')
    })

    test('# 없는 값은 예외를 던진다', () => {
        expect(() => toRuleForeground('cba6f7')).toThrow(InvalidHexColorError)
    })

    test('8자리(alpha) hex 는 룰 전경색으로 허용하지 않는다', () => {
        expect(() => toRuleForeground('#fab38766')).toThrow(InvalidHexColorError)
    })

    test('색 이름·rgb() 등 hex 가 아닌 값은 예외를 던진다', () => {
        expect(() => toRuleForeground('red')).toThrow(InvalidHexColorError)
        expect(() => toRuleForeground('rgb(0,0,0)')).toThrow(InvalidHexColorError)
    })
})

describe('toThemeColor', () => {
    test('6자리 hex 를 그대로 반환한다', () => {
        expect(toThemeColor('#1e1e2e')).toBe('#1e1e2e')
    })

    test('8자리(alpha) hex 를 허용한다', () => {
        expect(toThemeColor('#fab38766')).toBe('#fab38766')
    })

    test('CSS 변수는 예외를 던진다', () => {
        expect(() => toThemeColor('var(--editor-background)')).toThrow(InvalidHexColorError)
    })
})

describe('toMonacoFontStyle', () => {
    test('bold 와 italic 을 공백으로 합친다', () => {
        expect(toMonacoFontStyle({ fg: '#fff', bold: true, italic: true })).toBe('bold italic')
    })

    test('bold 만 있으면 bold 만 반환한다', () => {
        expect(toMonacoFontStyle({ fg: '#fff', bold: true })).toBe('bold')
    })

    test('스타일이 없으면 undefined 를 반환한다', () => {
        expect(toMonacoFontStyle({ fg: '#fff' })).toBeUndefined()
    })
})

describe('buildTokenRules', () => {
    test('syntax 토큰을 Monaco 룰 배열로 변환한다', () => {
        const rules = buildTokenRules(baseTheme.syntax)
        expect(rules).toContainEqual({ token: 'keyword', foreground: 'cba6f7' })
        expect(rules).toContainEqual({ token: 'comment', foreground: '6c7086', fontStyle: 'italic' })
        expect(rules).toContainEqual({ token: 'constant', foreground: 'fab387', fontStyle: 'bold italic' })
    })
})

describe('buildThemeColors', () => {
    test('editor 네임스페이스 토큰만 Monaco colorId 로 변환한다', () => {
        const colors = buildThemeColors(baseTheme.colors)
        expect(colors['editor.background']).toBe('#1e1e2e')
        expect(colors['editor.foreground']).toBe('#cdd6f4')
        expect(colors['editor.findMatchHighlightBackground']).toBe('#fab38766')
    })

    test('Monaco 가 모르는 앱 전용 네임스페이스는 포함하지 않는다', () => {
        const colors = buildThemeColors(baseTheme.colors)
        expect(colors['appSidebar.background']).toBeUndefined()
        expect(Object.keys(colors)).not.toContain('appSidebar.background')
    })

    test('테마에 없는 색은 결과에서 생략한다', () => {
        const colors = buildThemeColors({ 'editor.background': '#1e1e2e' })
        expect(Object.keys(colors)).toEqual(['editor.background'])
    })
})

describe('buildMonacoThemeData', () => {
    test('dark 테마는 vs-dark 를 base 로 사용한다', () => {
        expect(buildMonacoThemeData(baseTheme).base).toBe('vs-dark')
    })

    test('light 테마는 vs 를 base 로 사용한다', () => {
        expect(buildMonacoThemeData({ ...baseTheme, type: 'light' }).base).toBe('vs')
    })

    test('inherit 는 항상 true 다', () => {
        expect(buildMonacoThemeData(baseTheme).inherit).toBe(true)
    })
})

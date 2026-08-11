import { describe, expect, test } from 'bun:test'
import { convertVscodeTheme } from '@shared/lib/theme-convert/convert'

const MINIMAL_DARK_SOURCE = {
    colors: {
        'editor.background': '#1e1e1e',
        'editor.foreground': '#d4d4d4',
        foreground: '#d4d4d4',
    },
}

describe('convertVscodeTheme', () => {
    test('최소 입력으로도 133 색상 전량을 폴백으로 채워 ok 상태를 반환한다', () => {
        const result = convertVscodeTheme([MINIMAL_DARK_SOURCE], 'dark')

        expect(result.status).toBe('ok')
        expect(result.missingColors).toEqual([])
        expect(result.missingSyntax).toEqual([])
        expect(result.missingTerminal).toEqual([])
        expect(result.outputColorErrors).toEqual([])
        expect(result.colors['editor.background']).toBe('#1e1e1e')
        expect(result.colors['editor.foreground']).toBe('#d4d4d4')
    })

    test('원본에 없는 토큰은 safe-default 경고로 기록된다', () => {
        const result = convertVscodeTheme([MINIMAL_DARK_SOURCE], 'dark')
        expect(result.safeDefaultNotices.length).toBeGreaterThan(0)
    })

    test('terminal.ansi* 가 전혀 없으면 VS Code 기본 ANSI 16색 전량이 폴백 목록에 오른다', () => {
        const result = convertVscodeTheme([MINIMAL_DARK_SOURCE], 'dark')
        expect(result.ansiFallbackTokens).toHaveLength(16)
        expect(result.terminal.green).toBe('#0dbc79')
    })

    test('include 체인은 base 를 먼저, 가장 구체적인 항목을 마지막에 병합한다 (나중 항목이 이긴다)', () => {
        const base = { colors: { 'editor.background': '#000000', 'editor.foreground': '#d4d4d4', foreground: '#d4d4d4' } }
        const override = { colors: { 'editor.background': '#111111' } }

        const result = convertVscodeTheme([base, override], 'dark')

        expect(result.colors['editor.background']).toBe('#111111')
    })

    test('tokenColors 의 가장 구체적인 scope 가 우선한다', () => {
        const source = {
            ...MINIMAL_DARK_SOURCE,
            tokenColors: [
                { scope: 'keyword', settings: { foreground: '#ff0000' } },
                { scope: 'keyword.control', settings: { foreground: '#00ff00' } },
            ],
        }

        const result = convertVscodeTheme([source], 'dark')

        expect(result.syntax.keyword.fg).toBe('#00ff00')
    })

    test('한 파일 안에서 동일 scope 를 중복 정의하면 나중 규칙이 이긴다', () => {
        const source = {
            ...MINIMAL_DARK_SOURCE,
            tokenColors: [
                { scope: 'keyword.control', settings: { foreground: '#ff0000' } },
                { scope: 'keyword.control', settings: { foreground: '#00ff00' } },
            ],
        }

        const result = convertVscodeTheme([source], 'dark')

        expect(result.syntax.keyword.fg).toBe('#00ff00')
    })

    test('include 체인에서 leaf 가 base 와 동일 scope 를 재정의하면 leaf 가 이긴다', () => {
        const base = { ...MINIMAL_DARK_SOURCE, tokenColors: [{ scope: 'keyword.control', settings: { foreground: '#ff0000' } }] }
        const leaf = { tokenColors: [{ scope: 'keyword.control', settings: { foreground: '#00ff00' } }] }

        const result = convertVscodeTheme([base, leaf], 'dark')

        expect(result.syntax.keyword.fg).toBe('#00ff00')
    })
})

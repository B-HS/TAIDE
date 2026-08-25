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

    test('tokenColors 는 fontStyle 원문 문자열을 그대로 보존한다', () => {
        const source = {
            ...MINIMAL_DARK_SOURCE,
            tokenColors: [{ scope: 'comment', settings: { foreground: '#6a9955', fontStyle: 'italic underline' } }],
        }

        const result = convertVscodeTheme([source], 'dark')

        expect(result.tokenColors).toContainEqual({ scope: ['comment'], settings: { foreground: '#6a9955', fontStyle: 'italic underline' } })
    })

    test('tokenColors 는 background 를 보존한다', () => {
        const source = {
            ...MINIMAL_DARK_SOURCE,
            tokenColors: [{ scope: 'markup.inserted', settings: { background: '#003300' } }],
        }

        const result = convertVscodeTheme([source], 'dark')

        expect(result.tokenColors).toContainEqual({ scope: ['markup.inserted'], settings: { background: '#003300' } })
    })

    test('scope 없는 전역 룰도 tokenColors 에 보존한다', () => {
        const source = {
            ...MINIMAL_DARK_SOURCE,
            tokenColors: [{ settings: { foreground: '#d4d4d4' } }],
        }

        const result = convertVscodeTheme([source], 'dark')

        expect(result.tokenColors).toContainEqual({ scope: [], settings: { foreground: '#d4d4d4' } })
    })

    test('settings 가 비어 있는 룰은 tokenColors 에서 제외한다', () => {
        const source = {
            ...MINIMAL_DARK_SOURCE,
            tokenColors: [
                { scope: 'comment', settings: {} },
                { scope: 'keyword', settings: { foreground: '#ff0000' } },
            ],
        }

        const result = convertVscodeTheme([source], 'dark')

        expect(result.tokenColors).toHaveLength(1)
        expect(result.tokenColors[0]?.scope).toEqual(['keyword'])
    })

    test('include 체인의 tokenColors 는 base 를 먼저, leaf 를 뒤에 이어붙인다', () => {
        const base = { ...MINIMAL_DARK_SOURCE, tokenColors: [{ scope: 'comment', settings: { foreground: '#6a9955' } }] }
        const leaf = { tokenColors: [{ scope: 'keyword', settings: { foreground: '#ff0000' } }] }

        const result = convertVscodeTheme([base, leaf], 'dark')

        expect(result.tokenColors.map((rule) => rule.scope)).toEqual([['comment'], ['keyword']])
    })

    test('list.highlightForeground 와 editor.findMatchHighlightBackground 가 모두 없으면 panel.matchHighlight 는 safe-default 상태색으로 폴백한다', () => {
        const result = convertVscodeTheme([MINIMAL_DARK_SOURCE], 'dark')

        expect(result.colors['panel.matchHighlight']).toBe('#569CD6')
        expect(result.outputColorErrors).toEqual([])
    })

    test('editor.findMatchHighlightBackground 가 반투명(github-dark 실사례 #ffd33d22)이면 그 값을 쓰지 않고 safe-default 로 폴백한다', () => {
        const source = { colors: { ...MINIMAL_DARK_SOURCE.colors, 'editor.findMatchHighlightBackground': '#ffd33d22' } }

        const result = convertVscodeTheme([source], 'dark')

        expect(result.colors['panel.matchHighlight']).toBe('#569CD6')
        expect(result.colors['panel.matchHighlight']).not.toBe('#ffd33d22')
        expect(result.outputColorErrors).toEqual([])
    })

    test('editor.findMatchHighlightBackground 가 반투명(github-light 실사례 #ffdf5d66)이어도 그 값을 쓰지 않고 safe-default 로 폴백한다', () => {
        const lightSource = {
            colors: {
                'editor.background': '#ffffff',
                'editor.foreground': '#1e1e1e',
                foreground: '#1e1e1e',
                'editor.findMatchHighlightBackground': '#ffdf5d66',
            },
        }

        const result = convertVscodeTheme([lightSource], 'light')

        expect(result.colors['panel.matchHighlight']).toBe('#0066BF')
        expect(result.colors['panel.matchHighlight']).not.toBe('#ffdf5d66')
        expect(result.outputColorErrors).toEqual([])
    })

    test('list.highlightForeground 가 불투명이면 editor.findMatchHighlightBackground 보다 우선해 그대로 사용한다', () => {
        const source = {
            colors: { ...MINIMAL_DARK_SOURCE.colors, 'list.highlightForeground': '#ffffff', 'editor.findMatchHighlightBackground': '#ffd33d22' },
        }

        const result = convertVscodeTheme([source], 'dark')

        expect(result.colors['panel.matchHighlight']).toBe('#ffffff')
    })

    test('list.highlightForeground 가 불투명이어도 app.foreground(본문 전경)과 지각적으로 동일(ΔE 0)하면 그 값을 쓰지 않고 safe-default 로 폴백한다(monokai 실사례 #f8f8f2, dark)', () => {
        const source = {
            colors: { 'editor.background': '#272822', 'editor.foreground': '#f8f8f2', foreground: '#f8f8f2', 'list.highlightForeground': '#f8f8f2' },
        }

        const result = convertVscodeTheme([source], 'dark')

        expect(result.colors['panel.matchHighlight']).toBe('#569CD6')
        expect(result.colors['panel.matchHighlight']).not.toBe('#f8f8f2')
        expect(result.outputColorErrors).toEqual([])
    })

    test('list.highlightForeground 가 app.foreground 와 지각적으로 동일하면 light 타입에서도 같은 방식으로 safe-default 로 폴백한다', () => {
        const source = {
            colors: { 'editor.background': '#ffffff', 'editor.foreground': '#1e1e1e', foreground: '#1e1e1e', 'list.highlightForeground': '#1e1e1e' },
        }

        const result = convertVscodeTheme([source], 'light')

        expect(result.colors['panel.matchHighlight']).toBe('#0066BF')
        expect(result.colors['panel.matchHighlight']).not.toBe('#1e1e1e')
    })

    test('list.highlightForeground 가 app.foreground 와 다르지만 지각적으로 가까운(ΔE 5.4, one-monokai 실사례) 경우는 배제하지 않고 그대로 사용한다', () => {
        const source = {
            colors: { ...MINIMAL_DARK_SOURCE.colors, foreground: '#D4D4D4', 'editor.foreground': '#D4D4D4', 'list.highlightForeground': '#C5C5C5' },
        }

        const result = convertVscodeTheme([source], 'dark')

        expect(result.colors['panel.matchHighlight']).toBe('#C5C5C5')
    })

    test('구별성 가드가 배제한 후보를 대비 수리가 다시 본문 전경으로 되돌리는 재유입 시나리오에서, 상태색 후보가 전혀 없으면 여전히 본문 전경과 동일값으로 떨어지되 repairs 에 2패스 고지를 남긴다', () => {
        const source = {
            colors: {
                foreground: '#D8DEE9',
                'editor.foreground': '#D8DEE9',
                'editor.background': '#2E3440',
                'sideBar.background': '#5A5A5A',
                'list.highlightForeground': '#D8DEE9',
            },
        }

        const result = convertVscodeTheme([source], 'dark')

        expect(result.colors['panel.matchHighlight']).toBe('#D8DEE9')
        expect(result.outputColorErrors).toEqual([])
        expect(result.repairs.some((repair) => repair.startsWith('panel.matchHighlight') && repair.includes('구별 가능한 후보 없음'))).toBe(true)
    })

    test('같은 재유입 시나리오에서 상태색 후보(textLink.foreground)가 있으면 본문 전경 대신 그 색을 채택해 ΔE ≥ 2.3 구별성을 유지한다', () => {
        const source = {
            colors: {
                foreground: '#D8DEE9',
                'editor.foreground': '#D8DEE9',
                'editor.background': '#2E3440',
                'sideBar.background': '#5A5A5A',
                'list.highlightForeground': '#D8DEE9',
                'textLink.foreground': '#88C0D0',
            },
        }

        const result = convertVscodeTheme([source], 'dark')

        expect(result.colors['panel.matchHighlight']).toBe('#88C0D0')
        expect(result.outputColorErrors).toEqual([])
        expect(result.repairs.some((repair) => repair.includes('구별 가능한 후보 없음'))).toBe(false)
    })

    test('everforest-light 프록시(list.highlightForeground 만 저대비 accent, 상태색 후보 없음)를 재변환해도 여전히 outputColorErrors 가 비어 임포트를 막지 않는다', () => {
        const source = {
            colors: {
                'editor.background': '#fdf6e3',
                'editor.foreground': '#5c6a72',
                foreground: '#5c6a72',
                'sideBar.background': '#fdf6e3',
                'list.highlightForeground': '#8da101',
            },
        }

        const result = convertVscodeTheme([source], 'light')

        expect(result.outputColorErrors).toEqual([])
    })

    test('rose-pine-dawn 프록시(list.highlightForeground 는 저대비지만 textLink.foreground 에 구별 가능한 accent 존재)를 재변환해도 outputColorErrors 가 비어 임포트를 막지 않는다', () => {
        const source = {
            colors: {
                'editor.background': '#faf4ed',
                'editor.foreground': '#575279',
                foreground: '#575279',
                'sideBar.background': '#faf4ed',
                'list.highlightForeground': '#d7827e',
                'textLink.foreground': '#907aa9',
            },
        }

        const result = convertVscodeTheme([source], 'light')

        expect(result.colors['panel.matchHighlight']).toBe('#907aa9')
        expect(result.outputColorErrors).toEqual([])
    })
})

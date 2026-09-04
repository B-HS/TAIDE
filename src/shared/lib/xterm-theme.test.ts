import { describe, expect, test } from 'bun:test'
import type { ResolvedTheme } from '@shared/api/bindings'
import { TERMINAL_ANSI_TOKENS } from '@shared/lib/theme-convert/types'
import { toXtermTheme } from '@shared/lib/xterm-theme'

/**
 * The adapter between a resolved TAIDE theme (`terminal` is an open string map, because an imported
 * vsix theme may define any subset) and xterm's `ITheme`. The interesting half is what it refuses
 * to copy: xterm treats a present-but-empty color as a real value and paints with it, so a theme
 * that simply omits `brightBlack` must leave the key absent rather than hand over `''`.
 */
const buildResolvedTheme = (terminal: Record<string, string>): ResolvedTheme => ({
    id: 'test-theme',
    name: 'Test Theme',
    type: 'dark',
    colors: {},
    syntax: {},
    terminal,
    tokenColors: null,
    syntaxOverrides: [],
    warnings: [],
    author: null,
    license: null,
    source: null,
})

const BASE_TERMINAL = { background: '#101010', foreground: '#f0f0f0', cursor: '#ff8800', selection: '#334455' }

describe('toXtermTheme', () => {
    test('배경·전경·커서를 그대로 옮기고 selection 은 selectionBackground 로 이름을 바꾼다', () => {
        const result = toXtermTheme(buildResolvedTheme(BASE_TERMINAL))

        expect(result.background).toBe('#101010')
        expect(result.foreground).toBe('#f0f0f0')
        expect(result.cursor).toBe('#ff8800')
        expect(result.selectionBackground).toBe('#334455')
        expect('selection' in result).toBe(false)
    })

    test('16 ANSI 토큰이 모두 있으면 모두 옮긴다', () => {
        const terminal: Record<string, string> = { ...BASE_TERMINAL }
        for (const token of TERMINAL_ANSI_TOKENS) terminal[token] = `#${token}`
        const result = toXtermTheme(buildResolvedTheme(terminal))

        for (const token of TERMINAL_ANSI_TOKENS) expect(result[token]).toBe(`#${token}`)
    })

    test('정의되지 않은 ANSI 토큰은 키 자체를 만들지 않는다', () => {
        const result = toXtermTheme(buildResolvedTheme({ ...BASE_TERMINAL, red: '#ff0000' }))

        expect(result.red).toBe('#ff0000')
        expect('brightBlack' in result).toBe(false)
        expect('green' in result).toBe(false)
    })

    test('빈 문자열 ANSI 토큰도 생략한다 (xterm 이 빈 색을 실제 색으로 취급하므로)', () => {
        const result = toXtermTheme(buildResolvedTheme({ ...BASE_TERMINAL, blue: '' }))

        expect('blue' in result).toBe(false)
    })

    test('ANSI 목록에 없는 terminal 키는 옮기지 않는다', () => {
        const result = toXtermTheme(buildResolvedTheme({ ...BASE_TERMINAL, someVendorColor: '#123456' }))

        expect('someVendorColor' in result).toBe(false)
    })

    test('terminal 이 비어 있으면 4개 기본 키가 undefined 인 객체가 된다 (throw 없음)', () => {
        const result = toXtermTheme(buildResolvedTheme({}))

        expect(result.background).toBeUndefined()
        expect(result.selectionBackground).toBeUndefined()
        expect(Object.keys(result)).toEqual(['background', 'foreground', 'cursor', 'selectionBackground'])
    })
})

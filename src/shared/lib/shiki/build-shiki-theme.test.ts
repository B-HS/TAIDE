import { describe, expect, test } from 'bun:test'
import type { ResolvedTheme, SyntaxStyle } from '@shared/api/bindings'
import { buildShikiTheme, fallbackFromSyntax } from '@shared/lib/shiki/build-shiki-theme'
import { SYNTAX_SCOPE_CANDIDATES, SYNTAX_TOKENS } from '@shared/lib/theme-convert/mapping-tables'

const buildSyntax = (overrides: Partial<Record<(typeof SYNTAX_TOKENS)[number], SyntaxStyle>> = {}): ResolvedTheme['syntax'] =>
    Object.fromEntries(SYNTAX_TOKENS.map((token) => [token, overrides[token] ?? { fg: '#ffffff' }]))

const baseTheme: ResolvedTheme = {
    id: 'taide-dark',
    name: 'TAIDE Dark',
    type: 'dark',
    colors: { 'editor.background': '#1e1e2e', 'editor.foreground': '#cdd6f4' },
    syntax: buildSyntax(),
    terminal: {},
    syntaxOverrides: [],
    warnings: [],
    author: null,
    license: null,
    source: null,
}

describe('fallbackFromSyntax', () => {
    test('31 토큰 전량에 대해 룰을 1개씩 생성한다', () => {
        const rules = fallbackFromSyntax(buildSyntax())
        expect(rules.length).toBe(SYNTAX_TOKENS.length)
    })

    test('comment 와 docComment 가 공유하는 scope(comment) 는 먼저 등장하는 comment 가 소유한다', () => {
        const rules = fallbackFromSyntax(buildSyntax())
        const commentRule = rules.find((rule) => rule.scope.includes('comment.line'))
        const docCommentRule = rules.find((rule) => rule.scope.includes('comment.block.documentation'))
        expect(commentRule?.scope).toContain('comment')
        expect(docCommentRule?.scope).not.toContain('comment')
    })

    test('type 계열이 공유하는 entity.name.type 은 먼저 등장하는 type 토큰이 소유한다', () => {
        const rules = fallbackFromSyntax(buildSyntax())
        const typeRule = rules.find((rule) => rule.scope.includes('support.type'))
        const classRule = rules.find((rule) => rule.scope.includes('entity.name.type.class'))
        const interfaceRule = rules.find((rule) => rule.scope.includes('entity.name.type.interface'))
        const enumRule = rules.find((rule) => rule.scope.includes('entity.name.type.enum'))
        expect(typeRule?.scope).toContain('entity.name.type')
        expect(classRule?.scope).not.toContain('entity.name.type')
        expect(interfaceRule?.scope).not.toContain('entity.name.type')
        expect(enumRule?.scope).not.toContain('entity.name.type')
    })

    test('bold/italic 을 space-separated fontStyle 문자열로 변환하고, 없으면 fontStyle 키를 생략한다', () => {
        const rules = fallbackFromSyntax(buildSyntax({ keyword: { fg: '#ff0000', bold: true, italic: true } }))
        const keywordRule = rules.find((rule) => rule.scope.includes('keyword.control'))
        const stringRule = rules.find((rule) => rule.scope.includes('string.quoted'))
        expect(keywordRule?.settings).toEqual({ foreground: '#ff0000', fontStyle: 'bold italic' })
        expect(stringRule?.settings).toEqual({ foreground: '#ffffff' })
    })
})

describe('buildShikiTheme', () => {
    test('theme 이름은 항상 taide 이고 type 은 resolved.type 을 따른다', () => {
        expect(buildShikiTheme(baseTheme).name).toBe('taide')
        expect(buildShikiTheme(baseTheme).type).toBe('dark')
        expect(buildShikiTheme({ ...baseTheme, type: 'light' }).type).toBe('light')
    })

    test('colors 에 editor.background/editor.foreground 를 항상 포함한다', () => {
        const shikiTheme = buildShikiTheme(baseTheme)
        expect(shikiTheme.colors['editor.background']).toBe('#1e1e2e')
        expect(shikiTheme.colors['editor.foreground']).toBe('#cdd6f4')
    })

    test('raw tokenColors 가 있으면(빈 배열 포함) 그대로 변환해 사용하고, fallback 을 타지 않는다', () => {
        const withRaw: ResolvedTheme = {
            ...baseTheme,
            tokenColors: [{ scope: ['keyword.control'], settings: { foreground: '#123456', fontStyle: 'bold' } }],
        }
        const shikiTheme = buildShikiTheme(withRaw)
        expect(shikiTheme.tokenColors).toEqual([{ scope: ['keyword.control'], settings: { foreground: '#123456', fontStyle: 'bold' } }])

        const withEmptyRaw: ResolvedTheme = { ...baseTheme, tokenColors: [] }
        expect(buildShikiTheme(withEmptyRaw).tokenColors).toEqual([])
    })

    test('raw tokenColors 가 없으면(null/undefined) syntax 로부터 역생성한다', () => {
        const withoutRaw: ResolvedTheme = { ...baseTheme, tokenColors: null }
        expect(buildShikiTheme(withoutRaw).tokenColors.length).toBe(SYNTAX_TOKENS.length)
    })

    test('syntaxOverrides 에 있는 토큰만 raw 뒤에 오버레이로 append 한다', () => {
        const withOverride: ResolvedTheme = {
            ...baseTheme,
            tokenColors: [{ scope: ['keyword.control'], settings: { foreground: '#111111' } }],
            syntax: buildSyntax({ keyword: { fg: '#222222' } }),
            syntaxOverrides: ['keyword'],
        }
        const shikiTheme = buildShikiTheme(withOverride)
        expect(shikiTheme.tokenColors.length).toBe(2)
        const overlay = shikiTheme.tokenColors[shikiTheme.tokenColors.length - 1]
        expect(overlay?.scope).toEqual(SYNTAX_SCOPE_CANDIDATES.keyword)
        expect(overlay?.settings.foreground).toBe('#222222')
    })

    test('syntaxOverrides 에 유효하지 않은 토큰 이름이 섞여 있어도 무시한다', () => {
        const withInvalidOverride: ResolvedTheme = { ...baseTheme, syntaxOverrides: ['not-a-real-token'] }
        expect(() => buildShikiTheme(withInvalidOverride)).not.toThrow()
        expect(buildShikiTheme(withInvalidOverride).tokenColors.length).toBe(SYNTAX_TOKENS.length)
    })
})

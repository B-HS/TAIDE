import { describe, expect, test } from 'bun:test'
import type { ResolvedTheme, SyntaxStyle } from '@shared/api/bindings'
import { buildShikiTheme, fallbackFromSyntax } from '@shared/lib/shiki/build-shiki-theme'
import { SEMANTIC_TOKEN_TYPE_MAP, SYNTAX_SCOPE_CANDIDATES, SYNTAX_TOKENS, toSemanticTokenLegendScope } from '@shared/lib/theme-convert/mapping-tables'

const buildSyntax = (overrides: Partial<Record<(typeof SYNTAX_TOKENS)[number], SyntaxStyle>> = {}): ResolvedTheme['syntax'] =>
    Object.fromEntries(SYNTAX_TOKENS.map((token) => [token, overrides[token] ?? { fg: '#ffffff' }]))

/** `buildShikiTheme` always appends one rule per distinct `SEMANTIC_TOKEN_TYPE_MAP` target after everything else (contract §3.1) — every count assertion below must account for it. */
const SEMANTIC_TOKEN_TARGET_COUNT = new Set(Object.values(SEMANTIC_TOKEN_TYPE_MAP)).size

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

    test('raw tokenColors 가 있으면(빈 배열 포함) 그대로 앞부분에 보존하고, fallback 을 타지 않는다', () => {
        const withRaw: ResolvedTheme = {
            ...baseTheme,
            tokenColors: [{ scope: ['keyword.control'], settings: { foreground: '#123456', fontStyle: 'bold' } }],
        }
        const shikiTheme = buildShikiTheme(withRaw)
        expect(shikiTheme.tokenColors[0]).toEqual({ scope: ['keyword.control'], settings: { foreground: '#123456', fontStyle: 'bold' } })
        expect(shikiTheme.tokenColors.length).toBe(1 + SEMANTIC_TOKEN_TARGET_COUNT)

        const withEmptyRaw: ResolvedTheme = { ...baseTheme, tokenColors: [] }
        expect(buildShikiTheme(withEmptyRaw).tokenColors.length).toBe(SEMANTIC_TOKEN_TARGET_COUNT)
    })

    test('raw tokenColors 가 없으면(null/undefined) syntax 로부터 역생성한다', () => {
        const withoutRaw: ResolvedTheme = { ...baseTheme, tokenColors: null }
        expect(buildShikiTheme(withoutRaw).tokenColors.length).toBe(SYNTAX_TOKENS.length + SEMANTIC_TOKEN_TARGET_COUNT)
    })

    test('syntaxOverrides 에 있는 토큰만 raw 뒤에 오버레이로 append 한다', () => {
        const withOverride: ResolvedTheme = {
            ...baseTheme,
            tokenColors: [{ scope: ['keyword.control'], settings: { foreground: '#111111' } }],
            syntax: buildSyntax({ keyword: { fg: '#222222' } }),
            syntaxOverrides: ['keyword'],
        }
        const shikiTheme = buildShikiTheme(withOverride)
        expect(shikiTheme.tokenColors.length).toBe(2 + SEMANTIC_TOKEN_TARGET_COUNT)
        const overlay = shikiTheme.tokenColors[1]
        expect(overlay?.scope).toEqual(SYNTAX_SCOPE_CANDIDATES.keyword)
        expect(overlay?.settings.foreground).toBe('#222222')
    })

    test('syntaxOverrides 에 유효하지 않은 토큰 이름이 섞여 있어도 무시한다', () => {
        const withInvalidOverride: ResolvedTheme = { ...baseTheme, syntaxOverrides: ['not-a-real-token'] }
        expect(() => buildShikiTheme(withInvalidOverride)).not.toThrow()
        expect(buildShikiTheme(withInvalidOverride).tokenColors.length).toBe(SYNTAX_TOKENS.length + SEMANTIC_TOKEN_TARGET_COUNT)
    })
})

describe('buildShikiTheme semantic token rules', () => {
    test('SEMANTIC_TOKEN_TYPE_MAP 의 매핑 대상 토큰명을 taideSemantic. 네임스페이스 scope 로 하는 rule 을 tokenColors 맨 끝에 추가한다', () => {
        const shikiTheme = buildShikiTheme({ ...baseTheme, tokenColors: [] })
        const namespaceRule = shikiTheme.tokenColors.find((rule) => rule.scope[0] === toSemanticTokenLegendScope('namespace'))
        expect(namespaceRule).toEqual({ scope: [toSemanticTokenLegendScope('namespace')], settings: { foreground: '#ffffff' } })

        const semanticTargetScopes = new Set(Object.values(SEMANTIC_TOKEN_TYPE_MAP).map(toSemanticTokenLegendScope))
        const lastRule = shikiTheme.tokenColors[shikiTheme.tokenColors.length - 1]
        expect(lastRule !== undefined && semanticTargetScopes.has(lastRule.scope[0])).toBe(true)
    })

    test('semantic rule 은 새 색을 도입하지 않고 해당 토큰의 기존 syntax 색을 그대로 재사용한다', () => {
        const shikiTheme = buildShikiTheme({ ...baseTheme, tokenColors: [], syntax: buildSyntax({ variable: { fg: '#abcdef' } }) })
        const variableRule = shikiTheme.tokenColors.find((rule) => rule.scope[0] === toSemanticTokenLegendScope('variable'))
        expect(variableRule?.settings.foreground).toBe('#abcdef')
    })

    test('raw tokenColors 뒤, syntaxOverrides 오버레이 뒤에 위치한다', () => {
        const withOverride: ResolvedTheme = {
            ...baseTheme,
            tokenColors: [{ scope: ['keyword.control'], settings: { foreground: '#111111' } }],
            syntaxOverrides: ['string'],
        }
        const shikiTheme = buildShikiTheme(withOverride)
        expect(shikiTheme.tokenColors[0]?.scope).toEqual(['keyword.control'])
        expect(shikiTheme.tokenColors[1]?.scope).toEqual(SYNTAX_SCOPE_CANDIDATES.string)
        expect(shikiTheme.tokenColors.slice(2).every((rule) => rule.scope.length === 1)).toBe(true)
    })

    test('semantic 매핑이 없는 SYNTAX_TOKENS(docComment/tag/attribute/markdown 계열)는 추가되지 않는다', () => {
        const shikiTheme = buildShikiTheme({ ...baseTheme, tokenColors: [] })
        const unmappedTokens = ['docComment', 'tag', 'attribute', 'link', 'markdownHeading']
        for (const token of unmappedTokens)
            expect(shikiTheme.tokenColors.some((rule) => rule.scope[0] === toSemanticTokenLegendScope(token as never))).toBe(false)
    })

    /**
     * Regression coverage for the washout-fix bug (contract §5 "워시아웃 방어 실효"): a theme's own
     * raw `tokenColors` rule for a bare `SYNTAX_TOKENS` name (e.g. `'variable'`, exactly what
     * several bundled themes — github-dark among them — declare) must survive `buildShikiTheme`
     * completely unmodified, and no semantic rule may ever carry that same bare scope. Before the
     * fix, `buildSemanticTokenThemeRules` appended a rule scoped exactly `['variable']`, which
     * monaco's token-theme trie (sorted-by-token, last-index-wins on an exact match) resolved to
     * *replace* the theme's own rule for every regular (non-semantic) token painted through that
     * scope — not just semantic ones.
     */
    test('실제 테마의 bare scope(예: variable) rule 은 append 이후에도 그대로 유지되고, semantic rule 은 그 scope 를 절대 쓰지 않는다', () => {
        const withBareVariableRule: ResolvedTheme = {
            ...baseTheme,
            tokenColors: [{ scope: ['variable'], settings: { foreground: '#ffab70' } }],
        }
        const shikiTheme = buildShikiTheme(withBareVariableRule)

        expect(shikiTheme.tokenColors[0]).toEqual({ scope: ['variable'], settings: { foreground: '#ffab70' } })

        const bareVariableRules = shikiTheme.tokenColors.filter((rule) => rule.scope.length === 1 && rule.scope[0] === 'variable')
        expect(bareVariableRules).toEqual([{ scope: ['variable'], settings: { foreground: '#ffab70' } }])
    })
})

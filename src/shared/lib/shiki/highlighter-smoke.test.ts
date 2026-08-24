import { describe, expect, test } from 'bun:test'
import { createHighlighterCore } from '@shikijs/core'
import { createJavaScriptRegexEngine } from '@shikijs/engine-javascript'
import type { ResolvedTheme } from '@shared/api/bindings'
import { buildShikiTheme } from '@shared/lib/shiki/build-shiki-theme'
import { loadTaideGrammar, type TaideLanguageId } from '@shared/lib/shiki/lang-map'
import { SYNTAX_TOKENS } from '@shared/lib/theme-convert/ui-token-vocabulary'

const SMOKE_LANGUAGE_IDS = ['typescript', 'typescriptreact', 'rust'] as const satisfies readonly TaideLanguageId[]

const SMOKE_CODE_BY_LANGUAGE: Record<(typeof SMOKE_LANGUAGE_IDS)[number], string> = {
    typescript: 'const answer: number = 42',
    typescriptreact: 'const App = () => <div>hi</div>',
    rust: 'fn main() { println!("hi"); }',
}

const smokeResolvedTheme: ResolvedTheme = {
    id: 'smoke',
    name: 'Smoke',
    type: 'dark',
    colors: { 'editor.background': '#1e1e2e', 'editor.foreground': '#cdd6f4' },
    syntax: Object.fromEntries(SYNTAX_TOKENS.map((token) => [token, { fg: '#ffffff' }])),
    terminal: {},
    syntaxOverrides: [],
    warnings: [],
    author: null,
    license: null,
    source: null,
}

describe('shiki highlighter 스모크 테스트 (JS RegExp 엔진)', () => {
    test('typescript·tsx·rust grammar 가 createHighlighterCore + createJavaScriptRegexEngine 로 예외 없이 토큰화된다', async () => {
        const grammars = (await Promise.all(SMOKE_LANGUAGE_IDS.map((id) => loadTaideGrammar(id)))).flat()
        const highlighter = await createHighlighterCore({
            themes: [buildShikiTheme(smokeResolvedTheme)],
            langs: grammars,
            engine: createJavaScriptRegexEngine(),
        })

        try {
            expect(highlighter.getLoadedLanguages()).toContain('typescriptreact')
            for (const languageId of SMOKE_LANGUAGE_IDS) {
                const lines = highlighter.codeToTokensBase(SMOKE_CODE_BY_LANGUAGE[languageId], { lang: languageId, theme: 'taide' })
                expect(lines.length).toBeGreaterThan(0)
                expect(lines[0]?.length).toBeGreaterThan(0)
            }
        } finally {
            highlighter.dispose()
        }
    })
})

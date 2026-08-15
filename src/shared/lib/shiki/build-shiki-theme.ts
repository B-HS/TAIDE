import type { ResolvedTheme, SyntaxStyle, ThemeType, TokenColorRule } from '@shared/api/bindings'
import { buildThemeColors, TAIDE_MONACO_THEME_NAME, toMonacoFontStyle, toThemeColor } from '@shared/lib/monaco/theme'
import { SEMANTIC_TOKEN_TYPE_MAP, SYNTAX_SCOPE_CANDIDATES, SYNTAX_TOKENS, toSemanticTokenLegendScope } from '@shared/lib/theme-convert/mapping-tables'

export type ShikiTokenColorSettings = {
    foreground?: string
    background?: string
    fontStyle?: string
}

export type ShikiTokenColorRule = {
    scope: string[]
    settings: ShikiTokenColorSettings
}

export type ShikiThemeInput = {
    name: string
    type: ThemeType
    colors: Record<string, string>
    tokenColors: ShikiTokenColorRule[]
}

const isSyntaxToken = (value: string): value is (typeof SYNTAX_TOKENS)[number] => (SYNTAX_TOKENS as readonly string[]).includes(value)

const toSyntaxSettings = (style: SyntaxStyle): ShikiTokenColorSettings => {
    const fontStyle = toMonacoFontStyle(style)
    return { foreground: toThemeColor(style.fg), ...(fontStyle ? { fontStyle } : {}) }
}

const toShikiTokenColorRule = (rule: TokenColorRule): ShikiTokenColorRule => ({
    scope: rule.scope,
    settings: {
        ...(rule.settings.foreground ? { foreground: rule.settings.foreground } : {}),
        ...(rule.settings.background ? { background: rule.settings.background } : {}),
        ...(rule.settings.fontStyle ? { fontStyle: rule.settings.fontStyle } : {}),
    },
})

const buildClaimedScopeRules = (tokens: readonly (typeof SYNTAX_TOKENS)[number][], syntax: ResolvedTheme['syntax']): ShikiTokenColorRule[] => {
    const tokenSet = new Set(tokens)
    const claimedScopes = new Set<string>()
    const rules: ShikiTokenColorRule[] = []
    for (const token of SYNTAX_TOKENS) {
        if (!tokenSet.has(token)) continue
        const style = syntax[token]
        if (!style) continue
        const scope = SYNTAX_SCOPE_CANDIDATES[token].filter((candidate) => !claimedScopes.has(candidate))
        if (scope.length === 0) continue
        for (const candidate of scope) claimedScopes.add(candidate)
        rules.push({ scope, settings: toSyntaxSettings(style) })
    }
    return rules
}

/**
 * Derives TextMate token color rules from TAIDE's flat 31-token `syntax` map, for themes that
 * carry no raw `tokenColors` (legacy themes, hand-built themes edited only through the per-token
 * syntax editor). Scope candidates are shared across several tokens (e.g. `entity.name.type` for
 * `type`/`class`/`interface`/`enum`); when two tokens compete for the same scope, the token that
 * appears first in `SYNTAX_TOKENS` keeps it and later tokens fall back to their remaining
 * unclaimed candidates.
 */
export const fallbackFromSyntax = (syntax: ResolvedTheme['syntax']): ShikiTokenColorRule[] => buildClaimedScopeRules(SYNTAX_TOKENS, syntax)

/** Every distinct `SYNTAX_TOKENS` name a semantic token type can map to (`SEMANTIC_TOKEN_TYPE_MAP`'s value set), in first-appearance order. */
const SEMANTIC_TOKEN_THEME_TARGETS = [...new Set(Object.values(SEMANTIC_TOKEN_TYPE_MAP))]

/**
 * Appends one namespaced-scope rule (`toSemanticTokenLegendScope(token)`, e.g.
 * `'taideSemantic.variable'` — never a bare TextMate scope like `'variable'`) per
 * `SEMANTIC_TOKEN_THEME_TARGETS` entry, reusing `buildClaimedScopeRules`'s own color source
 * (`syntax[token]`, the theme's per-token syntax color — never a new color). monaco's semantic
 * token styling looks up `[type, ...modifiers].join('.')` against this same theme's `rules` trie
 * (`standaloneThemeService.js`) and falls back step by step to shorter prefixes; the adapter's own
 * legend (`adapters/semantic-tokens.ts`'s `buildSemanticTokensLegendMapping`) reports this same
 * namespaced string as each type's monaco-facing name, so `type` in that join is always
 * `taideSemantic.<token>` and the lookup bottoms out on the rule appended here.
 *
 * The namespace prefix is not cosmetic: a bare token name (e.g. `'variable'`) is also a real
 * TextMate scope many bundled themes' own `tokenColors` already declare a rule for. monaco's token
 * theme trie (`resolveParsedTokenThemeRules`) sorts all rules by their `token` string and, for two
 * rules with the *identical* string, by array index — so a bare-scope rule appended here would sort
 * to the same trie node as the theme's own rule and, per `ThemeTrieElement.insert`'s exact-match
 * `acceptOverwrite`, the later (appended) rule always wins and silently replaces the theme's color
 * for every *regular* (non-semantic) token that resolves through that scope — not just semantic
 * ones. Namespacing avoids that collision entirely: no TextMate grammar emits a scope under
 * `taideSemantic.*`, so a rule scoped there can never exact-match — or even prefix-match — a scope
 * a real grammar or theme author actually uses.
 */
const buildSemanticTokenThemeRules = (syntax: ResolvedTheme['syntax']): ShikiTokenColorRule[] => {
    const rules: ShikiTokenColorRule[] = []
    for (const token of SEMANTIC_TOKEN_THEME_TARGETS) {
        const style = syntax[token]
        if (!style) continue
        rules.push({ scope: [toSemanticTokenLegendScope(token)], settings: toSyntaxSettings(style) })
    }
    return rules
}

/**
 * Assembles the single `taide` shiki theme from a resolved TAIDE theme: UI colors (reusing the
 * existing Monaco color mapping, plus an explicit `editor.background`/`editor.foreground` pair so
 * shiki's own fg/bg derivation never falls back to its built-in defaults), and TextMate
 * tokenColors — either the theme's own raw rules or a syntax-derived fallback, with any
 * `syntaxOverrides` tokens appended afterward as a best-effort overlay (a wide overlay scope can
 * lose to a narrower raw scope under TextMate specificity rules — see docs/theme-system.md).
 */
export const buildShikiTheme = (resolved: ResolvedTheme): ShikiThemeInput => {
    const colors: Record<string, string> = {
        ...buildThemeColors(resolved.colors),
        'editor.background': resolved.colors['editor.background'],
        'editor.foreground': resolved.colors['editor.foreground'],
    }
    const rawTokenColors = resolved.tokenColors ? resolved.tokenColors.map(toShikiTokenColorRule) : fallbackFromSyntax(resolved.syntax)
    const overlayTokens = (resolved.syntaxOverrides ?? []).filter(isSyntaxToken)
    const overlayTokenColors = buildClaimedScopeRules(overlayTokens, resolved.syntax)
    const semanticTokenColors = buildSemanticTokenThemeRules(resolved.syntax)
    return {
        name: TAIDE_MONACO_THEME_NAME,
        type: resolved.type,
        colors,
        tokenColors: [...rawTokenColors, ...overlayTokenColors, ...semanticTokenColors],
    }
}

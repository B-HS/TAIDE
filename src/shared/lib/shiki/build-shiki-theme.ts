import type { ResolvedTheme, SyntaxStyle, ThemeType, TokenColorRule } from '@shared/api/bindings'
import { buildThemeColors, TAIDE_MONACO_THEME_NAME, toMonacoFontStyle, toThemeColor } from '@shared/lib/monaco/theme'
import { SYNTAX_SCOPE_CANDIDATES, SYNTAX_TOKENS } from '@shared/lib/theme-convert/mapping-tables'

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
    return {
        name: TAIDE_MONACO_THEME_NAME,
        type: resolved.type,
        colors,
        tokenColors: [...rawTokenColors, ...overlayTokenColors],
    }
}

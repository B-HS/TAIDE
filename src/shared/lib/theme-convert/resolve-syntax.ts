import { compositeOverBackground, HEX_ALPHA_LENGTH, isHexColor } from '@shared/lib/color'
import type { SyntaxStyle, VscodeTheme, VscodeTokenColorRule } from '@shared/lib/theme-convert/types'
import { SYNTAX_SCOPE_CANDIDATES, SYNTAX_TOKENS } from '@shared/lib/theme-convert/ui-token-vocabulary'

const normalizeSyntaxForeground = (fg: string, editorBackground: string) =>
    isHexColor(fg) && fg.length === HEX_ALPHA_LENGTH ? compositeOverBackground(fg, editorBackground) : fg

const findBestRule = (candidateScope: string, rules: VscodeTokenColorRule[]) => {
    let best: { rule: VscodeTokenColorRule; length: number } | null = null
    for (const rule of rules) {
        for (const scope of rule.scopes) {
            const matches = candidateScope === scope || candidateScope.startsWith(`${scope}.`)
            if (matches && rule.fg && (!best || scope.length >= best.length)) best = { rule, length: scope.length }
        }
    }
    return best?.rule
}

export const resolveSyntax = (theme: VscodeTheme, editorForeground: string, editorBackground: string) => {
    const syntax: Record<string, SyntaxStyle> = {}
    for (const token of SYNTAX_TOKENS) {
        const candidates = SYNTAX_SCOPE_CANDIDATES[token]
        const matchedRule = candidates.map((candidate) => findBestRule(candidate, theme.tokenColors)).find((rule) => rule !== undefined)
        const fg = matchedRule?.fg ?? editorForeground
        syntax[token] = {
            fg: normalizeSyntaxForeground(fg, editorBackground),
            bold: matchedRule?.bold ?? false,
            italic: matchedRule?.italic ?? false,
        }
    }
    return syntax
}

import { hexToRgb, isHexColor, rgbToHex } from '@shared/lib/color'
import type { SyntaxStyle, VscodeTheme, VscodeTokenColorRule } from '@shared/lib/theme-convert/types'
import { SYNTAX_SCOPE_CANDIDATES, SYNTAX_TOKENS } from '@shared/lib/theme-convert/mapping-tables'

const HEX_ALPHA_LENGTH = 9
const ALPHA_CHANNEL_MAX = 255

const compositeAlphaOverBackground = (fgHex8: string, backgroundHex: string) => {
    const fgRgb = hexToRgb(fgHex8.slice(0, 7))
    const bgRgb = hexToRgb(backgroundHex)
    if (!fgRgb || !bgRgb) return fgHex8.slice(0, 7)
    const alpha = Number.parseInt(fgHex8.slice(7, HEX_ALPHA_LENGTH), 16) / ALPHA_CHANNEL_MAX
    return rgbToHex({
        r: fgRgb.r * alpha + bgRgb.r * (1 - alpha),
        g: fgRgb.g * alpha + bgRgb.g * (1 - alpha),
        b: fgRgb.b * alpha + bgRgb.b * (1 - alpha),
    })
}

const normalizeSyntaxForeground = (fg: string, editorBackground: string) =>
    isHexColor(fg) && fg.length === HEX_ALPHA_LENGTH ? compositeAlphaOverBackground(fg, editorBackground) : fg

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

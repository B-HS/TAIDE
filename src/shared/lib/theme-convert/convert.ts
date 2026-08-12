import type { TokenColorRule } from '@shared/api/bindings'
import { mergeVscodeThemeChain } from '@shared/lib/theme-convert/merge'
import { resolveAnsiLookup, resolveColors } from '@shared/lib/theme-convert/resolve-colors'
import { resolveSyntax } from '@shared/lib/theme-convert/resolve-syntax'
import { resolveTerminal } from '@shared/lib/theme-convert/resolve-terminal'
import { repairContrastPairs, validateOutputColors } from '@shared/lib/theme-convert/contrast'
import { validateCompleteness } from '@shared/lib/theme-convert/validate-completeness'
import type { SyntaxStyle, ThemeTypeArg, VscodeTokenColorRule } from '@shared/lib/theme-convert/types'

const FALLBACK_BACKGROUND = '#000000'
const FALLBACK_FOREGROUND = '#000000'

export type ThemeConversionResult = {
    status: 'ok' | 'incomplete'
    colors: Record<string, string>
    syntax: Record<string, SyntaxStyle>
    terminal: Record<string, string>
    tokenColors: TokenColorRule[]
    missingColors: string[]
    missingSyntax: string[]
    missingTerminal: string[]
    safeDefaultNotices: string[]
    ansiFallbackTokens: string[]
    repairs: string[]
    outputColorErrors: string[]
}

/**
 * Converts merged VS Code tokenColors rules (base-first, name discarded) into TAIDE's
 * `TokenColorRule` shape, dropping rules whose settings end up empty (no foreground,
 * background, or fontStyle survives normalization).
 */
const buildTokenColors = (rules: VscodeTokenColorRule[]): TokenColorRule[] =>
    rules
        .map((rule) => ({
            scope: rule.scopes,
            settings: {
                ...(rule.fg !== undefined && { foreground: rule.fg }),
                ...(rule.background !== undefined && { background: rule.background }),
                ...(rule.fontStyle && { fontStyle: rule.fontStyle }),
            },
        }))
        .filter((rule) => Object.keys(rule.settings).length > 0)

/**
 * Converts an already-parsed VS Code theme include chain (base-first, most specific last —
 * see {@link mergeVscodeThemeChain}) into TAIDE's theme token shape. Pure function: no file IO,
 * no console output, no process exit — callers (the CLI script, the VSIX import flow) decide how
 * to surface `missingColors`/`missingSyntax`/`missingTerminal` (fatal — internal mapping-table
 * inconsistency) and `outputColorErrors` (fatal — unrepairable low contrast) as an error.
 */
export const convertVscodeTheme = (rawChain: Record<string, unknown>[], type: ThemeTypeArg): ThemeConversionResult => {
    const theme = mergeVscodeThemeChain(rawChain)

    const { ansi, fallbackTokens: ansiFallbackTokens } = resolveAnsiLookup(theme.colors, type)
    const { colors: resolvedColors, safeDefaultNotices } = resolveColors(theme.colors, type, ansi)
    const { colors, repairs } = repairContrastPairs(resolvedColors, theme.colors)
    const editorBackground = colors['editor.background'] ?? theme.colors['editor.background'] ?? FALLBACK_BACKGROUND
    const editorForeground = colors['editor.foreground'] ?? theme.colors['editor.foreground'] ?? theme.colors.foreground ?? FALLBACK_FOREGROUND
    const syntax = resolveSyntax(theme, editorForeground, editorBackground)
    const terminal = resolveTerminal(colors, ansi)
    const tokenColors = buildTokenColors(theme.tokenColors)

    const { missingColors, missingSyntax, missingTerminal } = validateCompleteness(colors, syntax, terminal)
    const outputColorErrors = validateOutputColors(colors)

    return {
        status: missingColors.length === 0 && missingSyntax.length === 0 && missingTerminal.length === 0 ? 'ok' : 'incomplete',
        colors,
        syntax,
        terminal,
        tokenColors,
        missingColors,
        missingSyntax,
        missingTerminal,
        safeDefaultNotices,
        ansiFallbackTokens,
        repairs,
        outputColorErrors,
    }
}

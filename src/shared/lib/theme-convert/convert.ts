import type { TokenColorRule } from '@shared/api/bindings'
import { mergeVscodeThemeChain } from '@shared/lib/theme-convert/merge'
import { resolveAnsiLookup, resolveColors } from '@shared/lib/theme-convert/resolve-colors'
import { resolveSyntax } from '@shared/lib/theme-convert/resolve-syntax'
import { resolveTerminal } from '@shared/lib/theme-convert/resolve-terminal'
import { repairContrastPairs, validateOutputColors } from '@shared/lib/theme-convert/contrast'
import { repairStateDistinctness, validateStateDistinctness } from '@shared/lib/theme-convert/state-distinctness'
import { validateCompleteness } from '@shared/lib/theme-convert/validate-completeness'
import type { SyntaxStyle, ThemeTypeArg, VscodeTokenColorRule } from '@shared/lib/theme-convert/types'

const FALLBACK_BACKGROUND = '#000000'
const FALLBACK_FOREGROUND = '#000000'

/**
 * A converted VS Code theme carries no named palette — `scripts/convert-vscode-theme.ts` writes
 * `palette: {}` because the mapping tables resolve every token to a literal color. The palette-first
 * branch of {@link repairStateDistinctness} therefore never fires on this path; it exists for
 * hand-authored and user-saved themes, which do use the format's `@name` references.
 */
const CONVERTED_THEME_PALETTE: Record<string, string> = {}

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
    /**
     * Advisory counterpart to `outputColorErrors` for the state-vs-container axes
     * (`state-distinctness-pairs.ts`), listing whatever survived {@link repairStateDistinctness}.
     * Deliberately not folded into `outputColorErrors`: that field rejects a VSIX import, and
     * promoting a distinctness shortfall to a rejection would open a new import-failure case that
     * `docs/acknowledge/2026-08-25-d40-selection-row-contrast-contract.md` §1-a's carried-over d-33
     * decision forbids. Callers surface it as a warning.
     */
    stateDistinctnessErrors: string[]
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
 * inconsistency), `outputColorErrors` (fatal — unrepairable low contrast) and
 * `stateDistinctnessErrors` (advisory — a state color that stayed indistinguishable from the surface
 * it renders on) as an error or a warning.
 *
 * State distinctness is repaired before contrast, not after: contrast repair reads the backgrounds
 * it measures foregrounds against, so moving one of those afterwards could undo a contrast fix.
 * The reverse is bounded rather than impossible. Contrast repair writes `tooltip.background`,
 * `menu.itemHover` and foreground tokens, none of which is a *container or surface* of a
 * state-distinctness pair, so no pair is re-measured against a moved backdrop. Two of them are a
 * pair's *state* token though — `menu.itemHover` (`menuItemHover`) and `appSidebar.badge`
 * (`sidebarBadge`) — so a contrast repair can in principle collapse those two axes. Both writes are
 * constrained against that: `repairComponentBackgrounds` only substitutes a value that still clears
 * `STATE_MIN_DISTINCT_DELTA_E` against the state's own surface, and
 * `scripts/repair-theme-contrast.ts` plus the catalog gate re-run {@link validateStateDistinctness}
 * after the contrast pass, so a regression surfaces as a failure rather than silently.
 */
export const convertVscodeTheme = (rawChain: Record<string, unknown>[], type: ThemeTypeArg): ThemeConversionResult => {
    const theme = mergeVscodeThemeChain(rawChain)

    const { ansi, fallbackTokens: ansiFallbackTokens } = resolveAnsiLookup(theme.colors, type)
    const { colors: resolvedColors, safeDefaultNotices } = resolveColors(theme.colors, type, ansi)
    const { colors: distinctColors, repairs: distinctnessRepairs } = repairStateDistinctness(resolvedColors, CONVERTED_THEME_PALETTE)
    const { colors, repairs: contrastRepairs } = repairContrastPairs(distinctColors, theme.colors)
    const editorBackground = colors['editor.background'] ?? theme.colors['editor.background'] ?? FALLBACK_BACKGROUND
    const editorForeground = colors['editor.foreground'] ?? theme.colors['editor.foreground'] ?? theme.colors.foreground ?? FALLBACK_FOREGROUND
    const syntax = resolveSyntax(theme, editorForeground, editorBackground)
    const terminal = resolveTerminal(colors, ansi)
    const tokenColors = buildTokenColors(theme.tokenColors)

    const { missingColors, missingSyntax, missingTerminal } = validateCompleteness(colors, syntax, terminal)
    const outputColorErrors = validateOutputColors(colors)
    const stateDistinctnessErrors = validateStateDistinctness(colors)

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
        repairs: [...distinctnessRepairs, ...contrastRepairs],
        outputColorErrors,
        stateDistinctnessErrors,
    }
}

import { VSCODE_DEFAULT_ANSI_PALETTE } from '@shared/lib/theme-convert/ansi-palette'
import { COLOR_MAPPING, FAMILY_FALLBACK_SOURCE_KEYS, SAFE_DEFAULT_COLORS } from '@shared/lib/theme-convert/mapping-tables'
import type { AnsiLookup, ColorCategory, ColorMappingEntry, ResolveContext, ThemeTypeArg } from '@shared/lib/theme-convert/types'
import { TERMINAL_ANSI_TOKENS } from '@shared/lib/theme-convert/types'

const SELF_REF_PREFIX = '@'
const TERMINAL_ANSI_CANDIDATE_PREFIX = 'terminal.ansi'

const ansiNameFromCandidate = (candidate: string): (typeof TERMINAL_ANSI_TOKENS)[number] | undefined => {
    if (!candidate.startsWith(TERMINAL_ANSI_CANDIDATE_PREFIX)) return undefined
    const rest = candidate.slice(TERMINAL_ANSI_CANDIDATE_PREFIX.length)
    const name = `${rest.charAt(0).toLowerCase()}${rest.slice(1)}` as (typeof TERMINAL_ANSI_TOKENS)[number]
    return TERMINAL_ANSI_TOKENS.includes(name) ? name : undefined
}

const resolveCandidate = (candidate: string, ctx: ResolveContext): string | undefined => {
    if (candidate.startsWith(SELF_REF_PREFIX)) return ctx.resolved[candidate.slice(SELF_REF_PREFIX.length)]
    const ansiName = ansiNameFromCandidate(candidate)
    return ansiName ? ctx.ansi[ansiName] : ctx.vscodeColors[candidate]
}

const resolveFamilyFallback = (category: ColorCategory, ctx: ResolveContext): string | undefined =>
    FAMILY_FALLBACK_SOURCE_KEYS[category].map((key) => ctx.vscodeColors[key]).find((value) => value !== undefined)

const resolveColorEntry = (entry: ColorMappingEntry, ctx: ResolveContext): string | undefined => {
    if (entry.derive) return entry.derive(ctx) ?? resolveFamilyFallback(entry.category, ctx)
    for (const candidate of entry.candidates ?? []) {
        const value = resolveCandidate(candidate, ctx)
        if (value) return value
    }
    return resolveFamilyFallback(entry.category, ctx)
}

const readAnsiFromVscodeColors = (vscodeColors: Record<string, string>): Partial<AnsiLookup> => {
    const names: (typeof TERMINAL_ANSI_TOKENS)[number][] = ['black', 'red', 'green', 'yellow', 'blue', 'magenta', 'cyan', 'white']
    const ansi: Partial<AnsiLookup> = {}
    for (const name of names) {
        const capitalized = name[0].toUpperCase() + name.slice(1)
        const base = vscodeColors[`terminal.ansi${capitalized}`]
        const bright = vscodeColors[`terminal.ansiBright${capitalized}`]
        if (base) ansi[name] = base
        if (bright) ansi[`bright${capitalized}` as (typeof TERMINAL_ANSI_TOKENS)[number]] = bright
    }
    return ansi
}

/**
 * Mirrors VS Code's own terminal ANSI resolution: when a theme declares no
 * `terminal.ansi*` colors, the editor falls back to its built-in default ANSI
 * palette (see docs/theme-system.md §8.2) rather than leaving the terminal
 * uncolored. Every consumer of ANSI colors (terminal output and ANSI-derived
 * semantic tokens like `graph.lane*`) shares this resolved, always-complete
 * lookup so a theme with no ANSI colors still gets VS Code's real defaults
 * instead of a generic single-color fallback.
 */
export const resolveAnsiLookup = (vscodeColors: Record<string, string>, type: ThemeTypeArg): { ansi: AnsiLookup; fallbackTokens: string[] } => {
    const sourceAnsi = readAnsiFromVscodeColors(vscodeColors)
    const fallbackTokens: string[] = []
    const ansi = {} as AnsiLookup
    for (const name of TERMINAL_ANSI_TOKENS) {
        const value = sourceAnsi[name]
        if (value) {
            ansi[name] = value
            continue
        }
        ansi[name] = VSCODE_DEFAULT_ANSI_PALETTE[type][name]
        fallbackTokens.push(name)
    }
    return { ansi, fallbackTokens }
}

export const resolveColors = (vscodeColors: Record<string, string>, type: ThemeTypeArg, ansi: AnsiLookup) => {
    const ctx: ResolveContext = { vscodeColors, resolved: {}, ansi, type }
    const safeDefaultNotices: string[] = []

    for (const entry of COLOR_MAPPING) {
        const resolved = resolveColorEntry(entry, ctx)
        if (resolved === undefined) safeDefaultNotices.push(`${entry.taideKey} (category: ${entry.category})`)
        ctx.resolved[entry.taideKey] = resolved ?? SAFE_DEFAULT_COLORS[type][entry.category]
    }

    return { colors: ctx.resolved, safeDefaultNotices }
}

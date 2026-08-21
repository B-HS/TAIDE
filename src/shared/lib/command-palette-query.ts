import type { IRange, languages } from 'monaco-editor'

export type PaletteMode = 'commands' | 'files' | 'symbol' | 'line' | 'workspaceSymbol'

export const COMMAND_MODE_PREFIX = '>'
export const SYMBOL_MODE_PREFIX = '@'
export const LINE_MODE_PREFIX = ':'
export const WORKSPACE_SYMBOL_MODE_PREFIX = '#'

export type ParsedPaletteQuery = {
    mode: PaletteMode
    searchTerm: string
}

const PALETTE_MODE_PREFIXES: { prefix: string; mode: PaletteMode }[] = [
    { prefix: COMMAND_MODE_PREFIX, mode: 'commands' },
    { prefix: SYMBOL_MODE_PREFIX, mode: 'symbol' },
    { prefix: LINE_MODE_PREFIX, mode: 'line' },
    { prefix: WORKSPACE_SYMBOL_MODE_PREFIX, mode: 'workspaceSymbol' },
]

export const parsePaletteQuery = (rawQuery: string): ParsedPaletteQuery => {
    const matchedPrefix = PALETTE_MODE_PREFIXES.find(({ prefix }) => rawQuery.startsWith(prefix))
    if (!matchedPrefix) return { mode: 'files', searchTerm: rawQuery }
    return { mode: matchedPrefix.mode, searchTerm: rawQuery.slice(matchedPrefix.prefix.length).trimStart() }
}

export const buildCommandModeQuery = (searchTerm: string = '') => `${COMMAND_MODE_PREFIX}${searchTerm}`

export type PaletteLineTarget = { line: number; column: number }

const LINE_TARGET_PATTERN = /^(\d+)(?::(\d+))?$/

/** Parses a `line` mode search term (`"123"` or `"123:45"`, already stripped of `:` by {@link parsePaletteQuery}) into a 1-based line/column target, or `null` when the input isn't a valid line reference. */
export const parseLineModeTarget = (searchTerm: string): PaletteLineTarget | null => {
    const match = LINE_TARGET_PATTERN.exec(searchTerm.trim())
    if (!match) return null
    const line = Number(match[1])
    const column = match[2] ? Number(match[2]) : 1
    if (line < 1 || column < 1) return null
    return { line, column }
}

export type FlatPaletteSymbol = {
    name: string
    detail: string
    kind: number
    containerLabel: string
    selectionRange: IRange
}

/** Flattens a monaco `DocumentSymbol` hierarchy (from `requestDocumentSymbols`) into a fuzzy-filterable list for `symbol` mode, each entry carrying an ancestor breadcrumb (`"Class > method"`) as `containerLabel`. */
export const flattenDocumentSymbols = (symbols: languages.DocumentSymbol[], containerLabel: string = ''): FlatPaletteSymbol[] =>
    symbols.flatMap((symbol) => [
        { name: symbol.name, detail: symbol.detail, kind: symbol.kind, containerLabel, selectionRange: symbol.selectionRange },
        ...flattenDocumentSymbols(symbol.children ?? [], containerLabel ? `${containerLabel} > ${symbol.name}` : symbol.name),
    ])

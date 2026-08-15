import type { languages } from 'monaco-editor'
import type { TreeRow } from '@shared/api/bindings'

const PATH_SEPARATOR = '/'

export type CursorPosition = { lineNumber: number; column: number }

export const containsPosition = (range: languages.DocumentSymbol['range'], position: CursorPosition) => {
    if (position.lineNumber < range.startLineNumber || position.lineNumber > range.endLineNumber) return false
    if (position.lineNumber === range.startLineNumber && position.column < range.startColumn) return false
    if (position.lineNumber === range.endLineNumber && position.column > range.endColumn) return false
    return true
}

/**
 * Walks `symbols` top-down, following the child whose range contains `position` at each level, and
 * returns the resulting chain from the outermost enclosing symbol to the innermost one. Mirrors
 * `outline-panel`'s reuse of the same hierarchical `DocumentSymbol[]` shape (flat `SymbolInformation`
 * results are already normalized to childless `DocumentSymbol`s upstream in `requestDocumentSymbols`).
 */
export const findEnclosingSymbolChain = (symbols: languages.DocumentSymbol[], position: CursorPosition): languages.DocumentSymbol[] => {
    for (const symbol of symbols) {
        if (!containsPosition(symbol.range, position)) continue
        return [symbol, ...findEnclosingSymbolChain(symbol.children ?? [], position)]
    }
    return []
}

export const splitRelativePathSegments = (relativePath: string) => relativePath.split(PATH_SEPARATOR).filter(Boolean)

const trimTrailingSlash = (value: string) => (value.endsWith(PATH_SEPARATOR) ? value.slice(0, -1) : value)

/** Absolute path for every prefix of `segments`, joined onto `root` (index `i` = path through `segments[0..=i]`). */
export const buildSegmentPaths = (root: string, segments: string[]) => {
    const normalizedRoot = trimTrailingSlash(root)
    return segments.map((_, index) => `${normalizedRoot}${PATH_SEPARATOR}${segments.slice(0, index + 1).join(PATH_SEPARATOR)}`)
}

export const parentDirOf = (path: string) => {
    const index = path.lastIndexOf(PATH_SEPARATOR)
    return index <= 0 ? PATH_SEPARATOR : path.slice(0, index)
}

/** Direct children of `parentPath` among `rows` (siblings for a single breadcrumb path segment). */
export const filterDirectChildren = (rows: TreeRow[], parentPath: string) => rows.filter((row) => parentDirOf(row.path) === parentPath)

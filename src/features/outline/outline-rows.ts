import type { languages } from 'monaco-editor'

export type OutlineRow = {
    id: string
    symbol: languages.DocumentSymbol
    depth: number
    hasChildren: boolean
    collapsed: boolean
}

/**
 * One row is a single `text-xs` line box (16px) inside `py-0.5` (2px each side); the kind icon and
 * the chevron are both smaller than that line box, so every row measures the same. The rows are a
 * fixed size rather than re-measured because the virtualizer can then place the whole outline
 * without ever mounting a row.
 */
export const OUTLINE_ROW_HEIGHT_PX = 20

/**
 * Flattens the symbol tree into one linear row list — the shape a virtualizer needs, since it
 * addresses items by a single index and cannot descend into children. The panel used to render the
 * whole tree recursively with no collapsing at all (research 3a M2), and a document symbol request
 * is re-issued 400ms after every edit (`docs/features/lsp.md` §4), so a large file redrew every
 * symbol on each pause in typing.
 *
 * A collapsed row keeps its own row and contributes none of its descendants. Ids are the row's
 * position in the tree (`/0/2/1`) rather than its name: two sibling overloads legitimately share a
 * name and kind, and an id collision would collapse both at once.
 */
export const buildOutlineRows = (symbols: readonly languages.DocumentSymbol[], collapsedIds: ReadonlySet<string>): OutlineRow[] => {
    const rows: OutlineRow[] = []

    const visit = (nodes: readonly languages.DocumentSymbol[], depth: number, parentId: string) => {
        nodes.forEach((symbol, index) => {
            const id = `${parentId}/${index}`
            const children = symbol.children ?? []
            const collapsed = collapsedIds.has(id)
            rows.push({ id, symbol, depth, hasChildren: children.length > 0, collapsed })
            if (collapsed || children.length === 0) return
            visit(children, depth + 1, id)
        })
    }

    visit(symbols, 0, '')

    return rows
}

/**
 * Where ArrowLeft goes from a row that has nothing to collapse: the nearest row above it that sits
 * one level out. Returns `-1` for a top-level row, which the caller treats as "stay put".
 */
export const findOutlineParentIndex = (rows: readonly OutlineRow[], fromIndex: number) => {
    if (fromIndex < 0 || fromIndex >= rows.length) return -1
    const depth = rows[fromIndex].depth
    for (let index = fromIndex - 1; index >= 0; index -= 1) {
        if (rows[index].depth < depth) return index
    }
    return -1
}
